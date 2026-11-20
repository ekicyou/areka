//! frame のテストが `areka-runtime` と本体の両方で使う補助（結線・定常のハーネス・拡大率の変化と
//! 鎖の詰め直しの駆動・3 体の鎖の窓の一式）。`frame_test_support.rs` は 1,000 行の上限に近いので
//! 兄弟のファイルに置く。

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use wintf::ecs::{Point, SizeI, WindowPos};

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
use areka_placement::chain_finalize::{ChainFinalized, ScopeChainState};
use areka_placement::follow::OffsetBase;
use areka_placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::{GhostWindows, spawn_ghost_windows};
use areka_placement::test_support::give_fake_handles;
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

pub fn high_sizes() -> PerTargetSizes {
    sizes(HIGH_SIZE_0, HIGH_SIZE_1)
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

/// 100% → 200% を 1 フレームで流し込む。
pub fn raise_the_scale(harness: &mut FrameHarness, source: &mut FakeReports) {
    change_scale(harness, source, HIGH_DPI, HIGH_SIZE_0, HIGH_SIZE_1);
}

/// キャラ窓の今の左上。
pub fn char_pos(harness: &FrameHarness, scope: usize) -> (i32, i32) {
    let p = pos_of(&harness.world, harness.char_window(scope)).expect("position がある");
    (p.x, p.y)
}

pub fn char_x(harness: &FrameHarness, scope: usize) -> i32 {
    char_pos(harness, scope).0
}

/// 当該スコープの既定の位置（台帳）の x。
pub fn default_x(harness: &FrameHarness, scope: usize) -> Option<i32> {
    harness
        .world
        .resource::<GhostWindows>()
        .default_char_pos(scope)
        .map(|p| p.x)
}

/// キャラ窓を横へ `dx` だけずらす（利用者のドラッグが着いた後の位置。ドラッグは route を持たない
/// 書き込みなので、既定の位置は動かさない）。
pub fn drag_char_window_by(harness: &mut FrameHarness, scope: usize, dx: i32) {
    let x = char_x(harness, scope) + dx;
    place_char_window_x(harness, scope, x);
}

/// キャラ窓の左上の x だけをその値に置く（ドラッグや台本の結果の位置。既定の位置も印も触らない）。
pub fn place_char_window_x(harness: &mut FrameHarness, scope: usize, x: i32) {
    let window = harness.char_window(scope);
    let mut wp = harness
        .world
        .get_mut::<WindowPos>(window)
        .expect("WindowPos がある");
    let wp = wp.bypass_change_detection();
    let y = wp.position.expect("position がある").y;
    wp.position = Some(Point { x, y });
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

/// 3 体の鎖の作業領域の下端（3 体とも下端に接地する）。
pub const CHAIN3_BOTTOM: i32 = 1444;

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

/// 3 スコープの窓の一式（偽 WindowHandle 付き・起動の並べ直し済み）。ハーネスは 2 スコープ固定なので
/// 3 体の鎖はこちらで組む。
pub fn chain3_world() -> (World, GhostWindows) {
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
    give_fake_handles(&mut world, &gw);
    world.insert_resource(ChainFinalized);
    (world, gw)
}

/// 幅と高さを半分にし、下端中央を保って置き直す（拡大率の相の再射影の結果の形）。
pub fn halve_keeping_bottom_center(world: &mut World, gw: &GhostWindows) {
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
        wp.size = Some(SizeI::new(w, h));
    }
}

/// 今の窓の位置と幅から解きの入力を作る（本番の走査と同じ形・既定の位置は無い）。
pub fn chain3_scan(world: &World) -> (Vec<ScopeChainState>, Vec<(usize, Entity, PointPx)>) {
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

pub fn chain3_pos(world: &World, gw: &GhostWindows, scope: usize) -> (i32, i32) {
    let p = pos_of(world, gw.char_window(scope).expect("キャラ窓")).expect("position がある");
    (p.x, p.y)
}
