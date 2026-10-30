//! strict の語彙（spec: mcp-strict-errors）。検出点の列挙・種類と文面の定数・判定の差し込み口・
//! 位置の数え方・記録 1 件の書き方を置く。
//!
//! 文面の定数は `check_script`（`areka` の `mcp/check_script_judge.rs`）と共用する正本の置き場で、
//! 文書の正本は `doc/ssp-mcp/areka-tools.md` の「⑶ 診断の種類と文面」。

use std::ops::Range;
use std::sync::Arc;

use dola::cue::CueFinding;

/// 記録の宛先（`areka` の `log_history::TARGET_ERROR` と同じ。`get_log` の `log_type=error` で読める）。
pub const RECORD_TARGET: &str = "areka::log::error";

// 文言（正本は `doc/ssp-mcp/areka-tools.md`。形は「何が起きているか; 再生でどうなるか」の英文 1 文）。
// どれも `:` を含まない（記録の本文で最初の `: ` までが文面になる）。
pub const MSG_UNKNOWN_TAG: &str = "areka does not know this tag; playback drops it";
pub const MSG_UNKNOWN_COMMAND: &str =
    "no part of areka handles this \\! command; playback ignores it";
pub const MSG_MISSING_SURFACE: &str =
    "no such surface in the current shell; playback does not change the surface";
pub const MSG_MISSING_BALLOON: &str =
    "no such balloon ID in the current balloon; playback does not change the balloon";
pub const MSG_UNCLOSED: &str =
    "the bracket or quote is not closed; playback drops everything from here to the end";
pub const MSG_DEFAULTED: &str =
    "the argument is missing or unreadable; playback uses the default value";
pub const MSG_IGNORED: &str = "areka accepts this but it has no effect yet";
pub const MSG_ANCHOR_UNCLOSED: &str =
    "the anchor is not closed; playback extends it to the end of the script";
pub const MSG_ANCHOR_REOPENED: &str =
    "a new anchor opens before the previous one closes; playback closes the previous one here";
pub const MSG_ANCHOR_STRAY_CLOSE: &str = "no anchor is open; playback ignores this close";

/// 再生の側で strict の記録を書く分岐の全部（設計「検出点の表」）。予約の種類
/// （`missing_animation`・`unknown_entity`）の点は持たない（要件 2.8）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetectionPoint {
    ReadUnknownTag,
    ReadUnclosed,
    ReadDefaulted,
    ReadMarkerIgnored,
    CommandUnhandled,
    ChoiceTimeoutUnreadable,
    FontIgnored,
    FontUnknownKey,
    AnchorReopened,
    AnchorUnclosed,
    AnchorStrayClose,
    SurfaceUnresolved,
    SurfaceAbsent,
    BalloonInvalid,
    BalloonAbsent,
    BalloonBoxMissing,
}

impl DetectionPoint {
    /// 表の順の 16 個。
    pub const ALL: [DetectionPoint; 16] = {
        use DetectionPoint::*;
        [
            ReadUnknownTag,
            ReadUnclosed,
            ReadDefaulted,
            ReadMarkerIgnored,
            CommandUnhandled,
            ChoiceTimeoutUnreadable,
            FontIgnored,
            FontUnknownKey,
            AnchorReopened,
            AnchorUnclosed,
            AnchorStrayClose,
            SurfaceUnresolved,
            SurfaceAbsent,
            BalloonInvalid,
            BalloonAbsent,
            BalloonBoxMissing,
        ]
    };

