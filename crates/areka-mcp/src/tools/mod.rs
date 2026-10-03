//! SSP 2.9.05 と同じ 10 本のツールの入口（spec: areka-P0-mcp-tool-entrances）。
//!
//! 10 本の表（SSP の並び）・検査を通った呼び出し・登録表を組む受け口を置く。
//! 各ツールの定義と引数の詰め替えはツールごとのファイルに持つ。

pub mod bridge;
pub mod outcome;

pub mod dump_balloon;
pub mod dump_surface;
pub mod get_active_ghost_list;
pub mod get_expression_table;
pub mod get_log;
pub mod get_property;
pub mod get_status;
pub mod raise_event;
pub mod reload;
pub mod sakurascript;

use std::time::Duration;

use serde_json::{Map, Value};

use crate::ToolSpec;
use crate::check::as_integer;

/// 返事を待つ上限（本番の値）。
pub const REPLY_WAIT: Duration = Duration::from_secs(10);

/// 検査を通った呼び出し 1 件（型の付いた引数つき）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    GetActiveGhostList,
    GetStatus(get_status::Args),
    GetExpressionTable(get_expression_table::Args),
    GetProperty(get_property::Args),
    GetLog(get_log::Args),
    Sakurascript(sakurascript::Args),
    RaiseEvent(raise_event::Args),
    Reload(reload::Args),
    DumpSurface(dump_surface::Args),
    DumpBalloon(dump_balloon::Args),
}

impl ToolCall {
    /// ツール名（記録に載せる）。
    pub fn name(&self) -> &'static str {
        match self {
            ToolCall::GetActiveGhostList => "get_active_ghost_list",
            ToolCall::GetStatus(_) => "get_status",
            ToolCall::GetExpressionTable(_) => "get_expression_table",
            ToolCall::GetProperty(_) => "get_property",
            ToolCall::GetLog(_) => "get_log",
            ToolCall::Sakurascript(_) => "sakurascript",
            ToolCall::RaiseEvent(_) => "raise_event",
            ToolCall::Reload(_) => "reload",
            ToolCall::DumpSurface(_) => "dump_surface",
            ToolCall::DumpBalloon(_) => "dump_balloon",
        }
    }
}

/// 検査を通った arguments を `ToolCall` へ詰め替える関数。
pub(crate) type Parse = fn(&Map<String, Value>) -> ToolCall;

/// 10 本の表（定義の文字列, 詰め替え）。並びの正本＝SSP 2.9.05 の `tools/list` の並び。
pub(crate) const TABLE: [(&str, Parse); 10] = [
    (
        get_active_ghost_list::DEFINITION,
        get_active_ghost_list::parse,
    ),
    (get_status::DEFINITION, get_status::parse),
    (
        get_expression_table::DEFINITION,
        get_expression_table::parse,
    ),
    (get_property::DEFINITION, get_property::parse),
    (get_log::DEFINITION, get_log::parse),
    (sakurascript::DEFINITION, sakurascript::parse),
    (raise_event::DEFINITION, raise_event::parse),
    (reload::DEFINITION, reload::parse),
    (dump_surface::DEFINITION, dump_surface::parse),
    (dump_balloon::DEFINITION, dump_balloon::parse),
];

/// 定義の文字列（MCP のツール定義 1 個ぶん）を `ToolSpec` にする。`name` と `inputSchema`（object）は必須。
pub(crate) fn spec_from_definition(definition: &str) -> Result<ToolSpec, String> {
    let value: Value = serde_json::from_str(definition).map_err(|e| e.to_string())?;
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(ToolSpec {
        name: text("name").ok_or("definition has no name")?,
        title: text("title"),
        description: text("description"),
        input_schema: value
            .get("inputSchema")
            .and_then(Value::as_object)
            .cloned()
            .ok_or("definition has no inputSchema object")?,
    })
}

// 以下は各ツールの `parse` が使う読み方。`parse` は検査（`check_arguments`）を通った後にしか呼ばれないので
// 型は合っている。必須の欄も検査が在ることを保証するが、念のため無ければ空の文字列にして panic しない。

fn required_string(args: &Map<String, Value>, key: &str) -> String {
    optional_string(args, key).unwrap_or_default()
}

/// 省略と `null` は `None`。空の文字列はそのまま `Some("")`。
fn optional_string(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn optional_integer(args: &Map<String, Value>, key: &str) -> Option<i64> {
    args.get(key).and_then(as_integer)
}

fn optional_bool(args: &Map<String, Value>, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

/// 省略と `null` は空の列。
fn string_list(args: &Map<String, Value>, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;

#[cfg(test)]
#[path = "tools_socket_tests.rs"]
mod tools_socket_tests;
