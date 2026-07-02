//! Windows.UI.Composition (WUC) interop 安全 wrapper 群。
//!
//! WUC の interop インターフェイス（`ICompositorInterop` /
//! `ICompositorDesktopInterop` / `ICompositionDrawingSurfaceInterop`）は、
//! IID＋void** out-param を伴う `unsafe` メソッドを露出する。本モジュールは
//! `com/dcomp.rs` の Ext trait パターン（`#[inline(always)]`・`Result` 返却・
//! `unsafe` の局所化）に倣い、それらを安全な Rust wrapper へ包む。
//!
//! 特に [`DrawingSurfaceInteropExt::begin_draw`] は移行前の DComp 版
//! `DCompositionSurfaceExt::begin_draw`（撤去済み・task 3.2）と戻り型・cast ロジックを
//! 完全一致させて設計されており、下流 `render_surface` のコードを byte-identical に保つ。

use windows::UI::Composition::CompositionGraphicsDevice;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Direct2D::{ID2D1Device, ID2D1DeviceContext, ID2D1DeviceContext3};
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DQTYPE_THREAD_CURRENT,
    DispatcherQueueOptions,
};
use windows::Win32::System::WinRT::Composition::{
    ICompositionDrawingSurfaceInterop, ICompositorDesktopInterop, ICompositorInterop,
};
use windows::System::DispatcherQueueController;
use windows::core::{Interface, Result};

/// `ICompositorInterop` の interop メソッドを安全 wrapper 化する Ext trait。
///
/// 既存の D2D デバイスから `CompositionGraphicsDevice` を生成する
/// `CreateGraphicsDevice`（要件 2.1）を包む。
pub trait CompositorInteropExt {
    /// CreateGraphicsDevice — 既存 D2D デバイスから合成グラフィックスデバイスを生成。
    fn create_graphics_device(&self, d2d_device: &ID2D1Device)
    -> Result<CompositionGraphicsDevice>;
}

impl CompositorInteropExt for ICompositorInterop {
    #[inline(always)]
    fn create_graphics_device(
        &self,
        d2d_device: &ID2D1Device,
    ) -> Result<CompositionGraphicsDevice> {
        unsafe { self.CreateGraphicsDevice(d2d_device) }
    }
}

/// `ICompositorDesktopInterop` の interop メソッドを安全 wrapper 化する Ext trait。
///
/// HWND に束縛した `DesktopWindowTarget` を生成する
/// `CreateDesktopWindowTarget`（要件 4.1）を包む。
pub trait CompositorDesktopInteropExt {
    /// CreateDesktopWindowTarget — HWND への合成出力先ターゲットを生成。
    fn create_desktop_window_target(
        &self,
        hwnd: HWND,
        topmost: bool,
    ) -> Result<DesktopWindowTarget>;
}

impl CompositorDesktopInteropExt for ICompositorDesktopInterop {
    #[inline(always)]
    fn create_desktop_window_target(
        &self,
        hwnd: HWND,
        topmost: bool,
    ) -> Result<DesktopWindowTarget> {
        unsafe { self.CreateDesktopWindowTarget(hwnd, topmost) }
    }
}

/// `ICompositionDrawingSurfaceInterop` の interop メソッドを安全 wrapper 化する Ext trait。
///
/// D2D 直描き経路（`BeginDraw`→D2D DC 描画→`EndDraw`・要件 6.1/6.2）を包む。
pub trait DrawingSurfaceInteropExt {
    /// BeginDraw — 更新矩形を渡し D2D デバイスコンテキストと atlas offset を得る。
    ///
    /// 戻り型・cast ロジックは移行前の DComp 版 `begin_draw`（撤去済み・task 3.2）と
    /// 完全一致させて設計されており、下流 `render_surface` の差分をゼロに保つ。
    fn begin_draw(&self, update_rect: Option<&RECT>) -> Result<(ID2D1DeviceContext3, POINT)>;
    /// EndDraw — 描画を終了する。
    fn end_draw(&self) -> Result<()>;
}

impl DrawingSurfaceInteropExt for ICompositionDrawingSurfaceInterop {
    #[inline(always)]
    fn begin_draw(&self, update_rect: Option<&RECT>) -> Result<(ID2D1DeviceContext3, POINT)> {
        // com/dcomp.rs L254-259 と byte 一致（cast/戻り型を踏襲）。
        let update_rect = update_rect.map(|r| r as *const _);
        let mut update_offset = POINT::default();
        let dc: ID2D1DeviceContext = unsafe { self.BeginDraw(update_rect, &mut update_offset)? };
        Ok((dc.cast()?, update_offset))
    }

    #[inline(always)]
    fn end_draw(&self) -> Result<()> {
        unsafe { self.EndDraw() }
    }
}

