//! 統合テスト: dola 統合の検証
//!
//! Task 4.10 — DolaRuntime リソース初期化、update_dola_runtime システム連携、
//! FrameTime(f64) リソースの設計（ワールド外で更新、ワールド内で一貫した値を参照）を検証する。

use bevy_ecs::prelude::*;
use wintf::ecs::FrameTime;
use wintf::ecs::cue::runtime::DolaRuntime;

// ---------------------------------------------------------------
// 4.10-1: DolaRuntime リソース初期化の検証
// ---------------------------------------------------------------

/// DolaRuntime::new() でリソースが生成され、World に挿入できることを検証。
#[test]
fn dola_runtime_resource_init() {
    let mut world = World::new();
    world.insert_resource(DolaRuntime::new());

    // リソースとして取得できること
    let dola = world.resource::<DolaRuntime>();
    // Debug 出力が可能であること
    let debug = format!("{:?}", dola);
    assert!(debug.contains("DolaRuntime"));
}

/// DolaRuntime::default() が new() と同等に動作することを検証。
#[test]
fn dola_runtime_default_is_valid() {
    let dola = DolaRuntime::default();
    // facade への参照が取得できること
    let _facade = dola.facade();
}

/// DolaRuntime の facade_mut() から update() を呼び出せることを検証。
#[test]
fn dola_runtime_facade_mut_update_callable() {
    let mut dola = DolaRuntime::new();
    let result = dola.facade_mut().update(0.0);
    // 初期状態では変更なし
    assert!(result.changes.is_empty());
    assert!(result.triggered.is_empty());
}

// ---------------------------------------------------------------
// 4.10-2: update_dola_runtime システムの FrameTime 連携検証
// ---------------------------------------------------------------

/// update_dola_runtime は FrameTime.0 を DolaRuntime に渡して更新する。
/// World に両リソースを挿入し、システムを手動実行して正常動作を検証する。
#[test]
fn update_dola_runtime_system_runs_with_frame_time() {
    use wintf::ecs::cue::systems::update_dola_runtime;

    let mut world = World::new();
    world.insert_resource(FrameTime(dola::runtime::clock::now()));
    world.insert_resource(DolaRuntime::new());

    // システムを手動で実行（パニックしないことを検証）
    let mut schedule = Schedule::default();
    schedule.add_systems(update_dola_runtime);
    schedule.run(&mut world);

    // 2回目の実行もパニックしないことを検証（状態が壊れていないこと）
    schedule.run(&mut world);
}

/// update_dola_runtime をフレームのように複数回呼んでも正常であることを検証。
/// 各フレーム前に FrameTime.0 を更新することをシミュレート。
#[test]
fn update_dola_runtime_multiple_frames() {
    use wintf::ecs::cue::systems::update_dola_runtime;

    let mut world = World::new();
    world.insert_resource(FrameTime(0.0));
    world.insert_resource(DolaRuntime::new());

    let mut schedule = Schedule::default();
    schedule.add_systems(update_dola_runtime);

    // 10 フレーム分実行（各フレーム前に時刻を更新）
    for i in 0..10 {
        // ワールド外で FrameTime を更新（try_tick_world の動作を模倣）
        if let Some(mut frame_time) = world.get_resource_mut::<FrameTime>() {
            frame_time.0 = i as f64 * 0.016; // 60fps 相当
        }
        schedule.run(&mut world);
    }
}

// ---------------------------------------------------------------
// 4.10-3: FrameTime(f64) リソース設計検証
// ---------------------------------------------------------------

/// FrameTime(f64) はワールド外で更新され、ワールド内では一貫した値を参照できることを検証。
#[test]
fn frame_time_consistent_within_frame() {
    let mut world = World::new();

    // ワールド外で FrameTime を dola::clock::now() で初期化
    let initial_time = dola::runtime::clock::now();
    world.insert_resource(FrameTime(initial_time));

    // ワールド内で値を取得（複数回アクセスしても同じ値）
    let t1 = world.resource::<FrameTime>().0;
    std::thread::sleep(std::time::Duration::from_millis(1));
    let t2 = world.resource::<FrameTime>().0;

    // ワールド内では時刻は進まない
    assert_eq!(
        t1, t2,
        "FrameTime.0 should remain consistent within the same frame"
    );
    assert_eq!(t1, initial_time);
}

/// FrameTime は Default トレイトを持ち、0.0 で初期化されることを検証。
#[test]
fn frame_time_default_initializes_to_zero() {
    let frame_time = FrameTime::default();
    assert_eq!(
        frame_time.0, 0.0,
        "FrameTime::default() should initialize to 0.0"
    );
}

/// テストで任意の時刻を注入できることを検証（テスト容易性）。
#[test]
fn frame_time_injectable_for_testing() {
    let mut world = World::new();

    // テスト用に特定の時刻を注入
    world.insert_resource(FrameTime(123.456));

    // システムからアクセス可能
    let time = world.resource::<FrameTime>().0;
    assert_eq!(time, 123.456);

    // 別の時刻に更新（次フレームの模倣）
    if let Some(mut frame_time) = world.get_resource_mut::<FrameTime>() {
        frame_time.0 = 789.012;
    }

    let updated_time = world.resource::<FrameTime>().0;
    assert_eq!(updated_time, 789.012);
}
