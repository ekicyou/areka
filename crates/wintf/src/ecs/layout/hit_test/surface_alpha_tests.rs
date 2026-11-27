//! `HitTestMode::SurfaceAlpha` と `surface_alpha_at`（実 GPU の入れ物）

use super::tests::make_global_arrangement;
use super::*;
use crate::ecs::graphics::composite::SurfaceComposite;
use crate::ecs::graphics::{
    AlphaRead, AutoPromotion, GraphicsCore, Phase, PlacementKind, SurfaceGraphics,
    SurfacePlacement, WucGraphicsResource, over, surface_alpha_at,
};
use log_capture_kit::count_levels;
use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::{D2D1_DEVICE_CONTEXT_OPTIONS_NONE, ID2D1DeviceContext};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_numerics::Matrix3x2;

const SIZE: (u32, u32) = (8, 8);

struct Gpu {
    core: GraphicsCore,
    resource: WucGraphicsResource,
    dc: ID2D1DeviceContext,
}

fn gpu() -> Gpu {
    // SAFETY: 呼ぶスレッドの COM を MTA で初期化するだけ（既に初期化済みでも害は無い）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore");
    let resource = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    // SAFETY: Windows API 境界。有効な装置から DC を作るだけ。
    let dc = unsafe {
        core.d2d_device()
            .unwrap()
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
    }
    .expect("DC");
    Gpu { core, resource, dc }
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

/// `area` を α `alpha` の白で塗って 1 度合成した入れ物を持つ描画面（実 GPU の装置）。
fn surface(gpu: &Gpu, size: (u32, u32), area: D2D_RECT_F, alpha: f32) -> SurfaceGraphics {
    let drawing = gpu
        .resource
        .graphics_device()
        .expect("graphics_device")
        .CreateDrawingSurface(
            Size {
                Width: size.0 as f32,
                Height: size.1 as f32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )
        .expect("CreateDrawingSurface");
    let brush = gpu
        .resource
        .compositor()
        .expect("compositor")
        .CreateSurfaceBrushWithSurface(&drawing)
        .expect("brush");
    let mut sg = SurfaceGraphics::new(drawing, brush, size);
    let dc = &gpu.dc;
    // SAFETY: Windows API 境界。同じ DC で命令の列を 1 本記録して閉じる。
    let list = unsafe {
        let list = dc.CreateCommandList().unwrap();
        dc.SetTarget(&list);
        dc.BeginDraw();
        let color = D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: alpha,
        };
        let b = dc.CreateSolidColorBrush(&color, None).unwrap();
        dc.FillRectangle(&area, &b);
        dc.EndDraw(None, None).unwrap();
        dc.SetTarget(None);
        list.Close().unwrap();
        list
    };
    let mut comp = SurfaceComposite::new(dc, size).expect("入れ物");
    comp.compose(dc, Matrix3x2::identity(), Some(&list), &[])
        .expect("合成");
    *sg.composite_slot() = Some(comp);
    sg
}

fn world_with(gpu: Gpu, sg: SurfaceGraphics, ga: GlobalArrangement) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(gpu.core);
    let owner = world.spawn((sg, HitTest::surface_alpha(), ga)).id();
    (world, owner)
}

/// 左半分が不透明な 8×8 の持ち主を (10, 20) に置いた World。
fn world_with_owner() -> (World, Entity) {
    let gpu = gpu();
    let sg = surface(&gpu, SIZE, rect(0.0, 0.0, 4.0, 8.0), 1.0);
    world_with(gpu, sg, make_global_arrangement(10.0, 20.0, 18.0, 28.0))
}

#[test]
fn opaque_pixel_hits_and_transparent_pixel_does_not() {
    let (world, owner) = world_with_owner();
    assert!(hit_test_entity(&world, owner, PointF::new(11.5, 21.5)));
    assert!(!hit_test_entity(&world, owner, PointF::new(15.5, 21.5)));
    assert_eq!(surface_alpha_at(&world, owner, 1, 1), AlphaRead::Alpha(255));
    assert_eq!(surface_alpha_at(&world, owner, 5, 1), AlphaRead::Alpha(0));
    assert_eq!(surface_alpha_at(&world, owner, 8, 0), AlphaRead::Outside);
}

