//! `report/<ドメイン>.md` の本文（要件 7.1・設計 D-11）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。台帳 1 本と
//! テーマ名を受け取り、文字列を返す。書き出しは入出力層（`io::files::write_lf`）の
//! 担当である。
//!
//! # 入力はその台帳 1 本とテーマ名だけ（設計 D-11）
//!
//! カタログも証拠も他の台帳も受け取らない。これは省略ではなく要件そのもので、
//! 4 本の調査 spec が互いの成果物を古くせずに並走できること（要件 3.4）が、この関数が
//! **その台帳だけ**から決まることに載っている。ページ名は項目 id の 2 番目の区切りから
//! 取れるので（[`crate::model::EntryId::page`]）、ページ別の分布にカタログは要らない。
//! 証拠の有無は `report/summary.md` の担当である。
//!
//! # 載せるのは要件 7.1 の 5 項目だけ
//!
//! ⑴ 状態の分布（ドメイン全体とページ別）⑵ SSP 世代別の対応表 ⑶ 別名の一覧
//! ⑷ テーマ別の状態分布 ⑸ ドメイン内で関連が閉じている束の一覧。順番も設計の
//! 列挙どおりに固定する。数えるところは [`super::tally`]、束を作るところは
//! [`super::bundle`] が受け持ち、ここは並べるだけ——同じ数を 2 か所で数えない。
//!
//! # 未分類の件数の正はページ別の表（要件 6.9）
//!
//! 台帳には未分類の件数を宣言する欄が無い。「未分類の残りがどのページに何件あるか」を
//! 答えるのはページ別の表ただ 1 つで、合計だけを載せた行に畳まない。
//!
//! # 生成日時を書かない（要件 7.3・設計 Data Models）
//!
//! 本文に時刻も環境も混ぜない。同じ台帳からは何度作っても 1 バイトも変わらない。
//! 見出し構成は固定で、数字は表で書く。改行は `\n` だけを使う（設計 D-6。復帰文字が
//! 1 つでも混じると、新しさの検査（要件 7.4）が永久に赤くなる）。
//!
//! # 呼び名は 1 か所でしか綴らない
//!
//! 状態の平易な日本語（要件 7.8）は [`Status::as_japanese`] が正本で、ここでは綴り
//! 直さない。同じ語彙を 2 か所に綴ると、片方を並べ替える誤りがテストを素通りする。

use std::collections::{BTreeMap, BTreeSet};

use super::bundle::{Bundle, bundles};
use super::tally::{StatusCounts, tally};
use crate::ledger::Ledger;
use crate::model::{EntryId, Status};

/// 別名の行に参照先が書かれていないときの表示。
///
/// 行ごと落とすと「別名なのに一覧に出てこない」が説明の付かない形で起きる。空欄でも
/// 落とさず、書かれていないことを書く（参照先の実在の検査は要件 6.7 の担当）。
const NO_ALIAS_TARGET: &str = "（未設定）";

/// ドメイン別報告の本文を組み立てる（要件 7.1・設計 D-11）。
///
/// `themes` は報告に並べるテーマ名（要件 4.4 の 8 つ）。台帳が 1 件も使っていない
/// テーマも 0 件の行として並べる——欄ごと消すと「0 件だった」と「そもそも数えて
/// いない」が読み手には区別できない。台帳にだけ現れた名前も落とさず並べる。
pub fn render_domain(ledger: &Ledger, themes: &[&str]) -> String {
    let tally = tally(ledger);
    let mut out = String::new();

    out.push_str(&format!("# {} の網羅状況\n\n", ledger.domain.as_key()));
    out.push_str(
        "この本文は台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。\n",
    );

    // ⑴ 状態の分布——ドメイン全体。
    out.push_str("\n## 状態の分布\n\n");
    out.push_str("| 状態 | 件数 |\n| --- | ---: |\n");
    for (name, count) in tally.overall.japanese_rows() {
        out.push_str(&format!("| {name} | {count} |\n"));
    }
    out.push_str(&format!("| 合計 | {} |\n", tally.overall.total()));

    // ⑴ 状態の分布——ページ別。未分類の件数はこの表が正である（要件 6.9）。
    out.push_str("\n## ページ別の状態の分布\n\n");
    out.push_str("未分類の残りがどのページに何件あるかは、この表の「未分類」の列が正です。\n\n");
    push_status_header(&mut out, "ページ");
    for (page, counts) in &tally.by_page {
        push_status_row(&mut out, page.as_str(), counts);
    }

    // ⑵ SSP 世代別の対応表。並びは名前順（`2.10` が `2.9` より前）。
    out.push_str("\n## SSP 世代別の対応表\n\n");
    push_status_header(&mut out, "世代");
    for (generation, counts) in &tally.by_generation {
        push_status_row(&mut out, generation, counts);
    }

    // ⑶ 別名の一覧。
    out.push_str("\n## 別名の一覧\n\n");
    push_alias_list(&mut out, ledger);

    // ⑷ テーマ別の状態分布。
    out.push_str("\n## テーマ別の状態分布\n\n");
    push_status_header(&mut out, "テーマ");
    for name in theme_names(&tally.by_theme, themes) {
        let empty = StatusCounts::new();
        let counts = tally.by_theme.get(name).unwrap_or(&empty);
        push_status_row(&mut out, name, counts);
    }

    // ⑸ ドメイン内で関連が閉じている束。
    out.push_str("\n## ドメイン内で関連が閉じている束\n\n");
    push_bundle_list(&mut out, &closed_bundles(ledger));

    out
}

