//! MCP のツールの要求を UI スレッドで汲んで振り分ける口（spec: areka-P0-mcp-tool-entrances）。
//!
//! 受け口（`McpInbox`）・据え付け・系の登録・終了の途中の始末・汲む系・振り分けを置く。
//! 各ツールの処理はツールごとのファイルに持ち、テストの接続もそのファイル自身に書く。

mod resolve;

mod dump_balloon;
mod dump_surface;
mod get_active_ghost_list;
mod get_expression_table;
mod get_log;
mod get_property;
mod get_status;
mod raise_event;
mod reload;
mod sakurascript;

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;
