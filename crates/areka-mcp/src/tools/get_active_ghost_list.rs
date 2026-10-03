//! `get_active_ghost_list` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "get_active_ghost_list",
    "title": "Get Active Ghost List",
    "description": "Get current active ghost list suitable for ghost_name parameter. Please use this tool before using sakurascript tool.",
    "inputSchema": {
        "type": "object",
        "properties": {}
    }
}"##;

/// 引数は無い（余計な欄は見ない）。
pub(super) fn parse(_args: &Map<String, Value>) -> ToolCall {
    ToolCall::GetActiveGhostList
}
