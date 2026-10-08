//! 時間切れの知らせ（表 D）の檻（areka-P0-balloon-lifecycle-events タスク 2.5）。
//!
//! 最上位の `step` に時間切れの知らせ（`Input::BalloonTimeout`）を入れて駆動する。定常でない相・
//! 終了の保留は本番の流れで作ると長くなるので、トークを 1 つ再生し終えた控えを持つ状態の相だけを
//! 差し替える。

use super::super::log_capture::{assert_not_logged, logged_once};
use super::super::{Input, Phase, TermCause};
use super::done_tests::{
    assert_no_balloon_event, assert_not_sent, done, feed_capturing, field, gets,
};
use super::tests::{FINAL, ORIGINAL, feed, reply_value, start_translated, steady_none};
use crate::msg::{CloseReason, MonotonicMs};
use crate::schedule::{Action, State};
use crate::talk::{TalkEndReason, TalkId};
use tracing::Level;

/// 時間切れの知らせ。
fn timeout(talk_id: TalkId) -> Input {
    Input::BalloonTimeout { talk_id }
}

/// 再生中のトークを自分で最後まで流れて終わらせる。
fn finish(s: &mut State, talk: TalkId) {
    feed(s, done(talk, TalkEndReason::Ended, false));
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "前提: 再生中のトークの無い定常"
    );
}

/// 定常でトークを再生し終え、読み終えたバルーンが出ている状態（控えはそのトーク）。
fn steady_after_talk() -> (State, TalkId) {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    finish(&mut s, talk);
    (s, talk)
}

/// 指示の列の GET が `OnBalloonTimeout`（Reference0＝控えの台本・Reference1＝`0`）1 本だけ。
fn assert_one_timeout_get(actions: &[Action]) {
    assert_eq!(
        gets(actions),
        vec![(
            "OnBalloonTimeout",
            &[FINAL.to_string(), "0".to_string()][..]
        )]
    );
}

/// D6: 番号が控えと合い、再生中のトークの無い定常なら `OnBalloonTimeout` を 1 本送り、送り済みの
/// 印を立てて記録する（要件 2.1〜2.3・7.1）。
/// D4: 同じ番号の 2 通目は送らず、理由を記録する（要件 4.5）。
#[test]
fn d6_matching_notice_sends_one_balloon_timeout_then_d4_refuses_the_second() {
    let (mut s, talk) = steady_after_talk();
    let (actions, ev) = feed_capturing(&mut s, timeout(talk));
    assert_one_timeout_get(&actions);
    assert_eq!(actions.len(), 1, "指示は GET 1 本だけ");
    assert!(s.shown.as_ref().is_some_and(|shown| shown.timeout_sent));
    let line = logged_once(&ev, Level::INFO, "balloon_event_sent");
    assert_eq!(
        (
            field(line, "id"),
            field(line, "cause"),
            field(line, "talk_id")
        ),
        (
            Some("OnBalloonTimeout"),
            Some("timeout"),
            Some(talk.0.to_string().as_str())
        ),
        "イベント名・きっかけ・トークの番号を 1 行で記録する（要件 7.1）。\n捕捉={ev:#?}"
    );
    assert_not_logged(&ev, "balloon_event_not_sent");

    let (actions, ev) = feed_capturing(&mut s, timeout(talk));
    assert!(actions.is_empty(), "2 通目は何も送らない");
    assert_not_sent(&ev, "OnBalloonTimeout", "already_sent", "Steady");
}

/// D5: 完了の知らせより先に届いた知らせは預かって記録し、何も送らない。同じトークの完了で
/// 表 B の B5 が `OnBalloonTimeout` を送る（要件 2.1・4.5）。預かり中の 2 通目は D4 で止まる。
#[test]
fn d5_notice_before_completion_is_held_and_sent_on_completion() {
    let mut s = steady_none();
    let talk = start_translated(&mut s, 2_000, FINAL);
    let (actions, ev) = feed_capturing(&mut s, timeout(talk));
    assert!(actions.is_empty(), "再生中は送らない");
    assert!(s.shown.as_ref().is_some_and(|shown| shown.timeout_pending));
    let line = logged_once(&ev, Level::INFO, "balloon_timeout_deferred");
    assert_eq!(
        field(line, "talk_id"),
        Some(talk.0.to_string().as_str()),
        "預かったトークの番号を記録する。\n捕捉={ev:#?}"
    );
    assert_not_logged(&ev, "balloon_event_not_sent");

    let (actions, ev) = feed_capturing(&mut s, timeout(talk));
    assert!(actions.is_empty(), "預かり中の 2 通目は何も送らない");
    assert_not_sent(&ev, "OnBalloonTimeout", "already_deferred", "Steady");

    let actions = feed(&mut s, done(talk, TalkEndReason::Ended, false));
    assert_one_timeout_get(&actions);
    let shown = s.shown.as_ref().expect("控えは残る");
    assert!(shown.timeout_sent && !shown.timeout_pending);
}

/// D2: 次のトークが再生中なら、前のトークの知らせは送らず「次のトークが既に始まっていた」と
/// 記録する。預かりもしない（要件 2.6）。
#[test]
fn d2_next_talk_playing_refuses_the_old_notice() {
    let (mut s, old) = steady_after_talk();
    let next = start_translated(&mut s, 3_000, FINAL);
    assert_ne!(next, old);
    let (actions, ev) = feed_capturing(&mut s, timeout(old));
    assert!(actions.is_empty());
    assert_not_sent(&ev, "OnBalloonTimeout", "next_talk_started", "Steady");
    let line = logged_once(&ev, Level::INFO, "balloon_event_not_sent");
    assert_eq!(field(line, "talk_id"), Some(old.0.to_string().as_str()));
    let shown = s.shown.as_ref().expect("控えは次のトーク");
    assert!(shown.talk_id == next && !shown.timeout_pending && !shown.timeout_sent);

    // 次のトークの完了でも、前のトークの知らせは送られない。
    let actions = feed(&mut s, done(next, TalkEndReason::Ended, false));
    assert_no_balloon_event(&actions, "次のトークの完了");
}

