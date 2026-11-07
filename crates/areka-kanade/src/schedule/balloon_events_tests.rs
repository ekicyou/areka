//! バルーンの 3 つのイベントの判断の檻（areka-P0-balloon-lifecycle-events）。
//!
//! どれも最上位の [`step`] に実際の入力を順に入れて駆動する（時刻は `Tick` で注入する）。
//! ここではトークの控え（`State.shown`）と中断の控え（`State.user_break_talk`）の出入りを固定する
//! ——翻訳の後の最終の台本が控えに入ること、切替の `OnClose` の別れの台詞も控えに入ること（要件 1.2・
//! 1.3・2.2・3.2）。表 B の檻は `balloon_events_done_tests.rs` に、置き換えで控えを捨てる檻は
//! `balloon_events_cleanup_tests.rs` に置き、ここの補助を使う。

use super::super::{Input, Phase, step};
use super::ShownTalk;
use super::done_tests::done;
use crate::msg::{MonotonicMs, ShioriOutcome};
use crate::schedule::translate_test_support::change_request;
use crate::schedule::{Action, State};
use crate::talk::{ScriptOrigin, TalkEndReason, TalkId};

pub(super) use crate::schedule::translate_test_support::config as cfg;

/// SHIORI が返す元の台本。
pub(super) const ORIGINAL: &str = "\\0元の台本\\e";
/// 翻訳の結果（最終の台本）。
pub(super) const FINAL: &str = "\\0最終の台本\\e";

/// `Steady{talk: None}`（終了の保留なし・控えなし・次の番号 5）。
pub(super) fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..State::initial()
    }
}

/// 入力を最上位の [`step`] に入れ、状態を進めて指示の列を返す。
pub(super) fn feed(s: &mut State, input: Input) -> Vec<Action> {
    let state = std::mem::replace(s, State::initial());
    let (state, actions) = step(state, input, &cfg());
    *s = state;
    actions
}

/// 台本の応答を入れる（出所は応答を待っている GET の ID）。
pub(super) fn reply_value(s: &mut State, script: &str, origin: &'static str) -> Vec<Action> {
    feed(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(script.to_string(), areka_talk::SecurityLevel::Local),
            origin,
        },
    )
}

pub(super) fn started(actions: &[Action]) -> (TalkId, &str) {
    actions
        .iter()
        .find_map(|a| match a {
            Action::StartTalk(t) => Some((t.talk_id, t.script.as_str())),
            _ => None,
        })
        .expect("再生の開始があるはず")
}

/// `origin` の応答で `ORIGINAL` を返し、翻訳の結果 `final_script` で再生を始める。
pub(super) fn reply_and_start(s: &mut State, origin: &'static str, final_script: &str) -> TalkId {
    let actions = reply_value(s, ORIGINAL, origin);
    assert!(
        matches!(actions.as_slice(), [Action::Translate(_)]),
        "台本の応答は翻訳へ預けられる"
    );
    let actions = feed(
        s,
        Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(
            final_script,
        ))),
    );
    started(&actions).0
}

/// 毎秒のポンプの応答で `ORIGINAL` を返し、翻訳の結果 `final_script` で再生を始める。
pub(super) fn start_translated(s: &mut State, now: u64, final_script: &str) -> TalkId {
    feed(
        s,
        Input::Tick {
            now: MonotonicMs(now),
        },
    );
    reply_and_start(s, "OnSecondChange", final_script)
}

/// 定常からゴーストの切替を受理する（`OnGhostChanging` の応答待ちへ）。
pub(super) fn accept_change(s: &mut State) {
    feed(s, Input::ChangeGhost(change_request()));
}

/// 再生中のトークを自分で最後まで流れて終わらせる。
pub(super) fn finish(s: &mut State, talk: TalkId) {
    feed(s, done(talk, TalkEndReason::Ended, false));
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "前提: 再生中のトークの無い定常"
    );
}

/// 定常でトークを再生し終え、読み終えたバルーンが出ている状態（控えはそのトーク）。
pub(super) fn steady_after_talk() -> (State, TalkId) {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    finish(&mut s, talk);
    (s, talk)
}

/// 翻訳の後の最終の台本が控えに入り、送り済み・預かりの印は新しいトークで下りる（要件 1.2・2.2・3.2）。
#[test]
fn shown_talk_keeps_the_final_script_after_translation() {
    let mut s = steady_none();
    // 前のトークの印が立っていても、新しいトークの開始で丸ごと置き換わる。
    s.shown = Some(ShownTalk {
        talk_id: TalkId(4),
        script: "\\0前のトーク\\e".to_string(),
        origin: ScriptOrigin::GHOST,
        close_sent: true,
        timeout_sent: true,
        timeout_pending: false,
    });

    feed(
        &mut s,
        Input::Tick {
            now: MonotonicMs(2_000),
        },
    );
    let actions = reply_value(&mut s, ORIGINAL, "OnSecondChange");
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    assert_eq!(
        s.shown.as_ref().map(|shown| shown.talk_id),
        Some(TalkId(4)),
        "翻訳へ預けている間は再生が始まっていないので、控えは前のトークのまま"
    );

    let actions = feed(
        &mut s,
        Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(FINAL))),
    );
    let (talk_id, _) = started(&actions);
    assert_eq!(
        s.shown,
        Some(ShownTalk {
            talk_id,
            script: FINAL.to_string(),
            origin: ScriptOrigin::GHOST,
            close_sent: false,
            timeout_sent: false,
            timeout_pending: false,
        }),
        "控えは再生を始めたトークの番号と翻訳の後の最終の台本（印はすべて下りる）"
    );
}

/// 切替の `OnClose` の別れの台詞（`ChangeCloseTalkWait`）も控えに入る（設計 D2）。
#[test]
fn shown_talk_keeps_the_change_farewell_script() {
    let mut s = steady_none();
    accept_change(&mut s);
    feed(
        &mut s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnGhostChanging",
        },
    );
    assert!(matches!(s.phase, Phase::ChangeClosePending));
    let talk_id = reply_and_start(&mut s, "OnClose", FINAL);
    assert!(
        matches!(s.phase, Phase::ChangeCloseTalkWait { talk_id: t, .. } if t == talk_id),
        "別れの台詞の完了待ちへ進む"
    );
    let shown = s.shown.as_ref().expect("別れの台詞も控えに入る");
    assert_eq!((shown.talk_id, shown.script.as_str()), (talk_id, FINAL));
}
