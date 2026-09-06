//! `report/summary.md` の本文（要件 7.2・設計 D-11）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。カタログと
//! 台帳 4 本と証拠の索引とテーマ名を受け取り、文字列を返す。書き出しは入出力層
//! （`io::files::write_lf`）の担当である。
//!
//! # 載せるのは要件 7.2 の 5 項目
//!
//! ⑴ 冒頭にカタログのスナップショット生成日時と、各台帳の項目数・未分類件数
//! ⑵ 状態の分布（全体・ドメイン別）⑶ ドメインを跨いで繋がった束の一覧
//! ⑷ テーマ別の状態分布（全体）⑸ ドメインごとの証拠あり件数。順番も設計の列挙
//! どおりに固定する。数えるところは [`super::tally`]、束を作るところは
//! [`super::bundle`] が受け持ち、ここは並べるだけ——同じ数を 2 か所で数えない。
//!
//! # 時刻はカタログから来る 1 つだけ（要件 7.3）
//!
//! 本文に現れる時刻は [`crate::catalog::SnapshotMeta::generated_at`] のただ 1 つで、
//! これは**読み込んだ値**であって壁時計ではない。時計を読む呼び出しはここに 1 つも
//! 無い（あると同じ入力から 2 回作った本文が食い違い、要件 7.3 が破れる）。
//!
//! # 証拠は件数だけ・場所は書かない（要件 2.3・設計 D-11）
//!
//! ドメイン別報告は証拠を載せない（載せるとソース側の変更で他 spec の報告が古くなり、
//! 4 本の独立性が壊れる）。要件 2.3 の「報告には証拠の有無だけを載せる」は、常時検査の
//! 対象外であるこの報告（要件 7.6）にドメインごとの件数を載せることで満たす。
//! **ファイルパスは 1 つも書かない**——どのファイルに書かれているかを示すのは検査の
//! 出力の役目である（要件 5.5）。
//!
//! # 束の辺は `links` だけから作る（設計 D-11）
//!
//! `alias_of` と `supersedes` は辺にしない。ここには申し送りの制約がある——要件付録
//! A.2 は `alias_of`／`supersedes` を専用の欄にだけ書くことを許すので、`supersedes`
//! だけで書かれた対はドメイン別報告にもこの報告にも現れない（`alias_of` だけの対は
//! ドメイン別報告の別名の一覧に出る）。**束に出したい繋がりは `links` にも書く**
//! ことを README で案内する前提で、規則はドメイン別報告と揃えてある。
//!
//! # 並びは 2 通りに決め打つ（設計 report 節の事後条件）
//!
//! - **状態**は要件 2.2 の語彙の順（[`Status::ALL`] の並び）。
//! - **ドメイン・テーマ・束 id**は名前順（UTF-8 のバイト順であって五十音順ではない）。
//!
//! 台帳は渡された順に頼らない。`ledgers` がどの順で来ても本文は同じになる。
//!
//! # 呼び名は 1 か所でしか綴らない
//!
//! 状態の平易な日本語（要件 7.8）は [`Status::as_japanese`] が正本で、ここでは綴り
//! 直さない。ただし冒頭の表の列見出し「未分類」だけは字面として書いている——状態の
//! 描画ではなく列の名前だからだが、`as_japanese` の綴りが変わればここも直すこと。
//! 改行は `\n` だけを使う（設計 D-6）。

use std::collections::{BTreeMap, BTreeSet};

use super::bundle::{Bundle, bundles};
use super::tally::{StatusCounts, Tally, status_counts, tally};
use crate::catalog::Catalog;
use crate::evidence::EvidenceIndex;
use crate::ledger::Ledger;
use crate::model::{EntryId, Status};

/// 束の構成 id がどの台帳にも無いときの、そのドメインの呼び名。
///
/// ドメイン別報告は「構成 id が全部その台帳に属する束」だけを載せるので、どの台帳にも
/// 無い id を含む束はどのドメインの報告からも落ちる。ここで落とすとその束はどこにも
/// 現れないので、載せたうえで所属が無いことを書く。この id 自体を赤にするのは検査層で、
/// 担当は要件 6.3・6.4（カタログとの対応）と要件 6.7（関連の相手の実在）である。
const NO_LEDGER: &str = "（台帳に無い）";

