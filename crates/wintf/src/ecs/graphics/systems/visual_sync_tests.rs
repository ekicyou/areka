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

/// 描画面の持ち主の欄へ直に書かれた NaN は 1.0 として写し、`warn!` を変わったときに 1 度残す。
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

// --- WUC の木の同期: 祖先の持ち主・節の通り抜け・並べ直し ---

/// 親の WUC の子の並び（下→上）。
fn wuc_children(parent: &WucVisual) -> Vec<*mut core::ffi::c_void> {
    use windows::UI::Composition::ContainerVisual;
    let it = parent
        .cast::<ContainerVisual>()
        .unwrap()
        .Children()
        .unwrap()
        .First()
        .unwrap();
    let mut out = Vec::new();
    while it.HasCurrent().unwrap() {
        out.push(it.Current().unwrap().as_raw());
        it.MoveNext().unwrap();
    }
    out
}

fn hierarchy_world() -> (World, impl Fn() -> WucVisual) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    let compositor = wuc.compositor().expect("compositor").clone();
    let create = move || -> WucVisual { compositor.CreateSpriteVisual().unwrap().cast().unwrap() };
    (World::new(), create)
}

fn run_hierarchy(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_hierarchy_sync_system);
    schedule.run(world);
}

/// 節の下の持ち主は節を通り抜けて祖先の持ち主の WUC の子に入り、並びは節を通り抜けた前順
/// （最初が最上）。節の `Children` の並びだけを変えた回にも並べ直す。
#[test]
fn owner_under_a_node_joins_the_nearest_owner_and_follows_node_child_order() {
    use crate::ecs::graphics::WorldVisual;
    let (mut world, create) = hierarchy_world();
    let (vr, vh, va, vb, vc) = (create(), create(), create(), create(), create());
    let root = world.spawn(VisualGraphics::new(vr)).id();
    let host = world
        .spawn((VisualGraphics::new(vh.clone()), ChildOf(root)))
        .id();
    let node = world.spawn((WorldVisual::PENDING, ChildOf(host))).id();
    world.spawn((VisualGraphics::new(va.clone()), ChildOf(node)));
    world.spawn((VisualGraphics::new(vb.clone()), ChildOf(node)));
    world.spawn((VisualGraphics::new(vc.clone()), ChildOf(host)));

    run_hierarchy(&mut world);
    // 前順 [A, B, C] を InsertAtBottom → 下から C, B, A
    assert_eq!(
        wuc_children(&vh),
        vec![vc.as_raw(), vb.as_raw(), va.as_raw()]
    );

    // 節の子の並びだけを入れ替える（親は変わらない）
    world.get_mut::<Children>(node).unwrap().swap(0, 1);
    run_hierarchy(&mut world);
    assert_eq!(
        wuc_children(&vh),
        vec![vc.as_raw(), va.as_raw(), vb.as_raw()],
        "並びだけの変化を同じ回に写す"
    );
}

/// 用意中・戻した直後の自動の持ち主（3 部品を持つ節で目印が無い）は親に繋がず古い親から外し、
/// その下の持ち主は通り抜けて祖先の持ち主に入る。
#[test]
fn detached_auto_owner_leaves_its_old_parent_and_is_passed_through() {
    use crate::ecs::graphics::{OwnSurface, WorldVisual};
    let (mut world, create) = hierarchy_world();
    let (vh, vn, vd) = (create(), create(), create());
    let window = world.spawn(VisualGraphics::new(create())).id();
    let host = world
        .spawn((VisualGraphics::new(vh.clone()), ChildOf(window)))
        .id();
    let node = world
        .spawn((
            WorldVisual::PENDING,
            OwnSurface,
            VisualGraphics::new(vn.clone()),
            ChildOf(host),
        ))
        .id();
    world.spawn((VisualGraphics::new(vd.clone()), ChildOf(node)));

    run_hierarchy(&mut world);
    assert_eq!(wuc_children(&vh), vec![vn.as_raw()], "目印の在る節は持ち主");
    assert_eq!(wuc_children(&vn), vec![vd.as_raw()]);

    world.entity_mut(node).remove::<OwnSurface>();
    run_hierarchy(&mut world);
    assert_eq!(
        wuc_children(&vh),
        vec![vd.as_raw()],
        "目印の外れた節は外し、その下の持ち主は祖先の持ち主へ"
    );
    let vg = world.get::<VisualGraphics>(node).unwrap();
    assert_eq!(
        vg.parent_visual().map(|v| v.as_raw()),
        Some(vn.as_raw()),
        "根の印"
    );
}

