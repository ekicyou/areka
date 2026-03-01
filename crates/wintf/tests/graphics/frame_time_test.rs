//! FrameTime(f64) リソース設計検証
//!
//! 移行元: cue_dola_integration_test.rs (Section 4.10-3)

use bevy_ecs::prelude::*;
use wintf::ecs::FrameTime;

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
