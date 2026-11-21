//! 初期台帳の生成と、既存本文を保ったままの差し込み。
//!
//! ここは純粋層である。**ファイルには触らない**——本文を組み立てて文字列で返すだけで、
//! どこへ書くかは入出力層（`io::files::write_lf`）の仕事である。
//!
//! # 組み立て直さず、切り貼りする
//!
//! 台帳は**人が手で書く文書**で、差し込みは既存の項目を一切書き換えず欠けた id だけを
//! 挿入する。値を読んで組み立て直すと、備考の書き方も区切りの空白も空行も持ち主の
//! 書いたものと変わってしまうので、既存の本文をそのまま残すには本文を切り貼りするしかない。
//!
//! したがって [`merge_initial`] は、既存の塊も**前置きも**バイト列のまま写す。前置きを
//! 組み立て直さないので、担当ページの一覧が古いままでも書き換わらない——それを咎める
//! のは整合検査の仕事であって、ここではない。
//!
//! # 並び順は厳密な昇順
//!
//! 差し込む位置は id の文字順で決まるので、既存の塊が id 順に並んでいることが前提に
//! なる。並んでいなければ**並べ替えず、順序を破る id を告げて失敗する**（持ち主が
//! 直す）。
//!
//! 判定は**厳密な**昇順で、前の id と等しい隣り合わせも落とす。これは重複を捕まえる
//! ためである。備考の複数行文字列の中に行頭の `[entry."…"]` が既にある id と同じ綴りで
//! 現れると、`blocks::split` の較正（`toml` の鍵**集合**との一致）では見抜けない。
//! `ledger-init` は整合検査より先に走るので、ここが緩いと
//! 重複した塊のまま差し込みが通る（次の整合検査では `check::structure::check_file_order` が
//! 同じ形で拾う）。`priority-apply` も同じ確かめを頭で通す
//! （[`crate::ledger::patch::replace_priority`]）。
//!
//! # 差し込む位置とコメント
//!
//! 塊の終わりは次の見出しの直前なので、項目の真上に書いたコメントは 1 つ前の塊（最初の
//! 項目なら前置き）の尻尾に入る。差し込む塊は、その尻尾に続く行頭 `#` の連続した行の
//! **前**に置く——後ろに置くと、次の項目に付いていたコメントが差し込んだ項目に付いたように
//! 読める。
//!
//! # 1 バイトの安定
//!
//! 欄の並びも区切りの空白も [`INITIAL_COLUMNS`] と [`render_initial_entry`] の 1 か所
//! だけが決める。欠けた id が 1 つも無ければ、返る本文は渡された本文と 1 バイトも違わ
//! ない——写す以外に通る道が無いからである。値はすべて [`crate::tomlout`] を通す
//! （`toml` の書き出しは使わない）。

use std::collections::BTreeSet;

use super::blocks::{self, Block};
use super::{ENTRY_TABLE, at_ledger_file, ledger_file};
use crate::error::SurveyError;
use crate::model::{Domain, EntryId, PageName, Status};
use crate::tomlout::{basic_string, keyed_table_header, string_array};

/// 前置きの表の見出し。
const LEDGER_HEADER: &str = "[ledger]";

/// 冒頭の注意書きのうち、ドメインに依らない 2 行。
///
/// 1 行目はファイル名を書くのでドメインごとに変わり、[`render_prologue`] が組み立てる。
const HEADER_LINES: [&str; 2] = [
    "# 人手で記入・機械で検査する台帳。形式の正本は",
    "# .kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。",
];

/// 初期値を書く欄の並び。
///
/// 任意の 2 欄（`alias_of`・`supersedes`）は**書かない**。`alias_of` は
/// `status = "alias"` のときだけ許され、初期値の状態は `unclassified` だからである。
/// 空で置くと読み取りがそこで落ちる。
const INITIAL_COLUMNS: [&str; 7] = [
    "status",
    "introduced",
    "owner",
    "priority",
    "values",
    "links",
    "note",
];

