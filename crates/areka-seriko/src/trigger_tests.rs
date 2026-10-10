//! 引き金の判定（`runonce`・`periodic`・`talk`）の決定論テスト
//! （spec: areka-P0-seriko-trigger-intervals 要件 2.1・2.2・3.1〜3.5・4.1・4.5〜4.7・5.3・5.9・6.1・
//! 9.3・9.4）。
//!
//! 時刻は全部ただの数（ms）。判定はコマ列を読まないので、animation のコマ列は空で組む。
//! 文字が現れる時刻は、序数から壁時刻への偽の写し（閉包）で渡す。

use super::*;
use std::num::{NonZeroU32, NonZeroU64};

fn runonce(id: u32) -> LoopAnimation {
    LoopAnimation {
        id,
        trigger: LoopTrigger::Runonce,
        frames: Vec::new(),
    }
}

fn periodic(id: u32, period_ms: u64) -> LoopAnimation {
    LoopAnimation {
        id,
        trigger: LoopTrigger::Periodic {
            period_ms: NonZeroU64::new(period_ms).expect("正"),
        },
        frames: Vec::new(),
    }
}

fn talk(id: u32, every: u32) -> LoopAnimation {
    LoopAnimation {
        id,
        trigger: LoopTrigger::Talk {
            every: NonZeroU32::new(every).expect("正"),
        },
        frames: Vec::new(),
    }
}

/// 開いた窓で `at_ms` に見え始めた状態。
fn armed_at(at_ms: u64) -> Armed {
    Armed::arm(Some(at_ms), true, None)
}

/// 開いた窓で見え始め、その時点で `revealed` 文字が現れていた状態（`talk` の数えを持つ）。
fn armed_with_text(revealed: u64) -> Armed {
    Armed::arm(Some(0), true, Some(revealed))
}

/// `talk` の判定に渡す刻みの時刻。文字が現れる時刻のどれよりも後に置き、開始の時刻が
/// 刻みの時刻でないことを見分ける。
const TICK_MS: u64 = 90_000;

/// 序数 `g` の文字が 50 ms おきに現れる偽の写し（0 文字目が 1000 ms）。
fn every_50ms(g: u64) -> u64 {
    1000 + g * 50
}

/// 刻み 1 回分: 数えから文字の窓を組んで判定し、判定の後に数えを進める（一番上の面の配線と同じ順）。
fn talk_tick(
    armed: &mut Armed,
    anim: &LoopAnimation,
    now_seen: u64,
    playing: bool,
    wall_ms: &dyn Fn(u64) -> u64,
) -> Option<u64> {
    let (base, prev_seen) = armed.talk_window_bounds().expect("数えを持つ");
    let window = TalkWindow {
        base,
        prev_seen,
        now_seen,
        wall_ms,
    };
    let started = armed.poll(anim, TICK_MS, playing, Some(&window));
    armed.advance_talk(now_seen);
    started
}

// ── runonce ──────────────────────────────────────────────────────────────

/// 構えた後の最初の判定で 1 回だけ鳴り、開始の時刻は判定の時刻でなく見え始めの時刻。
/// 印は animation の番号ごと（別の `runonce` は自分の 1 回を鳴らす）。
#[test]
fn runonce_fires_once_and_starts_at_visible_since() {
    let mut armed = armed_at(1234);
    let (a, b) = (runonce(1), runonce(2));

    assert_eq!(armed.poll(&a, 1250, false, None), Some(1234));
    assert_eq!(armed.poll(&a, 1266, true, None), None, "再生中");
    assert_eq!(armed.poll(&a, 9000, false, None), None, "再生が終わった後");

    assert_eq!(armed.poll(&b, 9000, false, None), Some(1234));
    assert_eq!(armed.poll(&b, 9016, false, None), None);
}

/// 再生中の判定は印を付けない（鳴らしていないものを鳴らしたことにしない）。
#[test]
fn runonce_polled_while_playing_keeps_its_one_shot() {
    let mut armed = armed_at(100);
    let a = runonce(1);

    assert_eq!(armed.poll(&a, 100, true, None), None);
    assert_eq!(armed.poll(&a, 116, false, None), Some(100));
}

// ── periodic ─────────────────────────────────────────────────────────────

/// 見え始めの瞬間には鳴らず、起点から周期ごとの境目で鳴る。境目を過ぎた刻みで判定しても、
/// 開始の時刻は境目そのもの。同じ周の中では 2 度鳴らない。
#[test]
fn periodic_fires_at_each_lap_boundary_counted_from_visible_since() {
    let mut armed = armed_at(500);
    let p = periodic(3, 2000);

    assert_eq!(armed.poll(&p, 500, false, None), None, "見え始めの瞬間");
    assert_eq!(armed.poll(&p, 2499, false, None), None, "境目の 1 ms 前");
    assert_eq!(
        armed.poll(&p, 2500, false, None),
        Some(2500),
        "境目ちょうど"
    );
    assert_eq!(armed.poll(&p, 2516, false, None), None, "同じ周");
    assert_eq!(
        armed.poll(&p, 4510, false, None),
        Some(4500),
        "境目を 10 ms 過ぎた刻み"
    );
}

