//! 定常運転の Phase 分岐（Req 2・3・pump ゲート・talk 調停・保留 close）。
//!
//! 本モジュールは [`Phase::Steady`] における Tick pump（OnSecondChange GET/NOTIFY の
//! 使い分け）・talk 調停（Value→StartTalk）・boot 中／active talk 中に受領した close
//! 指示の保留処理を担う。
//!
//! # pump ゲート（Req 3.1・3.4・DD-6）
//! 定常運転でのみ毎秒 pump を発行する（boot 中・close 握手中以降・終了系列では発行しない
//! ——ゲートは Steady に閉じている）。再生中でないとき（`talk: None`）は問い合わせ（GET・
//! Ref3=1）、再生中（`talk: Some`）は NOTIFY（Ref3=0）で発行し分ける。NOTIFY 応答は
//! 構造的に破棄されるため、active talk 中に Value が届く重複調停を**発生源から断つ**
//! （DD-6）。
//!
//! # 保留 close の消化（System Flows 補足遷移）
//! boot 中・active talk 中に受領した close は `pending_close` に記録され（本層では作らない・
//! boot.rs／CloseRequest アームが記録する）、次の消化点で握手を開始する:
//! - `Steady{talk: None}` の次 Tick（Steady 遷移直後の握手開始）
//! - active talk の `TalkDone{reason: Ended | Interrupted}` 受領時
//!
//! 握手開始＝`OnClose` GET 発行＋`ClosePending` への遷移（ClosePending 以降は close.rs＝
//! タスク 2.5 の責務）。

use super::{events, Action, ActiveTalk, Input, Phase, State};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriOutcome};
use crate::talk::{StartTalk, TalkDone, TalkId};

/// 定常運転（Steady）のフェーズ分岐。
///
/// [`Phase::Steady`] にルーティングされた入力（Tick／ShioriReply／
/// TalkDone{reason: Ended | Interrupted}／CloseRequest）を処理する。Tick は pump ゲート
/// （GET/NOTIFY 使い分け）・ShioriReply は talk 調停（Value→StartTalk）・TalkDone/CloseRequest
/// は保留 close の記録／消化を担う。
pub(crate) fn step(state: State, input: Input, _config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        Input::Tick { now } => on_tick(state, now),
        Input::ShioriReply { outcome } => on_reply(state, outcome),
        Input::TalkDone(done) => on_talk_done(state, done),
        Input::CloseRequest { reason } => on_close_request(state, reason),
        // 上記以外（Boot・ForceQuit 等）は横断アームで捌かれ Steady には届かない。
        other => {
            let _ = &other;
            tracing::warn!(target: "kanade", event = "steady_input_ignored", "Steady に無関係な入力を無視");
            (state, Vec::new())
        }
    }
}

/// Steady での Tick（pump ゲート・Req 3.1／3.4／DD-6）。
///
/// まず `last_now` を必ず更新する（時刻は発行有無に依らず進む・close 期限計算の基準）。
/// その上で:
/// - `talk: None` かつ `pending_close` あり → close 握手開始（OnClose GET・ClosePending へ）。
/// - `talk: None` かつ `pending_close` なし → OnSecondChange **GET**（Ref3=1・pump 問い合わせ）。
/// - `talk: Some` → OnSecondChange **NOTIFY**（Ref3=0・応答無視・pending_close は消化しない）。
fn on_tick(mut state: State, now: MonotonicMs) -> (State, Vec<Action>) {
    state.last_now = Some(now);
    match state.phase {
        Phase::Steady { talk: None } => {
            if let Some(reason) = state.pending_close.take() {
                begin_close(state, reason)
            } else {
                // pump 問い合わせ（GET・Ref3=1）。応答待ちのまま Steady{None} を維持する。
                (
                    state,
                    vec![Action::ShioriRequest(events::on_second_change(now, true))],
                )
            }
        }
        Phase::Steady { talk: Some(_) } => {
            // active talk 中は NOTIFY（Ref3=0・応答は構造的に破棄）。pending_close があっても
            // ここでは握手を開始しない——当該 talk の TalkDone を待つ（DD-6・補足遷移）。
            (
                state,
                vec![Action::ShioriRequest(events::on_second_change(now, false))],
            )
        }
        // Steady 以外はルーティング上到達しない（防御アーム）。
        _ => steady_phase_unexpected(state, "Tick"),
    }
}

