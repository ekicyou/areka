//! frame のテストのうち `areka-runtime` と本体の両方で使う補助（`frame_move_order_tests.rs` の結線と定常のハーネス・
//! `frame_chain_realign_adjacency_tests.rs` の記憶から戻した組の起動と拡大率の変化）。`frame_test_support.rs` は
//! 1,000 行の上限に近いので兄弟のファイルに置く（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

use super::shell_target;
use super::test_support::{FakeReports, size_of};
use super::test_support::{FrameHarness, PerTargetSizes, pos_of, synth_assets, zero_clock};
use super::{BootAssets, RuntimeWiring};
use crate::move_cue::PositionCue;
use crate::talk_lifecycle::TalkLifecycleSignal;
use crate::zorder_cue::ZOrderDirective;
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_placement::chain_finalize::ChainFinalized;
use areka_placement::spawn::GhostWindows;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};

/// 位置の線の送り口と、それを受ける headless な結線。
pub fn wiring() -> (Sender<PositionCue>, RuntimeWiring) {
    let (tx, rx) = mpsc::channel::<PositionCue>();
    let mut wiring = wiring_with(
        synth_assets(&[(0, 0)]),
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        EmoPresenter::new(),
    );
    wiring.move_rx = rx;
    (tx, wiring)
}

/// 受け口を送り手の無い既定で埋め、時計を 0 で止めた結線。
pub fn wiring_with(
    assets: BootAssets,
    runtime: Rc<RefCell<TextLayerRuntime>>,
    presenter: EmoPresenter,
) -> RuntimeWiring {
    RuntimeWiring::new(
        presenter,
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<PositionCue>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        mpsc::channel::<ZOrderDirective>().1,
        runtime,
        zero_clock(),
        assets,
    )
}

/// 起動の確定が済んだ定常のハーネス（連鎖の確定は一度きりで、このフレームでは何もしない）。
pub fn steady_harness() -> FrameHarness {
    let mut harness = FrameHarness::new();
    harness.world.insert_resource(ChainFinalized);
    harness.advance_frame();
    harness
}

/// 変わる前の拡大率（実機の 200%）。
pub const HIGH_DPI: u16 = 192;

/// 変わった後の拡大率（実機の 100%＝幅がちょうど半分）。
pub const LOW_DPI: u16 = 96;

/// scope0 の実表示寸（200%・実機の幅 764）。
pub const HIGH_SIZE_0: (u32, u32) = (764, 1094);

/// scope0 の実表示寸（100%）。
pub const LOW_SIZE_0: (u32, u32) = (382, 547);

/// scope1 の実表示寸（200%・実機の幅 672）。
pub const HIGH_SIZE_1: (u32, u32) = (672, 596);

/// scope1 の実表示寸（100%）。
pub const LOW_SIZE_1: (u32, u32) = (336, 298);

/// 詰め直しを入れないときに 100% で残る隙間（実機 359px の決定論版）。
pub const GAP_WITHOUT_REALIGN: i32 = 359;

pub fn sizes(s0: (u32, u32), s1: (u32, u32)) -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(s0)), (1, Some(s1))])
}

pub fn low_sizes() -> PerTargetSizes {
    sizes(LOW_SIZE_0, LOW_SIZE_1)
}

/// `dpi` で起動して scope1 を scope0 の左に隣り合わせ、全スコープの既定の位置を外す
/// （記憶から戻した形）。
pub fn boot_remembered_at(
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
pub fn boot_remembered_at_high(harness: &mut FrameHarness, source: &mut FakeReports) {
    boot_remembered_at(harness, source, HIGH_DPI, HIGH_SIZE_0, HIGH_SIZE_1);
}

/// 拡大率の変化を 1 フレームで流し込む。
pub fn change_scale(
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
pub fn lower_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    change_scale(harness, source, LOW_DPI, LOW_SIZE_0, LOW_SIZE_1);
}

/// キャラ窓の今の左上。
pub fn char_pos(harness: &FrameHarness, scope: usize) -> (i32, i32) {
    let p = pos_of(&harness.world, harness.char_window(scope)).expect("position がある");
    (p.x, p.y)
}

/// キャラ窓の今の幅。
pub fn char_width(harness: &FrameHarness, scope: usize) -> i32 {
    size_of(&harness.world, harness.char_window(scope))
        .expect("size がある")
        .width
}

/// scope0 の左端 − scope1 の右端（scope1 が scope0 の左に隣り合う並び）。
pub fn gap(harness: &FrameHarness) -> i32 {
    char_pos(harness, 0).0 - (char_pos(harness, 1).0 + char_width(harness, 1))
}
