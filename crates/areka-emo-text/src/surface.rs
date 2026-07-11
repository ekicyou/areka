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
//! - **単一の真実源** `source_tex`（D3D11 DEFAULT・RENDER_TARGET＝task 6 の D2D ターゲット兼）。
//!   生成時に全 0（premultiplied 透明）で初期化＝装着直後は空。
//! - 提示は `CopyResource(backbuffer, source_tex)`→`Present(0)` のみ（World 不要）。
//! - readback は `CopyResource(staging, source_tex)`→`Map(READ)`（flip model の backbuffer は
//!   直接 Map 不可のため source_tex を読む——記憶 gpu-draw-verification-offscreen-d2d-target）。

use bevy_ecs::prelude::*;

use windows::UI::Composition::{Compositor, SpriteVisual, Visual as WucVisual};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
    ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
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

/// DEFAULT usage の B8G8R8A8 `source_tex`（単一の真実源・D2D ターゲット兼）を
/// **全 0（premultiplied 透明）初期化**で作る——装着直後の読み戻し＝空を決定論化する
/// （D3D11 DEFAULT テクスチャの初期内容は未定義のため初期データで確定させる）。
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
    /// 単一の真実源（DEFAULT・B8G8R8A8・RENDER_TARGET＝task 6 の D2D ターゲット兼・`size` と同寸）。
    source_tex: ID3D11Texture2D,
    /// readback 用 staging（CPU_READ・`size` と同寸）。
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

        let source_tex = create_transparent_source_tex(&d3d, w, h)?;
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
            source_tex,
            staging,
            context,
            sprite,
            size: physical_size,
        })
    }

    /// 描画済み内容の提示（`CopyResource(backbuffer, source_tex)`→`Present(0)` のみ・
    /// World 不要）。装着後のグリフ更新はこれで完結し、バルーン surface 本体の再合成
    /// （emo-compose 再駆動）を強要しない（R9.3）。
    ///
    /// 失敗は `error!`＋`Err`——当該フレームの提示を skip し次フレーム再試行（design
    /// Error Handling）。
    pub fn present(&mut self) -> Result<(), TextLayerError> {
        let src_res: ID3D11Resource = self
            .source_tex
            .cast()
            .map_err(device_err("source_tex->Resource cast"))?;

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

    /// 決定論検証用 readback（`CopyResource(staging, source_tex)`→`Map(READ)`→
    /// `stride = width*4` の密 BGRA 配列）。
    ///
    /// flip model の backbuffer は直接 Map 不可のため、単一の真実源 `source_tex` を読む
    /// （表示内容＝`present` の転送元と同一・記憶 gpu-draw-verification-offscreen-d2d-target）。
    pub fn read_back(&self) -> Result<Vec<u8>, TextLayerError> {
        let (w, h) = self.size;
        let src_res: ID3D11Resource = self
            .source_tex
            .cast()
            .map_err(device_err("source_tex->Resource cast"))?;
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

        let src_res: windows::Win32::Graphics::Direct3D11::ID3D11Resource = surface
            .source_tex
            .cast()
            .expect("source_tex->Resource cast");
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
}
