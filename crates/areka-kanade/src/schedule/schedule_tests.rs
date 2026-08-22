use super::log_capture::{assert_logged, assert_not_logged, capture};
use super::*;
use crate::msg::ShioriFailure;
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
        choice: None,
        choice_prev_talk: None,
    }
}

fn steady_with_talk(talk_id: TalkId) -> Phase {
    Phase::Steady {
        talk: Some(ActiveTalk {
            talk_id,
            origin: "steady",
            script: String::new(),
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
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));
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
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Quit
        }
    ));
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
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Forced
        }
    ));
    assert_eq!(actions.len(), 2);
    match &actions[0] {
        Action::ShioriRequest(ShioriCall::Notify { id, references, .. }) => {
            assert_eq!(id.as_str(), "OnClose");
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
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Fault
        }
    ));
    assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
}

// --- 4. ShioriReply{Failed} from a waiting phase → Unloading{Fault} + [ShioriUnload] ---

#[test]
fn shiori_reply_failed_goes_to_unloading_fault() {
    let (next, actions) = step(
        state_in(Phase::BootType),
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "test",
        },
        &config(),
    );
    assert!(matches!(
        next.phase,
        Phase::Unloading {
            cause: TermCause::Fault
        }
    ));
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
            origin: "test",
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
            origin: "test",
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
            origin: "test",
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

// --- 10. Mouse 横断ルーティング（Task 1・DD-IE-8） ---

use crate::msg::{MouseEventKind, MouseInput};

fn mouse_move() -> MouseInput {
    MouseInput {
        scope: 0,
        x: 10,
        y: 20,
        region: Some("head".to_string()),
        kind: MouseEventKind::Move,
    }
}

// 非 Steady フェーズ（boot／close／terminate 後）への Mouse は状態不変・SHIORI 問い合わせ
// 一切なし（DD-IE-8: マウスは Steady でのみ受理・他は安全に無視）。
#[test]
fn mouse_input_in_non_steady_phases_is_ignored() {
    for phase in [
        Phase::Idle,
        Phase::BootMain,
        Phase::ClosePending {
            reason: CloseReason::User,
        },
        Phase::Stopped,
    ] {
        let before = std::mem::discriminant(&phase);
        let (next, actions) = step(state_in(phase), Input::Mouse(mouse_move()), &config());
        assert_eq!(
            std::mem::discriminant(&next.phase),
            before,
            "非 Steady フェーズでは Mouse で phase が変わらない"
        );
        assert!(
            actions.is_empty(),
            "非 Steady フェーズでは Mouse で Action（SHIORI 問い合わせ含む）を発行しない"
        );
    }
}

/// 単一 Action が期待 GET（id・references・status）と厳密一致することを検証する。
/// ShioriCall は PartialEq を持たないため field 単位で比較する。
fn assert_get(action: &Action, expected: &ShioriCall) {
    match (action, expected) {
        (
            Action::ShioriRequest(ShioriCall::Get {
                id,
                references,
                status,
            }),
            ShioriCall::Get {
                id: eid,
                references: erefs,
                status: estatus,
            },
        ) => {
            assert_eq!(id, eid, "GET id 不一致");
            assert_eq!(references, erefs, "GET references 不一致");
            assert_eq!(status.render(), estatus.render(), "GET status 不一致");
        }
        _ => panic!("期待した GET でない Action"),
    }
}

// Steady では横断アームが steady::on_mouse へ委譲し、Task 2.2 の実装で GET を発行する
// （seam の意図的充填）。step() 経由（横断アーム込み）で GET が 1 件出ることを固定する。
#[test]
fn mouse_input_in_steady_emits_get_via_crosscutting_arm() {
    // Steady{None} + Move → OnMouseMove GET（Status 行なし・INACTIVE）。phase 不変。
    let (next, actions) = step(
        state_in(Phase::Steady { talk: None }),
        Input::Mouse(mouse_move()),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "マウス GET は phase を変えない"
    );
    assert_eq!(
        actions.len(),
        1,
        "マウス入力で GET を 1 件発行する（Task 2.2）"
    );
    assert_get(
        &actions[0],
        &events::on_mouse_move(10, 20, 0, Some("head"), &ExecutionSnapshot::INACTIVE),
    );

    // Steady{Some} + Move → GET のまま発行され Status: talking を帯びる（DD-IE-1）。phase 不変。
    let (next, actions) = step(
        state_in(steady_with_talk(TalkId(5))),
        Input::Mouse(mouse_move()),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "active talk は維持される"
    );
    assert_eq!(actions.len(), 1);
    assert_get(
        &actions[0],
        &events::on_mouse_move(
            10,
            20,
            0,
            Some("head"),
            &ExecutionSnapshot {
                talk_active: true,
                choice_active: false,
            },
        ),
    );
}

