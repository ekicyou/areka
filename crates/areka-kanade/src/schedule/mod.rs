//! schedule — 純粋運行状態機械（`src/schedule/`）。
//!
//! 全運行判断を純粋関数 [`step`] として実装する層である（I/O・スレッド・channel
//! 非依存＝決定的単体テストの本体）。[`step`] は現在の [`State`] と [`Input`] から
//! 次の [`State`] と副作用指示 [`Action`] の列を返す唯一の遷移入口であり、`tracing`
//! によるログ発行以外の副作用を持たない（可観測性の側効果であり状態・出力の決定性に
//! 影響しない・DD-3）。DD-9 により本モジュールは `pub(crate)` に閉じる。
//!
//! # 責務分割（本タスク 2.1 の担当範囲）
//! 本 `mod.rs` は**由来・状態を問わない横断遷移**（[`Input::TalkDone`] の理由が
//! [`TalkEndReason::Quit`] の場合／[`Input::ForceQuit`]／[`Input::ShioriDown`]／[`Input::ShioriReply`] の
//! 失敗）と、Unload 完了・防御アーム（未知 talk_id・Idle 以外の Boot・応答待ちでない
//! ShioriReply）を実装する。フェーズ固有の遷移は [`boot`]／[`steady`]／[`close`] の
//! 各サブモジュールへ委譲する（後続タスク 2.3／2.4／2.5 が本体を実装する）。
//!
//! # ログ規律（steering: areka-log-first-no-silent-failure）
//! すべての失敗・防御アームは `tracing::error!`／`tracing::warn!` を発行する。沈黙の
//! 失敗経路は存在しない。panic は新規導入しない（回復不能はすべて
//! `Unloading{Fault}`→`Stopped` の正規遷移で表現する・Req 6.4）。

use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, ShioriCall, ShioriOutcome};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkDone, TalkEndReason, TalkId};

pub(crate) mod boot;
pub(crate) mod close;
/// タスク 6.1: 純粋 step 層の失敗・防御アームのログ発火検証（テスト専用）。
#[cfg(test)]
pub(crate) mod log_capture;
/// ukadoc Reference 表の実装正本（純粋関数群）。DD-9 の例外として `pub`。
/// クレート公開面への露出は [`crate::events`] ファサード経由（[`crate::lib`] 参照）。
pub mod events;
pub(crate) mod steady;

/// 状態機械への入力。`KanadeMsg`（外部入力）＋シェルが同期往復で得た SHIORI 応答。
/// `ShioriReply` が `KanadeMsg` に存在しないため、応答注入経路はシェル内部に閉じる。
pub(crate) enum Input {
    Boot,
    Tick { now: MonotonicMs },
    TalkDone(TalkDone),
    CloseRequest { reason: CloseReason },
    ForceQuit { reason: CloseReason },
    ShioriDown { reason: String },
    /// 直前の Action::ShioriRequest／ShioriUnload の結果（シェルが即時再投入）。
    ShioriReply { outcome: ShioriOutcome },
}

/// 運行フェーズ（可視化は System Flows の状態機械図）。各待ち点は「直前に発行した
/// 呼出の応答待ち」を表す（in-flight ≤ 1 ゆえ相関 id 不要）。
pub(crate) enum Phase {
    Idle,
    BootInit,
    BootType,
    BootMain,
    BootVersion,
    Steady { talk: Option<ActiveTalk> },
    ClosePending { reason: CloseReason },
    CloseTalkWait { talk_id: TalkId, deadline: Option<MonotonicMs> },
    Unloading { cause: TermCause },
    Stopped,
}

/// 現在再生中の talk（origin は起動由来ラベル・ログ用）。
pub(crate) struct ActiveTalk {
    pub talk_id: TalkId,
    pub origin: &'static str,
}