/// 周の数えは animation の番号ごと（周期の違う 2 本が互いの周を食わない）。
#[test]
fn periodic_laps_are_counted_per_animation() {
    let mut armed = armed_at(0);
    let (fast, slow) = (periodic(1, 1000), periodic(2, 3000));

    assert_eq!(armed.poll(&fast, 2000, false, None), Some(2000));
    assert_eq!(armed.poll(&slow, 2000, false, None), None);
    assert_eq!(armed.poll(&fast, 3000, false, None), Some(3000));
    assert_eq!(armed.poll(&slow, 3000, false, None), Some(3000));
}

/// 再生中に来た周は飛ばす。飛ばした周は、再生が終わっても後から鳴らない。
#[test]
fn periodic_lap_reached_while_playing_is_skipped_for_good() {
    let mut armed = armed_at(0);
    let p = periodic(3, 1000);

    assert_eq!(armed.poll(&p, 1000, true, None), None, "再生中の周");
    assert_eq!(armed.poll(&p, 1016, false, None), None, "飛ばした周");
    assert_eq!(armed.poll(&p, 2000, false, None), Some(2000), "次の周");
}

/// 1 回の刻みで 2 周以上またいでも 1 回だけ鳴り、開始の時刻は最新の境目。
#[test]
fn periodic_tick_spanning_laps_fires_once_at_the_latest_boundary() {
    let mut armed = armed_at(100);
    let p = periodic(3, 1000);

    assert_eq!(armed.poll(&p, 3350, false, None), Some(3100));
    assert_eq!(
        armed.poll(&p, 3350, false, None),
        None,
        "またいだ分を積まない"
    );
    assert_eq!(armed.poll(&p, 4100, false, None), Some(4100));
}

// ── 隠す・現す ───────────────────────────────────────────────────────────

/// 隠れている間は何も鳴らない。現れた時刻が `periodic` の新しい起点になり（周は 0 から）、
/// 鳴らし済みの `runonce` は現れても鳴らない。
#[test]
fn hide_silences_and_show_rebases_periodic_but_keeps_the_runonce_mark() {
    let mut armed = armed_at(0);
    let (r, p) = (runonce(1), periodic(3, 1000));
    assert_eq!(armed.poll(&r, 0, false, None), Some(0));
    assert_eq!(armed.poll(&p, 1000, false, None), Some(1000));

    armed.hide();
    assert_eq!(armed.poll(&p, 5000, false, None), None, "隠れている間");
    assert_eq!(armed.poll(&r, 5000, false, None), None);

    armed.show(5300, None);
    assert_eq!(armed.poll(&r, 5300, false, None), None, "印は残る");
    assert_eq!(armed.poll(&p, 5300, false, None), None, "現れた瞬間");
    assert_eq!(armed.poll(&p, 6299, false, None), None, "古い起点の境目");
    assert_eq!(
        armed.poll(&p, 6300, false, None),
        Some(6300),
        "新しい起点の 1 周目"
    );
}

/// 閉じた窓で構えたら、現れるまで何も鳴らない。まだ鳴らしていない `runonce` は現れた時刻で鳴る。
#[test]
fn armed_closed_stays_silent_until_shown() {
    let mut armed = Armed::arm(Some(100), false, None);
    let (r, p) = (runonce(1), periodic(3, 1000));
    assert_eq!(armed.poll(&r, 100, false, None), None);
    assert_eq!(armed.poll(&p, 4000, false, None), None);

    armed.show(4200, None);
    assert_eq!(armed.poll(&r, 4216, false, None), Some(4200));
    assert_eq!(armed.poll(&p, 5200, false, None), Some(5200));
}

// ── talk ─────────────────────────────────────────────────────────────────

/// 3 文字ごとの区切り（3 文字目・6 文字目）で鳴り、開始の時刻は刻みの時刻でなく区切りの文字が
/// 現れた時刻。区切りの間の文字では鳴らない。
#[test]
fn talk_fires_at_every_third_glyph_and_starts_at_its_reveal_time() {
    let mut armed = armed_with_text(0);
    let t = talk(1, 3);

    assert_eq!(talk_tick(&mut armed, &t, 0, false, &every_50ms), None);
    assert_eq!(talk_tick(&mut armed, &t, 2, false, &every_50ms), None);
    assert_eq!(
        talk_tick(&mut armed, &t, 3, false, &every_50ms),
        Some(1100),
        "3 文字目（序数 2）"
    );
    assert_eq!(
        talk_tick(&mut armed, &t, 3, false, &every_50ms),
        None,
        "同じ区切り"
    );
    assert_eq!(talk_tick(&mut armed, &t, 5, false, &every_50ms), None);
    assert_eq!(
        talk_tick(&mut armed, &t, 6, false, &every_50ms),
        Some(1250),
        "6 文字目（序数 5）"
    );
}

