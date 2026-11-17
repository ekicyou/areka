//! `mcp` の兄弟のテスト。子も受け口も使わない試験（答えの写し＝文の片の繋ぎ方と失敗の印・引数の文の
//! 読み方・失敗の表示の文・名乗りの訳の文）、手で書いた出力のファイルと生かしておく子・終わった子での試験
//! （待受の観測の分岐・`call_until` が待受の失敗をそのまま返す）、出力のファイルを書かずに生かしておく
//! 子だけでの試験（答えが 1 度も無いままの `call_until` の締切）、同じプロセスに立てた本物の受け口
//! （`serve`）での試験（受け口が落ちた後の `Unreachable`・誤りの答えの `Refused`・つなぎの使い回し・
//! 最後の答えを添えた `call_until` の締切）を確かめる。偽の子との試験は `tests/fake_mcp_test.rs`。

use super::*;
use crate::test_support::living_child;
use rmcp::model::{CallToolResult, ContentBlock};
use std::process::Command;
use temp_path_kit::TempPath;

/// 見回りの間隔（試験は短く回す）。
const POLL: Duration = Duration::from_millis(10);

/// 締切だけを変え、見回り・呼び直し・猶予は短くした時間。
fn waits(deadline: Duration) -> Limits {
    Limits {
        deadline,
        poll: POLL,
        call_retry: POLL,
        exit_grace: POLL,
        ..Limits::default()
    }
}

#[test]
fn text_parts_are_joined_by_newline_and_images_skipped() {
    let result = CallToolResult::success(vec![
        ContentBlock::text("一つめ"),
        ContentBlock::image("AAAA", "image/png"),
        ContentBlock::text("二つめ"),
    ]);
    assert_eq!(
        reply_of(result),
        ToolReply {
            text: "一つめ\n二つめ".to_string(),
            is_error: false,
        }
    );
}

#[test]
fn missing_is_error_means_success_and_true_means_failure() {
    let mut none = CallToolResult::success(vec![ContentBlock::text("x")]);
    none.is_error = None;
    assert!(!reply_of(none).is_error);
    let failed = CallToolResult::error(vec![ContentBlock::text("だめ")]);
    assert_eq!(failed.is_error, Some(true));
    assert_eq!(
        reply_of(failed),
        ToolReply {
            text: "だめ".to_string(),
            is_error: true,
        }
    );
}

