//! `get_status` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "get_status",
    "title": "Get Ghost Status",
    "description": "Get current status of a ghost as comma separated flags (same as SSTP EXECUTE GetStatus). Empty means idle. Flags: talking, choosing, minimizing, changing, induction, passive, timecritical, nouserbreak, online, opening(...), balloon(...). Sending SakuraScript while talking / choosing / timecritical / opening may interrupt the ghost or be rejected.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "ghost_name": {
                "type": "string",
                "description": "Optional: target ghost name or full path of its root folder"
            }
        }
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::GetStatus(Args {
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}
