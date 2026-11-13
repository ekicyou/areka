// タスク7.3: GraphicsCoreからのCOMオブジェクト作成テスト
//
// WUC 移行: DComp デバイス/Visual/Commit を直接叩いていた旧テスト
// （test_create_visual / test_create_multiple_visuals / test_commit）は、
// (1) create_visual は WUC の Compositor::CreateSpriteVisual へ写像され、
// (2) commit は暗黙反映化で廃止された（要件 7.1）ため、WUC 等価テストへ更新した。
// WUC デバイス群のライフサイクル/往復は wuc_resource.rs・com/wuc.rs の統合テストが
// 別途担保する。ここでは GraphicsCore＋WUC Compositor から Visual を作れることを確認する。
//
// COM は MTA 初期化してから WucGraphicsResource::new を呼ぶ（本番 UI スレッド再現）。

#[cfg(test)]
mod graphics_core_tests {
    use crate::ecs::graphics::GraphicsCore;
    use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
    use windows::Win32::Graphics::Direct2D::D2D1_DEVICE_CONTEXT_OPTIONS_NONE;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    fn init_mta() {
        // S_FALSE / RPC_E_CHANGED_MODE は無視（既に初期化済みでも可）。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    #[test]
    fn test_graphics_core_creation() {
        let _graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");

        // GraphicsCoreが正常に作成されたことを確認（すべてのフィールドが初期化されている）
        println!("[TEST PASS] GraphicsCore created successfully with all valid devices");
    }

    #[test]
    fn test_create_device_context() {
        let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");

        use crate::com::d2d::D2D1DeviceExt;
        let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
        let _dc = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("DeviceContext作成失敗");

        println!("[TEST PASS] ID2D1DeviceContext created successfully");
    }

    #[test]
    fn test_create_visual() {
        init_mta();
        let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");
        let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
        let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");

        let compositor = wuc_resource.compositor().expect("Compositorが無効");
        // DComp の create_visual に対応する WUC 生成（全 Visual を SpriteVisual で統一）。
        let _visual = compositor
            .CreateSpriteVisual()
            .expect("SpriteVisual作成失敗");

        println!("[TEST PASS] WUC SpriteVisual created successfully");
    }

    #[test]
    fn test_create_multiple_device_contexts() {
        let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");

        use crate::com::d2d::D2D1DeviceExt;
        let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");

        // 複数のDeviceContextを作成できることを確認
        let _dc1 = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("DeviceContext1作成失敗");

        let _dc2 = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("DeviceContext2作成失敗");

        let _dc3 = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("DeviceContext3作成失敗");

        println!("[TEST PASS] Multiple ID2D1DeviceContext created successfully");
    }

    #[test]
    fn test_create_multiple_visuals() {
        init_mta();
        let graphics = GraphicsCore::new().expect("GraphicsCore作成失敗");
        let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
        let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");

        let compositor = wuc_resource.compositor().expect("Compositorが無効");

        // 複数のVisualを作成できることを確認
        let _v1 = compositor
            .CreateSpriteVisual()
            .expect("SpriteVisual1作成失敗");
        let _v2 = compositor
            .CreateSpriteVisual()
            .expect("SpriteVisual2作成失敗");
        let _v3 = compositor
            .CreateSpriteVisual()
            .expect("SpriteVisual3作成失敗");

        println!("[TEST PASS] Multiple WUC SpriteVisual created successfully");
    }

    // Note: 旧 test_commit（IDCompositionDevice3::Commit）は WUC 移行で削除。
    // WUC は DispatcherQueue 経由の暗黙反映のため明示 commit が存在しない（要件 7.1）。
}

// 入れ物の寿命（要件 4.3, 4.5）: 描画面の入れ替え（大きさの変化）と放し（持ち主から外れる・
// 装置の失い。どちらも `clear` を通る）の後に入れ物が残らない。
#[cfg(test)]
mod surface_composite_lifetime_tests {
    use crate::ecs::graphics::GraphicsCore;
    use crate::ecs::graphics::SurfaceGraphics;
    use crate::ecs::graphics::composite::SurfaceComposite;
    use crate::ecs::graphics::visual::emit_test_support::warp_dc;
    use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
    use windows::Foundation::Size;
    use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
    use windows::UI::Composition::{CompositionDrawingSurface, CompositionSurfaceBrush};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    fn make_surface(
        resource: &WucGraphicsResource,
        w: u32,
        h: u32,
    ) -> (CompositionDrawingSurface, CompositionSurfaceBrush) {
        let gd = resource.graphics_device().expect("graphics_device");
        let surface = gd
            .CreateDrawingSurface(
                Size {
                    Width: w as f32,
                    Height: h as f32,
                },
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )
            .expect("CreateDrawingSurface");
        let brush = resource
            .compositor()
            .expect("compositor")
            .CreateSurfaceBrushWithSurface(&surface)
            .expect("CreateSurfaceBrushWithSurface");
        (surface, brush)
    }

    fn fill_composite(sg: &mut SurfaceGraphics) {
        let dc = warp_dc();
        *sg.composite_slot() = Some(SurfaceComposite::new(&dc, sg.size).expect("入れ物"));
        assert!(sg.composite().is_some());
    }

    #[test]
    fn set_surface_and_clear_release_the_composite() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let core = GraphicsCore::new().expect("GraphicsCore作成失敗");
        let resource =
            WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WucGraphicsResource");
        let (s1, b1) = make_surface(&resource, 16, 16);
        let (s2, b2) = make_surface(&resource, 24, 20);

        let mut sg = SurfaceGraphics::new(s1, b1, (16, 16));
        assert!(sg.composite().is_none(), "作った直後は入れ物が無い");
        fill_composite(&mut sg);
        sg.set_surface(s2, b2, (24, 20));
        assert!(
            sg.composite().is_none(),
            "描画面を入れ替えた後に入れ物が残る"
        );

        fill_composite(&mut sg);
        sg.clear();
        assert!(sg.composite().is_none(), "描画面を放した後に入れ物が残る");
    }
}
