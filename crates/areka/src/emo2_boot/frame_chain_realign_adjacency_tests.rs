//! 拡大率の後の詰め直しが、変わる直前の隣り合いで組を詰める檻
//! （`dpi-realign-remembered-chain` 要件 1.1・1.4・1.8・8.1）。
//!
//! # ここが押さえるもの
//!
//! 記憶から戻した窓は既定の位置を持たない。詰め直しが既定の位置で対象を決めていたころは、
//! 2 回目以降の起動（全スコープが記憶から戻った形）で 200% → 100% に変えると隙間 359 が
//! 開いたまま残った。新しい決め方は変わる直前の矩形の隣り合いで決めるので、既定の位置が
//! 無くても隙間が閉じる。

use crate::placement::chain_finalize::{ADJACENT_TOLERANCE_PX, ChainFinalized};
use crate::placement::spawn::GhostWindows;

use super::shell_target;
use super::test_support::{FakeReports, FrameHarness, PerTargetSizes, pos_of, size_of};

/// 変わる前の拡大率（実機の 200%）。
const HIGH_DPI: u16 = 192;
/// 変わった後の拡大率（実機の 100%＝幅がちょうど半分）。
const LOW_DPI: u16 = 96;

/// scope0 の実表示寸（200%・実機の幅 764）。
const HIGH_SIZE_0: (u32, u32) = (764, 1094);
/// scope0 の実表示寸（100%）。
const LOW_SIZE_0: (u32, u32) = (382, 547);
/// scope1 の実表示寸（200%・実機の幅 672）。
const HIGH_SIZE_1: (u32, u32) = (672, 596);
/// scope1 の実表示寸（100%）。
const LOW_SIZE_1: (u32, u32) = (336, 298);

/// 詰め直しを入れないときに 100% で残る隙間（実機 359px の決定論版）。
const GAP_WITHOUT_REALIGN: i32 = 359;

fn high_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(HIGH_SIZE_0)), (1, Some(HIGH_SIZE_1))])
}

fn low_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(LOW_SIZE_0)), (1, Some(LOW_SIZE_1))])
}

/// 200% で起動して隣り合わせ、全スコープの既定の位置を外す（記憶から戻した形）。
fn boot_remembered_at_high(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(HIGH_DPI);
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
    source.refresh.insert(shell_target(0).0, HIGH_SIZE_0);
    source.refresh.insert(shell_target(1).0, HIGH_SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
    harness.run_chain_finalize(&high_sizes());
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "前提が崩れている: 起動の並べ直しが駆動していない"
    );
    assert_eq!(gap(harness), 0, "前提が崩れている: 200% で隣り合っていない");

    // 本番の起動が記憶から戻したスコープに行うのと同じ口で、既定の位置を外す。
    let scopes = harness.scopes().to_vec();
    let mut ghost_windows = harness.world.resource_mut::<GhostWindows>();
    for scope in scopes {
        assert!(ghost_windows.clear_default_char_pos(scope));
    }
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
}

/// 200% → 100% を 1 フレームで流し込む。
fn lower_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_table_for_dpi(LOW_DPI);
    harness.set_window_dpi(LOW_DPI);
    source.refresh.insert(shell_target(0).0, LOW_SIZE_0);
    source.refresh.insert(shell_target(1).0, LOW_SIZE_1);
    harness.advance_frame();
    harness.run_placement_phases(source);
}

/// キャラ窓の今の左上。
fn char_pos(harness: &FrameHarness, scope: usize) -> (i32, i32) {
    let p = pos_of(&harness.world, harness.char_window(scope)).expect("position がある");
    (p.x, p.y)
}

/// scope0 の左端 − scope1 の右端（scope1 が scope0 の左に隣り合う並び）。
fn gap(harness: &FrameHarness) -> i32 {
    let width1 = size_of(&harness.world, harness.char_window(1))
        .expect("size がある")
        .width;
    char_pos(harness, 0).0 - (char_pos(harness, 1).0 + width1)
}

/// 記憶から戻した（既定の位置の無い）形で 200% → 100% に変えても、隙間が許す幅以内に閉じ、
/// 縦の位置は変わらない（要件 1.1・1.4・1.8・8.1）。
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
    let y_before = (char_pos(&harness, 0).1, char_pos(&harness, 1).1);

    harness.run_chain_realign(&low_sizes());

    let gap_after = gap(&harness);
    assert!(
        gap_after.abs() <= ADJACENT_TOLERANCE_PX,
        "記憶から戻した組の隙間が閉じていない（隙間 {gap_after}・要件 1.8）"
    );
    assert_eq!(
        (char_pos(&harness, 0).1, char_pos(&harness, 1).1),
        y_before,
        "詰め直しが縦の位置を変えた（要件 1.4）"
    );
}
