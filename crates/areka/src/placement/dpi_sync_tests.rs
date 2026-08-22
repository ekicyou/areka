//! 整合ゲートの純判定と、待ち札の適用範囲の不変条件（設計 C5・Unit Tests 3）。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::{DPI, Point, SizeI, WindowHandle, WindowPos};

use super::*;
use crate::placement::diag::PlacementRoute;
use crate::placement::follow::{
    MonitorDpiEntry, MonitorDpiTable, move_window_to, move_window_with_route,
};
use crate::placement::resolver::RectPx;

/// 遷移前後の 2 水準（値そのものは主題でない・**異なる**ことだけが要る）。
const LOW: u32 = 96;
const HIGH: u32 = 192;

// ---------------------------------------------------------------------------
// 純判定（World も時刻も読まない）
// ---------------------------------------------------------------------------

#[test]
fn a_matching_table_proceeds() {
    assert_eq!(
        dpi_sync_decision(HIGH, Some(HIGH), None, 10),
        DpiSyncDecision::Proceed
    );
    // 既に待っていた窓も、一致した瞬間に進む（待ちフレーム数は結論を変えない）。
    assert_eq!(
        dpi_sync_decision(HIGH, Some(HIGH), Some(0), 10),
        DpiSyncDecision::Proceed
    );
}

#[test]
fn an_absent_table_entry_proceeds() {
    // 表そのものが無い／どのモニタにも属さない窓は待たせない（待たせると毎回上限まで待つ）。
    assert_eq!(
        dpi_sync_decision(HIGH, None, None, 10),
        DpiSyncDecision::Proceed
    );
    assert_eq!(
        dpi_sync_decision(HIGH, None, Some(0), 10),
        DpiSyncDecision::Proceed
    );
}

#[test]
fn a_mismatch_holds_until_the_bound() {
    // これから待ち始める（`held_since` なし）＝経過 0。
    assert_eq!(
        dpi_sync_decision(HIGH, Some(LOW), None, 10),
        DpiSyncDecision::Hold
    );
    // 上限の 1 手前まで待つ。
    assert_eq!(
        dpi_sync_decision(HIGH, Some(LOW), Some(10), 10 + DPI_SYNC_HOLD_MAX_FRAMES - 1),
        DpiSyncDecision::Hold
    );
}

#[test]
fn a_mismatch_gives_up_at_the_bound() {
    assert_eq!(
        dpi_sync_decision(HIGH, Some(LOW), Some(10), 10 + DPI_SYNC_HOLD_MAX_FRAMES),
        DpiSyncDecision::ProceedAfterTimeout
    );
    assert_eq!(
        dpi_sync_decision(HIGH, Some(LOW), Some(10), 10 + DPI_SYNC_HOLD_MAX_FRAMES + 1),
        DpiSyncDecision::ProceedAfterTimeout
    );
}

/// フレーム番号は `u32` で周回する。素の減算だと周回した瞬間に巨大な差になり、待ちが 1 度も
/// 効かないフレームが 1 周に 1 回生まれる。
#[test]
fn the_wait_is_measured_across_the_frame_counter_wrap() {
    let since = u32::MAX - 2;
    // 周回をまたいで 4 フレーム経過（MAX-2 → 1）: まだ上限には遠い。
    assert_eq!(
        dpi_sync_decision(HIGH, Some(LOW), Some(since), 1),
        DpiSyncDecision::Hold
    );
    // 同じ起点から上限ちょうど。
    let now = since.wrapping_add(DPI_SYNC_HOLD_MAX_FRAMES);
    assert_eq!(
        dpi_sync_decision(HIGH, Some(LOW), Some(since), now),
        DpiSyncDecision::ProceedAfterTimeout
    );
}

/// 判定語・観測点語はリテラルを持たず [`transition_diag`] の定数を引く（単一定義元）。
#[test]
fn the_decision_and_site_words_come_from_the_single_source() {
    assert_eq!(
        DpiSyncDecision::Proceed.as_str(),
        transition_diag::HOLD_DECISION_PROCEED
    );
    assert_eq!(
        DpiSyncDecision::Hold.as_str(),
        transition_diag::HOLD_DECISION_HOLD
    );
    assert_eq!(
        DpiSyncDecision::ProceedAfterTimeout.as_str(),
        transition_diag::HOLD_DECISION_PROCEED_AFTER_TIMEOUT
    );
    assert_eq!(HoldSite::Dpi.as_str(), transition_diag::HOLD_SITE_DPI);
    assert_eq!(
        HoldSite::Reconcile.as_str(),
        transition_diag::HOLD_SITE_RECONCILE
    );
    assert_eq!(HoldSite::Resnap.as_str(), transition_diag::HOLD_SITE_RESNAP);
    assert_eq!(
        HoldSite::WorkAreaResnap.as_str(),
        transition_diag::HOLD_SITE_WORK_AREA_RESNAP
    );
    // 2 つの「再スナップ」は**別語**である（同じ語だと、ログ上でどちらの点が見送ったのか
    // 判らない＝task 6.5 が解いた曖昧さがそのまま観測面へ戻る）。
    assert_ne!(HoldSite::Resnap.as_str(), HoldSite::WorkAreaResnap.as_str());
}