/// 運行状態の全体（[`step`] の唯一の被写体）。Phase 外の帳簿はここに置く。
pub(crate) struct State {
    pub phase: Phase,
    /// 直近 Tick の注入時刻（Tick 受領ごとに更新・close 期限計算の基準）。
    pub last_now: Option<MonotonicMs>,
    /// talk_id 採番カウンタ（単調増番・再利用しない・StartTalk 生成時にインクリメント）。
    pub next_talk_id: u64,
    /// boot 中・active talk 中に受領した close 指示の保留（System Flows 補足遷移）。
    pub pending_close: Option<CloseReason>,
}

impl State {
    /// 初期運行状態（[`Phase::Idle`]・Tick 未受領・採番カウンタ 1・保留 close なし）。
    ///
    /// `next_talk_id` は 1 起点の単調増番であり、StartTalk 生成のたびにインクリメントし
    /// 再利用しない（[`crate::talk::TalkId`] の一意性契約）。
    pub(crate) fn initial() -> State {
        State {
            phase: Phase::Idle,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        }
    }
}

/// 終了系列の起因（ログ語彙・遷移は共通）。
pub(crate) enum TermCause {
    Quit,
    Forced,
    CloseSilent,
    DeadlineExceeded,
    Fault,
}

/// 状態機械が返す副作用指示（シェルが実行する）。
pub(crate) enum Action {
    /// GET／NOTIFY 発行（シェルが oneshot 往復し ShioriReply を再投入する）。
    ShioriRequest(ShioriCall),
    /// unload 発行（同上）。
    ShioriUnload,
    StartTalk(StartTalk),
    /// 終了系列完了（シェルは shiori へ Close を送り自身も Break する）。
    StopSelf,
}

/// 唯一の遷移入口。現在の [`State`] と [`Input`] から次の [`State`] と副作用指示
/// [`Action`] の列を返す純粋関数（`tracing` ログ発行のみ側効果として許容）。
///
/// 本タスク 2.1 は**横断遷移**（由来・状態を問わず終了系列へ進む共通ロジック）と
/// 防御アームを実装し、フェーズ固有の遷移は各サブモジュールへ委譲する。処理順は
/// 「横断遷移を先に判定 → 該当しなければフェーズ分岐」である。
pub(crate) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        // --- 横断遷移: 由来・状態を問わず終了系列へ進む共通ロジック ---

        // ForceQuit（全 Phase・DD-10）: best-effort OnClose NOTIFY を Action 先頭に積み、
        // quit ゲートを迂回して Unloading{Forced} へ直行する（Req 4.4）。
        Input::ForceQuit { reason } => force_quit(state, reason),

        // 死活報告（暫定 seam・DD-4）: error! 記録の上 Unloading{Fault} へ（Req 5.4）。
        Input::ShioriDown { reason } => {
            tracing::error!(target: "kanade", event = "shiori_down", reason = %reason, "SHIORI 死活報告を受領——終了系列（Fault）へ");
            to_unloading_fault(state)
        }

        // TalkDone: reason（3 値）・talk_id 突合を横断的に判定する（Req 2.5・4.3・6.2）。
        Input::TalkDone(done) => on_talk_done(state, done, config),

        // ShioriReply: Unload 完了・呼出失敗（Failed）の横断判定を先に行い、
        // それ以外は応答待ちフェーズへ委譲する（Req 6.1）。
        Input::ShioriReply { outcome } => on_shiori_reply(state, outcome, config),

        // --- 防御アーム・フェーズ固有遷移への委譲 ---

        // Idle 以外での Boot は不整合（warn!＋現 Phase 維持・Req 6.2）。Idle のみ boot へ委譲。
        Input::Boot => match state.phase {
            Phase::Idle => boot::step(state, Input::Boot, config),
            _ => {
                tracing::warn!(target: "kanade", event = "boot_ignored", "Idle 以外での Boot 指示を無視");
                (state, Vec::new())
            }
        },

        // Tick・CloseRequest はフェーズ固有遷移（後続タスクが本体を実装）。
        Input::Tick { now } => dispatch_phase(state, Input::Tick { now }, config),
        Input::CloseRequest { reason } => {
            dispatch_phase(state, Input::CloseRequest { reason }, config)
        }
    }
}