/// 上げた（Active の）自動の持ち主は、用意中の間は親に繋がず、上げた回に親の持ち主へ繋ぎ、
/// 並びは手動の持ち主より下（WUC の子の一番下）。
#[test]
fn active_auto_owner_attaches_to_its_parent_owner_below_manual_owners() {
    use crate::ecs::graphics::{SurfaceReadiness, WorldVisual};
    let (mut world, create) = hierarchy_world();
    let (vh, vx, vm) = (create(), create(), create());
    let window = world.spawn(VisualGraphics::new(create())).id();
    let host = world
        .spawn((VisualGraphics::new(vh.clone()), ChildOf(window)))
        .id();
    // X を先に生む: 前順は [X, M] なので、そのまま下へ積むと下から [M, X] になる
    let x = world
        .spawn((
            WorldVisual::PENDING,
            VisualGraphics::new(vx.clone()),
            ChildOf(host),
        ))
        .id();
    world.spawn((VisualGraphics::new(vm.clone()), ChildOf(host)));
    {
        let mut a = world.get_mut::<AutoPromotion>(x).unwrap();
        for i in 0..8 {
            a.rule
                .step(i as f64 * 0.1, true, || Ok(()), SurfaceReadiness::Pending);
        }
        assert_eq!(a.rule.phase(), Phase::Preparing);
    }

    run_hierarchy(&mut world);
    assert_eq!(wuc_children(&vh), vec![vm.as_raw()], "用意中は親に繋がない");

    {
        let mut a = world.get_mut::<AutoPromotion>(x).unwrap();
        a.rule.step(0.8, false, || Ok(()), SurfaceReadiness::Ready);
        assert_eq!(a.rule.phase(), Phase::Active);
    }
    run_hierarchy(&mut world);
    assert_eq!(
        wuc_children(&vh),
        vec![vx.as_raw(), vm.as_raw()],
        "上げた回に親の持ち主へ繋ぎ、手動の持ち主より下"
    );
}

/// 持ち主の変形・節の下の持ち主・自動の持ち主の写しと、置き場・変形の外れを同じ回に写す。
#[test]
fn owner_transform_and_placement_map_and_revert_on_removal() {
    use crate::ecs::graphics::{PlacementClip, PlacementKind, SurfacePlacement, VisualTransform};
    use crate::numerics::Transform2D;
    use windows_numerics::{Matrix3x2, Vector2};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    let compositor = wuc.compositor().expect("compositor");
    let visual: WucVisual = compositor.CreateSpriteVisual().unwrap().cast().unwrap();

    let mut world = World::new();
    let e = world
        .spawn((
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            VisualGraphics::new(visual.clone()),
        ))
        .id();
    world.get_mut::<Arrangement>(e).unwrap().offset = Offset { x: 10.0, y: 20.0 };
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_property_sync_system);
    schedule.run(&mut world);
    let offset = visual.Offset().unwrap();
    assert_eq!((offset.X, offset.Y), (10.0, 20.0));

    // 置き場が無い持ち主の VisualTransform（拡大）は行列で写り、位置は 0
    let mut t = Transform2D::identity();
    t.scale = Vector2 { X: 2.0, Y: 2.0 };
    world.entity_mut(e).insert(VisualTransform(t));
    schedule.run(&mut world);
    let m = visual.TransformMatrix().unwrap();
    assert_eq!((m.M11, m.M22, m.M41, m.M42), (2.0, 2.0, 10.0, 20.0));
    assert_eq!(visual.Offset().unwrap().X, 0.0);

    // 変形の外れは同じ回に位置だけの写しへ戻る
    world.entity_mut(e).remove::<VisualTransform>();
    schedule.run(&mut world);
    let m = visual.TransformMatrix().unwrap();
    assert_eq!((m.M11, m.M22, m.M41, m.M42), (1.0, 1.0, 0.0, 0.0));
    assert_eq!(visual.Offset().unwrap().X, 10.0);

    // 節の下の手動の持ち主: 親の節の世界の行列を掛け、不透明度は祖先の積
    world
        .entity_mut(e)
        .insert(SurfacePlacement(PlacementKind::Embedded {
            parent_world: Matrix3x2::translation(100.0, 0.0),
            opacity: 0.5,
            visible: true,
            clip: PlacementClip::None,
        }));
    schedule.run(&mut world);
    let offset = visual.Offset().unwrap();
    assert_eq!((offset.X, offset.Y), (110.0, 20.0));
    assert_eq!(visual.Opacity().unwrap(), 0.25);

    // 自動で上げた持ち主: 整数の原点と大きさだけ・不透明度 1
    *world.get_mut::<SurfacePlacement>(e).unwrap() = SurfacePlacement(PlacementKind::Auto {
        origin_px: (7, 9),
        size_px: (40, 30),
    });
    schedule.run(&mut world);
    let offset = visual.Offset().unwrap();
    assert_eq!((offset.X, offset.Y), (7.0, 9.0));
    let size = visual.Size().unwrap();
    assert_eq!((size.X, size.Y), (40.0, 30.0));
    assert_eq!(visual.Opacity().unwrap(), 1.0);

    // 置き場の外れも同じ回に今の写しへ戻る
    world.entity_mut(e).remove::<SurfacePlacement>();
    schedule.run(&mut world);
    let offset = visual.Offset().unwrap();
    assert_eq!((offset.X, offset.Y), (10.0, 20.0));
    assert_eq!(visual.Opacity().unwrap(), 0.5);
}
