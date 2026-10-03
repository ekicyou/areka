//! SSP 2.9.05 と同じ 10 本のツールの入口（spec: areka-P0-mcp-tool-entrances）。
//!
//! 10 本の表（SSP の並び）・検査を通った呼び出し・登録表を組む受け口を置く。
//! 各ツールの定義と引数の詰め替えはツールごとのファイルに持つ。

pub mod bridge;
pub mod outcome;

pub mod dump_balloon;
pub mod dump_surface;
pub mod get_active_ghost_list;
pub mod get_expression_table;
pub mod get_log;
pub mod get_property;
pub mod get_status;
pub mod raise_event;
pub mod reload;
pub mod sakurascript;

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;

#[cfg(test)]
#[path = "tools_socket_tests.rs"]
mod tools_socket_tests;
