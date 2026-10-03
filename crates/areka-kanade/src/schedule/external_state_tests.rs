// =============================================================================
// 外から届いた実行状態の写し（`Input::ExecutionState`）の決定論テスト
// =============================================================================
//
// 入口は `schedule::step` の横断の腕で、殻（`actor.rs`）が `KanadeMsg::ExecutionState` をそのまま
// 写して渡す。ここでは `step` を直に呼び、写しが変わることと行動を返さないことを見る。

use super::{ActiveTalk, Input, Phase, State, TermCause, step};
use crate::msg::{CloseReason, KanadeConfig};
use crate::status::{BalloonBinding, ExecutionStateUpdate, ExternalStates};
use crate::talk::TalkId;

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

/// 起動前・定常（トーク中）・終了の握手・切替・終了中・停止後の各相（相を問わないことを見る）。
fn phases() -> Vec<(&'static str, Phase)> {
    vec![
        ("Idle", Phase::Idle),
        (
            "Steady{Some}",
            Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(3),
                    origin: "OnTest",
                    script: r"\0こんにちは\e".to_string(),
                }),
            },
        ),
        (
            "ClosePending",
            Phase::ClosePending {
                reason: CloseReason::System,
            },
        ),
        ("ChangePending", Phase::ChangePending),
        (
            "Unloading",
            Phase::Unloading {
                cause: TermCause::Quit,
            },
        ),
        ("Stopped", Phase::Stopped),
    ]
}

/// 要件 3.1・3.2・4.5・4.6・5.1: 知らせはどの相でも写しを変え、行動を返さず、相も変えない。
#[test]
fn execution_state_updates_the_copy_in_every_phase_without_actions() {
    for (label, phase) in phases() {
        let state = State {
            phase,
            ..State::initial()
        };

        let (state, actions) = step(
            state,
            Input::ExecutionState(ExecutionStateUpdate::NoUserBreak(true)),
            &config(),
        );
        assert!(actions.is_empty(), "{label}: 行動を返さない");
        assert!(state.external.no_user_break, "{label}: 旗の写しが立つ");

        let (state, actions) = step(
            state,
            Input::ExecutionState(ExecutionStateUpdate::Balloons(vec![
                binding(0, 2),
                binding(1, 0),
            ])),
            &config(),
        );
        assert!(actions.is_empty(), "{label}: 行動を返さない");
        assert_eq!(
            state.external,
            ExternalStates {
                no_user_break: true,
                online: false,
                balloons: vec![binding(0, 2), binding(1, 0)],
            },
            "{label}: バルーンの組が写る"
        );

        // 下ろす向き（旗 false・空の組）も写る。
        let (state, _) = step(
            state,
            Input::ExecutionState(ExecutionStateUpdate::NoUserBreak(false)),
            &config(),
        );
        let (state, actions) = step(
            state,
            Input::ExecutionState(ExecutionStateUpdate::Balloons(Vec::new())),
            &config(),
        );
        assert!(actions.is_empty(), "{label}: 行動を返さない");
        assert_eq!(
            state.external,
            ExternalStates::default(),
            "{label}: 下ろした値も写る"
        );
        assert_eq!(
            super::phase_label(&state.phase),
            label.split('{').next().unwrap(),
            "{label}: 相は変わらない"
        );
    }
}

/// リクエストの `Status` 行の値（無ければ `None`）。
fn status_of(action: &super::Action) -> Option<String> {
    match action {
        super::Action::ShioriRequest(
            crate::msg::ShioriCall::Get { status, .. }
            | crate::msg::ShioriCall::Notify { status, .. },
        ) => status.render(),
        _ => panic!("リクエストのはず"),
    }
}

/// 要件 3.3: 旗の写しが真でも、トークが無ければ nouserbreak は載らず、トーク中なら載る。
/// online と balloons は写しのまま載る（talking・choosing の計算は変えない）。
#[test]
fn snapshot_gates_nouserbreak_by_talk_and_copies_online_and_balloons() {
    let external = ExternalStates {
        no_user_break: true,
        online: true,
        balloons: vec![binding(0, 2)],
    };
    let (_, talking) = phases().swap_remove(1);
    let with_talk = State {
        phase: talking,
        external: external.clone(),
        ..State::initial()
    }
    .snapshot();
    assert!(with_talk.talk_active && with_talk.no_user_break);
    assert!(with_talk.online && with_talk.balloons == vec![binding(0, 2)]);

    let without_talk = State {
        phase: Phase::Steady { talk: None },
        external,
        ..State::initial()
    }
    .snapshot();
    assert!(!without_talk.talk_active && !without_talk.no_user_break);
    assert!(without_talk.online && without_talk.balloons == vec![binding(0, 2)]);
}

/// 要件 5.1: 起動の最初の段（OnInitialize）も写しの online と balloon を載せる。
#[test]
fn boot_request_carries_online_and_balloons_from_the_copy() {
    let state = State {
        external: ExternalStates {
            no_user_break: true,
            online: true,
            balloons: vec![binding(0, 2)],
        },
        ..State::initial()
    };
    let (_, actions) = step(state, Input::Boot, &config());
    assert_eq!(
        status_of(&actions[0]).as_deref(),
        Some("online,balloon(0=2)"),
        "会話なしの作り方は旗を落とし online と balloon だけを残す"
    );
}
