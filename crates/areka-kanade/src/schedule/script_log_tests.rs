//! script 種別のログの決定論テスト（`schedule/script_log.rs` の兄弟・要件 7.1 ⑽）。
//!
//! 最上位の `step` から各道を通し、呼んだスレッドで捕まえた `areka::log::script` の記録が
//! トーク 1 つにつき 1 件・表示の語が表のとおり・本文が翻訳の後の台本であることを固定する（要件 4.1〜4.3）。

use super::{PLAIN_LABEL, SSTP_SEND_LABEL, TARGET, event_label};
use crate::msg::{
    ChoiceInput, CloseReason, KanadeConfig, MonotonicMs, MouseButton, MouseEventKind, MouseInput,
    ShioriCall, ShioriFailure, ShioriOutcome,
};
use crate::schedule::log_capture::capture;
use crate::schedule::{Action, ActiveTalk, ChoicePhase, ChoiceState, Input, Phase, State, step};
use crate::talk::TalkId;
use tracing::Level;

const GHOST: &str = "えもこ";

fn config() -> KanadeConfig {
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.ghost_name = GHOST.to_string();
    // 起動記録あり＝根なしで OnBoot へ直行する（起動の挨拶を OnBoot の応答で出す）。
    config.first_boot = false;
    config
}

fn steady(talk: Option<ActiveTalk>) -> State {
    State {
        phase: Phase::Steady { talk },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 10,
        ..State::initial()
    }
}

fn reply(state: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    step(
        state,
        Input::ShioriReply {
            outcome,
            origin: "test",
        },
        &config(),
    )
}

/// 一括の最後の GET の ID（応答を返す相手の確かめ）。
fn last_get(actions: &[Action]) -> &str {
    actions
        .iter()
        .rev()
        .find_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, .. }) => Some(id.as_str()),
            _ => None,
        })
        .expect("GET を送っているはず")
}

/// 台本の応答を返し、翻訳の依頼を `translated` で入れ直して再生の開始まで進める。
fn answer(state: State, script: &str, translated: &str) -> State {
    let (state, actions) = reply(state, ShioriOutcome::Value(script.to_string()));
    assert!(
        matches!(actions.as_slice(), [Action::Translate(_)]),
        "台詞の応答は翻訳へ預けるはず"
    );
    let (state, actions) = step(
        state,
        Input::TranslateDone(Ok(translated.to_string())),
        &config(),
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::StartTalk(s) if s.script == translated)),
        "翻訳の後の台本で再生を始めるはず"
    );
    state
}

/// 捕まえた記録のうち script 種別のもの（レベル・ghost・label・本文）。
fn script_records(f: impl FnOnce()) -> Vec<(Level, String, String, String)> {
    capture(f)
        .into_iter()
        .filter(|e| e.target == TARGET)
        .map(|e| {
            (
                e.level,
                e.fields.get("ghost").cloned().unwrap_or_default(),
                e.fields.get("label").cloned().unwrap_or_default(),
                e.message.unwrap_or_default(),
            )
        })
        .collect()
}

fn assert_one(records: &[(Level, String, String, String)], label: &str, script: &str) {
    assert_eq!(
        records,
        [(
            Level::INFO,
            GHOST.to_string(),
            label.to_string(),
            script.to_string()
        )],
        "トーク 1 つにつき 1 件・表示の語 {label}"
    );
}

#[test]
fn labels_follow_the_table() {
    assert_eq!(TARGET, "areka::log::script");
    assert_eq!(SSTP_SEND_LABEL, "SSTP(Local,Auth)");
    assert_eq!(PLAIN_LABEL, "Ghost");
    assert_eq!(event_label("OnAiTalk"), "Ghost:OnAiTalk");
}

/// 起動の挨拶（OnBoot の応答）は `Ghost:OnBoot`。
#[test]
fn boot_greeting_is_logged_once_as_ghost_onboot() {
    let records = script_records(|| {
        let cfg = config();
        let (s, _) = step(State::initial(), Input::Boot, &cfg);
        let (s, _) = reply(s, ShioriOutcome::Notified); // OnInitialize → username 照会
        let (s, actions) = reply(s, ShioriOutcome::NoContent); // 照会 → OnBoot
        assert_eq!(last_get(&actions), "OnBoot");
        answer(s, "\\h挨拶\\e", "\\h訳した挨拶\\e");
    });
    assert_one(&records, "Ghost:OnBoot", "\\h訳した挨拶\\e");
}

/// 定常の自発の台本（OnSecondChange の応答）は `Ghost:OnSecondChange`。
#[test]
fn steady_spontaneous_talk_is_logged_once() {
    let records = script_records(|| {
        let (s, actions) = step(
            steady(None),
            Input::Tick {
                now: MonotonicMs(1500),
            },
            &config(),
        );
        assert_eq!(last_get(&actions), "OnSecondChange");
        answer(s, "\\h自発\\e", "\\h訳した自発\\e");
    });
    assert_one(&records, "Ghost:OnSecondChange", "\\h訳した自発\\e");
}

