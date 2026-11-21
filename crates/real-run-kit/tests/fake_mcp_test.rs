//! MCP の各分岐を偽の子と本物の受け口で確かめる。
//!
//! 偽の子 `real-run-fake-areka` は本物の `areka_mcp::start` を呼ぶので、待受の行の綴りが受け口と
//! 窓口の間でずれたらここが赤になる。生かす子は最後に `window` を置いて利用者と同じ閉じ方で閉じる
//! （`hang` だと破棄が締切まで待ってから止め、panic の途中でなければ panic する）。試験の側の
//! 環境変数は変えない。

mod common;

use common::fake;
use real_run_kit::{EndedBy, McpError, Run};

/// 閉じて利用者と同じ閉じ方だったことを確かめる。
fn close_ok(mut run: Run) {
    assert_eq!(run.close().by, EndedBy::UserClose);
}

/// 成功の答えは文と失敗でない印。2 回目も通る（つなぎの使い回しは `mcp_tests.rs` が確かめる）。
#[test]
fn call_returns_text() {
    let mut run = fake("listen;window").spawn();
    let reply = run
        .call("echo", r#"{"text":"こんにちは"}"#)
        .expect("答える");
    assert_eq!(reply.text, "こんにちは");
    assert!(!reply.is_error);
    let again = run.call("echo", r#"{"text":"二度め"}"#).expect("答える");
    assert_eq!(again.text, "二度め");
    close_ok(run);
}

/// ツールの失敗の印を成功と見分ける。
#[test]
fn tool_error_flag_is_kept() {
    let mut run = fake("listen;window").spawn();
    let reply = run
        .call("echo", r#"{"text":"だめ","error":true}"#)
        .expect("答える");
    assert_eq!(reply.text, "だめ");
    assert!(reply.is_error);
    close_ok(run);
}

/// 記録を警告より上に絞っても、窓口が待受の行の段を開けるので観測できる。
#[test]
fn listen_is_seen_under_warn_filter() {
    let mut run = fake("listen;window").rust_log("warn").spawn();
    let reply = run.call("echo", r#"{"text":"見える"}"#).expect("答える");
    assert_eq!(reply.text, "見える");
    close_ok(run);
}

/// 選んだ番号を先に取られていたら、その行と番号を添えた失敗。
#[test]
fn taken_port_is_listen_failed() {
    let mut run = fake("listen-taken;window").spawn();
    let port = run.mcp_port();
    match run.call("echo", "{}") {
        Err(McpError::ListenFailed { port: got, line }) => {
            assert_eq!(got, port);
            assert!(
                line.contains("どの候補のポートも束ねられなかった"),
                "{line}"
            );
        }
        other => panic!("ListenFailed のはず: {other:?}"),
    }
    close_ok(run);
}

/// 待ち受けない子は、その行を添えた失敗。
#[test]
fn listen_off_is_listen_failed() {
    let mut run = fake("listen-off;window").spawn();
    match run.call("echo", "{}") {
        Err(McpError::ListenFailed { line, .. }) => {
            assert!(line.contains("MCP: 待ち受けない"), "{line}");
        }
        other => panic!("ListenFailed のはず: {other:?}"),
    }
    close_ok(run);
}

/// 待受の直後に受け口を閉じて生き続ける子は、つながらない（子の終わりではない）。
#[test]
fn dropped_listener_is_unreachable() {
    let mut run = fake("listen-drop;window").spawn();
    let port = run.mcp_port();
    match run.call("echo", "{}") {
        Err(McpError::Unreachable { target, .. }) => {
            assert_eq!(target, format!("127.0.0.1:{port}"));
        }
        other => panic!("Unreachable のはず: {other:?}"),
    }
    close_ok(run);
}

/// 答える前に終わる子は、子の終わりと終了コード。
#[test]
fn child_dying_in_call_is_child_exited() {
    let mut run = fake("listen;window").spawn();
    match run.call("echo", r#"{"die":true}"#) {
        Err(McpError::ChildExited(e)) => assert_eq!(e.status.code(), Some(9)),
        other => panic!("ChildExited のはず: {other:?}"),
    }
}

/// 3 回目に文字列が出るツールへ呼び直して見つける（1 回目は自分で呼んで空を確かめる）。
#[test]
fn call_until_finds_on_third_call() {
    let mut run = fake("listen;window").spawn();
    let args = r#"{"text":"出た","after":3}"#;
    assert_eq!(run.call("echo", args).expect("答える").text, "");
    let reply = run.call_until("echo", args, "出た").expect("答える");
    assert_eq!(reply.text, "出た");
    close_ok(run);
}

/// 受け口が JSON-RPC の誤りを答えたら `Refused`。続く正しい呼び出しも通る（つなぎの
/// 使い回しと、輸送の失敗の猶予を待たないことは `mcp_tests.rs` が確かめる）。
#[test]
fn an_mcp_error_reply_is_refused_and_the_next_call_still_answers() {
    let mut run = fake("listen;window").spawn();
    run.call("echo", r#"{"text":"先"}"#).expect("答える");
    match run.call("echo", r#"{"text":5}"#) {
        Err(McpError::Refused { code, message }) => assert_eq!(code, -32602, "{message}"),
        other => panic!("Refused のはず: {other:?}"),
    }
    let again = run.call("echo", r#"{"text":"後"}"#).expect("続けて答える");
    assert_eq!(again.text, "後");
    close_ok(run);
}

/// 呼び手の `env` で積んだ `AREKA_MCP_PORT` が、子の待ち受ける番号であり窓口が話す番号。
#[test]
fn an_env_override_of_the_mcp_port_is_the_port_the_kit_talks_to() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .expect("空きの番号")
        .port();
    let mut run = fake("listen;window")
        .env("AREKA_MCP_PORT", port.to_string())
        .spawn();
    assert_eq!(run.mcp_port(), port);
    let reply = run
        .call("echo", r#"{"text":"上書き"}"#)
        .expect("上書きの番号で答える");
    assert_eq!(reply.text, "上書き");
    close_ok(run);
}