/// Steady での ShioriReply（talk 調停・Req 2.1／2.3／3.3）。
///
/// mod.rs は非 Failed 応答のみを Steady へ委譲する（Failed は横断アームで Unloading{Fault}）。
/// - `talk: None` + `Value(script)` → 一意 talk_id 採番＋StartTalk・`Steady{Some}` へ（Req 3.3・2.1）。
/// - `talk: None` + `NoContent`（204）→ StartTalk なし・`Steady{None}` 維持（Req 2.3）。
/// - `talk: Some` + `Notified` → NOTIFY pump の応答・`Steady{Some}` 維持（無視）。
/// - `talk: Some` + `Value` → DD-6 防御（構造上発生しない）・warn!＋破棄・`Steady{Some}` 維持。
fn on_reply(mut state: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Steady { talk: None } => match outcome {
            ShioriOutcome::Value(script) => {
                let talk_id = TalkId(state.next_talk_id);
                state.next_talk_id += 1;
                tracing::info!(target: "kanade", event = "steady_talk", talk_id = talk_id.0, "OnSecondChange にスクリプト——再生起動");
                state.phase = Phase::Steady {
                    talk: Some(ActiveTalk {
                        talk_id,
                        origin: "OnSecondChange",
                    }),
                };
                (state, vec![Action::StartTalk(StartTalk { talk_id, script })])
            }
            ShioriOutcome::NoContent => {
                // 204: talk なし（Req 2.3）。Steady{None} を維持し次 Tick で pump 再開。
                (state, Vec::new())
            }
            other => steady_reply_unexpected(state, "Steady{None}", other),
        },
        Phase::Steady { talk: Some(_) } => match outcome {
            ShioriOutcome::Notified => {
                // NOTIFY pump（Ref3=0）の応答。構造的に無視し Steady{Some} を維持する。
                (state, Vec::new())
            }
            ShioriOutcome::Value(_) => {
                // DD-6 防御: active talk は NOTIFY を発行するため Value は届かないはず。
                // 万一届いても StartTalk・キュー・中断を一切行わず破棄する。
                tracing::warn!(target: "kanade", event = "steady_value_during_talk", "active talk 中に Value——構造上想定外・破棄（キュー/中断なし）");
                (state, Vec::new())
            }
            other => steady_reply_unexpected(state, "Steady{Some}", other),
        },
        _ => steady_phase_unexpected(state, "ShioriReply"),
    }
}

/// Steady での TalkDone{reason: Ended | Interrupted}（Req 3.4 復帰／補足遷移）。
///
/// mod.rs は既知 talk の Ended／Interrupted（非 quit）のみを Steady へ委譲する（Quit は
/// 横断アームで Unloading{Quit}・未知 talk_id は error!＋維持）。ゆえに本アームは現 Steady talk の
/// 完了のみを受ける:
/// - `pending_close` あり → close 握手開始（OnClose GET・ClosePending へ・補足遷移）。
/// - `pending_close` なし → `Steady{None}` へ復帰（次 Tick で pump 再開・Req 3.4）。
fn on_talk_done(mut state: State, done: TalkDone) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Steady { talk: Some(_) } => {
            let _ = done; // talk_id 突合は mod.rs 済み（既知 talk の非 quit のみ到達）。
            if let Some(reason) = state.pending_close.take() {
                tracing::info!(target: "kanade", event = "steady_talk_done_close", reason = reason.as_ref_str(), "talk 完了——保留 close を消化し握手開始");
                begin_close(state, reason)
            } else {
                tracing::info!(target: "kanade", event = "steady_talk_done", "talk 完了——定常運転へ復帰");
                state.phase = Phase::Steady { talk: None };
                (state, Vec::new())
            }
        }
        _ => steady_phase_unexpected(state, "TalkDone"),
    }
}

/// Steady での CloseRequest（Req 4.1 起点／補足遷移）。
///
/// - `talk: None` → 待つべき talk がないため即握手開始（OnClose GET・ClosePending へ）。
/// - `talk: Some` → `pending_close` に記録するのみ・`Steady{Some}` 維持（TalkDone を待つ）。
fn on_close_request(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Steady { talk: None } => {
            tracing::info!(target: "kanade", event = "steady_close_now", reason = reason.as_ref_str(), "close 指示——active talk なし・即握手開始");
            begin_close(state, reason)
        }
        Phase::Steady { talk: Some(_) } => {
            tracing::info!(target: "kanade", event = "steady_close_pending", reason = reason.as_ref_str(), "close 指示——active talk あり・保留記録（TalkDone を待つ）");
            state.pending_close = Some(reason);
            (state, Vec::new())
        }
        _ => steady_phase_unexpected(state, "CloseRequest"),
    }
}

