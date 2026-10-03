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

// -----------------------------------------------------------------------------
// 会話なしの作り方と、それを使う各リクエスト（要件 3.3・5.1・8.1）
// -----------------------------------------------------------------------------

/// 旗・online・バルーンがすべて立った写し（旗は会話なしの作り方では落ちるはず）。
fn full_copy() -> ExternalStates {
    ExternalStates {
        no_user_break: true,
        online: true,
        balloons: vec![binding(0, 2), binding(1, 0)],
    }
}

/// 会話なしの作り方が残す `Status` の値（online とバルーンだけ）。
const WITHOUT_TALK: &str = "online,balloon(0=2/1=0)";

/// リクエスト 1 件の (イベント名, `Status` 行の値)。
fn request_of(action: &super::Action) -> (String, Option<String>) {
    match action {
        super::Action::ShioriRequest(
            crate::msg::ShioriCall::Get { id, status, .. }
            | crate::msg::ShioriCall::Notify { id, status, .. },
        ) => (id.as_str().to_string(), status.render()),
        _ => panic!("リクエストのはず"),
    }
}

/// 行動列の最後のリクエストの (イベント名, `Status` 行の値)。
fn last_request(actions: &[super::Action]) -> (String, Option<String>) {
    let action = actions
        .iter()
        .rev()
        .find(|a| matches!(a, super::Action::ShioriRequest(_)))
        .expect("リクエストが 1 件はあるはず");
    request_of(action)
}

/// 要件 3.3・5.1: 会話なしの作り方は、トーク中の相から作っても talk・choice・
/// nouserbreak を落とし、online とバルーンだけを写しから残す。
#[test]
fn snapshot_without_talk_keeps_only_online_and_balloons() {
    let (_, talking) = phases().swap_remove(1);
    let state = State {
        phase: talking,
        external: full_copy(),
        ..State::initial()
    };
    let snapshot = state.snapshot_without_talk();
    assert!(!snapshot.talk_active && !snapshot.choice_active && !snapshot.no_user_break);
    assert!(snapshot.online);
    assert_eq!(snapshot.balloons, vec![binding(0, 2), binding(1, 0)]);
    assert_eq!(
        crate::status::ExecutionStatus::derive(&snapshot)
            .render()
            .as_deref(),
        Some(WITHOUT_TALK)
    );
}

/// 写しを満たした `state` から `inputs` を順に流し、各入力が返した最後のリクエストを集める
/// （翻訳の行動が出たら、`OnTranslate` が 204 を返したときの続きのリクエストを集める）。
fn walk(state: State, inputs: Vec<Input>, config: &KanadeConfig) -> Vec<(String, Option<String>)> {
    let mut state = State {
        external: full_copy(),
        ..state
    };
    let mut seen = Vec::new();
    for input in inputs {
        let next = step(state, input, config);
        let (next, actions) = crate::schedule::translate_test_support::pass_translate(next, config);
        seen.push(last_request(&actions));
        state = next;
    }
    seen
}

fn reply(outcome: crate::msg::ShioriOutcome) -> Input {
    Input::ShioriReply {
        outcome,
        origin: "test",
    }
}

fn expected(rows: &[(&str, &str)]) -> Vec<(String, Option<String>)> {
    rows.iter()
        .map(|(id, status)| (id.to_string(), Some(status.to_string())))
        .collect()
}

