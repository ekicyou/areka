//! 拡大率の後の詰め直しが、変わる直前の隣り合いで組を詰める檻。
//!
//! # ここが押さえるもの
//!
//! 記憶から戻した窓は既定の位置を持たない。詰め直しは変わる直前の矩形の隣り合いで対象を
//! 決めるので、2 回目以降の起動（全スコープが記憶から戻った形）で 200% → 100% に変えても、
//! 既定の位置が無くても隙間 359 が閉じる。
//!
//! 合わせて、向き（100% → 200% の重なり・往復）、解決の前の 2 度目の変化、右の組、3 体の鎖を
//! 確かめる。3 体の鎖はハーネス（2 スコープ固定）に乗らないので、窓の一式を 3 スコープで組み、
//! 武装と解決の関数を直に呼ぶ。

use areka_placement::chain_finalize::ADJACENT_TOLERANCE_PX;
use areka_placement::chain_realign::{
    arm_chain_realign, realign_chain_once_with, snapshot_char_rects,
};

use super::shared_test_support::{
    CHAIN3_BOTTOM, GAP_WITHOUT_REALIGN, HIGH_SIZE_0, LOW_DPI, LOW_SIZE_0, LOW_SIZE_1,
    boot_remembered_at, boot_remembered_at_high, chain3_pos, chain3_scan, chain3_world,
    change_scale, char_pos, char_width, gap, halve_keeping_bottom_center, high_sizes, low_sizes,
    lower_the_scale, place_char_window_x, raise_the_scale, sizes,
};
use super::test_support::{FakeReports, FrameHarness, capture_logs};

/// 中間の拡大率（150%）。解決の前に 2 度目の変化を起こすために使う。
const MID_DPI: u16 = 144;
/// scope0 の実表示寸（150%）。
const MID_SIZE_0: (u32, u32) = (573, 820);
/// scope1 の実表示寸（150%）。
const MID_SIZE_1: (u32, u32) = (504, 447);

/// scope1 の左端 − scope0 の右端（scope1 が scope0 の右に隣り合う並び）。
fn gap_right(harness: &FrameHarness) -> i32 {
    char_pos(harness, 1).0 - (char_pos(harness, 0).0 + char_width(harness, 0))
}

/// 両スコープの縦の位置。
fn ys(harness: &FrameHarness) -> (i32, i32) {
    (char_pos(harness, 0).1, char_pos(harness, 1).1)
}

/// 記憶から戻した（既定の位置の無い）形で 200% → 100% に変えても、隙間が許す幅以内に閉じ、
/// 縦の位置は変わらない。組の判断・動かした量・直前の隙間が記録に残る。
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
    assert_eq!(pair_lines.len(), 1, "組ごとに 1 行: {logs:?}");
    let line = pair_lines[0];
    for needle in [
        "decision=\"adjacent-left\"",
        // 変わる直前（200%）は隣り合っていた＝隙間 0。
        "before_gap=0 ",
        &format!("dx={GAP_WITHOUT_REALIGN}"),
    ] {
        assert!(line.contains(needle), "{needle} が無い: {line}");
    }

    let gap_after = gap(&harness);
    assert!(
        gap_after.abs() <= ADJACENT_TOLERANCE_PX,
        "記憶から戻した組の隙間が閉じていない（隙間 {gap_after}）"
    );
    assert_eq!(ys(&harness), y_before, "詰め直しが縦の位置を変えた");
}

/// 100% → 200% で幅が倍になって重なった組も、詰め直しで重なりが閉じる。
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
        "100% → 200% の重なりが閉じていない（隙間 {gap_after}）"
    );
    assert_eq!(ys(&harness), y_before, "詰め直しが縦の位置を変えた");
}

/// 200% → 100% → 200% の往復で、どちらの変化の後も組が隣り合い、本体は元の位置へ戻る
/// （遷移ごとに武装と解決が 1 往復する）。
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
/// その控えで組を詰める。2 度目の控え（100% の離れた矩形）で解くと離れたまま残る。
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
        "最初の控えで解いていない（2 度目の控えで「離れ」と決めた形・隙間 {gap_after}）"
    );
    assert_eq!(ys(&harness), y_before, "詰め直しが縦の位置を変えた");
}

/// 変わる直前に本体の右に隣り合っていた組は、詰め直しで右のまま詰まる（左へ回り込まない）。
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
        "右に隣り合っていた組が右のまま詰まっていない（右の隙間 {gap_after}）"
    );
    assert!(
        char_pos(&harness, 1).0 > char_pos(&harness, 0).0,
        "右に隣り合っていた組が左へ回り込んだ"
    );
    assert_eq!(ys(&harness), y_before, "詰め直しが縦の位置を変えた");
}

// ---------------------------------------------------------------------------
// 3 体の鎖
// ---------------------------------------------------------------------------

/// 3 体の鎖では、前の組を解いた後の x が次の組の基準になる。
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
        "3 体の鎖が前の結果を次の基準にして詰まっていない"
    );
    assert_eq!(
        (0..3)
            .map(|s| chain3_pos(&world, &gw, s).1)
            .collect::<Vec<_>>(),
        ys_before,
        "詰め直しが縦の位置を変えた"
    );
}

/// 詰め直した窓が作業領域の外になるときは、反対の側へ並べ替えて作業領域の中に置く。
///
/// 3 体の鎖を半分にした後（x: 1100／775／550・幅 200／150／100）、作業領域の左端を 1000 に
/// すると、scope1 を左に詰めた 950 は外へ出る。寄せは scope1 を本体の右の隣 1300 へ並べ替え、
/// 本体は動かさない。scope2 は並べ替えた scope1 の左の隣 1200 へ詰まり、収まるので寄せない
/// （本体と重なるのは許す・3 体以上の鎖は extra-character-windows の持ち場）。記録には場面
/// `dpi-realign` と `flipped` の寄せが scope1 に 1 行残る。
#[test]
fn a_realigned_window_outside_the_work_area_flips_to_the_other_side() {
    let (mut world, gw) = chain3_world();
    world.insert_resource(areka_placement::follow::MonitorSnapshot {
        work_areas: vec![areka_placement::resolver::RectPx {
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

    assert_eq!(chain3_pos(&world, &gw, 0).0, 1100, "本体が動いた");
    assert_eq!(
        chain3_pos(&world, &gw, 1).0,
        1300,
        "作業領域の外へ詰めた窓が本体の右の隣へ並べ替えられていない"
    );
    assert_eq!(
        chain3_pos(&world, &gw, 2).0,
        1200,
        "3 体目が並べ替えた scope1 の左の隣へ詰まっていない（本体と重なるのは許す。3 体以上の鎖の重なりはここでは見ない）"
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
        .filter(|l| l.contains(areka_placement::follow::WORK_AREA_CLAMP_TAG))
        .collect();
    assert_eq!(
        clamp
            .iter()
            .filter(|l| l.contains("scene=\"dpi-realign\"")
                && l.contains("scope=1")
                && l.contains("how=\"flipped\""))
            .count(),
        1,
        "詰め直しの寄せの記録が scope1 に 1 行でない: {logs:?}"
    );
    assert!(
        !clamp.iter().any(|l| l.contains("scope=2")
            && (l.contains("how=\"flipped\"") || l.contains("how=\"edge\""))),
        "収まっている scope2 が寄せられた: {logs:?}"
    );
}
