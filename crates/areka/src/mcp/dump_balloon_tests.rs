//! `dump_balloon` の決定論テスト。
//!
//! 結線状態の無い空の World で呼ぶと、その場で `NG:This ghost has no window`（isError: true・
//! content は本文 1 つ）を返す（要件 4.5・4.8・7.5）。

use std::path::PathBuf;
use std::time::Duration;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::super::dump_surface::finish;
use super::*;

#[test]
fn answers_no_window_with_an_empty_world() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        scope: Some(0),
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpBalloon(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:This ghost has no window".to_string())]
    );
    assert!(answer.outcome.is_error);
}

/// 文字の層の無い 1×1 の仕事を UI 時間 1,234 µs で符号化のスレッドの体に通すと、成功の記録が 1 件で
/// `ui_us` が 1234・`encode_us` が数として読める（要件 5.3・7.1）。
#[test]
fn success_record_carries_ui_us_and_encode_us() {
    let job = balloon_job(0, vec![0, 0, 0, 0], (1, 1), None);

    let (answered, events) =
        log_capture_kit::capture(|| finish(TOOL, 0, job, Duration::from_micros(1234)));

    assert!(!answered.is_error);
    let records: Vec<_> = events
        .iter()
        .filter(|e| e.message() == "[mcp] 絵を返す")
        .collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].field("ui_us"), Some("1234"));
    assert!(
        records[0]
            .field("encode_us")
            .and_then(|v| v.parse::<u64>().ok())
            .is_some()
    );
}
