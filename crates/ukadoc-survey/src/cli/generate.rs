//! 生成側の副手続き（`catalog`・`ledger-init`・`report`・`report-summary`）。
//!
//! 中身はタスク 6.2 が埋める。それまでは「まだ中身が繋がっていない」と告げて失敗する
//! ——黙って成功したことにはしないし、途中で止まりもしない（設計 Error Handling）。

use crate::error::SurveyError;

/// 正典のカタログを作り直す（要件 1.1〜1.9）。
pub fn catalog() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired { name: "catalog" })
}

/// 初期の台帳を作って既存の台帳へ差し込む（要件 3.3・3.3a）。
pub fn ledger_init() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired {
        name: "ledger-init",
    })
}

/// ドメイン別の報告 4 本を作り直す（要件 7.1・7.3）。
pub fn report() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired { name: "report" })
}

/// 全体の報告を作り直す（要件 7.2・7.3）。
pub fn report_summary() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired {
        name: "report-summary",
    })
}