/// 初期台帳を組み立て、既存本文があれば欠けた id だけを差し込む。
///
/// `existing` が `None` なら前置きと全項目を新規に書き出す。`Some` なら既存の塊を
/// **1 バイトも変えずに写し**、`ids` にあって本文に無い id の塊だけを id の文字順の
/// 位置へ差し込む。本文にあって `ids` に無い項目は落とさない（人が書いたものを消さない）。
///
/// `ids` は順も重複も問わない。ここで文字順に整えて重複を畳む。
///
/// # 落ちる場合
///
/// - 既存本文を塊に切り分けられない（[`blocks::split`] の失敗に台帳の置き場を添えて上げる）
/// - 既存の塊が id の**厳密な**昇順に並んでいない（[`SurveyError::LedgerOutOfOrder`]。
///   同じ id が 2 度現れる場合もここで落ちる）
pub fn merge_initial(
    existing: Option<&str>,
    domain: Domain,
    pages: &[PageName],
    ids: &[EntryId],
) -> Result<String, SurveyError> {
    // 文字順・重複なしの一覧にする。差し込む位置はこの並びで決まる（id はすべて ASCII
    // なので byte 昇順がそのまま文字順になる）。
    let wanted: BTreeSet<&EntryId> = ids.iter().collect();
    match existing {
        None => Ok(render_new(domain, pages, &wanted)),
        Some(text) => splice(text, domain, &wanted),
    }
}

/// 1 項目分の初期値を組み立てる。
///
/// 返る塊は見出し行・欄 7 行・**区切りの空行 1 行**をこの順に持ち、必ず改行で終わる。
/// 空行を塊の側に持たせるのは、切り分け（[`blocks::split`]）が塊の終わりを次の見出しの
/// 直前と定めているからである。区切りを塊の外に置くと、差し込んだ塊と写した塊で空行の
/// 持ち主が食い違い、2 度目の差し込みで本文が動く。
pub fn render_initial_entry(id: &EntryId) -> String {
    let values = [
        basic_string(Status::Unclassified.as_key()),
        basic_string(""),
        basic_string(""),
        basic_string(""),
        string_array(&[]),
        // 関連はインラインテーブルの配列だが、空の配列は要素の型を問わず同じ綴りになる。
        string_array(&[]),
        basic_string(""),
    ];

    let mut out = String::new();
    push_line(&mut out, &keyed_table_header(ENTRY_TABLE, id.as_str()));
    for (column, value) in INITIAL_COLUMNS.into_iter().zip(values) {
        push_field(&mut out, column, &value);
    }
    push_line(&mut out, "");
    out
}

/// 既存本文が無いときの全文。
fn render_new(domain: Domain, pages: &[PageName], wanted: &BTreeSet<&EntryId>) -> String {
    let mut out = render_prologue(domain, pages);
    for id in wanted {
        out.push_str(&render_initial_entry(id));
    }
    out
}

/// 冒頭の注意書きと `[ledger]`。
///
/// 末尾は空行で終わる。最初の項目の見出しがそのまま続けられる形にしておくと、項目が
/// 0 件のときと 1 件以上のときで前置きの綴りが変わらない。
fn render_prologue(domain: Domain, pages: &[PageName]) -> String {
    let mut out = String::new();
    push_line(&mut out, &format!("# {}", ledger_file(domain)));
    for line in HEADER_LINES {
        push_line(&mut out, line);
    }
    push_line(&mut out, "");

    push_line(&mut out, LEDGER_HEADER);
    push_field(&mut out, "domain", &basic_string(domain.as_key()));
    let names: Vec<String> = pages.iter().map(|page| page.as_str().to_owned()).collect();
    push_field(&mut out, "pages", &string_array(&names));
    push_line(&mut out, "");
    out
}

