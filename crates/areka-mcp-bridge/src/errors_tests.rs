//! 中継が作る JSON-RPC のエラー（`errors.rs`）のテスト: 番号・文の書き出し・`id` の形。

use serde_json::{Value, json};

use super::*;

/// 行の形のエラーは `id` の欄を `null` で持つ（欄が無い形ではない）。値の一致に加えて、
/// 書き出した 1 行の字面でも見る。
fn assert_line(line: Value, expected: Value) {
    assert_eq!(line, expected);
    let text = serde_json::to_string(&line).expect("エラーの行は JSON にできる");
    assert!(text.contains(r#""id":null"#), "{text}");
    assert!(!text.contains('\n'), "{text}");
}

/// JSON でない行への答え（要件 1.6）: 番号 `-32700`・`id` は `null`。
#[test]
fn parse_error_line_shape() {
    assert_line(
        parse_error(),
        json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}}),
    );
}

/// オブジェクトでない行・欄の形が違う行への答え（要件 1.7）: 番号 `-32600`・`id` は `null`。
#[test]
fn invalid_request_line_shape() {
    assert_line(
        invalid_request(),
        json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "Invalid Request"}}),
    );
}

/// 番号 `-1` の 3 つ: SSP と同じ書き出しに理由が続く（要件の細部 6）。`data` は付けない。
#[test]
fn bridge_errors_are_code_minus_one_with_the_ssp_prefixes() {
    let cases = [
        (
            server_not_available("reason"),
            "Server not available: reason",
        ),
        (
            no_response("timed out after 30s"),
            "No response from server: timed out after 30s",
        ),
        (
            invalid_json_response(500, "<html>"),
            "Invalid JSON response from server: HTTP 500 <html>",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.code.0, -1, "{text}");
        assert_eq!(error.message, text);
        assert_eq!(error.data, None, "{text}");
    }
}
