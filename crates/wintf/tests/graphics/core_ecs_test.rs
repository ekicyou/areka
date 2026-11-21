//! GraphicsCoreのECS統合テスト
//!
//! このテストは以下を検証します:
//! - init_graphics_coreシステムがリソースを作ること
//! - 有効なGraphicsCoreが既に在れば作り直さないこと（冪等）

use bevy_ecs::prelude::*;
use wintf::ecs::{FrameCount, GraphicsCore, init_graphics_core};

#[test]
fn test_init_graphics_core_creates_resource() {
    let mut world = World::new();

    world.insert_resource(FrameCount::default());

    assert!(
        world.get_resource::<GraphicsCore>().is_none(),
        "初期状態ではGraphicsCoreリソースは存在しない"
    );

    let mut system = IntoSystem::into_system(init_graphics_core);
    system.initialize(&mut world);
    system
        .run((), &mut world)
        .expect("init_graphics_core の実行");

    system.apply_deferred(&mut world);

    let graphics_res = world.get_resource::<GraphicsCore>();
    assert!(
        graphics_res.is_some(),
        "init_graphics_core実行後、GraphicsCoreリソースが存在すること"
    );
}

#[test]
fn test_init_graphics_core_idempotent() {
    let mut world = World::new();
    world.insert_resource(FrameCount::default());

    let first_graphics = GraphicsCore::new().expect("GraphicsCore作成成功");
    world.insert_resource(first_graphics);
    let before = world
        .resource::<GraphicsCore>()
        .d2d_device()
        .cloned()
        .expect("事前のD2D装置");

    let mut system = IntoSystem::into_system(init_graphics_core);
    system.initialize(&mut world);
    system
        .run((), &mut world)
        .expect("init_graphics_core の実行");
    system.apply_deferred(&mut world);

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
