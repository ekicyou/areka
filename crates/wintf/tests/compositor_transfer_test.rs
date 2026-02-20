//! Task 5.3: transfer_to_hbitmap 転送テスト
//!
//! - 実 D2D1 ステージングビットマップからの転送を検証
//! - pitch == stride 時の一括コピーを検証
//! - Map/Unmap サイクルが正常に動作することを検証
//!
//! 注意: transfer_to_hbitmap は実 COM リソースが必要なため、
//! GraphicsCore::new() → DeviceContext → Bitmap 作成 → CopyFromBitmap → transfer
//! の統合的なテストとなる。

use wintf::com::ulw::transfer_to_hbitmap;
use wintf::ecs::GraphicsCore;
use wintf::ecs::compositor::WindowD3D11Compositor;

// ==========================================================================
// transfer_to_hbitmap: 実 COM リソースを使った転送テスト
// ==========================================================================

#[test]
fn test_transfer_to_hbitmap_basic() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let width = 64u32;
    let height = 64u32;

    // WindowD3D11Compositor を作成してリソースを取得
    let compositor = WindowD3D11Compositor::new(dc, width, height).expect("Compositor 作成失敗");

    let comp_bmp = compositor.composition_bitmap().expect("composition_bitmap");
    let staging = compositor.staging_bitmap().expect("staging_bitmap");
    let dib_bits = compositor.dib_bits().expect("dib_bits");

    // composition_bitmap → staging_bitmap にコピー
    let copy_result = unsafe { staging.CopyFromBitmap(None, comp_bmp, None) };
    assert!(
        copy_result.is_ok(),
        "CopyFromBitmap 成功: {:?}",
        copy_result.err()
    );

    // staging → dib_bits にピクセル転送
    let result = unsafe { transfer_to_hbitmap(staging, dib_bits, width, height) };
    assert!(
        result.is_ok(),
        "transfer_to_hbitmap 成功: {:?}",
        result.err()
    );

    eprintln!("✅ transfer_to_hbitmap が正常に完了した ({width}×{height})");
}

#[test]
fn test_transfer_to_hbitmap_various_sizes() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    // 異なるサイズでのテスト: pitch と stride の不一致を誘発するサイズも含む
    let sizes = [(1, 1), (3, 3), (7, 5), (64, 64), (100, 50), (255, 1)];

    for (width, height) in sizes {
        let compositor = WindowD3D11Compositor::new(dc, width, height)
            .unwrap_or_else(|e| panic!("Compositor 作成失敗 ({width}×{height}): {e}"));

        let comp_bmp = compositor.composition_bitmap().unwrap();
        let staging = compositor.staging_bitmap().unwrap();
        let dib_bits = compositor.dib_bits().unwrap();

        unsafe { staging.CopyFromBitmap(None, comp_bmp, None) }
            .unwrap_or_else(|e| panic!("CopyFromBitmap 失敗 ({width}×{height}): {e}"));

        unsafe { transfer_to_hbitmap(staging, dib_bits, width, height) }
            .unwrap_or_else(|e| panic!("transfer_to_hbitmap 失敗 ({width}×{height}): {e}"));

        eprintln!("  ✅ {width}×{height} 転送成功");
    }

    eprintln!("✅ 全サイズで transfer_to_hbitmap が正常に完了した");
}

// ==========================================================================
// CopyFromBitmap → transfer の完全パイプラインテスト
// ==========================================================================

#[test]
fn test_transfer_pipeline_after_resize() {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("DeviceContext 取得失敗");

    let mut compositor = WindowD3D11Compositor::new(dc, 32, 32).expect("Compositor 作成失敗");

    // 転送テスト（初期サイズ）
    {
        let comp_bmp = compositor.composition_bitmap().unwrap();
        let staging = compositor.staging_bitmap().unwrap();
        let dib_bits = compositor.dib_bits().unwrap();
        unsafe { staging.CopyFromBitmap(None, comp_bmp, None) }.unwrap();
        unsafe { transfer_to_hbitmap(staging, dib_bits, 32, 32) }.unwrap();
    }

    // リサイズ後の転送テスト
    compositor.resize(dc, 128, 96).expect("resize 失敗");
    {
        let comp_bmp = compositor.composition_bitmap().unwrap();
        let staging = compositor.staging_bitmap().unwrap();
        let dib_bits = compositor.dib_bits().unwrap();
        unsafe { staging.CopyFromBitmap(None, comp_bmp, None) }.unwrap();
        unsafe { transfer_to_hbitmap(staging, dib_bits, 128, 96) }.unwrap();
    }

    eprintln!("✅ resize 後も transfer パイプラインが正常に動作する");
}
