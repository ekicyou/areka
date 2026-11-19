//! URL → 項目 id・ページ URL → 名前の突き合わせ（設計 D-4・D-5）。
//!
//! # 3 段の解決（設計 D-4）
//!
//! 取り出した URL 1 件ごとに、次の順で行き先が決まる。
//!
//! 1. カタログの項目 URL と**完全一致**すれば、その 1 項目の証拠。
//! 2. 一致しなければ、フラグメントを外したページ URL と完全一致するか見る。一致
//!    すれば要件 5.4 の語彙表の目印として扱い、名前の突き合わせへ進む。
//! 3. どちらでもなければ [`EvidenceIndex::unresolved`] へ回す。ここでは赤にしない
//!    ——赤にするのは検査の段（`SourceUrlNotInCatalog`）である。
//!
//! 同じ URL を持つ 2 項目はカタログの読み込み（`catalog::read`）と組み立て
//! （`catalog::build`）がともに拒むので、完全一致で曖昧さなく 1 件に定まる。アンカーを持たない項目は項目 URL と
//! ページ URL の綴りが同じになるが、1 段目が先なので項目の証拠になる。
//!
//! 同じ URL が複数のファイルに現れても赤にしない（設計 D-4）。証拠は id ごとに
//! **重複を除いた名前順**のファイルパスの一覧として並べる（要件 5.5）。
//!
//! # 語彙表の取り出し規則（設計 D-5・開発者裁定 2026-09-02 設計議題 1・案 ⒜）
//!
//! ページ URL の行（`/// ukadoc: <ページ URL>` の単独行）の**直後に始まる最初の
//! スライス定数**（`= &[` から対応する `]` まで）の中で、**要素ごとの最初の文字列
//! リテラル**を要素とみなす。要素の区切りはスライス直下の深さのコンマ。実物の 3 形
//! （`&[&str]`・`&[(&str, SetSemantics)]`・`&[FlatEntry]`）はいずれも名前が要素の
//! 最初の文字列なので、同じ規則で拾える。
//!
//! ページ URL の行の後にスライス定数が始まらない（間にほかの項目が挟まる、あるいは
//! 閉じない）場合は [`NameMatchFailure::TableMissing`] として検査の出力に並べる。**赤にはしない**。
//!
//! # 名前の突き合わせ（設計 D-5）
//!
//! 要素の文字列とそのページの見出しを [`normalized`] で揃えてから**完全一致**で
//! 比べる。部分一致は使わない。一致が 1 件に定まったときだけ証拠にし、0 件のときも
//! 2 件以上のときも [`EvidenceIndex::unmatched_names`] へ回す（要件 5.9 のとおり
//! 判定は人手に委ねる）。
//!
//! # 既存資産との関係（要件 9.2・9.3）
//!
//! ここが作るのは「カタログ id → その語彙表の要素名が置かれたファイル」の対応だけで
//! ある。`doc/shiori/fragments/` の契約カタログも `crates/areka-sylphya/src/vocab/` の
//! 語彙台帳も**置き換えない**（要件 9.2）。対応が付いた項目は以後その資産側の名前で
//! 辿れるので、同じ項目を 2 か所で数えることにはならない（要件 9.3）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。ソースの
//! 本文は入出力層の [`crate::io::sources::walk`] が読んで渡す。

use std::collections::{BTreeMap, BTreeSet};

use super::extract::extract;
use super::lex::{
    SLICE_START, line_offsets, read_raw_string, read_string, skip_block_comment, skip_char_literal,
    skip_line_comment, slice_element_names,
};
use super::{EvidenceIndex, NameMatchFailure, UnmatchedName, UnresolvedUrl, UrlHit};
use crate::catalog::Catalog;
use crate::model::{EntryId, PageName};

/// 全角空白。NFKC で半角空白 1 個になる。
const IDEOGRAPHIC_SPACE: char = '\u{3000}';
/// 全角形（`！`〜`～`）の最初の符号位置。
const FULLWIDTH_FIRST: u32 = 0xFF01;
/// 全角形の最後の符号位置（`～` FULLWIDTH TILDE）。
const FULLWIDTH_LAST: u32 = 0xFF5E;
/// 全角形から対応する ASCII への隔たり。
const FULLWIDTH_OFFSET: u32 = 0xFEE0;

/// ソース文の組から証拠の索引を作る（取り出し → 解決）。
///
/// `check`・`evidence` の副手続きとテストの見本が同じ経路を通るための 1 か所。
/// 片方だけが別の集め方をすると、検査が見た証拠と `evidence` が並べた証拠が食い違う。
pub fn index_of(sources: &[(String, String)], catalog: &Catalog) -> EvidenceIndex {
    let hits: Vec<UrlHit> = sources
        .iter()
        .flat_map(|(path, text)| extract(path, text))
        .collect();
    resolve(&hits, sources, catalog)
}

