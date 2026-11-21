//! 記憶から戻した組の寄せの檻。
//!
//! 数値は開発者の机の形の合成: 主の画面の作業領域 `0..2880 × 0..1704`。記憶の位置は
//! 右に外したモニタ（x=5000 から）で、今のどのモニタとも交わらない。`shown` は窓ごとの判定が
//! 寄せた結果（見えている窓は `shown == projected`）を手で置く。

use super::test_support::{guard_events, point, rect};
use super::work_area_clamp::{WORK_AREA_KEEP_TAG, WORK_AREA_UNRESOLVED_TAG};
use super::{
    GroupShift, MonitorSnapshot, RestoredWindow, WORK_AREA_CLAMP_TAG, keep_restored_groups,
};
use crate::chain_finalize::{AdjacencyMark, ChainRect};
use crate::chain_rule::{ChainRule, ChainTerms};
use crate::resolver::Anchor::{self, Bottom, Right};
use crate::resolver::PointPx;
use log_capture_kit::{ExpectField, capture, expect_one};

fn desk() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(0, 0, 2880, 1704)],
    }
}

/// 窓の一式の鎖（スコープ, descript の揃え方）。記憶の戻しと同じく今の揃え方も descript のまま。
fn chain(scopes: &[(usize, Anchor)]) -> ChainRule {
    ChainRule::from_terms(
        scopes
            .iter()
            .map(|&(scope, anchor)| {
                let terms = ChainTerms {
                    descript: anchor,
                    default_x: None,
                    live: anchor,
                };
                (scope, terms)
            })
            .collect(),
    )
}

/// スコープ 0 と 1 がどちらも下端の鎖（組 (0, 1) は鎖の前の窓との組）。
fn pair() -> ChainRule {
    chain(&[(0, Bottom), (1, Bottom)])
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

/// 全員が外の、隣り合っていた組（印 right）と重なっていた組（印 none）は、寄せた後も互いの
/// 差が同じで、和の矩形が作業領域に入る。`Clamp`（how=shifted）が残る。
#[test]
fn groups_all_outside_shift_together() {
    let cases = [
        // 隣り合っていた組（印 right・記憶の矩形も右に隣り合う）
        [
            win(0, point(5000, 864), point(2240, 864), None),
            win(
                1,
                point(5640, 864),
                point(2240, 864),
                Some(AdjacencyMark::Right),
            ),
        ],
        // 重なっていた組（印 none・記憶の矩形が重なる）
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
        let (shifts, events) = capture(|| keep_restored_groups(&windows, &pair(), &desk()));
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
        let clamps = guard_events(&events, WORK_AREA_CLAMP_TAG);
        assert_eq!(clamps.len(), shifts.len());
        assert!(!shifts.is_empty());
        assert_eq!(clamps[0].level, tracing::Level::INFO);
        assert_eq!(clamps[0].expect_field("scene"), "\"restore-group\"");
        assert_eq!(clamps[0].expect_field("how"), "\"shifted\"");
    }
}

/// 見えている窓のある組は見えている窓を動かさず、外の窓は窓ごとの結果のまま
/// （`Keep` reason=partner-visible）。
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
    let (shifts, events) = capture(|| keep_restored_groups(&windows, &pair(), &desk()));
    assert_eq!(shifts, vec![]);
    let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
    assert_eq!(keep.level, tracing::Level::DEBUG);
    assert_eq!(keep.expect_field("reason"), "\"partner-visible\"");
    assert_eq!(keep.expect_field("scope"), "1");
}

/// 離れていた組は組にしない。0・1・2 が鎖に入り 1 が記憶に無いと、2 の印 `right` は鎖の前の
/// 窓 1（既定の位置のまま・起動の確定が付け直す）との組の印なので、記憶の矩形が隣り合っていても
/// 0 と 2 を組にしない（窓ごとの結果のまま）。記録もしない。
#[test]
fn apart_pair_or_pair_with_missing_chain_partner_is_not_grouped() {
    let apart = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(7000, 864),
            point(2240, 864),
            Some(AdjacencyMark::Separate),
        ),
    ];
    let missing = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            2,
            point(5640, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let three = chain(&[(0, Bottom), (1, Bottom), (2, Bottom)]);
    let ((a, b), events) = capture(|| {
        (
            keep_restored_groups(&apart, &pair(), &desk()),
            keep_restored_groups(&missing, &three, &desk()),
        )
    });
    assert_eq!((a, b), (vec![], vec![]));
    assert!(
        guard_events(&events, "[work-area-clamp]").is_empty(),
        "{events:?}"
    );
}

