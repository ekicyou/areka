//! SHIORI/2.x の書き方（版の型・確かめの要求と判定）。純粋・決定的（記録は呼び手が出す）。
//!
//! 版の決め方は SSP の測り（研究記録 10.1 節）に合わせ、`GET Version` の答えの
//! 先頭の行の版の字だけで決める（状態の番号は見ない）。

use crate::charset::Charset;

/// 相手の SHIORI の版（接続のあいだ変わらない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShioriVersion {
    /// SHIORI/3.0 の書き方で話す（今と同じ・決められないときもこれ）。
    Shiori3,
    /// SHIORI/2.x の書き方で話す（細かい版は区別しない）。
    Shiori2,
}

impl ShioriVersion {
    /// 記録に書く綴り（"SHIORI/3.0"／"SHIORI/2.x"）。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ShioriVersion::Shiori3 => "SHIORI/3.0",
            ShioriVersion::Shiori2 => "SHIORI/2.x",
        }
    }
}

/// 確かめの結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionVerdict {
    /// 版を決めた。evidence は答えの先頭の行そのまま（例 "SHIORI/2.6 200 OK"）。
    Decided {
        version: ShioriVersion,
        evidence: String,
    },
    /// 決められない（答えが空・先頭の行に SHIORI/2. も SHIORI/3. も無い・往復の失敗）。
    Undecided { reason: String },
}

/// 確かめの要求（GET Version SHIORI/2.6 / Charset / Sender / 空行・CRLF）。
///
/// 全文を `charset` の 1 本の経路で符号化する（`Sender` が ASCII の外でも同じ）。
#[must_use]
pub fn version_probe_request(charset: Charset, sender: &str) -> Vec<u8> {
    let text = format!(
        "GET Version SHIORI/2.6\r\nCharset: {}\r\nSender: {sender}\r\n\r\n",
        charset.name()
    );
    charset.encode(&text).0.into_owned()
}

/// 答えの先頭の行の最初の字句で版を決める（状態の番号は見ない）。
///
/// 状態の行は ASCII の範囲なので、復号の前のバイトのまま読める。
#[must_use]
pub fn decide_version(reply: &[u8]) -> VersionVerdict {
    let first = reply.split(|&b| b == b'\n').next().unwrap_or_default();
    let line = String::from_utf8_lossy(first).trim().to_string();
    let token = line.split_whitespace().next().unwrap_or_default();
    let version = if token.starts_with("SHIORI/2.") {
        ShioriVersion::Shiori2
    } else if token.starts_with("SHIORI/3.") {
        ShioriVersion::Shiori3
    } else if line.is_empty() {
        return VersionVerdict::Undecided {
            reason: "empty_status_line".to_string(),
        };
    } else {
        return VersionVerdict::Undecided {
            reason: format!("unknown_version: {line}"),
        };
    };
    VersionVerdict::Decided {
        version,
        evidence: line,
    }
}

#[cfg(test)]
#[path = "shiori2_tests.rs"]
mod tests;
