//! com/d3d11.rs — D3D11 ラッパーのテスト
//!
//! 実デバイス生成は既存テストの前例（GraphicsCore::new() を用いる
//! tests/graphics 配下の多数のテストがヘッドレスで合格）に従う。
//! 生成パラメータは src/ecs/graphics/core.rs::create_device_3d を踏襲する。

use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use wintf::com::d3d11::*;

// ============================================================
// d3d11_create_device
// ============================================================

/// featurelevel / immediatecontext の出力パラメータ（Option → 生ポインタ
/// 変換ロジック）が双方とも書き込まれることを検証する。
#[test]
fn create_device_fills_optional_out_params() {
    let mut feature_level = D3D_FEATURE_LEVEL::default();
    let mut context: Option<ID3D11DeviceContext> = None;

    let _device = d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        Some(&mut feature_level),
        Some(&mut context),
    )
    .expect("d3d11_create_device with out params");

    assert!(
        feature_level.0 >= D3D_FEATURE_LEVEL_9_1.0,
        "feature level should be written: {:?}",
        feature_level
    );
    assert!(context.is_some(), "immediate context should be written");
}