/// 7 語彙を列に並べた表の見出しを書く。
///
/// 列の名前は [`Status::as_japanese`] から引き、[`Status::ALL`] の順に並べる
/// （要件 2.2 の語彙の順・要件 7.8 の呼び名）。ここで綴り直さない。
fn push_status_header(out: &mut String, first: &str) {
    out.push_str(&format!("| {first}"));
    for status in Status::ALL {
        out.push_str(&format!(" | {}", status.as_japanese()));
    }
    out.push_str(" | 合計 |\n| ---");
    // 1 列目を除く残り（7 語彙＋合計）はすべて右寄せ。
    for _ in 0..Status::ALL.len() + 1 {
        out.push_str(" | ---:");
    }
    out.push_str(" |\n");
}

/// 7 語彙を列に並べた表の 1 行を書く。
///
/// [`StatusCounts`] は 7 語彙すべてを持つので、0 件の列も欠けない。
fn push_status_row(out: &mut String, label: &str, counts: &StatusCounts) {
    out.push_str(&format!("| {label}"));
    for (_, count) in counts.rows() {
        out.push_str(&format!(" | {count}"));
    }
    out.push_str(&format!(" | {} |\n", counts.total()));
}

/// 並べるテーマ名を名前順で決める。
///
/// 引数のテーマ名と、台帳が実際に使った名前の**両方**を並べる。引数だけを見ると台帳に
/// 書かれた見知らぬ名前の件数が消え、台帳だけを見ると 1 件も使われていないテーマの
/// 欄が消える。
fn theme_names<'a>(
    by_theme: &'a BTreeMap<String, StatusCounts>,
    themes: &[&'a str],
) -> Vec<&'a str> {
    let mut names: BTreeSet<&str> = by_theme.keys().map(String::as_str).collect();
    for theme in themes {
        names.insert(theme);
    }
    names.into_iter().collect()
}

/// ⑶ 別名の一覧を書く。
///
/// 並びは id の byte 昇順（`entries` がその順で持っている）。
fn push_alias_list(out: &mut String, ledger: &Ledger) {
    let aliases: Vec<&crate::ledger::LedgerEntry> = ledger
        .entries
        .values()
        .filter(|entry| entry.status == Status::Alias)
        .collect();
    if aliases.is_empty() {
        out.push_str("別名の項目はありません。\n");
        return;
    }
    out.push_str("| 別名の id | 指す先の id |\n| --- | --- |\n");
    for entry in aliases {
        let target = entry
            .alias_of
            .as_ref()
            .map(EntryId::as_str)
            .unwrap_or(NO_ALIAS_TARGET);
        out.push_str(&format!("| {} | {} |\n", entry.id.as_str(), target));
    }
}

/// ⑸ 束の一覧を書く。並びは束 id の昇順（[`bundles`] が並べている）。
fn push_bundle_list(out: &mut String, closed: &[Bundle]) {
    if closed.is_empty() {
        out.push_str("ドメイン内で閉じている束はありません。\n");
        return;
    }
    out.push_str("| 束 id | 構成 id |\n| --- | --- |\n");
    for bundle in closed {
        let members: Vec<&str> = bundle.members.iter().map(EntryId::as_str).collect();
        out.push_str(&format!(
            "| {} | {} |\n",
            bundle.id.as_str(),
            members.join(", ")
        ));
    }
}

/// この台帳の `links` から辺を集め、構成 id が全部この台帳に属する束だけを返す
/// （設計 D-11）。
///
/// 他ドメインへ伸びた束は `report/summary.md` の担当なので、ここでは丸ごと落とす。
/// 所属の判定はこの台帳の `entries` に鍵があるかどうかで行う。ドメイン別報告は割り当て
/// 表を受け取らない（設計 D-11）ため、手元にあるのは `entries` と前置きの `pages` の 2 つ
/// だけで、前置きは手書きで陳腐化しうる側なので採らない（集計も同じ理由で `entries` を
/// 正としている）。この物差しは要件 6.3・6.4 が緑である限り「このドメインに属する」と
/// 一致する——検査が赤の間だけ、まだ台帳に書かれていない同ドメインの id を指す束が
/// どちらの報告からも落ちる。
fn closed_bundles(ledger: &Ledger) -> Vec<Bundle> {
    bundles(&domain_links(ledger))
        .into_iter()
        .filter(|bundle| {
            bundle
                .members
                .iter()
                .all(|member| ledger.entries.contains_key(member))
        })
        .collect()
}

/// 台帳の `links` を辺の列にする。
///
/// **1 つの id から何本でも辺が出る**（要件 4.3 の関連は種別違いで何本でも書ける）。
/// 元 id を鍵にした表で集めると 2 本目以降が黙って落ち、繋がるはずの束が割れる。
///
/// `alias_of` と `supersedes` は辺にしない。束は `links` だけで作る（設計 D-11）。
/// 別名は ⑶ の一覧が受け持つ。
fn domain_links(ledger: &Ledger) -> Vec<(EntryId, EntryId)> {
    let mut edges: Vec<(EntryId, EntryId)> = Vec::new();
    for entry in ledger.entries.values() {
        for link in &entry.links {
            edges.push((entry.id.clone(), link.to.clone()));
        }
    }
    edges
}

#[cfg(test)]
#[path = "domain_tests.rs"]
mod tests;
