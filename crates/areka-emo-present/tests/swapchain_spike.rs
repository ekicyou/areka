//! Task 1.3 spike（実証・GO ゲート）: swap chain 供給面の生成→アップロード→リサイズ→readback 往復。
//!
//! 本タスク 3.1 の `SwapChainPresenter` を形式化する前に、その往復（R8 の実体）を
//! **統合テスト**として先行実装し GO を確認する（design.md §「Integration Tests」spike #1・
//! ギャップ分析 §4 Option D の指示）。
//!
//! 検証内容（受け入れ基準）:
//! 1. wintf 1.2 ヘルパで合成 swap chain 供給面を生成（自前所有・読み戻し可能・R8.1）。
//!    加えて単一の真実源 `source_tex`（D3D11 DEFAULT・同寸・B8G8R8A8）と readback 用
//!    staging（CPU_READ）を生成する。
//! 2. 既知バイトパターン（premultiplied BGRA・stride = width*4）を `source_tex` へ
//!    `UpdateSubresource`→`CopyResource(backbuffer, source_tex)`→`Present(0)`。
//! 3. readback: `CopyResource(staging, source_tex)`→`Map(READ)`→`RowPitch ≥ stride` を考慮し
//!    密な `stride=width*4` バッファへ行単位コピー（R8.3）。
//! 4. アップロードした全画素と readback の完全バイト一致を `assert_eq!`（R6.7 シームの檻）。
//! 5. `ResizeBuffers(2, new_w, new_h, B8G8R8A8, 0)`（backbuffer 参照を解放後に呼ぶ・R8.5）→
//!    `source_tex`/staging を新寸で再作成→新パターンで再 upload→再 readback→再度一致。
//!
//! WARP でなく本番同様 HARDWARE デバイス（`wintf::ecs::GraphicsCore::new`）を使用する。
//! これはリポジトリ既存の wintf 1.2 テスト（com/dxgi.rs・com/wuc.rs）と一致する
//! （それらも `GraphicsCore::new` = HARDWARE を使う）。CONCERNS 参照。
//!
//! `unsafe`（UpdateSubresource/CopyResource/Map/Present/ResizeBuffers）は spike ゆえ本テストに置く
//! （task 3.1 が SwapChainPresenter へ形式化する）。swap chain *生成* interop は wintf 1.2 ヘルパへ
//! 隔離済み（本テストは安全 wrapper のみを触る）。

use wintf::com::dxgi::create_composition_swap_chain;
use wintf::com::wuc::{CompositorInteropExt, create_dispatcher_queue_controller};
use wintf::ecs::GraphicsCore;

use windows::UI::Composition::{Compositor, ICompositionSurface};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Device, ID3D11Resource,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, DXGI_SWAP_CHAIN_FLAG, IDXGISwapChain1};
use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
use windows::core::Interface;

/// テスト用の WUC apartment / dispatcher を組む（wintf 1.2 テストと同一方針）。
///
/// cargo test の各テストは専用スレッドで走り COM 未初期化ゆえ、design.md §2.1
/// 「未初期化なら DQTAT_COM_ASTA」に従い ASTA を第一候補、失敗時 NONE を保険とする。
/// controller は Compositor より長寿命であることを要するため呼び出し側で保持する。
fn make_dispatcher_and_compositor() -> (windows::System::DispatcherQueueController, Compositor) {
    let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
        .or_else(|e_asta| create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta))
        .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
    let compositor = Compositor::new().expect("Compositor::new 失敗");
    (dq, compositor)
}

/// 既知の premultiplied BGRA パターンを width*height 分生成する（stride = width*4）。
///
/// 各画素を座標由来の決定論値にし、リサイズ後のパターンと区別できるよう `salt` を混ぜる。
/// premultiplied 制約（各成分 ≤ α）を満たすため α を最大にして成分をそれ以下へ丸める。
fn make_pattern(width: u32, height: u32, salt: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let a: u8 = 0xFF;
            // 成分は座標＋salt から決定論的に作り、premultiplied 不変（≤ α=0xFF）を自明に満たす。
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            buf.push(b);
            buf.push(g);
            buf.push(r);
            buf.push(a);
        }
    }
    buf
}

/// DEFAULT usage の B8G8R8A8 `source_tex`（単一の真実源）を作る。
fn create_source_tex(d3d: &ID3D11Device, width: u32, height: u32) -> ID3D11Texture2D {
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
        .expect("source_tex（DEFAULT）生成失敗");
    tex.expect("source_tex が None")
}

/// CPU_READ の staging テクスチャを作る（readback 用）。
fn create_staging(d3d: &ID3D11Device, width: u32, height: u32) -> ID3D11Texture2D {
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
        .expect("staging（CPU_READ）生成失敗");
    tex.expect("staging が None")
}

