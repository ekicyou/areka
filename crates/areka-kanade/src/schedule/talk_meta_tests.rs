//! 再生の開始に押される実行するゴーストと送り手。
//!
//! どの確かめも最上位の [`step`] を通し、台本を作る所ごとに最後の入力で出る再生の開始の付帯情報と
//! 出どころを見る。

use std::path::PathBuf;

use dola::time::testing::{at_ms, delta_ms};

use super::super::events::PROPERTY_EVENT_ORIGIN;
use super::super::sstp::{NOTIFY_ORIGIN, RAISE_EVENT_TOOL, SAKURASCRIPT_TOOL};
use super::super::translate::Translated;
use super::super::{Action, Input, State, step};
use crate::msg::{
    ChoiceInput, ClickButton, CloseReason, KanadeConfig, MouseEventKind, MouseInput, ShioriOutcome,
};
use crate::schedule::translate_test_support::{
    CHOICE_ID, change_request, config as base_config, pass_translate, steady_none,
};
use crate::talk::{
    EpilogueCommand, GhostPlace, ScriptOrigin, StartTalk, TalkDone, TalkEndReason, TalkId,
    TalkRunner, TalkSender,
};
use areka_talk::SecurityLevel;

const SCRIPT: &str = r"\0台本\e";
const SCRIPT_CHOICE: &str = r"script:\e";
const CLOSE_BY_USER: CloseReason = CloseReason::User { scope: 0 };

fn place() -> GhostPlace {
    GhostPlace {
        master: PathBuf::from(r"C:\ghosts\a\ghost\master"),
        root: PathBuf::from(r"C:\ghosts\a"),
    }
}

fn config() -> KanadeConfig {
    KanadeConfig {
        ghost_name: "あ".to_string(),
        ghost_place: Some(place()),
        ..base_config()
    }
}

fn value(origin: &'static str) -> Input {
    Input::ShioriReply {
        outcome: ShioriOutcome::Value(SCRIPT.to_string(), SecurityLevel::Local),
        origin,
    }
}

fn reply(outcome: ShioriOutcome) -> Input {
    Input::ShioriReply {
        outcome,
        origin: "test",
    }
}

fn last_start_of(actions: Vec<Action>) -> Option<StartTalk> {
    actions.into_iter().rev().find_map(|action| match action {
        Action::StartTalk(start) => Some(start),
        _ => None,
    })
}

/// 入力を順に [`step`] へ通し（翻訳は台詞そのままで返す）、最後の入力で出た再生の開始を返す。
fn last_start(config: &KanadeConfig, state: State, inputs: Vec<Input>) -> StartTalk {
    let mut state = state;
    let mut actions = Vec::new();
    for input in inputs {
        (state, actions) = pass_translate(step(state, input, config), config);
    }
    last_start_of(actions).expect("最後の入力で再生の開始が出るはず")
}

/// 入力を順に [`step`] へ通し、最後の入力が翻訳へ預けたことを確かめ、翻訳の結果で出た再生の開始を返す。
fn start_after_translation(config: &KanadeConfig, inputs: Vec<Input>) -> StartTalk {
    let mut state = steady_none();
    let mut actions = Vec::new();
    for input in inputs {
        (state, actions) = step(state, input, config);
    }
    assert!(
        matches!(actions.as_slice(), [Action::Translate(_)]),
        "台本は翻訳へ預けられる"
    );
    let (_, actions) = step(
        state,
        Input::TranslateDone(Ok(Translated::local(SCRIPT))),
        config,
    );
    last_start_of(actions).expect("翻訳の結果で再生の開始が出るはず")
}

fn assert_started(start: &StartTalk, origin: ScriptOrigin, sender: TalkSender) {
    assert_eq!(
        start.meta.runner,
        TalkRunner {
            name: "あ".to_string(),
            place: Some(place()),
        },
        "実行するゴーストは設定の名と置き場"
    );
    assert_eq!(start.meta.sender, sender);
    assert_eq!(start.origin, origin);
}

/// 毎秒のポンプの応答でトーク 5 を始める入力。
fn ghost_talk() -> Vec<Input> {
    vec![Input::Tick { now: at_ms(2_000) }, value("OnSecondChange")]
}

fn choice_waiting(choice_id: &str, timeout_directive: Option<dola::TimeDelta>) -> Input {
    Input::ChoiceWaiting {
        talk_id: TalkId(5),
        choice_ids: vec![choice_id.to_string()],
        display_end: at_ms(3_000),
        timeout_directive,
    }
}

fn choose(choice_id: &str) -> Input {
    Input::Choice(ChoiceInput {
        id: choice_id.to_string(),
        label: "選ぶ".to_string(),
        scope: 0,
        references: Vec::new(),
    })
}

fn with(mut inputs: Vec<Input>, rest: Vec<Input>) -> Vec<Input> {
    inputs.extend(rest);
    inputs
}

