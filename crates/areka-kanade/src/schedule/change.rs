//! 切替の相（`src/schedule/change.rs`）。
//!
//! ゴーストの切替の要求を受理してから止まるまでの判断を置く。送り出しの握手は正典の順序で
//! 行う（`OnGhostChanging` → 204 なら `OnClose`（Ref0＝`system`）→ 台詞の完了で降ろす）。相は 4 つ:
//! - [`Phase::ChangePending`]（`OnGhostChanging` の応答待ち）
//! - [`Phase::ChangeTalkWait`]（その台詞の完了待ち／期限判定）
//! - [`Phase::ChangeClosePending`]（切替の `OnClose` の応答待ち）
//! - [`Phase::ChangeCloseTalkWait`]（その別れの台詞の完了待ち／期限判定）
//!
//! 降ろすときの停止原因は今日の値（`Quit`・`CloseSilent`・`DeadlineExceeded`・`Fault`）のまま使い、
//! 「切替で止まった」ことは [`State::change`] が停止通知の切替の中身として表す。終了の握手
//! （`close.rs`）とは期限の計算（[`deadline_from`]）だけを共有する。
//!
//! 送り出しの台詞を利用者が中断したら切替を中止して定常へ戻る（[`cancel_by_user_break`]）。
//! 定常へ戻る経路はここにだけ置き、終了の握手には足さない。終了要求が届いたら切替を取りやめ、
//! 相ごとに今日の終了の握手へ合流する（[`yield_to_close`]）。
//!
//! 汎用の通知の入口（外から頼まれた許可表のイベントを定常でだけ送る）の判断
//! [`on_raise_event`] もここに置く。

use super::close::deadline_from;
use super::{
    Action, ActiveTalk, Input, Phase, State, TermCause, clear_choice_ledger, events, phase_label,
    steady, to_unloading_quit,
};
use crate::change::{CancelReason, ChangeRequest, KanadeNotice, ShioriMethod};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriOutcome};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkDone, TalkEndReason, TalkId};

/// 受理した切替の帳簿（[`super::State::change`]）。
///
/// 受理で立ち、中止・取りやめでだけ消え、`Unloading` まで残る。止まった時点の値が
/// 停止通知の切替の中身（`KanadeStopped.handoff`）の源になる。
pub(crate) struct ChangeState {
    /// 受理した切替の要求。
    pub req: ChangeRequest,
    /// `OnGhostChanging` が返した台本（送らなかった・204・失敗は `None`）。
    pub script: Option<String>,
}

/// 横断の腕 `Input::ChangeGhost` の受理。
///
/// 終了の保留が無い `Steady{talk: None}` が受理する。終了の保留が無い `Steady{talk: Some}` は
/// 要求を [`State::pending_change`] に控え、そのトークの完了で [`consume_pending`] が消化する
/// （保留の間 [`State::change`] は空のまま）。それ以外（終了の保留あり・保留済み・起動系列・
/// 終了系列・切替の相）は `warn!` の上で「切替の中止（受理しなかった）」を通知し、状態を変えない
/// （受理しなかった要求は必ず UI へ返し、UI の目印を残さない）。
pub(super) fn on_change_ghost(mut state: State, req: ChangeRequest) -> (State, Vec<Action>) {
    let rejected = match (&state.phase, &state.pending_close) {
        (_, Some(_)) => Some("pending_close"),
        (Phase::Steady { talk: None }, None) => None,
        (Phase::Steady { talk: Some(_) }, None) if state.pending_change.is_some() => {
            Some("change_pending")
        }
        (Phase::Steady { talk: Some(_) }, None) => {
            tracing::info!(target: "kanade", event = "change_pending", to = %req.target.name, raise_event = req.raise_event, "再生中に切替の要求を受けた——トークの完了まで保留する");
            state.pending_change = Some(req);
            return (state, Vec::new());
        }
        _ => Some("not_steady"),
    };
    if let Some(reason) = rejected {
        tracing::warn!(
            target: "kanade",
            event = "change_rejected_phase",
            reason,
            phase = phase_label(&state.phase),
            to = %req.target.name,
            "切替の要求を受けられる状態ではない——受理せず切替の中止を通知する"
        );
        let notice = KanadeNotice::ChangeCancelled {
            reason: CancelReason::Rejected,
        };
        return (state, vec![Action::Notice(notice)]);
    }
    begin_change(state, req)
}

