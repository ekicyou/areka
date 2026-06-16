//! com/d3d11.rs — D3D11 ラッパーのテスト
//!
//! 実デバイス生成は既存テストの前例（GraphicsCore::new() を用いる
//! tests/graphics 配下の多数のテストがヘッドレスで合格）に従う。
//! 生成パラメータは src/ecs/graphics/core.rs::create_device_3d を踏襲する。

use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use wintf::com::d3d11::*;

fn create_device() -> ID3D11Device {
    d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        None,
        None,
    )
    .expect("d3d11_create_device")
}

fn texture_desc(width: u32, height: u32) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    }
}

// ============================================================
// d3d11_create_device
// ============================================================

#[test]
fn create_device_without_out_params_succeeds() {
    let _device = create_device();
}

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

// ============================================================
// D3D11DeviceExt
// ============================================================

#[test]
fn get_device_removed_reason_is_ok_on_healthy_device() {
    let device = create_device();
    device
        .get_device_removed_reason()
        .expect("healthy device should report S_OK");
}

#[test]
fn create_texture2d_succeeds_and_roundtrips_desc() {
    let device = create_device();
    let texture = device
        .create_texture2d(&texture_desc(16, 16), None)
        .expect("create_texture2d");

    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
    assert_eq!(desc.Width, 16);
    assert_eq!(desc.Height, 16);
    assert_eq!(desc.Format, DXGI_FORMAT_B8G8R8A8_UNORM);
}

/// pinitialdata Some 経路（Option → 生ポインタ変換）の検証。
/// IMMUTABLE テクスチャは初期データ必須のためこの経路を確実に通る。
#[test]
fn create_texture2d_with_initial_data_succeeds() {
    let device = create_device();

    let pixels = vec![0xFFu8; 16 * 16 * 4];
    let initial = D3D11_SUBRESOURCE_DATA {
        pSysMem: pixels.as_ptr() as *const _,
        SysMemPitch: 16 * 4,
        SysMemSlicePitch: 0,
    };
    let desc = D3D11_TEXTURE2D_DESC {
        Usage: D3D11_USAGE_IMMUTABLE,
        ..texture_desc(16, 16)
    };

    device
        .create_texture2d(&desc, Some(&initial))
        .expect("create_texture2d with initial data");
}

/// 不正な記述子（幅 0）はエラーとして伝播する。
/// `texture2d.unwrap()` は成功時のみ到達するため panic しない。
#[test]
fn create_texture2d_with_invalid_desc_fails() {
    let device = create_device();
    let result = device.create_texture2d(&texture_desc(0, 0), None);
    assert!(result.is_err(), "zero-sized texture should fail");
}