// close 保留中は Steady であってもマウス GET を発行しない防御（構造的シーム・DD-IE-8）。
#[test]
fn mouse_input_in_steady_with_pending_close_emits_no_get() {
    let mut s = state_in(Phase::Steady { talk: None });
    s.pending_close = Some(CloseReason::System);
    let (next, actions) = step(s, Input::Mouse(mouse_move()), &config());
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert!(
        matches!(next.pending_close, Some(CloseReason::System)),
        "guard は pending_close を消費しない"
    );
    assert!(actions.is_empty(), "close 保留中はマウス GET を発行しない");
}

// ============================================================
// 11. 選択系の additive 追加（タスク 4.1・Req4.4・DD-3／DD-10）
// ============================================================

/// 檻用の選択確定入力（内容は本檻で load-bearing でない＝写像・帳簿の存在のみを見る）。
fn choice_input() -> ChoiceInput {
    ChoiceInput {
        id: "OnMenu".to_string(),
        label: "メニュー".to_string(),
        scope: 0,
        references: vec!["a0".to_string()],
    }
}

/// 檻用の選択待ち通知入力（同上）。
fn choice_waiting_input() -> Input {
    Input::ChoiceWaiting {
        talk_id: TalkId(5),
        choice_ids: vec!["OnMenu".to_string()],
        display_end: MonotonicMs(2_000),
        timeout_directive_secs: None,
    }
}

/// Req4.4: 既存 `Phase` の 11 variant が無改変であること。
///
/// 本 match は wildcard を持たないため、variant の削除・改名・形（フィールド構成）の
/// 変更はコンパイルを壊す。DD-3 が要求する「Phase を一切触らない」を構造で固定する
/// （`State.choice` は Phase の外＝`pending_close` と同型に置かれる）。
#[test]
fn existing_phase_variants_are_unchanged() {
    fn tag(phase: &Phase) -> &'static str {
        match phase {
            Phase::Idle => "Idle",
            Phase::BootInit => "BootInit",
            Phase::BootPrefetch => "BootPrefetch",
            Phase::BootType => "BootType",
            Phase::BootMain => "BootMain",
            Phase::BootVersion { .. } => "BootVersion",
            Phase::Steady { .. } => "Steady",
            Phase::ClosePending { .. } => "ClosePending",
            Phase::CloseTalkWait { .. } => "CloseTalkWait",
            Phase::Unloading { .. } => "Unloading",
            Phase::Stopped => "Stopped",
        }
    }
    assert_eq!(tag(&Phase::Idle), "Idle");
    assert_eq!(tag(&Phase::Steady { talk: None }), "Steady");
    assert_eq!(tag(&Phase::Stopped), "Stopped");
}

/// Req4.4: 既存 `Action` 5 variant が無改変で、選択系 2 variant が additive に増えたこと。
///
/// wildcard なしの網羅 match ゆえ、既存 5 variant のいずれかが消える／改名される／
/// 形が変わると本檻はコンパイルできない。
#[test]
fn action_variants_are_existing_five_plus_choice_two() {
    fn tag(action: &Action) -> &'static str {
        match action {
            Action::ShioriRequest(_) => "ShioriRequest",
            Action::ShioriUnload => "ShioriUnload",
            Action::StartTalk(_) => "StartTalk",
            Action::ResourceOutcome { .. } => "ResourceOutcome",
            Action::StopSelf => "StopSelf",
            Action::ResolveChoice { .. } => "ResolveChoice",
            Action::CancelChoice { .. } => "CancelChoice",
        }
    }
    assert_eq!(tag(&Action::ShioriUnload), "ShioriUnload");
    assert_eq!(tag(&Action::StopSelf), "StopSelf");
    assert_eq!(
        tag(&Action::ResolveChoice {
            talk_id: TalkId(5),
            id: "OnMenu".to_string(),
        }),
        "ResolveChoice"
    );
    assert_eq!(
        tag(&Action::CancelChoice { talk_id: TalkId(5) }),
        "CancelChoice"
    );
}

