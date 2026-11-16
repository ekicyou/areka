//! Windows.UI.Composition (WUC) interop 安全 wrapper 群。
//!
//! WUC の interop インターフェイス（`ICompositorInterop` /
//! `ICompositorDesktopInterop` / `ICompositionDrawingSurfaceInterop`）は、
//! IID＋void** out-param を伴う `unsafe` メソッドを露出する。本モジュールは
//! それらを安全な Rust wrapper（`#[inline(always)]`・`Result` 返却・`unsafe` の局所化）へ包む。

use bind_next::Windows::System::DispatcherQueueController;
use bind_next::Windows::UI::Composition::CompositionGraphicsDevice;
use bind_next::Windows::UI::Composition::Desktop::DesktopWindowTarget;
use bind_next::Windows::UI::Composition::ICompositionSurface;
use bind_next::Windows::Win32::IDXGISwapChain1;
use bind_next::Windows::Win32::{
    CreateDispatcherQueueController, DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DQTYPE_THREAD_CURRENT,
    DispatcherQueueOptions,
};
use bind_next::Windows::Win32::{HWND, POINT, RECT};
use bind_next::Windows::Win32::{
    ICompositionDrawingSurfaceInterop, ICompositorDesktopInterop, ICompositorInterop,
};
use bind_next::Windows::Win32::{ID2D1Device, ID2D1DeviceContext, ID2D1DeviceContext3};
use windows_core::{Interface, Result};

/// `ICompositorInterop` の interop メソッドを安全 wrapper 化する Ext trait。
///
/// 既存の D2D デバイスから `CompositionGraphicsDevice` を生成する
/// `CreateGraphicsDevice`（要件 2.1）を包む。
pub trait CompositorInteropExt {
    /// CreateGraphicsDevice — 既存 D2D デバイスから合成グラフィックスデバイスを生成。
    fn create_graphics_device(&self, d2d_device: &ID2D1Device)
    -> Result<CompositionGraphicsDevice>;

    /// CreateCompositionSurfaceForSwapChain — 自前所有 swap chain から合成面を生成（要件 8.1）。
    ///
    /// `com/dxgi::create_composition_swap_chain` が生成した読み戻し可能な供給面を
    /// WUC の `ICompositionSurface`（SpriteVisual＋SurfaceBrush への装着材料）へ包む
    /// 安全 wrapper。`unsafe` は本メソッドへ隔離する（汎用・emo 非依存）。
    fn create_composition_surface_for_swap_chain(
        &self,
        swapchain: &IDXGISwapChain1,
    ) -> Result<ICompositionSurface>;
}

impl CompositorInteropExt for ICompositorInterop {
    #[inline(always)]
    fn create_graphics_device(
        &self,
        d2d_device: &ID2D1Device,
    ) -> Result<CompositionGraphicsDevice> {
        // 相互運用はインターフェース（ICompositionGraphicsDevice）を返すので、クラスへ cast する。
        unsafe { self.CreateGraphicsDevice(d2d_device)?.cast() }
    }

    #[inline(always)]
    fn create_composition_surface_for_swap_chain(
        &self,
        swapchain: &IDXGISwapChain1,
    ) -> Result<ICompositionSurface> {
        unsafe { self.CreateCompositionSurfaceForSwapChain(swapchain) }
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
    // この rev の HWND は裸のポインタの別名なので clippy が咎める。取っ手は OS へ渡すだけで参照しない。
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[inline(always)]
    fn create_desktop_window_target(
        &self,
        hwnd: HWND,
        topmost: bool,
    ) -> Result<DesktopWindowTarget> {
        // 相互運用はインターフェース（IDesktopWindowTarget）を返すので、クラスへ cast する。
        unsafe { self.CreateDesktopWindowTarget(hwnd, topmost)?.cast() }
    }
}

/// `ICompositionDrawingSurfaceInterop` の interop メソッドを安全 wrapper 化する Ext trait。
///
/// D2D 直描き経路（`BeginDraw`→D2D DC 描画→`EndDraw`・要件 6.1/6.2）を包む。
pub trait DrawingSurfaceInteropExt {
    /// BeginDraw — 更新矩形を渡し、`ID2D1DeviceContext3` へ cast した D2D デバイスコンテキストと
    /// atlas offset（描く位置の平行移動）を返す。
    fn begin_draw(&self, update_rect: Option<&RECT>) -> Result<(ID2D1DeviceContext3, POINT)>;
    /// EndDraw — 描画を終了する。
    fn end_draw(&self) -> Result<()>;
}

impl DrawingSurfaceInteropExt for ICompositionDrawingSurfaceInterop {
    #[inline(always)]
    fn begin_draw(&self, update_rect: Option<&RECT>) -> Result<(ID2D1DeviceContext3, POINT)> {
        let update_rect = update_rect.map(|r| r as *const _);
        let mut update_offset = POINT::default();
        let dc: ID2D1DeviceContext = unsafe { self.BeginDraw(update_rect, &mut update_offset)? };
        Ok((dc.cast()?, update_offset))
    }

    #[inline(always)]
    fn end_draw(&self) -> Result<()> {
        // 戻り値の無い COM メソッドは HRESULT を返す。
        unsafe { self.EndDraw().ok() }
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
    // 戻りは IDispatcherQueueController（PDISPATCHERQUEUECONTROLLER）なので、クラスへ cast する。
    unsafe { CreateDispatcherQueueController(options)?.cast() }
}
