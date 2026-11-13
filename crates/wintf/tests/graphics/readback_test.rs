//! 読み戻しの公開の口（要件 6.4）が crate の外から使える。装置なしで動く（GPU に触らない）。

use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use wintf::ecs::{PixelRead, SurfaceGraphics, SurfaceImage, SurfaceReadback};

#[test]
fn readback_is_usable_from_outside_the_crate() {
    let mut world = World::new();
    let owner = world.spawn(SurfaceGraphics::default()).id();
    let answers = world
        .run_system_once(move |readback: SurfaceReadback| {
            (readback.read_pixel(owner, 0, 0), readback.read_all(owner))
        })
        .unwrap();
    assert_eq!(answers, (PixelRead::Unreadable, SurfaceImage::Unreadable));
}
