//! `chunk_reveal_offsets` の檻。
//!
//! 今の式 `r = max(前の字 + 間隔, 塊の頭)`（間隔は `duration ÷ count` の実数）を、整数の倍で誤差なく
//! 求めた値と比べる。

use super::*;
use dola::time::testing::{delta_ms, ms};

fn span(ticks: i64) -> TimeSpan {
    TimeSpan::try_from_ticks(ticks).unwrap()
}

fn offsets(prev: Option<i64>, duration: i64, count: usize) -> Vec<i64> {
    chunk_reveal_offsets(prev.map(TimeDelta::from_ticks), span(duration), count)
        .map(TimeSpan::as_ticks)
        .collect()
}

/// 今の式の字の時刻を `scale` 倍の整数で返す。`prev`・`head` も `scale` 倍。`scale` は `count` で割り切れる。
fn exact_times(
    prev: Option<i128>,
    head: i128,
    duration: i64,
    count: usize,
    scale: i128,
) -> Vec<i128> {
    assert_eq!(scale % count.max(1) as i128, 0);
    let interval = i128::from(duration) * scale / count.max(1) as i128;
    let mut last = prev;
    (0..count)
        .map(|_| {
            let r = last.map_or(head, |p| (p + interval).max(head));
            last = Some(r);
            r
        })
        .collect()
}

/// 1 塊を比べる: どの字も今の式との差が 100ns 未満。
fn assert_within_one_tick(prev: Option<i64>, duration: i64, count: usize) -> Vec<i64> {
    let scale = count.max(1) as i128;
    let got = offsets(prev, duration, count);
    let exact = exact_times(
        prev.map(|p| i128::from(p) * scale),
        0,
        duration,
        count,
        scale,
    );
    for (g, e) in got.iter().zip(&exact) {
        assert!(
            (i128::from(*g) * scale - e).abs() < scale,
            "got {g}, exact {e}/{scale}"
        );
    }
    got
}

#[test]
fn indivisible_length_from_chunk_head() {
    assert_eq!(
        assert_within_one_tick(None, 1_000_000, 3),
        [0, 333_333, 666_667]
    );
}

#[test]
fn indivisible_length_continuing_from_previous_char() {
    assert_eq!(
        assert_within_one_tick(Some(-333_333), 1_000_000, 3),
        [0, 333_334, 666_667]
    );
}

#[test]
fn exact_half_from_chunk_head_rounds_last_char_up() {
    let got = assert_within_one_tick(None, 10_000, 32);
    assert_eq!(got[31], 9_688);
}

#[test]
fn exact_half_continuing_from_previous_char() {
    let got = assert_within_one_tick(Some(-312), 10_000, 32);
    assert_eq!(got[..3], [0, 313, 625]);
}

#[test]
fn previous_char_at_chunk_head() {
    let got = assert_within_one_tick(Some(0), 10_000, 32);
    assert_eq!(got[..2], [312, 625]);
}

#[test]
fn last_char_of_continuing_chunk_is_previous_plus_duration() {
    for (p, d, c) in [(-333_333, 1_000_000, 3), (-312, 10_000, 32), (17, 1_000, 7)] {
        let got = assert_within_one_tick(Some(p), d, c);
        assert_eq!(*got.last().unwrap(), p + d);
    }
}

#[test]
fn far_previous_char_starts_from_chunk_head() {
    assert_eq!(
        assert_within_one_tick(Some(-1_000_000), 1_000_000, 3),
        [0, 333_333, 666_667]
    );
}

#[test]
fn previous_char_plus_interval_exactly_at_head_continues_identically() {
    assert_eq!(assert_within_one_tick(Some(-100), 300, 3), [0, 100, 200]);
    assert_eq!(assert_within_one_tick(Some(-101), 300, 3), [0, 100, 200]);
}

#[test]
fn zero_length_chunk() {
    assert_eq!(assert_within_one_tick(None, 0, 3), [0, 0, 0]);
    assert_eq!(assert_within_one_tick(Some(-5), 0, 3), [0, 0, 0]);
    assert_eq!(assert_within_one_tick(Some(0), 0, 3), [0, 0, 0]);
    assert_eq!(assert_within_one_tick(Some(7), 0, 3), [7, 7, 7]);
}

#[test]
fn zero_chars_yield_nothing() {
    assert!(offsets(None, 1_000_000, 0).is_empty());
    assert!(offsets(Some(-5), 1_000_000, 0).is_empty());
}

#[test]
fn chained_chunks_keep_error_below_one_tick_and_never_go_back() {
    // (塊の頭, 長さ, 字数): 頭の間を長さと同じ・短い・長いに散らし、続く塊と頭から始まる塊を混ぜる。
    let mut chunks = Vec::new();
    let mut head = 0i64;
    for k in 0..60i64 {
        let (duration, count) =
            [(1_000_000, 3), (10_000, 32), (999_999, 7), (123_457, 5)][k as usize % 4];
        chunks.push((head, duration, count));
        head += match k % 5 {
            0 => duration / 2,
            1 => duration * 3,
            _ => duration,
        };
    }
    let scale: i128 = 3 * 32 * 7 * 5;

    let (mut actual_prev, mut exact_prev): (Option<i64>, Option<i128>) = (None, None);
    let mut continuing = 0;
    for (head, duration, count) in chunks {
        let p = actual_prev.map(|prev| prev - head);
        continuing += usize::from(p.is_some_and(|p| p * count as i64 + duration >= 0));
        let got: Vec<i64> = offsets(p, duration, count)
            .iter()
            .map(|o| head + o)
            .collect();
        let exact = exact_times(exact_prev, i128::from(head) * scale, duration, count, scale);
        for (g, e) in got.iter().zip(&exact) {
            assert!(
                (i128::from(*g) * scale - e).abs() < scale,
                "chunk at {head}: got {g}, exact {e}/{scale}"
            );
            assert!(
                actual_prev.is_none_or(|prev| *g >= prev),
                "chunk at {head} went back"
            );
            actual_prev = Some(*g);
        }
        exact_prev = exact.last().copied();
    }
    assert!(continuing >= 3, "only {continuing} continuing chunks");
}

#[test]
fn production_length_matches_current_values() {
    for count in [1usize, 2, 5, 17] {
        let duration = ms(50 * count as u32);
        let from_head: Vec<TimeSpan> = chunk_reveal_offsets(None, duration, count).collect();
        let want: Vec<TimeSpan> = (0..count as u32).map(|i| ms(50 * i)).collect();
        assert_eq!(from_head, want);

        let after_previous: Vec<TimeSpan> =
            chunk_reveal_offsets(Some(delta_ms(-50)), duration, count).collect();
        assert_eq!(after_previous, want);

        let overlapping: Vec<TimeSpan> =
            chunk_reveal_offsets(Some(delta_ms(-20)), duration, count).collect();
        let want: Vec<TimeSpan> = (0..count as u32).map(|i| ms(30 + 50 * i)).collect();
        assert_eq!(overlapping, want);
    }
}
