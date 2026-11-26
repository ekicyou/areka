//! 台本の絶対の発火時刻ちょうどの tick で、再生機が合図を配る検証。
//!
//! 予定そのもの（TimedSchedule）の検証は `src/cue/schedule/tests.rs`。

use crate::test_support::{recording_sink, text};
use dola::cue::{CuePlayer, CueSheet};
use dola::time::{TimePoint, TimeSpan};

/// 刻印した台本の `absolute_fire_time` ちょうどの tick で配り、1 刻み手前では配らない。
#[test]
fn player_emits_cue_exactly_at_sheet_absolute_fire_time() {
    let at = TimeSpan::try_from_ticks(25_999_999).unwrap();
    let sheet = CueSheet::new(vec![text(at, "x", TimeSpan::ZERO)])
        .with_absolute_start_time(TimePoint::from_ticks(1_000_000_000));
    let fire = sheet.absolute_fire_time(&sheet.cues()[0]).unwrap();
    assert_eq!(fire.as_ticks(), 1_025_999_999);
    let mut player = CuePlayer::from_sheet(&sheet).unwrap();
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(TimePoint::from_ticks(fire.as_ticks() - 1));
    assert!(log.borrow().is_empty());

    player.tick(fire);
    let received: Vec<TimeSpan> = log.borrow().iter().map(|c| c.at).collect();
    assert_eq!(received, vec![at]);
}
