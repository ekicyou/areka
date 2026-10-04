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
//! - [`on_done`]: 翻訳の結果の入力（`Input::TranslateDone`）の腕。預けた一括を返すか故障へ倒す。

use super::events::SourceEvent;
use super::{Action, Input, Phase, State};
use crate::msg::{EventId, ShioriCall, ShioriFailure, ShioriFault, ShioriOutcome};
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

/// [`before`] が入力から取り出した、出口の規則（[`after`]）の材料。
pub(super) struct Replied {
    /// 入力が SHIORI の応答で結果が台詞（`Value`）のときの、その台詞（それ以外は `None`）。
    script: Option<String>,
    /// 応答を待っていた GET の元のイベント（控えから取り出したもの）。
    source: Option<SourceEvent>,
}

/// [`step`](super::step) が `route` の前に呼ぶ。入力が SHIORI の応答なら、応答を待っていた GET の
/// 元のイベントの控えを取り出し（控えは 1 回だけ使う）、台詞の応答ならその台詞と一緒に返す。
/// 他の入力では控えに触れない。
pub(super) fn before(state: &mut State, input: &Input) -> Replied {
    match input {
        Input::ShioriReply { outcome, .. } => Replied {
            script: match outcome {
                ShioriOutcome::Value(script) => Some(script.clone()),
                _ => None,
            },
            source: state.reply_source.take(),
        },
        _ => Replied {
            script: None,
            source: None,
        },
    }
}

/// [`step`](super::step) が `route` の後に呼ぶ。
///
/// まず出口の規則（[`capture`]）で SHIORI の台詞の再生開始を捕まえ、一括を帳簿へ預けて
/// `[Action::Translate]` に替える。その後、返す一括の最後の往復が GET ならその ID と Reference の
/// 写しを控え、NOTIFY・降ろす往復・翻訳の依頼なら控えを空にする。往復の無い一括では変えない。
///
/// 殻は一括の最後の往復の結果だけを入れ直すので、次の応答の入力はここで控えた往復のものになる。
pub(super) fn after(state: &mut State, replied: Replied, actions: Vec<Action>) -> Vec<Action> {
    let actions = capture(state, replied, actions);
    let last_round_trip = actions.iter().rev().find_map(|action| match action {
        Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => Some(Some(SourceEvent {
            id: id.clone(),
            references: references.clone(),
        })),
        Action::ShioriRequest(ShioriCall::Notify { .. })
        | Action::ShioriUnload
        | Action::Translate(_) => Some(None),
        _ => None,
    });
    if let Some(source) = last_round_trip {
        state.reply_source = source;
    }
    actions
}

