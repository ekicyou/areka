//! 実ソケットのテスト: `initialize`〜`server/discover`・`Accept`・本文の上限（要件 3・5）。
//!
//! 差の一覧（`doc/ssp-mcp/transport-diff-areka.md`）に写す値は、各テストの assert に
//! 具体の値で固定する（別の記録は作らない）。測った版は rmcp 3.5.0。

use serde_json::{Value, json};

use crate::ToolRegistry;
use crate::dispatch::MAX_BODY_BYTES;
use crate::testkit::{MCP_V1, RPC_ACCEPT, Response, post_rpc, request, rpc, serve};

/// `initialize` を持つ旧式の 4 版（要件 3.1）。
const LEGACY_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
/// rmcp が対応する 5 版（`server/discover` の `supportedVersions` に古い順で並ぶ）。
const SUPPORTED_VERSIONS: [&str; 5] = [
    "2024-11-05",
    "2025-03-26",
    "2025-06-18",
    "2025-11-25",
    "2026-07-28",
];

/// `initialize` の本文（`protocolVersion` だけを変える）。
fn initialize(version: &str) -> String {
    rpc(
        "initialize",
        Some(1),
        Some(json!({
            "protocolVersion": version,
            "capabilities": {},
            "clientInfo": { "name": "areka-test", "version": "0" }
        })),
    )
}

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

/// (要件 3.1) 旧式の 4 版は要求どおりの版・`tools` だけの能力・serverInfo・instructions。
#[test]
fn initialize_returns_requested_version_for_each_of_four() {
    let (_server, addr) = serve(ToolRegistry::default());
    for version in LEGACY_VERSIONS {
        let result = result_of(&post_rpc(addr, &[], &initialize(version)));
        assert_eq!(result["protocolVersion"], version, "{result}");
        let capabilities = &result["capabilities"];
        assert!(capabilities.get("tools").is_some(), "{result}");
        assert!(capabilities.get("resources").is_none(), "{result}");
        assert!(capabilities.get("prompts").is_none(), "{result}");
        assert_eq!(
            result["serverInfo"],
            json!({ "name": "areka-mcp-server", "version": env!("CARGO_PKG_VERSION") }),
            "{result}"
        );
        assert!(
            result["instructions"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "{result}"
        );
    }
}

/// (要件 3.1・設計 B-11) `initialize` を持たない `2026-07-28` は `2025-11-25` へ倒れる（成功・200）。
#[test]
fn initialize_with_no_initialize_version_falls_back_to_2025_11_25() {
    let (_server, addr) = serve(ToolRegistry::default());
    let result = result_of(&post_rpc(addr, &[], &initialize("2026-07-28")));
    assert_eq!(result["protocolVersion"], "2025-11-25", "{result}");
}

/// (要件 3.2) 未知の版 `1999-01-01` も失敗にせず、測った値は `2025-11-25`（差の一覧へ）。
#[test]
fn initialize_unknown_version_falls_back() {
    let (_server, addr) = serve(ToolRegistry::default());
    let result = result_of(&post_rpc(addr, &[], &initialize("1999-01-01")));
    assert_eq!(result["protocolVersion"], "2025-11-25", "{result}");
}

/// (要件 3.3) `id` の無い通知は 202・本文 0 バイト。
#[test]
fn initialized_notification_is_202_without_body() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = post_rpc(addr, &[], &rpc("notifications/initialized", None, None));
    assert_eq!(response.status, 202, "{response:?}");
    assert!(response.body.is_empty(), "{response:?}");
}

/// (要件 3.4) 登録 0 本の `tools/list` は空の配列。
#[test]
fn tools_list_is_empty() {
    let (_server, addr) = serve(ToolRegistry::default());
    let result = result_of(&post_rpc(addr, &[], &rpc("tools/list", Some(1), None)));
    assert_eq!(result["tools"], json!([]), "{result}");
}

