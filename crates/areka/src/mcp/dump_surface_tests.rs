//! `dump_surface` の決定論テスト（窓の無い World）。
//!
//! 結線状態の無い空の World で呼ぶと、その場で `NG:This ghost has no window`（isError: true・
//! content は本文 1 つ＝画像なし）を返す（要件 4.5・4.8・7.5）。描画を通る場合は GPU のテストが固定する。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

#[test]
fn answers_no_window_at_once_with_an_empty_world() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        scope: Some(0),
        surface: Some(0),
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:This ghost has no window".to_string())]
    );
    assert!(answer.outcome.is_error);
}
