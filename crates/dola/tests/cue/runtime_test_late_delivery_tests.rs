//! 遅れた台本と区切りを解く口の檻: 起点が過ぎた台本・起点が来ていない台本・解くだけでは配らず完了にならない。

use super::{BarrierKind, CueCommand, CuePlayer, CuePlayerState, CueSheet, at_ms, sheet_at_zero};
use crate::test_support::{barrier, choice, logged_commands, ms, recording_sink, text};
use dola::time::TimePoint;

fn t(s: &str) -> CueCommand {
    CueCommand::Text(s.into())
}

fn sheet_at(origin: TimePoint) -> CueSheet {
    CueSheet::new(vec![
        text(ms(0), "a", ms(50)),
        text(ms(300), "b", ms(50)),
        text(ms(900), "c", ms(50)),
    ])
    .with_absolute_start_time(origin)
}

#[test]
fn sheet_whose_origin_is_past_delivers_all_past_cues_in_order_with_own_fire_times() {
    let mut player = CuePlayer::from_sheet(&sheet_at(at_ms(-1000))).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(0));

    assert_eq!(logged_commands(&log), vec![t("a"), t("b"), t("c")]);
    let fires: Vec<_> = log.borrow().iter().map(|c| c.fire_time).collect();
    assert_eq!(fires, vec![at_ms(-1000), at_ms(-700), at_ms(-100)]);
    assert!(player.is_completed(), "占有の終わり (-50) も過ぎている");
}

#[test]
fn sheet_whose_origin_has_not_arrived_delivers_nothing() {
    let mut player = CuePlayer::from_sheet(&sheet_at(at_ms(5000))).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(4999));
    assert!(log.borrow().is_empty());
    assert_eq!(player.state(), &CuePlayerState::Playing);

    player.tick(at_ms(5000));
    assert_eq!(logged_commands(&log), vec![t("a")]);
}

fn barrier_sheet(kind: BarrierKind) -> CueSheet {
    let mut cues = vec![text(ms(0), "a", ms(0))];
    if matches!(kind, BarrierKind::WaitForChoice { .. }) {
        cues.push(choice(ms(50), "x", "X"));
    }
    cues.push(barrier(ms(100), kind));
    cues.push(text(ms(300), "c", ms(50)));
    sheet_at_zero(cues)
}

#[test]
fn resolve_click_alone_delivers_nothing_and_does_not_complete_then_tick_does_both() {
    let mut player = CuePlayer::from_sheet(&barrier_sheet(BarrierKind::WaitForInput)).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(10_000));
    assert_eq!(player.state(), &CuePlayerState::WaitingForInput);
    assert_eq!(logged_commands(&log), vec![t("a")]);

    player.resolve_click();
    assert_eq!(
        logged_commands(&log),
        vec![t("a")],
        "解いただけでは配らない"
    );
    assert!(!player.is_completed());
    assert_eq!(player.state(), &CuePlayerState::Playing);

    player.tick(at_ms(10_000));
    assert_eq!(logged_commands(&log), vec![t("a"), t("c")]);
    assert!(player.is_completed(), "占有の終わり (350) を過ぎている");
}

#[test]
fn tick_right_after_resolve_before_the_occupancy_end_delivers_but_does_not_complete() {
    let mut player = CuePlayer::from_sheet(&barrier_sheet(BarrierKind::WaitForInput)).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);
    player.tick(at_ms(150));

    player.resolve_click();
    player.tick(at_ms(300));
    assert_eq!(logged_commands(&log), vec![t("a"), t("c")]);
    assert!(!player.is_completed(), "350 までは占有中");

    player.tick(at_ms(350));
    assert!(player.is_completed());
}

#[test]
fn resolve_choice_alone_delivers_nothing_and_does_not_complete_then_tick_does_both() {
    let mut player =
        CuePlayer::from_sheet(&barrier_sheet(BarrierKind::WaitForChoice { timeout: None }))
            .unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(10_000));
    assert_eq!(player.state(), &CuePlayerState::WaitingForChoice);
    let before = logged_commands(&log);

    assert_eq!(player.resolve_choice("x"), Some("x".to_string()));
    assert_eq!(logged_commands(&log), before, "解いただけでは配らない");
    assert!(!player.is_completed());

    player.tick(at_ms(10_000));
    let mut expected = before;
    expected.push(t("c"));
    assert_eq!(logged_commands(&log), expected);
    assert!(player.is_completed());
}
