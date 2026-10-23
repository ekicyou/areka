//! SHIORI/2.x の書き方（版の型・確かめの要求と判定・組み替えの表と要求の組み立て・答えの読み）。純粋・決定的（記録は呼び手が出す）。
//!
//! 版の決め方は SSP の測り（研究記録 10.1 節）に合わせ、`GET Version` の答えの
//! 先頭の行の版の字だけで決める（状態の番号は見ない）。

use crate::charset::{Charset, CharsetPolicy};
use crate::error::ShioriError;
use crate::shiori3::{
    ABSENT_REFERENCE, EncodedRequest, Method, ParsedResponse, ShioriRequest, parse_response_with,
};

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

/// 2.x の要求の経路（組み替えの表の行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// `GET Sentence SHIORI/2.2`（台本を求めるイベント・通知だけのイベント）。
    Sentence,
    /// `GET String SHIORI/2.5`（資源の問い合わせ）。
    String,
    /// `TRANSLATE Sentence SHIORI/2.6`（台本の翻訳）。
    Translate,
}

/// 組み替えの表のコードの 1 か所（研究記録 10.2 節）。`None` は「2.x の相手へは送らない」。
///
/// 資源かどうかは名前の形でなく、呼び手（kanade の正本の一覧）が付けた印で決める。
fn route(method: Method, id: &str, is_resource: bool) -> Option<Route> {
    match method {
        Method::Get if id == "OnTranslate" => Some(Route::Translate),
        Method::Get if is_resource => Some(Route::String),
        Method::Get => Some(Route::Sentence),
        // SSP も 2.x の相手へ送らなかった（測 5）。
        Method::Notify if id == "basewareversion" => None,
        Method::Notify => Some(Route::Sentence),
    }
}

/// 3.0 の要求の材料から 2.x の要求を組む。None は「2.x の相手へは送らない」。
/// is_resource は呼び手（kanade の正本の一覧）が決めた「資源の問い合わせか」。
///
/// 見出しの並びは設計の「2.x の要求の形」のとおり。`SenderType` はどの要求にも書かない
/// （SSP も書かない・測 6）。全文を `req.charset` の 1 本の経路で符号化する（3.0 と同じ）。
#[must_use]
pub fn build_request(req: &ShioriRequest<'_>, is_resource: bool) -> Option<EncodedRequest> {
    let route = route(req.method, req.id, is_resource)?;
    let h = &req.headers;
    let mut out = String::from(match route {
        Route::Sentence => "GET Sentence SHIORI/2.2\r\n",
        Route::String => "GET String SHIORI/2.5\r\n",
        Route::Translate => "TRANSLATE Sentence SHIORI/2.6\r\n",
    });
    let mut line = |name: &str, value: &str| {
        out.push_str(name);
        out.push_str(": ");
        out.push_str(value);
        out.push_str("\r\n");
    };
    line("Charset", req.charset.name());
    line("SecurityLevel", h.security_level);
    // `SecurityOrigin` は `GET String`・`TRANSLATE` にだけ付く（測 6）。
    if route != Route::Sentence {
        line("SecurityOrigin", h.security_origin);
    }
    line("Sender", h.sender);
    match route {
        Route::Sentence => line("Event", req.id),
        Route::String => line("ID", req.id),
        Route::Translate => {}
    }
    if let Some(status) = req.status {
        line("Status", status);
    }
    if route == Route::Translate {
        // 翻訳する台本は 3.0 の Reference0。References は書かない（SSP も書かない）。
        line(
            "Sentence",
            req.references.first().map_or("", String::as_str),
        );
    } else {
        for (n, reference) in req.references.iter().enumerate() {
            // 欠番の印の位置は行を出さず、番号だけ進める（3.0 と同じ）。
            if reference != ABSENT_REFERENCE {
                line(&format!("Reference{n}"), reference);
            }
        }
    }
    out.push_str("\r\n");
    let (bytes, replaced) = req.charset.encode(&out);
    Some(EncodedRequest {
        bytes: bytes.into_owned(),
        replaced,
    })
}

/// 2.x の答えを読む。要求の種類で値の見出し（`Sentence`／`String`）を選び、
/// 2.x に無い見出し（`ErrorLevel`・`ErrorDescription`・`SecurityLevel`）は読まない。
///
/// 状態の番号・`Charset` の拾い方・読めない応答の判定は 3.0 と同じ（`parse_response_with`）。
/// is_resource は [`build_request`] に渡したものと同じ印（資源なら `String` を読む）。
pub fn parse_response(
    bytes: &[u8],
    policy: CharsetPolicy,
    method: Method,
    id: &str,
    is_resource: bool,
) -> Result<ParsedResponse, ShioriError> {
    // 送らない要求（route が None）の答えは来ないが、来ても台本と同じに読む。
    let value_header = match route(method, id, is_resource) {
        Some(Route::String) => "String",
        _ => "Sentence",
    };
    let parsed = parse_response_with(bytes, policy, value_header)?;
    Ok(ParsedResponse {
        error_level: None,
        error_description: None,
        security_level: None,
        ..parsed
    })
}

#[cfg(test)]
#[path = "shiori2_tests.rs"]
mod tests;