    /// 点の名前（列挙子と同じ綴り。記録の欄 `point` に載る）。
    pub fn name(self) -> &'static str {
        use DetectionPoint::*;
        match self {
            ReadUnknownTag => "ReadUnknownTag",
            ReadUnclosed => "ReadUnclosed",
            ReadDefaulted => "ReadDefaulted",
            ReadMarkerIgnored => "ReadMarkerIgnored",
            CommandUnhandled => "CommandUnhandled",
            ChoiceTimeoutUnreadable => "ChoiceTimeoutUnreadable",
            FontIgnored => "FontIgnored",
            FontUnknownKey => "FontUnknownKey",
            AnchorReopened => "AnchorReopened",
            AnchorUnclosed => "AnchorUnclosed",
            AnchorStrayClose => "AnchorStrayClose",
            SurfaceUnresolved => "SurfaceUnresolved",
            SurfaceAbsent => "SurfaceAbsent",
            BalloonInvalid => "BalloonInvalid",
            BalloonAbsent => "BalloonAbsent",
            BalloonBoxMissing => "BalloonBoxMissing",
        }
    }

    /// 種類の名前（`check_script` の結果の `kind` と同じ綴り）。
    pub fn kind(self) -> &'static str {
        use DetectionPoint::*;
        match self {
            ReadUnknownTag | FontUnknownKey => "unknown_tag",
            ReadUnclosed | ReadDefaulted | ChoiceTimeoutUnreadable => "unreadable_argument",
            ReadMarkerIgnored | FontIgnored => "ignored",
            CommandUnhandled => "unknown_command",
            AnchorReopened | AnchorUnclosed | AnchorStrayClose => "unpaired_tag",
            SurfaceUnresolved | SurfaceAbsent => "missing_surface",
            BalloonInvalid | BalloonAbsent | BalloonBoxMissing => "missing_balloon",
        }
    }

    /// 文面（`check_script` と同じ定数）。
    pub fn message(self) -> &'static str {
        use DetectionPoint::*;
        match self {
            ReadUnknownTag | FontUnknownKey => MSG_UNKNOWN_TAG,
            ReadUnclosed => MSG_UNCLOSED,
            ReadDefaulted | ChoiceTimeoutUnreadable => MSG_DEFAULTED,
            ReadMarkerIgnored | FontIgnored => MSG_IGNORED,
            CommandUnhandled => MSG_UNKNOWN_COMMAND,
            AnchorReopened => MSG_ANCHOR_REOPENED,
            AnchorUnclosed => MSG_ANCHOR_UNCLOSED,
            AnchorStrayClose => MSG_ANCHOR_STRAY_CLOSE,
            SurfaceUnresolved | SurfaceAbsent => MSG_MISSING_SURFACE,
            BalloonInvalid | BalloonAbsent | BalloonBoxMissing => MSG_MISSING_BALLOON,
        }
    }

    /// 合図の受け手が返す知らせへの写し。
    pub fn finding(self) -> CueFinding {
        CueFinding {
            point: self.name(),
            kind: self.kind(),
            message: self.message(),
        }
    }
}

/// 台本の解釈の外にある判定（`areka` が実装して差し込む）。
pub trait StrictJudges: Send + Sync {
    /// `\!` の名前と第 1 引数の組を誰かが拾うか（`ConsumerLedger::consumer_of` が Some）。
    fn command_handled(&self, name: &str, first: Option<&str>) -> bool;
    /// `\f` の引数の分類。`FontIgnored` か `FontUnknownKey`、診ないものは None。
    fn font(&self, args: &[String]) -> Option<DetectionPoint>;
}

/// talk へ渡すゴースト名と判定の束。
#[derive(Clone)]
pub struct StrictHooks {
    /// 記録の名（`get_active_ghost_list` と同じ値）。
    pub ghost: Arc<str>,
    pub judges: Arc<dyn StrictJudges>,
}

/// 文字の数での位置（0 始まり・`end` は含まない）と綴り。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// バイトの範囲を文字の数へ直す。前から 1 度だけ数える（範囲は台本の順に来るのが常）。
pub struct CharCursor<'a> {
    script: &'a str,
    /// ここまで数えたバイトの位置と、そこまでの文字の数。
    byte: usize,
    chars: usize,
}

impl<'a> CharCursor<'a> {
    pub fn new(script: &'a str) -> Self {
        Self {
            script,
            byte: 0,
            chars: 0,
        }
    }

    /// `span` はバイトの範囲。前の呼び出しより前へ戻ったときは先頭から数え直す。
    pub fn place(&mut self, span: Range<usize>) -> Place {
        if span.start < self.byte {
            (self.byte, self.chars) = (0, 0);
        }
        // 同じ命令に 2 件目が来たときは `byte == span.start` のままなので数え直さない。
        self.chars += self.script[self.byte..span.start].chars().count();
        self.byte = span.start;
        let text = &self.script[span];
        Place {
            start: self.chars,
            end: self.chars + text.chars().count(),
            text: text.to_string(),
        }
    }
}

/// strict の記録 1 件を書く。本文は `<kind> <start>-<end> <message>: <text>`、取り決めの欄 `ghost`・`point` 付き。
pub fn record(ghost: &str, finding: CueFinding, place: &Place) {
    tracing::warn!(
        target: RECORD_TARGET,
        ghost = ghost,
        point = finding.point,
        "{} {}-{} {}: {}",
        finding.kind,
        place.start,
        place.end,
        finding.message,
        place.text
    );
}

#[cfg(test)]
#[path = "strict_tests.rs"]
mod tests;