/// 横断の腕 `Input::RaiseEvent`（汎用の通知の入口）の判断。
///
/// 許可表に無いイベントと定常以外での依頼は `warn!` の上で捨てる（待ち行列に積まない）。
/// 定常なら渡された Reference 列のまま GET／NOTIFY を送る。応答は定常の応答の腕へ流れ、
/// 再生中なら [`events::value_replaces_active_talk`] に従って今のトークを置き換える。
pub(super) fn on_raise_event(
    state: State,
    id: String,
    references: Vec<String>,
    method: ShioriMethod,
) -> (State, Vec<Action>) {
    let Some(id) = events::allowed_static(&id) else {
        tracing::warn!(target: "kanade", event = "raise_event_not_allowed", id = %id, "許可表に無いイベントの依頼——送らずに捨てる");
        return (state, Vec::new());
    };
    if !matches!(state.phase, Phase::Steady { .. }) {
        tracing::warn!(target: "kanade", event = "raise_event_not_steady", id, phase = phase_label(&state.phase), "定常以外でのイベントの依頼——送らずに捨てる（積まない）");
        return (state, Vec::new());
    }
    let call = events::raise(id, references, method, &state.snapshot());
    (state, vec![Action::ShioriRequest(call)])
}

/// 横断の腕（`on_talk_done`）の問い: 定常の再生中のトークに保留の切替が掛かっているか。
pub(super) fn has_pending(state: &State) -> bool {
    matches!(state.phase, Phase::Steady { talk: Some(_) }) && state.pending_change.is_some()
}

/// 保留の切替を持つトークが終わった（最後まで流れた・`\-` の予約なしで中断された）ときの消化。
///
/// 終了の保留が在れば終了が勝つ: 保留の切替を捨てて「切替の中止（終了要求）」を通知し、
/// 残り（保留の終了の握手）は定常の完了の処理へ任せる。無ければ保留を取り出して切替を始める
/// （`raise_event` 無しなら黙って降ろす）。
pub(super) fn consume_pending(
    mut state: State,
    done: TalkDone,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    let Some(req) = state.pending_change.take() else {
        return steady::step(state, Input::TalkDone(done), config);
    };
    if state.pending_close.is_some() {
        tracing::info!(target: "kanade", event = "change_yield_to_close", phase = phase_label(&state.phase), to = %req.target.name, "保留の切替より終了の保留が勝つ——切替を取りやめる");
        let (state, mut actions) = steady::step(state, Input::TalkDone(done), config);
        let notice = KanadeNotice::ChangeCancelled {
            reason: CancelReason::CloseRequest,
        };
        actions.insert(0, Action::Notice(notice));
        return (state, actions);
    }
    tracing::info!(target: "kanade", event = "change_pending_consumed", talk_id = done.talk_id.0, reason = ?done.reason, "保留の切替を持つトークが終わった——切替を始める");
    begin_change(state, req)
}

/// 終了系列へ進むトークに掛かっていた保留の切替を捨てる（`\-` の予約・到達が勝つ）。
pub(super) fn drop_pending_by_quit(state: &mut State) {
    if let Some(req) = state.pending_change.take() {
        tracing::info!(target: "kanade", event = "change_dropped_by_quit", to = %req.target.name, "終了で終わるトークに保留の切替が掛かっていた——切替を捨てて終了系列へ");
    }
}