/// 送出時点の運行フェーズから実行状態スナップショットを導出する（DD-IT-3）。
/// アクティブな talk を運ぶ phase のみ talk_active=true。
pub(crate) fn snapshot_of(phase: &Phase) -> ExecutionSnapshot {
    match phase {
        Phase::Steady { talk: Some(_) } => ExecutionSnapshot { talk_active: true },
        _ => ExecutionSnapshot::INACTIVE,
    }
}

/// ForceQuit の横断遷移（DD-10）: best-effort OnClose NOTIFY を先頭に積み Unloading{Forced} へ。
///
/// OnClose NOTIFY の構築は events.rs（[`events::on_close_notify`]）へ委譲する——events.rs が
/// `ShioriCall` 構築の単一列挙点であり、force_quit はもはや inline 構築しない（DD-IT-8）。
/// スナップショットは Unloading へ遷移**後**の [`snapshot_of`]（＝INACTIVE）を渡す（DD-IT-4）。
fn force_quit(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "force_quit", reason = reason.as_ref_str(), "強制終了指示——終了系列（Forced）へ直行");
    state.phase = Phase::Unloading {
        cause: TermCause::Forced,
    };
    let notify =
        Action::ShioriRequest(events::on_close_notify(reason, &snapshot_of(&state.phase)));
    (state, vec![notify, Action::ShioriUnload])
}

/// 呼出失敗・死活報告の共通終端: Unloading{Fault}＋ShioriUnload（unload は best-effort）。
fn to_unloading_fault(mut state: State) -> (State, Vec<Action>) {
    state.phase = Phase::Unloading {
        cause: TermCause::Fault,
    };
    (state, vec![Action::ShioriUnload])
}