/// 既知パターンを source_tex へ UpdateSubresource し、backbuffer へ CopyResource → Present(0)。
///
/// backbuffer 参照は転送中のみ取得しスコープで解放する（ResizeBuffers 前提・R8.5 規約）。
fn upload(
    d3d: &ID3D11Device,
    swapchain: &IDXGISwapChain1,
    source_tex: &ID3D11Texture2D,
    width: u32,
    pattern: &[u8],
) {
    let ctx = unsafe { d3d.GetImmediateContext() }.expect("ImmediateContext 取得失敗");
    let stride = width * 4;

    // ① UpdateSubresource(source_tex, bytes, stride) — 単一の真実源へ書込。
    let src_res: ID3D11Resource = source_tex.cast().expect("source_tex→Resource cast 失敗");
    unsafe {
        ctx.UpdateSubresource(&src_res, 0, None, pattern.as_ptr() as *const _, stride, 0);
    }

    // ② CopyResource(backbuffer, source_tex) — backbuffer 参照はこのスコープ内のみ保持。
    {
        let backbuffer: ID3D11Texture2D =
            unsafe { swapchain.GetBuffer(0) }.expect("GetBuffer(0) 失敗");
        let back_res: ID3D11Resource = backbuffer.cast().expect("backbuffer→Resource cast 失敗");
        unsafe { ctx.CopyResource(&back_res, &src_res) };
    } // backbuffer をここで drop（ResizeBuffers 前提）。

    // ③ Present(0)。
    unsafe { swapchain.Present(0, DXGI_PRESENT(0)) }
        .ok()
        .expect("Present(0) 失敗");
}

/// staging 経由で source_tex を CPU 読み戻し、RowPitch ≥ stride を考慮し密配列へ詰める（R8.3）。
fn read_back(
    d3d: &ID3D11Device,
    source_tex: &ID3D11Texture2D,
    staging: &ID3D11Texture2D,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let ctx = unsafe { d3d.GetImmediateContext() }.expect("ImmediateContext 取得失敗");
    let src_res: ID3D11Resource = source_tex.cast().expect("source_tex→Resource cast 失敗");
    let staging_res: ID3D11Resource = staging.cast().expect("staging→Resource cast 失敗");

    // flip model backbuffer は直接 Map 不可のため source_tex を読む（design §SwapChainPresenter）。
    unsafe { ctx.CopyResource(&staging_res, &src_res) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { ctx.Map(&staging_res, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
        .expect("Map(READ) 失敗");

    let stride = (width * 4) as usize;
    let row_pitch = mapped.RowPitch as usize;
    assert!(
        row_pitch >= stride,
        "RowPitch({}) < stride({}) — 想定外",
        row_pitch,
        stride
    );

    let mut dense = vec![0u8; stride * height as usize];
    unsafe {
        let base = mapped.pData as *const u8;
        for y in 0..height as usize {
            let src_row = base.add(y * row_pitch);
            let dst_row = dense.as_mut_ptr().add(y * stride);
            std::ptr::copy_nonoverlapping(src_row, dst_row, stride);
        }
        ctx.Unmap(&staging_res, 0);
    }
    dense
}

/// spike 本体: 生成→upload→readback 一致→ResizeBuffers→再作成→再 upload→再一致。
#[test]
fn swapchain_supply_roundtrip_and_resize() {
    // (1) WUC apartment / Compositor（供給面が SpriteVisual/brush へ装着できることの前提）。
    let (_dq, compositor) = make_dispatcher_and_compositor();
    let interop = compositor
        .cast::<windows::Win32::System::WinRT::Composition::ICompositorInterop>()
        .expect("ICompositorInterop へ cast 失敗");

    // GraphicsCore（本番同様 HARDWARE）から d3d/dxgi を取得。
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
    let d3d = core.d3d().expect("d3d が None").clone();
    let dxgi = core.dxgi().expect("dxgi が None").clone();

    // ---- 第1ラウンド: 初期寸で往復 ----
    let (w0, h0) = (64u32, 48u32);
    let swapchain = create_composition_swap_chain(&d3d, &dxgi, w0, h0)
        .expect("create_composition_swap_chain 失敗");

    // 供給面が WUC ICompositionSurface へ包めること（R8.1・装着材料）を確認。
    let surface: ICompositionSurface = interop
        .create_composition_surface_for_swap_chain(&swapchain)
        .expect("create_composition_surface_for_swap_chain 失敗");
    let _typed: &ICompositionSurface = &surface;

    let mut source_tex = create_source_tex(&d3d, w0, h0);
    let mut staging = create_staging(&d3d, w0, h0);

    let pattern0 = make_pattern(w0, h0, 0x11);
    upload(&d3d, &swapchain, &source_tex, w0, &pattern0);
    let readback0 = read_back(&d3d, &source_tex, &staging, w0, h0);
    assert_eq!(
        readback0, pattern0,
        "第1ラウンド readback がアップロードパターンと全画素一致しない（{}x{}）",
        w0, h0
    );

    // ---- リサイズ: backbuffer 参照は upload 内スコープで解放済み ----
    let (w1, h1) = (100u32, 72u32);
    unsafe {
        swapchain.ResizeBuffers(
            2,
            w1,
            h1,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_SWAP_CHAIN_FLAG(0),
        )
    }
    .expect("ResizeBuffers 失敗（backbuffer 未解放なら DXGI_ERROR_INVALID_CALL）");

    // source_tex / staging を新寸で再作成。brush/ICompositionSurface は swap chain 本体を
    // 包むため作り直し不要（design §SwapChainPresenter・R8.5）。
    source_tex = create_source_tex(&d3d, w1, h1);
    staging = create_staging(&d3d, w1, h1);

    // ---- 第2ラウンド: 新寸・新パターンで再往復 ----
    let pattern1 = make_pattern(w1, h1, 0xA5);
    upload(&d3d, &swapchain, &source_tex, w1, &pattern1);
    let readback1 = read_back(&d3d, &source_tex, &staging, w1, h1);
    assert_eq!(
        readback1, pattern1,
        "第2ラウンド（リサイズ後）readback がアップロードパターンと全画素一致しない（{}x{}）",
        w1, h1
    );

    // 供給面が引き続き有効（swap chain 本体を包むため ResizeBuffers 後も再作成不要）であることを確認。
    let _still: &ICompositionSurface = &surface;
}