/// 数え始めは構えた時点で現れていた文字の数（それより前の文字は数えない）。現し直すと、
/// その時点の数から数え直す。
#[test]
fn talk_counts_from_the_glyphs_visible_when_armed_and_recounts_on_show() {
    let mut armed = armed_with_text(4);
    let t = talk(1, 3);

    assert_eq!(talk_tick(&mut armed, &t, 6, false, &every_50ms), None);
    assert_eq!(
        talk_tick(&mut armed, &t, 7, false, &every_50ms),
        Some(1300),
        "構えてから 3 文字目（序数 6）"
    );

    armed.hide();
    armed.show(5000, Some(8));
    assert_eq!(armed.talk_window_bounds(), Some((8, 8)));
    assert_eq!(
        talk_tick(&mut armed, &t, 10, false, &every_50ms),
        None,
        "古い数え始めなら 9 文字目の区切り"
    );
    assert_eq!(
        talk_tick(&mut armed, &t, 11, false, &every_50ms),
        Some(1500),
        "現れてから 3 文字目（序数 10）"
    );
}

/// 1 回の刻みで区切りを 2 つ以上越えても 1 回だけ鳴る。開始の時刻は越えた区切りのうち最新のもの。
/// 文字が一度に現れた（区切りが同じ時刻に重なった）ときも 1 回。
#[test]
fn talk_boundaries_crossed_in_one_window_fire_once_at_the_latest() {
    let t = talk(1, 3);

    let mut late = armed_with_text(0);
    assert_eq!(
        talk_tick(&mut late, &t, 7, false, &every_50ms),
        Some(1250),
        "序数 2 と 5 を越えた"
    );
    assert_eq!(
        talk_tick(&mut late, &t, 7, false, &every_50ms),
        None,
        "越えた分を積まない"
    );
    assert_eq!(talk_tick(&mut late, &t, 9, false, &every_50ms), Some(1400));

    let at_once = |_g: u64| 2000;
    let mut burst = armed_with_text(0);
    assert_eq!(talk_tick(&mut burst, &t, 9, false, &at_once), Some(2000));
    assert_eq!(talk_tick(&mut burst, &t, 9, false, &at_once), None);
}

/// 再生中に越えた区切りでは始め直さない。数えは進むので、再生が終わっても後から鳴らない。
#[test]
fn talk_boundary_reached_while_playing_is_skipped_for_good() {
    let mut armed = armed_with_text(0);
    let t = talk(1, 3);

    assert_eq!(talk_tick(&mut armed, &t, 3, true, &every_50ms), None);
    assert_eq!(armed.talk_window_bounds(), Some((0, 3)), "数えは進む");
    assert_eq!(
        talk_tick(&mut armed, &t, 4, false, &every_50ms),
        None,
        "見送った区切り"
    );
    assert_eq!(
        talk_tick(&mut armed, &t, 6, false, &every_50ms),
        Some(1250),
        "次の区切り"
    );
}

/// 数えは単調。今見えている数が減った刻み（数え始めより手前まで減った刻みも）は数えを進めず、
/// 鳴らさない。数え済みの区切りは、数が戻っても鳴り直さない。
#[test]
fn talk_count_never_goes_back_when_fewer_glyphs_are_visible() {
    let mut armed = armed_with_text(1);
    let t = talk(1, 3);
    assert_eq!(
        talk_tick(&mut armed, &t, 6, false, &every_50ms),
        Some(1150),
        "序数 3"
    );

    assert_eq!(talk_tick(&mut armed, &t, 2, false, &every_50ms), None);
    assert_eq!(
        talk_tick(&mut armed, &t, 0, false, &every_50ms),
        None,
        "数え始めより手前"
    );
    assert_eq!(armed.talk_window_bounds(), Some((1, 6)), "進めない");

    assert_eq!(
        talk_tick(&mut armed, &t, 6, false, &every_50ms),
        None,
        "数え済みの区切り"
    );
    assert_eq!(
        talk_tick(&mut armed, &t, 7, false, &every_50ms),
        Some(1300),
        "序数 6"
    );
}

/// 文字の窓が無い判定と、隠れている間の判定は鳴らない。数えを持たない状態（部品・表に `talk` が
/// 無い面）は、進めても数えを持たないまま。
#[test]
fn talk_without_a_window_or_while_hidden_stays_silent() {
    let t = talk(1, 3);
    let crossed = TalkWindow {
        base: 0,
        prev_seen: 0,
        now_seen: 3,
        wall_ms: &every_50ms,
    };

    let mut open = armed_with_text(0);
    assert_eq!(open.poll(&t, TICK_MS, false, None), None, "窓が無い");

    let mut closed = Armed::arm(Some(0), false, Some(0));
    assert_eq!(
        closed.poll(&t, TICK_MS, false, Some(&crossed)),
        None,
        "隠れている"
    );

    let mut part = armed_at(0);
    part.advance_talk(3);
    assert_eq!(part.talk_window_bounds(), None);
    assert_eq!(
        part.poll(&t, TICK_MS, false, Some(&crossed)),
        Some(1100),
        "借りた窓で鳴る"
    );
}
