//! `get_active_ghost_list` の決定論テスト。
//!
//! 名前あり → その名前・`name` 無し → フルパス（末尾の区切りなし）・0 体 → 空の本文。
//! どれも 1 行・末尾の改行なし・isError: false（要件 4.1・4.2・4.3・4.5）。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

/// `active` で呼んだときの答え（本文, isError）。
fn answer(active: Option<&ActiveGhost>) -> (String, bool) {
    let (req, pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    handle(active, req.reply);
    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    match answer.outcome.content.as_slice() {
        [ToolContent::Text(text)] => (text.clone(), answer.outcome.is_error),
        other => panic!("本文 1 つのはず: {other:?}"),
    }
}

#[test]
fn named_ghost_lists_its_name() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    assert_eq!(answer(Some(&ghost)), ("Emily/Phase4.5".to_string(), false));
}

#[test]
fn unnamed_ghost_lists_full_path_without_trailing_separator() {
    let ghost = ActiveGhost {
        name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4\"),
    };
    assert_eq!(
        answer(Some(&ghost)),
        (r"C:\ssp\ghost\emily4".to_string(), false)
    );
}

#[test]
fn no_active_ghost_is_an_empty_body() {
    assert_eq!(answer(None), (String::new(), false));
}
