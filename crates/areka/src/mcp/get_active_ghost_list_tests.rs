//! `get_active_ghost_list` の決定論テスト。
//!
//! 名前あり → その名前・`name` 無し → フルパス（末尾の区切りなし）・0 体 → 空の本文。
//! どれも 1 行・末尾の改行なし・isError: false。

use std::path::PathBuf;

use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;
use crate::mcp::kanade_fake_test_support::{self, body, ghost};

/// `active` で呼んだときの答え（本文, isError）。
fn answer(active: Option<&ActiveGhost>) -> (String, bool) {
    let (req, pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    handle(active, req.reply);
    let outcome = kanade_fake_test_support::answer(&pending);
    (body(&outcome).to_owned(), outcome.is_error)
}

#[test]
fn named_ghost_lists_its_name() {
    let ghost = ghost();
    assert_eq!(answer(Some(&ghost)), ("Emily/Phase4.5".to_string(), false));
}

#[test]
fn unnamed_ghost_lists_full_path_without_trailing_separator() {
    let ghost = ActiveGhost {
        name: None,
        sakura_name: None,
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
