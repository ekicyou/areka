//! visual_resource_management_system の早期リターン経路ギャップテスト (W3b-T)
//!
//! 既存テスト（graphics_auto_creation_test）は正常系と GraphicsCore 無効を
//! 検証済みだが、WucGraphicsResource 不在・空の早期リターンは未固定だったため追加する。
//! 対象: crates/wintf/src/ecs/graphics/visual_manager.rs

use bevy_ecs::prelude::*;
use windows::core::Result;
use wintf::ecs::world::FrameCount;
use wintf::ecs::{Visual, VisualGraphics, WucGraphicsResource, visual_resource_management_system};

use super::common::setup_graphics;

fn run_system(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_resource_management_system);
    schedule.run(world);
}

/// WucGraphicsResource リソース不在 → 早期リターンし VisualGraphics は無効のまま
#[test]
fn early_return_when_wuc_resource_missing() -> Result<()> {
    let graphics = setup_graphics()?;
    let mut world = World::new();
    world.insert_resource(graphics);
    world.insert_resource(FrameCount(0));
    // WucGraphicsResource は挿入しない

    let entity = world
        .spawn((Visual::default(), VisualGraphics::default()))
        .id();
    world.flush();

    run_system(&mut world);

    let vg = world.get::<VisualGraphics>(entity).unwrap();
    assert!(
        !vg.is_valid(),
        "without WucGraphicsResource no visual must be created"
    );
    Ok(())
}

/// WucGraphicsResource が空（new_empty）→ compositor() None で早期リターン
#[test]
fn early_return_when_wuc_resource_empty() -> Result<()> {
    let graphics = setup_graphics()?;
    let mut world = World::new();
    world.insert_resource(graphics);
    world.insert_resource(FrameCount(0));
    world.insert_resource(WucGraphicsResource::new_empty());

    let entity = world
        .spawn((Visual::default(), VisualGraphics::default()))
        .id();
    world.flush();

    run_system(&mut world);

    let vg = world.get::<VisualGraphics>(entity).unwrap();
    assert!(
        !vg.is_valid(),
        "with empty WucGraphicsResource no visual must be created"
    );
    Ok(())
}
