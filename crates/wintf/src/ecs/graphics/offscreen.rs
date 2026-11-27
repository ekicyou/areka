//! 画面の外へ描くための WARP の D2D の装置（プロセスで 1 つ）。

use crate::com::d3d11::d3d11_create_device;
use std::sync::OnceLock;
use tracing::error;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;

fn create() -> windows::core::Result<ID2D1Device> {
    let d3d = d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_WARP,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        None,
        None,
    )?;
    let dxgi: IDXGIDevice = d3d.cast()?;
    // SAFETY: Windows API 境界。工場は MULTI_THREADED。
    unsafe {
        let factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None)?;
        factory.CreateDevice(&dxgi)
    }
}

/// WARP の D2D の装置（プロセスで 1 つ・MULTI_THREADED の工場）を返す。作れなければ `error!` を残して `Err`。
pub fn warp_d2d_device() -> windows::core::Result<ID2D1Device> {
    static WARP: OnceLock<windows::core::Result<ID2D1Device>> = OnceLock::new();
    WARP.get_or_init(|| {
        let device = create();
        if let Err(e) = &device {
            error!(error = ?e, "[Offscreen] WARP D2D device creation failed");
        }
        device
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_device_is_shared_per_process() {
        let a = warp_d2d_device().unwrap();
        let b = warp_d2d_device().unwrap();
        assert_eq!(a.as_raw(), b.as_raw());
    }
}
