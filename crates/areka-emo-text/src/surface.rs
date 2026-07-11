//! # surface — 自前 swapchain 供給面（COM 層）
//!
//! `TextSurface`（自前 swapchain・text_slot Visual への brush 装着・提示・決定論検証用
//! readback）を担う。装着後のグリフ更新は Present のみで完結し、バルーン surface 本体の
//! 再合成（emo-compose 再駆動）を強要しない。
//!
//! **層規律**: COM 層——UI スレッド専有。`windows`（DXGI/WUC）を触るのは
//! 本モジュールと draw のみ。失敗は log-first（`tracing::error!`＋`Err`）で扱い panic しない。
//!
//! # 構造（emo-present `SwapChainPresenter`（`pub(crate)`）と同型・wintf pub ヘルパから lift）
//!
//! - 供給面 = `create_composition_swap_chain`（flip model・premultiplied・B8G8R8A8・
//!   BufferCount=2）＋ `CompositorInteropExt::create_composition_surface_for_swap_chain`。
//! - **描画面はダブルバッファ** `sources: [_; 2]`＋`front: usize`（ともに D3D11 DEFAULT・
//!   RENDER_TARGET・生成時に全 0（premultiplied 透明）で初期化）。front＝最新確定面・back＝
//!   次フレームの描き先。面内 blit（front→back の whole-pixel 平行移動）＋`flip`（役割交換）で
//!   確定ピクセルを保持したままスクロールを表現する（viewbox 方式・R1.2）。
//! - 提示・readback は常に **front** を対象にする（`CopyResource(backbuffer, front)`→`Present(0)`／
//!   `CopyResource(staging, front)`→`Map(READ)`）。flip model の backbuffer は直接 Map 不可のため
//!   front を読む——記憶 gpu-draw-verification-offscreen-d2d-target。

use bevy_ecs::prelude::*;

use windows::UI::Composition::{Compositor, SpriteVisual, Visual as WucVisual};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{DXGI_PRESENT, IDXGISwapChain1};
use windows::Win32::System::WinRT::Composition::ICompositorInterop;
use windows::core::Interface;
use windows_numerics::Vector2;

use wintf::com::dxgi::create_composition_swap_chain;
use wintf::com::wuc::CompositorInteropExt;
use wintf::ecs::{Arrangement, GraphicsCore, LayoutScale, Offset, Size, VisualGraphics};

use crate::TextLayerError;
use crate::actor::TextSlotBinding;

/// `windows_core::Error` を [`TextLayerError::Device`]（ログ＋`HRESULT`＋文脈）へ写像する
/// クロージャ（log-first: `error!`＋`Err` 戻り値・panic 禁止）。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> TextLayerError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "D3D/DXGI/WUC 呼び出しが失敗");
        TextLayerError::Device { hresult, context }
    }
}

/// `Option` が `None`（本来到達しない成功時 None・デバイス未初期化・entity 不在）を
/// [`TextLayerError::Device`] にする（log-first）。
fn none_err(context: &'static str) -> TextLayerError {
    tracing::error!(context, "必須リソースが欠落（デバイス未初期化 または 前提 entity 不在）");
    TextLayerError::Device { hresult: 0, context }
}

/// DEFAULT usage の B8G8R8A8 描画面テクスチャ（`sources` の 1 枚・D2D ターゲット兼）を
/// **全 0（premultiplied 透明）初期化**で作る——装着直後の読み戻し＝空を決定論化する
/// （D3D11 DEFAULT テクスチャの初期内容は未定義のため初期データで確定させる）。
/// front/back の 2 枚とも本関数で生成し、面寸は attach 時に確定する（寸変更 API を持たない）。
fn create_transparent_source_tex(
    d3d: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, TextLayerError> {
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
    let zeros = vec![0u8; (width * height * 4) as usize];
    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: zeros.as_ptr() as *const _,
        SysMemPitch: width * 4,
        SysMemSlicePitch: 0,
    };
    let mut tex: Option<ID3D11Texture2D> = None;
    unsafe { d3d.CreateTexture2D(&desc, Some(&init), Some(&mut tex)) }
        .map_err(device_err("CreateTexture2D(source_tex)"))?;
    tex.ok_or_else(|| none_err("CreateTexture2D(source_tex) returned None"))
}

