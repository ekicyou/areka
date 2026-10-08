//! 11 本の登録（[`all_entrances`]）の実ソケットのテスト（一覧・引数の検査・指示文・help）。
//! spec: areka-P0-mcp-author-tools（要件 1.1〜1.5・1.7・2.1・4.1〜4.3・5.1〜5.3）。
//!
//! 土台は `tools_socket_tests.rs` と同じ（`127.0.0.1` の空きポート・ネットへ出ない・受け手はテストの
//! スレッドの偽物）。旧式（ヘッダも `_meta` も無い）と無状態版（`2026-07-28`）の両方の経路で確かめる。

use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use super::{ToolCall, ToolRequest, all_entrances, check_script};
use crate::testkit::{Response, post_rpc, request, rpc, serve};

/// 保存した SSP の定義（テストだけが読む）。
const SAVED_LIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../doc/ssp-mcp/tools-list-ssp-2.9.05.json"
);
/// 受け手の要求を待つ上限。届かなければテストを止めずに失敗させる（正しさは待ち時間に頼らない）。
const RECV_WAIT: Duration = Duration::from_secs(10);
/// 返事の上限（受け手は必ず答えるので、ここで切れることは無い）。
const LONG_WAIT: Duration = Duration::from_secs(30);
/// 無状態版の版。
const STATELESS: &str = "2026-07-28";
/// help のパス。
const HELP: &str = "/api/mcp/help";

/// 無状態版の要求に付ける本文の `_meta`。
fn stateless_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": STATELESS,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "areka-test", "version": "0" }
    })
}

/// 旧式の経路で 1 件送る。
fn post_legacy(addr: SocketAddr, method: &str, id: u64, params: Option<Value>) -> Response {
    post_rpc(addr, &[], &rpc(method, Some(id), params))
}

/// 無状態版の経路で 1 件送る（`MCP-Protocol-Version`・`Mcp-Method`・`tools/call` なら `Mcp-Name`・本文の `_meta`）。
fn post_stateless(addr: SocketAddr, method: &str, id: u64, params: Value) -> Response {
    let mut params = params;
    params["_meta"] = stateless_meta();
    let name = params["name"].as_str().map(str::to_owned);
    let mut headers = vec![("MCP-Protocol-Version", STATELESS), ("Mcp-Method", method)];
    if let Some(name) = name.as_deref() {
        headers.push(("Mcp-Name", name));
    }
    post_rpc(addr, &headers, &rpc(method, Some(id), Some(params)))
}

/// 両方の経路で同じ要求を送る（経路の名前, 応答）。`params` が None なら旧式は欄ごと省き、無状態版は `{}`。
fn post_both(
    addr: SocketAddr,
    method: &str,
    params: Option<Value>,
) -> [(&'static str, Response); 2] {
    [
        ("旧式", post_legacy(addr, method, 1, params.clone())),
        (
            "無状態版",
            post_stateless(addr, method, 2, params.unwrap_or_else(|| json!({}))),
        ),
    ]
}

/// `tools/call` の `params`。
fn call_params(name: &str, arguments: Value) -> Value {
    json!({ "name": name, "arguments": arguments })
}

/// 200 で JSON 1 件の応答を読み、`result` を返す（`error` があれば失敗）。
fn result_of(path: &str, response: &Response) -> Value {
    assert_eq!(response.status, 200, "{path}: {response:?}");
    let value = response.json();
    assert!(value.get("error").is_none(), "{path}: {value}");
    value["result"].clone()
}

/// 受け手に要求が 1 件も届いていない。検査は handler を呼ぶ前に同期で済み、応答はその後に返るので、
/// 応答を受けた後に覗けば十分。
fn assert_no_request(rx: &Receiver<ToolRequest>) {
    match rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        Ok(request) => panic!("受け手に要求が届いた: {:?}", request.call),
        Err(TryRecvError::Disconnected) => panic!("受け口が閉じている"),
    }
}

