//! `SwapChainPresenter`（chain.rs）: 自前所有・読み戻し可能なコンポジター供給面（R8 の実体）。
//!
//! task 1.3 の spike（`tests/swapchain_spike.rs`）で実証済みの往復ロジックを製品コンポーネントへ
//! 形式化する。供給面 = `CreateSwapChainForComposition`（flip model・premultiplied・B8G8R8A8・
//! BufferCount=2）、**単一の真実源** `source_tex`（D3D11 DEFAULT）を所有し、更新は
//! ①`UpdateSubresource(source_tex, bytes, stride)` ②`CopyResource(backbuffer, source_tex)`
//! ③`Present(0)`。readback は `CopyResource(staging, source_tex)`→`Map(READ)`（flip model の
//! backbuffer は直接 Map 不可のため source_tex を読む）。**D2D 非経由の純バイト転送**であり、
//! ピクセル形式変換・サンプリング・ブレンドが介在しない（R1.2 無変換・R6.2/R8.2 golden 決定論）。
//!
//! swap chain *生成* interop（`GetAdapter`→`GetParent`→`CreateSwapChainForComposition` /
//! `CreateCompositionSurfaceForSwapChain`）は wintf 1.2 ヘルパへ隔離済み。本モジュールの `unsafe`
//! は D3D11 転送（UpdateSubresource/CopyResource/Map/Present/ResizeBuffers）に限る（design が
//! SwapChainPresenter に許した範囲）。失敗経路はログ + `PresentError::Device` で表現しパニックしない。

use areka_emo_compose::ComposedSurface;

use windows::UI::Composition::{Compositor, ICompositionSurface};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Device,
    ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, DXGI_SWAP_CHAIN_FLAG, IDXGISwapChain1};
use windows::Win32::System::WinRT::Composition::ICompositorInterop;
use windows::core::Interface;

use wintf::com::dxgi::create_composition_swap_chain;
use wintf::com::wuc::CompositorInteropExt;
use wintf::ecs::GraphicsCore;

use crate::command::PresentError;

/// `windows_core::Error` を [`PresentError::Device`]（ログ＋`HRESULT`＋文脈）へ写像するクロージャ。
///
/// 失敗経路はログ規律（error! + `Err` 戻り値・パニック禁止）に従い、発生箇所の静的文脈を添えて
/// 構造化エラーへ畳む。`.map_err(device_err("<where>"))?` の形で D3D/DXGI 呼び出しを包む。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> PresentError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "D3D/DXGI 呼び出しが失敗");
        PresentError::Device { hresult, context }
    }
}

/// `Option` が `None`（本来到達しない成功時 None・デバイス未初期化）を [`PresentError::Device`] にする。
fn none_err(context: &'static str) -> PresentError {
    tracing::error!(
        context,
        "必須リソースが None（デバイス未初期化 または 成功時 None）"
    );
    PresentError::Device {
        hresult: 0,
        context,
    }
}

