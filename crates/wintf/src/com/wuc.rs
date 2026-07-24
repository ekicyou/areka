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

use std::time::{Duration, Instant};

use tracing::warn;
use windows::UI::Composition::CompositionGraphicsDevice;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::ICompositionSurface;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain1;
use windows::Win32::Graphics::Direct2D::{ID2D1Device, ID2D1DeviceContext, ID2D1DeviceContext3};
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DQTYPE_THREAD_CURRENT,
    DispatcherQueueOptions,
};
use windows::Win32::System::WinRT::Composition::{
    ICompositionDrawingSurfaceInterop, ICompositorDesktopInterop, ICompositorInterop,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};
use windows::System::DispatcherQueueController;
use windows::core::{Interface, Result};

/// [`drain_dispatcher_queue`] のドレインループ 1 反復の停止判定（純関数・決定論テスト対象）。
///
/// `AsyncStatus` の `Started`（`.0 == 0`）は「まだ実行中」。それ以外（Completed/Error/Canceled）は
/// 完了とみなしてループを抜ける。到達不能な短時間 timeout でも `TimedOut` で必ず有界に抜ける。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrainStep {
    /// `ShutdownQueueAsync` が完了（Started 以外）。正常ドレイン成立。
    Completed,
    /// 有界 timeout を超過。log-first で警告し、ハングせず抜ける。
    TimedOut,
    /// まだ Started かつ timeout 未超過。ポンプ継続。
    Continue,
}

/// ドレインループの停止判定（純関数）。`still_started` は `Status() == Started`。
///
/// COM に依存しないため、タイムアウト分岐を含む全経路を決定論的に単体テストできる。
#[inline]
pub(crate) fn drain_step(still_started: bool, elapsed: Duration, timeout: Duration) -> DrainStep {
    if !still_started {
        DrainStep::Completed
    } else if elapsed >= timeout {
        DrainStep::TimedOut
    } else {
        DrainStep::Continue
    }
}

