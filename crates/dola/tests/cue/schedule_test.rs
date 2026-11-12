//! 台本の絶対の発火時刻ちょうどの tick で、再生機が合図を配る検証。
//!
//! 予定そのもの（TimedSchedule）の検証は `src/cue/schedule/tests.rs`。

use crate::test_support::{recording_sink, text};
use dola::cue::{CuePlayer, CueSheet};

/// アンカーを刻んだときに、引き算で相対へ戻すと丸めで手前に落ちる相対時刻。
const ROUNDS_DOWN: f64 = 2.5999999999999996;

/// 刻印した台本の `absolute_fire_time` ちょうどの tick で、再生機がその合図を配る。
#[test]
fn player_emits_cue_at_sheet_absolute_fire_time() {
    let sheet = CueSheet::new(vec![text(ROUNDS_DOWN, "x", 0.0)]).with_absolute_start_time(100.0);
    let fire = sheet.absolute_fire_time(&sheet.cues()[0]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(fire);
    let received: Vec<f64> = log.borrow().iter().map(|c| c.at).collect();
    assert_eq!(received, vec![ROUNDS_DOWN]);
}
