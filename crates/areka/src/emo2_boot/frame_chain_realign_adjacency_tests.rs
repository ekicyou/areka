//! 拡大率の後の詰め直しが、変わる直前の隣り合いで組を詰める檻
//! （`dpi-realign-remembered-chain` 要件 1.1・1.3・1.4・1.5・1.7・1.8・8.1・8.4）。
//!
//! # ここが押さえるもの
//!
//! 記憶から戻した窓は既定の位置を持たない。詰め直しが既定の位置で対象を決めていたころは、
//! 2 回目以降の起動（全スコープが記憶から戻った形）で 200% → 100% に変えると隙間 359 が
//! 開いたまま残った。新しい決め方は変わる直前の矩形の隣り合いで決めるので、既定の位置が
//! 無くても隙間が閉じる。
//!
//! 合わせて、向き（100% → 200% の重なり・往復）、解決の前の 2 度目の変化、右の組、3 体の鎖を
//! 確かめる。3 体の鎖はハーネス（2 スコープ固定）に乗らないので、窓の一式を 3 スコープで組み、
//! 武装と解決の関数を直に呼ぶ。

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::World;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::{Point, WindowHandle, WindowPos};

use crate::placement::chain_finalize::{ADJACENT_TOLERANCE_PX, ChainFinalized, ScopeChainState};
use crate::placement::chain_realign::{
    arm_chain_realign, realign_chain_once_with, snapshot_char_rects,
};
use crate::placement::follow::OffsetBase;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};

use super::shell_target;
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, capture_logs, pos_of, size_of,
};

/// 変わる前の拡大率（実機の 200%）。
const HIGH_DPI: u16 = 192;
/// 変わった後の拡大率（実機の 100%＝幅がちょうど半分）。
const LOW_DPI: u16 = 96;
/// 中間の拡大率（150%）。解決の前に 2 度目の変化を起こすために使う。
const MID_DPI: u16 = 144;

/// scope0 の実表示寸（200%・実機の幅 764）。
const HIGH_SIZE_0: (u32, u32) = (764, 1094);
/// scope0 の実表示寸（100%）。
const LOW_SIZE_0: (u32, u32) = (382, 547);
/// scope0 の実表示寸（150%）。
const MID_SIZE_0: (u32, u32) = (573, 820);
/// scope1 の実表示寸（200%・実機の幅 672）。
const HIGH_SIZE_1: (u32, u32) = (672, 596);
/// scope1 の実表示寸（100%）。
const LOW_SIZE_1: (u32, u32) = (336, 298);
/// scope1 の実表示寸（150%）。
const MID_SIZE_1: (u32, u32) = (504, 447);

/// 詰め直しを入れないときに 100% で残る隙間（実機 359px の決定論版）。
pub(super) const GAP_WITHOUT_REALIGN: i32 = 359;

fn sizes(s0: (u32, u32), s1: (u32, u32)) -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(s0)), (1, Some(s1))])
}

fn high_sizes() -> PerTargetSizes {
    sizes(HIGH_SIZE_0, HIGH_SIZE_1)
}

pub(super) fn low_sizes() -> PerTargetSizes {
    sizes(LOW_SIZE_0, LOW_SIZE_1)
}

/// `dpi` で起動して scope1 を scope0 の左に隣り合わせ、全スコープの既定の位置を外す
/// （記憶から戻した形）。
fn boot_remembered_at(
    harness: &mut FrameHarness,
    source: &mut FakeReports,
    dpi: u16,
    s0: (u32, u32),
    s1: (u32, u32),
) {
    harness.set_monitor_sources_for_dpi(dpi);
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    source.refresh.insert(shell_target(0).0, s0);
    source.refresh.insert(shell_target(1).0, s1);
    harness.advance_frame();
    harness.run_placement_phases(source);
    harness.run_chain_finalize(&sizes(s0, s1));
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "前提が崩れている: 起動の並べ直しが駆動していない"
    );
    assert_eq!(
        gap(harness),
        0,
        "前提が崩れている: 起動の拡大率で隣り合っていない"
    );

    // 本番の起動が記憶から戻したスコープに行うのと同じ口で、既定の位置を外す。
    let scopes = harness.scopes().to_vec();
    let mut ghost_windows = harness.world.resource_mut::<GhostWindows>();
    for scope in scopes {
        assert!(ghost_windows.clear_default_char_pos(scope));
    }
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
}

