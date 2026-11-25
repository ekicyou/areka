//! `dump_balloon` の決定論テスト。
//!
//! 結線状態の無い空の World で呼ぶと、その場で `NG:This ghost has no window`（isError: true・
//! content は本文 1 つ）を返す。

use areka_mcp::tools::{ToolCall, ToolRequest};

use super::super::dump_surface::dump_surface_tests::assert_success_record;
use super::*;
use crate::mcp::kanade_fake_test_support::{self, ghost, text};

#[test]
fn answers_no_window_with_an_empty_world() {
    let ghost = ghost();
    let args = Args {
        scope: Some(0),
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpBalloon(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answered = kanade_fake_test_support::answer(&pending);
    assert_eq!(answered.content, text("NG:This ghost has no window"));
    assert!(answered.is_error);
}

/// 文字の層の無い 1×1 の仕事の成功の記録の欄。
#[test]
fn success_record_carries_ui_us_and_encode_us() {
    assert_success_record(balloon_job(0, vec![0, 0, 0, 0], (1, 1), None, None));
}