#[test]
fn steady_reply_talk_is_sent_by_runner() {
    let start = last_start(&config(), steady_none(), ghost_talk());
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn replacing_reply_talk_is_sent_by_runner() {
    let mouse_up = Input::Mouse(MouseInput {
        scope: 0,
        x: 1,
        y: 2,
        region: None,
        kind: MouseEventKind::ButtonUp {
            button: ClickButton::Left,
            second_of_double: true,
        },
    });
    let start = last_start(
        &config(),
        steady_none(),
        with(ghost_talk(), vec![mouse_up, value("OnMouseUp")]),
    );
    assert_eq!(start.talk_id, TalkId(6), "流れているトークを置き換える");
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn choice_stage_reply_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        with(
            ghost_talk(),
            vec![
                choice_waiting(CHOICE_ID, None),
                choose(CHOICE_ID),
                value("OnChoiceSelectEx"),
            ],
        ),
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn choice_timeout_reply_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        with(
            ghost_talk(),
            vec![
                choice_waiting(CHOICE_ID, Some(delta_ms(1_000))),
                Input::Tick { now: at_ms(10_000) },
                value("OnChoiceTimeout"),
            ],
        ),
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn balloon_event_reply_talk_is_sent_by_runner() {
    let ended = Input::TalkDone(TalkDone {
        talk_id: TalkId(5),
        reason: TalkEndReason::Ended,
        quit_reserved: false,
    });
    let start = last_start(
        &config(),
        steady_none(),
        with(
            ghost_talk(),
            vec![
                ended,
                Input::BalloonTimeout { talk_id: TalkId(5) },
                value("OnBalloonTimeout"),
            ],
        ),
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn property_event_reply_talk_keeps_property_origin_and_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        vec![
            Input::PropertyEvent {
                id: "OnX".to_string(),
                references: Vec::new(),
            },
            value(PROPERTY_EVENT_ORIGIN),
        ],
    );
    assert_started(&start, ScriptOrigin::PROPERTY, TalkSender::Runner);
}

#[test]
fn boot_greeting_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        State::initial(),
        vec![
            Input::Boot,
            reply(ShioriOutcome::Notified),
            reply(ShioriOutcome::NoContent),
            value("test"),
        ],
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn epilogue_only_boot_talk_is_sent_by_runner() {
    let config = KanadeConfig {
        first_boot: true,
        first_boot_epilogue: vec![EpilogueCommand {
            name: "areka.prop.set".to_string(),
            tokens: vec!["areka.boot.count".to_string(), "1".to_string()],
        }],
        ..config()
    };
    let start = last_start(
        &config,
        State::initial(),
        vec![
            Input::Boot,
            reply(ShioriOutcome::Notified),
            reply(ShioriOutcome::NoContent),
            reply(ShioriOutcome::NoContent),
            reply(ShioriOutcome::NoContent),
        ],
    );
    assert_eq!(start.script, "", "後書きだけのトーク");
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn ghost_changing_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        vec![
            Input::ChangeGhost(change_request()),
            value("OnGhostChanging"),
        ],
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn change_on_close_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        vec![
            Input::ChangeGhost(change_request()),
            reply(ShioriOutcome::NoContent),
            value("OnClose"),
        ],
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn yielded_change_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        vec![
            Input::ChangeGhost(change_request()),
            Input::CloseRequest {
                reason: CLOSE_BY_USER,
            },
            value("OnGhostChanging"),
        ],
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn close_on_close_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        vec![
            Input::CloseRequest {
                reason: CLOSE_BY_USER,
            },
            value("OnClose"),
        ],
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn mcp_sakurascript_talk_is_sent_by_tool_after_translation() {
    let start = start_after_translation(
        &config(),
        vec![Input::SstpSend {
            script: SCRIPT.to_string(),
            strict: None,
        }],
    );
    assert_started(
        &start,
        ScriptOrigin::MCP_SEND,
        TalkSender::Tool(SAKURASCRIPT_TOOL.to_string()),
    );
}

#[test]
fn mcp_raise_event_reply_talk_is_sent_by_tool_after_translation() {
    let start = start_after_translation(
        &config(),
        vec![
            Input::SstpNotify {
                event: "OnTest".to_string(),
                references: Vec::new(),
                strict: None,
            },
            value(NOTIFY_ORIGIN),
        ],
    );
    assert_started(
        &start,
        ScriptOrigin::MCP_NOTIFY,
        TalkSender::Tool(RAISE_EVENT_TOOL.to_string()),
    );
}

#[test]
fn script_choice_after_mcp_talk_is_sent_by_tool() {
    let start = last_start(
        &config(),
        steady_none(),
        vec![
            Input::SstpSend {
                script: SCRIPT.to_string(),
                strict: None,
            },
            choice_waiting(SCRIPT_CHOICE, None),
            choose(SCRIPT_CHOICE),
        ],
    );
    assert_started(
        &start,
        ScriptOrigin::MCP_SEND,
        TalkSender::Tool(SAKURASCRIPT_TOOL.to_string()),
    );
}

#[test]
fn script_choice_after_ghost_talk_is_sent_by_runner() {
    let start = last_start(
        &config(),
        steady_none(),
        with(
            ghost_talk(),
            vec![choice_waiting(SCRIPT_CHOICE, None), choose(SCRIPT_CHOICE)],
        ),
    );
    assert_started(&start, ScriptOrigin::GHOST, TalkSender::Runner);
}

#[test]
fn kanade_without_place_starts_talk_with_no_runner_place() {
    let start = last_start(&base_config(), steady_none(), ghost_talk());
    assert_eq!(start.meta.runner.place, None);
    assert_eq!(start.meta.sender, TalkSender::Runner);
}