/// CPU_READ の staging テクスチャを作る（readback 用）。
fn create_staging(
    d3d: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, TextLayerError> {
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

/// 自前 swapchain 供給面＋text_slot への brush 装着＋readback（R9.1/R9.3）。
///
/// [`Self::attach`] が actor ごと**初回のみ**予約スロットへ brush として装着し、以降の
/// グリフ更新は [`Self::present`]（供給面の提示のみ・World 不要）で完結する＝バルーン
/// surface 本体の再合成（emo-compose 再駆動）を強要しない。決定論検証は
/// [`Self::read_back`]（`source_tex`→staging→bytes）。
///
/// slot の可視性・寿命は emo-present（`VisualMount`）の領分——本層は brush の中身だけを
/// 所有する。UI スレッド専有。
pub struct TextSurface {
    /// 合成供給面（flip model・premultiplied・`ICompositionSurface` が包む本体）。
    swapchain: IDXGISwapChain1,
    /// 描画面ダブルバッファ（2 枚とも DEFAULT・B8G8R8A8・RENDER_TARGET・`size` と同寸・
    /// premultiplied 透明初期化）。`front` が最新確定面・`1 - front` が次フレームの描き先（back）。
    /// 面寸は attach 時確定で変更 API を持たない（無限成長の構造的排除・R1.3/R4.1）。
    sources: [ID3D11Texture2D; 2],
    /// `sources` の front インデックス（初期 0・`flip` で 0↔1 交換）。present/read_back は front を読む。
    front: usize,
    /// readback 用 staging（CPU_READ・`size` と同寸・1 枚のまま——常に front を copy する）。
    staging: ID3D11Texture2D,
    /// immediate context（転送 API の発行先・`source_tex` と同一デバイス）。
    context: ID3D11DeviceContext,
    /// slot へ装着した SpriteVisual（brush 保持者・外形追随（将来の resize 点）の操作口）。
    /// COM 参照の生存は ECS 側 `VisualGraphics` と共有だが、装着物の所有を本型で明示する。
    #[allow(dead_code)]
    sprite: SpriteVisual,
    /// 現在の供給面サイズ（物理 px＝`ceil(validrect 寸 × k)`・論理 px 不在）。
    size: (u32, u32),
}

impl TextSurface {
    /// 初回装着（UI スレッド・`&mut World`）: 自前 swapchain 供給面を生成し、
    /// `binding.slot`（予約スロット entity）へ `VisualGraphics::new(sprite)`（自前
    /// `SurfaceBrush` 装着済み SpriteVisual）＋`Arrangement`（物理 px 直接・
    /// offset＝validrect 原点×k）を insert する（mount.rs の donor パターンの写し）。
    ///
    /// `GraphicsCommandList` は挿入しない＝wintf の widget 描画経路は発火せず、
    /// バルーン surface 本体の再合成を要求しない（R9.3 構造）。
    ///
    /// - `physical_size`: `ceil(validrect 寸 × k)`（`ScaleContract::physical_extent` の導出値）
    /// - `physical_offset`: validrect 原点 × k（窓クライアント原点基準の物理 px）
    pub fn attach(
        world: &mut World,
        binding: &TextSlotBinding,
        compositor: &Compositor,
        core: &GraphicsCore,
        physical_size: (u32, u32),
        physical_offset: (f32, f32),
    ) -> Result<TextSurface, TextLayerError> {
        let (w, h) = physical_size;
        let d3d = core.d3d().ok_or_else(|| none_err("GraphicsCore::d3d"))?.clone();
        let dxgi = core
            .dxgi()
            .ok_or_else(|| none_err("GraphicsCore::dxgi"))?
            .clone();

        // 供給面の生成（swap chain interop は wintf pub ヘルパへ隔離済み・安全 wrapper のみ触る）。
        let swapchain = create_composition_swap_chain(&d3d, &dxgi, w, h)
            .map_err(device_err("create_composition_swap_chain"))?;
        let interop: ICompositorInterop = compositor
            .cast()
            .map_err(device_err("Compositor->ICompositorInterop cast"))?;
        let comp_surface = interop
            .create_composition_surface_for_swap_chain(&swapchain)
            .map_err(device_err("create_composition_surface_for_swap_chain"))?;

        // 描画面ダブルバッファ（front/back 2 枚とも透明初期化・面寸は attach 時確定）。
        let sources = [
            create_transparent_source_tex(&d3d, w, h)?,
            create_transparent_source_tex(&d3d, w, h)?,
        ];
        let staging = create_staging(&d3d, w, h)?;
        let context =
            unsafe { d3d.GetImmediateContext() }.map_err(device_err("GetImmediateContext"))?;

        // SpriteVisual ＋ SurfaceBrush（mount.rs と同一パターン・SetSize は物理 px）。
        let sprite = compositor
            .CreateSpriteVisual()
            .map_err(device_err("CreateSpriteVisual"))?;
        let brush = compositor
            .CreateSurfaceBrushWithSurface(&comp_surface)
            .map_err(device_err("CreateSurfaceBrushWithSurface"))?;
        sprite
            .SetSize(Vector2 {
                X: w as f32,
                Y: h as f32,
            })
            .map_err(device_err("SpriteVisual::SetSize"))?;
        sprite
            .SetBrush(&brush)
            .map_err(device_err("SpriteVisual::SetBrush"))?;
        let wuc_visual: WucVisual = sprite
            .cast()
            .map_err(device_err("SpriteVisual->Visual cast"))?;

        // 予約スロット entity へ donor 装着（有効な VisualGraphics を渡すことで wintf
        // `Visual` フックの既定値上書き・`deferred_surface_creation_system` と競合しない。
        // `GraphicsCommandList` は挿入しない）。
        let Ok(mut slot) = world.get_entity_mut(binding.slot) else {
            tracing::error!(
                slot = ?binding.slot,
                "text_slot entity が World に存在しない（装着不能・binding が古い可能性）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "text_slot entity not found",
            });
        };
        slot.insert((
            VisualGraphics::new(wuc_visual),
            Arrangement {
                offset: Offset {
                    x: physical_offset.0,
                    y: physical_offset.1,
                },
                scale: LayoutScale::default(),
                size: Size {
                    width: w as f32,
                    height: h as f32,
                },
            },
        ));
        // insert フックの遅延コマンドを確定させる（mount.rs と同じ規律）。
        world.flush();

        Ok(TextSurface {
            swapchain,
            sources,
            front: 0,
            staging,
            context,
            sprite,
            size: physical_size,
        })
    }

    /// 描画済み内容の提示（`CopyResource(backbuffer, front)`→`Present(0)` のみ・
    /// World 不要）。装着後のグリフ更新はこれで完結し、バルーン surface 本体の再合成
    /// （emo-compose 再駆動）を強要しない（R9.3）。転送元は常に front（最新確定面）。
    ///
    /// 失敗は `error!`＋`Err`——当該フレームの提示を skip し次フレーム再試行（design
    /// Error Handling）。
    pub fn present(&mut self) -> Result<(), TextLayerError> {
        let src_res: ID3D11Resource = self.sources[self.front]
            .cast()
            .map_err(device_err("front->Resource cast"))?;

        // backbuffer 参照は tight scope でのみ保持し即解放する（将来の ResizeBuffers 前提）。
        {
            let backbuffer: ID3D11Texture2D =
                unsafe { self.swapchain.GetBuffer(0) }.map_err(device_err("GetBuffer(0)"))?;
            let back_res: ID3D11Resource = backbuffer
                .cast()
                .map_err(device_err("backbuffer->Resource cast"))?;
            unsafe { self.context.CopyResource(&back_res, &src_res) };
        }

        unsafe { self.swapchain.Present(0, DXGI_PRESENT(0)) }
            .ok()
            .map_err(device_err("Present(0)"))?;

        Ok(())
    }

    /// 決定論検証用 readback（`CopyResource(staging, front)`→`Map(READ)`→
    /// `stride = width*4` の密 BGRA 配列）。
    ///
    /// flip model の backbuffer は直接 Map 不可のため、最新確定面 front を読む
    /// （表示内容＝`present` の転送元と同一・記憶 gpu-draw-verification-offscreen-d2d-target）。
    pub fn read_back(&self) -> Result<Vec<u8>, TextLayerError> {
        let (w, h) = self.size;
        let src_res: ID3D11Resource = self.sources[self.front]
            .cast()
            .map_err(device_err("front->Resource cast"))?;
        let staging_res: ID3D11Resource = self
            .staging
            .cast()
            .map_err(device_err("staging->Resource cast"))?;

        unsafe { self.context.CopyResource(&staging_res, &src_res) };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context
                .Map(&staging_res, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(device_err("Map(READ)"))?;

        let stride = (w * 4) as usize;
        let row_pitch = mapped.RowPitch as usize;

        // RowPitch は stride 以上（GPU が行を余分にパディングし得る）。逆は想定外＝デバイス異常。
        if row_pitch < stride {
            unsafe { self.context.Unmap(&staging_res, 0) };
            tracing::error!(row_pitch, stride, "RowPitch < stride（想定外）");
            return Err(TextLayerError::Device {
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
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// front 面（レガシー本番描画の描き先・task 5 でオラクルが `front_tex` へ移行するまでの
    /// 移行アクセサ）への crate 内アクセス。
    ///
    /// draw.rs 本番 DrawExecutor が `CreateBitmapFromDxgiSurface` で D2D ターゲット bitmap を
    /// 巻いて全域再描画の描き先にする（draw.rs は本アクセサを無改変で使い続ける）。生成寸
    /// （物理 px）・premultiplied 透明初期化・RENDER_TARGET bind の契約は本型が所有する。
    pub(crate) fn source_tex(&self) -> &ID3D11Texture2D {
        &self.sources[self.front]
    }

    /// back 面（`1 - front`）への crate 内アクセス——後続 ViewboxExecutor（task 6）が
    /// D2D ターゲット bitmap を巻く本番描画の書き込み先。front（確定面）を読み口に残したまま
    /// back へ描き、`flip` で交換する（面内 blit ＋ ダーティ描画の描き先・R1.2）。
    /// task 6 の配線までは未使用のため dead_code を許容する（前方シーム）。
    #[allow(dead_code)]
    pub(crate) fn back_tex(&self) -> &ID3D11Texture2D {
        &self.sources[1 - self.front]
    }

    /// front 面への読み出し口（比較専用オラクル用・task 5）。`source_tex`（本番描画の描き先）
    /// と実体は同じ front だが、オラクルが読む「テスト限定の読み出し口」として命名を分ける。
    /// task 5 のオラクル配線までは未使用のため dead_code を許容する（前方シーム）。
    #[cfg(test)]
    #[allow(dead_code)]
    fn front_tex(&self) -> &ID3D11Texture2D {
        &self.sources[self.front]
    }

    /// 保持ピクセルの面内平行移動: front の残存域を `blit` ぶんずらして back へ写す（R1.2）。
    ///
    /// - `blit == (0,0)`: `CopyResource(back, front)`（全面複製）。
    /// - `blit == (dx,dy) != (0,0)`: `CopySubresourceRegion` で front の残存域を back へずらして写す。
    ///   面寸 `(W,H)` のとき front の px (x,y) が back の (x+dx,y+dy) へ移るよう、dst 原点＝
    ///   `(max(0,dx), max(0,dy))`・src box＝`{ left:max(0,-dx), top:max(0,-dy),
    ///   right:min(W,W-dx), bottom:min(H,H-dy), front:0, back:1 }` を発行する。box 退化
    ///   （`right ≤ left` または `bottom ≤ top`＝全域スクロールアウト）なら copy を発行しない。
    ///
    /// **src=front と dst=back は別テクスチャ**ゆえ重なり未定義動作はない（ダブルバッファの
    /// 存在理由）。back の露出帯（非コピー域）は呼び手（ViewboxExecutor）がダーティ描画で埋める
    /// 契約——本メソッドは平行移動のみを担う。immediate context（`self.context`）で発行する。
    /// task 6 の ViewboxExecutor 配線までは本番非経路（テストのみ使用）のため dead_code を許容する。
    #[allow(dead_code)]
    pub(crate) fn copy_front_to_back_shifted(&mut self, blit: (i32, i32)) {
        let front = &self.sources[self.front];
        let back = &self.sources[1 - self.front];
        // SAFETY: front/back は attach 済み・同一デバイス・同寸・別テクスチャの ID3D11Texture2D。
        // `cast::<ID3D11Resource>` は同一 COM オブジェクトの別インターフェース取得で失敗しない
        // （失敗は `expect` で致命——本来到達しない・記憶 areka-log-first-no-silent-failure に照らし
        // ここは COM 契約違反＝致命扱い）。CopyResource/CopySubresourceRegion は immediate context
        // への発行のみで、box は面寸内へクランプ済み（範囲外アクセスなし）。
        let (dx, dy) = blit;
        if dx == 0 && dy == 0 {
            let back_res: ID3D11Resource = back.cast().expect("back->Resource cast");
            let front_res: ID3D11Resource = front.cast().expect("front->Resource cast");
            unsafe { self.context.CopyResource(&back_res, &front_res) };
            return;
        }

        let (w, h) = (self.size.0 as i32, self.size.1 as i32);
        // dst 原点＝ずらし先の正側・src box＝残存域（ずらしの逆側と面寸で挟んだ矩形）。
        let dstx = dx.max(0);
        let dsty = dy.max(0);
        let left = (-dx).max(0);
        let top = (-dy).max(0);
        let right = w.min(w - dx);
        let bottom = h.min(h - dy);
        // box 退化＝全域スクロールアウト（写る残存域なし）——copy を発行しない（back は呼び手が埋める）。
        if right <= left || bottom <= top {
            return;
        }
        let srcbox = D3D11_BOX {
            left: left as u32,
            top: top as u32,
            front: 0,
            right: right as u32,
            bottom: bottom as u32,
            back: 1,
        };
        let back_res: ID3D11Resource = back.cast().expect("back->Resource cast");
        let front_res: ID3D11Resource = front.cast().expect("front->Resource cast");
        unsafe {
            self.context.CopySubresourceRegion(
                &back_res,
                0,
                dstx as u32,
                dsty as u32,
                0,
                &front_res,
                0,
                Some(&srcbox as *const _),
            );
        }
    }

    /// front/back の役割交換（コピーなし・EndDraw 成功後に呼ぶ）。以降 present/read_back は
    /// 新しい front（直前の back＝最新確定面）を読む。
    /// task 6 の ViewboxExecutor 配線までは本番非経路（テストのみ使用）のため dead_code を許容する。
    #[allow(dead_code)]
    pub(crate) fn flip(&mut self) {
        self.front = 1 - self.front;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::TextSlotBinding;

    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::name::Name;

    use windows::UI::Composition::Compositor;
    use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};

    use wintf::com::wuc::create_dispatcher_queue_controller;
    use wintf::ecs::{Arrangement, GraphicsCommandList, GraphicsCore, Visual, VisualGraphics};

    /// テスト用 WUC apartment / dispatcher（emo-present chain.rs / mount.rs テストと同一方針）。
    ///
    /// cargo test の各テストは専用スレッドで走り COM 未初期化ゆえ、ASTA を第一候補・失敗時
    /// NONE を保険とする（記憶 areka-wuc-runs-on-mta-thread: 未初期化スレッドでは ASTA が通る）。
    /// controller は Compositor より長寿命を要するため呼び出し側で保持する。
    fn make_dispatcher_and_compositor()
    -> (windows::System::DispatcherQueueController, Compositor) {
        let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e_asta| {
                create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
            })
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        let compositor = Compositor::new().expect("Compositor::new 失敗");
        (dq, compositor)
    }

    /// emo-present `VisualMount` と同型の予約スロット（`Name("emo-text-layer-slot")`＋
    /// `Visual` のみ・窓の子・内容なし）を World に組む。返り値は (window, slot)。
    fn spawn_reserved_slot(world: &mut World) -> (Entity, Entity) {
        let window = world.spawn_empty().id();
        let slot = world
            .spawn((
                Name::new("emo-text-layer-slot"),
                Visual::default(),
                ChildOf(window),
            ))
            .id();
        world.flush();
        (window, slot)
    }

    /// タスク 5 観測完了状態の檻:
    /// 供給面を装着したスロットに対して提示と読み戻しが行え、読み戻したバイト列から
    /// 装着直後は空（透明＝全 0）であることを確認できる（R9.1/R9.3）。
    ///
    /// 併せて装着の構造契約を assert する:
    /// - slot entity へ有効な `VisualGraphics`（emo 自前 brush 装着済み SpriteVisual）が入る
    /// - `Arrangement` は物理 px 直接（寸＝physical_size・offset＝physical_offset）
    /// - `GraphicsCommandList` は挿入しない（wintf widget 描画経路を発火させない＝
    ///   バルーン surface 本体の再合成を要求しない構造）
    #[test]
    fn attach_then_present_and_readback_is_transparent() {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        let binding = TextSlotBinding::new(slot, window, 1.0, (10, 8));

        // 物理寸＝ceil(validrect 寸 × k)・offset＝validrect 原点 × k は呼び手（ScaleContract）の
        // 導出結果を受け取る契約——ここでは validrect (2,1)-(6,4) 相当の値を与える。
        let mut surface =
            TextSurface::attach(&mut world, &binding, &compositor, &core, (4, 3), (2.0, 1.0))
                .expect("TextSurface::attach 失敗");

        // --- 装着の構造契約 ---
        let vg = world
            .get::<VisualGraphics>(slot)
            .expect("slot に VisualGraphics（emo 自前 brush）が装着される");
        assert!(vg.is_valid(), "装着された VisualGraphics は有効（brush 装着済み）");
        let arr = world
            .get::<Arrangement>(slot)
            .expect("slot に Arrangement（物理 px 直接）が装着される");
        assert_eq!(
            (arr.size.width, arr.size.height),
            (4.0, 3.0),
            "Arrangement 寸は物理寸そのまま（論理 px 不在）"
        );
        assert_eq!(
            (arr.offset.x, arr.offset.y),
            (2.0, 1.0),
            "Arrangement offset は validrect 原点 × k（物理 px）"
        );
        assert!(
            world.get::<GraphicsCommandList>(slot).is_none(),
            "GraphicsCommandList は挿入しない（wintf 描画系と競合しない・R9.3 構造）"
        );

        // --- 装着直後の読み戻し＝空（透明） ---
        let bytes = surface.read_back().expect("read_back（装着直後）失敗");
        assert_eq!(bytes.len(), (4 * 3 * 4) as usize, "BGRA 密配列（stride=w*4）");
        assert!(
            bytes.iter().all(|&b| b == 0),
            "装着直後の供給面は空（premultiplied 透明＝全 0）"
        );

        // --- 提示は World 不要で完結し、内容は変わらない（グリフ更新＝Present のみの構造） ---
        surface.present().expect("present（装着直後）失敗");
        let bytes = surface.read_back().expect("read_back（present 後）失敗");
        assert!(
            bytes.iter().all(|&b| b == 0),
            "present は提示のみ（source_tex の内容を変えない）"
        );
    }

    /// read_back が実際に供給面（source_tex）を読んでいることの往復檻
    /// （模造の全 0 バッファ返却では通らない）。上流描画（task 6）不在のため、
    /// テスト専用に UpdateSubresource で既知パターンを source_tex へ直接書き、
    /// present→read_back が全バイト一致で返すことを確認する（chain.rs 往復檻と同轍）。
    #[test]
    fn readback_roundtrips_source_tex_contents() {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        let binding = TextSlotBinding::new(slot, window, 1.0, (10, 8));

        let (w, h) = (3u32, 2u32);
        let mut surface =
            TextSurface::attach(&mut world, &binding, &compositor, &core, (w, h), (0.0, 0.0))
                .expect("TextSurface::attach 失敗");

        // 既知の非退化パターン（premultiplied 整合: 成分 ≤ α）を source_tex へ直接書く。
        let mut pattern = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let a = 0xFFu8;
                pattern.push((x as u8).wrapping_mul(40)); // B
                pattern.push((y as u8).wrapping_mul(80)); // G
                pattern.push(((x + y) as u8).wrapping_mul(30)); // R
                pattern.push(a);
            }
        }
        assert!(pattern.iter().any(|&b| b != 0), "パターンは非退化");

        let src_res: windows::Win32::Graphics::Direct3D11::ID3D11Resource = surface.sources
            [surface.front]
            .cast()
            .expect("front->Resource cast");
        unsafe {
            surface.context.UpdateSubresource(
                &src_res,
                0,
                None,
                pattern.as_ptr() as *const _,
                w * 4,
                0,
            );
        }

        surface.present().expect("present 失敗");
        let bytes = surface.read_back().expect("read_back 失敗");
        assert_eq!(bytes, pattern, "read_back は source_tex の内容を全バイト一致で返す");
    }

    /// log-first: slot entity が World に存在しない場合、attach は panic せず
    /// `TextLayerError::Device`（error! 済み）を返す。
    #[test]
    fn attach_fails_without_panic_when_slot_entity_missing() {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

        let mut world = World::new();
        let (window, slot) = spawn_reserved_slot(&mut world);
        world.despawn(slot);
        let binding = TextSlotBinding::new(slot, window, 1.0, (10, 8));

        let err =
            TextSurface::attach(&mut world, &binding, &compositor, &core, (4, 3), (0.0, 0.0))
                .err()
                .expect("slot 不在で attach は Err を返す（panic しない）");
        assert!(
            matches!(err, TextLayerError::Device { .. }),
            "slot 不在は Device エラー（log-first）: {err:?}"
        );
    }

    /// 既知の非退化パターン（premultiplied 整合: 各成分 ≤ α）を w×h で作る。
    /// B=f(x)・G=f(y)・R=f(x+y) で 1 px ごとに識別性を持たせ、blit の off-by-one を殺す。
    fn premul_pattern(w: u32, h: u32) -> Vec<u8> {
        let mut p = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let a = 0xFFu8; // α=255 ゆえ premultiplied 制約（各成分 ≤ α）は自明に成立。
                p.push((x as u8).wrapping_mul(17).wrapping_add(3)); // B
                p.push((y as u8).wrapping_mul(23).wrapping_add(5)); // G
                p.push(((x + y) as u8).wrapping_mul(13).wrapping_add(7)); // R
                p.push(a);
            }
        }
        p
    }

    /// `copy_front_to_back_shifted(blit)` の期待値: front の px (x,y) が back の (x+dx,y+dy) へ
    /// 移り、写域外（露出帯）は透明（全 0）に残る——`CopySubresourceRegion` の面内平行移動の
    /// 純関数モデル（byte 決定論の期待側）。
    fn expected_shift(pattern: &[u8], w: i32, h: i32, dx: i32, dy: i32) -> Vec<u8> {
        let mut out = vec![0u8; (w * h * 4) as usize];
        for yy in 0..h {
            for xx in 0..w {
                let sx = xx - dx;
                let sy = yy - dy;
                if sx >= 0 && sx < w && sy >= 0 && sy < h {
                    let di = ((yy * w + xx) * 4) as usize;
                    let si = ((sy * w + sx) * 4) as usize;
                    out[di..di + 4].copy_from_slice(&pattern[si..si + 4]);
                }
            }
        }
        out
    }

    /// タスク 4.1 往復檻（観測可能な完了状態）:
    /// front へ既知パターンを直書き→`copy_front_to_back_shifted(blit)`→`flip()`→`read_back()` で、
    /// ずらし量 ゼロ・正・負（(0,0)・(0,+N)・(0,-N)・(+N,0)・(-N,0)）それぞれについて
    /// 元の内容が期待位置に byte 一致で現れ、露出域（非コピー域）が透明（全 0）であることを確認する
    /// （R1.2/R1.3——面内 whole-pixel blit ＋ ダブルバッファ flip の決定論）。
    ///
    /// 小さめ面寸（8×6）で端の off-by-one を検出できる形にし、各ケースを新規 surface で独立させて
    /// back の透明初期化（attach 時の全 0）を担保することで byte 決定論を保つ。
    #[test]
    fn copy_front_to_back_shifted_roundtrips_whole_pixel_blit() {
        let (_dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

        let (w, h) = (8u32, 6u32);
        // ずらし量: ゼロ・各軸の正負（面外へ全スクロールしない小さな N＝露出帯が必ず生じる）。
        for &(dx, dy) in &[(0i32, 0i32), (0, 3), (0, -3), (3, 0), (-3, 0)] {
            let mut world = World::new();
            let (window, slot) = spawn_reserved_slot(&mut world);
            let binding = TextSlotBinding::new(slot, window, 1.0, (10, 8));
            let mut surface =
                TextSurface::attach(&mut world, &binding, &compositor, &core, (w, h), (0.0, 0.0))
                    .expect("TextSurface::attach 失敗");

            // front（sources[front]・attach 直後は 0）へ既知パターンを直書きする。
            let pattern = premul_pattern(w, h);
            let src_res: ID3D11Resource =
                surface.source_tex().cast().expect("front->Resource cast");
            unsafe {
                surface.context.UpdateSubresource(
                    &src_res,
                    0,
                    None,
                    pattern.as_ptr() as *const _,
                    w * 4,
                    0,
                );
            }

            // front の残存域を blit ぶんずらして back（透明）へ写し、front/back を交換する。
            surface.copy_front_to_back_shifted((dx, dy));
            surface.flip();

            // flip 後の front（＝写し先 back）を読み戻し、期待位置に元内容・露出域は透明を確認。
            let bytes = surface.read_back().expect("read_back 失敗");
            let expected = expected_shift(&pattern, w as i32, h as i32, dx, dy);
            assert_eq!(
                bytes, expected,
                "blit ({dx},{dy}) の往復: 元内容が期待位置に byte 一致・露出域は透明"
            );
        }
    }
}
