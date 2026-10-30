//! `request_shape_error` と `shape_error_status` の決定論テスト（spec: mcp-strict-errors 要件 6.1・6.2・6.4）。

use hyper::StatusCode;
use rmcp::model::{ErrorCode, NumberOrString};
use serde_json::{Value, json};

use super::{request_shape_error, shape_error_status};

fn request(id: Value, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/list", "params": params})
}

/// 答えの番号と id（`Some` でなければ落とす）。
fn answer(raw: &Value) -> (ErrorCode, Option<NumberOrString>) {
    let err = request_shape_error(raw).expect("答えるはず");
    (err.error.code, err.id)
}

#[test]
fn scalar_params_answer_invalid_request_with_the_id() {
    for params in [json!(1), json!(1.5), json!("x"), json!(true), json!(false)] {
        assert_eq!(
            answer(&request(json!(7), params.clone())),
            (ErrorCode::INVALID_REQUEST, Some(NumberOrString::Number(7))),
            "params={params}"
        );
    }
}

#[test]
fn array_params_answer_invalid_params_with_the_id() {
    for params in [json!([]), json!([1, "a"])] {
        assert_eq!(
            answer(&request(json!("abc"), params.clone())),
            (
                ErrorCode::INVALID_PARAMS,
                Some(NumberOrString::String("abc".into()))
            ),
            "params={params}"
        );
    }
}

#[test]
fn unknown_method_gets_the_same_answer() {
    let raw = json!({"jsonrpc": "2.0", "id": 3, "method": "no/such", "params": 1});
    assert_eq!(
        answer(&raw),
        (ErrorCode::INVALID_REQUEST, Some(NumberOrString::Number(3)))
    );
}

#[test]
fn answer_serializes_as_one_jsonrpc_error() {
    let err = request_shape_error(&request(json!(9), json!([]))).unwrap();
    let v = serde_json::to_value(&err).unwrap();
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["id"], 9);
    assert_eq!(v["error"]["code"], -32602);
    assert!(v["error"]["message"].is_string());
}

#[test]
fn no_answer_when_not_a_shape_error() {
    let cases = [
        // 通知（id が無い）
        json!({"jsonrpc": "2.0", "method": "tools/list", "params": 1}),
        // params が無い・null・オブジェクト
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
        request(json!(1), Value::Null),
        request(json!(1), json!({})),
        // id が数・文字列でない
        request(Value::Null, json!(1)),
        request(json!(1.5), json!(1)),
        request(json!({}), json!(1)),
        // jsonrpc が "2.0" でない・method が無い
        json!({"jsonrpc": "1.0", "id": 1, "method": "tools/list", "params": 1}),
        json!({"id": 1, "method": "tools/list", "params": 1}),
        json!({"jsonrpc": "2.0", "id": 1, "params": 1}),
        // オブジェクトでない本文
        json!([request(json!(1), json!(1))]),
        json!("text"),
    ];
    for raw in cases {
        assert!(request_shape_error(&raw).is_none(), "raw={raw}");
    }
}

#[test]
fn status_is_400_only_for_invalid_params_on_the_stateless_version() {
    let stateless = Some("2026-07-28");
    assert_eq!(
        shape_error_status(stateless, ErrorCode::INVALID_PARAMS),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        shape_error_status(stateless, ErrorCode::INVALID_REQUEST),
        StatusCode::OK
    );
    // rmcp と同じ文字列の比較: `2026-07-28` より後の版も無状態の扱い（415 の経路では rmcp はヘッダを見ない）。
    let newer = Some("2099-01-01");
    assert_eq!(
        shape_error_status(newer, ErrorCode::INVALID_PARAMS),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        shape_error_status(newer, ErrorCode::INVALID_REQUEST),
        StatusCode::OK
    );
    for legacy in [
        None,
        Some("2024-11-05"),
        Some("2025-03-26"),
        Some("2025-06-18"),
        Some("2025-11-25"),
    ] {
        for code in [ErrorCode::INVALID_PARAMS, ErrorCode::INVALID_REQUEST] {
            assert_eq!(
                shape_error_status(legacy, code),
                StatusCode::OK,
                "{legacy:?}"
            );
        }
    }
}

/// 受け口の配線: rmcp の 415 の後に答えを返す。JSON でない `Content-Type` の 415 は今のまま
/// （5 つの版の表は 8.2 の受け口のテストが持つ）。
#[test]
fn forward_answers_after_rmcp_415_only_for_json_bodies() {
    use crate::ToolRegistry;
    use crate::testkit::{MCP_V1, RPC_ACCEPT, post_rpc, request as http, serve};

    let (_server, addr) = serve(ToolRegistry::default());
    let body = r#"{"jsonrpc":"2.0","id":5,"method":"tools/list","params":[1]}"#;
    let response = post_rpc(addr, &[("MCP-Protocol-Version", "2026-07-28")], body);
    assert_eq!(response.status, 400, "{response:?}");
    assert_eq!(response.header("content-type"), Some("application/json"));
    let v = response.json();
    assert_eq!(
        (v["id"].clone(), v["error"]["code"].clone()),
        (json!(5), json!(-32602))
    );

    let response = http(
        addr,
        "POST",
        MCP_V1,
        &[("Content-Type", "text/plain"), ("Accept", RPC_ACCEPT)],
        body.as_bytes(),
    );
    assert_eq!(response.status, 415, "{response:?}");
}
