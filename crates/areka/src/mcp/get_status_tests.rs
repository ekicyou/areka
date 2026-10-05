//! `get_status` の決定論テスト。
//!
//! 置き場に問い合わせ先が無い（空の World）と `NG:Status is not available`（isError: true）で
//! その場で答える（要件 3.6）。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

#[test]
fn answers_not_available_without_a_kanade_in_the_slot() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetStatus(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:Status is not available".to_string())]
    );
    assert!(answer.outcome.is_error);
}
