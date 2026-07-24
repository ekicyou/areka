//! window_pos.rs 系システムのギャップテスト (W3b-T)
//!
//! - invalidate_dependent_components: WucGraphicsResource 連動は既存テスト
//!   （dcomp_integration_test）で固定済みだが、BitmapSourceGraphics の無効化ループと
//!   no-op 経路は未検証だったため追加する。
//! - apply_window_pos_changes: SetWindowPosCommand キュー（TLS）は覗き見 API が
//!   ないため、分岐の完走 characterization に留める（所見・提案はセル断片に記録）。
//!
//! 対象: crates/wintf/src/ecs/graphics/systems/window_pos.rs

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use wintf::ecs::{
    BitmapSourceGraphics, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    apply_window_pos_changes, invalidate_dependent_components,
};

fn run_invalidate(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(invalidate_dependent_components);
    schedule.run(world);
}

fn run_apply_window_pos(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(apply_window_pos_changes);
    schedule.run(world);
}

/// テスト用の実 ID2D1Bitmap1 を作成する（CPU 読み出し不可・描画不可の最小構成）
fn create_test_bitmap(graphics: &GraphicsCore) -> ID2D1Bitmap1 {
    let dc = graphics.device_context().expect("device context");
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        dpiX: 96.0,
        dpiY: 96.0,
        ..Default::default()
    };
    let size = D2D_SIZE_U {
        width: 2,
        height: 2,
    };
    unsafe { dc.CreateBitmap(size, None, 0, &props) }.expect("test bitmap")
}

// ========================================================================
// invalidate_dependent_components
// ========================================================================

/// GraphicsCore 無効時に BitmapSourceGraphics が無効化される
#[test]
fn invalidate_invalidates_bitmap_source_when_graphics_invalid() {
    crate::common::on_gpu_owner_thread(move || {
    let mut graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let bitmap = create_test_bitmap(&graphics);

    let mut bsg = BitmapSourceGraphics::new();
    bsg.set_bitmap(bitmap);
    assert!(bsg.is_valid());

    graphics.invalidate();

    let mut world = World::new();
    world.insert_resource(graphics);
    let entity = world.spawn(bsg).id();

    run_invalidate(&mut world);

    let bsg = world.get::<BitmapSourceGraphics>(entity).unwrap();
    assert!(
        !bsg.is_valid(),
        "bitmap source should be invalidated when GraphicsCore is invalid"
    );
    });
}