/// 切替を始める。帳簿を立て、`raise_event` なら `OnGhostChanging` を GET で送って応答を待つ。
/// `raise_event` でなければ `OnClose` も送らず黙って降ろす。
fn begin_change(mut state: State, req: ChangeRequest) -> (State, Vec<Action>) {
    tracing::info!(
        target: "kanade",
        event = "change_accepted",
        to = %req.target.name,
        origin = req.origin.as_ref_str(),
        raise_event = req.raise_event,
        "切替の要求を受理した"
    );
    clear_choice_ledger(&mut state, "change_accepted");
    if !req.raise_event {
        state.change = Some(ChangeState { req, script: None });
        state.phase = Phase::Unloading {
            cause: TermCause::CloseSilent,
        };
        return (state, vec![Action::ShioriUnload]);
    }
    let call = events::on_ghost_changing(&req, &ExecutionSnapshot::INACTIVE);
    state.change = Some(ChangeState { req, script: None });
    state.phase = Phase::ChangePending;
    (state, vec![Action::ShioriRequest(call)])
}

/// 切替の 4 相のいずれかか（横断の腕 `on_talk_done` が `\-` の予約を終了へ結ばない判定に使う）。
pub(super) fn is_change_phase(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::ChangePending
            | Phase::ChangeTalkWait { .. }
            | Phase::ChangeClosePending
            | Phase::ChangeCloseTalkWait { .. }
    )
}

