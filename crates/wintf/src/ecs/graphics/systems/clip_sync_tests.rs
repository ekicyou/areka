//! `clip_key_for`・`placed_key`・`placed_clip_px` の純粋なテスト（装置なし・工場だけ）と、
//! WARP の上の WUC の見た目へ置き場の切り抜きを当てる係のテスト。

use super::*;
use crate::ecs::test_support::capture_under_filter;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::*;

const WARN_TEXT: &str = "WUC では範囲外なので切り抜かない";

fn geometry() -> ClipShape {
    let factory: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }.unwrap();
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 10.0,
        bottom: 10.0,
    };
    let g = unsafe { factory.CreateRectangleGeometry(&rect) }.unwrap();
    ClipShape::Geometry(g.into())
}

fn key(clip: Option<&ClipShape>, prev: Option<&ClipKey>) -> ClipKey {
    clip_key_for(
        Entity::from_raw_u32(7).unwrap(),
        Some(&Name::new("clip-owner")),
        clip,
        100.0,
        50.0,
        2.0,
        2.0,
        prev,
    )
}

#[test]
fn geometry_becomes_no_clip_key_and_warns_once() {
    let shape = geometry();
    let mut keys = Vec::new();
    let log = capture_under_filter("warn", || {
        let first = key(Some(&shape), None);
        // 同じ図形のまま配置だけ変わった回: 鍵は同じで warn は出ない。
        let second = key(Some(&shape), Some(&first));
        keys.push(first);
        keys.push(second);
    });
    for k in &keys {
        assert_eq!(k.3, None, "図形は控えに持たない（切り抜きなし）");
    }
    assert_eq!(keys[0], keys[1]);
    assert_eq!(log.matches(WARN_TEXT).count(), 1, "{log}");
    assert!(log.contains("clip-owner"), "実体名が要る: {log}");
}

#[test]
fn geometry_after_no_clip_changes_key_and_warns() {
    let shape = geometry();
    let none = key(None, None);
    let mut got = None;
    let log = capture_under_filter("warn", || got = Some(key(Some(&shape), Some(&none))));
    assert_ne!(got.unwrap(), none, "図形へ変わった回は鍵が変わる");
    assert_eq!(log.matches(WARN_TEXT).count(), 1, "{log}");
}

#[test]
fn rect_key_keeps_shape_in_physical_pixels() {
    let k = key(Some(&ClipShape::Rectangle), None);
    assert_eq!(
        k,
        (200.0, 100.0, 2.0, Some(ClipShape::Rectangle), None, false)
    );
}

#[test]
fn clip_shape_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ClipShape>();
    assert_send_sync::<crate::ecs::graphics::Visual>();
    assert_send_sync::<VisualGraphics>();
}

// --- 置き場の切り抜き（placed_key・placed_clip_px） ---

use crate::ecs::graphics::{PlacementClip, SurfacePlacement};
use crate::numerics::Aabb;

const SIZE: (f32, f32) = (100.0, 50.0);

fn embedded(parent_world: Matrix3x2, clip: PlacementClip) -> PlacementKind {
    PlacementKind::Embedded {
        parent_world,
        opacity: 1.0,
        visible: true,
        clip,
    }
}

fn placed(own: Option<&ClipShape>, placement: &PlacementKind, local: Matrix3x2) -> ClipKey {
    placed_key(key(own, None), own, Some(placement), local, SIZE)
}

#[test]
fn no_placement_clip_keeps_the_own_key() {
    let own = ClipShape::Rectangle;
    let k = key(Some(&own), None);
    assert_eq!(
        placed_key(k.clone(), Some(&own), None, Matrix3x2::identity(), SIZE),
        k
    );
    let p = embedded(Matrix3x2::translation(5.0, 5.0), PlacementClip::None);
    assert_eq!(placed(Some(&own), &p, Matrix3x2::identity()), k);
}

#[test]
fn axis_aligned_placement_clip_becomes_a_local_rect() {
    let p = embedded(
        Matrix3x2::translation(5.0, 5.0),
        PlacementClip::Rect(Aabb::new(0.0, 0.0, 50.0, 40.0)),
    );
    let k = placed(None, &p, Matrix3x2::translation(10.0, 10.0));
    assert_eq!(k.4, Some((Aabb::new(-15.0, -15.0, 35.0, 25.0), None)));
    assert!(!k.5);
    // 同じ置き場・同じ位置なら鍵は同じ（当て直さない）、動けば変わる（当て直す）
    assert_eq!(placed(None, &p, Matrix3x2::translation(10.0, 10.0)), k);
    assert_ne!(placed(None, &p, Matrix3x2::translation(11.0, 10.0)), k);
}

#[test]
fn own_rect_is_intersected_and_an_empty_intersection_shows_nothing() {
    let own = ClipShape::Rectangle;
    let p = embedded(
        Matrix3x2::identity(),
        PlacementClip::Rect(Aabb::new(50.0, -10.0, 200.0, 20.0)),
    );
    let k = placed(Some(&own), &p, Matrix3x2::identity());
    assert_eq!(k.4, Some((Aabb::new(50.0, 0.0, 100.0, 20.0), None)));

    let far = embedded(
        Matrix3x2::identity(),
        PlacementClip::Rect(Aabb::new(300.0, 300.0, 400.0, 400.0)),
    );
    let k = placed(Some(&own), &far, Matrix3x2::identity());
    let placed_clip = k.4.expect("空でも置き場の切り抜きを当てる");
    let (_, size, _) = placed_clip_px(&placed_clip, 2.0);
    assert_eq!((size.X, size.Y), (0.0, 0.0), "空の交わりは何も出さない");
}

