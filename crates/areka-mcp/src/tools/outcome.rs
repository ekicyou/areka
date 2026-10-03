//! ツールの結果の 4 つの形（素の値・成功・失敗・画像つき）。

use crate::{ToolContent, ToolOutcome};

/// ⑴ 素の値（OK: なし・isError: false）。
pub fn value(text: impl Into<String>) -> ToolOutcome {
    ToolOutcome {
        content: vec![ToolContent::Text(text.into())],
        is_error: false,
    }
}

/// ⑵ 成功。付言が空なら本文 "OK"、あれば "OK:<付言>"（isError: false）。
///
/// 空で "OK" だけになるのは SSP の `sakurascript` の成功の本文に合わせるため。
pub fn ok(note: &str) -> ToolOutcome {
    if note.is_empty() {
        value("OK")
    } else {
        value(format!("OK:{note}"))
    }
}

/// ⑶ 失敗。本文 "NG:<理由>"（isError: true）。
pub fn ng(reason: impl AsRef<str>) -> ToolOutcome {
    ToolOutcome {
        content: vec![ToolContent::Text(format!("NG:{}", reason.as_ref()))],
        is_error: true,
    }
}

/// ⑷ 本文の後に画像（base64 済みの PNG・mimeType "image/png"）を 1 枚足す。
pub fn with_image(mut outcome: ToolOutcome, png_base64: String) -> ToolOutcome {
    outcome.content.push(ToolContent::Image {
        data: png_base64,
        mime_type: "image/png".to_string(),
    });
    outcome
}

#[cfg(test)]
#[path = "outcome_tests.rs"]
mod outcome_tests;
