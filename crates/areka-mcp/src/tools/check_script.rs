//! `check_script` の入口（定義・型の付いた引数・help の 1 行・診断の型・結果の組み立て）。
//!
//! SSP に無い areka 独自のツール（spec: areka-P0-mcp-author-tools）。台本を再生せずに確かめ、
//! 診断の列を結果の本文にする。判断はアプリ本体の側（`crates/areka/src/mcp/check_script_judge.rs`）が持つ。

use serde_json::{Map, Value, json};

use super::{ToolCall, outcome};
use crate::ToolOutcome;

/// このツールの定義。SSP の 10 本と同じ形で、説明の英文に 5 点
/// （独自のツール・何もさせない・返すもの・タグ 1 つでも確かめられる・`sakurascript` の前に使う）を書く。
pub(super) const DEFINITION: &str = r##"{
    "name": "check_script",
    "title": "Check SakuraScript Without Playing",
    "description": "areka's own tool (SSP does not have it). Checks SakuraScript without playing it: the ghost does not speak, move or change. Returns one diagnostic per problem: unknown tags and \\! commands, tags and \\! commands areka accepts but ignores, unreadable arguments, unpaired \\_a anchor tags, and \\s / \\b IDs missing from the ghost's current shell and balloon. Pass a single tag to check whether areka supports it. Use this before the sakurascript tool.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "script": {
                "type": "string",
                "description": "SakuraScript to check"
            },
            "ghost_name": {
                "type": "string",
                "description": "Optional: target ghost name or full path of its root folder"
            }
        },
        "required": [
            "script"
        ]
    }
}"##;

/// help のページに載せる日本語の 1 行。help は HTML へエスケープせずに埋め込むので、`<`・`>`・`&` を書かない。
pub(super) const SUMMARY_JA: &str = "台本を再生せずに確かめ、areka で効かないタグ・コマンド、今のシェルとバルーンに無い ID を位置つきで返す（ゴーストには何もさせない）";

/// 窓の無いゴーストで surface とバルーンを診なかったときに、1 行目の末尾へ括弧で足す注記。
pub const NO_WINDOW_NOTE: &str =
    "surface and balloon IDs were not checked: this ghost has no window";

/// 型の付いた引数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub script: String,
    pub ghost_name: Option<String>,
}

/// 検査を通った arguments を型の付いた引数へ詰め替える（検査の後なので失敗しない）。
pub(super) fn parse(args: &Map<String, Value>) -> ToolCall {
    ToolCall::CheckScript(Args {
        script: super::required_string(args, "script"),
        ghost_name: super::optional_string(args, "ghost_name"),
    })
}

/// 診断の種類。名前の正本は `doc/ssp-mcp/areka-tools.md`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    UnknownTag,
    UnknownCommand,
    MissingSurface,
    MissingBalloon,
    UnreadableArgument,
    Ignored,
    /// 開きと閉じの対応の崩れ（今はアンカー `\_a` だけ）。
    UnpairedTag,
}

impl Kind {
    /// 結果の `kind` の欄に載せる名前。
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::UnknownTag => "unknown_tag",
            Kind::UnknownCommand => "unknown_command",
            Kind::MissingSurface => "missing_surface",
            Kind::MissingBalloon => "missing_balloon",
            Kind::UnreadableArgument => "unreadable_argument",
            Kind::Ignored => "ignored",
            Kind::UnpairedTag => "unpaired_tag",
        }
    }
}

/// 診断 1 件。位置は台本の先頭から数えた文字の数（0 始まり・`end` は含まない）。影響の段の欄は持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    /// その範囲の綴りそのまま。
    pub text: String,
    pub message: &'static str,
}

/// 診断の列を結果にする。1 行目 `OK:<n> diagnostics`（`unchecked` があれば末尾に ` (<注記>)`）、
/// 続けて診断 1 件につき JSON の 1 行。件数に依らず `isError: false`（検査そのものは成功）。
pub fn render(diagnostics: &[Diagnostic], unchecked: Option<&str>) -> ToolOutcome {
    let mut text = format!("{} diagnostics", diagnostics.len());
    if let Some(note) = unchecked {
        text.push_str(&format!(" ({note})"));
    }
    for d in diagnostics {
        // serde_json は改行・引用符をエスケープするので、綴りに何が在っても 1 件が 1 行に収まる。
        let line = json!({
            "kind": d.kind.as_str(),
            "start": d.start,
            "end": d.end,
            "text": d.text,
            "message": d.message,
        });
        text.push('\n');
        text.push_str(&line.to_string());
    }
    outcome::ok(&text)
}

#[cfg(test)]
#[path = "check_script_tests.rs"]
mod check_script_tests;
