//! 実ソケットのテスト: `Origin`／`Host`・help・404／405・登録 1 本の往復（要件 4・6・7）。
//!
//! 拒否の `warn!` と要求ごとの `debug!` は mcp スレッドで出るので、ここでは数えない
//! （実機確認の `RUST_LOG` で確かめる＝要件 9.6）。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::testkit::{Response, post_rpc, request, rpc, serve};
use crate::{ToolContent, ToolHandler, ToolOutcome, ToolRegistry, ToolSpec};

const HELP: &str = "/api/mcp/help";
const EVIL_ORIGIN: &str = "http://evil.example";

/// 200 で JSON 1 件の応答を読み、`result` を返す（`error` があれば失敗）。
fn result_of(response: &Response) -> Value {
    assert_eq!(response.status, 200, "{response:?}");
    let value = response.json();
    assert!(value.get("error").is_none(), "{value}");
    value["result"].clone()
}

/// 200 で JSON 1 件の応答を読み、`error` を返す（`result` があれば失敗）。
fn error_of(response: &Response) -> Value {
    assert_eq!(response.status, 200, "{response:?}");
    let value = response.json();
    assert!(value.get("result").is_none(), "{value}");
    value["error"].clone()
}

/// 403 で、本文が JSON-RPC でない（MCP の処理に入っていない）ことを確かめる。
fn assert_forbidden_before_mcp(response: &Response) {
    assert_eq!(response.status, 403, "{response:?}");
    assert!(
        serde_json::from_slice::<Value>(&response.body).is_err(),
        "本文が平文でない: {response:?}"
    );
}

/// (要件 4.3・6.3) 悪い `Origin` は `initialize`・`ping`・help のどれも 403（MCP の処理に入らない）。
#[test]
fn bad_origin_is_403_before_mcp() {
    let (_server, addr) = serve(ToolRegistry::default());
    let origin = [("Origin", EVIL_ORIGIN)];
    let initialize = rpc(
        "initialize",
        Some(1),
        Some(json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "areka-test", "version": "0" }
        })),
    );
    assert_forbidden_before_mcp(&post_rpc(addr, &origin, &initialize));
    assert_forbidden_before_mcp(&post_rpc(addr, &origin, &rpc("ping", Some(2), None)));
    let help = request(addr, "GET", HELP, &origin, b"");
    assert_forbidden_before_mcp(&help);
    assert!(
        !String::from_utf8_lossy(&help.body).contains("<html"),
        "{help:?}"
    );
}

/// (要件 4.4) ループバックでない `Host` は 403。受け口では rmcp の `allowed_hosts` も
/// 同じく拒むので、rmcp を通らない help でも確かめて `gate` の検査を踏む。
#[test]
fn bad_host_is_403() {
    let (_server, addr) = serve(ToolRegistry::default());
    let host = [("Host", "evil.example")];
    assert_forbidden_before_mcp(&post_rpc(addr, &host, &rpc("ping", Some(1), None)));
    assert_forbidden_before_mcp(&request(addr, "GET", HELP, &host, b""));
}

/// (要件 6.1・6.2) help は 200・`text/html; charset=utf-8`・本文の URL は実番号。
#[test]
fn help_is_200_html_with_actual_port() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = request(addr, "GET", HELP, &[], b"");
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(
        response.header("content-type"),
        Some("text/html; charset=utf-8"),
        "{response:?}"
    );
    let body = String::from_utf8(response.body).expect("本文が UTF-8");
    let url = format!("http://127.0.0.1:{}/api/mcp/v1", addr.port());
    assert!(body.contains(&url), "{body}");
    assert!(
        body.contains(&format!("claude mcp add --transport http areka {url}")),
        "{body}"
    );
    // 空きポートがたまたま 9821 なら、固定の番号の検出はできない（実番号の在りかは上で確かめた）。
    if addr.port() != 9821 {
        assert!(!body.contains("127.0.0.1:9821"), "{body}");
    }
}

/// (要件 6.4) 受け口と help 以外のパスは 404。
#[test]
fn unknown_path_is_404() {
    let (_server, addr) = serve(ToolRegistry::default());
    for path in ["/", "/api/mcp/", "/api/mcp/help/x"] {
        let response = request(addr, "GET", path, &[], b"");
        assert_eq!(response.status, 404, "{path}: {response:?}");
    }
}

