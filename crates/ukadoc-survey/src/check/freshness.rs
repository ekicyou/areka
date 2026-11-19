//! 新しさの検査——報告 5 本と台帳の突き合わせ（要件 7.4・7.5）。
//!
//! 設計 check 節の「判定の内訳」から、この段が受け持つのは 1 種だけである。
//!
//! - `DomainReportStale`（7.4・7.5）——ドメイン別報告 4 本か全体報告 `summary.md` が、
//!   台帳から作り直した本文と一致しない
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。失敗も
//! しない——食い違いは [`Finding`] として全部集めて返し、1 件目で止めない（設計
//! Error Handling）。
//!
//! # 突き合わせ相手は入力から作り直す（設計 D-11）
//!
//! ドメイン別報告は [`render_domain`] の出力、つまりその台帳 1 本と
//! [`CheckInput::themes`] だけの関数と比べる。カタログも証拠も他の台帳も使わないので、
//! 隣の台帳を直しても自分のドメインの報告は古くならない。
//!
//! 全体報告は [`render_summary`] の出力、つまりカタログと台帳 4 本とテーマ名から作った
//! 本文と比べる。証拠の件数は載せないので、ソースに正典 URL を 1 行足しても古くならない。
//! 台帳をどれか 1 本直せば全体報告も古くなるので、`report` と `report-summary` の両方を
//! 走らせ直す。
//!
//! # 復帰文字は**保存されている側だけ**落とす（設計 D-6）
//!
//! この repo に `.gitattributes` は無く `core.autocrlf` が効くので、新しく clone した
//! 作業ツリーの報告は復帰文字付きで取り出される。入出力層の読み込み
//! （`io::files::read_normalized`）はそこで復帰文字を落とすが、[`CheckInput`] を手で
//! 組む呼び手（在中テストや将来の入口）は復帰文字付きの本文を渡しうる。だから突き
//! 合わせの側でも落とす。規則を 2 か所に写さないよう、落とすのは
//! [`strip_cr`]——`&str` → `String` の純粋な関数で、ファイルには触らない。
//!
//! 落とすのは**保存されている側だけ**である。[`render_domain`] と [`render_summary`] は
//! 改行だけを書くと決まっており（設計 D-6。`report` の在中テストが釘付けしている）、
//! 書き出しも `io::files::write_lf` が改行だけにする。作り直した側まで落とすと、報告の生成が復帰
//! 文字を混ぜ始めたときにこの検査がそれを隠してしまう——`render_domain` に復帰文字を
//! 1 つ入れる実験で確かめた（保存側だけ落とす形は赤くなり、両側を落とす形は緑に
//! なった）。
//!
//! # 行番号はどこにも出ない
//!
//! 所見が言うのは「どの報告を作り直すか」だけで、本文のどこが食い違ったかは
//! 言わない。要件 6.11 の「整理では壊れない」と同じ向きで（この spec 群の裁定
//! 「備考に行番号を書かない」も同じ向き）、行番号以外の「ここが違う」という言い方は
//! 本文の整理で動いてしまう。直し方は 1 つしかない——手で直さず作り直す（要件 7.7）
//! ので、場所さえ判れば所見として用は足りる。

use super::{CheckInput, Finding, FindingKind};
use crate::io::files::strip_cr;
use crate::ledger::Ledger;
use crate::model::Domain;
use crate::report::domain::render_domain;
use crate::report::summary::render_summary;

/// その報告の場所（所見の「場所」に載る綴り）。区切りは `/` で、OS の区切りは使わない。
fn report_file(domain: Domain) -> String {
    format!("doc/ukadoc-coverage/report/{}.md", domain.as_key())
}

/// 全体報告の場所（所見の「場所」に載る綴り）。
const SUMMARY_FILE: &str = "doc/ukadoc-coverage/report/summary.md";

/// 報告の古さを集める。
///
/// 並びは [`Domain::ALL`] の順のドメイン別報告、最後に全体報告である。台帳を渡された
/// 順に見ると、同じ入力を並べ替えただけで所見の並びが変わってしまう（要件 7.3 の
/// 決まり方を検査の出力にも通す）。同じドメインの台帳が 2 本渡された場合は 2 本とも
/// 同じ報告と突き合わせる——黙って片方を落とさない。
///
/// **台帳の無いドメインは見ない。** 作り直す元が無いので「古い」とは言えないからで、
/// 台帳が欠けていること自体は構造の検査が `CatalogIdMissingFromLedgers` として拾う
/// （要件 6.4）。
pub fn check(input: &CheckInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    for domain in Domain::ALL {
        for ledger in input.ledgers.iter().filter(|led| led.domain == domain) {
            check_one(input, ledger, &mut findings);
        }
    }
    if strip_cr(input.summary_report) != render_summary(input.catalog, input.ledgers, input.themes)
    {
        findings.push(Finding::new(
            FindingKind::DomainReportStale,
            None,
            SUMMARY_FILE.to_owned(),
            "全体報告がカタログと台帳から作り直した本文と一致しない。手で直さず `report-summary` で作り直すこと".to_owned(),
        ));
    }
    findings
}

/// 報告 1 本を台帳 1 本と突き合わせる（要件 7.4・7.5）。
///
/// 主語は項目ではなくドメインなので id は付かない（設計 finding 節。作り物の id を
/// 付けるとかえって読めなくなる）。
fn check_one(input: &CheckInput, ledger: &Ledger, findings: &mut Vec<Finding>) {
    let domain = ledger.domain;
    let Some(stored) = input.domain_reports.get(&domain) else {
        // 本文が渡されていないなら、一致しているとは言えない。黙って飛ばすと報告
        // 1 本が丸ごと無い入力が緑になる（要件 7.5 は「どのドメインの再生成が要るか」を
        // 求めるので、無い側もその答えを言える形にしておく）。
        findings.push(Finding::new(
            FindingKind::DomainReportStale,
            None,
            report_file(domain),
            format!(
                "{} の報告の本文が渡されていない。作り直すこと",
                domain.as_key()
            ),
        ));
        return;
    };

    if strip_cr(stored) == render_domain(ledger, input.themes) {
        return;
    }
    findings.push(Finding::new(
        FindingKind::DomainReportStale,
        None,
        report_file(domain),
        format!(
            "{} の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと",
            domain.as_key()
        ),
    ));
}

#[cfg(test)]
#[path = "freshness_tests.rs"]
mod tests;
