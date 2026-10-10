//! 引き金の判定（`runonce`・`periodic`）の決定論テスト
//! （spec: areka-P0-seriko-trigger-intervals 要件 2.1・2.2・3.1〜3.5・5.3・5.9・6.1・9.3）。
//!
//! 時刻は全部ただの数（ms）。判定はコマ列を読まないので、animation のコマ列は空で組む。

use super::*;
use std::num::NonZeroU64;

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

/// 開いた窓で `at_ms` に見え始めた状態。
fn armed_at(at_ms: u64) -> Armed {
    Armed::arm(Some(at_ms), true, None)
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
