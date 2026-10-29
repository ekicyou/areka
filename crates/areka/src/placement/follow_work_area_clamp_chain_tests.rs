//! 鎖を作業領域の中に置く純関数の核と World の口の檻（window-work-area-clamp・task 1.1・1.2・
//! 要件 10.1 ⑴⑵⑸⑹⑺・7.4・9.1・9.2）。
//!
//! 数値は開発者の机の形の合成: 主の画面（200%）の作業領域 `0..2880 × 0..1704`、その左に
//! 150% の画面。本体は主の画面の左端に立つ。

use bevy_ecs::prelude::*;

use super::super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::test_support::{point, rect, window_pos_sized};
use super::{
    ChainClampOutcome, ChainPlacement, ClampScene, MonitorSnapshot, WORK_AREA_CLAMP_TAG,
    WORK_AREA_KEEP_TAG, WORK_AREA_UNRESOLVED_TAG, place_chain_in_work_area,
    resolve_chain_in_work_area,
};
use crate::placement::chain_finalize::{ChainRect, ChainSlot, KeepReason, PairDecision};
use crate::placement::resolver::PointPx;

fn slot(scope: usize, r: ChainRect, decision: PairDecision) -> ChainSlot {
    ChainSlot {
        scope,
        current_x: r.x,
        width: r.w,
        decision,
    }
}

fn cr(x: i32, y: i32, w: i32, h: i32) -> ChainRect {
    ChainRect { x, y, w, h }
}

/// 主の画面と、その左の 150% の画面。
fn desk() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(0, 0, 2880, 1704), rect(-1920, 0, 0, 1032)],
    }
}

/// 本体（主の画面の左端）。
const HEAD: ChainRect = ChainRect {
    x: 0,
    y: 864,
    w: 640,
    h: 840,
};

fn run(
    rects: &[ChainRect],
    decisions: &[PairDecision],
    snap: &MonitorSnapshot,
) -> Vec<ChainPlacement> {
    let slots: Vec<ChainSlot> = rects
        .iter()
        .zip(decisions)
        .enumerate()
        .map(|(i, (r, d))| slot(i, *r, *d))
        .collect();
    resolve_chain_in_work_area(&slots, rects, snap)
}

/// 先頭（本体）は返さない＝動かさない（要件 1.3）。
fn assert_head_untouched(out: &[ChainPlacement]) {
    assert!(
        out.iter().all(|p| p.scope != 0),
        "本体が置き場に入った: {out:?}"
    );
}

/// ⑴ 相方を左へ並べると x=−1108 で左の画面へ渡る形 → 本体の右の隣へ並べ替える。
#[test]
fn joined_left_off_area_flips_to_right_neighbor() {
    let partner = cr(1000, 900, 1108, 800);
    let out = run(
        &[HEAD, partner],
        &[PairDecision::JoinLeft, PairDecision::JoinLeft],
        &desk(),
    );
    assert_head_untouched(&out);
    assert_eq!(
        out,
        vec![ChainPlacement {
            scope: 1,
            from: point(1000, 900),
            joined: point(-1108, 900),
            to: point(640, 900),
            outcome: ChainClampOutcome::Flipped,
        }]
    );
}

/// ⑴ 右にも入らない（作業領域が本体＋相方より狭い）→ 並べた位置を作業領域の端へ寄せる
/// （本体と重なってよい・要件 1.2）。
#[test]
fn neither_side_fits_clamps_to_area_edge() {
    let snap = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1500, 1704), rect(-1920, 0, 0, 1032)],
    };
    let partner = cr(1000, 900, 1108, 800);
    let out = run(
        &[HEAD, partner],
        &[PairDecision::JoinLeft, PairDecision::JoinLeft],
        &snap,
    );
    assert_head_untouched(&out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].joined, point(-1108, 900));
    assert_eq!(out[0].to, point(0, 900));
    assert_eq!(out[0].outcome, ChainClampOutcome::Edge);
}

