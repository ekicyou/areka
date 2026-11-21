//! DXGI 合成用 swap chain の安全 wrapper（汎用・emo 非依存）。
//!
//! WUC の `ICompositorInterop::CreateCompositionSurfaceForSwapChain` へ供給する
//! 自前所有・読み戻し可能なオフスクリーン供給面を生成する。
//! `unsafe`（`GetParent` / `CreateSwapChainForComposition`）は本ヘルパへ隔離し、
//! 呼び出し側（emo-present）は安全 API のみを触る。

use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice4, IDXGIFactory2, IDXGISwapChain1,
};
use windows_core::Result;

/// 合成用 swap chain（HWND 無し）を生成する汎用ヘルパ。
///
/// `dxgi`（`IDXGIDevice4`）から親 `IDXGIFactory2` を辿り、
/// `CreateSwapChainForComposition` で自前所有・読み戻し可能なオフスクリーン供給面を得る。
/// 記述は emo-present 表示層の固定契約に従い、以下を固定値とする:
///
/// - swap effect: `DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL`（flip model）
/// - alpha mode: `DXGI_ALPHA_MODE_PREMULTIPLIED`
/// - format: `DXGI_FORMAT_B8G8R8A8_UNORM`
/// - `BufferCount = 2` / `SampleDesc = {1, 0}`
/// - usage: `DXGI_USAGE_RENDER_TARGET_OUTPUT`
/// - scaling: `DXGI_SCALING_STRETCH`
#[inline]
pub fn create_composition_swap_chain(
    d3d: &ID3D11Device,
    dxgi: &IDXGIDevice4,
    width: u32,
    height: u32,
) -> Result<IDXGISwapChain1> {
    // device に直接 GetParent すると adapter が返り E_NOINTERFACE になるので、
    // device→GetAdapter→adapter.GetParent(factory) の順に辿る。
    let adapter = unsafe { dxgi.GetAdapter()? };
    let factory: IDXGIFactory2 = unsafe { adapter.GetParent::<IDXGIFactory2>()? };

    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
        Flags: 0,
    };

    unsafe { factory.CreateSwapChainForComposition(d3d, &desc, None) }
}

#[cfg(test)]
mod tests {
    use super::super::dxgi::create_composition_swap_chain;
    use crate::ecs::GraphicsCore;

    /// create_composition_swap_chain が実 GraphicsCore デバイスから合成 swap chain を生成する。
    #[test]
    fn create_composition_swap_chain_from_core() {
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d3d = core.d3d().expect("d3d が None");
        let dxgi = core.dxgi().expect("dxgi が None");

        let swapchain = create_composition_swap_chain(d3d, dxgi, 64, 48)
            .expect("create_composition_swap_chain 失敗");

        use windows::Win32::Graphics::Dxgi::Common::{
            DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
        };
        use windows::Win32::Graphics::Dxgi::{
            DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
        };
        let desc = unsafe { swapchain.GetDesc1() }.expect("GetDesc1 失敗");
        assert_eq!(desc.Width, 64);
        assert_eq!(desc.Height, 48);
        assert_eq!(desc.Format, DXGI_FORMAT_B8G8R8A8_UNORM);
        assert_eq!(desc.AlphaMode, DXGI_ALPHA_MODE_PREMULTIPLIED);
        assert_eq!(desc.SwapEffect, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL);
        assert_eq!(desc.BufferCount, 2);
        assert_eq!(desc.SampleDesc.Count, 1);
        assert_eq!(desc.SampleDesc.Quality, 0);
        assert!(
            (desc.BufferUsage & DXGI_USAGE_RENDER_TARGET_OUTPUT) == DXGI_USAGE_RENDER_TARGET_OUTPUT
        );
    }
}
