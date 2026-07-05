//! close 握手の Phase 分岐（Req 4・OnClose GET・期限判定・終了系列）。
//!
//! 本モジュールは close 各待ち点（[`Phase::ClosePending`] の OnClose 応答待ち・
//! [`Phase::CloseTalkWait`] の close talk 再生完了待ち／期限判定）の遷移本体を担う。
//! OnClose GET 自体は steady 側（`begin_close`）が ClosePending への遷移時に発行済み
//! であり、本モジュールはその応答（Value／NoContent）分岐から先を実装する。
//!
//! # close 握手の状態遷移（Req 4.1〜4.7・DD-11）
//! - `ClosePending{reason}` + `ShioriReply{Value(script)}` → 応答スクリプトを一意 talk_id
//!   で再生起動要求（StartTalk）として配送し、`CloseTalkWait` へ遷移して再生完了通知を
//!   待つ（Req 4.2・2.1・運行保留）。
//! - `ClosePending{reason}` + `ShioriReply{NoContent}`（204）→ **応答なし＝無言終了**
//!   （DD-11: 204 は拒否ではない・OnCloseAll は発行しない）。追加イベントを発行せず
//!   `Unloading{CloseSilent}` へ直行する（Req 4.6）。
//! - `CloseTalkWait{talk_id}` + `TalkDone{talk_id, quit:false}` → **終了拒否**（Req 4.5）。
//!   終了させず `Steady{None}` へ復帰し、次 Tick で pump が再開する。
//!   （quit:true は横断アームが `Unloading{Quit}` へ送るため本層には届かない。）
//! - `CloseTalkWait` + `Tick` → 再生完了待ちに時刻基準の上限（deadline）を設け、上限超過を
//!   判定して超過なら `error!` の上で `Unloading{DeadlineExceeded}` へ進む（Req 4.7）。
//!   握手入口で `last_now` が None（Tick 未受領）だった場合は deadline を None のまま入り、
//!   CloseTalkWait 最初の Tick 受領時点を起点に上限を設定する。
//!
//! # ログ規律（steering: areka-log-first-no-silent-failure）
//! すべての遷移・防御アームは `tracing` を発行する。沈黙の失敗経路は存在しない。

use super::{Action, Input, Phase, State, TermCause};
use crate::msg::{KanadeConfig, MonotonicMs, ShioriOutcome};
use crate::talk::{StartTalk, TalkId};