/// Req4.4: 既存 `Input` 8 variant が無改変で、選択系 2 variant が additive に増えたこと。
#[test]
fn input_variants_are_existing_eight_plus_choice_two() {
    fn tag(input: &Input) -> &'static str {
        match input {
            Input::Boot => "Boot",
            Input::Tick { .. } => "Tick",
            Input::TalkDone(_) => "TalkDone",
            Input::CloseRequest { .. } => "CloseRequest",
            Input::ForceQuit { .. } => "ForceQuit",
            Input::ShioriDown { .. } => "ShioriDown",
            Input::Mouse(_) => "Mouse",
            Input::ShioriReply { .. } => "ShioriReply",
            Input::Choice(_) => "Choice",
            Input::ChoiceWaiting { .. } => "ChoiceWaiting",
        }
    }
    assert_eq!(tag(&Input::Boot), "Boot");
    assert_eq!(tag(&Input::Mouse(mouse_move())), "Mouse");
    assert_eq!(tag(&Input::Choice(choice_input())), "Choice");
    assert_eq!(tag(&choice_waiting_input()), "ChoiceWaiting");
}

/// DD-3: 選択帳簿は `State`（Phase 外）に置かれ、初期値は両方とも空である。
#[test]
fn initial_state_has_empty_choice_ledger() {
    let s = State::initial();
    assert!(s.choice.is_none(), "初期状態に選択待ち帳簿は無い");
    assert!(
        s.choice_prev_talk.is_none(),
        "初期状態に 1 世代保持の旧 talk_id は無い"
    );
}

/// C4 規則 1・Req1.3: 非 Steady フェーズの選択確定は棄却する（状態不変・Action なし）。
///
/// 受理すべき選択待ちが構造上存在しないフェーズ（boot 中・close 握手以降・終了系列）へ
/// 届いた選択確定は、帳簿の有無に関わらず横断アームで棄却される。棄却は既存帳簿にも
/// 触れない（棄却の定義＝状態不変）。
#[test]
fn choice_input_in_non_steady_phases_is_rejected_without_changing_state() {
    for phase in [
        Phase::Idle,
        Phase::BootMain,
        Phase::BootVersion {
            talk: Some(ActiveTalk {
                talk_id: TalkId(5),
                origin: "boot",
                script: String::new(),
            }),
        },
        Phase::ClosePending {
            reason: CloseReason::User,
        },
        Phase::Stopped,
    ] {
        let before = std::mem::discriminant(&phase);
        let mut s = state_in(phase);
        s.choice = Some(ChoiceState {
            talk_id: TalkId(5),
            candidates: vec!["OnMenu".to_string()],
            deadline: None,
            phase: ChoicePhase::Waiting,
        });
        let (next, actions) = step(s, Input::Choice(choice_input()), &config());
        assert_eq!(
            std::mem::discriminant(&next.phase),
            before,
            "棄却は phase を変えない"
        );
        let ledger = next.choice.expect("棄却は既存帳簿にも触れない");
        assert!(matches!(ledger.phase, ChoicePhase::Waiting));
        assert!(actions.is_empty(), "棄却は Action を発行しない");
    }
}

// ============================================================
// 12. 選択待ち通知の受領と帳簿確立（タスク 4.2・Req7.1／7.6／7.7・C4 規則 4・DD-8）
// ============================================================

/// 檻用の選択待ち通知（候補列・起点・指令を明示して組む）。
fn choice_waiting_of(
    talk_id: TalkId,
    choice_ids: &[&str],
    display_end: MonotonicMs,
    timeout_directive_secs: Option<f64>,
) -> Input {
    Input::ChoiceWaiting {
        talk_id,
        choice_ids: choice_ids.iter().map(|s| s.to_string()).collect(),
        display_end,
        timeout_directive_secs,
    }
}

/// C4 規則 4・Req7.1: 現行トークと識別子が一致する通知は帳簿を確立する。
///
/// 確立された帳簿は通知の候補列を表示順のまま保持し、期限は DD-8 写像済みの値
/// （未指定→起点＋既定 30_000ms）を持ち、段フェーズは `Waiting` である。`Phase` は
/// 一切触らない（DD-3）。
#[test]
fn choice_waiting_matching_talk_id_establishes_waiting_ledger() {
    let (next, actions) = step(
        state_in(steady_with_talk(TalkId(5))),
        choice_waiting_of(TalkId(5), &["OnMenu", "choice1"], MonotonicMs(2_000), None),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "帳簿確立は Phase を触らない（DD-3）"
    );
    let ledger = next.choice.expect("一致通知は帳簿を確立する");
    assert_eq!(ledger.talk_id, TalkId(5));
    assert_eq!(
        ledger.candidates,
        vec!["OnMenu".to_string(), "choice1".to_string()],
        "候補列は通知どおり（表示順を保存・DD-7）"
    );
    assert_eq!(
        ledger.deadline,
        Some(MonotonicMs(32_000)),
        "未指定は既定 30_000ms を起点へ加算（DD-8）"
    );
    assert!(matches!(ledger.phase, ChoicePhase::Waiting));
    assert!(actions.is_empty(), "帳簿確立は Action を発行しない");
}

