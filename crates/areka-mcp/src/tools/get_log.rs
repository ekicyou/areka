//! `get_log` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "get_log",
    "title": "Get Log",
    "description": "Get SSP log entries, oldest first. One entry per line: \"#id time [type] name : text\" (continuation lines of multi-line text start with a tab). log_type: error (errors/warnings of SSP and ghosts, including sakurascript strict errors), script (played SakuraScripts), network, update, status. Use since_id with the largest id you have seen to get only new entries.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "log_type": {
                "type": "string",
                "description": "Optional: error (default) / script / network / update / status"
            },
            "ghost_name": {
                "type": "string",
                "description": "Optional: only entries of this ghost (ghost name or full path of its root folder)"
            },
            "since_id": {
                "type": "integer",
                "description": "Optional: only entries with id greater than this"
            },
            "max_count": {
                "type": "integer",
                "description": "Optional: maximum number of newest entries to return"
            }
        }
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub log_type: Option<String>,
    pub ghost_name: Option<String>,
    pub since_id: Option<i64>,
    pub max_count: Option<i64>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::GetLog(Args {
        log_type: super::optional_string(args, "log_type"),
        ghost_name: super::optional_string(args, "ghost_name"),
        since_id: super::optional_integer(args, "since_id"),
        max_count: super::optional_integer(args, "max_count"),
    })
}