/// 200% で起動して隣り合わせ、全スコープの既定の位置を外す（記憶から戻した形）。
pub(super) fn boot_remembered_at_high(harness: &mut FrameHarness, source: &mut FakeReports) {
    boot_remembered_at(harness, source, HIGH_DPI, HIGH_SIZE_0, HIGH_SIZE_1);
}

/// 拡大率の変化を 1 フレームで流し込む。
fn change_scale(
    harness: &mut FrameHarness,
    source: &mut FakeReports,
    dpi: u16,
    s0: (u32, u32),
    s1: (u32, u32),
) {
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    source.refresh.insert(shell_target(0).0, s0);
    source.refresh.insert(shell_target(1).0, s1);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// 200% → 100% を 1 フレームで流し込む。
pub(super) fn lower_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    change_scale(harness, source, LOW_DPI, LOW_SIZE_0, LOW_SIZE_1);
}

/// 100% → 200% を 1 フレームで流し込む。
fn raise_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    change_scale(harness, source, HIGH_DPI, HIGH_SIZE_0, HIGH_SIZE_1);
}

/// キャラ窓の左上の x だけをその値に置く（ドラッグや台本の結果の位置。既定の位置も印も触らない）。
pub(super) fn place_char_window_x(harness: &mut FrameHarness, scope: usize, x: i32) {
    let window = harness.char_window(scope);
    let mut wp = harness
        .world
        .get_mut::<WindowPos>(window)
        .expect("WindowPos がある");
    let wp = wp.bypass_change_detection();
    let y = wp.position.expect("position がある").y;
    wp.position = Some(Point { x, y });
}

/// キャラ窓の今の左上。
pub(super) fn char_pos(harness: &FrameHarness, scope: usize) -> (i32, i32) {
    let p = pos_of(&harness.world, harness.char_window(scope)).expect("position がある");
    (p.x, p.y)
}

/// キャラ窓の今の幅。
fn char_width(harness: &FrameHarness, scope: usize) -> i32 {
    size_of(&harness.world, harness.char_window(scope))
        .expect("size がある")
        .width
}

/// scope0 の左端 − scope1 の右端（scope1 が scope0 の左に隣り合う並び）。
pub(super) fn gap(harness: &FrameHarness) -> i32 {
    char_pos(harness, 0).0 - (char_pos(harness, 1).0 + char_width(harness, 1))
}

/// scope1 の左端 − scope0 の右端（scope1 が scope0 の右に隣り合う並び）。
fn gap_right(harness: &FrameHarness) -> i32 {
    char_pos(harness, 1).0 - (char_pos(harness, 0).0 + char_width(harness, 0))
}

/// 両スコープの縦の位置。
fn ys(harness: &FrameHarness) -> (i32, i32) {
    (char_pos(harness, 0).1, char_pos(harness, 1).1)
}

