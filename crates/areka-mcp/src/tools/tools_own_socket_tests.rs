//! 11 本の登録（[`all_entrances`]）の実ソケットのテスト（一覧・引数の検査・指示文・help）。
//! spec: areka-P0-mcp-author-tools（要件 1.1〜1.5・1.7・2.1・4.1〜4.3・5.1〜5.3）。
//!
//! 土台は `tools_socket_tests.rs` と同じ（`127.0.0.1` の空きポート・ネットへ出ない・受け手はテストの
//! スレッドの偽物）。旧式（ヘッダも `_meta` も無い）と無状態版（`2026-07-28`）の両方の経路で確かめる。

use std::net::SocketAddr;
use std::thread;

use serde_json::{Value, json};

use super::{ToolCall, all_entrances, check_script};
use crate::testkit::{
    HELP, LONG_WAIT, RECV_WAIT, Response, SAVED_LIST, assert_invalid_params, assert_no_request,
    call_params, post_legacy, post_stateless, request, serve,
};

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
        let result = response.result();
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
        let result = response.result();
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
        all_entrances(LONG_WAIT),
        "check_script",
        json!({ "ghost_name": "emo" }),
        "missing required argument: script",
    );
}

/// (要件 2.1・5.2) `script` が数の `check_script` は `-32602`・受け手へ届かない。
#[test]
fn check_script_numeric_script_is_invalid_params() {
    assert_invalid_params(
        all_entrances(LONG_WAIT),
        "check_script",
        json!({ "script": 42 }),
        "argument script must be string",
    );
}

/// (要件 1.7) 11 本のどれでもない名前は `-32602`・受け手へ届かない。
#[test]
fn name_outside_eleven_is_invalid_params() {
    assert_invalid_params(
        all_entrances(LONG_WAIT),
        "no_such_tool",
        json!({}),
        "tool not found",
    );
}

/// 消した後の指示文（定数を写さず、前の 3 文を字のまま書く）。
const THREE_SENTENCES: &str = "This server controls areka, a desktop mascot (Ukagaka-compatible baseware) running on this machine. It exposes the same tools as the MCP server of SSP; call get_active_ghost_list first to get the ghost name for the ghost_name parameter. check_script is an areka-only tool that SSP does not have: it checks SakuraScript without playing it (the ghost does nothing), so use it before sakurascript.";

/// (要件 4.1・spec: mcp-strict-errors 要件 5.1) 指示文（旧式は `initialize`・無状態版は
/// `server/discover`）は `check_script` の案内と先に `get_active_ghost_list` の案内を含む 3 文で、
/// 「まだのツール」の 1 文（`not implemented yet`）は消えている。前の 3 文は 1 字も変わらない。
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
        let result = response.result();
        let instructions = result["instructions"]
            .as_str()
            .expect("instructions が文字列");
        for phrase in [
            "check_script is an areka-only tool",
            "without playing it",
            "use it before sakurascript",
            "call get_active_ghost_list first",
        ] {
            assert!(
                instructions.contains(phrase),
                "{path}: {phrase}: {instructions}"
            );
        }
        assert_eq!(instructions, THREE_SENTENCES, "{path}");
        assert!(!instructions.contains("not implemented yet"), "{path}");
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
