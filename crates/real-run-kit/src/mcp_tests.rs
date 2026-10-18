//! `mcp` の兄弟のテスト。答えの写し（文の片の繋ぎ方・失敗の印）・引数の文の読み方・
//! 手で書いた出力のファイルでの待受の観測を確かめる。偽の子と本物の受け口との試験は 5.2。

use super::*;
use rmcp::model::{CallToolResult, ContentBlock};
use std::process::{Command, Stdio};
use temp_path_kit::TempPath;

/// 見回りの間隔（試験は短く回す）。
const POLL: Duration = Duration::from_millis(10);

/// 締切まで生き続ける子（終わりの分岐へ入らない相手）。試験の終わりに止める。
fn living_child() -> Child {
    Command::new("ping")
        .args(["-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

/// 締切だけを変え、見回り・呼び直し・猶予は短くした時間。
fn waits(deadline: Duration) -> Waits {
    Waits {
        deadline,
        poll: POLL,
        call_retry: POLL,
        exit_grace: POLL,
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
        &mut child,
        [&out, &err],
        "echo",
        "{}",
        waits(Duration::from_secs(30)),
    );
    child.kill().unwrap();
    child.wait().unwrap();
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
        &mut child,
        [&out, &err],
        "echo",
        "{}",
        waits(Duration::from_secs(30)),
    );
    child.kill().unwrap();
    child.wait().unwrap();
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
        &mut child,
        [&out, &err],
        "echo",
        "{}",
        waits(Duration::from_millis(100)),
    );
    child.kill().unwrap();
    child.wait().unwrap();
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
        waits(Duration::from_secs(30)),
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
}

#[test]
fn call_until_returns_listen_failed_as_is() {
    let dir = TempPath::under_target("mcp-until-off");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(&err, "INFO MCP: 待ち受けない（AREKA_MCP_PORT が 0）\n").unwrap();
    let mut child = living_child();
    let got = McpChannel::new(9825).call_until(
        &mut child,
        [&out, &err],
        "get_log",
        "{}",
        "never",
        waits(Duration::from_secs(30)),
    );
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(
        matches!(got, Err(McpError::ListenFailed { port: 9825, .. })),
        "{got:?}"
    );
}

/// 落ちても止める子（`should_panic` の試験で巻き戻しの間に止める）。
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 同じ過程に本物の受け口を立てる。ツール `echo` はいつも `text` を答える。
fn serve(text: &'static str) -> areka_mcp::McpServer {
    let spec = areka_mcp::ToolSpec {
        name: "echo".to_owned(),
        title: None,
        description: None,
        input_schema: serde_json::json!({ "type": "object" })
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

#[test]
#[should_panic(expected = "最後の答えの印")]
fn call_until_deadline_panics_with_the_last_reply() {
    let server = serve("最後の答えの印");
    let port = server.local_addr().unwrap().port();
    let dir = TempPath::under_target("mcp-until-deadline");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    std::fs::write(
        &out,
        format!("INFO MCP: 待受を始めた addr=127.0.0.1:{port}\n"),
    )
    .unwrap();
    let mut child = KillOnDrop(living_child());
    // 呼び直しの間隔を締切より長くし、眠りが締切ちょうどで覚める形を作る。
    let mut w = waits(Duration::from_secs(3));
    w.call_retry = Duration::from_secs(30);
    let got =
        McpChannel::new(port).call_until(&mut child.0, [&out, &err], "echo", "{}", "never", w);
    drop(server);
    eprintln!("panic せずに返った: {got:?}");
}

#[test]
#[should_panic(expected = "答え無し")]
fn call_until_deadline_before_any_reply_panics() {
    let dir = TempPath::under_target("mcp-until-no-listen");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let mut child = KillOnDrop(living_child());
    let got = McpChannel::new(9826).call_until(
        &mut child.0,
        [&out, &err],
        "echo",
        "{}",
        "never",
        waits(Duration::from_millis(100)),
    );
    eprintln!("panic せずに返った: {got:?}");
}
