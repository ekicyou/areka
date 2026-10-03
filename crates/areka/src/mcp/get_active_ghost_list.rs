//! `get_active_ghost_list` の本物の処理（spec: areka-P0-mcp-tool-entrances）。

use areka_mcp::tools::{ReplyTo, outcome};

use super::resolve::{ActiveGhost, listed_value};

/// 起動中のゴーストを一覧に出す値（素の値・1 行・末尾の改行なし）で答える。0 体なら空の本文（要件 4.1〜4.3）。
pub(super) fn handle(active: Option<&ActiveGhost>, reply: ReplyTo) {
    reply.send(outcome::value(active.map(listed_value).unwrap_or_default()));
}

#[cfg(test)]
#[path = "get_active_ghost_list_tests.rs"]
mod get_active_ghost_list_tests;
