//! `reload` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "reload",
    "title": "Reload Ghost",
    "description": "Reload ghost / shiori (dictionaries) / shell / balloon / makoto / descript of a ghost, for example after editing its files. Errors while loading are recorded to the error log; check them with get_log tool.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "target": {
                "type": "string",
                "description": "What to reload: ghost / shiori / shell / balloon / makoto / descript"
            },
            "ghost_name": {
                "type": "string",
                "description": "Optional: target ghost name or full path of its root folder"
            }
        },
        "required": [
            "target"
        ]
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub target: String,
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::Reload(Args {
        target: super::required_string(args, "target"),
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}
