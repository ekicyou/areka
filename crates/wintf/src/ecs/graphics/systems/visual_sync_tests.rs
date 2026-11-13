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

/// 描画面の持ち主の欄へ直に書かれた NaN は 1.0 として写し、`warn!` を変わったときに 1 度残す（2.8・6.3）。
/// 持ち主は fill_world_visuals が訪れないので、ここで記録しないと黙って消える。
#[test]
fn owner_nan_opacity_syncs_as_one_and_warns_once_per_change() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    let compositor = wuc.compositor().expect("compositor");
    let wuc_visual: WucVisual = compositor.CreateSpriteVisual().unwrap().cast().unwrap();

    let mut world = World::new();
    let entity = world
        .spawn((
            Visual {
                opacity: f32::NAN,
                ..Default::default()
            },
            VisualGraphics::new(wuc_visual.clone()),
            Name::new("nan-owner"),
        ))
        .id();
    let mut schedule = Schedule::default();
    // 捕捉はスレッドローカルなので、係を同じスレッドで回す
    schedule.set_executor(bevy_ecs::schedule::SingleThreadedExecutor::new());
    schedule.add_systems(visual_property_sync_system);

    let log = crate::ecs::test_support::capture_under_filter("warn", || {
        schedule.run(&mut world);
        // 位置だけが動いても、NaN の記録は繰り返さない
        world.get_mut::<Arrangement>(entity).unwrap().offset = Offset { x: 1.0, y: 2.0 };
        schedule.run(&mut world);
    });
    assert_eq!(wuc_visual.Opacity().unwrap(), 1.0, "NaN は 1.0 として写す");
    assert_eq!(log.matches("不透明度が NaN").count(), 1, "{log}");
    assert!(log.contains("nan-owner"), "実体名が要る: {log}");
}