/// DEFAULT usage の B8G8R8A8 `source_tex`（単一の真実源）を作る。
fn create_source_tex(
    d3d: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, PresentError> {
    let desc = D3D11_TEXTURE2D_DESC {
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
        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut tex: Option<ID3D11Texture2D> = None;
    unsafe { d3d.CreateTexture2D(&desc, None, Some(&mut tex)) }
        .map_err(device_err("CreateTexture2D(source_tex)"))?;
    tex.ok_or_else(|| none_err("CreateTexture2D(source_tex) returned None"))
}

/// CPU_READ の staging テクスチャを作る（readback 用）。
fn create_staging(
    d3d: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, PresentError> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
    };
    let mut tex: Option<ID3D11Texture2D> = None;
    unsafe { d3d.CreateTexture2D(&desc, None, Some(&mut tex)) }
        .map_err(device_err("CreateTexture2D(staging)"))?;
    tex.ok_or_else(|| none_err("CreateTexture2D(staging) returned None"))
}

/// 自前所有・読み戻し可能なコンポジター供給面（R8 の実体）。
///
/// `pub(crate)`（公開 API ではない）。`ICompositionSurface` を `new` で返し、`VisualMount` が
/// `SurfaceBrush` へ装着する。表示・readback は本型が UI スレッド上で担う。
///
/// 単一の真実源は `source_tex`（D3D11 DEFAULT・B8G8R8A8・同寸）。更新は
/// `UpdateSubresource(source_tex)`→`CopyResource(backbuffer, source_tex)`→`Present(0)`、
/// readback は `CopyResource(staging, source_tex)`→`Map(READ)`。backbuffer は転送中のみ取得し
/// 即解放する（`ResizeBuffers` 前提・R8.5）。`context` は immediate context（`d3d` と同一デバイス）。
pub(crate) struct SwapChainPresenter {
    /// 合成供給面（`ICompositionSurface` が包む本体・`ResizeBuffers` で寸法追随）。
    swapchain: IDXGISwapChain1,
    /// 単一の真実源（DEFAULT usage・B8G8R8A8・現在の `size` と同寸）。
    source_tex: ID3D11Texture2D,
    /// readback 用 staging（CPU_READ・現在の `size` と同寸）。
    staging: ID3D11Texture2D,
    /// D3D11 デバイス（テクスチャ再作成に用いる）。
    d3d: ID3D11Device,
    /// immediate context（転送 API の発行先）。
    context: ID3D11DeviceContext,
    /// 現在の供給面サイズ（物理 px）。
    size: (u32, u32),
}

impl SwapChainPresenter {
    /// 供給面 + `source_tex` + staging を生成し、装着材料 `ICompositionSurface` を併せて返す。
    pub(crate) fn new(
        gfx: &GraphicsCore,
        compositor: &Compositor,
        width: u32,
        height: u32,
    ) -> Result<(Self, ICompositionSurface), PresentError> {
        let d3d = gfx
            .d3d()
            .ok_or_else(|| none_err("GraphicsCore::d3d"))?
            .clone();
        let dxgi = gfx
            .dxgi()
            .ok_or_else(|| none_err("GraphicsCore::dxgi"))?
            .clone();

        // swap chain 生成 interop は wintf 1.2 ヘルパへ隔離済み（安全 wrapper のみ触る）。
        let swapchain = create_composition_swap_chain(&d3d, &dxgi, width, height)
            .map_err(device_err("create_composition_swap_chain"))?;

        // Compositor → ICompositorInterop（装着材料 ICompositionSurface を得るため）。
        let interop: ICompositorInterop = compositor
            .cast()
            .map_err(device_err("Compositor->ICompositorInterop cast"))?;
        let surface = interop
            .create_composition_surface_for_swap_chain(&swapchain)
            .map_err(device_err("create_composition_surface_for_swap_chain"))?;

        let source_tex = create_source_tex(&d3d, width, height)?;
        let staging = create_staging(&d3d, width, height)?;
        let context =
            unsafe { d3d.GetImmediateContext() }.map_err(device_err("GetImmediateContext"))?;

        Ok((
            Self {
                swapchain,
                source_tex,
                staging,
                d3d,
                context,
                size: (width, height),
            },
            surface,
        ))
    }

    /// `ComposedSurface` の内容を供給面へ反映（外形が変われば内部リサイズ）。UI スレッド。
    pub(crate) fn upload(&mut self, surface: &ComposedSurface) -> Result<(), PresentError> {
        let (w, h) = (surface.width(), surface.height());

        // 外形が変われば内部リサイズ。backbuffer 参照は本メソッド外に持ち越さない（下の tight scope
        // でのみ取得・即解放）ため、ResizeBuffers 前に別途解放する必要はない（R8.5 規約充足）。
        if (w, h) != self.size {
            unsafe {
                self.swapchain.ResizeBuffers(
                    2,
                    w,
                    h,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_SWAP_CHAIN_FLAG(0),
                )
            }
            .map_err(device_err("ResizeBuffers"))?;
            // source_tex / staging を新寸で再作成。swapchain / ICompositionSurface は本体を包むため
            // 作り直し不要（design §SwapChainPresenter・R8.5）。
            self.source_tex = create_source_tex(&self.d3d, w, h)?;
            self.staging = create_staging(&self.d3d, w, h)?;
            self.size = (w, h);
        }

        let src_res: ID3D11Resource = self
            .source_tex
            .cast()
            .map_err(device_err("source_tex->Resource cast"))?;

        // ① UpdateSubresource(source_tex, bytes, stride) — 単一の真実源へ書込（無変換バイト転送）。
        unsafe {
            self.context.UpdateSubresource(
                &src_res,
                0,
                None,
                surface.bytes().as_ptr() as *const _,
                surface.stride(),
                0,
            );
        }

        // ② CopyResource(backbuffer, source_tex) — backbuffer 参照はこのスコープ内のみ保持。
        {
            let backbuffer: ID3D11Texture2D =
                unsafe { self.swapchain.GetBuffer(0) }.map_err(device_err("GetBuffer(0)"))?;
            let back_res: ID3D11Resource = backbuffer
                .cast()
                .map_err(device_err("backbuffer->Resource cast"))?;
            unsafe { self.context.CopyResource(&back_res, &src_res) };
        } // backbuffer をここで drop（ResizeBuffers 前提）。

        // ③ Present(0)。
        unsafe { self.swapchain.Present(0, DXGI_PRESENT(0)) }
            .ok()
            .map_err(device_err("Present(0)"))?;

        Ok(())
    }

    /// 表示中画素の CPU 読み戻し（`stride = width*4` の密配列・BGRA）。
    pub(crate) fn read_back(&self) -> Result<Vec<u8>, PresentError> {
        let (w, h) = self.size;
        let src_res: ID3D11Resource = self
            .source_tex
            .cast()
            .map_err(device_err("source_tex->Resource cast"))?;
        let staging_res: ID3D11Resource = self
            .staging
            .cast()
            .map_err(device_err("staging->Resource cast"))?;

        // flip model backbuffer は直接 Map 不可のため source_tex を読む（design §SwapChainPresenter）。
        unsafe { self.context.CopyResource(&staging_res, &src_res) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context
                .Map(&staging_res, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(device_err("Map(READ)"))?;

        let stride = (w * 4) as usize;
        let row_pitch = mapped.RowPitch as usize;

        // R8.3: RowPitch は stride 以上（GPU が行を余分にパディングし得る）。逆は想定外＝デバイス異常。
        if row_pitch < stride {
            unsafe { self.context.Unmap(&staging_res, 0) };
            tracing::error!(row_pitch, stride, "RowPitch < stride（想定外）");
            return Err(PresentError::Device {
                hresult: 0,
                context: "RowPitch < stride",
            });
        }

        let mut dense = vec![0u8; stride * h as usize];
        unsafe {
            let base = mapped.pData as *const u8;
            for y in 0..h as usize {
                let src_row = base.add(y * row_pitch);
                let dst_row = dense.as_mut_ptr().add(y * stride);
                std::ptr::copy_nonoverlapping(src_row, dst_row, stride);
            }
            self.context.Unmap(&staging_res, 0);
        }
        Ok(dense)
    }

    /// 現在の供給面サイズ（物理 px）。
    pub(crate) fn size(&self) -> (u32, u32) {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use areka_emo_atlas::{
        AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState};
    use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
    use std::path::Path;

    use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
    use wintf::com::wuc::create_dispatcher_queue_controller;
    use wintf::ecs::GraphicsCore;

    /// テスト用の WUC apartment / dispatcher を組む（spike / wintf 1.2 テストと同一方針）。
    ///
    /// cargo test の各テストは専用スレッドで走り COM 未初期化ゆえ、design §2.1「未初期化なら ASTA」
    /// に従い ASTA を第一候補・失敗時 NONE を保険とする。controller は Compositor より長寿命を要する
    /// ため呼び出し側で保持する。
    fn make_dispatcher_and_compositor() -> (windows::System::DispatcherQueueController, Compositor)
    {
        let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e_asta| {
                create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
            })
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        let compositor = Compositor::new().expect("Compositor::new 失敗");
        (dq, compositor)
    }

    // ── ComposedSurface 生成補助（cache.rs テストと同技法）─────────────────────
    // `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接焼けない。
    // よって「既知の非退化パターン」は上流公開 API（atlas bake → EmoWorld → Composer::compose）で
    // 本物を合成して得る（模造バッファでの偽陽性を避ける）。

    fn elem(path: &str, x: i64, y: i64) -> Element {
        Element {
            layer: 0,
            path: ElementPath::new(path.to_string()),
            x,
            y,
        }
    }

    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        }
    }

    /// `w×h` の**全不透明・座標由来グラデーション**を単一 element として本物合成する。
    ///
    /// α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h` になる。各画素は
    /// 座標＋`salt` から決定論的に作り（成分 ≤ α=255 で premultiplied 不変を自明に満たす）、
    /// リサイズ前後で異なるパターンとして区別できる。全 0 でない＝非退化を呼び出し側が assert する。
    fn composed_of_size(w: u32, h: u32, salt: u8) -> ComposedSurface {
        let base = Path::new("shell/master");
        let surfaces = vec![surface(1000, vec![elem("p.png", 0, 0)])];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let a: u8 = 0xFF;
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.push(b);
                img.push(g);
                img.push(r);
                img.push(a);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(
            baked.errors.is_empty(),
            "atlas bake セットアップは失敗しない"
        );

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));

        let mut composer = Composer::new();
        composer
            .compose(
                &world,
                &baked.table,
                1000,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("静的 element 単体の合成は Ok")
    }

    /// R8.2 観測完了: upload → read_back が `ComposedSurface.bytes()` と全バイト一致し、
    /// 外形変更時は内部リサイズを経て再度一致する（R1.5/R8.5）。純バイト往復の檻。
    #[test]
    fn upload_read_back_roundtrip_and_resize() {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");

        // ---- 第1ラウンド: 初期寸で往復 ----
        let s0 = composed_of_size(3, 2, 0x11);
        assert!(
            s0.bytes().iter().any(|&b| b != 0),
            "fixture は非退化（全 0 でない）でなければ檻にならない"
        );
        let (mut presenter, _surface) =
            SwapChainPresenter::new(&core, &compositor, s0.width(), s0.height())
                .expect("SwapChainPresenter::new 失敗");
        assert_eq!(presenter.size(), (3, 2), "初期サイズは new の指定寸");

        presenter.upload(&s0).expect("upload(s0) 失敗");
        let rb0 = presenter.read_back().expect("read_back 失敗");
        assert_eq!(
            rb0,
            s0.bytes(),
            "第1ラウンド readback が upload バイトと全画素一致しない（3x2）"
        );

        // ---- 第2ラウンド: 異なる外形（リサイズ経路）で再往復 ----
        let s1 = composed_of_size(5, 4, 0xA5);
        assert!(s1.bytes().iter().any(|&b| b != 0));
        assert_ne!(
            (s1.width(), s1.height()),
            (s0.width(), s0.height()),
            "リサイズ経路を踏むため外形は第1ラウンドと異なること"
        );

        presenter.upload(&s1).expect("upload(s1・リサイズ)失敗");
        assert_eq!(
            presenter.size(),
            (5, 4),
            "size() がリサイズ後の新寸を反映する"
        );
        let rb1 = presenter.read_back().expect("read_back（リサイズ後）失敗");
        assert_eq!(
            rb1,
            s1.bytes(),
            "第2ラウンド（リサイズ後）readback が upload バイトと全画素一致しない（5x4）"
        );
    }
}