/// (要件 3.5) `ping` は `result: {}`。
#[test]
fn ping_returns_empty_object() {
    let (_server, addr) = serve(ToolRegistry::default());
    let result = result_of(&post_rpc(addr, &[], &rpc("ping", Some(7), None)));
    assert_eq!(result, json!({}));
}

/// (要件 3.6) どの名前・引数の `tools/call` も JSON-RPC のエラー `-32602`（`isError` の結果ではない）。
#[test]
fn tools_call_any_name_is_invalid_params() {
    let (_server, addr) = serve(ToolRegistry::default());
    for params in [
        json!({ "name": "get_status", "arguments": {} }),
        json!({ "name": "no_such_tool", "arguments": { "x": 1 } }),
    ] {
        let error = error_of(&post_rpc(
            addr,
            &[],
            &rpc("tools/call", Some(1), Some(params)),
        ));
        assert_eq!(error["code"], -32602, "{error}");
    }
}

/// (要件 3.7) 未知メソッドは rmcp のまま `-32601`・旧式の経路で HTTP 200（差の一覧へ）。
#[test]
fn unknown_method_is_jsonrpc_error_from_rmcp() {
    let (_server, addr) = serve(ToolRegistry::default());
    let error = error_of(&post_rpc(addr, &[], &rpc("no/such", Some(1), None)));
    assert_eq!(error["code"], -32601, "{error}");
}

/// (要件 3.8) `Content-Type` が JSON でない本文は 415。
#[test]
fn non_json_content_type_is_415() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = request(
        addr,
        "POST",
        MCP_V1,
        &[("Content-Type", "text/plain"), ("Accept", RPC_ACCEPT)],
        rpc("ping", Some(1), None).as_bytes(),
    );
    assert_eq!(response.status, 415, "{response:?}");
}

/// (要件 3.8・設計 B-10) 壊れた JSON は 415・平文（JSON-RPC のエラーではない）。
/// 待受は落ちず、同じサーバへ続けて送った `ping` が通る。
#[test]
fn broken_json_body_is_415_and_next_request_works() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = post_rpc(addr, &[], "{");
    assert_eq!(response.status, 415, "{response:?}");
    assert!(!response.body.is_empty(), "{response:?}");
    assert!(
        serde_json::from_slice::<Value>(&response.body).is_err(),
        "本文が平文でない: {response:?}"
    );
    let result = result_of(&post_rpc(addr, &[], &rpc("ping", Some(2), None)));
    assert_eq!(result, json!({}));
}

/// (要件 3.9) `initialize` を経ない `ping`／`tools/list` も 200・応答に `Mcp-Session-Id` が無い。
#[test]
fn ping_without_initialize_and_without_session_id() {
    let (_server, addr) = serve(ToolRegistry::default());
    for method in ["ping", "tools/list"] {
        let response = post_rpc(addr, &[], &rpc(method, Some(1), None));
        result_of(&response);
        assert!(response.header("mcp-session-id").is_none(), "{response:?}");
    }
}

/// (要件 3.10) 応答は `application/json` の単発（JSON 1 件）で、SSE ではない。
#[test]
fn response_is_single_application_json() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = post_rpc(addr, &[], &rpc("ping", Some(1), None));
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(response.header("content-type"), Some("application/json"));
    assert!(response.json().is_object(), "{response:?}");
    assert!(
        !String::from_utf8_lossy(&response.body).contains("data:"),
        "{response:?}"
    );
}

/// (要件 3.11) `GET /api/mcp/v1` は 405・`Allow: POST`・フォーム無し（差の一覧へ）。
#[test]
fn get_v1_has_no_form() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = request(addr, "GET", MCP_V1, &[], b"");
    assert_eq!(response.status, 405, "{response:?}");
    assert_eq!(response.header("allow"), Some("POST"), "{response:?}");
    assert!(
        !String::from_utf8_lossy(&response.body).contains("<form"),
        "{response:?}"
    );
}

