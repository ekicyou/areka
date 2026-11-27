//! 置き直しの知らせを受けた文字の時刻の列の決定論テスト。
//!
//! 列は頭 1000 ms・再生時間 400 ms・8 文字（1000・1050・…・1350 ms）。期待の値は、文字の層と同じ
//! 決まり（効く時刻より後の時刻だけを動いた長さだけ動かし、効く時刻より前になる物は効く時刻にする）
//! から手で求めた固定の値。

use super::*;
use dola::time::testing::{at_ms, delta_ms, ms};

fn times_from(feed: &TalkFeed, from: u64) -> Vec<TimePoint> {
    (from..).map_while(|g| feed.time_of(g)).collect()
}

fn at_each(ms: &[i64]) -> Vec<TimePoint> {
    ms.iter().map(|&m| at_ms(m)).collect()
}

fn eight_from_1000() -> TalkFeed {
    let mut feed = TalkFeed::default();
    feed.push_chunk(at_ms(1000), ms(400), 8);
    feed
}

/// 効く時刻以前（ちょうどの時刻を含む）の字は動かない。
#[test]
fn glyphs_up_to_the_effective_time_keep_their_times() {
    let mut feed = eight_from_1000();
    feed.shift_after(at_ms(1100), delta_ms(500));
    assert_eq!(
        times_from(&feed, 0)[..3],
        at_each(&[1000, 1050, 1100])[..],
        "効く時刻ちょうどの字も動かない"
    );
}

/// 効く時刻より後の字は、動いた長さだけ後ろへ動く。動いた字は、動いた後の時刻まで現れない。
#[test]
fn glyphs_after_the_effective_time_move_by_the_delta() {
    let mut feed = eight_from_1000();
    feed.shift_after(at_ms(1120), delta_ms(500));
    assert_eq!(
        times_from(&feed, 0),
        at_each(&[1000, 1050, 1100, 1650, 1700, 1750, 1800, 1850])
    );
    assert_eq!(feed.revealed_until(at_ms(1640)), 3);
    assert_eq!(feed.revealed_until(at_ms(1650)), 4);
}

/// 前へ動いて効く時刻より前になる字は、効く時刻になる（列は単調のまま）。
#[test]
fn glyphs_moved_before_the_effective_time_land_on_it() {
    let mut feed = eight_from_1000();
    feed.shift_after(at_ms(1120), delta_ms(-150));
    assert_eq!(
        times_from(&feed, 0),
        at_each(&[1000, 1050, 1100, 1120, 1120, 1120, 1150, 1200])
    );
    assert_eq!(feed.revealed_until(at_ms(1120)), 6);
}

/// 続けて 2 回（後ろへ動かして、前へ戻す）: 2 回目は 1 回目で動いた時刻に当たる。
#[test]
fn moving_back_then_forward_again_follows_both_notices() {
    let mut feed = eight_from_1000();
    feed.shift_after(at_ms(1120), delta_ms(500));
    feed.shift_after(at_ms(1200), delta_ms(-500));
    assert_eq!(
        times_from(&feed, 0),
        at_each(&[1000, 1050, 1100, 1200, 1200, 1250, 1300, 1350])
    );
}

/// 字の列を全部刈り込んだ後も、「前の字の時刻」の欄が同じ決まりで動く。知らせの後に届いた塊は、
/// 動いた後の最後の字（1850）から続く（文字の層は自分の列の最後の字を読むので、同じ時刻になる）。
#[test]
fn previous_glyph_time_moves_with_the_list_after_pruning() {
    let mut feed = eight_from_1000();
    feed.prune_before(8);
    feed.shift_after(at_ms(1120), delta_ms(500));
    // 頭 1400・再生時間 100・2 文字: 前の字 1850 から 50 ずつ。
    feed.push_chunk(at_ms(1400), ms(100), 2);
    assert_eq!(times_from(&feed, 8), at_each(&[1900, 1950]));
}