/// `DispatcherQueueController` を明示 shutdown し、完了までメッセージポンプでドレインする。
///
/// `wuc_spike.rs` が実証した「`ShutdownQueueAsync` 発行 → `Status()` ポーリング →
/// タイムアウト保険」パターンの関数化。呼び出しスレッド＝当該 controller の生成スレッド
/// （`DQTYPE_THREAD_CURRENT` 束縛）であることを前提とする。
///
/// - 有界: `timeout` を超えたら log-first（`warn!`）で必ず抜ける（無限待機・ハングを排除）。
/// - panic-free: 失敗・タイムアウトは握り潰さず `warn!` へ可視化するのみで panic しない
///   （`Drop` 中からも安全に呼べる・無音失敗禁止＝memory `areka-log-first-no-silent-failure`）。
///
/// # Returns
/// `ShutdownQueueAsync` が完了まで drain できたら `true`、タイムアウト保険で抜けたら `false`。
pub fn drain_dispatcher_queue(controller: &DispatcherQueueController, timeout: Duration) -> bool {
    let action = match controller.ShutdownQueueAsync() {
        Ok(a) => a,
        Err(e) => {
            warn!("[drain_dispatcher_queue] ShutdownQueueAsync 発行失敗（継続）: {e:?}");
            return false;
        }
    };

    let start = Instant::now();
    loop {
        pump_current_thread_messages();
        // Status() 取得失敗（Err）は「これ以上待てない」＝完了扱いで抜ける（無限待機回避）。
        let still_started = action.Status().map(|s| s.0 == 0).unwrap_or(false);
        match drain_step(still_started, start.elapsed(), timeout) {
            DrainStep::Completed => return true,
            DrainStep::TimedOut => {
                warn!(
                    "[drain_dispatcher_queue] ドレインが timeout（{:?}）を超過。保険で抜ける",
                    timeout
                );
                return false;
            }
            DrainStep::Continue => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

/// 現在スレッドのメッセージキューを 1 バッチ空にする（DispatcherQueue tick の配送）。
///
/// `WM_QUIT` はドレイン中は無視して継続する（`wuc_spike::pump_once` と同型）。
#[inline]
fn pump_current_thread_messages() {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                return;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

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
        unsafe { self.CreateGraphicsDevice(d2d_device) }
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

    /// `drain_step`（ドレインループ停止判定・純関数）の決定論全網羅（要件 4.2/4.3）。
    ///
    /// COM 非依存の判断分岐ゆえ、正常完了・タイムアウト・継続の 3 経路を実 GPU なしで確定検証する。
    #[test]
    fn drain_step_covers_completed_timeout_and_continue() {
        // Started 以外（still_started=false）は timeout 未満でも即 Completed。
        assert_eq!(
            drain_step(false, Duration::from_millis(0), Duration::from_secs(2)),
            DrainStep::Completed
        );
        // まだ Started かつ timeout 未満 → Continue（ポンプ継続）。
        assert_eq!(
            drain_step(true, Duration::from_millis(10), Duration::from_secs(2)),
            DrainStep::Continue
        );
        // まだ Started かつ elapsed >= timeout → TimedOut（保険で抜ける・panic なし）。
        assert_eq!(
            drain_step(true, Duration::from_secs(2), Duration::from_secs(2)),
            DrainStep::TimedOut
        );
        // 到達不能な短時間 timeout（ZERO）でも Started なら即 TimedOut（ハングしない）。
        assert_eq!(
            drain_step(true, Duration::from_nanos(1), Duration::ZERO),
            DrainStep::TimedOut
        );
    }

    /// `drain_dispatcher_queue` の正常ドレイン（実 WUC・要件 4.2）。
    ///
    /// 生成スレッド上で DispatcherQueueController を作り、明示ドレインが panic せず完了することを確認。
    /// controller drop 前の明示 shutdown 手段（回帰檻・Path B の安全再生成プロトコルの正準手段）。
    ///
    /// apartment は本 in-source テスト群（`begin_draw_roundtrip` 等）と同じ ASTA 先行にする。
    /// 理由: `--lib` バイナリ内で唯一の MTA/NONE Compositor テストは `wuc_graphics_resource_lifecycle`
    /// のみであり、ここで 2 個目の MTA/NONE Compositor を別テストスレッドで生成すると根本原因
    /// （同一プロセス・別スレッドの 2 個目 MTA 束縛 Compositor＝AV）を自ら踏む。ASTA は per-thread
    /// 隔離アパートメントゆえ cross-thread でも安全（既存 --lib の ASTA 2 テスト共存で実証済み）。
    #[test]
    fn drain_dispatcher_queue_drains_live_controller_without_panic() {
        let controller = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e| create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e))
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        // Compositor を生成して DispatcherQueue に実体を持たせてからドレイン。
        let _compositor = Compositor::new().expect("Compositor::new 失敗");

        // 正常系: 十分な timeout で完了まで drain できる（true）・panic しない。
        let drained = drain_dispatcher_queue(&controller, Duration::from_secs(2));
        assert!(
            drained,
            "空に近いキューは十分な timeout 内で drain 完了するはず"
        );

        // 冪等性/panic-free: 既に shutdown 済みの controller へ再度呼んでも panic しない
        // （タイムアウト系相当の握り潰し経路も含め、Drop から安全に呼べることの確認）。
        let _second = drain_dispatcher_queue(&controller, Duration::from_millis(50));
    }

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

    /// create_composition_surface_for_swap_chain wrapper の単体テスト。
    ///
    /// 実グラフィックスデバイス（D3D11 HARDWARE）と WUC ランタイムを要する統合的テスト。
    /// `create_composition_swap_chain`（com/dxgi・要件 8.1）で自前所有面を生成し、
    /// `ICompositorInterop::CreateCompositionSurfaceForSwapChain` の安全 wrapper 経由で
    /// `ICompositionSurface` が取得できることを検証する（要件 6.1 の合成面装着の材料）。
    #[test]
    fn create_composition_surface_for_swap_chain_roundtrip() {
        use crate::com::dxgi::create_composition_swap_chain;

        // (1) Compositor 用の DispatcherQueue（begin_draw_roundtrip と同一方針）。
        let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e_asta| {
                create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
            })
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        let _dq = dq;

        // (2) Compositor 生成 → ICompositorInterop へ cast。
        let compositor = Compositor::new().expect("Compositor::new 失敗");
        let interop = compositor
            .cast::<ICompositorInterop>()
            .expect("ICompositorInterop へ cast 失敗");

        // (3) GraphicsCore から d3d/dxgi を得て自前所有の合成 swap chain を生成。
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d3d = core.d3d().expect("d3d が None");
        let dxgi = core.dxgi().expect("dxgi が None");
        let swapchain =
            create_composition_swap_chain(d3d, dxgi, 64, 48).expect("create_composition_swap_chain 失敗");

        // (4) 安全 wrapper 経由で ICompositionSurface を取得（本タスクの検証対象）。
        let surface = interop
            .create_composition_surface_for_swap_chain(&swapchain)
            .expect("create_composition_surface_for_swap_chain 失敗");

        // 取得した ICompositionSurface が有効な COM オブジェクトであることを確認。
        // SpriteVisual への装着（本番 render 経路）へ渡せる ICompositorInterop 由来の面。
        use windows::UI::Composition::ICompositionSurface;
        let _typed: &ICompositionSurface = &surface;
    }
}
