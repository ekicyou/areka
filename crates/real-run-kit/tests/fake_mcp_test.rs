//! MCP の各分岐を偽の子と本物の受け口で確かめる（spec: areka-P0-real-run-mcp-kit タスク 5.2・
//! 要件 4.2〜4.9・9.2・10.1・10.5）。
//!
//! 偽の子 `real-run-fake-areka` は本物の `areka_mcp::start` を呼ぶので、待受の行の綴りが受け口と
//! 窓口の間でずれたらここが赤になる。生かす子は最後に `window` を置いて利用者と同じ閉じ方で閉じる
//! （`hang` だと破棄が締切まで待ってから止める）。試験の側の環境変数は変えない。

use std::time::Duration;

use real_run_kit::{EndedBy, McpError, RealRun, Run};

/// 偽の子を手順つきで立てる（32bit の仲介は揃えない）。
fn fake(steps: &str) -> RealRun {
    RealRun::new(env!("CARGO_BIN_EXE_real-run-fake-areka"))
        .without_host32_helper()
        .env("REAL_RUN_FAKE_STEPS", steps)
}

/// 閉じて利用者と同じ閉じ方だったことを確かめる。
fn close_ok(mut run: Run) {
    assert_eq!(run.close().by, EndedBy::UserClose);
}

/// 4.2・4.6: 成功の答えは文と失敗でない印。つなぎを使い回して 2 回目も通る。
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

/// 4.6: ツールの失敗の印を成功と見分ける。
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

/// 4.4: 記録を警告より上に絞っても、窓口が待受の行の段を開けるので観測できる。
#[test]
fn listen_is_seen_under_warn_filter() {
    let mut run = fake("listen;window").rust_log("warn").spawn();
    let reply = run.call("echo", r#"{"text":"見える"}"#).expect("答える");
    assert_eq!(reply.text, "見える");
    close_ok(run);
}

/// 4.5・9.2: 選んだ番号を先に取られていたら、その行と番号を添えた失敗。
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

/// 4.5: 待ち受けない子は、その行を添えた失敗。
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

/// 4.7: 待受の直後に受け口を閉じて生き続ける子は、つながらない（子の終わりではない）。
/// 子は窓を持たないので、破棄の締切が短くなるよう締切を縮める。
#[test]
fn dropped_listener_is_unreachable() {
    let mut run = fake("listen-drop").deadline(Duration::from_secs(5)).spawn();
    let port = run.mcp_port();
    match run.call("echo", "{}") {
        Err(McpError::Unreachable { target, .. }) => {
            assert_eq!(target, format!("127.0.0.1:{port}"));
        }
        other => panic!("Unreachable のはず: {other:?}"),
    }
}

/// 4.8: 答える前に終わる子は、子の終わりと終了コード。
#[test]
fn child_dying_in_call_is_child_exited() {
    let mut run = fake("listen;window").spawn();
    match run.call("echo", r#"{"die":true}"#) {
        Err(McpError::ChildExited(e)) => assert_eq!(e.status.code(), Some(9)),
        other => panic!("ChildExited のはず: {other:?}"),
    }
}

/// 4.9: 3 回目に文字列が出るツールへ呼び直して見つける（1 回目は自分で呼んで空を確かめる）。
#[test]
fn call_until_finds_on_third_call() {
    let mut run = fake("listen;window").spawn();
    let args = r#"{"text":"出た","after":3}"#;
    assert_eq!(run.call("echo", args).expect("答える").text, "");
    let reply = run.call_until("echo", args, "出た").expect("答える");
    assert_eq!(reply.text, "出た");
    close_ok(run);
}
