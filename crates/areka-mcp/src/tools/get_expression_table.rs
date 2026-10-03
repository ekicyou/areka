//! `get_expression_table` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "get_expression_table",
    "title": "Get Surface Table",
    "description": "Get list of ghost expressions in markdown tabular form. Suitable for use with SakuraScript \\s[] tag. Please use this tool before using sakurascript tool.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "ghost_name": {
                "type": "string",
                "description": "Target ghost name or full path of its root folder"
            }
        },
        "required": [
            "ghost_name"
        ]
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::GetExpressionTable(Args {
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}
