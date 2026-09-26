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

use super::close::deadline_from;
use super::{
    Action, Input, Phase, State, TermCause, clear_choice_ledger, events, phase_label, steady,
    to_unloading_quit,
};
use crate::change::{CancelReason, ChangeRequest, KanadeNotice};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriOutcome};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkDone, TalkId};

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

/// 切替の相の振り分け（`dispatch_phase` から）。
pub(super) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
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
/// - `TalkDone` → 降ろす（`Unloading{Quit}`）。`\-` の完了は横断の腕が同じ終端へ送る。
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