/// 出口の規則。次の 4 つをすべて満たす一括を、順序を保って帳簿へ預け、翻訳の行動 1 つに替える。
///
/// ⑴ 入力が SHIORI の応答で結果が台詞 ⑵ その台詞が 1 文字以上 ⑶ 一括に再生の開始がある
/// ⑷ 元のイベントが `OnTranslate` でない（外から頼まれた `OnTranslate` の応答を再び翻訳しない）。
///
/// ⑴⑶ を満たして ⑵ か ⑷ を満たさないとき、または元のイベントが分からないとき（構造上は
/// 起きない）は、記録を 1 件残して今日どおり再生する。相・採番・期限・控えは腕が決めたまま
/// 触らない。Status は捕まえた時点（腕が相を決めた後＝再生を始める時点）の状態から導く。
fn capture(state: &mut State, replied: Replied, actions: Vec<Action>) -> Vec<Action> {
    let Some(script) = replied.script else {
        return actions;
    };
    let Some(talk_id) = actions.iter().find_map(|action| match action {
        Action::StartTalk(start) => Some(start.talk_id),
        _ => None,
    }) else {
        return actions;
    };
    if script.is_empty() {
        tracing::trace!(target: "kanade", event = "translate_skipped_empty", talk_id = talk_id.0, "0 文字の台詞——翻訳せずに再生する");
        return actions;
    }
    let Some(source) = replied.source else {
        tracing::error!(target: "kanade", event = "translate_source_missing", talk_id = talk_id.0, "台詞を返した応答の元のイベントが分からない——翻訳せずに再生する");
        return actions;
    };
    if source.id.as_str() == "OnTranslate" {
        tracing::debug!(target: "kanade", event = "translate_skipped_self", talk_id = talk_id.0, "OnTranslate の応答の台詞——再び翻訳せずに再生する");
        return actions;
    }
    tracing::debug!(target: "kanade", event = "translate_begin", talk_id = talk_id.0, source = %source.id, "台詞の再生開始を預けて OnTranslate へ");
    let status = ExecutionStatus::derive(&state.snapshot());
    state.translate = Some(TranslateWait {
        talk_id,
        source: source.id.clone(),
        deferred: actions,
    });
    vec![Action::Translate(TranslateRequest {
        script,
        source,
        status,
    })]
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

/// `route` の `Input::TranslateDone` の腕。
///
/// - `Ok(script)`: 預けた一括の再生の開始の台詞だけを `script` に差し替えて返す（起動の記録の
///   後ろ書き＝`epilogue` は触らない）。控え（再生中の台詞・切替の台詞）も書き換える
///   （[`rewrite_notes`]）。
/// - `Err`（輸送路の失敗）: `translate_failed` を残して預けた一括を捨て、既存の故障の遷移へ。
///   切替の送り出しの台詞なら、表示しなかった台詞を次のゴーストへ渡さないよう切替の台詞を空にする。
/// - 帳簿が無い: `translate_done_unexpected` を残して捨てる（構造上は起きない）。
pub(super) fn on_done(mut state: State, result: TranslateResult) -> (State, Vec<Action>) {
    let Some(TranslateWait {
        talk_id,
        source,
        mut deferred,
    }) = state.translate.take()
    else {
        tracing::warn!(target: "kanade", event = "translate_done_unexpected", ok = result.is_ok(), "帳簿が無いのに翻訳の結果が届いた——捨てる");
        return (state, Vec::new());
    };
    let script = match result {
        Ok(script) => script,
        Err(failure) => {
            tracing::error!(target: "kanade", event = "translate_failed", source = %source, talk_id = talk_id.0, error = %failure, "OnTranslate の輸送路が失敗——預けた一括を捨てて終了系列（Fault）へ");
            if matches!(state.phase, Phase::ChangeTalkWait { talk_id: t, .. } if t == talk_id)
                && let Some(change) = state.change.as_mut()
            {
                change.script = None;
            }
            return super::to_unloading_fault(state, ShioriFault::from_failure(&failure));
        }
    };
    let mut changed = false;
    for action in &mut deferred {
        if let Action::StartTalk(start) = action
            && start.talk_id == talk_id
        {
            changed = start.script != script;
            start.script.clone_from(&script);
        }
    }
    rewrite_notes(&mut state, talk_id, script);
    tracing::debug!(target: "kanade", event = "translate_resume", talk_id = talk_id.0, changed, "預けた一括を最終の台詞で返す");
    (state, deferred)
}

/// 控えを `talk_id` の一致で最終の台詞に書き換える（相は腕が決めたまま）。
///
/// - `Steady{talk: Some}`・`BootVersion{talk: Some}` → 再生中の台詞（`OnChoiceTimeout` の Reference0 の源）。
/// - `ChangeTalkWait` → 切替の台詞（停止通知の切替の中身の源）。この相は `OnGhostChanging` の台詞の
///   再生だけを表すので、切替の `OnClose` の台詞（`ChangeCloseTalkWait`）では書き換えない。
fn rewrite_notes(state: &mut State, talk_id: TalkId, script: String) {
    match &mut state.phase {
        Phase::Steady { talk: Some(active) } | Phase::BootVersion { talk: Some(active) }
            if active.talk_id == talk_id =>
        {
            active.script = script;
        }
        Phase::ChangeTalkWait { talk_id: t, .. } if *t == talk_id => {
            if let Some(change) = state.change.as_mut() {
                change.script = Some(script);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "translate_tests.rs"]
mod tests;
