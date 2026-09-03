//! 副手続きの振り分けと使い方の表示。
//!
//! 振り分け表（`catalog`・`ledger-init`・`report`・`report-summary`・`check`・
//! `evidence`・`candidates`・`diff` の 8 つ）は後続タスクが埋める。現時点では
//! 副手続きが 1 つも繋がっていないので、どの引数でも [`Outcome::Usage`] を返す
//! ——黙って成功したことにはしない。

use crate::error::SurveyError;

pub mod generate;
pub mod inspect;

/// 副手続きを走らせた結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 副手続きが最後まで走った（終了コード 0）。
    Done,
    /// 引数が副手続きとして読めなかった（終了コード 2）。
    Usage,
}

/// 引数を副手続きへ振り分ける。
///
/// 実行ファイル名は呼び出し側が既に落としているものとする。
pub fn run(args: &[String]) -> Result<Outcome, SurveyError> {
    let _ = args;
    Ok(Outcome::Usage)
}

/// 使い方の本文（1 行）。
pub fn usage() -> String {
    "使い方: ukadoc-survey <副手続き>".to_string()
}
