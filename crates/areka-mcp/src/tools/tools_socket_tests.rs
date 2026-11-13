//! `tools` の実ソケットのテスト（一覧の一致・`-32602`・橋・待ちの間の `ping`）。
//!
//! 登録表は本番と同じ [`entrances`] で組み、受け手はテストのスレッドの偽物。ネットへは出ない
//! （`127.0.0.1` の空きポートだけ＝要件 8.1）。測った版は rmcp 3.9.0。

use std::thread;

use serde_json::{Value, json};

use super::{
    ToolCall, dump_balloon, dump_surface, entrances, get_expression_table, get_log, get_property,
    get_status, outcome, raise_event, reload, sakurascript,
};
use crate::testkit::{
    LONG_WAIT, RECV_WAIT, Response, SAVED_LIST, assert_invalid_params, assert_no_request,
    call_params, post_legacy, post_stateless, serve,
};

/// (要件 1.1〜1.6) `tools/list` の `tools` が保存した JSON と配列ごと一致する（並び・欄の過不足を 1 度に見る）。
/// 旧式と無状態版の両方。無状態版の `ttlMs`・`cacheScope` は一覧の結果の欄で、`tools` の各要素には無い。
#[test]
fn tools_list_matches_saved_ssp_json() {
    let text = std::fs::read_to_string(SAVED_LIST).expect("保存した JSON が読めない");
    let saved: Value = serde_json::from_str(&text).expect("保存した JSON が JSON でない");
    let (registry, _rx) = entrances(LONG_WAIT);
    let (_server, addr) = serve(registry);

    let legacy = Response::result(&post_legacy(addr, "tools/list", 1, None));
    assert_eq!(legacy["tools"], saved["tools"], "旧式");

    let stateless = Response::result(&post_stateless(addr, "tools/list", 2, json!({})));
    assert_eq!(stateless["tools"], saved["tools"], "無状態版");
    assert_eq!(stateless["ttlMs"], 0, "{stateless}");
    assert_eq!(stateless["cacheScope"], "private", "{stateless}");
}

/// (要件 2.1・2.7・2.8) 10 本のどれでもない名前は `-32602`・受け手へ届かない。
#[test]
fn unknown_name_is_invalid_params() {
    assert_invalid_params(
        entrances(LONG_WAIT),
        "no_such_tool",
        json!({}),
        "tool not found",
    );
}

/// (要件 2.2・2.7・2.8) 必須の欄（`sakurascript` の `script`）が無いと `-32602`・受け手へ届かない。
#[test]
fn missing_required_is_invalid_params() {
    assert_invalid_params(
        entrances(LONG_WAIT),
        "sakurascript",
        json!({ "ghost_name": "emo" }),
        "missing required argument: script",
    );
}

/// (要件 2.4・2.7・2.8) 型違い（`sakurascript` の `strict` に文字列）は `-32602`・受け手へ届かない。
#[test]
fn wrong_type_is_invalid_params() {
    assert_invalid_params(
        entrances(LONG_WAIT),
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

    assert_eq!(
        Response::result(&post_legacy(addr, "ping", 2, None)),
        json!({})
    );
    let list = Response::result(&post_legacy(addr, "tools/list", 3, None));
    assert_eq!(list["tools"].as_array().map(Vec::len), Some(10), "{list}");
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