/// 既存本文へ欠けた id を差し込む。
///
/// 前置きも既存の塊もバイト列のまま写す。触るのは「塊と塊の間」だけである。
fn splice(text: &str, domain: Domain, wanted: &BTreeSet<&EntryId>) -> Result<String, SurveyError> {
    let file = ledger_file(domain);
    let (prologue_end, blocks) = blocks::split(text).map_err(|err| at_ledger_file(err, &file))?;
    check_strictly_ascending(&blocks, &file)?;

    let present: BTreeSet<&EntryId> = blocks.iter().map(|block| &block.id).collect();
    let missing: Vec<&EntryId> = wanted
        .iter()
        .copied()
        .filter(|id| !present.contains(id))
        .collect();

    let mut out = String::with_capacity(text.len());
    // 最初の塊の真上のコメントは前置きの尻尾にあるので、前置きも同じく切る。
    let prologue = &text[..prologue_end];
    let cut = trailing_comments_start(prologue);
    out.push_str(&prologue[..cut]);

    let mut next = 0usize;
    // 前の塊（または前置き）の尻尾のコメント。次の塊の前に差し込む項目より後ろに写す。
    let mut held = &prologue[cut..];
    for block in &blocks {
        while next < missing.len() && *missing[next] < block.id {
            push_entry(&mut out, missing[next]);
            next += 1;
        }
        out.push_str(held);
        let body = &text[block.start..block.end];
        let cut = trailing_comments_start(body);
        out.push_str(&body[..cut]);
        held = &body[cut..];
    }
    // 最後の塊の尻尾は後ろに続く項目の見出しではないので、そのまま写してから足す。
    out.push_str(held);
    while next < missing.len() {
        push_entry(&mut out, missing[next]);
        next += 1;
    }
    Ok(out)
}

/// 塊の尻尾に続く行頭 `#` の連続した行の始まり（無ければ塊の終わり）。
///
/// 備考の複数行文字列の最後の行が `#` で始まることもある。そこで切ると文字列が割れるので、
/// 末尾から 1 行ずつ切る位置を見て、切った前半が TOML として読める最も前の位置を採る。
/// どこで切っても読めなければ切らない。
fn trailing_comments_start(body: &str) -> usize {
    let mut at = body.len();
    let mut cut = body.len();
    for line in body.split_inclusive('\n').rev() {
        if !line.starts_with('#') {
            break;
        }
        at -= line.len();
        if body[..at].parse::<toml::Table>().is_ok() {
            cut = at;
        }
    }
    cut
}

/// 塊が id の**厳密な**昇順に並んでいることを確かめる。
///
/// 等しい隣り合わせも落とす。差し込みと `priority` の置換は、備考の中に隠れた重複をここで
/// 捕まえる（整合検査では `check::structure::check_file_order` が同じ形で拾う。切り分けの
/// 較正は集合で比べるので素通りする）。並べ替えはしない——手で書いた本文を
/// 機械が並べ直すと、持ち主の意図した並びが黙って消える。
///
/// `file` は失敗に添える台帳の置き場。
pub(super) fn check_strictly_ascending(blocks: &[Block], file: &str) -> Result<(), SurveyError> {
    for pair in blocks.windows(2) {
        if pair[1].id <= pair[0].id {
            return Err(SurveyError::LedgerOutOfOrder {
                file: file.to_owned(),
                // 順序を破っている側、つまり**後ろに来てしまった** id を告げる。
                id: pair[1].id.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

/// 差し込む塊を押し込む。見出しが必ず行頭から始まるようにする。
///
/// 手で書いた本文は末尾に改行が無いことがある。そのまま繋ぐと `note = ""[entry."…"]`
/// になって TOML として読めなくなるので、改行を 1 つだけ足す。既存の塊のバイト列は
/// 触らない（足すのはその後ろである）。空行までは足さない——塊と塊の間に空行を置かない
/// 書き方をしている本文へ、こちらの好みを差し込まないためである。
fn push_entry(out: &mut String, id: &EntryId) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&render_initial_entry(id));
}

/// `名前 = 値` の 1 行。値は組み上がった TOML の値の断片を受け取る。
fn push_field(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(" = ");
    push_line(out, value);
}

/// 1 行を改行付きで押し込む。
fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;
