//! 鎖を作業領域の中に置く純関数の核の檻（window-work-area-clamp・task 1.1・要件 10.1 ⑴⑵⑸⑹⑺）。
//!
//! 数値は開発者の机の形の合成: 主の画面（200%）の作業領域 `0..2880 × 0..1704`、その左に
//! 150% の画面。本体は主の画面の左端に立つ。

use super::test_support::{point, rect};
use super::{
    ChainClampOutcome, ChainPlacement, ClampScene, MonitorSnapshot, WORK_AREA_CLAMP_TAG,
    WORK_AREA_KEEP_TAG, WORK_AREA_UNRESOLVED_TAG, resolve_chain_in_work_area,
};
use crate::placement::chain_finalize::{ChainRect, ChainSlot, KeepReason, PairDecision};

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
