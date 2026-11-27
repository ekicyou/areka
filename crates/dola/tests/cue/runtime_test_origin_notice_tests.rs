//! 起点の知らせの檻: 登録の時の最初の起点・合図との順・効く時刻・変わらない置き直し・上書きしない受け手。

use super::{CuePlayer, CueSheet, CueSink, TalkCue, at_ms, sheet_at_zero};
use crate::test_support::{logged_commands, ms, recording_sink, text};
use dola::cue::{CueCommand, OriginNotice};
use dola::time::{TimeDelta, TimePoint};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
enum Event {
    Origin(OriginNotice),
    Cue(TimePoint, CueCommand),
    Preview(Vec<(TimePoint, CueCommand)>),
}

type EventLog = Rc<RefCell<Vec<Event>>>;

struct NoticeSink(EventLog);

impl CueSink for NoticeSink {
    fn emit(&mut self, cue: TalkCue) {
        self.0
            .borrow_mut()
            .push(Event::Cue(cue.fire_time, cue.command));
    }

    fn preview(&mut self, upcoming: &[TalkCue]) {
        let cues = upcoming
            .iter()
            .map(|cue| (cue.fire_time, cue.command.clone()))
            .collect();
        self.0.borrow_mut().push(Event::Preview(cues));
    }

    fn origin_set(&mut self, notice: OriginNotice) {
        self.0.borrow_mut().push(Event::Origin(notice));
    }
}

fn notice_sink() -> (EventLog, Box<dyn CueSink>) {
    let log: EventLog = Rc::default();
    (log.clone(), Box::new(NoticeSink(log)))
}

fn t(s: &str) -> CueCommand {
    CueCommand::Text(s.into())
}

fn delta_ms(millis: i64) -> TimeDelta {
    TimeDelta::from_ticks(millis * 10_000)
}

fn preview(cues: &[(i64, &str)]) -> Event {
    Event::Preview(cues.iter().map(|&(at, s)| (at_ms(at), t(s))).collect())
}

fn first_and_preview() -> [Event; 2] {
    [
        Event::Origin(OriginNotice::first(at_ms(0))),
        preview(&[(0, "a"), (100, "b"), (300, "c")]),
    ]
}

/// a(0) b(100) c(300) の 3 件・起点 0。
fn sheet() -> CueSheet {
    sheet_at_zero(vec![
        text(ms(0), "a", ms(50)),
        text(ms(100), "b", ms(50)),
        text(ms(300), "c", ms(50)),
    ])
}

#[test]
fn first_origin_is_delivered_at_registration_before_any_cue() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = notice_sink();
    player.register_sink(sink);
    assert_eq!(*log.borrow(), first_and_preview());

    player.tick(at_ms(0));
    let [first, upcoming] = first_and_preview();
    assert_eq!(
        *log.borrow(),
        vec![first, upcoming, Event::Cue(at_ms(0), t("a"))]
    );
}

#[test]
fn sink_registered_later_gets_the_origin_of_that_moment() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (early, sink) = notice_sink();
    player.register_sink(sink);
    player.tick(at_ms(100));
    let notice = player.shift_origin(at_ms(100), delta_ms(500)).unwrap();

    let (late, sink) = notice_sink();
    player.register_sink(sink);
    assert_eq!(
        *late.borrow(),
        vec![
            Event::Origin(OriginNotice::first(at_ms(500))),
            preview(&[(800, "c")]),
        ]
    );
    let [first, upcoming] = first_and_preview();
    assert_eq!(
        *early.borrow(),
        vec![
            first,
            upcoming,
            Event::Cue(at_ms(0), t("a")),
            Event::Cue(at_ms(100), t("b")),
            Event::Origin(notice),
        ]
    );
}

#[test]
fn cue_before_notice_before_cue_after_in_one_log() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = notice_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    let notice = player.reanchor(at_ms(100), at_ms(100), ms(300)).unwrap();
    player.tick(at_ms(100));

    let [first, upcoming] = first_and_preview();
    assert_eq!(
        *log.borrow(),
        vec![
            first,
            upcoming,
            Event::Cue(at_ms(0), t("a")),
            Event::Cue(at_ms(100), t("b")),
            Event::Origin(notice),
            Event::Cue(at_ms(100), t("c")),
        ]
    );
}

#[test]
fn notice_takes_effect_at_the_time_the_caller_passed() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = notice_sink();
    player.register_sink(sink);

    let notice = player.shift_origin(at_ms(40), delta_ms(250)).unwrap();
    assert_eq!(
        notice,
        OriginNotice {
            effective_at: at_ms(40),
            origin: at_ms(250),
            previous: Some(at_ms(0)),
        }
    );
    assert_eq!(notice.delta(), Some(delta_ms(250)));
    assert_eq!(log.borrow().last(), Some(&Event::Origin(notice)));
}

#[test]
fn unchanged_reorigin_emits_no_notice() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = notice_sink();
    player.register_sink(sink);

    assert_eq!(player.reanchor(at_ms(10), at_ms(0), ms(0)), None);
    assert_eq!(player.shift_origin(at_ms(10), delta_ms(0)), None);
    assert_eq!(*log.borrow(), first_and_preview());
}

#[test]
fn notice_reaches_every_sink_in_registration_order_and_none_after_completion() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let shared: EventLog = Rc::default();
    for _ in 0..2 {
        player.register_sink(Box::new(NoticeSink(shared.clone())));
    }
    let notice = player.shift_origin(at_ms(0), delta_ms(10)).unwrap();
    let [first, upcoming] = first_and_preview();
    let [again_first, again_upcoming] = first_and_preview();
    assert_eq!(
        *shared.borrow(),
        vec![
            first,
            upcoming,
            again_first,
            again_upcoming,
            Event::Origin(notice),
            Event::Origin(notice),
        ]
    );

    player.tick(at_ms(10_000));
    assert!(player.is_completed());
    let before = shared.borrow().len();
    assert_eq!(player.shift_origin(at_ms(10_000), delta_ms(5)), None);
    assert_eq!(shared.borrow().len(), before);
}

#[test]
fn sink_without_origin_set_override_behaves_the_same() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (plain, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    player.reanchor(at_ms(100), at_ms(100), ms(300));
    player.tick(at_ms(100));

    assert_eq!(logged_commands(&plain), vec![t("a"), t("b"), t("c")]);
    let fires: Vec<_> = plain.borrow().iter().map(|c| c.fire_time).collect();
    assert_eq!(fires, vec![at_ms(0), at_ms(100), at_ms(100)]);
}