/// 記憶から戻した（既定の位置の無い）形で 200% → 100% に変えても、隙間が許す幅以内に閉じ、
/// 縦の位置は変わらない。組の判断・動かした量・直前の隙間が記録に残る（要件 1.1・1.4・1.8・7.1・8.1）。
#[test]
fn remembered_windows_close_the_gap_after_200_to_100() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at_high(&mut harness, &mut source);

    lower_the_scale(&mut harness, &mut source);
    assert_eq!(
        gap(&harness),
        GAP_WITHOUT_REALIGN,
        "前提が崩れている: 変わった直後に隙間が開いていない（詰め直しを観測できない）"
    );
    let y_before = ys(&harness);

    let logs = capture_logs(|| harness.run_chain_realign(&low_sizes()));

    let pair_lines: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("拡大率の後の詰め直しで組の判断を下した"))
        .collect();
    assert_eq!(pair_lines.len(), 1, "組ごとに 1 行（要件 7.1）: {logs:?}");
    let line = pair_lines[0];
    for needle in [
        "decision=\"adjacent-left\"",
        // 変わる直前（200%）は隣り合っていた＝隙間 0。
        "before_gap=0 ",
        &format!("dx={GAP_WITHOUT_REALIGN}"),
    ] {
        assert!(line.contains(needle), "{needle} が無い（要件 7.1）: {line}");
    }

    let gap_after = gap(&harness);
    assert!(
        gap_after.abs() <= ADJACENT_TOLERANCE_PX,
        "記憶から戻した組の隙間が閉じていない（隙間 {gap_after}・要件 1.8）"
    );
    assert_eq!(
        ys(&harness),
        y_before,
        "詰め直しが縦の位置を変えた（要件 1.4）"
    );
}

/// 100% → 200% で幅が倍になって重なった組も、詰め直しで重なりが閉じる（要件 1.3・8.1）。
#[test]
fn the_overlap_closes_after_100_to_200() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at(&mut harness, &mut source, LOW_DPI, LOW_SIZE_0, LOW_SIZE_1);

    raise_the_scale(&mut harness, &mut source);
    assert_eq!(
        gap(&harness),
        -GAP_WITHOUT_REALIGN,
        "前提が崩れている: 変わった直後に重なっていない（詰め直しを観測できない）"
    );
    let y_before = ys(&harness);

    harness.run_chain_realign(&high_sizes());

    let gap_after = gap(&harness);
    assert!(
        gap_after.abs() <= ADJACENT_TOLERANCE_PX,
        "100% → 200% の重なりが閉じていない（隙間 {gap_after}・要件 1.3）"
    );
    assert_eq!(
        ys(&harness),
        y_before,
        "詰め直しが縦の位置を変えた（要件 1.4）"
    );
}

/// 200% → 100% → 200% の往復で、どちらの変化の後も組が隣り合い、本体は元の位置へ戻る
/// （遷移ごとに武装と解決が 1 往復する・要件 1.3・8.1）。
#[test]
fn a_round_trip_200_100_200_keeps_the_pair_adjacent() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at_high(&mut harness, &mut source);
    let start = (char_pos(&harness, 0), char_pos(&harness, 1));

    lower_the_scale(&mut harness, &mut source);
    harness.run_chain_realign(&low_sizes());
    let gap_low = gap(&harness);
    assert!(
        gap_low.abs() <= ADJACENT_TOLERANCE_PX,
        "往復の行き（200% → 100%）で隙間が閉じていない（隙間 {gap_low}）"
    );

    raise_the_scale(&mut harness, &mut source);
    assert_ne!(
        gap(&harness),
        0,
        "前提が崩れている: 帰りの変化の直後に重なりが出ていない（2 度目の詰め直しを観測できない）"
    );
    harness.run_chain_realign(&high_sizes());
    let gap_high = gap(&harness);
    assert!(
        gap_high.abs() <= ADJACENT_TOLERANCE_PX,
        "往復の帰り（100% → 200%）で重なりが閉じていない（隙間 {gap_high}）"
    );
    assert_eq!(
        (char_pos(&harness, 0), char_pos(&harness, 1)),
        start,
        "往復の後に組が起動したときの位置へ戻っていない"
    );
}

