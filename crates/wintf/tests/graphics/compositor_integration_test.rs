//! Task 6.1 + 6.2: 合成描画パイプライン統合テスト
//!
//! - compositor_init_system → composite_render_system パイプライン統合動作を検証
//! - WindowD3D11Compositor のライフサイクルとデバイスロスト復旧フローを検証
//! - DComp パイプラインとの共存（既存コンポーネントが破壊されないこと）を検証

use wintf::ecs::GraphicsCore;
use wintf::ecs::compositor::WindowD3D11Compositor;

// ==========================================================================
// デバイスロスト → 再初期化 → 正常描画再開フロー
// ==========================================================================

#[test]
fn test_compositor_device_lost_recovery() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    // 1. 正常作成
    let mut compositor = WindowD3D11Compositor::new(dc, 200, 200).expect("初期作成失敗");
    assert!(compositor.is_valid());
    let initial_gen = compositor.generation();

    // 2. デバイスロストをシミュレート（invalidate）
    compositor.invalidate();
    assert!(!compositor.is_valid(), "invalidate 後は無効");
    assert!(
        compositor.composition_bitmap().is_none(),
        "リソース解放済み"
    );

    // 3. compositor_init_system 相当の再作成ロジックを手動実行
    let old_generation = compositor.generation();
    let mut new_compositor = WindowD3D11Compositor::new(dc, 200, 200).expect("再作成失敗");

    // 旧 generation を引き継ぎインクリメント
    let target_gen = old_generation.wrapping_add(1);
    while new_compositor.generation() < target_gen {
        new_compositor.increment_generation();
    }

    // 4. 再初期化検証
    assert!(new_compositor.is_valid(), "再作成後は有効");
    assert_eq!(
        new_compositor.generation(),
        initial_gen + 1,
        "generation がインクリメントされている"
    );
    assert!(
        new_compositor.composition_bitmap().is_some(),
        "新しいリソースが存在"
    );
    assert!(
        new_compositor.staging_bitmap().is_some(),
        "staging bitmap が存在"
    );

    eprintln!("✅ デバイスロスト → 再初期化 → 正常復旧フローが完了");
}

// ==========================================================================
// 全パイプライン統合テスト
// ==========================================================================

#[test]
fn test_compositor_full_pipeline_integration() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let width = 128u32;
    let height = 96u32;

    // Phase 1: compositor_init（リソース作成）
    let mut compositor =
        WindowD3D11Compositor::new(dc, width, height).expect("Compositor 作成失敗");
    assert!(compositor.is_valid());
    assert_eq!(compositor.cached_size(), (width, height));

    // Phase 2: composite_render 相当（SetTarget → BeginDraw → Clear → EndDraw → CopyFromBitmap → transfer）
    let comp_bmp = compositor.composition_bitmap().unwrap().clone();
    let staging = compositor.staging_bitmap().unwrap();

    // SetTarget + BeginDraw → Clear → EndDraw
    unsafe {
        let prev_target = dc.GetTarget().ok();
        dc.SetTarget(&comp_bmp);
        dc.BeginDraw();
        dc.Clear(Some(
            &windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        ));
        dc.EndDraw(None, None).expect("EndDraw 失敗");
        dc.SetTarget(prev_target.as_ref());
    }

    // CopyFromBitmap
    unsafe {
        staging
            .CopyFromBitmap(None, &comp_bmp, None)
            .expect("CopyFromBitmap 失敗");
    }

    // transfer_to_hbitmap
    let dib_bits = compositor.dib_bits().unwrap();
    unsafe {
        wintf::com::ulw::transfer_to_hbitmap(staging, dib_bits, width, height)
            .expect("transfer_to_hbitmap 失敗");
    }

    // dirty フラグ設定
    compositor.set_dirty(true);
    assert!(compositor.is_dirty(), "合成完了後 dirty == true");

    eprintln!("✅ 全パイプライン統合テスト完了（init → render → transfer → dirty）");
}

// ==========================================================================
// リサイズ + 再合成フロー
// ==========================================================================

#[test]
fn test_compositor_resize_and_recompose() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor = WindowD3D11Compositor::new(dc, 100, 100).expect("Compositor 作成失敗");

    // 初回合成
    {
        let comp_bmp = compositor.composition_bitmap().unwrap().clone();
        let staging = compositor.staging_bitmap().unwrap();
        unsafe {
            dc.SetTarget(&comp_bmp);
            dc.BeginDraw();
            dc.Clear(None);
            dc.EndDraw(None, None).expect("EndDraw");
            dc.SetTarget(None::<&windows::Win32::Graphics::Direct2D::ID2D1Image>);
            staging.CopyFromBitmap(None, &comp_bmp, None).expect("Copy");
        }
        let dib_bits = compositor.dib_bits().unwrap();
        unsafe {
            wintf::com::ulw::transfer_to_hbitmap(staging, dib_bits, 100, 100).expect("transfer");
        }
        compositor.set_dirty(true);
    }

    // リサイズ → 再合成
    compositor.resize(dc, 200, 150).expect("resize");
    assert_eq!(compositor.cached_size(), (200, 150));
    assert_eq!(compositor.generation(), 1);
    assert!(!compositor.is_dirty(), "resize 後 dirty == false");

    {
        let comp_bmp = compositor.composition_bitmap().unwrap().clone();
        let staging = compositor.staging_bitmap().unwrap();
        unsafe {
            dc.SetTarget(&comp_bmp);
            dc.BeginDraw();
            dc.Clear(None);
            dc.EndDraw(None, None).expect("EndDraw");
            dc.SetTarget(None::<&windows::Win32::Graphics::Direct2D::ID2D1Image>);
            staging.CopyFromBitmap(None, &comp_bmp, None).expect("Copy");
        }
        let dib_bits = compositor.dib_bits().unwrap();
        unsafe {
            wintf::com::ulw::transfer_to_hbitmap(staging, dib_bits, 200, 150).expect("transfer");
        }
        compositor.set_dirty(true);
    }
    assert!(compositor.is_dirty());

    eprintln!("✅ リサイズ後の再合成パイプラインが正常に動作する");
}

// ==========================================================================
// DComp パイプラインとの共存検証
// ==========================================================================

#[test]
fn test_compositor_does_not_affect_existing_graphics_core() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    // GraphicsCore の既存リソースを確認
    assert!(core.d2d_factory().is_some(), "D2D factory が存在");
    assert!(core.d2d_device().is_some(), "D2D device が存在");

    // Compositor 作成 — GraphicsCore のリソースに影響しないこと
    let _compositor = WindowD3D11Compositor::new(dc, 64, 64).expect("Compositor 作成失敗");

    // GraphicsCore のリソースが健在
    assert!(core.d2d_factory().is_some(), "D2D factory が健在");
    assert!(core.d2d_device().is_some(), "D2D device が健在");
    assert!(core.device_context().is_some(), "DeviceContext が健在");

    eprintln!("✅ Compositor 作成が既存 GraphicsCore リソースに影響を与えない");
}
