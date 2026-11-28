//! 消去の後の `talk` の数えの揃え方の決定論テスト。
//!
//! 数え始めの序数（数え済み − 数えた字の数）と捨てた字の数の大小、数えた字が 0 の場合のそれぞれで、
//! 揃えの後も数えた字の数が変わらず、次の区切りが数えた字の続きで来ることを固定する。

use super::*;
use dola::time::testing::at_ms;
use std::num::NonZeroU32;

fn talk3() -> LoopAnimation {
    LoopAnimation {
        id: 1,
        trigger: LoopTrigger::Talk {
            every: NonZeroU32::new(3).expect("正"),
        },
        frames: Vec::new(),
    }
}

/// 序数 `g` の文字が 50 ms おきに現れる偽の写し（0 文字目が 1000 ms）。
fn every_50ms(g: u64) -> TimePoint {
    at_ms(1000 + g as i64 * 50)
}

/// 刻み 1 回分（判定 → 数えを進める）。始まった開始の時刻を返す。
fn tick(armed: &mut Armed, now_seen: u64) -> Vec<TimePoint> {
    let anim = talk3();
    let (counted, prev_seen) = armed.talk_window_bounds().expect("数えを持つ");
    let window = TalkWindow {
        counted,
        prev_seen,
        now_seen,
        wall: &every_50ms,
    };
    let starts = armed.next_starts(&anim, at_ms(90_000), Some(&window));
    let started = starts
        .into_iter()
        .filter(|&at| armed.commit(&anim, at, false))
        .collect();
    armed.advance_talk(now_seen);
    started
}

/// 数え始め 1 が捨てた 3 字より小さい: 数えた 4 字は保たれ、次の区切り（6 字目）は揃えの後の 2 字目。
#[test]
fn start_below_the_dropped_count_keeps_the_counted_glyphs() {
    let mut armed = Armed::arm(at_ms(0), true, Some(1));
    assert_eq!(tick(&mut armed, 5), [every_50ms(3)], "前提: 3 字目");

    armed.realign_talk(&[2, 3, 4, 5, 6]);
    assert_eq!(armed.talk_window_bounds(), Some((4, 2)));
    assert_eq!(tick(&mut armed, 3), [], "5 字目");
    assert_eq!(tick(&mut armed, 4), [every_50ms(3)], "6 字目（序数 3）");
}

/// 数え始め 3 が捨てた 3 字と等しい: 数えた 2 字を保ち、次の字（序数 2）が 3 字目。
#[test]
fn start_equal_to_the_dropped_count_keeps_the_counted_glyphs() {
    let mut armed = Armed::arm(at_ms(0), true, Some(3));
    assert_eq!(tick(&mut armed, 5), []);

    armed.realign_talk(&[2, 3, 4]);
    assert_eq!(armed.talk_window_bounds(), Some((2, 2)));
    assert_eq!(tick(&mut armed, 3), [every_50ms(2)], "3 字目（序数 2）");
}

/// 数え始め 4 が捨てた 1 字より大きい: 数え済みより後の捨てた字は数えに関わらない。
#[test]
fn start_above_the_dropped_count_keeps_the_counted_glyphs() {
    let mut armed = Armed::arm(at_ms(0), true, Some(4));
    assert_eq!(tick(&mut armed, 6), []);

    armed.realign_talk(&[5, 6, 7]);
    assert_eq!(armed.talk_window_bounds(), Some((2, 5)));
    assert_eq!(tick(&mut armed, 6), [every_50ms(5)], "3 字目（序数 5）");
}

/// 数えた字が 0: 数え済みだけが下がり、揃えの後の 3 字目で鳴る。
#[test]
fn nothing_counted_stays_at_zero_and_counts_the_new_glyphs() {
    let mut armed = Armed::arm(at_ms(0), true, Some(5));

    armed.realign_talk(&[3, 4, 5, 6]);
    assert_eq!(armed.talk_window_bounds(), Some((0, 3)));
    assert_eq!(tick(&mut armed, 5), []);
    assert_eq!(tick(&mut armed, 6), [every_50ms(5)], "3 字目（序数 5）");
}

/// 数え済みより前に捨てた字が無ければ何もしない。数えを持たなければ何もしない。
#[test]
fn nothing_dropped_before_the_seen_glyphs_changes_nothing() {
    let mut armed = Armed::arm(at_ms(0), true, Some(2));
    assert_eq!(tick(&mut armed, 4), []);
    armed.realign_talk(&[4, 5]);
    armed.realign_talk(&[]);
    assert_eq!(armed.talk_window_bounds(), Some((2, 4)));

    let mut part = Armed::arm(at_ms(0), true, None);
    part.realign_talk(&[0, 1]);
    assert_eq!(part.talk_window_bounds(), None);
}