/// 解決の前に 2 度目の変化（100% → 150%）が来ても、武装は最初の控え（200% の隣り合い）を保ち、
/// その控えで組を詰める（要件 1.7）。2 度目の控え（100% の離れた矩形）で解くと離れたまま残る。
#[test]
fn a_second_change_before_resolution_resolves_with_the_first_snapshot() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at_high(&mut harness, &mut source);

    lower_the_scale(&mut harness, &mut source);
    assert_eq!(
        gap(&harness),
        GAP_WITHOUT_REALIGN,
        "前提が崩れている: 1 度目の変化の直後に隙間が開いていない"
    );
    // 解決の前に 2 度目の変化。100% の隙間 359 は許す幅を超える＝この時点の矩形は「離れ」。
    change_scale(&mut harness, &mut source, MID_DPI, MID_SIZE_0, MID_SIZE_1);
    let gap_mid = gap(&harness);
    assert!(
        gap_mid > ADJACENT_TOLERANCE_PX,
        "前提が崩れている: 2 度目の変化の直後に隙間が残っていない（隙間 {gap_mid}）"
    );
    let y_before = ys(&harness);

    harness.run_chain_realign(&sizes(MID_SIZE_0, MID_SIZE_1));

    let gap_after = gap(&harness);
    assert!(
        gap_after.abs() <= ADJACENT_TOLERANCE_PX,
        "最初の控えで解いていない（2 度目の控えで「離れ」と決めた形・隙間 {gap_after}・要件 1.7）"
    );
    assert_eq!(
        ys(&harness),
        y_before,
        "詰め直しが縦の位置を変えた（要件 1.4）"
    );
}

/// 変わる直前に本体の右に隣り合っていた組は、詰め直しで右のまま詰まる（左へ回り込まない・要件 1.3）。
#[test]
fn a_pair_adjacent_on_the_right_stays_on_the_right() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at_high(&mut harness, &mut source);

    // scope1 を scope0 の右に付け直す（作業領域に収まるよう本体ごと左へ寄せる）。
    place_char_window_x(&mut harness, 0, 600);
    place_char_window_x(&mut harness, 1, 600 + HIGH_SIZE_0.0 as i32);
    assert_eq!(
        gap_right(&harness),
        0,
        "前提が崩れている: 右に隣り合っていない"
    );

    lower_the_scale(&mut harness, &mut source);
    assert_eq!(
        gap_right(&harness),
        GAP_WITHOUT_REALIGN,
        "前提が崩れている: 変わった直後に右の隙間が開いていない"
    );
    let y_before = ys(&harness);

    harness.run_chain_realign(&low_sizes());

    let gap_after = gap_right(&harness);
    assert!(
        gap_after.abs() <= ADJACENT_TOLERANCE_PX,
        "右に隣り合っていた組が右のまま詰まっていない（右の隙間 {gap_after}・要件 1.3）"
    );
    assert!(
        char_pos(&harness, 1).0 > char_pos(&harness, 0).0,
        "右に隣り合っていた組が左へ回り込んだ"
    );
    assert_eq!(
        ys(&harness),
        y_before,
        "詰め直しが縦の位置を変えた（要件 1.4）"
    );
}

// ---------------------------------------------------------------------------
// 3 体の鎖（要件 1.5・8.4）
// ---------------------------------------------------------------------------

/// 3 体の鎖の作業領域の下端（3 体とも下端に接地する）。
const CHAIN3_BOTTOM: i32 = 1444;

/// 3 体の鎖の 200% の矩形（x, w, h）。scope1 は scope0 の左、scope2 は scope1 の左に隣り合う。
const CHAIN3_HIGH: [(i32, i32, i32); 3] = [(1000, 400, 600), (700, 300, 400), (500, 200, 300)];

