//! `get_property` の入口（定義の逐語と引数の詰め替え）。

use serde_json::{Map, Value};

use super::ToolCall;

/// 保存した JSON（doc/ssp-mcp/tools-list-ssp-2.9.05.json）のこのツールの 1 個ぶんを逐語で貼る。
pub(super) const DEFINITION: &str = r##"{
    "name": "get_property",
    "title": "Get Property System",
    "description": "Get Ukagaka property system value : for detail please see https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html",
    "inputSchema": {
        "type": "object",
        "properties": {
            "property_name": {
                "type": "string",
                "description": "Property name"
            },
            "ghost_name": {
                "type": "string",
                "description": "Optional: target ghost name or full path of its root folder"
            }
        },
        "required": [
            "property_name"
        ]
    }
}"##;

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub property_name: String,
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::GetProperty(Args {
        property_name: super::required_string(args, "property_name"),
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}
