//! 絶対の発火時刻の檻: 取り出した時の起点＋相対・後の置き直しや遅れた刻みで変わらない。

use super::{CuePlayer, CueSheet, CueSink, Rc, RefCell, TalkCue, at_ms, sheet_at_zero};
use crate::test_support::{ms, recording_sink, text};
use dola::time::{TimeDelta, TimePoint};

fn delta_ms(millis: i64) -> TimeDelta {
    TimeDelta::from_ticks(millis * 10_000)
}

fn sheet() -> CueSheet {
    sheet_at_zero(vec![
        text(ms(0), "a", ms(50)),
        text(ms(100), "b", ms(50)),
        text(ms(300), "c", ms(50)),
    ])
}

fn fire_times(log: &Rc<RefCell<Vec<TalkCue>>>) -> Vec<TimePoint> {
    log.borrow().iter().map(|c| c.fire_time).collect()
}

#[test]
fn fire_time_is_origin_at_take_out_plus_relative() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    player.shift_origin(at_ms(100), delta_ms(1000));
    player.tick(at_ms(1300));

    assert_eq!(fire_times(&log), vec![at_ms(0), at_ms(100), at_ms(1300)]);
    let ats: Vec<_> = log.borrow().iter().map(|c| c.at).collect();
    assert_eq!(ats, vec![ms(0), ms(100), ms(300)]);
}

#[test]
fn already_delivered_fire_times_do_not_change_on_later_reorigin() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(100));
    let before = fire_times(&log);
    player.shift_origin(at_ms(100), delta_ms(-50));
    player.shift_origin(at_ms(100), delta_ms(700));
    assert_eq!(fire_times(&log), before);

    player.tick(at_ms(950));
    assert_eq!(fire_times(&log), vec![at_ms(0), at_ms(100), at_ms(950)]);
}

#[test]
fn late_tick_does_not_change_fire_time() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(at_ms(10_000));
    assert_eq!(fire_times(&log), vec![at_ms(0), at_ms(100), at_ms(300)]);
}

struct PreviewRecorder {
    previews: Rc<RefCell<Vec<Vec<TalkCue>>>>,
    emits: Rc<RefCell<Vec<TalkCue>>>,
}

impl CueSink for PreviewRecorder {
    fn emit(&mut self, cue: TalkCue) {
        self.emits.borrow_mut().push(cue);
    }

    fn preview(&mut self, upcoming: &[TalkCue]) {
        self.previews.borrow_mut().push(upcoming.to_vec());
    }
}

fn recorder() -> (
    Rc<RefCell<Vec<Vec<TalkCue>>>>,
    Rc<RefCell<Vec<TalkCue>>>,
    Box<dyn CueSink>,
) {
    let previews = Rc::new(RefCell::new(Vec::new()));
    let emits = Rc::new(RefCell::new(Vec::new()));
    let sink = Box::new(PreviewRecorder {
        previews: previews.clone(),
        emits: emits.clone(),
    });
    (previews, emits, sink)
}

#[test]
fn preview_then_reorigin_then_take_out_differs_by_exactly_the_delta() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (previews, emits, sink) = recorder();
    player.register_sink(sink);
    let shown: Vec<_> = previews.borrow()[0].iter().map(|c| c.fire_time).collect();
    assert_eq!(shown, vec![at_ms(0), at_ms(100), at_ms(300)]);

    player.shift_origin(at_ms(0), delta_ms(400));
    player.tick(at_ms(10_000));

    let got = fire_times_of(&emits.borrow());
    assert_eq!(
        got,
        vec![at_ms(400), at_ms(500), at_ms(700)],
        "新しい起点＋at"
    );
    for (g, s) in got.iter().zip(&shown) {
        assert_eq!(*g - *s, delta_ms(400), "先渡しとの差は置き直しの量ちょうど");
    }
}

#[test]
fn preview_equals_emit_when_not_reoriginated() {
    let mut player = CuePlayer::from_sheet(&sheet()).unwrap();
    let (previews, emits, sink) = recorder();
    player.register_sink(sink);
    player.tick(at_ms(10_000));
    assert_eq!(previews.borrow()[0], *emits.borrow());
}

fn fire_times_of(cues: &[TalkCue]) -> Vec<TimePoint> {
    cues.iter().map(|c| c.fire_time).collect()
}