/// (要件 6.5) help への `POST` は 405・`Allow: GET`・ページを返さない。
#[test]
fn help_post_is_405() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = request(addr, "POST", HELP, &[], b"");
    assert_eq!(response.status, 405, "{response:?}");
    assert_eq!(response.header("allow"), Some("GET"), "{response:?}");
    assert!(
        !String::from_utf8_lossy(&response.body).contains("<html"),
        "{response:?}"
    );
}

/// 往復に使う 1 本の定義（SSP の定義を詰める形と同じく逐語）。
fn echo_spec() -> ToolSpec {
    let input_schema = json!({
        "type": "object",
        "properties": { "text": { "type": "string", "description": "返す文字" } },
        "required": ["text"]
    });
    ToolSpec {
        name: "echo_args".to_owned(),
        title: Some("引数を返す".to_owned()),
        description: Some("受けた arguments を text で返し、画像を 1 枚添える。".to_owned()),
        input_schema: input_schema.as_object().unwrap().clone(),
    }
}

/// 受けた `arguments` を JSON の文字列で返し、画像 1 枚を添える実装。
/// `is_error` は `arguments` の `"fail": true` のときだけ立てる（両方の値の写しを踏むため）。
fn echo_handler() -> ToolHandler {
    Arc::new(|args| {
        Box::pin(async move {
            let is_error = args.get("fail") == Some(&Value::Bool(true));
            ToolOutcome {
                content: vec![
                    ToolContent::Text(args.to_string()),
                    ToolContent::Image {
                        data: "iVBORw0KGgo=".to_owned(),
                        mime_type: "image/png".to_owned(),
                    },
                ],
                is_error,
            }
        })
    })
}

fn echo_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::default();
    registry.register(echo_spec(), echo_handler());
    registry
}

/// (要件 7.1・7.2・7.4) 1 本登録 → `tools/list` に逐語で出る → `tools/call` が実装へ
/// `arguments` を届け、実装の `content`／`isError` をそのまま返す。
#[test]
fn registered_tool_roundtrip() {
    let (_server, addr) = serve(echo_registry());
    let spec = echo_spec();

    let list = result_of(&post_rpc(addr, &[], &rpc("tools/list", Some(1), None)));
    let tools = list["tools"].as_array().expect("tools が配列");
    assert_eq!(tools.len(), 1, "{list}");
    let tool = &tools[0];
    assert_eq!(tool["name"], json!(spec.name), "{tool}");
    assert_eq!(tool["title"], json!(spec.title), "{tool}");
    assert_eq!(tool["description"], json!(spec.description), "{tool}");
    assert_eq!(
        tool["inputSchema"],
        Value::Object(spec.input_schema),
        "{tool}"
    );

    // 失敗を返す呼び出しと成功を返す呼び出しの両方で、content と isError がそのまま返る。
    for (id, arguments, is_error) in [
        (
            2,
            json!({ "text": "こんにちは", "n": 3, "fail": true }),
            true,
        ),
        (3, json!({ "text": "こんにちは", "n": 3 }), false),
    ] {
        let call = result_of(&post_rpc(
            addr,
            &[],
            &rpc(
                "tools/call",
                Some(id),
                Some(json!({ "name": "echo_args", "arguments": arguments })),
            ),
        ));
        assert_eq!(
            call["content"],
            json!([
                { "type": "text", "text": arguments.to_string() },
                { "type": "image", "data": "iVBORw0KGgo=", "mimeType": "image/png" }
            ]),
            "{call}"
        );
        assert_eq!(call["isError"], is_error, "{call}");
    }
}

/// (要件 7.3) 登録済みのサーバでも、登録した名前以外の `tools/call` は `-32602`。
#[test]
fn unregistered_name_is_invalid_params() {
    let (_server, addr) = serve(echo_registry());
    let error = error_of(&post_rpc(
        addr,
        &[],
        &rpc(
            "tools/call",
            Some(1),
            Some(json!({ "name": "no_such_tool", "arguments": {} })),
        ),
    ));
    assert_eq!(error["code"], -32602, "{error}");
}
