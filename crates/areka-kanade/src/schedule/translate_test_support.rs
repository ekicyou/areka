//! 既存テスト用の補助（テスト専用）: 翻訳の行動が出たら、結果を台詞そのままで入れ直して続きを返す。
//!
//! 出口の規則が SHIORI の台詞の再生開始を `[Action::Translate]` に替えた後も、既存の `step` の
//! テストが「`OnTranslate` が 204 を返した」ときの続き（預けた一括）を確かめられるようにする。
//! あわせて、SSTP・切れ目・翻訳の経路のテストが共に使う下ごしらえを 1 つずつ置く。

use super::log_capture::CapturedEvent;
use super::sstp::BOOT_GREETING_ORIGIN;
use super::{
    Action, ActiveTalk, Input, Phase, State, TermCause, current_talk_id, phase_label, step,
};
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget};
use crate::msg::{KanadeConfig, MonotonicMs, ShioriOutcome};
use crate::talk::TalkId;
use tracing::Level;

/// `step` の返り値の一括が翻訳の行動 1 つだけなら、結果を台詞そのまま（`Ok(script)`）で入れ直して
/// 続きを返す。それ以外の一括はそのまま返す。
pub(crate) fn pass_translate(
    (state, actions): (State, Vec<Action>),
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    match actions.as_slice() {
        [Action::Translate(request)] => {
            let script = request.script.clone();
            step(
                state,
                Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(script))),
                config,
            )
        }
        _ => (state, actions),
    }
}

// ============================================================
// SSTP・切れ目・翻訳の経路のテストが共に使う下ごしらえ
// ============================================================

pub(crate) fn strings(refs: &[&str]) -> Vec<String> {
    refs.iter().map(|r| r.to_string()).collect()
}

pub(crate) fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

pub(crate) fn active(talk_id: u64, origin: &'static str) -> Option<ActiveTalk> {
    Some(ActiveTalk {
        talk_id: TalkId(talk_id),
        origin,
        script: "\\h流れている台本\\e".to_string(),
    })
}

pub(crate) fn steady(talk: Option<ActiveTalk>) -> State {
    State {
        phase: Phase::Steady { talk },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 10,
        ..State::initial()
    }
}

/// `Steady{talk: None}`（時刻 1,000・次の番号 5・保留と控えなし）。ここから実際の入力で前提へ進める。
pub(crate) fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..State::initial()
    }
}

/// 定常でない相の状態を 1 つずつ（次の番号 10・ほかは初期値）。外からの入口が受け付けない表の行。
pub(crate) fn non_steady_states() -> Vec<(&'static str, State)> {
    let deadline = Some(MonotonicMs(9_000));
    let talk_id = TalkId(9);
    [
        ("Idle", Phase::Idle),
        ("BootInit", Phase::BootInit),
        ("BootPrefetch", Phase::BootPrefetch),
        ("BootType", Phase::BootType),
        ("BootMain", Phase::BootMain),
        ("BootVersion{None}", Phase::BootVersion { talk: None }),
        (
            "BootVersion{Some}",
            Phase::BootVersion {
                talk: active(1, BOOT_GREETING_ORIGIN),
            },
        ),
        ("ClosePending", Phase::ClosePending),
        ("CloseTalkWait", Phase::CloseTalkWait { talk_id, deadline }),
        ("ChangePending", Phase::ChangePending),
        (
            "ChangeTalkWait",
            Phase::ChangeTalkWait { talk_id, deadline },
        ),
        ("ChangeClosePending", Phase::ChangeClosePending),
        (
            "ChangeCloseTalkWait",
            Phase::ChangeCloseTalkWait { talk_id, deadline },
        ),
        (
            "Unloading",
            Phase::Unloading {
                cause: TermCause::Quit,
            },
        ),
        ("Stopped", Phase::Stopped),
    ]
    .into_iter()
    .map(|(label, phase)| {
        (
            label,
            State {
                phase,
                next_talk_id: 10,
                ..State::initial()
            },
        )
    })
    .collect()
}

pub(crate) fn change_request() -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: r"C:\areka\ghost\r_post".to_string(),
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    }
}

/// 状態の見比べの指紋（`State` は `PartialEq` を持たないので、腕が触り得る欄を綴りにする）。
pub(crate) fn fingerprint(s: &State) -> String {
    format!(
        "phase={} talk={:?} next={} choice={} prev={:?} close={} change={} translate={} notify={:?} reply={}",
        phase_label(&s.phase),
        current_talk_id(&s.phase).map(|t| t.0),
        s.next_talk_id,
        s.choice.is_some(),
        s.prev_talk.map(|t| t.0),
        s.pending_close.is_some(),
        s.pending_change.is_some(),
        s.translate.is_some(),
        s.sstp_notify,
        s.reply_source.is_some(),
    )
}

pub(crate) fn warn_or_above(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == Level::WARN || e.level == Level::ERROR)
        .collect()
}

/// 起動の手順を実の `step` でたどり、起動の挨拶が流れている `Steady` まで進める。
pub(crate) fn boot_until_greeting_playing(cfg: &KanadeConfig) -> State {
    let reply = |s, outcome| {
        step(
            s,
            Input::ShioriReply {
                outcome,
                origin: "test",
            },
            cfg,
        )
        .0
    };
    let (s, _) = step(State::initial(), Input::Boot, cfg);
    let s = reply(s, ShioriOutcome::Notified); // BootInit → BootPrefetch
    let s = reply(s, ShioriOutcome::NoContent); // BootPrefetch → BootType
    let s = reply(
        s,
        ShioriOutcome::Value("\\h挨拶\\e".to_string(), areka_talk::SecurityLevel::Local),
    ); // → BootVersion{挨拶}
    let (s, _) = step(
        s,
        Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(
            "\\h挨拶\\e",
        ))),
        cfg,
    );
    let s = reply(s, ShioriOutcome::Notified); // → Steady{挨拶}
    assert!(
        matches!(&s.phase, Phase::Steady { talk: Some(t) } if t.origin == BOOT_GREETING_ORIGIN),
        "挨拶が流れている Steady のはず"
    );
    s
}