/// URL を項目 id へ解決する。ページ URL は語彙表の目印として扱う。
///
/// `hits` は取り出しの段（[`extract`]）が返した URL、`sources` は走査で読んだ
/// (パス, 本文) の組、`catalog` は正典の写し。
///
/// 失敗しない。綴りが違う URL も突き合わせできなかった名前も、値として持ち帰る
/// （赤にするかは検査の段が決める）。
///
/// 返る索引は**入力の並びに依らない**。3 つの欄はいずれも順序の決まった集合から
/// 作るので、`hits` を並べ替えても同じ値になる。`unresolved` と `unmatched_names` は
/// 同じ組が 2 度現れれば 1 件に畳む（同じ場所の同じ失敗を 2 度読ませない）。
pub fn resolve(hits: &[UrlHit], sources: &[(String, String)], catalog: &Catalog) -> EvidenceIndex {
    let by_url = catalog.by_url();
    let page_urls = catalog.page_urls();
    let texts: BTreeMap<&str, &str> = sources
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();

    let mut by_id: BTreeMap<EntryId, BTreeSet<String>> = BTreeMap::new();
    let mut unresolved: BTreeSet<UnresolvedUrl> = BTreeSet::new();
    // 目印は (パス, ページ URL) の組で 1 度だけ扱う。同じ組の取り出しが 2 件あれば
    // それは同じファイルの 2 つの目印行のことで、本文の側を数えれば足りる。
    let mut markers: BTreeSet<(&str, &str)> = BTreeSet::new();

    for hit in hits {
        if let Some(id) = by_url.get(hit.url.as_str()) {
            by_id
                .entry((*id).clone())
                .or_default()
                .insert(hit.path.clone());
        } else if page_urls.contains_key(&hit.url) {
            markers.insert((hit.path.as_str(), hit.url.as_str()));
        } else {
            unresolved.insert(UnresolvedUrl {
                path: hit.path.clone(),
                url: hit.url.clone(),
            });
        }
    }

    let mut unmatched: BTreeSet<UnmatchedName> = BTreeSet::new();
    for (path, page_url) in markers {
        // 目印が解決した以上、ページ名は必ず引ける。
        let Some(page) = page_urls.get(page_url) else {
            continue;
        };
        let text = texts.get(path).copied().unwrap_or_default();
        match_vocabulary(
            path,
            page_url,
            page,
            text,
            catalog,
            &mut by_id,
            &mut unmatched,
        );
    }

    EvidenceIndex {
        by_id: by_id
            .into_iter()
            .map(|(id, paths)| (id, paths.into_iter().collect()))
            .collect(),
        unresolved: unresolved.into_iter().collect(),
        unmatched_names: unmatched.into_iter().collect(),
    }
}

/// 1 ファイルの中の目印を数え、それぞれの語彙表を見出しへ突き合わせる。
fn match_vocabulary(
    path: &str,
    page_url: &str,
    page: &PageName,
    text: &str,
    catalog: &Catalog,
    by_id: &mut BTreeMap<EntryId, BTreeSet<String>>,
    unmatched: &mut BTreeSet<UnmatchedName>,
) {
    // 見出しは正規化した綴りで引けるようにしておく。同じ綴りが 2 つ以上ある見出しを
    // 持つページがあるので、id は一覧で持つ（設計 D-5 規則 3）。
    let mut titles: BTreeMap<String, Vec<&EntryId>> = BTreeMap::new();
    for (id, title) in catalog.titles_of_page(page) {
        titles.entry(normalized(title)).or_default().push(id);
    }

    let mut found_marker = false;
    for (offset, line) in line_offsets(text) {
        if !is_marker_line(line, page_url) {
            continue;
        }
        found_marker = true;
        // 行の次の文字から後ろだけを見る（「直後に始まる最初のスライス定数」）。
        let after = offset + line.len();
        let rest = &text.as_bytes()[after..];
        let Some(names) = find_slice_start(rest).and_then(|start| slice_element_names(rest, start))
        else {
            unmatched.insert(UnmatchedName {
                path: path.to_owned(),
                page_url: page_url.to_owned(),
                reason: NameMatchFailure::TableMissing,
            });
            continue;
        };
        for name in names {
            match titles.get(&normalized(&name)).map(Vec::as_slice) {
                Some([id]) => {
                    by_id
                        .entry((*id).clone())
                        .or_default()
                        .insert(path.to_owned());
                }
                Some(_) => {
                    unmatched.insert(UnmatchedName {
                        path: path.to_owned(),
                        page_url: page_url.to_owned(),
                        reason: NameMatchFailure::Ambiguous(name),
                    });
                }
                None => {
                    unmatched.insert(UnmatchedName {
                        path: path.to_owned(),
                        page_url: page_url.to_owned(),
                        reason: NameMatchFailure::NoMatch(name),
                    });
                }
            }
        }
    }

    if !found_marker {
        // 本文が渡されていない・目印の行が取り出しの形でない、のどちらか。どちらも
        // 「目印だが表が続かない」であって綴り違いではない（設計 D-5）。
        unmatched.insert(UnmatchedName {
            path: path.to_owned(),
            page_url: page_url.to_owned(),
            reason: NameMatchFailure::TableMissing,
        });
    }
}

