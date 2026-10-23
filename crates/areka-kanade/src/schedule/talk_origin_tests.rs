//! SHIORI の応答で始まるトークと外からの台本の出どころ（要件 1.1〜1.4・3.1・3.2）。
//!
//! どの確かめも最上位の [`step`] を通す: 元のイベント（ゴースト自身の毎秒・外から頼まれたイベント）を
//! 送らせ、その台詞の応答を入れ、翻訳の結果を入れ直して、最後に出る再生の開始の出どころを見る。
//! 外からの台本は入力そのもので始まる。

use tracing::Level;

use super::super::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use super::super::sstp::NOTIFY_ORIGIN;
use super::super::steady::test_support::base_state;
use super::super::translate::Translated;
use super::super::{Action, Input, Phase, State, step};
use crate::msg::{KanadeConfig, MonotonicMs, ShioriCall, ShioriOutcome};
use crate::talk::ScriptOrigin;
use areka_talk::SecurityLevel;

const SCRIPT: &str = "\\0台本\\e";

fn cfg() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn steady() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..base_state()
    }
}

/// 入力を順に [`step`] へ通し、最後の一括と全体の記録を返す。
fn run(inputs: Vec<Input>) -> (Vec<Action>, Vec<CapturedEvent>) {
    let cfg = cfg();
    let mut state = Some(steady());
    let mut last = Vec::new();
    let ev = capture(|| {
        for input in inputs {
            let (s, actions) = step(state.take().expect("状態"), input, &cfg);
            state = Some(s);
            last = actions;
        }
    });
    (last, ev)
}

fn translated(level: SecurityLevel) -> Input {
    Input::TranslateDone(Ok(Translated {
        script: SCRIPT.to_string(),
        level,
    }))
}

fn reply(level: SecurityLevel, origin: &'static str) -> Input {
    Input::ShioriReply {
        outcome: ShioriOutcome::Value(SCRIPT.to_string(), level),
        origin,
    }
}

/// 毎秒のポンプ（ゴースト自身のイベント）の応答で始まるトークの、翻訳の後の出どころ。
fn ghost_talk(
    reply_level: SecurityLevel,
    translate_level: SecurityLevel,
) -> (ScriptOrigin, Vec<CapturedEvent>) {
    let (actions, ev) = run(vec![
        Input::Tick {
            now: MonotonicMs(2_000),
        },
        reply(reply_level, "OnSecondChange"),
        translated(translate_level),
    ]);
    (started(&actions), ev)
}

/// 外から頼まれたイベントの応答で始まるトークの、翻訳の後の出どころ。
fn notify_talk(reply_level: SecurityLevel) -> (ScriptOrigin, Vec<CapturedEvent>) {
    let (actions, ev) = run(vec![
        Input::SstpNotify {
            event: "OnTest".to_string(),
            references: Vec::new(),
            strict: None,
        },
        reply(reply_level, NOTIFY_ORIGIN),
        translated(SecurityLevel::Local),
    ]);
    (started(&actions), ev)
}

fn started(actions: &[Action]) -> ScriptOrigin {
    match actions {
        [Action::StartTalk(start)] => start.origin,
        other => panic!("再生の開始が 1 つ返るはず: {}", other.len()),
    }
}

#[test]
fn ghost_event_reply_talk_is_ghost() {
    let (origin, ev) = ghost_talk(SecurityLevel::Local, SecurityLevel::Local);
    assert_eq!(origin, ScriptOrigin::GHOST, "要件 1.1");
    assert_not_logged(&ev, "script_security_lowered");
}

#[test]
fn raise_event_reply_talk_is_mcp_notify() {
    // 送った往復が外から頼まれたイベント（EventId::Sstp）であることを前提として確かめる。
    let (first, _) = run(vec![Input::SstpNotify {
        event: "OnTest".to_string(),
        references: Vec::new(),
        strict: None,
    }]);
    assert!(matches!(
        first.as_slice(),
        [Action::ShioriRequest(ShioriCall::Get { .. })]
    ));
    let (origin, ev) = notify_talk(SecurityLevel::Local);
    assert_eq!(origin, ScriptOrigin::MCP_NOTIFY, "要件 1.3");
    assert_not_logged(&ev, "script_security_lowered");
}

#[test]
fn external_script_talk_is_mcp_send() {
    let (actions, ev) = run(vec![
        Input::SstpSend {
            script: SCRIPT.to_string(),
            strict: None,
        },
        translated(SecurityLevel::Local),
    ]);
    assert_eq!(started(&actions), ScriptOrigin::MCP_SEND, "要件 1.2");
    assert_not_logged(&ev, "script_security_lowered");
}

#[test]
fn external_reply_lowers_and_local_translate_reply_does_not_raise() {
    let (origin, ev) = ghost_talk(SecurityLevel::External, SecurityLevel::Local);
    assert_eq!(
        origin,
        ScriptOrigin::GHOST.lowered_by(SecurityLevel::External),
        "応答の external で下がり、翻訳の local で上がらない（要件 3.1・3.2）"
    );
    let hit = logged_once(&ev, Level::INFO, "script_security_lowered");
    assert_eq!(hit.fields.get("talk_id").map(String::as_str), Some("5"));
    assert_eq!(hit.fields.get("by").map(String::as_str), Some("response"));

    let (origin, ev) = notify_talk(SecurityLevel::External);
    assert_eq!(
        origin,
        ScriptOrigin::MCP_NOTIFY.lowered_by(SecurityLevel::External),
        "外から頼まれたイベントでも応答のレベルを重ねる"
    );
    logged_once(&ev, Level::INFO, "script_security_lowered");
}

#[test]
fn external_translate_reply_lowers() {
    let (origin, ev) = ghost_talk(SecurityLevel::Local, SecurityLevel::External);
    assert_eq!(
        origin,
        ScriptOrigin::GHOST.lowered_by(SecurityLevel::External),
        "翻訳の応答の external で下がる（要件 3.1）"
    );
    let hit = logged_once(&ev, Level::INFO, "script_security_lowered");
    assert_eq!(hit.fields.get("by").map(String::as_str), Some("translate"));
}

/// 最後に再生を始めたトークの控えは、翻訳の後の最終の出どころを写す（選択肢の `script:` の引き継ぎの源・要件 1.6）。
#[test]
fn shown_talk_keeps_the_final_origin_after_translation() {
    let cfg = cfg();
    let mut state = steady();
    for input in [
        Input::Tick {
            now: MonotonicMs(2_000),
        },
        reply(SecurityLevel::Local, "OnSecondChange"),
        translated(SecurityLevel::External),
    ] {
        state = step(state, input, &cfg).0;
    }
    assert_eq!(
        state.shown.map(|shown| (shown.talk_id.0, shown.origin)),
        Some((5, ScriptOrigin::GHOST.lowered_by(SecurityLevel::External)))
    );
}
