//! 寿命の知らせの列と、絶対の時刻で下す寿命の判断の決定論テスト。
//!
//! 線から取り出した知らせを列へ足し、フレームの時刻が来た物を判断へ渡す所から、判断が持つ
//! 表示の終わりと期限までを通す。表示層も文字の層も使わない。

use areka_sakura::TalkId;
use areka_sakura::held::HeldQueue;
use dola::TimePoint;
use dola::cue::OriginNotice;
use dola::time::testing::{at_ms, ms};

use super::super::test_support::{TIMEOUT, hide, observations, seen};
use super::super::{
    BalloonVisibilityState, TalkLifecycleSignal, VisibilityDecision, VisibilityObservations,
    VisibilityTrigger, decide,
};
use super::{head_wait, take_due};

const TALK: TalkId = TalkId(9);

/// 可視性の相と同じ順（列へ足す → 時刻の来た物を取り出す → 判断）で 1 フレームを回す。
/// scope 0 のバルーンは見えていて、文字が 1 つ出ている。
#[derive(Default)]
struct Rig {
    state: BalloonVisibilityState,
    held: HeldQueue<TalkLifecycleSignal>,
}

impl Rig {
    fn frame(&mut self, arrived: Vec<TalkLifecycleSignal>, now: TimePoint) -> VisibilityDecision {
        let lifecycle = take_due(&mut self.held, arrived, Some(now));
        let obs = VisibilityObservations {
            lifecycle,
            ..observations(&[(0, seen(1, true))])
        };
        decide(&mut self.state, &obs, Some(now), TIMEOUT)
    }
}

fn started(at: i64) -> TalkLifecycleSignal {
    TalkLifecycleSignal::TalkStarted {
        at: at_ms(at),
        talk_id: Some(TALK),
    }
}

fn display_end(at: i64, end: i64) -> TalkLifecycleSignal {
    TalkLifecycleSignal::DisplayEndAt {
        at: at_ms(at),
        end: at_ms(end),
    }
}

fn ended(at: i64) -> TalkLifecycleSignal {
    TalkLifecycleSignal::TalkEnded { at: at_ms(at) }
}

fn moved(effective: i64, origin: i64, previous: i64) -> TalkLifecycleSignal {
    TalkLifecycleSignal::OriginMoved(OriginNotice {
        effective_at: at_ms(effective),
        origin: at_ms(origin),
        previous: Some(at_ms(previous)),
    })
}

/// 知らせが効くのは、フレームの時刻がその知らせの時刻に届いた最初のフレームで、その前のフレームでは効かない。
#[test]
fn notice_takes_effect_on_the_first_frame_that_reaches_its_time() {
    let mut rig = Rig::default();

    rig.frame(vec![started(1_000)], at_ms(999));
    assert_eq!(
        rig.state.talk_id(),
        None,
        "知らせの時刻の前のフレームでは効かない"
    );

    rig.frame(vec![], at_ms(1_000));
    assert_eq!(
        rig.state.talk_id(),
        Some(TALK),
        "時刻に届いたフレームで効く"
    );
}

/// 列の頭の物が待つ間は、時刻の来ている後ろの物も待ち、頭の時刻が来たフレームで届いた順に当たる。
#[test]
fn later_notices_wait_behind_the_head_and_apply_in_arrival_order() {
    let mut rig = Rig::default();

    let held_back = rig.frame(
        vec![
            started(1_000),
            TalkLifecycleSignal::UserBreak { at: at_ms(500) },
        ],
        at_ms(600),
    );
    assert_eq!(
        held_back.actions,
        vec![],
        "頭の開始が待つ間は、中断も当たらない"
    );

    let applied = rig.frame(vec![], at_ms(1_000));
    assert_eq!(
        applied.actions,
        vec![hide(&[0], VisibilityTrigger::UserBreak)]
    );
    assert_eq!(rig.state.talk_id(), Some(TALK));
    assert!(
        rig.state.break_latch,
        "開始の後に中断が当たった（届いた順）"
    );
}

/// 利用者の中断は、押下を受けた時の時計の読みを運び、それを取り出したフレームで効く。
#[test]
fn user_break_takes_effect_on_the_frame_that_takes_it() {
    let mut rig = Rig::default();
    let decision = rig.frame(
        vec![TalkLifecycleSignal::UserBreak { at: at_ms(5_000) }],
        at_ms(5_000),
    );
    assert_eq!(
        decision.actions,
        vec![hide(&[0], VisibilityTrigger::UserBreak)]
    );
}