/// 要件 5.1: 起動の各段（OnInitialize・username の照会・起動の根 3 種（OnFirstBoot・OnGhostChanged・
/// ネットワーク更新の知らせ）・OnBoot の 2 経路・基盤バージョン）
/// のリクエストに写しの online とバルーンが載る。基盤バージョンだけは送出時点の作り方なので、
/// 挨拶のトーク中ならトークと旗も載る（要件 3.3）。
#[test]
fn every_boot_stage_request_carries_online_and_balloons_from_the_copy() {
    use crate::msg::ShioriOutcome::{NoContent, Notified, Value};

    // 経路 A（初回起動）: 根 OnFirstBoot が 204 → OnBoot（根の 204 から）→ 204 → 基盤バージョン。
    let first_boot = walk(
        State::initial(),
        vec![
            Input::Boot,
            reply(Notified),
            reply(Value("名無し".to_string())),
            reply(NoContent),
            reply(NoContent),
        ],
        &config(),
    );
    assert_eq!(
        first_boot,
        expected(&[
            ("OnInitialize", WITHOUT_TALK),
            ("username", WITHOUT_TALK),
            ("OnFirstBoot", WITHOUT_TALK),
            ("OnBoot", WITHOUT_TALK),
            ("basewareversion", WITHOUT_TALK),
        ])
    );

    // 経路 B（根なし）: 照会の応答から OnBoot へ直行 → 台本 → 挨拶のトーク中の基盤バージョン。
    let mut plain = config();
    plain.first_boot = false;
    let no_root = walk(
        State::initial(),
        vec![
            Input::Boot,
            reply(Notified),
            reply(NoContent),
            reply(Value(r"\0やあ\e".to_string())),
        ],
        &plain,
    );
    assert_eq!(
        no_root,
        expected(&[
            ("OnInitialize", WITHOUT_TALK),
            ("username", WITHOUT_TALK),
            ("OnBoot", WITHOUT_TALK),
            (
                "basewareversion",
                "talking,nouserbreak,online,balloon(0=2/1=0)"
            ),
        ])
    );

    // 経路 C（切替で来た）: 根 OnGhostChanged。
    let mut changed = config();
    changed.first_boot = false;
    changed.boot_origin = crate::change::BootOrigin::ChangedFrom(crate::change::ChangedFrom {
        sakura_name: "かなで".to_string(),
        script: r"\0じゃあね\e".to_string(),
        name: "Kanade Ghost".to_string(),
        dir: r"C:\areka\ghost\kanade".to_string(),
    });
    let changed_root = walk(
        State::initial(),
        vec![Input::Boot, reply(Notified), reply(NoContent)],
        &changed,
    );
    assert_eq!(
        changed_root,
        expected(&[
            ("OnInitialize", WITHOUT_TALK),
            ("username", WITHOUT_TALK),
            ("OnGhostChanged", WITHOUT_TALK),
        ])
    );

    // 経路 D（ネットワーク更新で読み直した）: 根は渡された名前と Reference の GET。
    let mut updated = config();
    updated.first_boot = false;
    updated.boot_origin = crate::change::BootOrigin::Updated {
        id: "OnUpdateComplete",
        references: vec!["changed".to_string()],
    };
    let updated_root = walk(
        State::initial(),
        vec![Input::Boot, reply(Notified), reply(NoContent)],
        &updated,
    );
    assert_eq!(
        updated_root,
        expected(&[
            ("OnInitialize", WITHOUT_TALK),
            ("username", WITHOUT_TALK),
            ("OnUpdateComplete", WITHOUT_TALK),
        ])
    );
}

/// 定常（トークなし）で受理される切替の要求。
fn change_request() -> crate::change::ChangeRequest {
    crate::change::ChangeRequest {
        target: crate::change::ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: r"C:\areka\ghost\r_post".to_string(),
        },
        origin: crate::change::ChangeOrigin::Manual,
        raise_event: true,
    }
}

fn steady_idle() -> State {
    State {
        phase: Phase::Steady { talk: None },
        ..State::initial()
    }
}

/// 要件 5.1: 切替の OnGhostChanging と、その 204 の後の OnClose に写しの online とバルーンが載る。
#[test]
fn switching_requests_carry_online_and_balloons_from_the_copy() {
    let seen = walk(
        steady_idle(),
        vec![
            Input::ChangeGhost(change_request()),
            reply(crate::msg::ShioriOutcome::NoContent),
        ],
        &config(),
    );
    assert_eq!(
        seen,
        expected(&[("OnGhostChanging", WITHOUT_TALK), ("OnClose", WITHOUT_TALK)])
    );
}

/// 要件 5.1: 終了の握手の OnClose に写しの online とバルーンが載る。
#[test]
fn close_handshake_request_carries_online_and_balloons_from_the_copy() {
    let seen = walk(
        steady_idle(),
        vec![Input::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        }],
        &config(),
    );
    assert_eq!(seen, expected(&[("OnClose", WITHOUT_TALK)]));
}

/// 要件 5.1: 強制終了の OnClose は、トーク中に届いてもトークと旗を落とし、写しの online と
/// バルーンだけを載せる。
#[test]
fn force_quit_request_carries_online_and_balloons_from_the_copy() {
    let (_, talking) = phases().swap_remove(1);
    let seen = walk(
        State {
            phase: talking,
            ..State::initial()
        },
        vec![Input::ForceQuit {
            reason: CloseReason::System,
        }],
        &config(),
    );
    assert_eq!(seen, expected(&[("OnClose", WITHOUT_TALK)]));
}
