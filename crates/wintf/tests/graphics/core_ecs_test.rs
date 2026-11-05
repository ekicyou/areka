//! GraphicsCoreのECS統合テスト
//!
//! このテストは以下を検証します:
//! - init_graphics_coreシステムがリソースを作ること
//! - 有効なGraphicsCoreが既に在れば作り直さないこと（冪等）

use bevy_ecs::prelude::*;
use wintf::ecs::{FrameCount, GraphicsCore, init_graphics_core};

#[test]
fn test_init_graphics_core_creates_resource() {
    // init_graphics_coreシステムがリソースを作成することを確認

    let mut world = World::new();

    // FrameCountリソースを追加
    world.insert_resource(FrameCount::default());

    // 初期状態ではGraphicsCoreリソースが存在しないことを確認
    assert!(
        world.get_resource::<GraphicsCore>().is_none(),
        "初期状態ではGraphicsCoreリソースは存在しない"
    );

    // init_graphics_coreシステムを作成して実行
    let mut system = IntoSystem::into_system(init_graphics_core);
    system.initialize(&mut world);
    system
        .run((), &mut world)
        .expect("init_graphics_core の実行");

    // コマンドを適用
    system.apply_deferred(&mut world);

    // GraphicsCoreリソースが作成されたことを確認
    let graphics_res = world.get_resource::<GraphicsCore>();
    assert!(
        graphics_res.is_some(),
        "init_graphics_core実行後、GraphicsCoreリソースが存在すること"
    );
}

#[test]
fn test_init_graphics_core_idempotent() {
    // init_graphics_coreが冪等であることを確認
    // （有効なリソースが既に存在する場合は作り直さない）

    let mut world = World::new();
    world.insert_resource(FrameCount::default());

    // 最初のGraphicsCoreを作成して登録
    let first_graphics = GraphicsCore::new().expect("GraphicsCore作成成功");
    world.insert_resource(first_graphics);
    let before = world
        .resource::<GraphicsCore>()
        .d2d_device()
        .cloned()
        .expect("事前のD2D装置");

    // init_graphics_coreシステムを実行
    let mut system = IntoSystem::into_system(init_graphics_core);
    system.initialize(&mut world);
    system
        .run((), &mut world)
        .expect("init_graphics_core の実行");
    system.apply_deferred(&mut world);

    // 同じ装置のまま（作り直されていない）ことを確認
    let after = world
        .resource::<GraphicsCore>()
        .d2d_device()
        .cloned()
        .expect("実行後のD2D装置");
    assert!(
        before == after,
        "有効なGraphicsCoreが在るとき、init_graphics_coreは装置を作り直さない"
    );
}

#[test]
fn test_init_graphics_core_in_schedule() {
    // スケジュール内でinit_graphics_coreが正しく動作することを確認

    let mut world = World::new();

    // FrameCountリソースを追加
    world.insert_resource(FrameCount::default());

    let mut schedule = Schedule::default();

    // init_graphics_coreをスケジュールに追加
    schedule.add_systems(init_graphics_core);

    // 初期状態ではリソースが存在しない
    assert!(
        world.get_resource::<GraphicsCore>().is_none(),
        "スケジュール実行前はリソースが存在しない"
    );

    // スケジュールを実行
    schedule.run(&mut world);

    // リソースが作成されたことを確認
    assert!(
        world.get_resource::<GraphicsCore>().is_some(),
        "スケジュール実行後、リソースが存在する"
    );
}