/// 全体報告の本文を組み立てる（要件 7.2・設計 D-11）。
///
/// `themes` は報告に並べるテーマ名（要件 4.4 の 8 つ）。どの台帳も使っていないテーマも
/// 0 件の行として並べる——欄ごと消すと「0 件だった」と「そもそも数えていない」が
/// 読み手には区別できない。台帳にだけ現れた名前も落とさず並べる。
///
/// `ledgers` は 4 本を想定するが、順も本数も問わない。並べる順はドメイン名の名前順で
/// ここが決める。同じドメインの台帳が 2 本渡されたら 2 行として並べる（本来は整合検査が
/// 赤にする形なので、ここで黙って畳まない）。
pub fn render_summary(
    catalog: &Catalog,
    ledgers: &[Ledger],
    evidence: &EvidenceIndex,
    themes: &[&str],
) -> String {
    let ordered = ordered_ledgers(ledgers);
    let tallies: Vec<(&Ledger, Tally)> = ordered
        .iter()
        .map(|ledger| (*ledger, tally(ledger)))
        .collect();
    let owner = owning_domain(&ordered);

    let mut out = String::new();
    out.push_str("# ukadoc 網羅状況の全体報告\n\n");
    out.push_str(
        "この本文はカタログと台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。\n",
    );
    // 4 台帳を跨ぐ成果物なので常時検査の合否には入れない（要件 7.6）。
    out.push_str(
        "4 本の台帳を跨ぐ報告なので、新しさは常時検査の合否に入れません。作り直すのは統合担当です。\n",
    );

    // ⑴ 冒頭——スナップショットの生成日時。本文で時刻を含むのはこの 1 行だけ。
    out.push_str("\n## 元にしたカタログ\n\n");
    out.push_str(&format!(
        "スナップショットの生成日時は {} です。この報告がどれだけ古いかは、この日時で読みます。\n",
        catalog.snapshot.generated_at
    ));

    // ⑴ 冒頭——各台帳の項目数と未分類件数。合計 1 行に畳まない。
    out.push_str("\n## 台帳ごとの項目数と未分類件数\n\n");
    out.push_str("| ドメイン | 項目数 | 未分類 |\n| --- | ---: | ---: |\n");
    for (ledger, counted) in &tallies {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            ledger.domain.as_key(),
            ledger.entries.len(),
            counted.overall.get(Status::Unclassified)
        ));
    }

    // ⑵ 状態の分布——全体。
    let overall = status_counts(ordered.iter().flat_map(|ledger| ledger.entries.values()));
    out.push_str("\n## 状態の分布\n\n");
    out.push_str("| 状態 | 件数 |\n| --- | ---: |\n");
    for (name, count) in overall.japanese_rows() {
        out.push_str(&format!("| {name} | {count} |\n"));
    }
    out.push_str(&format!("| 合計 | {} |\n", overall.total()));

    // ⑵ 状態の分布——ドメイン別。
    out.push_str("\n## ドメイン別の状態の分布\n\n");
    push_status_header(&mut out, "ドメイン");
    for (ledger, counted) in &tallies {
        push_status_row(&mut out, ledger.domain.as_key(), &counted.overall);
    }

    // ⑶ ドメインを跨いで繋がった束。
    out.push_str("\n## ドメインを跨いで繋がった束\n\n");
    out.push_str(
        "ドメインの中で閉じている束は、それぞれのドメインの報告に載ります。ここに並ぶのは跨いだものだけです。\n\n",
    );
    push_bundle_list(&mut out, &crossing_bundles(&ordered, &owner));

    // ⑷ テーマ別の状態分布——全体。
    let by_theme = merged_themes(&tallies);
    out.push_str("\n## テーマ別の状態分布\n\n");
    push_status_header(&mut out, "テーマ");
    for name in theme_names(&by_theme, themes) {
        let empty = StatusCounts::new();
        let counts = by_theme.get(name).unwrap_or(&empty);
        push_status_row(&mut out, name, counts);
    }

    // ⑸ ドメインごとの証拠あり件数（要件 2.3 の「有無だけ」）。
    out.push_str("\n## ドメインごとの証拠あり件数\n\n");
    out.push_str(
        "載せるのは件数だけです。どのファイルに書かれているかは検査の出力が示します。\n\n",
    );
    out.push_str("| ドメイン | 証拠あり |\n| --- | ---: |\n");
    for (ledger, _) in &tallies {
        out.push_str(&format!(
            "| {} | {} |\n",
            ledger.domain.as_key(),
            evidence_count(ledger, evidence)
        ));
    }

    out
}

/// 台帳をドメイン名の名前順に並べ替える。
///
/// 渡された順は答えに残さない（要件 7.3。生成の副手続きが何順で渡しても本文は同じ）。
/// 名前順は要件 3.1 の並び（[`crate::model::Domain::ALL`]）とは違う——`assets` が
/// 先頭に来て `shiori` が最後になる。
fn ordered_ledgers(ledgers: &[Ledger]) -> Vec<&Ledger> {
    let mut ordered: Vec<&Ledger> = ledgers.iter().collect();
    ordered.sort_by_key(|ledger| ledger.domain.as_key());
    ordered
}

/// 項目 id からその id を持つ台帳のドメイン名を引く表。
///
/// 同じ id が 2 本の台帳にあるときは名前順で先の台帳を採る（本来は整合検査が赤にする
/// 形なので、ここでは黙って片方を採り、判定は作り直さない）。
fn owning_domain<'a>(ordered: &[&'a Ledger]) -> BTreeMap<&'a str, &'static str> {
    let mut owner: BTreeMap<&str, &'static str> = BTreeMap::new();
    for ledger in ordered {
        for id in ledger.entries.keys() {
            owner.entry(id.as_str()).or_insert(ledger.domain.as_key());
        }
    }
    owner
}