/// Req7.1: 通知受領後の帳簿が「選択確定を受理できる」前提を満たす。
///
/// 受理判定そのものはタスク 4.3 の担当であり、本檻はその判定が読む 3 条件——段フェーズが
/// `Waiting`・候補列が照合可能な形で保持されている・帳簿の `talk_id` が**現行 talk と一致**
/// （`ChoiceState` の不変条件）——を状態で固定する。
#[test]
fn established_ledger_satisfies_preconditions_for_choice_acceptance() {
    let (next, _) = step(
        state_in(steady_with_talk(TalkId(5))),
        choice_waiting_of(TalkId(5), &["OnMenu", "choice1"], MonotonicMs(2_000), None),
        &config(),
    );
    let active_talk_id = current_talk_id(&next.phase).expect("active talk は維持される");
    let ledger = next.choice.expect("一致通知は帳簿を確立する");
    assert!(
        matches!(ledger.phase, ChoicePhase::Waiting),
        "受理可能な段フェーズ（入力待ち）である"
    );
    assert_eq!(
        ledger.talk_id, active_talk_id,
        "帳簿の talk_id は現行 talk と一致する（ChoiceState の不変条件）"
    );
    assert!(
        ledger.candidates.iter().any(|c| c == "choice1"),
        "候補 ID は照合可能な形で保持される（4.3 の候補集合照合の前提）"
    );
}

/// DD-8・Req7.6／7.7: 期限は指令 3 値語彙どおりに写る（実値で突合）。
///
/// 既定値以外の明示秒指定も同一規律で写る（Req7.7）。無効化（0／-1）は期限なし＝無期限
/// であり、帳簿自体は確立される（Req7.6：計測を開始しないだけで選択待ちは継続する）。
#[test]
fn choice_waiting_deadline_follows_dd8_directive_mapping() {
    let display_end = MonotonicMs(2_000);
    let cases: [(Option<f64>, Option<MonotonicMs>); 5] = [
        // 未指定＝既定へ委譲（config.choice_timeout_default_ms = 30_000）。
        (None, Some(MonotonicMs(32_000))),
        // 無効化＝無期限（Req7.6）。
        (Some(0.0), None),
        (Some(-1.0), None),
        // 明示秒指定（Req7.7・既定値は関与しない）。
        (Some(5.0), Some(MonotonicMs(7_000))),
        (Some(2.5), Some(MonotonicMs(4_500))),
    ];
    for (directive, expected) in cases {
        let (next, _) = step(
            state_in(steady_with_talk(TalkId(5))),
            choice_waiting_of(TalkId(5), &["OnMenu"], display_end, directive),
            &config(),
        );
        let ledger = next
            .choice
            .unwrap_or_else(|| panic!("指令 {directive:?} でも帳簿は確立される"));
        assert_eq!(
            ledger.deadline, expected,
            "指令 {directive:?} の期限写像が DD-8 と一致しない"
        );
    }
}

/// C4 規則 4・Req7.1: 識別子が現行トークと一致しない通知は帳簿を確立しない。
#[test]
fn choice_waiting_with_mismatched_talk_id_does_not_establish_ledger() {
    let (next, actions) = step(
        state_in(steady_with_talk(TalkId(5))),
        choice_waiting_of(TalkId(999), &["OnMenu"], MonotonicMs(2_000), None),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "棄却は Phase を触らない"
    );
    assert!(next.choice.is_none(), "不一致通知は帳簿を確立しない");
    assert!(actions.is_empty(), "棄却は Action を発行しない");
}

/// C4 規則 4: 再生中でない Steady（`Steady{None}`）の通知は受理しない。
#[test]
fn choice_waiting_without_active_talk_does_not_establish_ledger() {
    let (next, actions) = step(
        state_in(Phase::Steady { talk: None }),
        choice_waiting_of(TalkId(5), &["OnMenu"], MonotonicMs(2_000), None),
        &config(),
    );
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert!(
        next.choice.is_none(),
        "active talk 不在では帳簿を確立しない"
    );
    assert!(actions.is_empty());
}

