//! 装置の失いからの立ち直りのテスト（audit-wintf-graphics-world 1・6 番）
//!
//! 作る係は本番では装置が使える回だけ回すが、使う人が係を直に登録したときの守りとして、
//! 装置（`GraphicsCore`）の無い World で回しても落ちないことを固定する。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use wintf::ecs::world::FrameCount;
use wintf::ecs::{Visual, VisualGraphics, visual_resource_management_system};

/// 装置を入れない World で見た目の資源の係を直に回しても落ちず、見た目は作られない
#[test]
fn visual_resource_system_without_graphics_core_does_not_panic() {
    let mut world = World::new();
    world.insert_resource(FrameCount(0));
    let entity = world
        .spawn((Visual::default(), VisualGraphics::default()))
        .id();

    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(visual_resource_management_system);
    schedule.run(&mut world);

    assert!(
        !world.get::<VisualGraphics>(entity).unwrap().is_valid(),
        "without GraphicsCore no visual must be created"
    );
}