#[test]
fn object_args_are_read_into_a_map() {
    let map = parse_args(r#"{"lines": 3, "name": "えみり"}"#);
    assert_eq!(map.len(), 2);
    assert_eq!(map["lines"], serde_json::json!(3));
    assert_eq!(map["name"], serde_json::json!("えみり"));
    assert!(parse_args("{}").is_empty());
}

#[test]
#[should_panic(expected = "JSON の object でない")]
fn non_object_args_panic() {
    parse_args("[1, 2]");
}

#[test]
#[should_panic(expected = "JSON として読めない")]
fn broken_json_args_panic() {
    parse_args(r#"{"lines": "#);
}

#[test]
fn failed_listen_line_is_returned_whole_with_port() {
    let dir = TempPath::under_target("mcp-listen-failed");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let line = "2026-10-18T00:00:00Z ERROR areka_mcp::server: MCP: どの候補のポートも束ねられなかった（待ち受けない） tried=[9821]";
    std::fs::write(&out, "").unwrap();
    std::fs::write(&err, format!("boot\n{line}\nnext\n")).unwrap();
    let mut child = living_child();
    let mut mcp = McpChannel::new(9821);
    let got = mcp.call(
        &mut child.0,
        [&out, &err],
        "echo",
        "{}",
        &waits(Duration::from_secs(30)),
    );
    drop(child);
    match got {
        Err(McpError::ListenFailed { port, line: got }) => {
            assert_eq!(port, 9821);
            assert_eq!(got, line);
        }
        other => panic!("ListenFailed でない: {other:?}"),
    }
}

#[test]
fn listen_off_line_is_listen_failed() {
    let dir = TempPath::under_target("mcp-listen-off");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(&out, "INFO MCP: 待ち受けない（AREKA_MCP_PORT が 0）\n").unwrap();
    let mut child = living_child();
    let got = McpChannel::new(9822).call(
        &mut child.0,
        [&out, &err],
        "echo",
        "{}",
        &waits(Duration::from_secs(30)),
    );
    drop(child);
    assert!(
        matches!(got, Err(McpError::ListenFailed { port: 9822, ref line }) if line.contains("待ち受けない（AREKA_MCP_PORT が 0）")),
        "{got:?}"
    );
}

#[test]
fn no_listen_line_until_deadline_is_unreachable() {
    let dir = TempPath::under_target("mcp-listen-none");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let mut child = living_child();
    let got = McpChannel::new(9823).call(
        &mut child.0,
        [&out, &err],
        "echo",
        "{}",
        &waits(Duration::from_millis(100)),
    );
    drop(child);
    match got {
        Err(McpError::Unreachable { target, .. }) => assert_eq!(target, "127.0.0.1:9823"),
        other => panic!("Unreachable でない: {other:?}"),
    }
}

#[test]
fn exited_child_before_listen_is_child_exited() {
    let dir = TempPath::under_target("mcp-listen-exited");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let mut child = Command::new("cmd").args(["/c", "exit 3"]).spawn().unwrap();
    let got = McpChannel::new(9824).call(
        &mut child,
        [&out, &err],
        "echo",
        "{}",
        &waits(Duration::from_secs(30)),
    );
    match got {
        Err(McpError::ChildExited(e)) => assert_eq!(e.status.code(), Some(3)),
        other => panic!("ChildExited でない: {other:?}"),
    }
}

#[test]
fn display_names_the_contents() {
    let failed = McpError::ListenFailed {
        port: 9821,
        line: "MCP: 待ち受けない".into(),
    };
    assert!(failed.to_string().contains("9821"));
    assert!(failed.to_string().contains("MCP: 待ち受けない"));
    let unreachable = McpError::Unreachable {
        target: "127.0.0.1:9821".into(),
        reason: "相手が居ない".into(),
    };
    assert!(unreachable.to_string().contains("127.0.0.1:9821"));
    assert!(unreachable.to_string().contains("相手が居ない"));
    let refused = McpError::Refused {
        code: -32602,
        message: "引数が合わない".into(),
    };
    assert!(refused.to_string().contains("-32602"));
    assert!(refused.to_string().contains("引数が合わない"));
}

#[test]
fn call_until_returns_listen_failed_as_is() {
    let dir = TempPath::under_target("mcp-until-off");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(&out, "").unwrap();
    std::fs::write(&err, "INFO MCP: 待ち受けない（AREKA_MCP_PORT が 0）\n").unwrap();
    let mut child = living_child();
    let got = McpChannel::new(9825).call_until(
        &mut child.0,
        [&out, &err],
        "get_log",
        "{}",
        "never",
        &waits(Duration::from_secs(30)),
    );
    drop(child);
    assert!(
        matches!(got, Err(McpError::ListenFailed { port: 9825, .. })),
        "{got:?}"
    );
}

/// 同じプロセスに本物の受け口を立てる。ツール `echo` はいつも `text` を答える。入力の `text` は文字の型
/// なので、型の違う引数には受け口が JSON-RPC の誤り（`-32602`）を答える。
fn serve(text: &'static str) -> areka_mcp::McpServer {
    let spec = areka_mcp::ToolSpec {
        name: "echo".to_owned(),
        title: None,
        description: None,
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "text": { "type": "string" } }
        })
        .as_object()
        .unwrap()
        .clone(),
    };
    let mut registry = areka_mcp::ToolRegistry::default();
    registry.register(
        spec,
        std::sync::Arc::new(move |_| {
            Box::pin(async move {
                areka_mcp::ToolOutcome {
                    content: vec![areka_mcp::ToolContent::Text(text.to_owned())],
                    is_error: false,
                }
            })
        }),
    );
    areka_mcp::start(&[0], registry, None)
}

/// audit-test-kits 指摘 28: つないだ後に受け口が落ち、子は生きている＝輸送の失敗の猶予の後に `Unreachable`。
/// 猶予は締切を越えない（猶予を締切より長くして確かめる）。
#[test]
fn a_dropped_server_with_a_living_child_is_unreachable_within_the_grace() {
    let server = serve("先の答え");
    let port = server.local_addr().unwrap().port();
    let dir = TempPath::under_target("mcp-dropped-server");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(
        &out,
        format!("INFO MCP: 待受を始めた url=http://127.0.0.1:{port}/api/mcp/v1\n"),
    )
    .unwrap();
    let mut child = living_child();
    let mut mcp = McpChannel::new(port);
    let mut l = waits(Duration::from_secs(3));
    let first = mcp.call(&mut child.0, [&out, &err], "echo", "{}", &l);
    assert_eq!(first.expect("答える").text, "先の答え");
    drop(server);
    l.exit_grace = Duration::from_secs(30);
    let started = Instant::now();
    let got = mcp.call(&mut child.0, [&out, &err], "echo", "{}", &l);
    let took = started.elapsed();
    drop(child);
    match got {
        Err(McpError::Unreachable { target, .. }) => {
            assert_eq!(target, format!("127.0.0.1:{port}"))
        }
        other => panic!("Unreachable でない: {other:?}"),
    }
    assert!(
        took < Duration::from_secs(10),
        "猶予が締切を越えた: {took:?}"
    );
}

