//! `get_log` の決定論テスト。
//!
//! 空の World と引数で呼ぶと `NG:not implemented yet`（isError: true）を返す
//! （空の World で答えられる＝ゴーストに何もさせていない・要件 5.1・5.2・5.4）。

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

#[test]
fn answers_not_implemented_yet_with_an_empty_world() {
    let args = Args {
        log_type: Some("error".to_string()),
        ghost_name: Some("Emily/Phase4.5".to_string()),
        since_id: Some(0),
        max_count: Some(10),
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetLog(args.clone()));

    handle(&mut World::new(), args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:not implemented yet".to_string())]
    );
    assert!(answer.outcome.is_error);
}
