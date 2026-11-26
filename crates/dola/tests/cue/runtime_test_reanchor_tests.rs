//! 起点の置き直しの檻: 前へ・後ろへ・取り出しごと・区切りの最中・今より後ろの起点。

use super::{BarrierKind, CuePlayer, CueSheet, at_ms, sheet_at_zero};
use crate::test_support::{barrier, logged_commands, ms, recording_sink, text};
use dola::cue::{CueCommand, OriginNotice, StepOutcome};
use dola::time::TimeDelta;

fn delta_ms(millis: i64) -> TimeDelta {
    TimeDelta::from_ticks(millis * 10_000)
}

fn t(s: &str) -> CueCommand {
    CueCommand::Text(s.into())
}

/// a(0) b(100) c(300) の 3 件。
fn sheet() -> CueSheet {
    sheet_at_zero(vec![
        text(ms(0), "a", ms(50)),
        text(ms(100), "b", ms(50)),
        text(ms(300), "c", ms(50)),
    ])
}

#[test]
fn moving_origin_earlier_delivers_later_cues_sooner() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    assert_eq!(logged_commands(&log), vec![t("a"), t("b")]);

    // 台本の 300 を今 (100) に当てる: c が今届く。
    let notice = player.reanchor(at_ms(100), at_ms(100), ms(300)).unwrap();
    assert_eq!(notice.origin, at_ms(-200));
    assert_eq!(notice.previous, Some(at_ms(0)));
    assert_eq!(notice.effective_at, at_ms(100));
    assert_eq!(notice.delta(), Some(delta_ms(-200)));
    player.tick(at_ms(100));
    assert_eq!(logged_commands(&log), vec![t("a"), t("b"), t("c")]);
}

#[test]
fn moving_origin_later_delays_remaining_cues() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    player.shift_origin(at_ms(100), delta_ms(500));
    player.tick(at_ms(300));
    assert_eq!(
        logged_commands(&log),
        vec![t("a"), t("b")],
        "300 では届かない"
    );
    player.tick(at_ms(799));
    assert_eq!(logged_commands(&log).len(), 2);
    player.tick(at_ms(800));
    assert_eq!(logged_commands(&log), vec![t("a"), t("b"), t("c")]);
}

#[test]
fn each_take_out_uses_the_origin_at_that_moment() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    assert_eq!(player.step(at_ms(100)), StepOutcome::Delivered);
    player.shift_origin(at_ms(100), delta_ms(-50));
    assert_eq!(player.step(at_ms(100)), StepOutcome::Delivered);
    assert_eq!(player.step(at_ms(100)), StepOutcome::Idle, "c は 250");

    let fires: Vec<_> = log.borrow().iter().map(|c| c.fire_time).collect();
    assert_eq!(fires, vec![at_ms(0), at_ms(50)]);
    assert_eq!(logged_commands(&log), vec![t("a"), t("b")]);
}

#[test]
fn reanchor_during_a_barrier_applies_after_it_is_resolved() {
    let mut player = CuePlayer::from_sheet(&sheet_at_zero(vec![
        text(ms(0), "a", ms(50)),
        barrier(ms(100), BarrierKind::WaitForInput),
        text(ms(300), "c", ms(50)),
    ]))
    .unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    assert_eq!(player.step(at_ms(100)), StepOutcome::Barrier);

    let notice = player.shift_origin(at_ms(150), delta_ms(1000)).unwrap();
    assert_eq!(notice.origin, at_ms(1000));
    assert_eq!(
        player.step(at_ms(1300)),
        StepOutcome::Barrier,
        "区切りは置き直しでは解けない"
    );

    player.resolve_click();
    player.tick(at_ms(1299));
    assert_eq!(logged_commands(&log), vec![t("a")]);
    player.tick(at_ms(1300));
    assert_eq!(logged_commands(&log), vec![t("a"), t("c")]);
}

#[test]
fn origin_later_than_now_delivers_nothing_until_it_arrives() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.reanchor(at_ms(0), at_ms(1000), ms(0));
    assert_eq!(player.origin(), at_ms(1000));
    player.tick(at_ms(999));
    assert!(log.borrow().is_empty());
    player.tick(at_ms(1000));
    assert_eq!(logged_commands(&log), vec![t("a")]);
}

#[test]
fn unchanged_origin_returns_none() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    assert_eq!(player.reanchor(at_ms(0), at_ms(0), ms(0)), None);
    assert_eq!(player.shift_origin(at_ms(0), delta_ms(0)), None);
    assert_eq!(player.origin(), at_ms(0));
}

#[test]
fn first_notice_has_no_previous() {
    let n = OriginNotice::first(at_ms(5));
    assert_eq!((n.previous, n.delta()), (None, None));
}
