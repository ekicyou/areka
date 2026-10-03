//! `tools` の実ソケットのテスト（一覧の一致・`-32602`・橋・待ちの間の `ping`）。
//!
//! 登録表は本番と同じ [`entrances`] で組み、受け手はテストのスレッドの偽物。ネットへは出ない
//! （`127.0.0.1` の空きポートだけ＝要件 8.1）。測った版は rmcp 3.5.0。

use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use super::{
    ToolCall, ToolRequest, dump_balloon, dump_surface, entrances, get_expression_table, get_log,
    get_property, get_status, outcome, raise_event, reload, sakurascript,
};
use crate::testkit::{Response, post_rpc, rpc, serve};

/// 保存した JSON（テストだけが読む＝要件 7.6）。
const SAVED_LIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../doc/ssp-mcp/tools-list-ssp-2.9.05.json"
);
/// 受け手の要求・応答を待つ上限。届かなければテストを止めずに失敗させる（正しさは待ち時間に頼らない）。
const RECV_WAIT: Duration = Duration::from_secs(10);
/// 返事の上限。待ちの間の `ping` のテストが上限で終わらないよう長く取る（返事は必ず放す）。
const LONG_WAIT: Duration = Duration::from_secs(30);
/// 無状態版の版。
const STATELESS: &str = "2026-07-28";

/// 無状態版（`2026-07-28`）の要求に付ける本文の `_meta`。
fn stateless_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": STATELESS,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "areka-test", "version": "0" }
    })
}

/// 旧式の経路（ヘッダも `_meta` も無い）で 1 件送る。
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

/// `tools/call` の `params`。
fn call_params(name: &str, arguments: Value) -> Value {
    json!({ "name": name, "arguments": arguments })
}

/// 200 で JSON 1 件の応答を読み、`result` を返す（`error` があれば失敗）。
fn result_of(response: &Response) -> Value {
    assert_eq!(response.status, 200, "{response:?}");
    let value = response.json();
    assert!(value.get("error").is_none(), "{value}");
    value["result"].clone()
}

/// 受け手に要求が 1 件も届いていない（検査で弾いた要求はアプリ本体へ届かない）。
/// 検査は handler を呼ぶ前に同期で済み、応答はその後に返るので、ここで覗けば十分。
fn assert_no_request(rx: &Receiver<ToolRequest>) {
    match rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        Ok(request) => panic!("受け手に要求が届いた: {:?}", request.call),
        Err(TryRecvError::Disconnected) => panic!("受け口が閉じている"),
    }
}

/// 旧式と無状態版の両方で同じ `tools/call` を送り、`-32602` の `error` と HTTP の状態を固定する。
/// どちらも `result` は無く、受け手への要求は 0 件。
fn assert_invalid_params(name: &str, arguments: Value, message: &str) {
    let (registry, rx) = entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);
    let params = call_params(name, arguments);
    // 旧式の経路は 200、無状態版の経路は 400（rmcp 3.5.0 で測った値＝差の一覧へ）。
    for (path, status, response) in [
        (
            "旧式",
            200,
            post_legacy(addr, "tools/call", 1, Some(params.clone())),
        ),
        (
            "無状態版",
            400,
            post_stateless(addr, "tools/call", 1, params.clone()),
        ),
    ] {
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

/// (要件 1.1〜1.6) `tools/list` の `tools` が保存した JSON と配列ごと一致する（並び・欄の過不足を 1 度に見る）。
/// 旧式と無状態版の両方。無状態版の `ttlMs`・`cacheScope` は一覧の結果の欄で、`tools` の各要素には無い。
#[test]
fn tools_list_matches_saved_ssp_json() {
    let text = std::fs::read_to_string(SAVED_LIST).expect("保存した JSON が読めない");
    let saved: Value = serde_json::from_str(&text).expect("保存した JSON が JSON でない");
    let (registry, _rx) = entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);

    let legacy = result_of(&post_legacy(addr, "tools/list", 1, None));
    assert_eq!(legacy["tools"], saved["tools"], "旧式");

    let stateless = result_of(&post_stateless(addr, "tools/list", 2, json!({})));
    assert_eq!(stateless["tools"], saved["tools"], "無状態版");
    assert_eq!(stateless["ttlMs"], 0, "{stateless}");
    assert_eq!(stateless["cacheScope"], "private", "{stateless}");
}

/// (要件 2.1・2.7・2.8) 10 本のどれでもない名前は `-32602`・受け手へ届かない。
#[test]
fn unknown_name_is_invalid_params() {
    assert_invalid_params("no_such_tool", json!({}), "tool not found");
}

/// (要件 2.2・2.7・2.8) 必須の欄（`sakurascript` の `script`）が無いと `-32602`・受け手へ届かない。
#[test]
fn missing_required_is_invalid_params() {
    assert_invalid_params(
        "sakurascript",
        json!({ "ghost_name": "emo" }),
        "missing required argument: script",
    );
}

/// (要件 2.4・2.7・2.8) 型違い（`sakurascript` の `strict` に文字列）は `-32602`・受け手へ届かない。
#[test]
fn wrong_type_is_invalid_params() {
    assert_invalid_params(
        "sakurascript",
        json!({ "script": "\\e", "strict": "yes" }),
        "argument strict must be boolean",
    );
}

/// (要件 5.1・5.3・5.4) 9 本それぞれが、型の付いた引数のまま受け手へ届き、
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

    let (registry, rx) = entrances(LONG_WAIT);
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
        let result = result_of(&post_legacy(
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

/// (要件 6.3・6.8) 返事を止めた `tools/call` の待ちの間に、`ping` と `tools/list` が答える。
/// 受け手が要求を受け取ったことを合図に問い、答えを確かめてから返事を放す。
#[test]
fn ping_answers_while_a_call_waits() {
    let (registry, rx) = entrances(LONG_WAIT);
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

    assert_eq!(result_of(&post_legacy(addr, "ping", 2, None)), json!({}));
    let list = result_of(&post_legacy(addr, "tools/list", 3, None));
    assert_eq!(list["tools"].as_array().map(Vec::len), Some(10), "{list}");
    assert!(!caller.is_finished(), "返事を放す前に呼び出しが終わった");

    request.reply.send(outcome::ng("not implemented yet"));
    let result = result_of(
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