/// マウスの反応（OnMouseDoubleClick の応答）は `Ghost:OnMouseDoubleClick`。
#[test]
fn mouse_reaction_is_logged_once() {
    let records = script_records(|| {
        let mouse = MouseInput {
            scope: 0,
            x: 10,
            y: 20,
            region: Some("Bust".to_string()),
            kind: MouseEventKind::DoubleClick {
                button: MouseButton::Left,
            },
        };
        let (s, actions) = step(steady(None), Input::Mouse(mouse), &config());
        assert_eq!(last_get(&actions), "OnMouseDoubleClick");
        answer(s, "\\hつつかれた\\e", "\\h訳したつつかれた\\e");
    });
    assert_one(
        &records,
        "Ghost:OnMouseDoubleClick",
        "\\h訳したつつかれた\\e",
    );
}

/// 選択肢の `script:` は SHIORI の往復から生まれていないので `Ghost`。
#[test]
fn choice_script_is_logged_once_as_plain_ghost() {
    let mut state = steady(Some(ActiveTalk {
        talk_id: TalkId(3),
        origin: "OnSecondChange",
        script: String::new(),
    }));
    state.choice = Some(ChoiceState {
        talk_id: TalkId(3),
        candidates: vec!["script:\\h選んだ\\e".to_string()],
        deadline: Some(MonotonicMs(32_000)),
        phase: ChoicePhase::Waiting,
    });
    let records = script_records(|| {
        let (_, actions) = step(
            state,
            Input::Choice(ChoiceInput {
                id: "script:\\h選んだ\\e".to_string(),
                label: "選ぶ".to_string(),
                scope: 0,
                references: Vec::new(),
            }),
            &config(),
        );
        assert!(actions.iter().any(|a| matches!(a, Action::StartTalk(_))));
    });
    assert_one(&records, PLAIN_LABEL, "\\h選んだ\\e");
}

/// 外からの台本は `SSTP(Local,Auth)`。翻訳に預けた歩では出さず、翻訳の結果の歩で 1 件。
#[test]
fn sstp_send_is_logged_once_after_translate() {
    let records = script_records(|| {
        let (s, actions) = step(
            steady(None),
            Input::SstpSend {
                script: "\\h外\\e".to_string(),
                strict: None,
            },
            &config(),
        );
        assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
        let (_, actions) = step(
            s,
            Input::TranslateDone(Ok("\\h訳した外\\e".into())),
            &config(),
        );
        assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    });
    assert_one(&records, SSTP_SEND_LABEL, "\\h訳した外\\e");
}

/// 外から頼まれたイベントの応答は `Ghost:<名前>`。
#[test]
fn sstp_notify_is_logged_once_as_the_event_name() {
    let records = script_records(|| {
        let (s, actions) = step(
            steady(None),
            Input::SstpNotify {
                event: "OnAiTalk".to_string(),
                references: Vec::new(),
                strict: Some(true),
            },
            &config(),
        );
        assert_eq!(last_get(&actions), "OnAiTalk");
        answer(s, "\\h頼まれた\\e", "\\h訳した頼まれた\\e");
    });
    assert_one(&records, "Ghost:OnAiTalk", "\\h訳した頼まれた\\e");
}

/// 終了の挨拶（OnClose の応答）は `Ghost:OnClose`。
#[test]
fn close_greeting_is_logged_once_as_ghost_onclose() {
    let records = script_records(|| {
        let (s, actions) = step(
            steady(None),
            Input::CloseRequest {
                reason: CloseReason::User { scope: 0 },
            },
            &config(),
        );
        assert_eq!(last_get(&actions), "OnClose");
        answer(s, "\\hまたね\\e", "\\h訳したまたね\\e");
    });
    assert_one(&records, "Ghost:OnClose", "\\h訳したまたね\\e");
}

/// 翻訳の輸送路が失敗したトークは始まらないので 0 件（要件 4.3）。
#[test]
fn translate_transport_failure_logs_nothing() {
    let records = script_records(|| {
        let (s, _) = step(
            steady(None),
            Input::Tick {
                now: MonotonicMs(1500),
            },
            &config(),
        );
        let (s, actions) = reply(s, ShioriOutcome::Value("\\h自発\\e".to_string()));
        assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
        let (_, actions) = step(
            s,
            Input::TranslateDone(Err(ShioriFailure::Ipc("切れた".to_string()))),
            &config(),
        );
        assert!(!actions.iter().any(|a| matches!(a, Action::StartTalk(_))));
    });
    assert!(records.is_empty(), "記録は 0 件のはず: {records:?}");
}

/// 0 文字の台本のトーク（空の 200 の応答・空の外からの台本）は 0 件。
#[test]
fn empty_script_talk_logs_nothing() {
    let records = script_records(|| {
        let (s, _) = step(
            steady(None),
            Input::Tick {
                now: MonotonicMs(1500),
            },
            &config(),
        );
        let (_, actions) = reply(s, ShioriOutcome::Value(String::new()));
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::StartTalk(s) if s.script.is_empty()))
        );
        let (_, actions) = step(
            steady(None),
            Input::SstpSend {
                script: String::new(),
                strict: None,
            },
            &config(),
        );
        assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
    });
    assert!(records.is_empty(), "記録は 0 件のはず: {records:?}");
}