/// close 握手（ClosePending / CloseTalkWait）のフェーズ分岐。
///
/// mod.rs のルーティング契約:
/// - `ClosePending` の ShioriReply（非 Failed）→ 本 step。
/// - 既知 talk の `CloseTalkWait` + `TalkDone{quit:false}` → 本 step。
/// - `ClosePending` / `CloseTalkWait` の Tick → 本 step。
///
/// quit:true・ForceQuit・ShioriDown・Failed は横断アームで捌かれ本層には届かない。
pub(crate) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match state.phase {
        Phase::ClosePending { .. } => on_close_pending(state, input, config),
        Phase::CloseTalkWait { .. } => on_close_talk_wait(state, input, config),
        // ルーティング上ここには ClosePending / CloseTalkWait 以外は届かない（防御アーム）。
        _ => {
            tracing::warn!(target: "kanade", event = "close_phase_unexpected", "close 以外の Phase への close 入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// `ClosePending`（OnClose GET 応答待ち）の遷移。
///
/// - `ShioriReply{Value(script)}` → StartTalk 配送＋`CloseTalkWait` へ（deadline を入口で計算）。
/// - `ShioriReply{NoContent}`（204）→ 無言終了（`Unloading{CloseSilent}`・追加イベントなし）。
/// - `ShioriReply{Notified}` → 構造上あり得ない（OnClose は GET）・warn!＋維持。
/// - `Tick{now}` → `last_now` 更新のみ・ClosePending 維持（close 中は pump しない）。
/// - その他 → 防御アーム（warn!＋維持）。
fn on_close_pending(mut state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        Input::ShioriReply { outcome } => match outcome {
            ShioriOutcome::Value(script) => {
                // 応答スクリプトあり: 一意 talk_id を採番し再生起動要求として配送、
                // CloseTalkWait へ遷移して再生完了通知まで運行を保留する（Req 4.2・2.1）。
                let talk_id = TalkId(state.next_talk_id);
                state.next_talk_id += 1;
                let deadline = deadline_from(state.last_now, config);
                tracing::info!(target: "kanade", event = "close_talk_start", talk_id = talk_id.0, "OnClose 応答スクリプト——close talk を再生起動し完了を待機");
                state.phase = Phase::CloseTalkWait { talk_id, deadline };
                (state, vec![Action::StartTalk(StartTalk { talk_id, script })])
            }
            ShioriOutcome::NoContent => {
                // 204 = 応答なし（≠拒否・DD-11）。追加イベントを発行せず無言終了へ直行する
                // （OnCloseAll は発行しない・Req 4.6）。
                tracing::info!(target: "kanade", event = "close_silent", "OnClose 応答なし（204）——追加イベントなしで無言終了へ");
                state.phase = Phase::Unloading {
                    cause: TermCause::CloseSilent,
                };
                (state, vec![Action::ShioriUnload])
            }
            ShioriOutcome::Notified => {
                // OnClose は GET ゆえ Notified は構造上あり得ない（防御アーム）。
                tracing::warn!(target: "kanade", event = "close_notified_unexpected", "ClosePending で Notified——OnClose は GET・構造上想定外・維持");
                (state, Vec::new())
            }
            other => {
                // Value/NoContent/Notified 以外（Unloaded 等）は待ち点に不整合。
                let _ = other;
                tracing::warn!(target: "kanade", event = "close_reply_unexpected", "ClosePending で想定外の SHIORI 応答——現 Phase 維持で継続");
                (state, Vec::new())
            }
        },
        Input::Tick { now } => {
            // close 中は pump しない（OnClose GET の応答はシェルの要求タイムアウトで律速）。
            state.last_now = Some(now);
            (state, Vec::new())
        }
        other => {
            let _ = other;
            tracing::warn!(target: "kanade", event = "close_pending_input_ignored", "ClosePending に無関係な入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// `CloseTalkWait`（close talk 再生完了待ち／期限判定）の遷移。
///
/// - `TalkDone{quit:false}` → **終了拒否**・`Steady{None}` へ復帰（Req 4.5・次 Tick で pump 再開）。
/// - `Tick{now}` → 期限判定（Req 4.7・注入時刻のみで決定的）:
///   - deadline 未設定（None）→ 本 Tick を起点に設定・維持。
///   - `now >= deadline` → error!＋`Unloading{DeadlineExceeded}`（終了系列継続）。
///   - `now < deadline` → `last_now` 更新のみ・維持。
/// - その他 → 防御アーム（warn!＋維持）。
fn on_close_talk_wait(
    mut state: State,
    input: Input,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    // 現 Phase から talk_id / deadline を取り出す（呼び出し契約上 CloseTalkWait 確定）。
    let (talk_id, deadline) = match state.phase {
        Phase::CloseTalkWait { talk_id, deadline } => (talk_id, deadline),
        _ => {
            tracing::warn!(target: "kanade", event = "close_phase_unexpected", "CloseTalkWait 以外——現 Phase 維持で継続");
            return (state, Vec::new());
        }
    };

    match input {
        Input::TalkDone(done) => {
            // mod.rs は既知 talk の quit:false のみを委譲する（quit:true は横断アームで
            // Unloading{Quit}）。ゆえに本アームは close talk 完了＝終了拒否のみを受ける。
            let _ = done; // talk_id 突合は mod.rs 済み。
            tracing::info!(target: "kanade", event = "close_refused", talk_id = talk_id.0, "close talk 完了・quit 偽——終了拒否・定常運転へ復帰");
            state.phase = Phase::Steady { talk: None };
            (state, Vec::new())
        }
        Input::Tick { now } => {
            state.last_now = Some(now);
            match deadline {
                None => {
                    // 握手入口で last_now が None だった場合、本 Tick を起点に上限を設定する。
                    let d = MonotonicMs(now.0.saturating_add(config.close_talk_deadline_ms));
                    tracing::info!(target: "kanade", event = "close_deadline_armed", talk_id = talk_id.0, deadline_ms = d.0, "CloseTalkWait 最初の Tick——再生完了待ち上限を設定");
                    state.phase = Phase::CloseTalkWait {
                        talk_id,
                        deadline: Some(d),
                    };
                    (state, Vec::new())
                }
                Some(d) if now >= d => {
                    // 上限超過（Req 4.7）: error! の上で終了系列を継続する。
                    let overshoot = now.0.saturating_sub(d.0);
                    tracing::error!(target: "kanade", event = "close_deadline_exceeded", talk_id = talk_id.0, overshoot_ms = overshoot, "close talk 再生完了待ちが上限超過——終了系列を継続");
                    state.phase = Phase::Unloading {
                        cause: TermCause::DeadlineExceeded,
                    };
                    (state, vec![Action::ShioriUnload])
                }
                Some(_) => {
                    // 未超過: last_now を更新し CloseTalkWait を維持する（発行なし）。
                    (state, Vec::new())
                }
            }
        }
        other => {
            let _ = other;
            tracing::warn!(target: "kanade", event = "close_talk_wait_input_ignored", "CloseTalkWait に無関係な入力——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// CloseTalkWait 入口の deadline を計算する（DD 表 close 再生完了待ち上限）。
///
/// `last_now == Some(n)` なら `n + close_talk_deadline_ms` を上限とする。`None`（Tick 未受領で
/// 握手に入った場合）は None のまま入り、CloseTalkWait 最初の Tick 受領時点で確定する。
fn deadline_from(last_now: Option<MonotonicMs>, config: &KanadeConfig) -> Option<MonotonicMs> {
    last_now.map(|n| MonotonicMs(n.0.saturating_add(config.close_talk_deadline_ms)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{CloseReason, ShioriCall};
    use crate::schedule::step;
    use crate::talk::TalkDone;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// `close_talk_deadline_ms` を任意に差し替えた config（期限判定を小さな値で回す）。
    fn config_with_deadline(deadline_ms: u64) -> KanadeConfig {
        let mut c = KanadeConfig::new("master", "1.0.0");
        c.close_talk_deadline_ms = deadline_ms;
        c
    }

    /// ClosePending 状態を任意の last_now / next_id で構築する。
    fn close_pending(reason: CloseReason, last_now: Option<MonotonicMs>, next_id: u64) -> State {
        State {
            phase: Phase::ClosePending { reason },
            last_now,
            next_talk_id: next_id,
            pending_close: None,
        }
    }

    /// CloseTalkWait 状態を任意の talk_id / deadline / last_now で構築する。
    fn close_talk_wait(
        talk_id: TalkId,
        deadline: Option<MonotonicMs>,
        last_now: Option<MonotonicMs>,
        next_id: u64,
    ) -> State {
        State {
            phase: Phase::CloseTalkWait { talk_id, deadline },
            last_now,
            next_talk_id: next_id,
            pending_close: None,
        }
    }

    // === 分岐1: 応答スクリプトあり（quit 真） ===
    // ClosePending + Value → StartTalk + CloseTalkWait（deadline = last_now + D）。
    // 続く TalkDone{quit:true} は横断アームで Unloading{Quit} + ShioriUnload。

    #[test]
    fn value_starts_close_talk_then_quit_true_unloads() {
        let d = 30_000; // 既定。
        let (s1, actions1) = step(
            close_pending(CloseReason::User, Some(MonotonicMs(1_000)), 5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("bye".to_string()),
            },
            &config(),
        );
        // StartTalk(5, "bye") + CloseTalkWait{5, deadline=Some(1000+D)}。
        match s1.phase {
            Phase::CloseTalkWait { talk_id, deadline } => {
                assert_eq!(talk_id, TalkId(5));
                assert_eq!(deadline, Some(MonotonicMs(1_000 + d)), "deadline は入口で last_now + D");
            }
            _ => panic!("expected CloseTalkWait"),
        }
        assert_eq!(s1.next_talk_id, 6, "talk_id 採番カウンタが進む");
        assert_eq!(actions1.len(), 1);
        match &actions1[0] {
            Action::StartTalk(StartTalk { talk_id, script }) => {
                assert_eq!(*talk_id, TalkId(5));
                assert_eq!(script, "bye");
            }
            _ => panic!("expected StartTalk"),
        }

        // quit:true TalkDone（既知 talk）→ 横断アームで Unloading{Quit} + ShioriUnload。
        let (s2, actions2) = step(
            s1,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                quit: true,
            }),
            &config(),
        );
        assert!(matches!(s2.phase, Phase::Unloading { cause: TermCause::Quit }));
        assert!(matches!(actions2.as_slice(), [Action::ShioriUnload]));
    }

    // === 分岐2: 応答スクリプトあり（quit 偽）→ 終了拒否→定常復帰→pump 再開 ===

    #[test]
    fn value_then_quit_false_refuses_close_and_resumes_pump() {
        let (s1, _a1) = step(
            close_pending(CloseReason::System, Some(MonotonicMs(1_000)), 5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("stay".to_string()),
            },
            &config(),
        );
        let talk_id = match s1.phase {
            Phase::CloseTalkWait { talk_id, .. } => talk_id,
            _ => panic!("expected CloseTalkWait"),
        };

        // TalkDone{quit:false}（既知 talk）→ 終了拒否・Steady{None} へ復帰。
        let (s2, actions2) = step(
            s1,
            Input::TalkDone(TalkDone {
                talk_id,
                quit: false,
            }),
            &config(),
        );
        assert!(matches!(s2.phase, Phase::Steady { talk: None }), "終了拒否で定常復帰");
        assert!(actions2.is_empty(), "TalkDone 自体は副作用なし");

        // 続く Tick で OnSecondChange GET（pump 再開・Req 4.5/3.4）。
        let now = MonotonicMs(9_000);
        let (s3, actions3) = step(s2, Input::Tick { now }, &config());
        assert!(matches!(s3.phase, Phase::Steady { talk: None }));
        assert_eq!(actions3.len(), 1);
        match &actions3[0] {
            Action::ShioriRequest(ShioriCall::Get { id, .. }) => {
                assert_eq!(*id, "OnSecondChange", "定常復帰後に pump が再開する");
            }
            _ => panic!("expected OnSecondChange GET after resume"),
        }
    }

    // === 分岐3: 応答なし（204）→ 無言終了 ===

    #[test]
    fn no_content_causes_silent_close() {
        let (next, actions) = step(
            close_pending(CloseReason::User, Some(MonotonicMs(1_000)), 5),
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Unloading { cause: TermCause::CloseSilent }),
            "204 は無言終了（DD-11）"
        );
        assert_eq!(next.next_talk_id, 5, "無言終了は talk 起動しない・採番しない");
        // ShioriUnload のみ・追加イベント（ShioriRequest = OnCloseAll 等）は発行しない（Req 4.6）。
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ShioriUnload));
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::ShioriRequest(_))),
            "無言終了は追加イベントを発行しない（OnCloseAll 非発行・DD-11）"
        );
    }

    // === 分岐4: 待ち時間超過 → Unloading{DeadlineExceeded} ===

    #[test]
    fn deadline_exceeded_continues_termination() {
        let d = 30_000;
        // CloseTalkWait に deadline=Some(1000+D) で入る（入口 last_now Some(1000)）。
        let s = close_talk_wait(
            TalkId(7),
            Some(MonotonicMs(1_000 + d)),
            Some(MonotonicMs(1_000)),
            8,
        );
        // now = 1000 + D（>= deadline）→ 超過。
        let (next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000 + d) }, &config());
        assert!(
            matches!(next.phase, Phase::Unloading { cause: TermCause::DeadlineExceeded }),
            "上限超過で終了系列（DeadlineExceeded）へ"
        );
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // === deadline: 入口 last_now None → 最初の Tick で確定 ===

    #[test]
    fn deadline_armed_on_first_tick_when_entered_without_now() {
        let small = 100;
        // 入口 last_now None → deadline None で CloseTalkWait に入る。
        let s = close_talk_wait(TalkId(2), None, None, 3);
        // 最初の Tick{now=500} → deadline = Some(500+100)・未超過・維持。
        let (s1, a1) = step(
            s,
            Input::Tick { now: MonotonicMs(500) },
            &config_with_deadline(small),
        );
        match s1.phase {
            Phase::CloseTalkWait { talk_id, deadline } => {
                assert_eq!(talk_id, TalkId(2));
                assert_eq!(deadline, Some(MonotonicMs(600)), "最初の Tick を起点に上限を設定");
            }
            _ => panic!("expected CloseTalkWait with armed deadline"),
        }
        assert_eq!(s1.last_now, Some(MonotonicMs(500)));
        assert!(a1.is_empty(), "起点設定のみ・発行なし");

        // 後続 Tick{now=600}（>= deadline）→ 超過 → Unloading{DeadlineExceeded}。
        let (s2, a2) = step(
            s1,
            Input::Tick { now: MonotonicMs(600) },
            &config_with_deadline(small),
        );
        assert!(matches!(s2.phase, Phase::Unloading { cause: TermCause::DeadlineExceeded }));
        assert!(matches!(a2.as_slice(), [Action::ShioriUnload]));
    }

    // === deadline: 未超過 Tick → 維持・last_now 更新・発行なし ===

    #[test]
    fn tick_before_deadline_stays_and_updates_last_now() {
        let d = 30_000;
        let s = close_talk_wait(
            TalkId(4),
            Some(MonotonicMs(1_000 + d)),
            Some(MonotonicMs(1_000)),
            5,
        );
        let now = MonotonicMs(1_000 + d - 1); // deadline 未満。
        let (next, actions) = step(s, Input::Tick { now }, &config());
        match next.phase {
            Phase::CloseTalkWait { talk_id, deadline } => {
                assert_eq!(talk_id, TalkId(4), "維持");
                assert_eq!(deadline, Some(MonotonicMs(1_000 + d)), "deadline 不変");
            }
            _ => panic!("expected CloseTalkWait preserved"),
        }
        assert_eq!(next.last_now, Some(now), "last_now は更新される");
        assert!(actions.is_empty(), "未超過は発行なし");
    }

    // === ClosePending + Tick → last_now 更新・ClosePending 維持 ===

    #[test]
    fn close_pending_tick_updates_last_now_and_stays() {
        let (next, actions) = step(
            close_pending(CloseReason::User, Some(MonotonicMs(1_000)), 5),
            Input::Tick { now: MonotonicMs(2_000) },
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::ClosePending { reason: CloseReason::User }),
            "ClosePending を維持"
        );
        assert_eq!(next.last_now, Some(MonotonicMs(2_000)), "last_now は更新される");
        assert!(actions.is_empty(), "close 中は pump しない");
    }

    // === ClosePending + Value で last_now None のとき deadline は None ===

    #[test]
    fn value_with_no_prior_tick_enters_wait_with_none_deadline() {
        let (next, actions) = step(
            close_pending(CloseReason::System, None, 5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("hi".to_string()),
            },
            &config(),
        );
        match next.phase {
            Phase::CloseTalkWait { talk_id, deadline } => {
                assert_eq!(talk_id, TalkId(5));
                assert_eq!(deadline, None, "Tick 未受領で握手に入ると deadline は None");
            }
            _ => panic!("expected CloseTalkWait with None deadline"),
        }
        assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    }

    // === 防御: ClosePending で Notified（構造上あり得ない）→ 維持・発行なし ===

    #[test]
    fn close_pending_notified_is_defensive_and_stays() {
        let (next, actions) = step(
            close_pending(CloseReason::User, Some(MonotonicMs(1_000)), 5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::ClosePending { reason: CloseReason::User }));
        assert!(actions.is_empty());
    }

    // === 防御: CloseTalkWait で予期しない ShioriReply → 維持・発行なし ===

    #[test]
    fn close_talk_wait_unexpected_reply_is_defensive() {
        let s = close_talk_wait(TalkId(2), Some(MonotonicMs(5_000)), Some(MonotonicMs(1_000)), 3);
        // CloseTalkWait は応答待ちではない（awaits_reply に含まれない）ため、mod.rs の
        // unexpected_reply アームで握り潰される想定だが、直接 close::step に渡した場合の
        // 防御も確認する（発行なし・維持）。
        let (next, actions) = on_close_talk_wait(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::CloseTalkWait { talk_id: TalkId(2), .. }));
        assert!(actions.is_empty());
    }
}