/// audit-test-kits 指摘 3（audit-test-kits 決め D1）: JSON-RPC の誤りの答えは `Refused` で、輸送の失敗の猶予を待たずに返る（猶予 30 秒・
/// 締切 10 秒にして、猶予の道へ落ちれば締切まで待って `Unreachable` になる形で確かめる）。
#[test]
fn an_mcp_error_reply_is_refused_without_waiting_for_the_grace() {
    let server = serve("先の答え");
    let port = server.local_addr().unwrap().port();
    let dir = TempPath::under_target("mcp-refused");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(
        &out,
        format!("INFO MCP: 待受を始めた url=http://127.0.0.1:{port}/api/mcp/v1\n"),
    )
    .unwrap();
    let mut child = living_child();
    let mut mcp = McpChannel::new(port);
    let mut l = waits(Duration::from_secs(10));
    l.exit_grace = Duration::from_secs(30);
    // 先につないでおき、測る時間を誤りの答えの道だけにする。
    let first = mcp.call(&mut child.0, [&out, &err], "echo", "{}", &l);
    assert_eq!(first.expect("答える").text, "先の答え");
    let started = Instant::now();
    let got = mcp.call(&mut child.0, [&out, &err], "echo", r#"{"text":5}"#, &l);
    let took = started.elapsed();
    drop(child);
    drop(server);
    match got {
        Err(McpError::Refused { code, message }) => assert_eq!(code, -32602, "{message}"),
        other => panic!("Refused でない: {other:?}"),
    }
    assert!(took < Duration::from_secs(5), "猶予を待った: {took:?}");
}

/// D-10: 答え（成功も JSON-RPC の誤りも）の後はつなぎを持ち続け、次の呼び出しが使い回す。
#[test]
fn the_link_is_kept_after_a_reply_and_after_a_refusal() {
    let server = serve("答え");
    let port = server.local_addr().unwrap().port();
    let dir = TempPath::under_target("mcp-link-kept");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(
        &out,
        format!("INFO MCP: 待受を始めた url=http://127.0.0.1:{port}/api/mcp/v1\n"),
    )
    .unwrap();
    let mut child = living_child();
    let mut mcp = McpChannel::new(port);
    let l = waits(Duration::from_secs(10));
    let first = mcp.call(&mut child.0, [&out, &err], "echo", "{}", &l);
    assert_eq!(first.expect("答える").text, "答え");
    assert!(mcp.link.is_some(), "成功の答えの後につなぎを捨てた");
    let refused = mcp.call(&mut child.0, [&out, &err], "echo", r#"{"text":5}"#, &l);
    assert!(
        matches!(refused, Err(McpError::Refused { .. })),
        "{refused:?}"
    );
    assert!(mcp.link.is_some(), "誤りの答えの後につなぎを捨てた");
    drop(child);
    drop(server);
}

/// audit-test-kits 指摘 28: 名乗りの確かめで別の相手だけが答えたら、その番号と名乗りを訳に書く。
#[test]
fn another_server_answering_is_named_in_the_reason() {
    let reason = not_found_reason(&[(9801, "ssp-mcp".to_string())]);
    assert!(reason.contains("別の相手が名乗った"), "{reason}");
    assert!(
        reason.contains("9801") && reason.contains("ssp-mcp"),
        "{reason}"
    );
    assert_eq!(not_found_reason(&[]), "areka と名乗る相手が居ない");
}

#[test]
#[should_panic(expected = "最後の答えの印")]
fn call_until_deadline_panics_with_the_last_reply() {
    let server = serve("最後の答えの印");
    let port = server.local_addr().unwrap().port();
    let dir = TempPath::under_target("mcp-until-deadline");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(
        &out,
        format!("INFO MCP: 待受を始めた url=http://127.0.0.1:{port}/api/mcp/v1\n"),
    )
    .unwrap();
    let mut child = living_child();
    // 呼び直しの間隔を締切より長くし、眠りが締切ちょうどで覚める形を作る。
    let mut w = waits(Duration::from_secs(3));
    w.call_retry = Duration::from_secs(30);
    let got =
        McpChannel::new(port).call_until(&mut child.0, [&out, &err], "echo", "{}", "never", &w);
    drop(server);
    eprintln!("panic せずに返った: {got:?}");
}

#[test]
#[should_panic(expected = "答え無し")]
fn call_until_deadline_before_any_reply_panics() {
    let dir = TempPath::under_target("mcp-until-no-listen");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let mut child = living_child();
    let got = McpChannel::new(9826).call_until(
        &mut child.0,
        [&out, &err],
        "echo",
        "{}",
        "never",
        &waits(Duration::from_millis(100)),
    );
    eprintln!("panic せずに返った: {got:?}");
}