/// reason（3 値）・talk_id 突合。既知 talk の `TalkEndReason::Quit` は横断的に終了系列（Quit）へ。
///
/// `Ended` と `Interrupted` はいずれも非 quit としてフェーズ固有遷移（定常復帰・close 終了拒否）
/// へ委譲する（設計「kanade schedule の 3 値写像」）。M1 には user-interrupt 配線が無く、かつ
/// dispatcher の slot 差替に伴う `Interrupted` は dispatcher が stale として破棄するため、
/// `Interrupted` が kanade まで到達することは想定されない。到達した場合も専用状態は起こさず
/// `Ended` と同一経路（非 quit）へ防御的に委譲し、`info!` でどの reason だったかを観測する。
fn on_talk_done(state: State, done: TalkDone, config: &KanadeConfig) -> (State, Vec<Action>) {
    match current_talk_id(&state.phase) {
        Some(active) if active == done.talk_id => match done.reason {
            TalkEndReason::Quit => {
                // 既知 talk の Quit → 終了系列（Quit）へ直行（Req 4.3）。
                let mut state = state;
                tracing::info!(target: "kanade", event = "talk_done_quit", talk_id = done.talk_id.0, "reason=Quit——終了系列（Quit）へ");
                state.phase = Phase::Unloading {
                    cause: TermCause::Quit,
                };
                (state, vec![Action::ShioriUnload])
            }
            TalkEndReason::Interrupted => {
                // 防御的に非 quit 扱い（M1 では到達しない想定・観測用ログ）。
                tracing::info!(target: "kanade", event = "talk_done_interrupted_as_non_quit", talk_id = done.talk_id.0, "reason=Interrupted——防御的に非 quit 扱い・フェーズ固有遷移へ委譲");
                dispatch_phase(state, Input::TalkDone(done), config)
            }
            TalkEndReason::Ended => {
                // Ended（定常復帰・close talk 完了）はフェーズ固有遷移へ委譲。
                dispatch_phase(state, Input::TalkDone(done), config)
            }
        },
        Some(_) | None => {
            // 未知 talk_id の TalkDone → error!＋現 Phase 維持（Req 2.5・6.2）。
            tracing::error!(target: "kanade", event = "unknown_talk_done", talk_id = done.talk_id.0, "未知 talk_id の再生完了通知——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// ShioriReply の横断判定（Unload 完了・Failed）＋応答待ちフェーズ委譲。
fn on_shiori_reply(
    state: State,
    outcome: ShioriOutcome,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    // Unloading 中の応答は Unload 完了として扱う。Unloaded／Failed のいずれも Stopped へ
    // 進む（Failed は error! の上で終了系列を継続・Error Handling「Unload 失敗」行）。
    if matches!(state.phase, Phase::Unloading { .. }) {
        return unloading_reply(state, outcome);
    }

    // 応答待ちフェーズでの Failed は横断的に Unloading{Fault} へ（Req 6.1）。
    if let ShioriOutcome::Failed(ref failure) = outcome
        && awaits_reply(&state.phase)
    {
        tracing::error!(target: "kanade", event = "shiori_failed", error = %failure, "SHIORI 呼出失敗——終了系列（Fault）へ");
        return to_unloading_fault(state);
    }

    // 応答待ちでない Phase への ShioriReply は構造上発生しない（防御アーム・Req 6.2）。
    if !awaits_reply(&state.phase) {
        tracing::warn!(target: "kanade", event = "unexpected_reply", "応答待ちでない Phase への SHIORI 応答を無視");
        return (state, Vec::new());
    }

    // 正常応答（Value／NoContent／Notified）は応答待ちフェーズ固有遷移へ委譲。
    dispatch_phase(state, Input::ShioriReply { outcome }, config)
}

/// Unloading 中の ShioriReply: Unloaded／Failed とも Stopped＋StopSelf へ（Failed は error!）。
fn unloading_reply(mut state: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    if let ShioriOutcome::Failed(ref failure) = outcome {
        tracing::error!(target: "kanade", event = "unload_failed", error = %failure, "Unload 失敗——終了系列は継続し停止する");
    }
    state.phase = Phase::Stopped;
    (state, vec![Action::StopSelf])
}

/// フェーズ固有遷移への委譲（boot／steady／close・後続タスクが本体を実装）。
fn dispatch_phase(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Idle | Phase::BootInit | Phase::BootType | Phase::BootMain | Phase::BootVersion => {
            boot::step(state, input, config)
        }
        Phase::Steady { .. } => steady::step(state, input, config),
        Phase::ClosePending { .. } | Phase::CloseTalkWait { .. } => {
            close::step(state, input, config)
        }
        // 終了系列（Unloading／Stopped）に届いた非横断入力は防御的に無視する。
        Phase::Unloading { .. } | Phase::Stopped => {
            tracing::warn!(target: "kanade", event = "input_after_terminate", "終了系列で受領した入力を無視");
            (state, Vec::new())
        }
    }
}

/// 現フェーズが応答待ち（直前に GET/NOTIFY/unload を発行済み）かを判定する。
///
/// `Steady` も応答待ちに含める: Steady の Tick は `OnSecondChange`（GET/NOTIFY）を発行し、
/// その応答をなお `Steady` のまま受ける（in-flight ≤ 1・シェルが直後に `ShioriReply` を
/// 再投入する）。ここに Steady が無いと Steady の `ShioriReply` が防御アーム
/// （unexpected_reply）で握り潰され、OnSecondChange の Value/NoContent 処理が壊れる。
/// これにより Steady の Failed は `on_shiori_reply` 経由で `Unloading{Fault}` へ（Req 6.1・
/// pump 含む任意の SHIORI 失敗で M1 は放棄）、Value/NoContent/Notified は `dispatch_phase`
/// → `steady::step` へ正しく委譲される。
fn awaits_reply(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::BootInit
            | Phase::BootType
            | Phase::BootMain
            | Phase::BootVersion
            | Phase::Steady { .. }
            | Phase::ClosePending { .. }
    )
}

/// 現フェーズが突合対象とする active talk の talk_id（無ければ None）。
fn current_talk_id(phase: &Phase) -> Option<TalkId> {
    match phase {
        Phase::Steady {
            talk: Some(active), ..
        } => Some(active.talk_id),
        Phase::CloseTalkWait { talk_id, .. } => Some(*talk_id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::ShioriFailure;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    fn state_in(phase: Phase) -> State {
        State {
            phase,
            last_now: Some(MonotonicMs(1_000)),
            next_talk_id: 5,
            pending_close: None,
        }
    }

    fn steady_with_talk(talk_id: TalkId) -> Phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id,
                origin: "steady",
            }),
        }
    }

    // --- 1. TalkDone{reason:Quit} for a KNOWN talk → Unloading{Quit} + [ShioriUnload] ---

    #[test]
    fn known_quit_from_steady_goes_to_unloading_quit() {
        let phase = steady_with_talk(TalkId(5));
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Quit }));
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ShioriUnload));
    }

    #[test]
    fn known_quit_from_close_talk_wait_goes_to_unloading_quit() {
        let phase = Phase::CloseTalkWait {
            talk_id: TalkId(9),
            deadline: None,
        };
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(9),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Quit }));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // --- 1b. TalkDone{reason:Interrupted} for a KNOWN talk → 非 quit 扱い（Ended と同一経路） ---

    #[test]
    fn known_interrupted_from_steady_is_routed_as_non_quit() {
        // Interrupted は防御的に非 quit 扱い（Ended と同一経路）へ委譲される。
        // steady::on_talk_done は pending_close が無ければ Steady{None} へ復帰する。
        let phase = steady_with_talk(TalkId(5));
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Interrupted,
            }),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: None }),
            "Interrupted は Quit 系列へ進まず、Ended と同じ定常復帰へ"
        );
        assert!(actions.is_empty());
    }

    // --- 2. ForceQuit from any phase → [Notify OnClose, ShioriUnload] + Unloading{Forced} ---

    #[test]
    fn force_quit_emits_onclose_notify_then_unload() {
        let (next, actions) = step(
            state_in(Phase::BootMain),
            Input::ForceQuit {
                reason: CloseReason::System,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Forced }));
        assert_eq!(actions.len(), 2);
        match &actions[0] {
            Action::ShioriRequest(ShioriCall::Notify { id, references, .. }) => {
                assert_eq!(*id, "OnClose");
                assert_eq!(references, &vec!["system".to_string()]);
            }
            _ => panic!("expected first action to be Notify OnClose"),
        }
        assert!(matches!(actions[1], Action::ShioriUnload));
    }

    // --- 3. ShioriDown from any phase → Unloading{Fault} + [ShioriUnload] ---

    #[test]
    fn shiori_down_goes_to_unloading_fault() {
        let (next, actions) = step(
            state_in(steady_with_talk(TalkId(5))),
            Input::ShioriDown {
                reason: "helper crashed".to_string(),
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Fault }));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // --- 4. ShioriReply{Failed} from a waiting phase → Unloading{Fault} + [ShioriUnload] ---

    #[test]
    fn shiori_reply_failed_goes_to_unloading_fault() {
        let (next, actions) = step(
            state_in(Phase::BootType),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Fault }));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // --- 5. Unknown talk_id TalkDone → phase unchanged, no actions ---

    #[test]
    fn unknown_talk_id_keeps_phase_and_emits_nothing() {
        let phase = steady_with_talk(TalkId(5));
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(999),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        // 未知 talk_id は reason=Quit でも横断遷移させず、現 Phase を維持する。
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(5)),
            _ => panic!("expected Steady phase to be preserved"),
        }
        assert!(actions.is_empty());
    }

    // --- 6. Non-Idle Boot → phase unchanged, no actions ---

    #[test]
    fn boot_in_non_idle_phase_is_ignored() {
        let (next, actions) = step(state_in(Phase::BootMain), Input::Boot, &config());
        assert!(matches!(next.phase, Phase::BootMain));
        assert!(actions.is_empty());
    }

    // --- 7. Defensive: ShioriReply when phase does not await a reply → unchanged ---

    #[test]
    fn shiori_reply_when_not_waiting_is_ignored() {
        let (next, actions) = step(
            state_in(Phase::Idle),
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Idle));
        assert!(actions.is_empty());
    }

    // --- 8. Unloading + ShioriReply{Unloaded|Failed} → Stopped + [StopSelf] ---

    #[test]
    fn unloading_reply_unloaded_goes_to_stopped() {
        let (next, actions) = step(
            state_in(Phase::Unloading {
                cause: TermCause::Quit,
            }),
            Input::ShioriReply {
                outcome: ShioriOutcome::Unloaded,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Stopped));
        assert!(matches!(actions.as_slice(), [Action::StopSelf]));
    }

    #[test]
    fn unloading_reply_failed_still_goes_to_stopped() {
        let (next, actions) = step(
            state_in(Phase::Unloading {
                cause: TermCause::Forced,
            }),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Stopped));
        assert!(matches!(actions.as_slice(), [Action::StopSelf]));
    }

    // --- 9. State::initial() invariants ---

    #[test]
    fn initial_state_is_idle_with_monotonic_counter() {
        let s = State::initial();
        assert!(matches!(s.phase, Phase::Idle));
        assert_eq!(s.last_now, None);
        assert_eq!(s.next_talk_id, 1);
        assert!(s.pending_close.is_none());
    }
}

