//! SHIORI/3.0 要求の追加のヘッダの入れ物（線の値・解釈しない）。
//!
//! `SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType` の 4 つの値を、`Status` と同じく
//! 解釈しない文字列として運ぶ（DD-IT-6）。出どころの語彙は kanade が持ち、本クレートは
//! 書くだけ。後続の spec は欄を足す（例 `X-MCP-PassThru-*`）。

use crate::charset::Charset;
use crate::shiori3::{Method, ShioriRequest};

/// 追加のヘッダの入れ物（wire 値・解釈しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestHeaders<'a> {
    /// `SecurityLevel` ヘッダの値（例 `local`）。
    pub security_level: &'a str,
    /// `SecurityOrigin` ヘッダの値（例 `null`）。
    pub security_origin: &'a str,
    /// `Sender` ヘッダの値（例 `areka`）。`"SSP"` 等の詐称はしない。
    pub sender: &'a str,
    /// `SenderType` ヘッダの値（例 `internal`）。
    pub sender_type: &'a str,
}

impl<'a> RequestHeaders<'a> {
    /// ゴースト自身の要求の既定（`local`・`null`・`sender`・`internal`）。
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#SenderType_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1
    #[must_use]
    pub const fn internal(sender: &'a str) -> RequestHeaders<'a> {
        RequestHeaders {
            security_level: "local",
            security_origin: "null",
            sender,
            sender_type: "internal",
        }
    }
}

/// GET の Value つき応答（値と、応答の `SecurityLevel` の生の値・無ければ `None`）。
///
/// レベルは解釈しない（`local`／`external` の判定は kanade・要件 3.5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueReply {
    /// `Value` ヘッダの値。
    pub value: String,
    /// 応答の `SecurityLevel` ヘッダの生の値（前後の空白を落とした値）。
    pub security_level: Option<String>,
}

/// 32bit の仲介と同じプロセスの 2 つの経路が共に使う要求の組み立て（要件 2.8 を構造で満たす）。
#[must_use]
pub fn request_of<'a>(
    method: Method,
    id: &'a str,
    references: &'a [String],
    status: Option<&'a str>,
    headers: RequestHeaders<'a>,
    charset: Charset,
) -> ShioriRequest<'a> {
    ShioriRequest {
        method,
        id,
        references,
        headers,
        status,
        charset,
    }
}

#[cfg(test)]
#[path = "request_headers_tests.rs"]
mod tests;