/// ⑵ 3 体の鎖: 3 番目は並べ替えた**後の** 2 番目を基準に付く（要件 1.5）。
#[test]
fn third_joins_flipped_second() {
    let partner = cr(1000, 900, 1108, 800);
    let third = cr(2000, 900, 300, 700);
    let out = run(
        &[HEAD, partner, third],
        &[
            PairDecision::JoinLeft,
            PairDecision::JoinLeft,
            PairDecision::JoinRight,
        ],
        &desk(),
    );
    assert_head_untouched(&out);
    assert_eq!(out[0].to, point(640, 900));
    assert_eq!(out[0].outcome, ChainClampOutcome::Flipped);
    // 並べ替える前の 2 番目（x=−1108）に付けば 0 になる。
    assert_eq!(out[1].joined, point(640 + 1108, 900));
    assert_eq!(out[1].to, point(1748, 900));
    assert_eq!(out[1].outcome, ChainClampOutcome::Fits);
}

/// ⑸ 窓が作業領域より大きい → 左辺と上辺が作業領域の左辺と上辺に合う（要件 7.2）。
#[test]
fn oversized_window_aligns_left_and_top() {
    let snap = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1000, 700)],
    };
    let head = cr(0, 0, 640, 600);
    let partner = cr(500, 900, 1108, 800);
    let out = run(
        &[head, partner],
        &[PairDecision::JoinRight, PairDecision::JoinRight],
        &snap,
    );
    assert_eq!(out[0].to, point(0, 0));
    assert_eq!(out[0].outcome, ChainClampOutcome::Edge);
}

/// ⑹ 空のモニタの写し → 並べた位置のまま「取れない」（要件 7.4）。
#[test]
fn empty_snapshot_leaves_joined_unresolved() {
    let partner = cr(1000, 900, 1108, 800);
    let out = run(
        &[HEAD, partner],
        &[PairDecision::JoinLeft, PairDecision::JoinLeft],
        &MonitorSnapshot { work_areas: vec![] },
    );
    assert_head_untouched(&out);
    assert_eq!(out[0].to, point(-1108, 900));
    assert_eq!(out[0].to, out[0].joined);
    assert_eq!(out[0].outcome, ChainClampOutcome::Unresolved);
}

/// ⑺ 収まっている並び → 動かない（`Fits`・`to == from`）。
#[test]
fn fitting_chain_does_not_move() {
    let partner = cr(640, 900, 1108, 800);
    let out = run(
        &[HEAD, partner],
        &[PairDecision::JoinRight, PairDecision::JoinRight],
        &desk(),
    );
    assert_eq!(out[0].from, out[0].to);
    assert_eq!(out[0].to, out[0].joined);
    assert_eq!(out[0].outcome, ChainClampOutcome::Fits);
}

/// 横に入る並びは y だけを同じ式で収める（要件 7.2 は両軸・x は変えない）。
#[test]
fn fitting_x_clamps_only_y() {
    let partner = cr(640, 1000, 1108, 800);
    let out = run(
        &[HEAD, partner],
        &[PairDecision::JoinRight, PairDecision::JoinRight],
        &desk(),
    );
    assert_eq!(out[0].to, point(640, 1704 - 800));
    assert_eq!(out[0].outcome, ChainClampOutcome::Fits);
}

/// 組の決めが Keep の窓（台本で置いた窓・ドラッグの窓ほか）は今の位置のまま
/// （作業領域の外でも・モニタの写しが無くても・要件 1.4・2.3）。
#[test]
fn kept_window_stays_put() {
    let partner = cr(-1500, 900, 1108, 800);
    for snap in [desk(), MonitorSnapshot { work_areas: vec![] }] {
        for reason in [
            KeepReason::ExplicitlyPlaced,
            KeepReason::Dragging { scope: 1 },
            KeepReason::ScriptMovedAfterArm { scope: 1 },
        ] {
            let out = run(
                &[HEAD, partner],
                &[PairDecision::JoinLeft, PairDecision::Keep(reason)],
                &snap,
            );
            assert_eq!(out[0].to, point(-1500, 900));
            assert_eq!(out[0].from, out[0].to);
            assert_eq!(out[0].outcome, ChainClampOutcome::Kept(reason));
        }
    }
}

/// 空の入力・1 スロットは空列（パニックしない）。
#[test]
fn empty_and_single_slot_yield_nothing() {
    assert!(run(&[], &[], &desk()).is_empty());
    assert!(run(&[HEAD], &[PairDecision::JoinLeft], &desk()).is_empty());
}