/// GraphicsCore 無効時でも WUC 系コンポーネント（VisualGraphics / SurfaceGraphics）は
/// 無効化されず stale なまま「有効」と判定され続ける（Phase 2 で無効化対象から除外された
/// 現行挙動の固定）。再初期化側は `!is_valid()` を前提とするため、デバイスロスト検出
/// （P40）が実装されてもこのままでは WUC パイプラインは復旧しない（W3b-V 断片参照）
///
/// WUC 移行: invalidate_dependent_components は WucGraphicsResource を無効化する。
#[test]
fn invalidate_leaves_wuc_graphics_components_stale() {
    crate::common::on_gpu_owner_thread(move || {
    use windows::Foundation::Size;
    use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
    use windows::UI::Composition::Visual;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use windows::core::Interface;
    use wintf::ecs::{SurfaceGraphics, VisualGraphics, WucGraphicsResource};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let d2d = graphics.d2d_device().expect("d2d device");
    let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource");
    let compositor = wuc_resource.compositor().expect("compositor").clone();
    let gd = wuc_resource.graphics_device().expect("graphics_device").clone();

    let visual: Visual = compositor
        .CreateSpriteVisual()
        .expect("visual")
        .cast()
        .expect("cast to Visual");
    let surface = gd
        .CreateDrawingSurface(
            Size {
                Width: 4.0,
                Height: 4.0,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )
        .expect("surface");
    let brush = compositor
        .CreateSurfaceBrushWithSurface(&surface)
        .expect("brush");

    // デバイスロストを模擬
    graphics.invalidate();

    let mut world = World::new();
    world.insert_resource(graphics);
    world.insert_resource(wuc_resource);
    let entity = world
        .spawn((
            VisualGraphics::new(visual),
            SurfaceGraphics::new(surface, brush, (4, 4)),
        ))
        .id();

    run_invalidate(&mut world);

    // WucGraphicsResource は無効化される（既存設計どおり）
    assert!(
        !world.resource::<WucGraphicsResource>().is_valid(),
        "WucGraphicsResource should be invalidated"
    );

    // 一方 VisualGraphics / SurfaceGraphics は旧デバイス由来の COM ポインタを
    // 保持したまま「有効」と判定され続ける（characterization: 再初期化トリガー不発）
    let vg = world.get::<VisualGraphics>(entity).unwrap();
    assert!(
        vg.is_valid(),
        "characterization: VisualGraphics stays stale-valid (not invalidated)"
    );
    let sg = world.get::<SurfaceGraphics>(entity).unwrap();
    assert!(
        sg.is_valid(),
        "characterization: SurfaceGraphics stays stale-valid (not invalidated)"
    );
    });
}

/// GraphicsCore リソース不在時は早期リターン（Option<Res> = None 経路）
#[test]
fn invalidate_noop_without_graphics_resource() {
    crate::common::on_gpu_owner_thread(move || {
    let graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let bitmap = create_test_bitmap(&graphics);
    let mut bsg = BitmapSourceGraphics::new();
    bsg.set_bitmap(bitmap);
    assert!(bsg.is_valid());
    drop(graphics); // リソースとして挿入しない

    let mut world = World::new();
    let entity = world.spawn(bsg).id();

    run_invalidate(&mut world);

    let bsg = world.get::<BitmapSourceGraphics>(entity).unwrap();
    assert!(
        bsg.is_valid(),
        "missing GraphicsCore resource must be a no-op"
    );
    });
}

// ========================================================================
// apply_window_pos_changes（characterization: TLS キューは観測 API なし）
// ========================================================================

/// CW_USEDEFAULT の WindowPos はスキップされる（パニックせず完走）
#[test]
fn apply_window_pos_skips_cw_usedefault() {
    crate::common::on_gpu_owner_thread(move || {
    let mut world = World::new();
    world.spawn((
        Window::default(),
        WindowHandle {
            hwnd: HWND::default(),
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point {
                x: CW_USEDEFAULT,
                y: 0,
            }),
            size: None,
            ..Default::default()
        },
    ));

    // CW_USEDEFAULT 検出で continue する経路。座標変換も enqueue も行われない。
    run_apply_window_pos(&mut world);
    });
}

/// 無効 HWND では座標変換が失敗しフォールバック経路を通る（パニックせず完走）
///
/// Note: 本テストは TLS キューに 1 件 enqueue するが flush しないため、
/// 実 SetWindowPos は呼ばれずスレッド終了時に破棄される。
#[test]
fn apply_window_pos_fallback_on_invalid_hwnd() {
    crate::common::on_gpu_owner_thread(move || {
    let mut world = World::new();
    world.spawn((
        Window::default(),
        WindowHandle {
            hwnd: HWND::default(),
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point { x: 10, y: 20 }),
            size: Some(SizeI {
                width: 100,
                height: 50,
            }),
            ..Default::default()
        },
    ));

    run_apply_window_pos(&mut world);
    });
}

/// Window を持たないエンティティは With<Window> フィルタで対象外（完走）
#[test]
fn apply_window_pos_ignores_non_window_entities() {
    crate::common::on_gpu_owner_thread(move || {
    let mut world = World::new();
    world.spawn((
        WindowHandle {
            hwnd: HWND::default(),
            instance: HINSTANCE::default(),
        },
        WindowPos {
            position: Some(Point { x: 1, y: 2 }),
            size: None,
            ..Default::default()
        },
    ));

    run_apply_window_pos(&mut world);
    });
}
