//! `raise_event` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "raise_event",
    "title": "Raise SHIORI Event",
    "description": "Send a SHIORI event to a ghost (same as SSTP NOTIFY with Event header) and play the script it returns. The returned script is included in the result. Useful to try random talks (OnAiTalk) or reactions (OnMouseDoubleClick etc.).",
    "inputSchema": {
        "type": "object",
        "properties": {
            "event": {
                "type": "string",
                "description": "SHIORI event ID (e.g. OnAiTalk, OnMouseDoubleClick)"
            },
            "references": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "description": "Optional: Reference0, Reference1, ... of the event"
            },
            "ghost_name": {
                "type": "string",
                "description": "Optional: target ghost name or full path of its root folder"
            },
            "strict": {
                "type": "boolean",
                "description": "Optional: if true, record script errors of the returned script to the error log during playback (same as sakurascript tool)."
            }
        },
        "required": [
            "event"
        ]
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub event: String,
    pub references: Vec<String>,
    pub ghost_name: Option<String>,
    pub strict: Option<bool>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::RaiseEvent(Args {
        event: super::required_string(args, "event"),
        references: super::string_list(args, "references"),
        ghost_name: super::optional_string(args, "ghost_name"),
        strict: super::optional_bool(args, "strict"),
    })
}
