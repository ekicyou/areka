//! 初期台帳の生成と、既存本文を保ったままの差し込み（要件 3.3・3.3a）。
//!
//! ここは純粋層である。**ファイルには触らない**——本文を組み立てて文字列で返すだけで、
//! どこへ書くかは入出力層（`io::files::write_lf`）の仕事である（要件 6.2・設計
//! Architecture）。
//!
//! # 組み立て直さず、切り貼りする
//!
//! 台帳は**人が手で書く文書**である。要件 3.3a は「既存の項目を一切書き換えず」欠けた
//! id だけを挿入せよと言う。値を読んで組み立て直すと、備考の書き方も区切りの空白も
//! 空行も持ち主の書いたものと変わってしまうので、既存の本文をそのまま残すには本文を
//! 切り貼りするしかない（設計 D-12）。
//!
//! したがって [`merge_initial`] は、既存の塊も**前置きも**バイト列のまま写す。前置きを
//! 組み立て直さないので、担当ページの一覧が古いままでも書き換わらない——それを咎める
//! のは整合検査の仕事であって、ここではない（要件 3.3a が優先する）。
//!
//! # 並び順は厳密な昇順（設計 D-12）
//!
//! 差し込む位置は id の文字順で決まるので、既存の塊が id 順に並んでいることが前提に
//! なる。並んでいなければ**並べ替えず、順序を破る id を告げて失敗する**（持ち主が
//! 直す）。
//!
//! 判定は**厳密な**昇順で、前の id と等しい隣り合わせも落とす。これは重複を捕まえる
//! ためである。備考の複数行文字列の中に行頭の `[entry."…"]` が既にある id と同じ綴りで
//! 現れると、`blocks::split` の較正（`toml` の鍵**集合**との一致）では見抜けない
//! （設計 D-12 が明記する盲点）。`ledger-init` は整合検査より先に走るので、ここが緩いと
//! 盲点は永久に開いたままになる。
//!
//! # 1 バイトの安定
//!
//! 欄の並びも区切りの空白も [`INITIAL_COLUMNS`] と [`render_initial_entry`] の 1 か所
//! だけが決める。欠けた id が 1 つも無ければ、返る本文は渡された本文と 1 バイトも違わ
//! ない——写す以外に通る道が無いからである。値はすべて [`crate::tomlout`] を通す
//! （`toml` の書き出しは使わない。設計「境界の要点」1 つ目）。

use std::collections::BTreeSet;

use super::blocks::{self, Block};
use crate::error::SurveyError;
use crate::model::{Domain, EntryId, PageName, Status};
use crate::tomlout::{basic_string, keyed_table_header, string_array};

/// 台帳を置くディレクトリ（`io::paths::ledger_path` と同じ場所・ワークスペース根から）。
///
/// `io::paths` は呼ばない——あれはワークスペース根から組み立てた絶対パスを返すので、
/// 失敗の本文が計算機ごとに変わってしまう（要件 6.1 の決定性）。純粋層が入出力層に
/// 触らない形も保てる。綴りの正本は `Domain::as_key` 1 つきりである。
const LEDGER_DIR: &str = "doc/ukadoc-coverage/ledger";

/// 前置きの表の見出し。
const LEDGER_HEADER: &str = "[ledger]";

/// 項目を置く表の名前。
const ENTRY_TABLE: &str = "entry";

/// 冒頭の注意書きのうち、ドメインに依らない 2 行（要件付録 A.1 の逐語）。
///
/// 1 行目はファイル名を書くのでドメインごとに変わり、[`render_prologue`] が組み立てる。
const HEADER_LINES: [&str; 2] = [
    "# 人手で記入・機械で検査する台帳。形式の正本は",
    "# .kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。",
];

/// 初期値を書く欄の並び（要件付録 A.1 の見本・A.2 の 1 文）。
///
/// 任意の 2 欄（`alias_of`・`supersedes`）は**書かない**。付録 A.2 は `alias_of` を
/// `status = "alias"` のときだけ許しており、初期値の状態は `unclassified` だからである。
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

