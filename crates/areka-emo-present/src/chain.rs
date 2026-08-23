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

/// `upload` が失敗し得る 7 点（分類: 外形変更 3・資源取得 3・提示 1）。
///
/// テストビルドでのみ実体を持つ注入点 [`fault_point`] の引数であり、通常ビルドでは「何もしない空の
/// 処理」の識別子としてのみ現れる（実行時の分岐・確保・呼出は増えない＝要件 5.5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UploadFault {
    // ── 外形変更（寸法変更）────────────────────────────────────
    /// 新寸 `source_tex` の作成直前。
    CreateSourceTex,
    /// 新寸 staging の作成直前。
    CreateStaging,
    /// `ResizeBuffers` の直前。
    ResizeBuffers,
    // ── 資源取得 ───────────────────────────────────────────────
    /// `source_tex`→`ID3D11Resource` cast の直前。
    SourceTexCast,
    /// `GetBuffer(0)` の直前。
    GetBuffer,
    /// backbuffer→`ID3D11Resource` cast の直前。
    BackbufferCast,
    // ── 提示 ───────────────────────────────────────────────────
    /// `Present(0)` の直前。
    Present,
}

// 次に踏む一致点で 1 回だけ失敗させる旗（スレッド局所・テストビルド限定）。
#[cfg(test)]
thread_local! {
    static ARMED_UPLOAD_FAULT: std::cell::Cell<Option<UploadFault>> =
        const { std::cell::Cell::new(None) };
}

/// 注入された失敗の文脈文字列（`device_err` は `&'static str` を要するため変位ごとに定数を持つ。
/// 字面は `<injected:{at:?}>`＝変位名の Debug 表現と一致する）。
#[cfg(test)]
fn injected_context(at: UploadFault) -> &'static str {
    match at {
        UploadFault::CreateSourceTex => "<injected:CreateSourceTex>",
        UploadFault::CreateStaging => "<injected:CreateStaging>",
        UploadFault::ResizeBuffers => "<injected:ResizeBuffers>",
        UploadFault::SourceTexCast => "<injected:SourceTexCast>",
        UploadFault::GetBuffer => "<injected:GetBuffer>",
        UploadFault::BackbufferCast => "<injected:BackbufferCast>",
        UploadFault::Present => "<injected:Present>",
    }
}

/// 失敗の注入点（テストビルド）。
///
/// 武装中の失敗点と一致したときだけ旗を降ろし、**既存の失敗経路と同じ形**——`device_err` を通す
/// ＝`error!` で記録を残してから [`PresentError::Device`]（`E_FAIL`）を返す——で失敗させる
/// （要件 5.1・5.3）。一致しなければ旗はそのまま（後続の一致点まで武装が残る）。
#[cfg(test)]
fn fault_point(at: UploadFault) -> Result<(), PresentError> {
    if ARMED_UPLOAD_FAULT.with(|armed| armed.get()) != Some(at) {
        return Ok(());
    }
    ARMED_UPLOAD_FAULT.with(|armed| armed.set(None));
    let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8000_4005u32 as i32));
    Err(device_err(injected_context(at))(e))
}

/// 失敗の注入点（通常ビルド）。何もしない空の処理＝常に `Ok(())`（要件 5.5）。
#[cfg(not(test))]
#[inline(always)]
fn fault_point(_at: UploadFault) -> Result<(), PresentError> {
    Ok(())
}

/// テスト専用: 次の一致点で 1 回だけ失敗させる（同一スレッド）。
#[cfg(test)]
#[allow(
    dead_code,
    reason = "消費側は task 5.2/5.3 の注入テスト（本タスクでは注入点の設置まで）"
)]
pub(crate) fn arm_upload_fault(at: UploadFault) {
    ARMED_UPLOAD_FAULT.with(|armed| armed.set(Some(at)));
}

