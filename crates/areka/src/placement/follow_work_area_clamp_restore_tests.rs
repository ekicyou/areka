//! 記憶から戻した組の寄せの檻（window-work-area-clamp・task 1.4・要件 4.1〜4.4・7.1・7.2・7.4・
//! 9.1・9.2・10.1 ⑷⑸⑹）。
//!
//! 数値は開発者の机の形の合成: 主の画面の作業領域 `0..2880 × 0..1704`。記憶の位置は
//! 右に外したモニタ（x=5000 から）で、今のどのモニタとも交わらない。`shown` は窓ごとの判定が
//! 寄せた結果（見えている窓は `shown == projected`）を手で置く。

use super::super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::test_support::{point, rect};
use super::{
    GroupShift, MonitorSnapshot, RestoredWindow, WORK_AREA_CLAMP_TAG, WORK_AREA_KEEP_TAG,
    WORK_AREA_UNRESOLVED_TAG, keep_restored_groups,
};
use crate::placement::chain_finalize::{AdjacencyMark, ChainRect};
use crate::placement::resolver::PointPx;

fn desk() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(0, 0, 2880, 1704)],
    }
}

/// 640×840 の窓。`projected` は記憶の位置のまま（吸着の当て直しで動かない形）。
fn win(scope: usize, at: PointPx, shown: PointPx, mark: Option<AdjacencyMark>) -> RestoredWindow {
    RestoredWindow {
        scope,
        remembered: ChainRect {
            x: at.x,
            y: at.y,
            w: 640,
            h: 840,
        },
        projected: at,
        shown,
        mark,
    }
}

/// 組の寄せの戻りを窓ごとの結果へ当てた、表示の位置の並び。
fn placed(windows: &[RestoredWindow], shifts: &[GroupShift]) -> Vec<PointPx> {
    windows
        .iter()
        .map(|w| {
            shifts
                .iter()
                .find(|s| s.scope == w.scope)
                .map_or(w.shown, |s| s.to)
        })
        .collect()
}

fn tagged<'a>(events: &'a [LogEvent], tag: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(tag))
        .collect()
}

/// ⑷ 全員が外の、隣り合っていた組（印 right）と重なっていた組（印 none）は、寄せた後も互いの
/// 差が同じで、和の矩形が作業領域に入る（要件 4.1・4.2・7.1）。`Clamp`（how=shifted）が残る。
#[test]
fn groups_all_outside_shift_together() {
    let cases = [
        // 隣り合っていた組（印 right・記憶の矩形も右に隣り合う）。
        [
            win(0, point(5000, 864), point(2240, 864), None),
            win(
                1,
                point(5640, 864),
                point(2240, 864),
                Some(AdjacencyMark::Right),
            ),
        ],
        // 重なっていた組（印 none・記憶の矩形が重なる）。
        [
            win(0, point(5000, 864), point(2240, 864), None),
            win(
                1,
                point(5300, 800),
                point(2240, 800),
                Some(AdjacencyMark::Separate),
            ),
        ],
    ];
    for windows in cases {
        let (shifts, events) = capture_logs(|| keep_restored_groups(&windows, &desk()));
        let after = placed(&windows, &shifts);
        let delta = |i: usize| {
            (
                after[i].x - windows[i].projected.x,
                after[i].y - windows[i].projected.y,
            )
        };
        assert_eq!(delta(0), delta(1), "互いの差が保たれる: {after:?}");
        let right = after[1].x + 640;
        assert!(
            right <= 2880 && after[0].x >= 0,
            "和が作業領域に入る: {after:?}"
        );
        let clamps = tagged(&events, WORK_AREA_CLAMP_TAG);
        assert_eq!(clamps.len(), shifts.len());
        assert!(!shifts.is_empty());
        assert_eq!(clamps[0].level, tracing::Level::INFO);
        assert_eq!(clamps[0].expect_field("scene"), "\"restore-group\"");
        assert_eq!(clamps[0].expect_field("how"), "\"shifted\"");
    }
}

/// 見えている窓のある組は見えている窓を動かさず、外の窓は窓ごとの結果のまま
/// （要件 4.1 の改め・`Keep` reason=partner-visible）。
#[test]
fn visible_member_keeps_group_per_window() {
    let windows = [
        win(0, point(2240, 864), point(2240, 864), None),
        win(
            1,
            point(2880, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let (shifts, events) = capture_logs(|| keep_restored_groups(&windows, &desk()));
    assert_eq!(shifts, vec![]);
    let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
    assert_eq!(keep.level, tracing::Level::DEBUG);
    assert_eq!(keep.expect_field("reason"), "\"partner-visible\"");
    assert_eq!(keep.expect_field("scope"), "1");
}

/// 離れていた組・番号の飛んだ対は組にせず窓ごとの結果のまま（要件 4.4）。記録もしない。
#[test]
fn apart_or_non_consecutive_pairs_are_not_grouped() {
    let apart = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(7000, 864),
            point(2240, 864),
            Some(AdjacencyMark::Separate),
        ),
    ];
    let skipped = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            2,
            point(5640, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let ((a, b), events) = capture_logs(|| {
        (
            keep_restored_groups(&apart, &desk()),
            keep_restored_groups(&skipped, &desk()),
        )
    });
    assert_eq!((a, b), (vec![], vec![]));
    assert!(
        tagged(&events, "[work-area-clamp]").is_empty(),
        "{events:?}"
    );
}

/// どれも見えている組は動かさず `Keep`（reason=remembered-visible）を残す（要件 4.3）。
#[test]
fn all_visible_group_is_kept() {
    let windows = [
        win(0, point(1000, 864), point(1000, 864), None),
        win(
            1,
            point(1640, 864),
            point(1640, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let (shifts, events) = capture_logs(|| keep_restored_groups(&windows, &desk()));
    assert_eq!(shifts, vec![]);
    let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
    assert_eq!(keep.expect_field("reason"), "\"remembered-visible\"");
}

/// ⑸ 和の矩形が作業領域より大きいと左辺と上辺が合い、互いの差は保たれる（要件 7.2）。
#[test]
fn oversized_union_aligns_left_and_top() {
    let small = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1000, 700)],
    };
    let windows = [
        win(0, point(5000, 5000), point(360, 0), None),
        win(
            1,
            point(5640, 5000),
            point(360, 0),
            Some(AdjacencyMark::Right),
        ),
    ];
    let shifts = keep_restored_groups(&windows, &small);
    assert_eq!(placed(&windows, &shifts), vec![point(0, 0), point(640, 0)]);
}

/// ⑹ モニタの写しが空だと返さず `Unresolved` の warn が出る（要件 7.4）。対照: 同じ捕捉窓で
/// 写しの有る呼び出しは `Clamp` を出す。
#[test]
fn missing_snapshot_warns_unresolved_with_contrast() {
    let windows = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(5640, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let empty = MonitorSnapshot { work_areas: vec![] };
    let ((a, b), events) = capture_logs(|| {
        (
            keep_restored_groups(&windows, &empty),
            keep_restored_groups(&windows, &desk()),
        )
    });
    assert_eq!(a, vec![]);
    assert!(!b.is_empty());
    let warn = expect_one(&events, WORK_AREA_UNRESOLVED_TAG);
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.expect_field("scene"), "\"restore-group\"");
    assert_eq!(warn.expect_field("reason"), "\"no-work-area\"");
    expect_one(&events, WORK_AREA_CLAMP_TAG);
}
