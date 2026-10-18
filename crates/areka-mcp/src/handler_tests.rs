//! `custom_request_error` の決定論テスト（要件 1.1・1.3・1.4・1.5・1.6）。ソケットは開かない。
//! 記録はテストのスレッドで呼ぶ純粋な関数から出るので `capture` で数える。

use log_capture_kit::capture;
use rmcp::model::ErrorCode;
use tracing::Level;

use super::custom_request_error;

/// (要件 1.1, 1.4, 1.5) 表の 3 つは `-32602`・文は `invalid params for <m>`・`data` なし・debug ちょうど 1 件。
#[test]
fn params_required_methods_answer_invalid_params_with_one_debug_record() {
    for m in ["tools/call", "initialize", "completion/complete"] {
        let (got, events) = capture(|| custom_request_error(m));
        assert_eq!(got.code, ErrorCode::INVALID_PARAMS, "{m}");
        assert_eq!(got.message, format!("invalid params for {m}"), "{m}");
        assert!(got.data.is_none(), "{m}: {:?}", got.data);
        assert_eq!(events.len(), 1, "{m}: {events:?}");
        assert_eq!(events[0].level, Level::DEBUG, "{m}");
        assert_eq!(events[0].field_str("method"), Some(m), "{m}");
        assert_eq!(
            events[0].message(),
            "MCP: params の形がメソッドに合わない",
            "{m}"
        );
    }
}

/// (要件 1.3, 1.6) 表の外は rmcp の既定と同じ `-32601`・文はメソッドの名前・`data` なし・記録 0 件。
#[test]
fn other_methods_keep_method_not_found_without_records() {
    for m in ["no/such", "ping", "tools/list", "prompts/get"] {
        let (got, events) = capture(|| custom_request_error(m));
        assert_eq!(got.code, ErrorCode::METHOD_NOT_FOUND, "{m}");
        assert_eq!(got.message, m, "{m}");
        assert!(got.data.is_none(), "{m}: {:?}", got.data);
        assert!(events.is_empty(), "{m}: {events:?}");
    }
}