/// テスト専用: 武装を解除する（未消費のまま残った旗を次のテストへ持ち越さない）。
#[cfg(test)]
#[allow(
    dead_code,
    reason = "消費側は task 5.2/5.3 の注入テスト（本タスクでは注入点の設置まで）"
)]
pub(crate) fn clear_upload_fault() {
    ARMED_UPLOAD_FAULT.with(|armed| armed.set(None));
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
    ///
    /// **prepare → commit**（design Flow 3）: 失敗し得る操作（テクスチャ作成・`ResizeBuffers`・
    /// cast・`GetBuffer`）をすべて先に済ませ、内部状態の更新（`source_tex`／`staging`／`size` の
    /// 一括代入）は画素の書き込み＝`UpdateSubresource` の**直前**まで遅らせる。これにより 7 失敗点
    /// のうち `Present` 以外の 6 点では struct の各項目が旧値のまま自己整合し、`read_back()` は
    /// 旧内容・旧寸を返す（要件 5.2・5.7）。成功時の D3D 呼出の集合と回数は並べ替え前と同一で、
    /// 外形不変の定常経路に新しい確保は無い（要件 5.5）。
    ///
    /// 既知の残余 2 件（設計で登記・是正しない）: ⒜ `Present` 失敗＝表示は前フレームのまま・
    /// `source_tex` は未提示の試行内容を持つ／⒝ 外形変更経路で `ResizeBuffers` 成功後の後段失敗＝
    /// struct は旧値で自己整合だが swap chain の表示バッファだけ新寸・未描画（次回 `upload` が
    /// `self.size` 不一致で `ResizeBuffers` を再度通り回復する）。
    pub(crate) fn upload(&mut self, surface: &ComposedSurface) -> Result<(), PresentError> {
        let (w, h) = (surface.width(), surface.height());

        // ── prepare ①: 外形が変われば新寸の資材を**ローカルへ**用意し、内部リサイズを済ませる。
        // backbuffer 参照は本メソッド外に持ち越さず ResizeBuffers より後でのみ取得するため、
        // ResizeBuffers 前に別途解放する必要はない（R8.5 規約充足）。
        let resized = if (w, h) != self.size {
            fault_point(UploadFault::CreateSourceTex)?;
            // source_tex / staging を新寸で再作成。swapchain / ICompositionSurface は本体を包むため
            // 作り直し不要（design §SwapChainPresenter・R8.5）。
            let new_source_tex = create_source_tex(&self.d3d, w, h)?;
            fault_point(UploadFault::CreateStaging)?;
            let new_staging = create_staging(&self.d3d, w, h)?;
            fault_point(UploadFault::ResizeBuffers)?;
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
            Some((new_source_tex, new_staging))
        } else {
            None
        };

        // ── prepare ②: 資源取得。cast 対象は外形変更時は新 source_tex・不変時は現 source_tex。
        fault_point(UploadFault::SourceTexCast)?;
        let src_tex: &ID3D11Texture2D = match &resized {
            Some((new_source_tex, _)) => new_source_tex,
            None => &self.source_tex,
        };
        let src_res: ID3D11Resource = src_tex
            .cast()
            .map_err(device_err("source_tex->Resource cast"))?;

        fault_point(UploadFault::GetBuffer)?;
        let backbuffer: ID3D11Texture2D =
            unsafe { self.swapchain.GetBuffer(0) }.map_err(device_err("GetBuffer(0)"))?;
        fault_point(UploadFault::BackbufferCast)?;
        let back_res: ID3D11Resource = backbuffer
            .cast()
            .map_err(device_err("backbuffer->Resource cast"))?;

        // ── commit: 失敗し得る操作はここまでで全て終えている。外形変更時のみ一括代入する。
        if let Some((new_source_tex, new_staging)) = resized {
            self.source_tex = new_source_tex;
            self.staging = new_staging;
            self.size = (w, h);
        }

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

        // ② CopyResource(backbuffer, source_tex)。
        unsafe { self.context.CopyResource(&back_res, &src_res) };
        // backbuffer 参照はここで解放する（Present より前・次回 ResizeBuffers 前提）。
        drop(back_res);
        drop(backbuffer);

        // ③ Present(0)。
        fault_point(UploadFault::Present)?;
        unsafe { self.swapchain.Present(0, DXGI_PRESENT(0)) }
            .ok()
            .map_err(device_err("Present(0)"))?;

        Ok(())
    }

    /// 直近に `upload` へ渡された内容の CPU 読み戻し（`stride = width*4` の密配列・BGRA）。
    ///
    /// 読む先は単一の真実源 `source_tex` であり、backbuffer の実表示内容は flip model では読み戻せ
    /// ない。よって `Present` が失敗した回については**未提示の試行内容**を返す（残余 ⒜）。
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

/// テストの共有ヘルパ（WUC apartment・既知パターンの本物合成）。`tests` と失敗注入テストの
/// 双方が引く（`structure.md`「テーマ間で共有するヘルパは `<stem>_test_support.rs`」）。
#[cfg(test)]
#[path = "chain_test_support.rs"]
mod test_support;

#[cfg(test)]
mod tests {
    use super::*;

    use wintf::ecs::GraphicsCore;

    use super::test_support::{composed_of_size, make_dispatcher_and_compositor};

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
