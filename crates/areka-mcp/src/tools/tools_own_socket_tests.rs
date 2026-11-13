//! 本番の 11 本の登録（[`all_entrances`]）の実ソケットのテスト（一覧・引数の検査・指示文・型つきの
//! 呼び出しの往復・待ちの間の `ping`）。spec: areka-P0-mcp-author-tools（要件 1.1〜1.5・1.7・2.1・4.1・
//! 5.1〜5.3）・areka-P0-mcp-tool-entrances（要件 5.1・5.3・5.4・6.3・6.8）。
//!
//! `127.0.0.1` の空きポートだけを使い、ネットへは出ない。受け手はテストのスレッドの偽物。旧式（ヘッダも
//! `_meta` も無い）と無状態版（`2026-07-28`）の両方の経路で確かめる。

use std::net::SocketAddr;
use std::thread;

use serde_json::{Value, json};

use super::{
    ToolCall, all_entrances, check_script, dump_balloon, dump_surface, get_expression_table,
    get_log, get_property, get_status, outcome, raise_event, reload, sakurascript,
};
use crate::testkit::{
    LONG_WAIT, RECV_WAIT, Response, assert_invalid_params, assert_no_request, call_params,
    initialize_params, post_legacy, post_stateless, saved_list, serve,
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
    let saved = saved_list();
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

/// `sakurascript` の必須の欄の欠落（`ghost_name` だけ）と型違い（`strict: "yes"`）は `-32602`・受け手へ届かない
/// （`doc/ssp-mcp/transport-diff-areka.md` の「未知のツール名・必須引数の欠落」の行の値）。
#[test]
fn sakurascript_missing_script_and_string_strict_are_invalid_params() {
    assert_invalid_params(
        all_entrances(LONG_WAIT),
        "sakurascript",
        json!({ "ghost_name": "emo" }),
        "missing required argument: script",
    );
    assert_invalid_params(
        all_entrances(LONG_WAIT),
        "sakurascript",
        json!({ "script": "s", "strict": "yes" }),
        "argument strict must be boolean",
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
/// `server/discover`）は、`check_script` の案内と先に `get_active_ghost_list` の案内を含む 3 文と字のまま
/// 一致する（「まだのツール」の 1 文は無い）。
#[test]
fn instructions_name_check_script_and_keep_existing_guidance() {
    let (registry, _rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    let initialize = post_legacy(addr, "initialize", 1, Some(initialize_params("2025-11-25")));
    let discover = post_stateless(addr, "server/discover", 2, json!({}));
    for (path, response) in [("旧式", initialize), ("無状態版", discover)] {
        let result = response.result();
        assert_eq!(result["instructions"], THREE_SENTENCES, "{path}");
    }
}

/// (spec: areka-P0-mcp-tool-entrances 要件 5.1・5.3・5.4) SSP と同じ 9 本それぞれが、本番の登録表を通って
/// 型の付いた引数のまま受け手へ届き、
/// 受け手が返した `NG:not implemented yet` がそのまま応答になる。
#[test]
fn each_of_nine_reaches_the_receiver_with_typed_args() {
    let emo = || Some("emo".to_owned());
    let cases = [
        (
            "get_status",
            json!({ "ghost_name": "emo" }),
            ToolCall::GetStatus(get_status::Args { ghost_name: emo() }),
        ),
        (
            "get_expression_table",
            json!({ "ghost_name": "emo" }),
            ToolCall::GetExpressionTable(get_expression_table::Args { ghost_name: emo() }),
        ),
        (
            "get_property",
            json!({ "property_name": "currentghost.name", "ghost_name": "emo" }),
            ToolCall::GetProperty(get_property::Args {
                property_name: "currentghost.name".to_owned(),
                ghost_name: emo(),
            }),
        ),
        (
            "get_log",
            json!({ "log_type": "error", "ghost_name": "emo", "since_id": 12, "max_count": 3 }),
            ToolCall::GetLog(get_log::Args {
                log_type: Some("error".to_owned()),
                ghost_name: emo(),
                since_id: Some(12),
                max_count: Some(3),
            }),
        ),
        (
            "sakurascript",
            json!({ "script": "\\0\\s[0]hi\\e", "ghost_name": "emo", "strict": true }),
            ToolCall::Sakurascript(sakurascript::Args {
                script: "\\0\\s[0]hi\\e".to_owned(),
                ghost_name: emo(),
                strict: Some(true),
            }),
        ),
        (
            "raise_event",
            json!({ "event": "OnTest", "references": ["a", "b"], "ghost_name": "emo", "strict": false }),
            ToolCall::RaiseEvent(raise_event::Args {
                event: "OnTest".to_owned(),
                references: vec!["a".to_owned(), "b".to_owned()],
                ghost_name: emo(),
                strict: Some(false),
            }),
        ),
        (
            "reload",
            json!({ "target": "shell", "ghost_name": "emo" }),
            ToolCall::Reload(reload::Args {
                target: "shell".to_owned(),
                ghost_name: emo(),
            }),
        ),
        (
            "dump_surface",
            json!({ "scope": 1, "surface": 10, "ghost_name": "emo" }),
            ToolCall::DumpSurface(dump_surface::Args {
                scope: Some(1),
                surface: Some(10),
                ghost_name: emo(),
            }),
        ),
        (
            "dump_balloon",
            json!({ "scope": 0, "ghost_name": "emo" }),
            ToolCall::DumpBalloon(dump_balloon::Args {
                scope: Some(0),
                ghost_name: emo(),
            }),
        ),
    ];

    let (registry, rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    // 偽の受け手: 受けた `ToolCall` をテストへ渡し、`NG:not implemented yet` で答える。
    let (seen_tx, seen_rx) = std::sync::mpsc::channel();
    let receiver = thread::spawn(move || {
        for request in rx.iter().take(9) {
            let _ = seen_tx.send(request.call);
            request.reply.send(outcome::ng("not implemented yet"));
        }
        rx
    });

    for (id, (name, arguments, expected)) in (1..).zip(cases) {
        let result = Response::result(&post_legacy(
            addr,
            "tools/call",
            id,
            Some(call_params(name, arguments)),
        ));
        let got = seen_rx.recv_timeout(RECV_WAIT).expect("受け手へ届いた");
        assert_eq!(got, expected, "{name}");
        assert_eq!(
            result,
            json!({
                "content": [{ "type": "text", "text": "NG:not implemented yet" }],
                "isError": true
            }),
            "{name}"
        );
    }
    // 1 回の呼び出しで届く要求は 1 件だけ（9 件を答えた後に余りが無い）。
    let rx = receiver.join().expect("偽の受け手が panic しなかった");
    assert_no_request(&rx);
}

/// (spec: areka-P0-mcp-tool-entrances 要件 6.3・6.8) 返事を止めた `tools/call` の待ちの間に、`ping` と `tools/list` が答える。
/// 受け手が要求を受け取ったことを合図に問い、答えを確かめてから返事を放す。
#[test]
fn ping_answers_while_a_call_waits() {
    let (registry, rx) = all_entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    let caller = thread::spawn(move || {
        post_legacy(
            addr,
            "tools/call",
            1,
            Some(call_params("get_status", json!({}))),
        )
    });
    // 受け手が要求を受け取った＝呼び出しは返事を待っている。
    let request = rx.recv_timeout(RECV_WAIT).expect("要求が受け手へ届いた");
    assert_eq!(
        request.call,
        ToolCall::GetStatus(get_status::Args { ghost_name: None })
    );

    assert_eq!(
        Response::result(&post_legacy(addr, "ping", 2, None)),
        json!({})
    );
    let list = Response::result(&post_legacy(addr, "tools/list", 3, None));
    assert_eq!(list["tools"].as_array().map(Vec::len), Some(11), "{list}");
    assert!(!caller.is_finished(), "返事を放す前に呼び出しが終わった");

    request.reply.send(outcome::ng("not implemented yet"));
    let result = Response::result(
        &caller
            .join()
            .expect("呼び出しのスレッドが panic しなかった"),
    );
    assert_eq!(
        result["content"],
        json!([{ "type": "text", "text": "NG:not implemented yet" }]),
        "{result}"
    );
    assert_eq!(result["isError"], true, "{result}");
}