/// 極端な座標でもパニックしない（飽和演算）。
#[test]
fn extreme_coordinates_do_not_panic() {
    let head = cr(i32::MIN, i32::MIN, i32::MAX, i32::MAX);
    let partner = cr(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    let _ = run(
        &[head, partner],
        &[PairDecision::JoinLeft, PairDecision::JoinLeft],
        &desk(),
    );
}

/// 記録の 3 語と場面の 5 語（要件 9.1・9.2）。
#[test]
fn vocabulary_words() {
    assert_eq!(WORK_AREA_CLAMP_TAG, "[work-area-clamp] Clamp");
    assert_eq!(WORK_AREA_KEEP_TAG, "[work-area-clamp] Keep");
    assert_eq!(WORK_AREA_UNRESOLVED_TAG, "[work-area-clamp] Unresolved");
    let words: Vec<&str> = [
        ClampScene::BootRepack,
        ClampScene::RememberedJoin,
        ClampScene::DpiRealign,
        ClampScene::SurfaceGrow,
        ClampScene::RestoreGroup,
    ]
    .into_iter()
    .map(ClampScene::as_str)
    .collect();
    assert_eq!(
        words,
        [
            "boot-repack",
            "remembered-join",
            "dpi-realign",
            "surface-grow",
            "restore-group"
        ]
    );
}

// -------------------------------------------------------------------------
// World の口（task 1.2）: 窓の高さを WindowPos から・作業領域を MonitorSnapshot 資源から読む
// -------------------------------------------------------------------------

/// 窓を `rects` の矩形で生やした World と、走査の形の `(scope, 窓, 今の位置)` と組のスロット。
fn world_chain(
    rects: &[ChainRect],
    decisions: &[PairDecision],
    snap: Option<MonitorSnapshot>,
) -> (World, Vec<ChainSlot>, Vec<(usize, Entity, PointPx)>) {
    let mut world = World::new();
    if let Some(snap) = snap {
        world.insert_resource(snap);
    }
    let targets = rects
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let e = world.spawn(window_pos_sized(r.x, r.y, r.w, r.h)).id();
            (i, e, point(r.x, r.y))
        })
        .collect();
    let slots = rects
        .iter()
        .zip(decisions)
        .enumerate()
        .map(|(i, (r, d))| slot(i, *r, *d))
        .collect();
    (world, slots, targets)
}

fn tagged<'a>(events: &'a [LogEvent], tag: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(tag))
        .collect()
}

/// World を組んでも核と同じ置き方になり（⑴⑵・3 体の鎖）、寄せた窓は `Clamp`（info・場面・
/// スコープ・how・from・to・作業領域）、収まっていた窓は `Keep`（debug・reason=fits）を残す。
#[test]
fn world_entry_places_like_core_and_logs_clamp_and_keep() {
    let rects = [HEAD, cr(1000, 900, 1108, 800), cr(2000, 900, 300, 700)];
    let decisions = [
        PairDecision::JoinLeft,
        PairDecision::JoinLeft,
        PairDecision::JoinRight,
    ];
    let (world, slots, targets) = world_chain(&rects, &decisions, Some(desk()));

    let (out, events) = capture_logs(|| {
        place_chain_in_work_area(&world, &slots, &targets, |_| ClampScene::BootRepack)
    });

    assert_eq!(out, run(&rects, &decisions, &desk()));
    assert_eq!(out[0].outcome, ChainClampOutcome::Flipped);

    let clamp = expect_one(&events, WORK_AREA_CLAMP_TAG);
    assert_eq!(clamp.level, tracing::Level::INFO);
    assert_eq!(clamp.expect_field("scene"), "\"boot-repack\"");
    assert_eq!(clamp.expect_field("scope"), "1");
    assert_eq!(clamp.expect_field("how"), "\"flipped\"");
    assert_eq!(
        clamp.expect_field("from"),
        format!("{:?}", point(1000, 900))
    );
    assert_eq!(clamp.expect_field("to"), format!("{:?}", point(640, 900)));
    assert_eq!(
        clamp.expect_field("work_area"),
        format!("{:?}", Some(rect(0, 0, 2880, 1704)))
    );

    let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
    assert_eq!(keep.level, tracing::Level::DEBUG);
    assert_eq!(keep.expect_field("scope"), "2");
    assert_eq!(keep.expect_field("reason"), "\"fits\"");
    assert!(tagged(&events, WORK_AREA_UNRESOLVED_TAG).is_empty());
}

