//! `sakurascript` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "sakurascript",
    "title": "Send SakuraScript",
    "description": "Send Ukagaka SakuraScript to a ghost (character). Please use get_active_ghost_list , get_expression_table tools first to obtain detailed information.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "script": {
                "type": "string",
                "description": "SakuraScript to send"
            },
            "ghost_name": {
                "type": "string",
                "description": "Optional: send target ghost name or full path of its root folder"
            },
            "strict": {
                "type": "boolean",
                "description": "Optional: if true, record script errors (unknown surface/balloon/animation/command) to the error log during playback. Read them with get_log tool (log_type=error, since_id=value returned by this tool)."
            }
        },
        "required": [
            "script"
        ]
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub script: String,
    pub ghost_name: Option<String>,
    pub strict: Option<bool>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::Sakurascript(Args {
        script: super::required_string(args, "script"),
        ghost_name: super::optional_string(args, "ghost_name"),
        strict: super::optional_bool(args, "strict"),
    })
}
