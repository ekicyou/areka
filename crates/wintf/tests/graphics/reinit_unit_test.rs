/// GraphicsCore再初期化システムの単体テスト
///
/// このテストはGraphicsCore、WindowGraphics、Visual、Surfaceの
/// Option<T>ラップと状態遷移機能をテストします。
///
/// WUC 移行: Visual/Surface は WUC オブジェクト（SpriteVisual / CompositionDrawingSurface）で構築する。
use wintf::ecs::{GraphicsCore, SurfaceGraphics, VisualGraphics, WucGraphicsResource};

/// WucGraphicsResource::new は DQTAT_COM_NONE を使うため COM 初期化済みスレッドを要求する。
fn init_com_mta() {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

#[test]
fn test_graphics_core_invalidate_and_is_valid() {
    crate::common::on_gpu_owner_thread(move || {
    let mut graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");

    // 初期状態: 有効
    assert!(
        graphics.is_valid(),
        "GraphicsCore should be valid after creation"
    );

    // 無効化
    graphics.invalidate();
    assert!(
        !graphics.is_valid(),
        "GraphicsCore should be invalid after invalidate()"
    );

    // アクセサメソッドがNoneを返す
    assert!(
        graphics.d2d_factory().is_none(),
        "d2d_factory() should return None after invalidate()"
    );
    assert!(
        graphics.d2d_device().is_none(),
        "d2d_device() should return None after invalidate()"
    );
    // Note: dcomp() and desktop() are no longer on GraphicsCore (moved to DCompGraphicsResource)
    assert!(
        graphics.dwrite_factory().is_none(),
        "dwrite_factory() should return None after invalidate()"
    );
    });
}

#[test]
fn test_visual_new_and_invalidate() {
    crate::common::on_gpu_owner_thread(move || {
    use windows::UI::Composition::Visual;
    use windows::core::Interface;

    init_com_mta();
    let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");
    let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
    let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");
    let compositor = wuc_resource.compositor().expect("compositor should exist");

    let visual_raw: Visual = compositor
        .CreateSpriteVisual()
        .expect("CreateSpriteVisual")
        .cast()
        .expect("cast to Visual");
    let mut visual = VisualGraphics::new(visual_raw);

    // 初期状態: 有効
    assert!(visual.is_valid(), "Visual should be valid after creation");
    assert!(
        visual.visual().is_some(),
        "visual() should return Some after creation"
    );

    // 無効化
    visual.invalidate();
    assert!(
        !visual.is_valid(),
        "Visual should be invalid after invalidate()"
    );
    assert!(
        visual.visual().is_none(),
        "visual() should return None after invalidate()"
    );
    });
}

#[test]
fn test_surface_new_and_invalidate() {
    crate::common::on_gpu_owner_thread(move || {
    use windows::Foundation::Size;
    use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};

    init_com_mta();
    let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");
    let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
    let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");
    let compositor = wuc_resource.compositor().expect("compositor should exist");
    let gd = wuc_resource.graphics_device().expect("graphics_device");

    let surface_raw = gd
        .CreateDrawingSurface(
            Size {
                Width: 800.0,
                Height: 600.0,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )
        .expect("Surface作成失敗");
    let brush = compositor
        .CreateSurfaceBrushWithSurface(&surface_raw)
        .expect("CreateSurfaceBrushWithSurface");

    let mut surface = SurfaceGraphics::new(surface_raw, brush, (800, 600));

    // 初期状態: 有効
    assert!(surface.is_valid(), "Surface should be valid after creation");
    assert!(
        surface.surface().is_some(),
        "surface() should return Some after creation"
    );

    // 無効化
    surface.invalidate();
    assert!(
        !surface.is_valid(),
        "Surface should be invalid after invalidate()"
    );
    assert!(
        surface.surface().is_none(),
        "surface() should return None after invalidate()"
    );
    });
}

#[test]
fn test_graphics_core_reinitialize() {
    crate::common::on_gpu_owner_thread(move || {
    let mut graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");

    // 初期状態
    assert!(graphics.is_valid());
    let initial_d2d = graphics.d2d_device().unwrap() as *const _;

    // 無効化
    graphics.invalidate();
    assert!(!graphics.is_valid());

    // 再初期化
    let new_graphics = GraphicsCore::new().expect("GraphicsCore再初期化失敗");
    assert!(new_graphics.is_valid());

    // 新しいインスタンスは異なるポインタを持つ
    let new_d2d = new_graphics.d2d_device().unwrap() as *const _;
    assert_ne!(
        initial_d2d, new_d2d,
        "Reinitialized GraphicsCore should have different device pointers"
    );
    });
}