/// 初期台帳を組み立て、既存本文があれば欠けた id だけを差し込む（要件 3.3・3.3a）。
///
/// `existing` が `None` なら前置きと全項目を新規に書き出す。`Some` なら既存の塊を
/// **1 バイトも変えずに写し**、`ids` にあって本文に無い id の塊だけを id の文字順の
/// 位置へ差し込む。本文にあって `ids` に無い項目は落とさない（人が書いたものを消さない）。
///
/// `ids` は順も重複も問わない。ここで文字順に整えて重複を畳む。
///
/// # 落ちる場合
///
/// - 既存本文を塊に切り分けられない（[`blocks::split`] の失敗をそのまま上げる）
/// - 既存の塊が id の**厳密な**昇順に並んでいない（[`SurveyError::LedgerOutOfOrder`]。
///   同じ id が 2 度現れる場合もここで落ちる・設計 D-12）
pub fn merge_initial(
    existing: Option<&str>,
    domain: Domain,
    pages: &[PageName],
    ids: &[EntryId],
) -> Result<String, SurveyError> {
    // 文字順・重複なしの一覧にする。差し込む位置はこの並びで決まる（設計 D-9。実測で
    // id はすべて ASCII なので byte 昇順がそのまま文字順になる）。
    let wanted: BTreeSet<&EntryId> = ids.iter().collect();
    match existing {
        None => Ok(render_new(domain, pages, &wanted)),
        Some(text) => splice(text, domain, &wanted),
    }
}

/// 1 項目分の初期値を組み立てる（要件付録 A.2 の 1 文）。
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

/// 既存本文が無いときの全文（要件 3.3）。
fn render_new(domain: Domain, pages: &[PageName], wanted: &BTreeSet<&EntryId>) -> String {
    let mut out = render_prologue(domain, pages);
    for id in wanted {
        out.push_str(&render_initial_entry(id));
    }
    out
}

/// 冒頭の注意書きと `[ledger]`（要件付録 A.1 の逐語）。
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
    let names: Vec<String> = pages
        .iter()
        .map(|page| page.as_str().to_owned())
        .collect::<Vec<String>>();
    push_field(&mut out, "pages", &string_array(&names));
    push_line(&mut out, "");
    out
}

/// 既存本文へ欠けた id を差し込む（要件 3.3a）。
///
/// 前置きも既存の塊もバイト列のまま写す。触るのは「塊と塊の間」だけである。
fn splice(text: &str, domain: Domain, wanted: &BTreeSet<&EntryId>) -> Result<String, SurveyError> {
    let (prologue_end, blocks) = blocks::split(text)?;
    check_strictly_ascending(&blocks, domain)?;

    let present: BTreeSet<&EntryId> = blocks.iter().map(|block| &block.id).collect();
    let missing: Vec<&EntryId> = wanted
        .iter()
        .copied()
        .filter(|id| !present.contains(id))
        .collect::<Vec<&EntryId>>();

    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..prologue_end]);

    let mut next = 0usize;
    for block in &blocks {
        // 欠けている id のうち、この塊より前に来るものを先に置く。
        while next < missing.len() && *missing[next] < block.id {
            push_entry(&mut out, missing[next]);
            next += 1;
        }
        out.push_str(&text[block.start..block.end]);
    }
    // 最後の塊より後ろに来るものが残っている。
    while next < missing.len() {
        push_entry(&mut out, missing[next]);
        next += 1;
    }
    Ok(out)
}

/// 塊が id の**厳密な**昇順に並んでいることを確かめる（設計 D-12）。
///
/// 等しい隣り合わせも落とす。これが備考の中に隠れた重複を捕まえる唯一の場所である
/// （切り分けの較正は集合で比べるので素通りする）。並べ替えはしない——手で書いた本文を
/// 機械が並べ直すと、持ち主の意図した並びが黙って消える。
fn check_strictly_ascending(blocks: &[Block], domain: Domain) -> Result<(), SurveyError> {
    for pair in blocks.windows(2) {
        if pair[1].id <= pair[0].id {
            return Err(SurveyError::LedgerOutOfOrder {
                file: ledger_file(domain),
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

/// 失敗の本文に添える台帳の置き場（要件 6.12）。
fn ledger_file(domain: Domain) -> String {
    format!("{LEDGER_DIR}/{}.toml", domain.as_key())
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
