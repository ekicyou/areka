//! 副手続きの振り分けと使い方の表示（設計「入口 / cli」・要件 6.12）。
//!
//! 引数の解析は自前で行う。副手続きは `catalog`・`ledger-init`・`report`・
//! `report-summary`・`check`・`evidence`・`candidates`・`diff` の 8 つで固定なので、
//! 引数解析の外部ライブラリは入れない（設計 Technology Stack）。
//!
//! # 振り分けは表 1 本
//!
//! 名前と中身の組は [`SUBCOMMANDS`] という 1 本の表に載っており、[`run`] はその表を
//! 引くだけである。手書きの分岐を別に持たないので、表と振り分けが食い違う余地が無い。
//!
//! # 出力先と終了コード
//!
//! - 標準出力に出るのは**結果だけ**。使い方も、断りの本文も、失敗の本文も標準エラーへ
//!   出す（結果を別のファイルへ流し込んでも断りが混ざらないようにするため）。
//! - 終了コードは 0 = 成功・1 = 失敗・2 = 使い方の誤り（設計「Error Handling / 見張り」）。
//!   実際に終了コードを返すのは実行ファイル側（`main.rs`）で、ここは [`Outcome`] と
//!   `Err` のどちらを返すかだけを決める。
//!
//! 知らない名前と余計な引数は、断りの 1 行を標準エラーへ出してから [`Outcome::Usage`] を
//! 返す。呼び手（`main.rs`）が続けて使い方を出すので、利用者は「何が違ったか」と
//! 「どう打てばよいか」を続けて読める（要件 6.12）。

use std::io::Write;

use crate::error::SurveyError;

pub mod generate;
pub mod inspect;

/// 副手続きの中身。引数は取らない（設計「入口 / cli」の表）。
type Handler = fn() -> Result<(), SurveyError>;

/// 名前と中身の組。
pub(crate) struct Subcommand {
    /// 利用者が打つ名前。
    pub(crate) name: &'static str,
    /// その名前で走る中身。
    pub(crate) handler: Handler,
}

/// 振り分け表（設計「入口 / cli」の表と同じ 8 つ・同じ並び）。
pub(crate) const SUBCOMMANDS: [Subcommand; 8] = [
    Subcommand {
        name: "catalog",
        handler: generate::catalog,
    },
    Subcommand {
        name: "ledger-init",
        handler: generate::ledger_init,
    },
    Subcommand {
        name: "report",
        handler: generate::report,
    },
    Subcommand {
        name: "report-summary",
        handler: generate::report_summary,
    },
    Subcommand {
        name: "check",
        handler: inspect::check,
    },
    Subcommand {
        name: "evidence",
        handler: inspect::evidence,
    },
    Subcommand {
        name: "candidates",
        handler: inspect::candidates,
    },
    Subcommand {
        name: "diff",
        handler: inspect::diff,
    },
];

/// 副手続きを走らせた結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 副手続きが最後まで走った（終了コード 0）。
    Done,
    /// 引数が副手続きとして読めなかった（終了コード 2）。
    Usage,
}

/// 名前から副手続きを引く。綴りは大文字小文字も区切りもそのまま比べる。
pub(crate) fn lookup(name: &str) -> Option<&'static Subcommand> {
    SUBCOMMANDS.iter().find(|sub| sub.name == name)
}

/// 引数を副手続きへ振り分ける。断りの本文は標準エラーへ出す。
///
/// 実行ファイル名は呼び出し側が既に落としているものとする。
pub fn run(args: &[String]) -> Result<Outcome, SurveyError> {
    run_reporting_to(args, &mut std::io::stderr())
}

/// [`run`] の中身。断りの本文の行き先を受け取る（テストが本文を読めるようにするため）。
pub(crate) fn run_reporting_to<W: Write>(
    args: &[String],
    notices: &mut W,
) -> Result<Outcome, SurveyError> {
    let Some(name) = args.first() else {
        // 引数が無いのは「打ち方が分からない」だけなので、断ることは無い。
        return Ok(Outcome::Usage);
    };
    let Some(sub) = lookup(name) else {
        write_notice(notices, &unknown_subcommand_notice(name))?;
        return Ok(Outcome::Usage);
    };
    let rest = &args[1..];
    if !rest.is_empty() {
        write_notice(notices, &extra_arguments_notice(sub.name, rest))?;
        return Ok(Outcome::Usage);
    }
    (sub.handler)()?;
    Ok(Outcome::Done)
}

/// 断りの 1 行を書き出す。書けなかったことも黙って捨てない。
fn write_notice<W: Write>(notices: &mut W, body: &str) -> Result<(), SurveyError> {
    writeln!(notices, "{body}").map_err(|err| SurveyError::Io {
        path: "<標準エラー>".to_string(),
        reason: err.to_string(),
    })
}

/// 知らない名前を打たれたときの断り（打たれた綴りをそのまま映す）。
pub(crate) fn unknown_subcommand_notice(name: &str) -> String {
    format!("知らない副手続き: {name}")
}

/// 名前の後ろに余計な引数が付いていたときの断り。
pub(crate) fn extra_arguments_notice(name: &str, rest: &[String]) -> String {
    format!(
        "副手続き {name} は引数を取らないのに、余計な引数がある: {}",
        rest.join(" ")
    )
}

/// 使い方の本文（末尾に改行は付けない。出すのは呼び手）。
pub fn usage() -> String {
    "使い方: cargo run -p ukadoc-survey -- <副手続き>

副手続きは 8 つ。いずれも引数を取らない。

  catalog         正典のカタログを作り直す（スナップショットが要る）
  ledger-init     初期の台帳を作って既存の台帳へ差し込む
  report          ドメイン別の報告 4 本を作り直す
  report-summary  全体の報告を作り直す
  check           台帳と正典とソースの食い違いを調べる
  evidence        項目ごとの証拠を並べる
  candidates      手掛かりの候補を並べる
  diff            今のカタログと新しいスナップショットの差を並べる（スナップショットが要る）

スナップショットの場所は環境変数 AREKA_UKADOC_SNAPSHOT で指定できる。"
        .to_string()
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