/// 表示の終わり・トークの終わり・期限は絶対の時刻で、期限はフレームの時刻と比べて切れる。
/// トークの終わりの知らせ（止まった時刻 1500）は、その時刻まで効かない。
#[test]
fn display_end_and_deadline_are_absolute_and_expire_against_the_frame_time() {
    let mut rig = Rig::default();

    rig.frame(
        vec![started(1_000), display_end(1_000, 2_000), ended(1_500)],
        at_ms(1_200),
    );
    assert_eq!(rig.state.display_end, Some(at_ms(2_000)));
    assert_eq!(
        rig.state.talk_end, None,
        "止まった時刻の前は、まだ見ていない"
    );
    assert_eq!(rig.state.deadline, None);

    rig.frame(vec![], at_ms(1_500));
    assert_eq!(rig.state.talk_end, Some(at_ms(1_500)));
    assert_eq!(
        rig.state.deadline,
        Some(at_ms(1_500) + TIMEOUT),
        "期限は、表示の終わりと止まった時刻の早い方＋待ち"
    );

    let before = rig.frame(vec![], at_ms(1_500) + TIMEOUT - ms(1));
    assert_eq!(before.actions, vec![]);
    let expired = rig.frame(vec![], at_ms(1_500) + TIMEOUT);
    assert_eq!(
        expired.actions,
        vec![hide(&[0], VisibilityTrigger::Timeout)]
    );
    assert_eq!(expired.timeout_notice, Some(TALK));
}

/// 置き直しの知らせで、効く時刻より後の表示の終わりだけが動く（後ろへは動いた長さだけ・前へは
/// 効く時刻より前にならない）。効く時刻以前の表示の終わりは動かない。
#[test]
fn re_origin_moves_only_a_display_end_after_the_effective_time() {
    let mut later = Rig::default();
    later.frame(
        vec![started(0), display_end(0, 2_000), moved(1_500, 1_000, 0)],
        at_ms(1_500),
    );
    assert_eq!(later.state.display_end, Some(at_ms(3_000)));

    let mut earlier = Rig::default();
    earlier.frame(
        vec![started(0), display_end(0, 2_000), moved(1_500, -1_000, 0)],
        at_ms(1_500),
    );
    assert_eq!(
        earlier.state.display_end,
        Some(at_ms(1_500)),
        "前へ動いても効く時刻に寄る"
    );

    let mut untouched = Rig::default();
    untouched.frame(
        vec![started(0), display_end(0, 1_000), moved(1_500, 1_000, 0)],
        at_ms(1_500),
    );
    assert_eq!(untouched.state.display_end, Some(at_ms(1_000)));
}

/// 置き直しの知らせは期限を動かさない（期限はトークの終わりの後の表示の終わりから作った物）。
#[test]
fn re_origin_does_not_move_the_deadline() {
    let mut rig = Rig::default();
    rig.frame(
        vec![started(0), display_end(0, 2_000), ended(2_000)],
        at_ms(2_000),
    );
    let deadline = rig.state.deadline;
    assert_eq!(deadline, Some(at_ms(2_000) + TIMEOUT));

    rig.frame(vec![moved(1_000, 1_000, 0)], at_ms(2_500));

    assert_eq!(rig.state.display_end, Some(at_ms(3_000)));
    assert_eq!(rig.state.deadline, deadline);
}

/// 線から取り出した物は、フレームの時刻が無ければ列に残る（判断へ渡さない）。
#[test]
fn nothing_is_taken_without_a_frame_time() {
    let mut held = HeldQueue::new();
    assert_eq!(take_due(&mut held, vec![started(0)], None), vec![]);
    assert_eq!(
        take_due(&mut held, vec![], Some(at_ms(0))),
        vec![started(0)]
    );
}

/// 起こしの予約に入れる長さは、列の頭の物が待っている時刻までの残りで、待つ物が無ければ無し。
#[test]
fn head_wait_is_the_time_left_until_the_head() {
    let mut held = HeldQueue::new();
    assert_eq!(head_wait(&held, Some(at_ms(0))), None, "列が空");

    take_due(&mut held, vec![started(1_000)], Some(at_ms(900)));
    assert_eq!(head_wait(&held, Some(at_ms(900))), Some(ms(100)));
    assert_eq!(head_wait(&held, None), None, "フレームの時刻が無い");

    take_due(&mut held, vec![], Some(at_ms(1_000)));
    assert_eq!(head_wait(&held, Some(at_ms(1_000))), None, "取り出した後");
}
