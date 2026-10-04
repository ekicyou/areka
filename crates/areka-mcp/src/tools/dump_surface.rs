//! `dump_surface` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "dump_surface",
    "title": "Dump Surface Image",
    "description": "Get an image (PNG with transparency) of a ghost's character. Without surface, captures what is currently shown on the screen: use it to check the result of sakurascript tool. With surface, renders that surface ID alone in its initial state regardless of the screen. One image per call.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "scope": {
                "type": "integer",
                "description": "Optional: character scope (0 = \\0 main character, 1 = \\1 partner, ...). Default 0"
            },
            "surface": {
                "type": "integer",
                "description": "Optional: surface ID to render. If omitted, the surface currently shown on the screen is captured as is"
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
    pub surface: Option<i64>,
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::DumpSurface(Args {
        scope: super::optional_integer(args, "scope"),
        surface: super::optional_integer(args, "surface"),
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}