/// C4 規則 4: 非 Steady フェーズの通知は受理しない（挨拶追跡中の `BootVersion{Some}` も含む）。
#[test]
fn choice_waiting_in_non_steady_phase_does_not_establish_ledger() {
    for phase in [
        Phase::Idle,
        Phase::BootMain,
        Phase::BootVersion {
            talk: Some(ActiveTalk {
                talk_id: TalkId(5),
                origin: "boot",
                script: String::new(),
            }),
        },
        Phase::ClosePending {
            reason: CloseReason::User,
        },
    ] {
        let before = std::mem::discriminant(&phase);
        let (next, actions) = step(
            state_in(phase),
            choice_waiting_of(TalkId(5), &["OnMenu"], MonotonicMs(2_000), None),
            &config(),
        );
        assert_eq!(
            std::mem::discriminant(&next.phase),
            before,
            "棄却は phase を変えない"
        );
        assert!(next.choice.is_none(), "非 Steady では帳簿を確立しない");
        assert!(actions.is_empty());
    }
}

/// C4 規則 4: 棄却は**既存の帳簿にも触れない**（状態不変が棄却の定義）。
#[test]
fn stale_choice_waiting_leaves_existing_ledger_untouched() {
    let mut s = state_in(steady_with_talk(TalkId(5)));
    s.choice = Some(ChoiceState {
        talk_id: TalkId(5),
        candidates: vec!["existing".to_string()],
        deadline: Some(MonotonicMs(9_000)),
        phase: ChoicePhase::Waiting,
    });
    let (next, actions) = step(
        s,
        choice_waiting_of(TalkId(999), &["OnMenu"], MonotonicMs(2_000), None),
        &config(),
    );
    let ledger = next.choice.expect("既存帳簿は棄却で消えない");
    assert_eq!(ledger.talk_id, TalkId(5));
    assert_eq!(ledger.candidates, vec!["existing".to_string()]);
    assert_eq!(ledger.deadline, Some(MonotonicMs(9_000)));
    assert!(actions.is_empty());
}

// ============================================================
// 13. 選択待ち中の実行状態導出（タスク 4.4・Req6.1／6.2／6.4・C5・裁定 6）
// ============================================================

/// C5: `choice_active` の源は `State.choice` の**3 段フェーズすべて**である。
///
/// `Cascading`／`TimeoutInFlight` は SHIORI 応答待ちであって選択待ちの終了ではない——
/// 段が進んでいる間も選択肢は表示されたままであり、`choosing` はアクティブであり続ける。
/// 併せて裁定 6／Req6.4（選択待ち中も talk slot 占有は継続＝`talking` 真）を固定する。
#[test]
fn state_snapshot_marks_choice_active_in_every_ledger_phase() {
    for phase in [
        ChoicePhase::Waiting,
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: Some(CascadeNext::Select),
        },
        ChoicePhase::TimeoutInFlight,
    ] {
        let label = match &phase {
            ChoicePhase::Waiting => "Waiting",
            ChoicePhase::Cascading { .. } => "Cascading",
            ChoicePhase::TimeoutInFlight => "TimeoutInFlight",
        };
        let mut s = state_in(steady_with_talk(TalkId(5)));
        s.choice = Some(ChoiceState {
            talk_id: TalkId(5),
            candidates: vec!["OnMenu".to_string()],
            deadline: None,
            phase,
        });
        let snapshot = s.snapshot();
        assert!(
            snapshot.choice_active,
            "{label}: 3 段フェーズのいずれでも choosing はアクティブ（C5）"
        );
        assert!(
            snapshot.talk_active,
            "{label}: 選択待ち中も talk slot 占有は継続する（Req6.4・裁定 6）"
        );
    }
}

/// Req6.2: 帳簿が消えた（解決・タイムアウト終了）状態では `choosing` は非アクティブへ戻る。
#[test]
fn state_snapshot_drops_choice_active_when_ledger_is_gone() {
    let s = state_in(steady_with_talk(TalkId(5)));
    let snapshot = s.snapshot();
    assert!(snapshot.talk_active, "再生自体は継続している");
    assert!(
        !snapshot.choice_active,
        "帳簿不在なら choosing は非アクティブ（Req6.2）"
    );
}