#[test]
fn rotated_placement_clip_keeps_the_host_rect_and_the_inverse() {
    let host = Aabb::new(0.0, 0.0, 50.0, 40.0);
    let p = embedded(Matrix3x2::identity(), PlacementClip::Rect(host));
    let local = Matrix3x2::rotation(30.0) * Matrix3x2::translation(10.0, 20.0);
    let k = placed(None, &p, local);
    let Some((rect, Some(inv))) = k.4 else {
        panic!("逆行列つきの矩形になる: {:?}", k.4);
    };
    assert_eq!(rect, host);
    let back = inv * local;
    for (got, want) in [
        (back.M11, 1.0),
        (back.M12, 0.0),
        (back.M21, 0.0),
        (back.M22, 1.0),
        (back.M31, 0.0),
        (back.M32, 0.0),
    ] {
        assert!((got - want).abs() < 1e-4, "{back:?}");
    }
}

#[test]
fn unsupported_combination_keeps_only_the_own_clip() {
    let own = ClipShape::rounded_rectangle(4.0);
    let p = embedded(
        Matrix3x2::identity(),
        PlacementClip::Rect(Aabb::new(0.0, 0.0, 50.0, 40.0)),
    );
    let k = key(Some(&own), None);
    assert_eq!(placed(Some(&own), &p, Matrix3x2::identity()), k);
    let p = embedded(Matrix3x2::identity(), PlacementClip::Unsupported);
    assert_eq!(placed(Some(&own), &p, Matrix3x2::identity()), k);
}

#[test]
fn placed_clip_converts_dip_to_physical_pixels() {
    let rect = Aabb::new(1.0, 2.0, 11.0, 22.0);
    let (offset, size, m) = placed_clip_px(&(rect, None), 2.0);
    assert_eq!((offset.X, offset.Y, size.X, size.Y), (2.0, 4.0, 20.0, 40.0));
    assert_eq!(m, None);

    let inv = Matrix3x2 {
        M11: 0.0,
        M12: 1.0,
        M21: -1.0,
        M22: 0.0,
        M31: 3.0,
        M32: -4.0,
    };
    let (_, _, m) = placed_clip_px(&(rect, Some(inv)), 2.0);
    assert_eq!(
        m,
        Some(Matrix3x2 {
            M31: 6.0,
            M32: -8.0,
            ..inv
        }),
        "線形の部分はそのまま・平行移動だけ拡大"
    );
}

// --- 係（WARP の上の WUC の見た目） ---

use crate::ecs::graphics::emit_test_support::warp_device;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::UI::Composition::{CompositionGeometricClip, CompositionRoundedRectangleGeometry};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

fn clip_world() -> (World, Schedule, Entity) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let resource = WucGraphicsResource::new(warp_device()).expect("WUC の資源");
    let visual: windows::UI::Composition::Visual = resource
        .compositor()
        .expect("compositor")
        .CreateSpriteVisual()
        .expect("CreateSpriteVisual")
        .cast()
        .expect("cast");
    let mut world = World::new();
    world.insert_resource(resource);
    let e = world.spawn(Visual::default()).id();
    world.flush();
    world.get_mut::<Arrangement>(e).unwrap().size = crate::ecs::layout::Size {
        width: SIZE.0,
        height: SIZE.1,
    };
    world.entity_mut(e).insert(VisualGraphics::new(visual));
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(clip_sync_system);
    (world, schedule, e)
}

/// 当たっている切り抜きの図形の (位置, 大きさ)。外れていれば `None`。
fn applied(world: &World, e: Entity) -> Option<(f32, f32, f32, f32)> {
    let clip = world.get::<VisualGraphics>(e)?.visual()?.Clip().ok()?;
    let g = clip
        .cast::<CompositionGeometricClip>()
        .unwrap()
        .Geometry()
        .unwrap()
        .cast::<CompositionRoundedRectangleGeometry>()
        .unwrap();
    let (o, s) = (g.Offset().unwrap(), g.Size().unwrap());
    Some((o.X, o.Y, s.X, s.Y))
}

#[test]
fn placement_clip_is_applied_and_removed_with_the_placement_and_the_phase() {
    let (mut world, mut schedule, e) = clip_world();
    schedule.run(&mut world);
    assert_eq!(applied(&world, e), None, "切り抜きなし");

    let p = embedded(
        Matrix3x2::translation(5.0, 5.0),
        PlacementClip::Rect(Aabb::new(0.0, 0.0, 50.0, 40.0)),
    );
    world.entity_mut(e).insert(SurfacePlacement(p));
    schedule.run(&mut world);
    assert_eq!(applied(&world, e), Some((-5.0, -5.0, 50.0, 40.0)));

    // 自動で上げた持ち主は切り抜きを外す
    world
        .entity_mut(e)
        .insert(SurfacePlacement(PlacementKind::Auto {
            origin_px: (0, 0),
            size_px: (10, 10),
        }));
    schedule.run(&mut world);
    assert_eq!(applied(&world, e), None);

    // 置き場が外れたら自分の切り抜きへ戻す
    world.get_mut::<Visual>(e).unwrap().clip = Some(ClipShape::Rectangle);
    world.entity_mut(e).remove::<SurfacePlacement>();
    schedule.run(&mut world);
    assert_eq!(applied(&world, e), Some((0.0, 0.0, 100.0, 50.0)));
}
