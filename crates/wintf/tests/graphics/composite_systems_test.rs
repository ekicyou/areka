//! 入れ物を通した描画面の実 GPU のテスト（wintf-d2d-composite-rules 7.1・要件 4.2, 6.1, 6.2, 7.10）
//!
//! 本物の登録（`EcsWorld` の `PreRenderSurface` と `RenderSurface`）を実の装置と WUC の資源で回す。
//! 窓は HWND の無い `Window` の部品だけ（画面に出さない）。WUC の描画面の画素は
//! `ICompositionDrawingSurfaceInterop2::CopySurface` で D2D のビットマップへ写して読む
//! （research.md §10.5・テストの確かめ方で本番には使わない）。

use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use log_capture_kit::count_levels;
use std::mem::ManuallyDrop;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BLEND_MODE_MULTIPLY, D2D1_COLOR_F,
    D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ,
    D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
    D2D1_INTERPOLATION_MODE_LINEAR, D2D1_MAP_OPTIONS_READ, ID2D1Bitmap1, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::Win32::System::WinRT::Composition::ICompositionDrawingSurfaceInterop2;
use windows::core::Interface;
use windows_numerics::Matrix3x2;
use wintf::ecs::layout::{Arrangement, GlobalArrangement, Size as LayoutSize};
use wintf::ecs::world::{EcsWorld, PreRenderSurface, RenderSurface};
use wintf::ecs::{
    GraphicsCore, PixelRead, Rect, SurfaceGraphics, SurfaceGraphicsDirty, SurfaceImage,
    SurfaceReadback, Visual, VisualDrawContent, Window, WucGraphicsResource,
};
use wintf::numerics::Aabb;

const SIZE: u32 = 32;

fn rect(l: f32, t: f32, r: f32, b: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

/// `area` を `color` で塗る中身。`blend` なら混ぜ方は MULTIPLY、そうでなければ SOURCE_OVER。
fn filled(
    core: &GraphicsCore,
    area: D2D_RECT_F,
    color: D2D1_COLOR_F,
    blend: bool,
) -> VisualDrawContent {
    let device = core.d2d_device().expect("d2d device");
    // SAFETY: Windows API 境界。記録用の DC で命令の列を 1 本記録して閉じる。
    let list = unsafe {
        let dc = device
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("recording dc");
        let list = dc.CreateCommandList().expect("command list");
        dc.SetTarget(&list);
        dc.BeginDraw();
        let brush = dc.CreateSolidColorBrush(&color, None).expect("brush");
        dc.FillRectangle(&area, &brush);
        dc.EndDraw(None, None).expect("EndDraw");
        list.Close().expect("close");
        list
    };
    if blend {
        VisualDrawContent::new_blend(
            Aabb(area),
            list,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_BLEND_MODE_MULTIPLY,
        )
    } else {
        VisualDrawContent::new(Aabb(area), list)
    }
}

fn color(r: f32, g: f32, b: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a: 1.0 }
}

/// 実の装置と WUC の資源を入れ、HWND の無い窓の下に 32x32 の持ち主と 3 つの節を置く。
/// 節は 赤の全面 → 不透明度 0.5 の青 (8,8)-(24,24) → MULTIPLY の黄 (12,12)-(20,20) の順。
fn setup() -> (EcsWorld, Entity) {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    let red = filled(
        &core,
        rect(0.0, 0.0, 32.0, 32.0),
        color(1.0, 0.0, 0.0),
        false,
    );
    let blue = filled(
        &core,
        rect(8.0, 8.0, 24.0, 24.0),
        color(0.0, 0.0, 1.0),
        false,
    );
    let yellow = filled(
        &core,
        rect(12.0, 12.0, 20.0, 20.0),
        color(1.0, 1.0, 0.0),
        true,
    );
    world.insert_resource(core);
    world.insert_resource(wuc);
    let window = world.spawn(Window::default()).id();
    let owner = world
        .spawn((
            Visual::default(),
            Arrangement {
                size: LayoutSize {
                    width: SIZE as f32,
                    height: SIZE as f32,
                },
                ..Default::default()
            },
            ChildOf(window),
        ))
        .id();
    let mut half = Visual::default();
    half.set_opacity(0.5);
    world.spawn((Visual::default(), red, ChildOf(owner)));
    world.spawn((half, blue, ChildOf(owner)));
    world.spawn((Visual::default(), yellow, ChildOf(owner)));
    world.flush();
    // レイアウトは回さないので、描画面の大きさは配置の結果で与える
    *world.get_mut::<GlobalArrangement>(owner).unwrap() = GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: SIZE as f32,
            bottom: SIZE as f32,
        },
    };
    (ecs, owner)
}