// ---------------------------------------------------------------------------
// World 越しの評価（帰属は表示基盤側の中心＋帰属規則を通る）
// ---------------------------------------------------------------------------

/// 2 台のモニタ（左＝[`LOW`]・右＝[`HIGH`]）の表。
fn two_monitor_table() -> MonitorDpiTable {
    MonitorDpiTable {
        entries: vec![
            MonitorDpiEntry {
                bounds: RectPx {
                    left: 0,
                    top: 0,
                    right: 1000,
                    bottom: 1000,
                },
                dpi: LOW,
            },
            MonitorDpiEntry {
                bounds: RectPx {
                    left: 1000,
                    top: 0,
                    right: 2000,
                    bottom: 1000,
                },
                dpi: HIGH,
            },
        ],
    }
}

/// 位置・寸・拡大率を持つ窓 1 枚と 2 台の表を持つ World。
fn world_with_window(position: Point, size: SizeI, dpi: u16) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(two_monitor_table());
    let window = world
        .spawn((
            WindowPos {
                position: Some(position),
                size: Some(size),
                ..Default::default()
            },
            DPI::from_dpi(dpi, dpi),
        ))
        .id();
    (world, window)
}

#[test]
fn the_gate_reads_the_table_entry_the_window_center_falls_into() {
    // 中心 (100, 100)＝左のモニタ（LOW）。窓も LOW ゆえ一致。
    let (world, window) = world_with_window(Point { x: 0, y: 0 }, SizeI::new(200, 200), LOW as u16);
    let outcome = evaluate(&world, window, 7);
    assert_eq!(outcome.table_dpi, Some(LOW));
    assert_eq!(outcome.decision, DpiSyncDecision::Proceed);

    // 中心 (1100, 100)＝右のモニタ（HIGH）。窓が LOW のままなら食い違い＝待つ。
    let (world, window) =
        world_with_window(Point { x: 1000, y: 0 }, SizeI::new(200, 200), LOW as u16);
    let outcome = evaluate(&world, window, 7);
    assert_eq!(outcome.table_dpi, Some(HIGH));
    assert_eq!(outcome.decision, DpiSyncDecision::Hold);
    assert_eq!(outcome.since_frame, 7, "これから待ち始めるので起点は今");
}

/// 窓生成前の `WindowPos`（`CW_USEDEFAULT`）は中心が求まらない＝表を引けない＝進む。
///
/// 中心の求め方を自前で書くと、この入力で整数桁溢れ（dev ビルドでは panic）を起こす。
#[test]
fn a_window_before_creation_proceeds_instead_of_overflowing() {
    let mut world = World::new();
    world.insert_resource(two_monitor_table());
    let window = world
        .spawn((
            WindowPos::default(),
            DPI::from_dpi(HIGH as u16, HIGH as u16),
        ))
        .id();
    let outcome = evaluate(&world, window, 3);
    assert_eq!(outcome.table_dpi, None);
    assert_eq!(outcome.decision, DpiSyncDecision::Proceed);
}

/// 表そのものが無い World（起動シームを通らない経路）も進む。
#[test]
fn a_world_without_the_table_proceeds() {
    let mut world = World::new();
    let window = world
        .spawn((
            WindowPos {
                position: Some(Point { x: 0, y: 0 }),
                size: Some(SizeI::new(200, 200)),
                ..Default::default()
            },
            DPI::from_dpi(HIGH as u16, HIGH as u16),
        ))
        .id();
    let outcome = evaluate(&world, window, 3);
    assert_eq!(outcome.table_dpi, None);
    assert_eq!(outcome.decision, DpiSyncDecision::Proceed);
}

/// 拡大率の相のゲートだけが札を付け外しする（判定 → 札 → 戻り値の対応）。
#[test]
fn only_the_scale_phase_gate_puts_and_removes_the_tag() {
    let (mut world, window) =
        world_with_window(Point { x: 1000, y: 0 }, SizeI::new(200, 200), LOW as u16);

    // 食い違い → 見送り（札が付く）。
    assert!(!apply_dpi_phase_gate(&mut world, window, 5));
    assert_eq!(
        world.get::<DpiSyncHold>(window).map(|h| h.since_frame),
        Some(5)
    );

    // 待ち続けても起点は据え置き（付け直して待ちを無限に延ばさない）。
    assert!(!apply_dpi_phase_gate(&mut world, window, 6));
    assert_eq!(
        world.get::<DpiSyncHold>(window).map(|h| h.since_frame),
        Some(5)
    );

    // 表と揃った瞬間に札が外れて進む。
    world.entity_mut(window).insert(DPI::from_dpi(192, 192));
    assert!(apply_dpi_phase_gate(&mut world, window, 7));
    assert!(world.get::<DpiSyncHold>(window).is_none());
}