/// 切替の相の振り分け（`dispatch_phase` から）。終了要求は相を問わず [`yield_to_close`] へ。
pub(super) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    if let Input::CloseRequest { reason } = input {
        return yield_to_close(state, reason);
    }
    match state.phase {
        Phase::ChangePending | Phase::ChangeClosePending => on_reply_wait(state, input, config),
        Phase::ChangeTalkWait { .. } | Phase::ChangeCloseTalkWait { .. } => {
            on_talk_wait(state, input, config)
        }
        _ => {
            tracing::warn!(target: "kanade", event = "change_phase_unexpected", phase = phase_label(&state.phase), "切替の相以外への切替の入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// `ChangePending`／`ChangeClosePending`（GET の応答待ち）。
///
/// - 台本 → 採番して再生し、台詞の完了待ちへ（`OnGhostChanging` の台本は帳簿へ控える）。
/// - 204 → `OnGhostChanging` なら続けて `OnClose`（Ref0＝`system`）を GET、`OnClose` なら黙って降ろす。
/// - `Tick` → 時刻だけ更新する。
fn on_reply_wait(mut state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    let closing = matches!(state.phase, Phase::ChangeClosePending);
    if !closing && let Some(reason) = state.pending_close {
        return on_yielded_reply(state, input, reason, config);
    }
    match input {
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(script),
            ..
        } => {
            let talk_id = TalkId(state.next_talk_id);
            state.next_talk_id += 1;
            let deadline = deadline_from(state.last_now, config);
            if !closing && let Some(change) = state.change.as_mut() {
                change.script = Some(script.clone());
            }
            state.phase = talk_wait(closing, talk_id, deadline);
            tracing::info!(target: "kanade", event = "change_talk_start", talk_id = talk_id.0, phase = phase_label(&state.phase), "送り出しの台詞を再生し完了を待つ");
            (
                state,
                vec![Action::StartTalk(StartTalk::new(talk_id, script))],
            )
        }
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            ..
        } if closing => {
            tracing::info!(target: "kanade", event = "change_close_silent", "切替の OnClose も応答なし（204）——台詞なしで降ろす");
            state.phase = Phase::Unloading {
                cause: TermCause::CloseSilent,
            };
            (state, vec![Action::ShioriUnload])
        }
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            ..
        } => {
            tracing::info!(target: "kanade", event = "change_close_begin", "OnGhostChanging は応答なし（204）——続けて OnClose を送る");
            state.phase = Phase::ChangeClosePending;
            let call = events::on_close(CloseReason::System, &ExecutionSnapshot::INACTIVE);
            (state, vec![Action::ShioriRequest(call)])
        }
        Input::Tick { now } => {
            state.last_now = Some(now);
            (state, Vec::new())
        }
        _ => {
            tracing::warn!(target: "kanade", event = "change_input_ignored", phase = phase_label(&state.phase), "切替の応答待ちに無関係な入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// `ChangeTalkWait`／`ChangeCloseTalkWait`（送り出しの台詞の完了待ち／期限判定）。
///
/// - `TalkDone{Ended}` → 降ろす（`Unloading{Quit}`）。`\-` の完了は横断の腕が同じ終端へ送る。
/// - `TalkDone{Interrupted}` → 利用者の中断＝切替の中止（`\-` の予約を見ない）。
/// - `Tick` → 期限の判定（`close.rs` の別れの台詞と同じ上限・入口で時刻が無ければ最初の Tick で決める）。
fn on_talk_wait(mut state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    let (talk_id, deadline, closing) = match state.phase {
        Phase::ChangeTalkWait { talk_id, deadline } => (talk_id, deadline, false),
        Phase::ChangeCloseTalkWait { talk_id, deadline } => (talk_id, deadline, true),
        _ => {
            tracing::warn!(target: "kanade", event = "change_phase_unexpected", phase = phase_label(&state.phase), "台詞の待ち以外への切替の入力——現 Phase 維持で継続");
            return (state, Vec::new());
        }
    };
    match input {
        Input::TalkDone(done) if done.reason == TalkEndReason::Interrupted => {
            cancel_by_user_break(state, talk_id)
        }
        Input::TalkDone(done) => {
            tracing::info!(target: "kanade", event = "change_talk_done_unload", talk_id = talk_id.0, reason = ?done.reason, "送り出しの台詞が終わった——ゴーストを降ろす");
            to_unloading_quit(state, "change_talk_done_unload")
        }
        Input::Tick { now } => {
            state.last_now = Some(now);
            match deadline {
                None => {
                    let d = MonotonicMs(now.0.saturating_add(config.close_talk_deadline_ms));
                    tracing::info!(target: "kanade", event = "change_deadline_armed", talk_id = talk_id.0, deadline_ms = d.0, "送り出しの台詞の最初の Tick——再生完了待ちの上限を設定");
                    state.phase = talk_wait(closing, talk_id, Some(d));
                    (state, Vec::new())
                }
                Some(d) if now >= d => {
                    tracing::error!(target: "kanade", event = "change_deadline_exceeded", talk_id = talk_id.0, overshoot_ms = now.0.saturating_sub(d.0), "送り出しの台詞の再生完了待ちが上限超過——打ち切って降ろす");
                    state.phase = Phase::Unloading {
                        cause: TermCause::DeadlineExceeded,
                    };
                    clear_choice_ledger(&mut state, "change_deadline_exceeded");
                    (state, vec![Action::ShioriUnload])
                }
                Some(_) => (state, Vec::new()),
            }
        }
        _ => {
            tracing::warn!(target: "kanade", event = "change_input_ignored", phase = phase_label(&state.phase), "送り出しの台詞の待ちに無関係な入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// 送り出しの台詞の利用者の中断: 切替を中止し、ゴーストを降ろさず元の定常へ戻す。
///
/// 切替の相に届く `Interrupted` は利用者の中断だけ（選択の帳簿は受理で消してある）。バルーンを
/// 隠すのは中断の受理の側（今日の規則のまま）で、ここは運行と通知だけを扱う。
fn cancel_by_user_break(mut state: State, talk_id: TalkId) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "change_cancelled", reason = "user_break", talk_id = talk_id.0, phase = phase_label(&state.phase), "送り出しの台詞を利用者が中断した——切替を中止して定常へ戻る");
    state.change = None;
    state.pending_change = None;
    state.phase = Phase::Steady { talk: None };
    let notice = KanadeNotice::ChangeCancelled {
        reason: CancelReason::UserBreak,
    };
    (state, vec![Action::Notice(notice)])
}

/// 切替の相への終了要求: 切替を取りやめ（中止を通知し）、相ごとに今日の終了の握手へ合流する。
///
/// - `ChangePending` → 終了を保留して応答を待つ（[`on_yielded_reply`]）。
/// - `ChangeTalkWait` → その台詞を定常のトークとして最後まで流し、完了で `OnClose`（定常の保留の終了）。
/// - `ChangeClosePending`／`ChangeCloseTalkWait` → `OnClose` は送ってあるので二度送らず、終了の握手の
///   同じ待ちへ移る（その別れの台詞の終わりで終了）。
///
/// 取りやめ済み（`ChangePending` で応答待ちのまま）の 2 件目は終了の保留を差し替えるだけ。
fn yield_to_close(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    let Some(change) = state.change.take() else {
        tracing::info!(target: "kanade", event = "change_close_pending", reason = reason.as_ref_str(), phase = phase_label(&state.phase), "切替は取りやめ済み——終了の保留を差し替えて応答を待つ");
        state.pending_close = Some(reason);
        return (state, Vec::new());
    };
    tracing::info!(target: "kanade", event = "change_yield_to_close", reason = reason.as_ref_str(), phase = phase_label(&state.phase), to = %change.req.target.name, "切替の相に終了要求——切替を取りやめ終了の握手へ合流する");
    state.pending_change = None;
    state.phase = match state.phase {
        Phase::ChangeTalkWait { talk_id, .. } => {
            state.pending_close = Some(reason);
            Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin: "OnGhostChanging",
                    script: change.script.unwrap_or_default(),
                }),
            }
        }
        Phase::ChangeClosePending => Phase::ClosePending { reason },
        Phase::ChangeCloseTalkWait { talk_id, deadline } => {
            Phase::CloseTalkWait { talk_id, deadline }
        }
        other => {
            state.pending_close = Some(reason);
            other
        }
    };
    let notice = KanadeNotice::ChangeCancelled {
        reason: CancelReason::CloseRequest,
    };
    (state, vec![Action::Notice(notice)])
}

/// 終了要求で取りやめた `ChangePending` の応答: 台本は定常のトークとして流し（完了で保留の終了を
/// 消化）、204 は保留の終了を今日の握手（`OnClose` GET）で始める。
fn on_yielded_reply(
    mut state: State,
    input: Input,
    reason: CloseReason,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    match input {
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(script),
            ..
        } => {
            let talk_id = TalkId(state.next_talk_id);
            state.next_talk_id += 1;
            tracing::info!(target: "kanade", event = "change_yielded_talk_start", talk_id = talk_id.0, "取りやめた切替の台本——定常のトークとして流し完了で終了の握手へ");
            state.phase = Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin: "OnGhostChanging",
                    script: script.clone(),
                }),
            };
            (
                state,
                vec![Action::StartTalk(StartTalk::new(talk_id, script))],
            )
        }
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            ..
        } => {
            state.pending_close = None;
            state.phase = Phase::Steady { talk: None };
            steady::step(state, Input::CloseRequest { reason }, config)
        }
        Input::Tick { now } => {
            state.last_now = Some(now);
            (state, Vec::new())
        }
        _ => {
            tracing::warn!(target: "kanade", event = "change_input_ignored", phase = phase_label(&state.phase), "取りやめた切替の応答待ちに無関係な入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// 台詞の完了待ちの相（`closing` なら切替の `OnClose` の台詞）。
fn talk_wait(closing: bool, talk_id: TalkId, deadline: Option<MonotonicMs>) -> Phase {
    if closing {
        Phase::ChangeCloseTalkWait { talk_id, deadline }
    } else {
        Phase::ChangeTalkWait { talk_id, deadline }
    }
}

#[cfg(test)]
#[path = "change_tests.rs"]
mod tests;