/// close 握手開始: `OnClose` GET を発行し `ClosePending{reason}` へ遷移する。
///
/// この Steady→ClosePending 遷移は本タスク（2.4）の責務である。ClosePending 以降の握手
/// （応答処理・CloseTalkWait・期限・quit:false→Steady・204→無言終了）は close.rs（タスク
/// 2.5）が実装する。
fn begin_close(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "close_handshake_begin", reason = reason.as_ref_str(), "OnClose GET を発行し握手を開始");
    state.phase = Phase::ClosePending { reason };
    (state, vec![Action::ShioriRequest(events::on_close(reason))])
}

/// Steady 以外の Phase が steady::step に届いた場合の防御アーム（構造上発生しない）。
fn steady_phase_unexpected(state: State, input: &str) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "steady_phase_unexpected", input = input, "Steady 以外の Phase への Steady 入力——現 Phase 維持で継続");
    (state, Vec::new())
}

/// Steady で想定外の SHIORI 応答（GET 待ちに Unloaded 等）を受けた場合の防御アーム。
fn steady_reply_unexpected(
    state: State,
    phase: &str,
    _outcome: ShioriOutcome,
) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "steady_unexpected_reply", phase = phase, "Steady 待ち点で想定外の SHIORI 応答——現 Phase 維持で継続");
    (state, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::ShioriCall;
    use crate::schedule::step;
    use crate::talk::TalkEndReason;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// Steady{talk: None}（pending_close なし）を任意時刻・任意採番で構築する。
    fn steady_none(next_id: u64) -> State {
        State {
            phase: Phase::Steady { talk: None },
            last_now: Some(MonotonicMs(500)),
            next_talk_id: next_id,
            pending_close: None,
        }
    }

    /// Steady{talk: Some(id)} を構築する。
    fn steady_some(talk_id: TalkId, next_id: u64) -> State {
        State {
            phase: Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin: "OnSecondChange",
                }),
            },
            last_now: Some(MonotonicMs(500)),
            next_talk_id: next_id,
            pending_close: None,
        }
    }

    /// 単一 Action が期待 ShioriCall（GET/NOTIFY・id・references）と一致することを検証する。
    fn assert_shiori(action: &Action, expected: &ShioriCall) {
        match (action, expected) {
            (
                Action::ShioriRequest(ShioriCall::Get { id, references }),
                ShioriCall::Get {
                    id: eid,
                    references: erefs,
                },
            ) => {
                assert_eq!(id, eid, "GET id 不一致");
                assert_eq!(references, erefs, "GET references 不一致");
            }
            (
                Action::ShioriRequest(ShioriCall::Notify { id, references }),
                ShioriCall::Notify {
                    id: eid,
                    references: erefs,
                },
            ) => {
                assert_eq!(id, eid, "NOTIFY id 不一致");
                assert_eq!(references, erefs, "NOTIFY references 不一致");
            }
            _ => panic!("ShioriRequest の GET/NOTIFY 種別が期待と不一致"),
        }
    }

    /// Action 列に OnSecondChange の ShioriRequest が一切ないことを検証する（ゲート閉）。
    fn assert_no_second_change(actions: &[Action]) {
        for a in actions {
            if let Action::ShioriRequest(
                ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. },
            ) = a
            {
                assert_ne!(*id, "OnSecondChange", "ゲートが閉じておらず OnSecondChange を発行した");
            }
        }
    }

    // === pump ゲート表駆動（観測可能な完了条件） ===
    // {起動中, Steady(None), Steady(Some), close 握手中以降} × Tick の発行有無・種別。

    // --- Steady{None} + Tick → OnSecondChange GET（Ref3=1）・Steady{None}・last_now 更新 ---

    #[test]
    fn steady_none_tick_emits_get_and_updates_last_now() {
        let now = MonotonicMs(7_200_000); // 2 hours。
        let (next, actions) = step(steady_none(5), Input::Tick { now }, &config());
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert_eq!(next.last_now, Some(now), "last_now は Tick ごとに更新される");
        assert_eq!(actions.len(), 1);
        // GET（Ref3=1）——events:: の出力と厳密一致。
        assert_shiori(&actions[0], &events::on_second_change(now, true));
    }

    // --- Steady{Some} + Tick → OnSecondChange NOTIFY（Ref3=0）・Steady{Some} ---

    #[test]
    fn steady_some_tick_emits_notify() {
        let now = MonotonicMs(3_600_000); // 1 hour。
        let (next, actions) = step(steady_some(TalkId(3), 6), Input::Tick { now }, &config());
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
            _ => panic!("expected Steady{{Some}}"),
        }
        assert_eq!(next.last_now, Some(now));
        assert_eq!(actions.len(), 1);
        // NOTIFY（Ref3=0）——events:: の出力と厳密一致。
        assert_shiori(&actions[0], &events::on_second_change(now, false));
    }

    // --- Steady{None} + Tick with pending_close → OnClose GET・ClosePending・pending 消化 ---

    #[test]
    fn steady_none_tick_with_pending_close_begins_handshake() {
        let now = MonotonicMs(1_000);
        let mut s = steady_none(5);
        s.pending_close = Some(CloseReason::User);
        let (next, actions) = step(s, Input::Tick { now }, &config());
        assert!(
            matches!(next.phase, Phase::ClosePending { reason: CloseReason::User }),
            "pending_close あり Tick は握手を開始し ClosePending へ"
        );
        assert!(next.pending_close.is_none(), "pending_close は消化される");
        assert_eq!(next.last_now, Some(now), "握手開始でも last_now は更新される");
        assert_eq!(actions.len(), 1);
        assert_shiori(&actions[0], &events::on_close(CloseReason::User));
        // OnSecondChange は発行しない。
        assert_no_second_change(&actions);
    }

    // --- boot 中（BootMain）+ Tick → OnSecondChange なし（ゲート閉・boot は pump しない） ---

    #[test]
    fn boot_phase_tick_emits_no_second_change() {
        let s = State {
            phase: Phase::BootMain,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        };
        let (_next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000) }, &config());
        // boot::step は pump を発行しない（ゲートは Steady に閉じている）。
        assert_no_second_change(&actions);
    }

    // --- close 握手中以降（ClosePending / CloseTalkWait）+ Tick → OnSecondChange なし ---

    #[test]
    fn close_pending_tick_emits_no_second_change() {
        let s = State {
            phase: Phase::ClosePending {
                reason: CloseReason::System,
            },
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        };
        let (_next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000) }, &config());
        // close::step は現状 stub（pump 非発行）——OnSecondChange が出ないことを検証。
        assert_no_second_change(&actions);
    }

    #[test]
    fn close_talk_wait_tick_emits_no_second_change() {
        let s = State {
            phase: Phase::CloseTalkWait {
                talk_id: TalkId(2),
                deadline: None,
            },
            last_now: None,
            next_talk_id: 3,
            pending_close: None,
        };
        let (_next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000) }, &config());
        assert_no_second_change(&actions);
    }

    // === talk 調停（ShioriReply） ===

    // --- Steady{None} + Value → StartTalk(id) + Steady{Some(id)}・id 単調増番 ---

    #[test]
    fn steady_none_value_starts_talk_and_ids_are_monotonic() {
        // 1 本目: next_id=5 → id=5。
        let (s1, actions1) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("hello".to_string()),
            },
            &config(),
        );
        match s1.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, origin }),
            } => {
                assert_eq!(talk_id, TalkId(5));
                assert_eq!(origin, "OnSecondChange");
            }
            _ => panic!("expected Steady{{Some}}"),
        }
        assert_eq!(s1.next_talk_id, 6, "採番カウンタが進む");
        assert_eq!(actions1.len(), 1);
        match &actions1[0] {
            Action::StartTalk(StartTalk { talk_id, script }) => {
                assert_eq!(*talk_id, TalkId(5));
                assert_eq!(script, "hello");
            }
            _ => panic!("expected StartTalk"),
        }

        // 2 本目（引き継いだカウンタ 6 で別 Steady{None} を想定）→ id=6（再利用しない）。
        let (s2, actions2) = step(
            steady_none(s1.next_talk_id),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("world".to_string()),
            },
            &config(),
        );
        let id2 = match &actions2[0] {
            Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
            _ => panic!("expected StartTalk"),
        };
        assert_eq!(id2, TalkId(6), "id は単調増番・再利用しない");
        assert_eq!(s2.next_talk_id, 7);
    }

    // --- Steady{None} + NoContent(204) → no StartTalk・Steady{None} 維持 ---

    #[test]
    fn steady_none_no_content_starts_no_talk() {
        let (next, actions) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert_eq!(next.next_talk_id, 5, "204 は採番しない");
        assert!(actions.is_empty(), "204 は talk 起動しない（Req 2.3）");
    }

    // --- Steady{Some} + Notified → Steady{Some} 維持（NOTIFY pump の応答・無視） ---

    #[test]
    fn steady_some_notified_stays_and_emits_nothing() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3)),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert!(actions.is_empty());
    }

    // --- Steady{Some} + Value → warn!+破棄・Steady{Some} 維持・StartTalk なし（DD-6 防御） ---

    #[test]
    fn steady_some_value_is_discarded_without_start_talk() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("late".to_string()),
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert_eq!(next.next_talk_id, 6, "破棄ゆえ採番しない");
        assert!(
            !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "Value-during-talk は StartTalk しない（DD-6）"
        );
        assert!(actions.is_empty(), "キュー・中断も発行しない");
    }

    // === TalkDone{reason: Ended | Interrupted}（非 quit の 2 値ルーティング網羅） ===

    // --- Steady{Some(id)} + TalkDone{id, Ended}, pending None → Steady{None}・次 Tick で pump 再開 ---

    #[test]
    fn steady_talk_done_ended_resumes_steady_and_pump_restarts() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }), "talk 完了で定常復帰");
        assert!(actions.is_empty(), "TalkDone 自体は副作用なし");

        // 復帰後の次 Tick で pump（GET）が再開することを確認（Req 3.4）。
        let now = MonotonicMs(9_000);
        let (after, tick_actions) = step(next, Input::Tick { now }, &config());
        assert!(matches!(after.phase, Phase::Steady { talk: None }));
        assert_eq!(tick_actions.len(), 1);
        assert_shiori(&tick_actions[0], &events::on_second_change(now, true));
    }

    // --- Steady{Some(id)} + TalkDone{id, Interrupted}, pending None → 同じく Steady{None} 復帰 ---
    // kanade の 3 値ルーティング（本タスクの担当）: Interrupted は Ended と同一経路（非 quit）
    // として steady::on_talk_done に到達する（mod.rs が防御的に非 quit 扱いへ振る）。

    #[test]
    fn steady_talk_done_interrupted_resumes_steady_same_as_ended() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Interrupted,
            }),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: None }),
            "Interrupted も Ended と同じく定常復帰へ"
        );
        assert!(actions.is_empty(), "TalkDone 自体は副作用なし");
    }

    // --- Steady{Some(id)} + TalkDone{id, Ended}, pending Some → OnClose GET + ClosePending ---

    #[test]
    fn steady_talk_done_with_pending_close_begins_handshake() {
        let mut s = steady_some(TalkId(3), 6);
        s.pending_close = Some(CloseReason::System);
        let (next, actions) = step(
            s,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::ClosePending { reason: CloseReason::System }),
            "talk 完了時に保留 close を消化して握手開始"
        );
        assert!(next.pending_close.is_none(), "pending_close は消化される");
        assert_eq!(actions.len(), 1);
        assert_shiori(&actions[0], &events::on_close(CloseReason::System));
    }

    // === CloseRequest ===

    // --- Steady{None} + CloseRequest → OnClose GET + ClosePending（即握手） ---

    #[test]
    fn steady_none_close_request_begins_handshake_now() {
        let (next, actions) = step(
            steady_none(5),
            Input::CloseRequest {
                reason: CloseReason::User,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::ClosePending { reason: CloseReason::User }));
        assert_eq!(actions.len(), 1);
        assert_shiori(&actions[0], &events::on_close(CloseReason::User));
    }

    // --- Steady{Some} + CloseRequest → pending_close 記録・Steady{Some} 維持（OnClose まだ） ---

    #[test]
    fn steady_some_close_request_records_pending_only() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::CloseRequest {
                reason: CloseReason::User,
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk 中は Steady{{Some}} を維持"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert!(
            matches!(next.pending_close, Some(CloseReason::User)),
            "pending_close に記録される（TalkDone を待つ）"
        );
        assert!(actions.is_empty(), "OnClose はまだ発行しない");
    }
}