/// DD-IT-3: 供給側の署名を `State` 全体へ広げても **talk 軸は `snapshot_of(&Phase)` と同一**
/// である（choice 軸の増設が既存の talk 導出を汚さない）。
#[test]
fn state_snapshot_preserves_the_talk_axis_of_phase() {
    for phase in [
        Phase::Idle,
        Phase::BootMain,
        Phase::BootVersion { talk: None },
        Phase::BootVersion {
            talk: Some(ActiveTalk {
                talk_id: TalkId(5),
                origin: "boot",
                script: String::new(),
            }),
        },
        Phase::Steady { talk: None },
        steady_with_talk(TalkId(5)),
        Phase::Stopped,
    ] {
        let expected = snapshot_of(&phase).talk_active;
        let s = state_in(phase);
        let snapshot = s.snapshot();
        assert_eq!(
            snapshot.talk_active, expected,
            "talk 軸は Phase 由来のまま（DD-IT-3）"
        );
        assert!(
            !snapshot.choice_active,
            "帳簿なしでは choosing は非アクティブ"
        );
    }
}

// ============================================================
// 1 世代 stale 防御（タスク 4.6・C4 規則 9・F1 残余レース・Req1.6）
// ============================================================
//
// choice 起因の slot 差替直後は、旧 talk の即時 `Done{Ended}`（drive.rs の即 settle）が
// dispatcher の slot 差替より前に投函され得る（design F1「順序の決定性と残余レース」）。
// 到着した遅延 `TalkDone` は `choice_prev_talk` と照合して `talk_done_stale_choice`（info）
// で棄却し、`unknown_talk_done`（error）を**真に未知の id 専用**に保つ。

/// `choice_prev_talk` を仕込んだ `Steady{Some(active)}` を組む。
fn state_with_prev_talk(active: TalkId, prev: TalkId) -> State {
    let mut s = state_in(steady_with_talk(active));
    s.choice_prev_talk = Some(prev);
    s
}

/// 規則 9: 1 世代保持した旧 talk_id の遅延 `TalkDone` は info で棄却し、状態を壊さない。
#[test]
fn stale_choice_talk_done_is_demoted_to_info_and_keeps_state() {
    let cfg = config();
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(
            state_with_prev_talk(TalkId(9), TalkId(3)),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &cfg,
        ));
    });
    let (next, actions) = out.expect("step は必ず結果を返す");
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(9), "遅延 Done は現行 slot を壊さない"),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert!(actions.is_empty(), "遅延 Done は Action を発行しない");
    assert_logged(&ev, Level::INFO, "talk_done_stale_choice");
    assert_not_logged(&ev, "unknown_talk_done");
    assert_eq!(
        next.choice_prev_talk,
        Some(TalkId(3)),
        "stale 帳簿は現 talk の TalkDone 到達まで保持する（1 世代・規則 9）"
    );
}

/// 規則 9: `unknown_talk_done`（error）は**真に未知の id 専用**のまま保つ。
#[test]
fn truly_unknown_talk_done_still_logs_error() {
    let cfg = config();
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(
            state_with_prev_talk(TalkId(9), TalkId(3)),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(777),
                reason: TalkEndReason::Ended,
            }),
            &cfg,
        ));
    });
    let (next, actions) = out.expect("step は必ず結果を返す");
    assert_logged(&ev, Level::ERROR, "unknown_talk_done");
    assert_not_logged(&ev, "talk_done_stale_choice");
    assert!(actions.is_empty());
    assert!(matches!(next.phase, Phase::Steady { talk: Some(_) }));
}

/// 規則 9: 現 talk の `TalkDone` 到達で 1 世代保持は消え、以後の同 id は真に未知へ戻る。
#[test]
fn current_talk_done_clears_the_one_generation_stale_slot() {
    let cfg = config();
    let (after_current, _) = step(
        state_with_prev_talk(TalkId(9), TalkId(3)),
        Input::TalkDone(TalkDone {
            talk_id: TalkId(9),
            reason: TalkEndReason::Ended,
        }),
        &cfg,
    );
    assert!(
        after_current.choice_prev_talk.is_none(),
        "現 talk の TalkDone 到達で 1 世代保持を消去する（規則 9）"
    );
    // 消去後に届く旧 id は真に未知＝error へ戻る（保持は 1 世代のみ）。
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(
            after_current,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &cfg,
        ));
    });
    let _ = out.expect("step は必ず結果を返す");
    assert_logged(&ev, Level::ERROR, "unknown_talk_done");
}