fn chain3_placement(scope: usize, (x, w, h): (i32, i32, i32)) -> ScopePlacement {
    let boff = PointPx { x: 0, y: 0 };
    ScopePlacement {
        scope,
        char_pos: PointPx {
            x,
            y: CHAIN3_BOTTOM - h,
        },
        char_size: SizePx { w, h },
        balloon_pos: PointPx {
            x,
            y: CHAIN3_BOTTOM - h,
        },
        balloon_size: SizePx { w: 200, h: 150 },
        balloon_offset: boff,
        balloon_offset_base: OffsetBase::unpinned(boff),
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 3 スコープの窓の一式（偽 WindowHandle 付き・起動の並べ直し済み）。
pub(super) fn chain3_world() -> (World, GhostWindows) {
    let placements: Vec<ScopePlacement> = CHAIN3_HIGH
        .iter()
        .enumerate()
        .map(|(scope, &rect)| chain3_placement(scope, rect))
        .collect();
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles((0..3).map(|scope| (scope, format!("s{scope}")))),
    );
    let mut raw = 0x100usize;
    for scope in 0..3 {
        for e in [
            gw.char_window(scope).expect("キャラ窓"),
            gw.balloon_window(scope).expect("バルーン窓"),
        ] {
            world.entity_mut(e).insert(WindowHandle {
                hwnd: HWND(raw as *mut _),
                instance: HINSTANCE::default(),
            });
            raw += 0x10;
        }
    }
    world.insert_resource(ChainFinalized);
    (world, gw)
}

/// 幅と高さを半分にし、下端中央を保って置き直す（拡大率の相の再射影の結果の形）。
pub(super) fn halve_keeping_bottom_center(world: &mut World, gw: &GhostWindows) {
    for scope in 0..3 {
        let mut wp = world
            .get_mut::<WindowPos>(gw.char_window(scope).expect("キャラ窓"))
            .expect("WindowPos がある");
        let wp = wp.bypass_change_detection();
        let (pos, size) = wp.position.zip(wp.size).expect("位置と大きさがある");
        let (w, h) = (size.width / 2, size.height / 2);
        let center = pos.x + size.width / 2;
        wp.position = Some(Point {
            x: center - w / 2,
            y: pos.y + size.height - h,
        });
        wp.size = Some(wintf::ecs::SizeI::new(w, h));
    }
}

/// 今の窓の位置と幅から解きの入力を作る（本番の走査と同じ形・既定の位置は無い）。
pub(super) fn chain3_scan(
    world: &World,
) -> (
    Vec<ScopeChainState>,
    Vec<(usize, bevy_ecs::entity::Entity, PointPx)>,
) {
    let gw = world.resource::<GhostWindows>();
    let mut states = Vec::new();
    let mut targets = Vec::new();
    for scope in gw.scopes() {
        let window = gw.char_window(scope).expect("キャラ窓");
        let wp = world.get::<WindowPos>(window).expect("WindowPos がある");
        let (pos, size) = wp.position.zip(wp.size).expect("位置と大きさがある");
        states.push(ScopeChainState {
            scope,
            current_x: pos.x,
            width: size.width,
            default_x: None,
        });
        targets.push((scope, window, PointPx { x: pos.x, y: pos.y }));
    }
    (states, targets)
}

pub(super) fn chain3_pos(world: &World, gw: &GhostWindows, scope: usize) -> (i32, i32) {
    let p = pos_of(world, gw.char_window(scope).expect("キャラ窓")).expect("position がある");
    (p.x, p.y)
}

/// 3 体の鎖では、前の組を解いた後の x が次の組の基準になる（要件 1.5・8.4）。
///
/// 200% の並び（x: 1000／700／500・幅 400／300／200）を半分にすると、下端中央を保って
/// x は 1100／775／550 になる。詰め直しは scope1 を 1100 − 150 = 950、scope2 を「scope1 の
/// 解いた後の x」950 − 100 = 850 へ置く。今の x（775）を基準にすると 675 になって隙間が残る。
#[test]
fn a_three_body_chain_uses_the_previous_result_as_the_next_base() {
    let (mut world, gw) = chain3_world();
    let before = snapshot_char_rects(&world);
    assert_eq!(
        before.len(),
        3,
        "前提が崩れている: 3 体の矩形を控えられていない"
    );

    halve_keeping_bottom_center(&mut world, &gw);
    arm_chain_realign(&mut world, before);
    assert_eq!(
        (0..3)
            .map(|s| chain3_pos(&world, &gw, s).0)
            .collect::<Vec<_>>(),
        vec![1100, 775, 550],
        "前提が崩れている: 半分にした後の x が想定と違う"
    );
    let ys_before: Vec<i32> = (0..3).map(|s| chain3_pos(&world, &gw, s).1).collect();

    realign_chain_once_with(&mut world, |world| Ok(chain3_scan(world)));

    assert_eq!(
        (0..3)
            .map(|s| chain3_pos(&world, &gw, s).0)
            .collect::<Vec<_>>(),
        vec![1100, 950, 850],
        "3 体の鎖が前の結果を次の基準にして詰まっていない（要件 1.5）"
    );
    assert_eq!(
        (0..3)
            .map(|s| chain3_pos(&world, &gw, s).1)
            .collect::<Vec<_>>(),
        ys_before,
        "詰め直しが縦の位置を変えた（要件 1.4）"
    );
}

/// 詰め直した窓が作業領域の外になるときは、反対の側へ並べ替えて作業領域の中に置く
/// （window-work-area-clamp 要件 2.1・2.3・7.7）。
///
/// 3 体の鎖を半分にした後（x: 1100／775／550・幅 200／150／100）、作業領域の左端を 1000 に
/// すると、scope1 を左に詰めた 950 は外へ出る。寄せは scope1 を本体の右の隣 1300 へ並べ替え、
/// 本体は動かさない。scope2 は並べ替えた scope1 の左の隣 1200 へ詰まり、収まるので寄せない
/// （本体と重なるのは許す・3 体以上の鎖は extra-character-windows の持ち場）。記録には場面
/// `dpi-realign` と `flipped` の寄せが scope1 に 1 行残る。
#[test]
fn a_realigned_window_outside_the_work_area_flips_to_the_other_side() {
    let (mut world, gw) = chain3_world();
    world.insert_resource(crate::placement::follow::MonitorSnapshot {
        work_areas: vec![crate::placement::resolver::RectPx {
            left: 1000,
            top: 0,
            right: 2000,
            bottom: CHAIN3_BOTTOM,
        }],
    });
    let before = snapshot_char_rects(&world);
    halve_keeping_bottom_center(&mut world, &gw);
    arm_chain_realign(&mut world, before);
    assert_eq!(
        chain3_pos(&world, &gw, 1).0,
        775,
        "前提が崩れている: 半分にした後の scope1 の x が想定と違う"
    );
    let ys_before: Vec<i32> = (0..3).map(|s| chain3_pos(&world, &gw, s).1).collect();

    let logs = capture_logs(|| realign_chain_once_with(&mut world, |world| Ok(chain3_scan(world))));

    assert_eq!(
        chain3_pos(&world, &gw, 0).0,
        1100,
        "本体が動いた（要件 2.3）"
    );
    assert_eq!(
        chain3_pos(&world, &gw, 1).0,
        1300,
        "作業領域の外へ詰めた窓が本体の右の隣へ並べ替えられていない（要件 2.1）"
    );
    assert_eq!(
        chain3_pos(&world, &gw, 2).0,
        1200,
        "3 体目が並べ替えた scope1 の左の隣へ詰まっていない（design テスト⑵・要件 1.5。本体と重なるのは許す＝design Risks の 3 体以上の鎖は extra-character-windows の持ち場）"
    );
    assert_eq!(
        (0..3)
            .map(|s| chain3_pos(&world, &gw, s).1)
            .collect::<Vec<_>>(),
        ys_before,
        "縦に収まっている窓の縦の位置が変わった"
    );
    let clamp: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains(crate::placement::follow::WORK_AREA_CLAMP_TAG))
        .collect();
    assert!(
        clamp.iter().any(|l| l.contains("scene=\"dpi-realign\"")
            && l.contains("scope=1")
            && l.contains("how=\"flipped\"")),
        "詰め直しの寄せの記録が無い（要件 7.7）: {logs:?}"
    );
    assert!(
        !clamp.iter().any(|l| l.contains("scope=2")
            && (l.contains("how=\"flipped\"") || l.contains("how=\"edge\""))),
        "収まっている scope2 が寄せられた: {logs:?}"
    );
}
