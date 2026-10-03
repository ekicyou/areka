//! `dump_balloon` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "dump_balloon",
    "title": "Dump Balloon Image",
    "description": "Get an image (PNG with transparency) of the balloon (speech bubble) of a character as last drawn. While the ghost is still talking the text may be partially drawn; wait until get_status no longer reports talking.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "scope": {
                "type": "integer",
                "description": "Optional: character scope whose balloon is captured (0 = \\0, 1 = \\1, ...). Default 0"
            },
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
    pub scope: Option<i64>,
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::DumpBalloon(Args {
        scope: super::optional_integer(args, "scope"),
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}
