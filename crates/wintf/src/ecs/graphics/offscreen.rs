//! 描画の木を画面の外の絵へ描く口と、そのための WARP の D2D の装置（プロセスで 1 つ）。

use super::composite::SurfaceComposite;
use super::visual::offscreen_tree_commands;
use crate::com::d3d11::d3d11_create_device;
use bevy_ecs::prelude::{Entity, World};
use std::sync::OnceLock;
use tracing::error;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;
use windows_numerics::Matrix3x2;

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

/// 画面の外へ描いた絵（乗算済み BGRA・1 行＝幅×4 バイト）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffscreenImage {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

/// 根 `root` の下の木を描く係と同じ順（世界の節を満たす → 組む → 描く）で、透明に塗った `size` の的へ
/// 描いて乗算済み BGRA で読み戻す。視野は根の `Arrangement.size`、`root_transform` は DIP → 物理。
/// 窓も WUC も要らない（使い捨ての World でよい）。失敗（装置の失いを含む）は `error!` を残して `Err`。
/// 節の世界行列と版を書き換えるので、画面に出している World で呼ぶと描く係が変化を取りこぼす（使い捨ての World で呼ぶ）。
pub fn render_tree_offscreen(
    device: &ID2D1Device,
    world: &mut World,
    root: Entity,
    size: (u32, u32),
    root_transform: Matrix3x2,
) -> windows::core::Result<OffscreenImage> {
    let result = render(device, world, root, size, root_transform);
    if let Err(e) = &result {
        error!(
            root = ?root,
            width = size.0,
            height = size.1,
            error = ?e,
            hresult = format_args!("0x{:08X}", e.code().0),
            "[render_tree_offscreen] 画面の外へ描けない"
        );
    }
    result
}

fn render(
    device: &ID2D1Device,
    world: &mut World,
    root: Entity,
    size: (u32, u32),
    root_transform: Matrix3x2,
) -> windows::core::Result<OffscreenImage> {
    // SAFETY: Windows API 境界。生きている装置から工場と DC を取るだけ。
    let (factory, dc) = unsafe {
        (
            device.GetFactory()?,
            device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?,
        )
    };
    let commands = offscreen_tree_commands(world, root, &factory)?;
    let mut composite = SurfaceComposite::new(&dc, size)?;
    composite.compose(&dc, root_transform, None, &commands)?;
    Ok(OffscreenImage {
        width: size.0,
        height: size.1,
        bytes: composite.read_all(device)?,
    })
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