/// タスク 6.1: 純粋 step 層の失敗・防御アームがログを発火することの実行可能検証。
///
/// Req 6.3（ログ無しの失敗経路を持たない）・Req 6.1（区別語彙ごとにログ）を、コードレビュー
/// でなく**テスト**で担保する。各 `error!` / `warn!` アームを `step()`（または各サブモジュール
/// `step`）で駆動し、`log_capture` で捕捉したイベントに `target="kanade"`・所定の `event`・所定
/// レベル（ERROR/WARN）が存在することを表明する。ログが除去・語彙変更・レベル変更されると当該
/// テストが失敗する。
///
/// ルーティング上 `step()` からは到達不能な防御アーム（構造上発生しない系）は、当該サブモジュール
/// の `step` を直接駆動して検証する（各テストのコメントで明示）。これらは「あり得ない Phase/入力の
/// 組」に対する防御であり、直接駆動が唯一かつ正当な網羅手段である。
#[cfg(test)]
mod log_firing_tests {
    use super::log_capture::{assert_logged, capture, CapturedEvent};
    use super::*;
    use crate::msg::{CloseReason, ShioriFailure};
    use crate::talk::TalkDone;
    use tracing::Level;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    fn state_in(phase: Phase) -> State {
        State {
            phase,
            last_now: Some(MonotonicMs(1_000)),
            next_talk_id: 5,
            pending_close: None,
        }
    }

