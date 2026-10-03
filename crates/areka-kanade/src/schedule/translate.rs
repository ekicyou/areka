//! 運行表の翻訳（`OnTranslate`）——帳簿の型・元のイベントの控え・応答の読み（純粋）。
//!
//! SHIORI が返した台詞は、再生の前に `OnTranslate` で 1 回だけ翻訳にかける。そのための
//! 運行表の側の部品をここに置く。外から渡す口の型（展開と MAKOTO の口）は別のモジュール
//! [`crate::translate`] で、`crate::change` と `schedule::change` と同じ分け方をとる。
//!
//! - [`TranslateWait`]・[`TranslateRequest`]・[`TranslateResult`]: 翻訳の待ちの帳簿・殻への依頼・
//!   殻が戻す結果。
//! - [`before`]・[`after`]: [`step`](super::step) の入口と出口。応答を待っている GET の元の
//!   イベント（[`State::reply_source`](super::State)）を控え、応答の入力で 1 回だけ取り出す。
//! - [`read_reply`]: `OnTranslate` の生の結果を読む（殻が呼ぶ）。

use super::events::SourceEvent;
use super::{Action, Input, State};
use crate::msg::{EventId, ShioriCall, ShioriFailure, ShioriOutcome};
use crate::status::ExecutionStatus;
use crate::talk::TalkId;

/// 翻訳の待ちの帳簿（[`State::translate`](super::State)）。高々 1 つ。
pub(crate) struct TranslateWait {
    /// 預けた一括の再生の開始の `talk_id`。
    pub talk_id: TalkId,
    /// 元のイベントの ID（記録用）。
    pub source: EventId,
    /// 腕が返した一括をそのまま（順序を保つ）。
    pub deferred: Vec<Action>,
}

/// 殻へ渡す翻訳の依頼（`Action::Translate` の中身）。
pub(crate) struct TranslateRequest {
    /// SHIORI が返したままの台詞（展開の前）。
    pub script: String,
    /// 元のイベント（`OnTranslate` の Reference2・3 の源）。
    pub source: SourceEvent,
    /// 捕まえた時点の状態から導いた Status。
    pub status: ExecutionStatus,
}

/// 殻が運行表へ戻す翻訳の結果（`Input::TranslateDone` の中身）。`Err` は輸送路の失敗だけ。
pub(crate) type TranslateResult = Result<String, ShioriFailure>;

/// `OnTranslate` の応答の読み（[`read_reply`] の返り値）。
#[derive(Debug)]
pub(crate) enum ReplyReading {
    /// MAKOTO の口へ渡す台詞。
    Proceed(String),
    /// 輸送路の失敗（記録は運行表が結果を受けたときに残す）。
    Failed(ShioriFailure),
}

/// [`step`](super::step) が `route` の前に呼ぶ。入力が SHIORI の応答なら、応答を待っていた GET の
/// 元のイベントの控えを取り出して返す（控えは 1 回だけ使う）。他の入力では控えに触れない。
pub(super) fn before(state: &mut State, input: &Input) -> Option<SourceEvent> {
    match input {
        Input::ShioriReply { .. } => state.reply_source.take(),
        _ => None,
    }
}

/// [`step`](super::step) が `route` の後に呼ぶ。返す一括の最後の往復が GET ならその ID と
/// Reference の写しを控え、NOTIFY・降ろす往復なら控えを空にする。往復の無い一括では変えない。
///
/// 殻は一括の最後の往復の結果だけを入れ直すので、次の応答の入力はここで控えた往復のものになる。
pub(super) fn after(state: &mut State, actions: Vec<Action>) -> Vec<Action> {
    let last_round_trip = actions.iter().rev().find_map(|action| match action {
        Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => Some(Some(SourceEvent {
            id: id.clone(),
            references: references.clone(),
        })),
        Action::ShioriRequest(ShioriCall::Notify { .. }) | Action::ShioriUnload => Some(None),
        _ => None,
    });
    if let Some(source) = last_round_trip {
        state.reply_source = source;
    }
    actions
}

/// `OnTranslate` の生の結果を読む（殻が呼ぶ純粋な関数）。
///
/// `expanded` は展開済みの元の台詞（204・エラー応答・GET では起きない結果で使う）。輸送路の
/// 失敗以外は、元のイベントの ID（`source`）つきの `translate_reply` を 1 件残す。輸送路の
/// 失敗の記録は、運行表が結果を受けたときの `translate_failed` が受け持つ（重ねて出さない）。
pub(crate) fn read_reply(
    source: &EventId,
    expanded: String,
    outcome: ShioriOutcome,
) -> ReplyReading {
    match outcome {
        ShioriOutcome::Value(script) => {
            let kind = if script.is_empty() {
                "empty"
            } else {
                "replaced"
            };
            tracing::info!(target: "kanade", event = "translate_reply", source = %source, kind, "OnTranslate の台詞を採る");
            ReplyReading::Proceed(script)
        }
        ShioriOutcome::NoContent => {
            tracing::info!(target: "kanade", event = "translate_reply", source = %source, kind = "no_content", "OnTranslate は 204——展開済みの元の台詞で進む");
            ReplyReading::Proceed(expanded)
        }
        ShioriOutcome::Failed(ShioriFailure::Shiori(error)) => {
            tracing::warn!(target: "kanade", event = "translate_reply", source = %source, kind = "error_response", error = %error, "OnTranslate がエラー応答——展開済みの元の台詞で進む");
            ReplyReading::Proceed(expanded)
        }
        ShioriOutcome::Failed(failure) => ReplyReading::Failed(failure),
        ShioriOutcome::Notified | ShioriOutcome::Unloaded => {
            tracing::warn!(target: "kanade", event = "translate_reply", source = %source, kind = "unexpected", "GET では起きない結果——204 と同じに展開済みの元の台詞で進む");
            ReplyReading::Proceed(expanded)
        }
    }
}

#[cfg(test)]
#[path = "translate_tests.rs"]
mod tests;
