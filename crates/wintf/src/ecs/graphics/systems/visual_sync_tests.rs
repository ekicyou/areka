//! visual_property_sync_system: 見た目が作り直された部品にも位置と不透明度を写す

use super::*;
use crate::ecs::graphics::{GraphicsCore, WucGraphicsResource};
use crate::ecs::layout::Offset;
use windows::UI::Composition::Visual as WucVisual;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

#[test]
fn recreated_visual_gets_offset_and_opacity() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    let compositor = wuc.compositor().expect("compositor");
    let create = || -> WucVisual { compositor.CreateSpriteVisual().unwrap().cast().unwrap() };

    let mut world = World::new();
    let first = create();
    let entity = world
        .spawn((
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            VisualGraphics::new(first.clone()),
        ))
        .id();
    world.get_mut::<Arrangement>(entity).unwrap().offset = Offset { x: 10.0, y: 20.0 };

    let mut schedule = Schedule::default();
    schedule.add_systems(visual_property_sync_system);
    schedule.run(&mut world);
    assert_eq!(first.Opacity().unwrap(), 0.5);

    // 本番の作り直し（visual_manager）と同じく、部品の中身を新しい見た目に差し替える
    let second = create();
    *world.get_mut::<VisualGraphics>(entity).unwrap() = VisualGraphics::new(second.clone());
    schedule.run(&mut world);

    assert_eq!(
        second.Opacity().unwrap(),
        0.5,
        "作り直した見た目に不透明度が写っていない"
    );
    let offset = second.Offset().unwrap();
    assert_eq!(
        (offset.X, offset.Y),
        (10.0, 20.0),
        "作り直した見た目に位置が写っていない"
    );
}