/// 中のスコープが無い（0 と 2 だけ）ゴーストで 2 の印が `right` なら、鎖の前の窓 0 と 2 を
/// 1 つの組として同じ量だけ寄せる。
#[test]
fn missing_middle_scope_groups_chain_neighbours() {
    let windows = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            2,
            point(5640, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let shifts = keep_restored_groups(&windows, &chain(&[(0, Bottom), (2, Bottom)]), &desk());
    // 和の矩形 5000..6280 を右端 2880 へ収める差（−3400）を 2 枚に足す。
    assert_eq!(
        placed(&windows, &shifts),
        vec![point(1600, 864), point(2240, 864)]
    );
}

/// 0 下端・1 右の辺・2 下端で 2 の印が `right` なら、辺を飛ばした鎖の前の窓 0 と 2 を
/// 組にし、辺のスコープ 1 と 2 を組にしない（1 は窓ごとの結果のまま）。
#[test]
fn edge_scope_is_skipped_for_the_mark_partner() {
    let windows = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(9000, 0),
            point(2240, 0),
            Some(AdjacencyMark::Separate),
        ),
        win(
            2,
            point(5640, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let rule = chain(&[(0, Bottom), (1, Right), (2, Bottom)]);
    let shifts = keep_restored_groups(&windows, &rule, &desk());
    assert_eq!(
        placed(&windows, &shifts),
        vec![point(1600, 864), point(2240, 0), point(2240, 864)]
    );
}

/// 1 が右の辺で記憶の矩形が 2 と離れていれば、2 の印が `right` でも 1 と 2 を組にしない
/// （印は鎖の前の窓 0 との組の印・0 は記憶に無い）。
#[test]
fn edge_scope_apart_from_next_is_not_grouped_by_mark() {
    let windows = [
        win(
            1,
            point(9000, 0),
            point(2240, 0),
            Some(AdjacencyMark::Separate),
        ),
        win(
            2,
            point(5640, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let rule = chain(&[(0, Bottom), (1, Right), (2, Bottom)]);
    let (shifts, events) = capture(|| keep_restored_groups(&windows, &rule, &desk()));
    assert_eq!(shifts, vec![]);
    assert!(
        guard_events(&events, "[work-area-clamp]").is_empty(),
        "{events:?}"
    );
}

/// 辺のスコープ 1（右の辺・鎖に入らない）が印 `right` を持っていても、印の線で鎖の前の窓 0 と
/// 組にしない（印の線は n が鎖に入るときだけ・記憶の矩形も離れている）。記録もしない。
#[test]
fn edge_scope_mark_does_not_group_with_chain_prev() {
    let windows = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(9000, 0),
            point(2240, 0),
            Some(AdjacencyMark::Right),
        ),
    ];
    let rule = chain(&[(0, Bottom), (1, Right)]);
    let (shifts, events) = capture(|| keep_restored_groups(&windows, &rule, &desk()));
    assert_eq!(shifts, vec![]);
    assert!(
        guard_events(&events, "[work-area-clamp]").is_empty(),
        "{events:?}"
    );
}

/// 0 が右の辺でも、番号の続く 1 と記憶の矩形が重なっていれば矩形で組にする（矩形の組は
/// 辺のスコープにも当たる）。
#[test]
fn edge_scope_overlapping_next_is_grouped_by_rect() {
    let windows = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(5300, 800),
            point(2240, 800),
            Some(AdjacencyMark::Separate),
        ),
    ];
    let shifts = keep_restored_groups(&windows, &chain(&[(0, Right), (1, Bottom)]), &desk());
    // 和の矩形 5000..5940 を右端 2880 へ収める差（−3060）を 2 枚に足す。
    assert_eq!(
        placed(&windows, &shifts),
        vec![point(1940, 864), point(2240, 800)]
    );
}

/// どれも見えている組は動かさず `Keep`（reason=remembered-visible）を残す。
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
    let (shifts, events) = capture(|| keep_restored_groups(&windows, &pair(), &desk()));
    assert_eq!(shifts, vec![]);
    let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
    assert_eq!(keep.expect_field("reason"), "\"remembered-visible\"");
}

/// 和の矩形が作業領域より大きいと左辺と上辺が合い、互いの差は保たれる。
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
    let shifts = keep_restored_groups(&windows, &pair(), &small);
    assert_eq!(placed(&windows, &shifts), vec![point(0, 0), point(640, 0)]);
}

/// モニタの写しが空だと返さず `Unresolved` の warn が出る。対照: 同じ捕捉窓で
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
    let ((a, b), events) = capture(|| {
        (
            keep_restored_groups(&windows, &pair(), &empty),
            keep_restored_groups(&windows, &pair(), &desk()),
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

/// 記憶の矩形が離れていても（隙間 60）印が `right` なら組として繋がり、同じ量だけ
/// 寄せられる。印の腕が無いと窓ごとの結果のまま 2 枚が重なる。
#[test]
fn separated_rects_joined_by_right_mark_shift_together() {
    let windows = [
        win(0, point(5000, 864), point(2240, 864), None),
        win(
            1,
            point(5700, 864),
            point(2240, 864),
            Some(AdjacencyMark::Right),
        ),
    ];
    let shifts = keep_restored_groups(&windows, &pair(), &desk());
    // 和の矩形 5000..6340 を右端 2880 へ収める差（−3460）を 2 枚に足す。
    assert_eq!(
        placed(&windows, &shifts),
        vec![point(1540, 864), point(2240, 864)]
    );
}

/// 印が `left`（n が鎖の前の窓の左）で記憶の矩形が離れていても同じく組になる。
#[test]
fn separated_rects_joined_by_left_mark_shift_together() {
    let windows = [
        win(0, point(5700, 864), point(2240, 864), None),
        win(
            1,
            point(5000, 864),
            point(2240, 864),
            Some(AdjacencyMark::Left),
        ),
    ];
    let shifts = keep_restored_groups(&windows, &pair(), &desk());
    assert_eq!(
        placed(&windows, &shifts),
        vec![point(2240, 864), point(1540, 864)]
    );
}