/// その行が当のページ URL の目印かどうか。
///
/// **判定は取り出しの段に任せる**（[`extract`] を 1 行だけに当てる）。行の形の規則を
/// ここで書き直すと、片方だけが直されて静かにずれる。説明文が続く行が目印にならない
/// のも、この委譲がそのまま効いている（要件 5.3）。
fn is_marker_line(line: &str, page_url: &str) -> bool {
    extract("", line)
        .first()
        .is_some_and(|hit| hit.url == page_url)
}

/// 最初のスライス定数の中身が始まる位置。無ければ `None`（＝「表が続かない」）。
///
/// `rest` は目印の行より後ろの本文（バイト列）。型の注記（`&[&str]` の `&[`）を起点と
/// 読み違えないよう、綴りは `= &[` で見る（設計 D-5）。文字列・生文字列・文字リテラル・
/// コメントの外で `;` か `{` に出会ったら、目印の直後の項目はスライス定数ではないので
/// そこで止める——止めないと、ずっと後ろの無関係な表を目印の表として読む。
fn find_slice_start(rest: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            b'"' => i = read_string(rest, i).1,
            b'r' => i = read_raw_string(rest, i).map_or(i + 1, |(_, next)| next),
            b'\'' => i = skip_char_literal(rest, i),
            b'/' if rest.get(i + 1) == Some(&b'/') => i = skip_line_comment(rest, i),
            b'/' if rest.get(i + 1) == Some(&b'*') => i = skip_block_comment(rest, i),
            _ if rest[i..].starts_with(SLICE_START.as_bytes()) => {
                return Some(i + SLICE_START.len());
            }
            b';' | b'{' => return None,
            _ => i += 1,
        }
    }
    None
}

/// 名前を突き合わせる前に揃える（設計 D-5 の「NFKC ＋ 連続空白の畳み込み ＋
/// 前後の空白落とし」）。
///
/// # コーパスに合わせた正規化であること
///
/// この道具は依存を増やせない（設計 D-1）ので完全な NFKC は持たない。代わりに、
/// スナップショットの見出しと語彙表の要素で NFKC が綴りを変える文字だけを写す。
///
/// - `U+3000`（全角空白）→ `U+0020`
/// - `U+FF5E`（全角チルダ `～`）→ `U+007E`
///
/// どちらも「全角形 → ASCII」の帯（`U+FF01`〜`U+FF5E`）と全角空白に収まるので、
/// 1 文字ずつの表ではなく**帯ごと**写す。
///
/// **これは一般の NFKC ではない。** 半角カナ（`U+FF61`〜`U+FF9F`）・丸囲み数字・
/// 合字・組文字は写さない。将来のスナップショットがそういう文字を持つ見出しを
/// 増やすと、その項目は黙って突き合わせに失敗する（赤にはならず
/// [`NameMatchFailure::NoMatch`] に落ちる）。気付ける仕掛けは
/// `SHIORI_RESOURCE_IDS` の較正テストで、そのページに限っては 1 件でも外れれば赤に
/// なる。他のページには同じ見張りが無い。
///
/// なお `U+3000` については写しと空白の畳み込みが**二重に効く**（全角空白は
/// [`char::is_whitespace`] でもある）。どちらか一方を外しても実データの結果は動かない
/// ので、この 1 文字について「写しが効いていること」を単体で示すテストは書けない。
/// 見出しに現れる非 ASCII 空白はこれだけなので、二重の守りをそのまま残す。
fn normalized(raw: &str) -> String {
    let widened: String = raw.chars().map(to_ascii_form).collect();
    let mut out = String::with_capacity(widened.len());
    for (index, word) in widened.split_whitespace().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// 全角形の 1 文字を対応する ASCII へ写す。帯の外はそのまま。
fn to_ascii_form(c: char) -> char {
    if c == IDEOGRAPHIC_SPACE {
        return ' ';
    }
    let code = c as u32;
    if (FULLWIDTH_FIRST..=FULLWIDTH_LAST).contains(&code) {
        char::from_u32(code - FULLWIDTH_OFFSET).unwrap_or(c)
    } else {
        c
    }
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;