/// (要件 3.12) 無状態版の `server/discover`（本文の `_meta` と `MCP-Protocol-Version: 2026-07-28`）。
/// 測った欄（差の一覧へ）: `supportedVersions` は 5 版・`capabilities` は `tools` だけ・
/// `instructions`・`ttlMs: 0`・`cacheScope: "private"`・`resultType: "complete"`・
/// serverInfo は `_meta.io.modelcontextprotocol/serverInfo`。
#[test]
fn server_discover_stateless() {
    let (_server, addr) = serve(ToolRegistry::default());
    let params = json!({ "_meta": {
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "areka-test", "version": "0" }
    }});
    let response = post_rpc(
        addr,
        &[
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "server/discover"),
        ],
        &rpc("server/discover", Some(1), Some(params)),
    );
    let result = result_of(&response);
    assert_eq!(
        result["supportedVersions"],
        json!(SUPPORTED_VERSIONS),
        "{result}"
    );
    assert_eq!(result["capabilities"], json!({ "tools": {} }), "{result}");
    assert_eq!(
        result["instructions"],
        crate::handler::INSTRUCTIONS,
        "{result}"
    );
    assert_eq!(result["ttlMs"], 0, "{result}");
    assert_eq!(result["cacheScope"], "private", "{result}");
    assert_eq!(result["resultType"], "complete", "{result}");
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"],
        json!({ "name": "areka-mcp-server", "version": env!("CARGO_PKG_VERSION") }),
        "{result}"
    );
}

/// (要件 3.13) `MCP-Protocol-Version` が無い・旧式の 4 版のどれでも、`ping`・`tools/list` は同じ答え。
#[test]
fn protocol_version_header_old_or_missing_is_passthrough() {
    let (_server, addr) = serve(ToolRegistry::default());
    let headers: Vec<Vec<(&str, &str)>> = std::iter::once(Vec::new())
        .chain(
            LEGACY_VERSIONS
                .iter()
                .map(|v| vec![("MCP-Protocol-Version", *v)]),
        )
        .collect();
    for extra in &headers {
        let ping = result_of(&post_rpc(addr, extra, &rpc("ping", Some(1), None)));
        assert_eq!(ping, json!({}), "{extra:?}");
        let list = result_of(&post_rpc(addr, extra, &rpc("tools/list", Some(2), None)));
        assert_eq!(list["tools"], json!([]), "{extra:?}");
    }
}

/// (要件 3.13) 5 版のどれでもないヘッダ値は rmcp のまま 400・平文（差の一覧へ）。
#[test]
fn unknown_protocol_version_header_is_rejected() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = post_rpc(
        addr,
        &[("MCP-Protocol-Version", "1999-01-01")],
        &rpc("ping", Some(1), None),
    );
    assert_eq!(response.status, 400, "{response:?}");
    assert_eq!(
        String::from_utf8_lossy(&response.body),
        "Bad Request: Unsupported MCP-Protocol-Version: 1999-01-01"
    );
}

/// (要件 5.1・設計 B-8) `Accept` の無い `ping` は rmcp が 406 で拒む（差の一覧へ）。
#[test]
fn accept_header_missing_is_406() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = request(
        addr,
        "POST",
        MCP_V1,
        &[("Content-Type", "application/json")],
        rpc("ping", Some(1), None).as_bytes(),
    );
    assert_eq!(response.status, 406, "{response:?}");
}

/// (要件 5.1・設計 B-7) 上限（4 MiB）を 1 バイト超えた本文は 413（差の一覧へ）。
#[test]
fn body_over_limit_is_413() {
    let (_server, addr) = serve(ToolRegistry::default());
    let body = vec![b' '; MAX_BODY_BYTES + 1];
    let response = request(
        addr,
        "POST",
        MCP_V1,
        &[("Content-Type", "application/json"), ("Accept", RPC_ACCEPT)],
        &body,
    );
    assert_eq!(response.status, 413, "{response:?}");
}