    fn steady_with_talk(talk_id: TalkId) -> Phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id,
                origin: "steady",
            }),
        }
    }

    /// `step()` を捕捉付きで駆動し、発行イベント列を返す（state は move）。
    fn run_step(phase: Phase, input: Input) -> Vec<CapturedEvent> {
        let cfg = config();
        capture(|| {
            let _ = step(state_in(phase), input, &cfg);
        })
    }

    // ============================================================
    // 失敗アーム（level = ERROR）
    // ============================================================

    #[test]
    fn error_shiori_down_logs() {
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::ShioriDown {
                reason: "helper crashed".to_string(),
            },
        );
        assert_logged(&ev, Level::ERROR, "shiori_down");
    }

    #[test]
    fn error_shiori_failed_logs() {
        // 応答待ちフェーズ（BootType）+ Failed → 横断アーム shiori_failed。
        let ev = run_step(
            Phase::BootType,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            },
        );
        assert_logged(&ev, Level::ERROR, "shiori_failed");
    }

    #[test]
    fn error_unknown_talk_done_logs() {
        // 突合対象 talk (5) と異なる talk_id (999) の TalkDone → unknown_talk_done。
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(999),
                reason: TalkEndReason::Ended,
            }),
        );
        assert_logged(&ev, Level::ERROR, "unknown_talk_done");
    }

    #[test]
    fn error_unload_failed_logs() {
        // Unloading 中の Failed 応答 → unload_failed（終了系列は継続）。
        let ev = run_step(
            Phase::Unloading {
                cause: TermCause::Quit,
            },
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
            },
        );
        assert_logged(&ev, Level::ERROR, "unload_failed");
    }

    #[test]
    fn error_close_deadline_exceeded_logs() {
        // CloseTalkWait で deadline 超過 Tick → close_deadline_exceeded。
        let cfg = config();
        let s = State {
            phase: Phase::CloseTalkWait {
                talk_id: TalkId(7),
                deadline: Some(MonotonicMs(1_000)),
            },
            last_now: Some(MonotonicMs(900)),
            next_talk_id: 8,
            pending_close: None,
        };
        let ev = capture(|| {
            let _ = step(s, Input::Tick { now: MonotonicMs(2_000) }, &cfg);
        });
        assert_logged(&ev, Level::ERROR, "close_deadline_exceeded");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— mod.rs
    // ============================================================

    #[test]
    fn warn_force_quit_logs() {
        let ev = run_step(
            Phase::BootMain,
            Input::ForceQuit {
                reason: CloseReason::System,
            },
        );
        assert_logged(&ev, Level::WARN, "force_quit");
    }

    #[test]
    fn warn_boot_ignored_logs() {
        // 非 Idle での Boot → mod.rs boot_ignored。
        let ev = run_step(Phase::BootMain, Input::Boot);
        assert_logged(&ev, Level::WARN, "boot_ignored");
    }

    #[test]
    fn warn_unexpected_reply_logs() {
        // 応答待ちでない Phase（Idle）への ShioriReply → mod.rs unexpected_reply。
        let ev = run_step(
            Phase::Idle,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
        );
        assert_logged(&ev, Level::WARN, "unexpected_reply");
    }

    #[test]
    fn warn_input_after_terminate_logs() {
        // 終了系列（Stopped）で受領した非横断入力（Tick）→ dispatch_phase input_after_terminate。
        let ev = run_step(Phase::Stopped, Input::Tick { now: MonotonicMs(1_000) });
        assert_logged(&ev, Level::WARN, "input_after_terminate");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— boot.rs
    // ============================================================

    #[test]
    fn warn_boot_input_ignored_logs() {
        // boot フェーズ（BootInit）+ boot 無関係入力（Tick）→ dispatch_phase→boot::step _ アーム。
        let ev = run_step(Phase::BootInit, Input::Tick { now: MonotonicMs(1_000) });
        assert_logged(&ev, Level::WARN, "boot_input_ignored");
    }

    #[test]
    fn warn_boot_unexpected_reply_logs() {
        // BootInit（Notified 待ち）に Value → boot::on_reply unexpected_reply。
        let ev = run_step(
            Phase::BootInit,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("unexpected".to_string()),
            },
        );
        assert_logged(&ev, Level::WARN, "boot_unexpected_reply");
    }

    #[test]
    fn warn_boot_reply_ignored_logs() {
        // boot::on_reply の防御 `_` アーム（応答待ちでない boot Phase）。step() 経由では
        // Idle が awaits_reply=false ゆえ mod.rs 側で握り潰され到達しない。構造上発生しない
        // 防御アームゆえ boot::step を直接駆動して検証する（唯一の網羅手段）。
        let cfg = config();
        let ev = capture(|| {
            let _ = boot::step(
                state_in(Phase::Idle),
                Input::ShioriReply {
                    outcome: ShioriOutcome::Notified,
                },
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "boot_reply_ignored");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— steady.rs
    // ============================================================

    #[test]
    fn warn_steady_value_during_talk_logs() {
        // Steady{Some} + Value（DD-6 防御）→ steady_value_during_talk。
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("late".to_string()),
            },
        );
        assert_logged(&ev, Level::WARN, "steady_value_during_talk");
    }

    #[test]
    fn warn_steady_unexpected_reply_logs() {
        // Steady{None} + Unloaded（想定外の応答）→ steady_reply_unexpected。
        let ev = run_step(
            Phase::Steady { talk: None },
            Input::ShioriReply {
                outcome: ShioriOutcome::Unloaded,
            },
        );
        assert_logged(&ev, Level::WARN, "steady_unexpected_reply");
    }

    #[test]
    fn warn_steady_input_ignored_logs() {
        // steady::step の `_` アーム（Steady に無関係な入力）。step() 経由では Boot は mod.rs、
        // ForceQuit/ShioriDown は横断アームで捌かれ steady へ届かない。構造上発生しない防御
        // アームゆえ steady::step を直接駆動して検証する。
        let cfg = config();
        let ev = capture(|| {
            let _ = steady::step(
                state_in(Phase::Steady { talk: None }),
                Input::Boot,
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "steady_input_ignored");
    }

    #[test]
    fn warn_steady_phase_unexpected_logs() {
        // steady::step に非 Steady Phase（BootMain）が届いた場合の防御。ルーティング上
        // 到達不能ゆえ steady::step を直接駆動して検証する。
        let cfg = config();
        let ev = capture(|| {
            let _ = steady::step(
                state_in(Phase::BootMain),
                Input::Tick { now: MonotonicMs(1_000) },
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "steady_phase_unexpected");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— close.rs
    // ============================================================

    #[test]
    fn warn_close_notified_unexpected_logs() {
        // ClosePending + Notified（OnClose は GET ゆえ構造上あり得ない）→ close_notified_unexpected。
        let ev = run_step(
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
        );
        assert_logged(&ev, Level::WARN, "close_notified_unexpected");
    }

    #[test]
    fn warn_close_reply_unexpected_logs() {
        // ClosePending + Unloaded（Value/NoContent/Notified 以外）→ close_reply_unexpected。
        let ev = run_step(
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Input::ShioriReply {
                outcome: ShioriOutcome::Unloaded,
            },
        );
        assert_logged(&ev, Level::WARN, "close_reply_unexpected");
    }

    #[test]
    fn warn_close_pending_input_ignored_logs() {
        // ClosePending + 無関係入力（CloseRequest）→ close_pending_input_ignored。
        let ev = run_step(
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Input::CloseRequest {
                reason: CloseReason::User,
            },
        );
        assert_logged(&ev, Level::WARN, "close_pending_input_ignored");
    }

    #[test]
    fn warn_close_talk_wait_input_ignored_logs() {
        // CloseTalkWait + 無関係入力（CloseRequest）→ close_talk_wait_input_ignored。
        let ev = run_step(
            Phase::CloseTalkWait {
                talk_id: TalkId(2),
                deadline: None,
            },
            Input::CloseRequest {
                reason: CloseReason::User,
            },
        );
        assert_logged(&ev, Level::WARN, "close_talk_wait_input_ignored");
    }

    #[test]
    fn warn_close_phase_unexpected_logs() {
        // close::step に非 close Phase（Steady）が届いた場合の防御アーム（上位 match `_`）。
        // ルーティング上到達不能ゆえ close::step を直接駆動して検証する。
        let cfg = config();
        let ev = capture(|| {
            let _ = close::step(
                state_in(Phase::Steady { talk: None }),
                Input::Tick { now: MonotonicMs(1_000) },
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "close_phase_unexpected");
    }

    // ============================================================
    // 観測用ログ（level = INFO）— TalkEndReason::Interrupted の防御的非 quit 扱い
    // ============================================================

    #[test]
    fn info_talk_done_interrupted_as_non_quit_logs() {
        // 既知 talk の reason=Interrupted → 防御的に非 quit 扱い（Ended と同一経路）へ委譲。
        // M1 では到達しない想定だが、到達した場合にどの reason だったか観測できることを検証する。
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Interrupted,
            }),
        );
        assert_logged(&ev, Level::INFO, "talk_done_interrupted_as_non_quit");
    }
}