/// 11 本の登録のサーバへ両方の経路で同じ `tools/call` を送り、`-32602` の `error` を固定する。
/// 旧式は 200・無状態版は 400（rmcp 3.5.0 で測った値・`tools_socket_tests.rs` と同じ）。受け手への要求は 0 件。
fn assert_invalid_params(name: &str, arguments: Value, message: &str) {
    let (registry, rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    for ((path, response), status) in
        post_both(addr, "tools/call", Some(call_params(name, arguments)))
            .into_iter()
            .zip([200, 400])
    {
        assert_eq!(response.status, status, "{path}: {response:?}");
        let value = response.json();
        assert!(value.get("result").is_none(), "{path}: {value}");
        assert_eq!(
            value["error"],
            json!({ "code": -32602, "message": message }),
            "{path}: {value}"
        );
    }
    assert_no_request(&rx);
}

/// (要件 1.1・1.3・1.4・1.5・5.1・5.3) `tools/list` は 11 本。先頭 10 本は保存した SSP の定義と配列ごと一致
/// （並び・欄の過不足を 1 度に見る）、11 本目は `check_script` の定義そのもので、欄は 4 つだけ。
#[test]
fn tools_list_is_ssp_ten_then_check_script() {
    let text = std::fs::read_to_string(SAVED_LIST).expect("保存した JSON が読めない");
    let saved: Value = serde_json::from_str(&text).expect("保存した JSON が JSON でない");
    let own: Value = serde_json::from_str(check_script::DEFINITION).expect("定義が JSON");
    let (registry, _rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);

    for (path, response) in post_both(addr, "tools/list", None) {
        let result = result_of(path, &response);
        let tools = result["tools"].as_array().expect("tools が配列");
        assert_eq!(tools.len(), 11, "{path}: {result}");
        assert_eq!(json!(tools[..10]), saved["tools"], "{path}");
        assert_eq!(tools[10], own, "{path}");
        let mut keys: Vec<&str> = tools[10]
            .as_object()
            .expect("定義が object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["description", "inputSchema", "name", "title"],
            "{path}"
        );
        assert_eq!(tools[10]["name"], "check_script", "{path}");
    }
}

/// (要件 2.1・5.2) 引数が全部ある `check_script` は、両方の経路で型の付いた `ToolCall::CheckScript` として
/// 受け手へ届き、受け手の答えがそのまま応答になる。1 回の呼び出しで届く要求は 1 件だけ。
#[test]
fn check_script_reaches_the_receiver_with_typed_args() {
    let script = "\\0\\s[0]hi\\e";
    let expected = ToolCall::CheckScript(check_script::Args {
        script: script.to_owned(),
        ghost_name: Some("emo".to_owned()),
    });
    let (registry, rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    // 偽の受け手: 受けた `ToolCall` をテストへ渡し、診断 0 件の結果で答える。
    let (seen_tx, seen_rx) = std::sync::mpsc::channel();
    let receiver = thread::spawn(move || {
        for request in rx.iter().take(2) {
            let _ = seen_tx.send(request.call);
            request.reply.send(check_script::render(&[], None));
        }
        rx
    });

    let params = call_params(
        "check_script",
        json!({ "script": script, "ghost_name": "emo" }),
    );
    // 経路ごとの要求は受け手が答えてから次へ進む（`post_both` は 1 件ずつ送る）ので、届いた順は経路の順。
    for (path, response) in post_both(addr, "tools/call", Some(params)) {
        let result = result_of(path, &response);
        let got = seen_rx.recv_timeout(RECV_WAIT).expect("受け手へ届いた");
        assert_eq!(got, expected, "{path}");
        assert_eq!(
            result["content"],
            json!([{ "type": "text", "text": "OK:0 diagnostics" }]),
            "{path}: {result}"
        );
        assert_eq!(result["isError"], false, "{path}: {result}");
    }
    let rx = receiver.join().expect("偽の受け手が panic しなかった");
    assert_no_request(&rx);
}

/// (要件 2.1・5.2) `script` が無い `check_script` は `-32602`・受け手へ届かない。
#[test]
fn check_script_missing_script_is_invalid_params() {
    assert_invalid_params(
        "check_script",
        json!({ "ghost_name": "emo" }),
        "missing required argument: script",
    );
}

/// (要件 2.1・5.2) `script` が数の `check_script` は `-32602`・受け手へ届かない。
#[test]
fn check_script_numeric_script_is_invalid_params() {
    assert_invalid_params(
        "check_script",
        json!({ "script": 42 }),
        "argument script must be string",
    );
}

/// (要件 1.7) 11 本のどれでもない名前は `-32602`・受け手へ届かない。
#[test]
fn name_outside_eleven_is_invalid_params() {
    assert_invalid_params("no_such_tool", json!({}), "tool not found");
}

/// (要件 4.1) 指示文（旧式は `initialize`・無状態版は `server/discover`）は `check_script` の案内と、
/// 今ある 2 つの案内（先に `get_active_ghost_list`・未実装の結果の形）を含む。
#[test]
fn instructions_name_check_script_and_keep_existing_guidance() {
    let (registry, _rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    let initialize = post_legacy(
        addr,
        "initialize",
        1,
        Some(json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "areka-test", "version": "0" }
        })),
    );
    let discover = post_stateless(addr, "server/discover", 2, json!({}));
    for (path, response) in [("旧式", initialize), ("無状態版", discover)] {
        let result = result_of(path, &response);
        let instructions = result["instructions"]
            .as_str()
            .expect("instructions が文字列");
        for phrase in [
            "check_script is an areka-only tool",
            "without playing it",
            "use it before sakurascript",
            "call get_active_ghost_list first",
            "\"NG:not implemented yet\"",
        ] {
            assert!(
                instructions.contains(phrase),
                "{path}: {phrase}: {instructions}"
            );
        }
    }
}

/// (要件 4.2・4.3) help のページに `check_script` の名前と日本語の 1 行が 1 項目として出る。
#[test]
fn help_lists_check_script_with_japanese_line() {
    let (registry, _rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    let response = request(addr, "GET", HELP, &[], b"");
    assert_eq!(response.status, 200, "{response:?}");
    let body = String::from_utf8(response.body).expect("本文が UTF-8");
    assert!(body.contains("<h2>areka 独自のツール</h2>"), "{body}");
    let item = format!(
        "<li><code>check_script</code> — {}</li>",
        check_script::SUMMARY_JA
    );
    assert!(body.contains(&item), "{body}");
}
