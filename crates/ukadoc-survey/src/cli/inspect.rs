//! 点検側の副手続き（`check`・`evidence`・`candidates`・`diff`）。
//!
//! 中身はタスク 6.3 が埋める。それまでは「まだ中身が繋がっていない」と告げて失敗する
//! ——黙って成功したことにはしないし、途中で止まりもしない（設計 Error Handling）。

use crate::error::SurveyError;

/// 台帳と正典とソースの食い違いを調べる（要件 5.5・6.3〜6.12・7.4）。
pub fn check() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired { name: "check" })
}

/// 項目ごとの証拠を並べる（要件 5.5）。
pub fn evidence() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired { name: "evidence" })
}

/// 手掛かりの候補を並べる（要件 5.8・5.9）。
pub fn candidates() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired { name: "candidates" })
}

/// 今のカタログと新しいスナップショットの差を並べる（要件 8.1〜8.3）。
pub fn diff() -> Result<(), SurveyError> {
    Err(SurveyError::NotWired { name: "diff" })
}