fn bitmap(dc: &ID2D1DeviceContext, options: D2D1_BITMAP_OPTIONS) -> ID2D1Bitmap1 {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: options,
        colorContext: ManuallyDrop::new(None),
    };
    let size = D2D_SIZE_U {
        width: SIZE,
        height: SIZE,
    };
    // SAFETY: Windows API 境界。初期の画素は渡さない。
    unsafe { dc.CreateBitmap(size, None, 0, &props) }.expect("CreateBitmap")
}

/// WUC の描画面を D2D のビットマップへ写し、行を幅×4 に詰めた BGRA で返す。
fn read_wuc_surface(world: &World, owner: Entity) -> Vec<u8> {
    let surface = world
        .get::<SurfaceGraphics>(owner)
        .unwrap()
        .surface()
        .expect("WUC surface")
        .cast::<ICompositionDrawingSurfaceInterop2>()
        .expect("ICompositionDrawingSurfaceInterop2");
    let device = world.resource::<GraphicsCore>().d2d_device().expect("d2d");
    let row = SIZE as usize * 4;
    let mut bytes = vec![0u8; row * SIZE as usize];
    // SAFETY: Windows API 境界。`Map(READ)` の `bits` は少なくとも `pitch * 高さ` バイトを指す。
    unsafe {
        let dc = device
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("dc");
        let target = bitmap(&dc, D2D1_BITMAP_OPTIONS_TARGET);
        let dxgi: IDXGISurface = target.GetSurface().expect("IDXGISurface");
        surface.CopySurface(&dxgi, 0, 0, None).expect("CopySurface");
        let staging = bitmap(
            &dc,
            D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        );
        staging
            .CopyFromBitmap(None, &target, None)
            .expect("CopyFromBitmap");
        let mapped = staging.Map(D2D1_MAP_OPTIONS_READ).expect("Map");
        for y in 0..SIZE as usize {
            std::ptr::copy_nonoverlapping(
                mapped.bits.add(y * mapped.pitch as usize),
                bytes.as_mut_ptr().add(y * row),
                row,
            );
        }
        staging.Unmap().expect("Unmap");
    }
    bytes
}

fn near(actual: [u8; 4], expected: [u8; 4]) -> bool {
    actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1)
}

/// ブレンドの節と不透明度 0.5 の節を含む木を描くと、`error!` が 0 で描き直しが続かず、
/// WUC の描画面の画素が入れ物の画素とバイトで一致し、1 画素の読み戻しが答えの点の値を返す
#[test]
fn composite_reaches_wuc_surface_on_real_gpu() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, owner) = setup();
        let ((), counts) = count_levels(|| {
            let world = ecs.world_mut();
            world.run_schedule(PreRenderSurface);
            world.run_schedule(RenderSurface);
        });
        assert_eq!(counts.error, 0, "描画の係が error を残した");
        let world = ecs.world_mut();
        assert_eq!(
            world.get::<SurfaceGraphics>(owner).unwrap().size,
            (SIZE, SIZE)
        );

        // 次の回に描き直しの印が立ち直らない（失敗の立て直しが続かない）
        world.increment_change_tick();
        let this_run = world.change_tick();
        let ((), counts) = count_levels(|| {
            world.run_schedule(PreRenderSurface);
            world.run_schedule(RenderSurface);
        });
        assert_eq!(counts.error, 0, "2 回目の描画が error を残した");
        let dirty = world
            .entity(owner)
            .get_change_ticks::<SurfaceGraphicsDirty>()
            .unwrap()
            .changed;
        assert!(dirty.get() < this_run.get(), "描き直しが続いている");

        let wuc = read_wuc_surface(world, owner);
        let (image, outer, half, blended) = world
            .run_system_once(move |readback: SurfaceReadback| {
                (
                    readback.read_all(owner),
                    readback.read_pixel(owner, 4, 4),
                    readback.read_pixel(owner, 10, 10),
                    readback.read_pixel(owner, 16, 16),
                )
            })
            .unwrap();
        let SurfaceImage::Pixels {
            width,
            height,
            bytes,
        } = image
        else {
            panic!("入れ物を読めない: {image:?}");
        };
        assert_eq!((width, height), (SIZE, SIZE));
        assert!(bytes == wuc, "WUC の描画面の画素が入れ物の画素と違う");

        // 赤だけ・赤に青 0.5・その上に黄の MULTIPLY（α 掛け済みの BGRA・±1）
        let expect = [
            (outer, [0, 0, 255, 255]),
            (half, [128, 0, 128, 255]),
            (blended, [0, 0, 128, 255]),
        ];
        for (read, expected) in expect {
            let PixelRead::Pixel(px) = read else {
                panic!("1 画素を読めない: {read:?}");
            };
            assert!(
                near(px, expected),
                "答えの点の値が違う: {px:?} 見本 {expected:?}"
            );
        }
        world.clear_entities();
    });
}