/// 上限を超えたら札を外して進む（有界・要件 4.4）。
#[test]
fn the_gate_gives_up_and_removes_the_tag_at_the_bound() {
    let (mut world, window) =
        world_with_window(Point { x: 1000, y: 0 }, SizeI::new(200, 200), LOW as u16);
    assert!(!apply_dpi_phase_gate(&mut world, window, 0));
    assert!(!apply_dpi_phase_gate(
        &mut world,
        window,
        DPI_SYNC_HOLD_MAX_FRAMES - 1
    ));
    assert!(
        apply_dpi_phase_gate(&mut world, window, DPI_SYNC_HOLD_MAX_FRAMES),
        "上限に達しても待ち続けている（有界でない）"
    );
    assert!(world.get::<DpiSyncHold>(window).is_none());
}

/// ほかの窓書込点のゲートは**読むだけ**——札を外さない（外すと解除フレームに書込が 2 本出る）。
#[test]
fn the_other_sites_only_read_the_tag() {
    let (mut world, window) =
        world_with_window(Point { x: 1000, y: 0 }, SizeI::new(200, 200), LOW as u16);
    assert!(!defers_window_write(&world, window, HoldSite::Reconcile));

    world
        .entity_mut(window)
        .insert(DpiSyncHold { since_frame: 1 });
    assert!(defers_window_write(&world, window, HoldSite::Reconcile));
    assert!(defers_window_write(&world, window, HoldSite::Resnap));
    assert_eq!(
        world.get::<DpiSyncHold>(window).map(|h| h.since_frame),
        Some(1),
        "読むだけの点が札を外している"
    );
}

// ---------------------------------------------------------------------------
// 待ち札の適用範囲の不変条件（単一の窓書込口での監視・設計 C5）
// ---------------------------------------------------------------------------

/// 窓書込に必要な最小構成（偽ハンドル・位置・GA 境界）を持つ窓 1 枚の World。
fn writable_world(hold: bool) -> (World, Entity) {
    let mut world = World::new();
    let mut entity = world.spawn((
        WindowPos {
            position: Some(Point { x: 10, y: 20 }),
            size: Some(SizeI::new(100, 200)),
            ..Default::default()
        },
        WindowHandle {
            hwnd: HWND(0x200usize as *mut _),
            instance: HINSTANCE::default(),
        },
        Arrangement {
            offset: Offset { x: 10.0, y: 20.0 },
            ..Default::default()
        },
    ));
    if hold {
        entity.insert(DpiSyncHold { since_frame: 0 });
    }
    let window = entity.id();
    (world, window)
}

/// **見送りが覆うべき経路**の書込が待ち札のある窓へ到達したら、単一の窓書込口が
/// **その場で**鳴る。
///
/// `debug_assert!` ゆえテストビルドでは panic する——すり抜け経路が増えたときに、実機ログを
/// 待たずに檻が落ちる（実機では `warn!` として見える）。
///
/// 経路語が `ChainRealign`（システム由来・遷移後の連鎖の解き直し）なのは task 7.5 の是正に
/// 追随したものである。本テストは当初 `move_window_to`（＝`MoveCue`）で書いていたが、
/// スクリプトの明示操作は**見送らないことが正しい**と本番コードが裁定しており、鳴らすのは
/// 偽の警報だった。監視が生きていることを示す役目は経路語を差し替えて保つ——分類そのものは
/// `follow_window_move_hold_watch_tests.rs` が 12 語＋route 無しで固定する。
#[test]
#[should_panic(expected = "DpiSyncHold")]
fn a_write_reaching_a_waiting_window_trips_the_single_writer() {
    let (mut world, window) = writable_world(true);
    move_window_with_route(&mut world, window, 30, 40, PlacementRoute::ChainRealign);
}

/// 陽性の対——札が無ければ同じ書込は素通りする（上の panic が「常に落ちる」ではないこと）。
#[test]
fn the_same_write_passes_when_the_window_is_not_waiting() {
    let (mut world, window) = writable_world(false);
    assert!(
        move_window_with_route(&mut world, window, 30, 40, PlacementRoute::ChainRealign),
        "札の無い窓への書込が通らない（監視が無条件に塞いでいる）"
    );
    let _residue = wintf::ecs::window::drain_window_pos_commands();
}

/// 明示操作（`\![move]`＝`MoveCue`）は同じ札のある窓へ届いても鳴らない——上の 2 本と
/// **同じ土台**で、変えるのは経路語 1 点だけである（task 7.5）。
#[test]
fn the_explicit_move_is_not_watched_on_a_waiting_window() {
    let (mut world, window) = writable_world(true);
    assert!(
        move_window_to(&mut world, window, 30, 40),
        "明示操作の書込が通らない（見送られてしまっている）"
    );
    let _residue = wintf::ecs::window::drain_window_pos_commands();
}
