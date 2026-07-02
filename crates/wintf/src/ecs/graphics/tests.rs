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
        let _visual = compositor.CreateSpriteVisual().expect("SpriteVisual作成失敗");

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
        let _v1 = compositor.CreateSpriteVisual().expect("SpriteVisual1作成失敗");
        let _v2 = compositor.CreateSpriteVisual().expect("SpriteVisual2作成失敗");
        let _v3 = compositor.CreateSpriteVisual().expect("SpriteVisual3作成失敗");

        println!("[TEST PASS] Multiple WUC SpriteVisual created successfully");
    }

    // Note: 旧 test_commit（IDCompositionDevice3::Commit）は WUC 移行で削除。
    // WUC は DispatcherQueue 経由の暗黙反映のため明示 commit が存在しない（要件 7.1）。
}

// Task 3.1: HasGraphicsResources メソッドのユニットテスト
// Note: HasGraphicsResources は空マーカーに変更されたため、
// 古いテスト（needs_init, request_init, mark_initialized）は削除
// Changed<HasGraphicsResources> で再初期化トリガーを検知する設計に移行
#[cfg(test)]
mod has_graphics_resources_tests {
    use crate::ecs::graphics::HasGraphicsResources;

    #[test]
    fn test_default_is_unit_struct() {
        // 空マーカーコンポーネントとして機能することを確認
        let _res = HasGraphicsResources::default();
        // HasGraphicsResources は () と同等の空構造体
    }

    #[test]
    fn test_clone_and_partial_eq() {
        let res1 = HasGraphicsResources::default();
        let res2 = res1.clone();
        assert_eq!(res1, res2, "クローンは同一");
    }
}

// Task 3.1: SurfaceGraphicsDirty コンポーネントのユニットテスト
#[cfg(test)]
mod surface_graphics_dirty_tests {
    use crate::ecs::graphics::SurfaceGraphicsDirty;

    #[test]
    fn test_default_requested_frame_is_zero() {
        let dirty = SurfaceGraphicsDirty::default();
        assert_eq!(dirty.requested_frame, 0, "デフォルトのrequested_frameは0");
    }

    #[test]
    fn test_requested_frame_can_be_updated() {
        let mut dirty = SurfaceGraphicsDirty::default();
        dirty.requested_frame = 42;
        assert_eq!(dirty.requested_frame, 42, "requested_frameを更新できる");
    }
}