/// 組の決めが Keep の窓は今の位置のまま `Keep`（reason は組の理由の語）。場面は `scene_of` が決める。
#[test]
fn world_entry_logs_kept_reason() {
    let rects = [HEAD, cr(-1500, 900, 1108, 800)];
    let decisions = [
        PairDecision::JoinLeft,
        PairDecision::Keep(KeepReason::ExplicitlyPlaced),
    ];
    let (world, slots, targets) = world_chain(&rects, &decisions, Some(desk()));

    let (out, events) = capture_logs(|| {
        place_chain_in_work_area(&world, &slots, &targets, |_| ClampScene::RememberedJoin)
    });

    assert_eq!(out[0].to, point(-1500, 900));
    let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
    assert_eq!(keep.expect_field("scene"), "\"remembered-join\"");
    assert_eq!(keep.expect_field("reason"), "\"explicitly-placed\"");
    assert!(tagged(&events, WORK_AREA_CLAMP_TAG).is_empty());
}

/// ⑹ モニタの写しの無い World → 並べた位置のまま `Unresolved` の warn が出る（要件 7.4）。
/// 対照: 同じ捕捉窓で写しの有る World を通すと warn は出ず `Clamp` が入る（捕捉が生きている）。
#[test]
fn world_without_snapshot_warns_unresolved_with_contrast() {
    let rects = [HEAD, cr(1000, 900, 1108, 800)];
    let decisions = [PairDecision::JoinLeft, PairDecision::JoinLeft];
    let (bare, slots, targets) = world_chain(&rects, &decisions, None);
    let (with_snap, slots2, targets2) = world_chain(&rects, &decisions, Some(desk()));

    let ((out, contrast), events) = capture_logs(|| {
        (
            place_chain_in_work_area(&bare, &slots, &targets, |_| ClampScene::DpiRealign),
            place_chain_in_work_area(&with_snap, &slots2, &targets2, |_| ClampScene::DpiRealign),
        )
    });

    assert_eq!(out[0].to, point(-1108, 900));
    assert_eq!(out[0].outcome, ChainClampOutcome::Unresolved);
    assert_eq!(contrast[0].outcome, ChainClampOutcome::Flipped);

    let warn = expect_one(&events, WORK_AREA_UNRESOLVED_TAG);
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.expect_field("scene"), "\"dpi-realign\"");
    assert_eq!(warn.expect_field("scope"), "1");
    assert_eq!(warn.expect_field("reason"), "\"no-monitor-snapshot\"");
    expect_one(&events, WORK_AREA_CLAMP_TAG);
}

/// 窓の高さが読めないスロットは `Unresolved`（no-window-height）で並べた位置に置き、
/// ほかのスロットは続ける。
#[test]
fn unreadable_height_is_unresolved_and_others_continue() {
    let rects = [HEAD, cr(1000, 900, 1108, 800), cr(2000, 900, 300, 700)];
    let decisions = [
        PairDecision::JoinLeft,
        PairDecision::JoinLeft,
        PairDecision::JoinRight,
    ];
    let (mut world, slots, targets) = world_chain(&rects, &decisions, Some(desk()));
    world
        .get_mut::<wintf::ecs::WindowPos>(targets[1].1)
        .unwrap()
        .size = None;

    let (out, events) = capture_logs(|| {
        place_chain_in_work_area(&world, &slots, &targets, |_| ClampScene::BootRepack)
    });

    assert_eq!(out[0].to, point(-1108, 900));
    assert_eq!(out[0].outcome, ChainClampOutcome::Unresolved);
    // 3 番目は並べた位置の 2 番目（x=−1108・左の画面）を基準に解かれ続ける: 右の隣（x=0）は
    // 左の画面に入らず左の隣へ並べ替え、y は左の画面の下端（1032）へ収まる。
    assert_eq!(out[1].joined, point(0, 900));
    assert_eq!(out[1].to, point(-1108 - 300, 1032 - 700));
    assert_eq!(out[1].outcome, ChainClampOutcome::Flipped);
    let warn = expect_one(&events, WORK_AREA_UNRESOLVED_TAG);
    assert_eq!(warn.expect_field("reason"), "\"no-window-height\"");
    assert_eq!(warn.expect_field("scope"), "1");
}
