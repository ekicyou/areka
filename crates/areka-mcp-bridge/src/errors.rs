//! 中継が作る JSON-RPC のエラー。
//!
//! 行の形のエラー 2 つ（`-32700`・`-32600`）と、番号 `-1` の 3 つ。番号 `-1` の文は SSP の中継と
//! 同じ書き出しに、理由を英語（ASCII）で続ける。理由の文を組むのは呼び手。

use rmcp::model::{ErrorCode, ErrorData};
use serde_json::{Value, json};

/// 中継が自分で作るエラーの番号（SSP の中継と同じ）。
const BRIDGE_ERROR: ErrorCode = ErrorCode(-1);

/// JSON として読めない行への答え。どの要求か分からないので `id` は `null`。
///
/// rmcp の `JsonRpcError` は `id` が無いとき欄ごと省くので、行の形のエラー 2 つは
/// rmcp の型を通さずに JSON の値で組む（書くのは標準入出力の輸送）。
pub fn parse_error() -> Value {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}})
}

/// JSON として読めるが要求の形でない行への答え。`id` は `null`。
pub fn invalid_request() -> Value {
    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "Invalid Request"}})
}

/// areka が見つからない・待ち受けない設定。
pub fn server_not_available(reason: &str) -> ErrorData {
    ErrorData::new(
        BRIDGE_ERROR,
        format!("Server not available: {reason}"),
        None,
    )
}

/// 届けた要求に応答が無い（本文なし・途中で閉じた・上限の時間を越えた）。
pub fn no_response(reason: &str) -> ErrorData {
    ErrorData::new(
        BRIDGE_ERROR,
        format!("No response from server: {reason}"),
        None,
    )
}

/// 応答の本文が JSON として読めない。`head` は本文の頭。
pub fn invalid_json_response(status: u16, head: &str) -> ErrorData {
    ErrorData::new(
        BRIDGE_ERROR,
        format!("Invalid JSON response from server: HTTP {status} {head}"),
        None,
    )
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod tests;
