//! 翻訳の応答のレベルでトークの出どころを下げる（要件 1.5・3.1・3.2）。
//!
//! 応答の読み（[`read_reply`]）が翻訳の応答のレベルを運び、翻訳の結果の腕（[`on_done`]）が預けた
//! 再生の開始の出どころを下げる（上げない）こと、下がったときだけ `script_security_lowered` が
//! 1 件残ることを確かめる。

use tracing::Level;

use super::*;
use crate::msg::{EventId, KanadeConfig};
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
use crate::schedule::step;
use crate::talk::{ScriptOrigin, StartTalk};

const TALK: TalkId = TalkId(7);

fn source() -> ScriptSource {
    ScriptSource::Event(SourceEvent {
        id: EventId::Static("OnBoot"),
        references: Vec::new(),
    })
}

/// 出どころ `origin` の再生の開始を 1 つ預けた状態。
fn waiting(origin: ScriptOrigin) -> State {
    let mut start = StartTalk::new(TALK, "元");
    start.origin = origin;
    State {
        translate: Some(TranslateWait {
            talk_id: TALK,
            source: source(),
            deferred: vec![Action::StartTalk(start)],
        }),
        ..State::initial()
    }
}

/// 翻訳の結果（台詞 `訳`・レベル `level`）を入れ、返った再生の開始の出どころと記録を返す。
fn resume(
    origin: ScriptOrigin,
    level: SecurityLevel,
) -> (
    ScriptOrigin,
    Vec<crate::schedule::log_capture::CapturedEvent>,
) {
    let cfg = KanadeConfig::new("master", "1.0.0");
    let mut out = None;
    let ev = capture(|| {
        out = Some(step(
            waiting(origin),
            Input::TranslateDone(Ok(Translated {
                script: "訳".to_string(),
                level,
            })),
            &cfg,
        ))
    });
    let (_, actions) = out.expect("step は必ず値を返す");
    match actions.as_slice() {
        [Action::StartTalk(start)] => {
            assert_eq!(start.script, "訳", "台詞は差し替わる");
            (start.origin, ev)
        }
        _ => panic!("預けた再生の開始が 1 つ返るはず"),
    }
}

#[test]
fn read_reply_carries_the_translate_reply_level_and_falls_back_to_local() {
    let expanded = || "展開".to_string();
    let cases = [
        (
            ShioriOutcome::Value("訳".to_string(), SecurityLevel::External),
            SecurityLevel::External,
        ),
        (
            ShioriOutcome::Value("訳".to_string(), SecurityLevel::Local),
            SecurityLevel::Local,
        ),
        (ShioriOutcome::NoContent, SecurityLevel::Local),
        (
            ShioriOutcome::Failed(ShioriFailure::Shiori("500".to_string())),
            SecurityLevel::Local,
        ),
        (ShioriOutcome::Notified, SecurityLevel::Local),
    ];
    for (outcome, want) in cases {
        match read_reply(&source(), expanded(), outcome) {
            ReplyReading::Proceed(_, level) => assert_eq!(level, want),
            ReplyReading::Failed(failure) => panic!("進むはず: {failure}"),
        }
    }
}

#[test]
fn external_translate_reply_lowers_the_talk_and_logs_once() {
    let (origin, ev) = resume(ScriptOrigin::GHOST, SecurityLevel::External);
    assert_eq!(
        origin,
        ScriptOrigin::GHOST.lowered_by(SecurityLevel::External),
        "レベルだけが external に下がり、語と SSTP の別は元のまま"
    );
    let hit = logged_once(&ev, Level::INFO, "script_security_lowered");
    assert_eq!(hit.fields.get("talk_id").map(String::as_str), Some("7"));
    assert_eq!(hit.fields.get("by").map(String::as_str), Some("translate"));
}

#[test]
fn local_translate_reply_never_raises_and_logs_nothing() {
    let external = ScriptOrigin::MCP_SEND.lowered_by(SecurityLevel::External);
    for (before, want) in [
        (external, external),
        (ScriptOrigin::GHOST, ScriptOrigin::GHOST),
    ] {
        let (origin, ev) = resume(before, SecurityLevel::Local);
        assert_eq!(origin, want, "local の翻訳の応答では上がらず変わらない");
        assert_not_logged(&ev, "script_security_lowered");
    }
    let (origin, ev) = resume(external, SecurityLevel::External);
    assert_eq!(origin, external, "既に external なら変わらない");
    assert_not_logged(&ev, "script_security_lowered");
}