/// 7 語彙を列に並べた表の見出しを書く。
///
/// 列の名前は [`Status::as_japanese`] から引き、[`Status::ALL`] の順に並べる
/// （要件 2.2 の語彙の順・要件 7.8 の呼び名）。ここで綴り直さない。
///
/// 表の形はドメイン別報告と揃えているが、版面はそれぞれの報告が自分で持つ。片方の
/// 版面を変えたときにもう片方が黙って動かないようにするためである（ドメイン別報告の
/// 版面は要件 7.4 の新しさの検査がバイト単位で固定しており、この報告は検査の対象外）。
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

/// 台帳ごとのテーマ別集計を足し合わせる。
///
/// どの項目がどのテーマに数えられるかの規則は [`tally`] が持つ。ここで台帳の項目を
/// 数え直すと同じ規則が 2 か所に散るので、出来上がった分布どうしを足す。
/// [`StatusCounts`] は 1 件ずつ数える口しか持たないので、件数の分だけ足している
/// （台帳の項目数は数千件までなので、この数え直しは目に見える手間にならない）。
fn merged_themes(tallies: &[(&Ledger, Tally)]) -> BTreeMap<String, StatusCounts> {
    let mut merged: BTreeMap<String, StatusCounts> = BTreeMap::new();
    for (_, counted) in tallies {
        for (theme, counts) in &counted.by_theme {
            let target = merged.entry(theme.clone()).or_default();
            for (status, count) in counts.rows() {
                for _ in 0..*count {
                    target.add(*status);
                }
            }
        }
    }
    merged
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

/// ⑶ 束の一覧を書く。並びは束 id の昇順（[`bundles`] が並べている）。
fn push_bundle_list(out: &mut String, crossing: &[(Bundle, Vec<&str>)]) {
    if crossing.is_empty() {
        out.push_str("ドメインを跨いだ束はありません。\n");
        return;
    }
    out.push_str("| 束 id | 跨ぐドメイン | 構成 id |\n| --- | --- | --- |\n");
    for (bundle, domains) in crossing {
        let members: Vec<&str> = bundle.members.iter().map(EntryId::as_str).collect();
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            bundle.id.as_str(),
            domains.join(", "),
            members.join(", ")
        ));
    }
}

/// 4 台帳すべての `links` から辺を集め、1 つの台帳の中で閉じていない束だけを返す
/// （設計 D-11）。
///
/// 添えるのはその束が跨ぐドメイン名の名前順の一覧。構成 id がどの台帳にも無ければ
/// [`NO_LEDGER`] を並べる——その束はドメイン別報告からも落ちるので、ここが落とすと
/// どの報告にも現れなくなる。
fn crossing_bundles<'a>(
    ordered: &[&Ledger],
    owner: &BTreeMap<&str, &'a str>,
) -> Vec<(Bundle, Vec<&'a str>)> {
    bundles(&all_links(ordered))
        .into_iter()
        .filter_map(|bundle| {
            let domains: BTreeSet<&str> = bundle
                .members
                .iter()
                .map(|member| *owner.get(member.as_str()).unwrap_or(&NO_LEDGER))
                .collect();
            // 1 つの台帳の中で閉じている束はそのドメインの報告の担当。
            if domains.len() < 2 {
                return None;
            }
            Some((bundle, domains.into_iter().collect()))
        })
        .collect()
}

/// 4 台帳の `links` を辺の列にする。
///
/// **1 つの id から何本でも辺が出る**（要件 4.3 の関連は種別違いで何本でも書ける）。
/// 元 id を鍵にした表で集めると 2 本目以降が黙って落ち、繋がるはずの束が割れる。
///
/// `alias_of` と `supersedes` は辺にしない。束は `links` だけで作る（設計 D-11）。
fn all_links(ordered: &[&Ledger]) -> Vec<(EntryId, EntryId)> {
    let mut edges: Vec<(EntryId, EntryId)> = Vec::new();
    for ledger in ordered {
        for entry in ledger.entries.values() {
            for link in &entry.links {
                edges.push((entry.id.clone(), link.to.clone()));
            }
        }
    }
    edges
}

/// その台帳の項目のうち、正典 URL がソースに 1 件以上置かれているものの件数
/// （要件 2.3 の「有無だけ」）。
///
/// 数えるのは**この台帳にある id** だけである。索引には台帳に無い id の証拠も入り
/// うる（綴りの誤りなど）ので、索引の大きさをそのまま載せない。
fn evidence_count(ledger: &Ledger, evidence: &EvidenceIndex) -> usize {
    ledger
        .entries
        .keys()
        .filter(|id| matches!(evidence.by_id.get(*id), Some(paths) if !paths.is_empty()))
        .count()
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