/// DispatcherQueueController を現在スレッド種別（`DQTYPE_THREAD_CURRENT`）で生成する。
///
/// WUC の `Compositor` は同一 UI スレッド上に DispatcherQueue を要求する（要件 3.1）。
/// `apartment` は呼び出し側の COM 状態に応じて選ぶ（COM 初期化済みなら `DQTAT_COM_NONE`、
/// 未初期化なら `DQTAT_COM_ASTA`）。後続タスク 2.1（WucGraphicsResource）とテストで再利用する。
#[inline(always)]
pub fn create_dispatcher_queue_controller(
    apartment: DISPATCHERQUEUE_THREAD_APARTMENTTYPE,
) -> Result<DispatcherQueueController> {
    let options = DispatcherQueueOptions {
        dwSize: core::mem::size_of::<DispatcherQueueOptions>() as u32,
        threadType: DQTYPE_THREAD_CURRENT,
        apartmentType: apartment,
    };
    unsafe { CreateDispatcherQueueController(options) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::GraphicsCore;
    use windows::Foundation::Size;
    use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
    use windows::UI::Composition::{CompositionDrawingSurface, Compositor};
    use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};

    /// BeginDraw wrapper の往復単体テスト（atlas offset 非ゼロケース含む）。
    ///
    /// 実グラフィックスデバイス（D3D11 HARDWARE）と WUC ランタイムを要する統合的テスト。
    /// スレッド上で DispatcherQueue → Compositor → CompositionGraphicsDevice →
    /// CompositionDrawingSurface を組み立て、interop 経由で `begin_draw`/`end_draw` の
    /// 往復が成立することを検証する。
    #[test]
    fn begin_draw_roundtrip() {
        // apartment 種別の実測（要件 10.2）:
        // cargo test の各テストは専用スレッドで走り、そのスレッドは COM 未初期化
        // （CoInitialize/RoInitialize なし）。design.md §2.1「未初期化なら DQTAT_COM_ASTA」に
        // 従い ASTA を第一候補とし、万一失敗したら NONE を試す（本開発機での実測により確定）。
        let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e_asta| {
                // NONE は COM 初期化済みスレッド向け。ASTA が失敗した場合の保険。
                create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
            })
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        // controller は Compositor より長寿命であること（要件 3.3）。テスト末尾まで保持。
        let _dq = dq;

        // (2) Compositor 生成。
        let compositor = Compositor::new().expect("Compositor::new 失敗");

        // (3) GraphicsCore から D2D デバイスを得て CreateGraphicsDevice。
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d2d_device = core.d2d_device().expect("d2d_device が None");

        let interop = compositor
            .cast::<ICompositorInterop>()
            .expect("ICompositorInterop へ cast 失敗");
        let graphics_device = interop
            .create_graphics_device(d2d_device)
            .expect("create_graphics_device 失敗");

        // (4) 描画サーフェス生成（B8G8R8A8・Premultiplied・要件 6.4）。
        //     atlas offset 非ゼロを引き出しやすいよう、やや大きめのサーフェスを用意。
        let size = Size {
            Width: 256.0,
            Height: 256.0,
        };
        let surface: CompositionDrawingSurface = graphics_device
            .CreateDrawingSurface(
                size,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )
            .expect("CreateDrawingSurface 失敗");

        let surface_interop = surface
            .cast::<ICompositionDrawingSurfaceInterop>()
            .expect("ICompositionDrawingSurfaceInterop へ cast 失敗");

        // 主経路（本番 render_surface と同一）: update_rect = None（全面）で BeginDraw。
        // WUC は各 CompositionDrawingSurface を内部 atlas テクスチャへ配置するため、
        // updateoffset には atlas 内配置位置（しばしば非ゼロ）が返る。これが本番コードの
        // 「offset を SetTransform の平行移動へ適用する」ロジックの入力となる。
        //
        // 実測所見（要件 10.2）: WUC の BeginDraw は Some(部分矩形) を渡すと
        // E_INVALIDARG (0x80070057) を返す（本開発機で確認）。本番 render_surface も
        // None のみを渡す（begin_draw(None)）ため、None を主経路の必須 assert とする。
        let (dc, offset) = surface_interop
            .begin_draw(None)
            .expect("begin_draw(None) 失敗");

        // DC が有効な ID2D1DeviceContext3 として取得できることを確認（cast 済み）。
        // 実挙動検証: DC 上で軽い D2D 呼び出し（Transform 読取）が成功することを確かめる。
        let mut transform = windows_numerics::Matrix3x2::default();
        unsafe { dc.GetTransform(&mut transform) };

        // atlas offset: WUC 実装依存だが >= 0 で取得できること（out-param が満たされたこと）を確認。
        // 非ゼロ性は atlas 配置に依存するため厳密 assert しない（設計指示どおり観測に留める）。
        assert!(
            offset.x >= 0 && offset.y >= 0,
            "updateoffset が負値: ({}, {})",
            offset.x,
            offset.y
        );
        // atlas offset の実測値をテスト出力へ残す（非ゼロケースの観測）。
        eprintln!(
            "[begin_draw_roundtrip] atlas updateoffset = ({}, {})",
            offset.x, offset.y
        );

        surface_interop.end_draw().expect("end_draw 失敗");
    }
}