/// D2: 次のトークの台本を翻訳へ預けている間（相は次のトークを再生中・控えは前のトークのまま）に
/// 届いた前のトークの知らせも、次のトークが既に始まっていたとして捨てる。預かると、出口の掃除が
/// `talk_gone` と取り違えて捨て直すことになる（要件 2.6）。
#[test]
fn d2_notice_while_next_talk_awaits_translation_is_refused_not_held() {
    let (mut s, old) = steady_after_talk();
    feed(
        &mut s,
        Input::Tick {
            now: MonotonicMs(3_000),
        },
    );
    let actions = reply_value(&mut s, ORIGINAL, "OnSecondChange");
    assert!(
        matches!(actions.as_slice(), [Action::Translate(_)]),
        "前提: 次のトークの台本は翻訳へ預けられている"
    );
    assert!(matches!(s.phase, Phase::Steady { talk: Some(_) }));
    let (actions, ev) = feed_capturing(&mut s, timeout(old));
    assert!(actions.is_empty());
    assert_not_sent(&ev, "OnBalloonTimeout", "next_talk_started", "Steady");
    assert_not_logged(&ev, "balloon_timeout_deferred");
    assert!(
        ev.iter()
            .all(|e| e.fields.get("reason").map(String::as_str) != Some("talk_gone")),
        "預からないので掃除の記録も出ない。\n捕捉={ev:#?}"
    );
    assert!(s.shown.as_ref().is_some_and(|shown| !shown.timeout_pending));
}

/// D2: 次のトークが始まって終わっていたら、前のトークの知らせは送らない（要件 2.6）。
/// 控えが無いときも同じ行に落ちる。
#[test]
fn d2_next_talk_started_and_ended_refuses_the_old_notice() {
    let (mut s, old) = steady_after_talk();
    let next = start_translated(&mut s, 3_000, FINAL);
    finish(&mut s, next);
    let (actions, ev) = feed_capturing(&mut s, timeout(old));
    assert!(actions.is_empty());
    assert_not_sent(&ev, "OnBalloonTimeout", "next_talk_started", "Steady");
    assert!(s.shown.as_ref().is_some_and(|shown| !shown.timeout_sent));

    let mut s = steady_none();
    let (actions, ev) = feed_capturing(&mut s, timeout(TalkId(3)));
    assert!(actions.is_empty(), "控えが無い");
    assert_not_sent(&ev, "OnBalloonTimeout", "next_talk_started", "Steady");
}

/// 残る行き違いその 1: cue を 1 つも出さないトーク（`\e` だけ）を挟むと、表示の側は前のトークの
/// 番号のまま時間切れを知らせ、kanade の控えは進んでいるので D2 で捨てる（設計の記録して許す行）。
#[test]
fn d2_notice_one_behind_a_cueless_talk_is_dropped() {
    let (mut s, old) = steady_after_talk();
    let cueless = start_translated(&mut s, 3_000, "\\e");
    finish(&mut s, cueless);
    let (actions, ev) = feed_capturing(&mut s, timeout(old));
    assert!(actions.is_empty(), "OnBalloonTimeout は出ない");
    assert_not_sent(&ev, "OnBalloonTimeout", "next_talk_started", "Steady");
}

/// D1: 相が定常でなければ送らず、理由と相を記録する。印も預かりも立てない（要件 2.6・4.4・7.2）。
#[test]
fn d1_not_steady_phases_send_nothing_and_record_why() {
    let phases = [
        ("BootVersion", Phase::BootVersion { talk: None }),
        (
            "ClosePending",
            Phase::ClosePending {
                reason: CloseReason::System,
            },
        ),
        ("ChangePending", Phase::ChangePending),
        ("ChangeClosePending", Phase::ChangeClosePending),
        (
            "Unloading",
            Phase::Unloading {
                cause: TermCause::Quit,
            },
        ),
        ("Stopped", Phase::Stopped),
    ];
    for (label, phase) in phases {
        let (mut s, talk) = steady_after_talk();
        s.phase = phase;
        let (actions, ev) = feed_capturing(&mut s, timeout(talk));
        assert!(actions.is_empty(), "{label}: 何も送らない");
        assert_not_sent(&ev, "OnBalloonTimeout", "not_steady", label);
        assert!(
            s.shown
                .as_ref()
                .is_some_and(|shown| !shown.timeout_sent && !shown.timeout_pending),
            "{label}: 印も預かりも立てない"
        );
    }
}

/// D3: 終了の要求を保留している間は送らず、理由を記録する。
#[test]
fn d3_close_pending_sends_nothing_and_records_why() {
    let (mut s, talk) = steady_after_talk();
    s.pending_close = Some(CloseReason::System);
    let (actions, ev) = feed_capturing(&mut s, timeout(talk));
    assert_no_balloon_event(&actions, "終了の保留中");
    assert_not_sent(&ev, "OnBalloonTimeout", "close_pending", "Steady");
    assert!(s.shown.as_ref().is_some_and(|shown| !shown.timeout_sent));
}