#[test]
fn same_point_and_generation_does_not_read_gpu_again() {
    let (world, owner) = world_with_owner();
    let reads = || {
        world
            .get::<SurfaceGraphics>(owner)
            .unwrap()
            .composite()
            .unwrap()
            .gpu_reads()
    };
    surface_alpha_at(&world, owner, 2, 2);
    let first = reads();
    assert_eq!(first, 1);
    surface_alpha_at(&world, owner, 2, 2);
    assert_eq!(reads(), first);
}

#[test]
fn alpha_below_half_does_not_hit() {
    let gpu = gpu();
    let sg = surface(&gpu, SIZE, rect(0.0, 0.0, 8.0, 8.0), 0.4);
    let (world, owner) = world_with(gpu, sg, make_global_arrangement(0.0, 0.0, 8.0, 8.0));
    assert!(!hit_test_entity(&world, owner, PointF::new(3.0, 3.0)));
}

#[test]
fn active_auto_descendant_is_overlaid() {
    let gpu = gpu();
    // 持ち主は全体が α 0.3、上げた子は (4, 0) に 4×4 で全体が α 0.3。
    let parent = surface(&gpu, SIZE, rect(0.0, 0.0, 8.0, 8.0), 0.3);
    let child = surface(&gpu, (4, 4), rect(0.0, 0.0, 4.0, 4.0), 0.3);
    let preparing = surface(&gpu, (4, 4), rect(0.0, 0.0, 4.0, 4.0), 1.0);
    let (mut world, owner) = world_with(gpu, parent, make_global_arrangement(0.0, 0.0, 8.0, 8.0));
    let auto = |origin_px| {
        SurfacePlacement(PlacementKind::Auto {
            origin_px,
            size_px: (4, 4),
        })
    };
    let node = world.spawn(ChildOf(owner)).id();
    world.spawn((
        ChildOf(node),
        child,
        auto((4, 0)),
        AutoPromotion::in_phase(Phase::Active),
    ));
    // 用意中の子はまだ親の描画面に描かれているので重ねない。
    world.spawn((
        ChildOf(owner),
        preparing,
        auto((0, 4)),
        AutoPromotion::in_phase(Phase::Preparing),
    ));

    let AlphaRead::Alpha(alone) = surface_alpha_at(&world, owner, 1, 1) else {
        panic!("読めない");
    };
    assert!(alone.abs_diff(77) <= 1, "alone={alone}");
    assert_eq!(
        surface_alpha_at(&world, owner, 5, 1),
        AlphaRead::Alpha(over(alone, alone))
    );
    assert_eq!(
        surface_alpha_at(&world, owner, 1, 5),
        AlphaRead::Alpha(alone)
    );
    assert!(!hit_test_entity(&world, owner, PointF::new(1.0, 1.0)));
    // 0.3 と 0.3 を重ねた 0.51 は当たる。
    assert!(hit_test_entity(&world, owner, PointF::new(5.0, 1.0)));
}

#[test]
fn unreadable_misses_and_logs_once_while_unreadable() {
    let (mut world, owner) = world_with_owner();
    let core = world.remove_resource::<GraphicsCore>().unwrap();
    let point = PointF::new(11.5, 21.5);
    let (hits, levels) = count_levels(|| {
        [
            hit_test_entity(&world, owner, point),
            hit_test_entity(&world, owner, point),
        ]
    });
    assert_eq!(hits, [false, false]);
    assert_eq!(levels.error, 1);

    // 読めたら印を戻し、また読めなくなれば 1 件。
    world.insert_resource(core);
    assert!(hit_test_entity(&world, owner, point));
    world.remove_resource::<GraphicsCore>();
    let (hit, levels) = count_levels(|| hit_test_entity(&world, owner, point));
    assert!(!hit);
    assert_eq!(levels.error, 1);
}
