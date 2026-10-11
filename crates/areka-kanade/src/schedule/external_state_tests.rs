// =============================================================================
// 外から届いた実行状態の写し（`Input::ExecutionState`）の決定論テスト
// =============================================================================
//
// 入口は `schedule::step` の横断の腕で、殻（`actor.rs`）が `KanadeMsg::ExecutionState` をそのまま
// 写して渡す。ここでは `step` を直に呼び、写しが変わることと行動を返さないことを見る。

use super::translate_test_support::pass_translate;
use super::{Action, ActiveTalk, Input, Phase, State, TermCause, step};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs};
use crate::status::{BalloonBinding, ExecutionStateUpdate, ExternalStates};
use crate::talk::{TalkDone, TalkEndReason, TalkId};

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

/// 再生中のトーク（番号 3）。
fn talk() -> Option<ActiveTalk> {
    Some(ActiveTalk {
        talk_id: TalkId(3),
        origin: "OnTest",
        script: r"\0こんにちは\e".to_string(),
    })
}

/// 運行の相のすべての種類。トークの有無で分かれる相（`BootVersion`・`Steady`）は両方作るので
/// 17 通り（相を問わないことと、相ごとの「再生中か」を見る）。2 番目はトーク中の定常のまま動かさない。
fn phases() -> Vec<(&'static str, Phase)> {
    let (talk_id, deadline) = (TalkId(3), None);
    vec![
        ("Idle", Phase::Idle),
        ("Steady{Some}", Phase::Steady { talk: talk() }),
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
        ("BootInit", Phase::BootInit),
        ("BootPrefetch", Phase::BootPrefetch),
        ("BootType", Phase::BootType),
        ("BootMain", Phase::BootMain),
        ("BootVersion{None}", Phase::BootVersion { talk: None }),
        ("BootVersion{Some}", Phase::BootVersion { talk: talk() }),
        ("Steady{None}", Phase::Steady { talk: None }),
        ("CloseTalkWait", Phase::CloseTalkWait { talk_id, deadline }),
        (
            "ChangeTalkWait",
            Phase::ChangeTalkWait { talk_id, deadline },
        ),
        ("ChangeClosePending", Phase::ChangeClosePending),
        (
            "ChangeCloseTalkWait",
            Phase::ChangeCloseTalkWait { talk_id, deadline },
        ),
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

// -----------------------------------------------------------------------------
// お別れの台詞の再生中の `talking`（areka-P0-farewell-talk-status 要件 1〜4）
// -----------------------------------------------------------------------------

/// 「再生中か」の期待の表（手書き。相を足すとここでコンパイルが止まり、判断が求められる）。
/// 判定の関数からも、番号を引く関数からも導かない。
fn plays_a_talk(phase: &Phase) -> bool {
    match phase {
        Phase::Idle | Phase::BootInit | Phase::BootPrefetch | Phase::BootType | Phase::BootMain => {
            false
        }
        Phase::BootVersion { talk: None } | Phase::Steady { talk: None } => false,
        Phase::BootVersion { talk: Some(_) } | Phase::Steady { talk: Some(_) } => true,
        Phase::ClosePending { .. } | Phase::ChangePending | Phase::ChangeClosePending => false,
        Phase::CloseTalkWait { .. }
        | Phase::ChangeTalkWait { .. }
        | Phase::ChangeCloseTalkWait { .. } => true,
        Phase::Unloading { .. } | Phase::Stopped => false,
    }
}

/// 要件 4.1・4.6（1.4・1.5・1.7・3.2）: 運行の相のすべての種類で、判定・再生中のトークの番号が
/// 引けるか・実行の状態の素の 3 つが、手書きの表と一致する。食い違う相はまとめて挙げる。
#[test]
fn talk_judgment_matches_the_table_in_every_phase() {
    let kinds: std::collections::BTreeSet<_> = phases()
        .iter()
        .map(|(_, phase)| super::phase_label(phase))
        .collect();
    assert_eq!(
        (phases().len(), kinds.len()),
        (17, 15),
        "相の 15 種類を 17 通りで並べる"
    );

    let mut wrong = Vec::new();
    for (label, phase) in phases() {
        let expected = plays_a_talk(&phase);
        let judged = super::talk_active_of(&phase);
        let numbered = super::current_talk_id(&phase).is_some();
        let state = State {
            phase,
            ..State::initial()
        };
        let snapshot = state.snapshot().talk_active;
        if (judged, numbered, snapshot) != (expected, expected, expected) {
            wrong.push(format!(
                "{label}: 表={expected} 判定={judged} 番号が引ける={numbered} 状態の素={snapshot}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "表と食い違う相がある:\n{}",
        wrong.join("\n")
    );
}

/// お別れの 3 つの場面: (名前, 台詞が返るまでに送る握手の要求の列, 再生中の相)。
/// 最後の要求にだけ台詞が返り、その前の要求は応答なし（204）。
const FAREWELLS: [(&str, &[&str], &str); 3] = [
    ("終了の挨拶", &["OnClose"], "CloseTalkWait"),
    (
        "切り替えの送り出しの台詞",
        &["OnGhostChanging"],
        "ChangeTalkWait",
    ),
    (
        "切り替えの別れの台詞",
        &["OnGhostChanging", "OnClose"],
        "ChangeCloseTalkWait",
    ),
];

/// SHIORI が返すお別れの台詞。
const FAREWELL: &str = r"\0さようなら\e";

/// お別れの台詞の再生中の `Status`（旗の写しが立っている間。`choosing` は無い）。
const TALKING_FLAGGED: &str = "talking,nouserbreak,online,balloon(0=2/1=0)";

/// その時点の状態から導いた `Status` の値（状態の問い合わせの答えと同じ素から作る）。
fn status_of(state: &State) -> Option<String> {
    crate::status::ExecutionStatus::derive(&state.snapshot()).render()
}

fn done(talk_id: TalkId, reason: TalkEndReason) -> Input {
    Input::TalkDone(TalkDone {
        talk_id,
        reason,
        quit_reserved: false,
    })
}

/// 写しを満たしたトーク無しの定常から、場面の握手を実際の入力で進め、台詞の応答が返した
/// (状態, 一括) を返す。応答待ちのたびに、握手の要求も、その時点の状態から導いた `Status` も
/// 会話なしであることを見る（要件 1.5・3.1）。
fn farewell_reply(label: &str, requests: &[&str]) -> (State, Vec<Action>) {
    let trigger = match requests[0] {
        "OnClose" => Input::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
        _ => Input::ChangeGhost(change_request()),
    };
    let start = State {
        external: full_copy(),
        last_now: Some(MonotonicMs(1_000)),
        ..steady_idle()
    };
    let mut next = step(start, trigger, &config());
    for (i, &request) in requests.iter().enumerate() {
        let (state, actions) = next;
        assert_eq!(
            expected(&[(request, WITHOUT_TALK)]),
            [last_request(&actions)],
            "{label}: 握手の要求は会話なし"
        );
        assert_eq!(
            status_of(&state).as_deref(),
            Some(WITHOUT_TALK),
            "{label}: {request} の応答待ちの間は talking 無し"
        );
        let outcome = if i + 1 < requests.len() {
            crate::msg::ShioriOutcome::NoContent
        } else {
            crate::msg::ShioriOutcome::Value(FAREWELL.to_string())
        };
        next = step(state, reply(outcome), &config());
    }
    next
}

/// 場面の台詞の再生が始まったところまで進め、(状態, 再生中のトークの番号) を返す。
/// 翻訳の結果を入れて返るのは再生の開始だけ（SHIORI への要求は 0・要件 3.3）。
fn farewell_playing(label: &str, requests: &[&str]) -> (State, TalkId) {
    let (state, actions) = pass_translate(farewell_reply(label, requests), &config());
    let [Action::StartTalk(start)] = actions.as_slice() else {
        panic!("{label}: 再生の開始だけが返るはず");
    };
    (state, start.talk_id)
}

/// 要件 1.1〜1.3・1.6・2.2・3.4・4.3・4.4: お別れの 3 つの場面とも、台詞の翻訳の依頼の `Status` と
/// 再生中の `Status` に talking と nouserbreak が載り、旗を下ろす知らせの後は nouserbreak だけ消える。
#[test]
fn farewell_talk_carries_talking_from_the_translation_through_the_playback() {
    for (label, requests, phase) in FAREWELLS {
        let (_, actions) = farewell_reply(label, requests);
        let [Action::Translate(request)] = actions.as_slice() else {
            panic!("{label}: 台詞の応答の一括は翻訳の行動 1 つのはず");
        };
        assert_eq!(
            request.status.render().as_deref(),
            Some(TALKING_FLAGGED),
            "{label}: 翻訳の依頼の Status"
        );

        let (state, _) = farewell_playing(label, requests);
        assert_eq!(super::phase_label(&state.phase), phase, "{label}");
        assert_eq!(
            status_of(&state).as_deref(),
            Some(TALKING_FLAGGED),
            "{label}: 再生中の Status"
        );

        let (state, actions) = step(
            state,
            Input::ExecutionState(ExecutionStateUpdate::NoUserBreak(false)),
            &config(),
        );
        assert!(actions.is_empty(), "{label}: 知らせは行動を返さない");
        assert_eq!(
            status_of(&state).as_deref(),
            Some("talking,online,balloon(0=2/1=0)"),
            "{label}: 旗を下ろした後は nouserbreak 無し"
        );
    }
}

/// お別れの台詞の終わり方（再生中の状態とトークの番号から、終わった後の状態と一括を返す）。
type Ending = fn(State, TalkId) -> (State, Vec<Action>);

/// 要件 1.4（3.3）: 終わり方 4 通りのどれでも、再生中に載っていた talking と nouserbreak が
/// 終わった後は消える（旗の写しは立てたまま）。切り替えの場面の利用者の中断は切り替えを中止して
/// 定常へ戻り、その後に送る `OnBalloonBreak` も会話なし。ほかは降ろすだけで、SHIORI への要求は 0。
///
/// 利用者の中断の受理は旗の写しを見ない（`user_break::on_user_break`）ので、旗を立てたまま流す。
#[test]
fn farewell_talk_drops_talking_after_every_ending() {
    let endings: [(&str, Ending); 4] = [
        ("最後まで", |state, talk_id| {
            step(state, done(talk_id, TalkEndReason::Ended), &config())
        }),
        (r"\- に達した", |state, talk_id| {
            step(state, done(talk_id, TalkEndReason::Quit), &config())
        }),
        ("利用者の中断", |state, talk_id| {
            let (state, stop) = step(state, Input::UserBreak { scope: 0 }, &config());
            assert!(
                matches!(stop.as_slice(), [Action::CancelChoice { .. }]),
                "中断を受理して止める"
            );
            step(state, done(talk_id, TalkEndReason::Interrupted), &config())
        }),
        ("上限超過", |state, _| {
            let now = MonotonicMs(1_000 + config().close_talk_deadline_ms);
            step(state, Input::Tick { now }, &config())
        }),
    ];
    for (label, requests, _) in FAREWELLS {
        for (ending, end) in endings {
            let (state, talk_id) = farewell_playing(label, requests);
            assert_eq!(
                status_of(&state).as_deref(),
                Some(TALKING_FLAGGED),
                "{label}／{ending}: 終わる前は再生中"
            );

            let (state, actions) = end(state, talk_id);
            assert_eq!(
                status_of(&state).as_deref(),
                Some(WITHOUT_TALK),
                "{label}／{ending}: 終わった後は talking も nouserbreak も無い"
            );
            if requests[0] == "OnGhostChanging" && ending == "利用者の中断" {
                assert_eq!(super::phase_label(&state.phase), "Steady", "{label}");
                assert_eq!(
                    expected(&[("OnBalloonBreak", WITHOUT_TALK)]),
                    [last_request(&actions)],
                    "{label}: 定常へ戻った後の OnBalloonBreak は会話なし"
                );
            } else {
                assert_eq!(super::phase_label(&state.phase), "Unloading", "{label}");
                assert!(
                    matches!(actions.as_slice(), [Action::ShioriUnload]),
                    "{label}／{ending}: 降ろすだけ"
                );
            }
        }
    }
}

/// 要件 3.1・4.5 の ⑴: 普段の会話の再生中に届いた終了の要求は会話の完了まで保留し、完了の後に
/// 送る `OnClose` は会話なし（⑵⑶＝`OnGhostChanging` とその応答なしの後の `OnClose` は、同じ写しで
/// 流す `switching_requests_carry_online_and_balloons_from_the_copy` が固定している）。
#[test]
fn close_request_held_through_a_talk_sends_on_close_without_talk() {
    let (_, talking) = phases().swap_remove(1);
    let state = State {
        phase: talking,
        external: full_copy(),
        ..State::initial()
    };
    let (state, actions) = step(
        state,
        Input::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
        &config(),
    );
    assert!(actions.is_empty(), "会話の完了まで保留する");
    assert_eq!(status_of(&state).as_deref(), Some(TALKING_FLAGGED));

    let (_, actions) = step(state, done(TalkId(3), TalkEndReason::Ended), &config());
    assert_eq!(
        expected(&[("OnClose", WITHOUT_TALK)]),
        [last_request(&actions)]
    );
}

/// 要件 3.1・4.5 の ⑷: お別れの 3 つの場面のどの再生中に強制終了が届いても、`OnClose` の通知は
/// 会話なし（talking も nouserbreak も載せない）。
#[test]
fn force_quit_during_a_farewell_talk_notifies_on_close_without_talk() {
    for (label, requests, _) in FAREWELLS {
        let (state, _) = farewell_playing(label, requests);
        let (_, actions) = step(
            state,
            Input::ForceQuit {
                reason: CloseReason::System,
            },
            &config(),
        );
        assert_eq!(
            expected(&[("OnClose", WITHOUT_TALK)]),
            [last_request(&actions)],
            "{label}"
        );
    }
}
