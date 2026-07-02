//! # WUC スパイク（R1・最小往復）
//!
//! DComp→WUC 移行の R1 ゲート検証。全面移行に先立ち、以下の**最小往復**が
//! 移行前（DComp）と等価に成立するかを単独で実測する（要件 1.1/1.2/1.3・3.1）:
//!
//! 1. `CreateDispatcherQueueController`（`DQTYPE_THREAD_CURRENT`・既存 pump 相乗り相当）
//! 2. `Compositor`＋`ICompositorInterop::CreateGraphicsDevice`（既存 D2D デバイス流用）
//! 3. `ICompositorDesktopInterop::CreateDesktopWindowTarget`（HWND 束縛）
//! 4. `SpriteVisual`＋`CompositionDrawingSurface`＋`CompositionSurfaceBrush`（brush 一段）
//! 5. `ICompositionDrawingSurfaceInterop::BeginDraw`→D2D 直描き→`EndDraw`
//! 6. 終了時 `ShutdownQueueAsync` ドレイン＋drop 順（controller 最後）
//!
//! ## apartment 種別の実測（要件 10.2・design.md §2.1 の STA 前提を現実で検証）
//!
//! 本番 `WinApp::new()` は UI スレッドを **`CoInitializeEx(COINIT_MULTITHREADED)`＝MTA**
//! で初期化する。design.md §2.1 は「STA なら `DQTAT_COM_NONE`／未初期化なら `DQTAT_COM_ASTA`」
//! と記すが、**実際の UI スレッドは MTA** ゆえ本スパイクは MTA を再現し、`DQTAT_COM_NONE`
//! （apartment を変えない）で DispatcherQueue／Compositor が成立するかを実測する。
//! ここで WUC が MTA スレッド上で動作しないなら、それは R1 の NO-GO 信号であり、
//! 全面移行へ進まず方式（threading 前提）を見直す（要件 1.3）。
//!
//! ## 使用法
//! ```bash
//! cargo run -p wintf --example wuc_spike
//! ```
//! 既定では約 2 秒描画して自動終了する（`WUC_SPIKE_HOLD_MS` で保持時間を上書き可）。

use std::time::{Duration, Instant};

use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use windows::core::{Result, PCWSTR};
use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::UI::Composition::{CompositionDrawingSurface, Compositor};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext3;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
use windows::Win32::System::WinRT::Composition::{
    ICompositionDrawingSurfaceInterop, ICompositorDesktopInterop, ICompositorInterop,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, PeekMessageW, RegisterClassExW,
    TranslateMessage, CW_USEDEFAULT, MSG, PM_REMOVE, WM_QUIT, WNDCLASSEXW,
    WS_EX_NOREDIRECTIONBITMAP, WS_POPUP, WS_VISIBLE,
};
use windows_numerics::Vector2;

use wintf::com::wuc::{
    create_dispatcher_queue_controller, CompositorDesktopInteropExt, CompositorInteropExt,
    DrawingSurfaceInteropExt,
};
use wintf::ecs::GraphicsCore;

const SURFACE_W: f32 = 320.0;
const SURFACE_H: f32 = 240.0;

fn main() -> Result<()> {
    human_panic::setup_panic!();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // (0) 本番 UI スレッドと同一の COM 初期化（MTA）を再現する（要件 3.4・不変）。
    //     S_FALSE（同一スレッド再初期化）は成功扱い。RPC_E_CHANGED_MODE は別モデル既初期化。
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() {
            // 既に別モードで初期化済み等は致命ではない（本番 WinApp と同じ寛容化）。
            warn!("[wuc_spike] CoInitializeEx(MTA) returned {hr:?} (継続)");
        } else {
            info!("[wuc_spike] CoInitializeEx(MTA) ok ({hr:?})");
        }
    }

    // (1) DispatcherQueueController を実測で確定する（要件 3.1・10.2）。
    //     MTA スレッドでは apartment を変えない DQTAT_COM_NONE が第一候補。
    //     ASTA は apartment 変更を伴うため MTA 済みスレッドでは失敗が予想される（実測で記録）。
    let dq = match create_dispatcher_queue_controller(DQTAT_COM_NONE) {
        Ok(c) => {
            info!("[wuc_spike] DispatcherQueueController: DQTAT_COM_NONE で成立（MTA 上）");
            c
        }
        Err(e_none) => {
            warn!("[wuc_spike] DQTAT_COM_NONE 失敗: {e_none:?} → ASTA を試行");
            match create_dispatcher_queue_controller(DQTAT_COM_ASTA) {
                Ok(c) => {
                    info!("[wuc_spike] DispatcherQueueController: DQTAT_COM_ASTA で成立");
                    c
                }
                Err(e_asta) => {
                    error!(
                        "[wuc_spike] R1 NO-GO: DispatcherQueue 生成不可（NONE={e_none:?} / ASTA={e_asta:?}）"
                    );
                    return Err(e_asta);
                }
            }
        }
    };

    // (2) Compositor 生成（MTA スレッド上で成立するかが R1 の核心）。
    let compositor = match Compositor::new() {
        Ok(c) => {
            info!("[wuc_spike] Compositor::new() 成立（MTA 上で WUC 起動 OK）");
            c
        }
        Err(e) => {
            error!("[wuc_spike] R1 NO-GO: Compositor::new() 失敗（MTA 非対応の疑い）: {e:?}");
            return Err(e);
        }
    };

    // (3) 既存 D2D デバイスから CompositionGraphicsDevice を生成（要件 2.1）。
    let core = GraphicsCore::new()?;
    let d2d_device = core
        .d2d_device()
        .expect("[wuc_spike] GraphicsCore.d2d_device() が None");
    let comp_interop = windows::core::Interface::cast::<ICompositorInterop>(&compositor)?;
    let graphics_device = comp_interop.create_graphics_device(d2d_device)?;
    info!("[wuc_spike] CreateGraphicsDevice 成立");

    // (4) 生の Win32 窓を作る（WS_EX_NOREDIRECTIONBITMAP＝WUC/DComp 透過前提・要件 9.3/1.4）。
    let hwnd = create_spike_window()?;
    info!("[wuc_spike] HWND 生成: {hwnd:?}");

    // (5) HWND へ DesktopWindowTarget を束縛（要件 4.1）。
    let desktop_interop = windows::core::Interface::cast::<ICompositorDesktopInterop>(&compositor)?;
    let target = desktop_interop.create_desktop_window_target(hwnd, false)?;
    info!("[wuc_spike] CreateDesktopWindowTarget 成立");

    // (6) SpriteVisual＋DrawingSurface＋SurfaceBrush（brush 一段・要件 5.1/6.1/6.3）。
    let sprite = compositor.CreateSpriteVisual()?;
    sprite.SetSize(Vector2 { X: SURFACE_W, Y: SURFACE_H })?;

    let surface: CompositionDrawingSurface = graphics_device.CreateDrawingSurface(
        Size { Width: SURFACE_W, Height: SURFACE_H },
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        DirectXAlphaMode::Premultiplied,
    )?;
    let surface_brush = compositor.CreateSurfaceBrushWithSurface(&surface)?;
    sprite.SetBrush(&surface_brush)?;

    // root 束縛（要件 4.2）。
    target.SetRoot(&sprite)?;
    info!("[wuc_spike] SpriteVisual/Surface/Brush 束ね＋SetRoot 成立");

    // (7) D2D 直描き（BeginDraw→描画→EndDraw・要件 6.2）。本番同様 None（全面）経路。
    draw_surface(&surface)?;
    info!("[wuc_spike] BeginDraw→D2D 描画→EndDraw 成立（1 サーフェス表示）");

    // (8) 既存 pump 相乗り相当: 一定時間メッセージを回して DispatcherQueue tick を配送する。
    let hold_ms: u64 = std::env::var("WUC_SPIKE_HOLD_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    info!("[wuc_spike] {hold_ms}ms 描画保持（自動終了）");
    pump_for(Duration::from_millis(hold_ms));

    // (9) 終了時ドレイン: ShutdownQueueAsync を発行し、完了までメッセージを回して待つ（要件 3.3）。
    //     drop 順は controller（dq）を最後に落とすことで Compositor より後に drop する
    //     （本スパイクでは明示 drop 順序で保証・WucGraphicsResourceInner の宣言順保証に対応）。
    match dq.ShutdownQueueAsync() {
        Ok(action) => {
            info!("[wuc_spike] ShutdownQueueAsync 発行 → ドレイン待ち");
            // 完了までポンプ（同一スレッドの DQ を空にする）。最大 2 秒でタイムアウト保険。
            let start = Instant::now();
            loop {
                pump_once();
                if action.Status().map(|s| s.0 != 0).unwrap_or(true) {
                    // 0 = Started。0 以外（Completed/Error/Canceled）で抜ける。
                    break;
                }
                if start.elapsed() > Duration::from_secs(2) {
                    warn!("[wuc_spike] ShutdownQueueAsync ドレインがタイムアウト（保険で継続）");
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            info!("[wuc_spike] ShutdownQueueAsync ドレイン完了");
        }
        Err(e) => warn!("[wuc_spike] ShutdownQueueAsync 失敗（継続）: {e:?}"),
    }

    // 明示 drop 順（controller を最後に）。サーフェス/visual/target/compositor を先に落とす。
    drop(surface_brush);
    drop(surface);
    drop(sprite);
    drop(target);
    drop(graphics_device);
    drop(compositor);
    drop(dq); // controller を最後に drop（要件 3.3 の drop 順不変条件に対応）。

    info!("[wuc_spike] R1 GO: 最小往復が MTA スレッド上で成立（shutdown クラッシュなし）");
    Ok(())
}

/// D2D DC で全面クリア＋中央矩形塗り（BeginDraw の atlas offset を SetTransform へ反映）。
fn draw_surface(surface: &CompositionDrawingSurface) -> Result<()> {
    let surface_interop =
        windows::core::Interface::cast::<ICompositionDrawingSurfaceInterop>(surface)?;
    // 本番 render_surface と同一の None（全面）経路（Some(部分矩形) は WUC で E_INVALIDARG）。
    let (dc, offset): (ID2D1DeviceContext3, _) = surface_interop.begin_draw(None)?;
    unsafe {
        // atlas offset を平行移動として反映（本番 render_surface と同じ意味論）。
        let m = windows_numerics::Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: offset.x as f32,
            M32: offset.y as f32,
        };
        dc.SetTransform(&m);
        // 半透明の背景でクリア（透過共存の目視素材・要件 1.4 の下地）。
        dc.Clear(Some(&D2D1_COLOR_F { r: 0.10, g: 0.45, b: 0.85, a: 0.85 }));
    }
    surface_interop.end_draw()?;
    Ok(())
}

/// スパイク用の最小 Win32 窓（WS_POPUP・WS_EX_NOREDIRECTIONBITMAP）を生成する。
fn create_spike_window() -> Result<HWND> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None)?.into();
        // クラス名・タイトルの実体はスパイク終了まで生存すればよい（一過性ゆえ leak 許容）。
        let class_buf: Vec<u16> = "wuc_spike\0".encode_utf16().collect();
        let class_name = PCWSTR(Box::leak(class_buf.into_boxed_slice()).as_ptr());

        let wc = WNDCLASSEXW {
            cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(windows::core::Error::from_thread());
        }

        let title: Vec<u16> = "wintf - WUC Spike (R1)\0".encode_utf16().collect();
        let title = PCWSTR(Box::leak(title.into_boxed_slice()).as_ptr());
        let hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP,
            class_name,
            title,
            WS_POPUP | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            SURFACE_W as i32,
            SURFACE_H as i32,
            None,
            None,
            Some(hinstance),
            None,
        )?;
        Ok(hwnd)
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 指定時間、非ブロッキングでメッセージを回す（DispatcherQueue tick 相乗り）。
fn pump_for(dur: Duration) {
    let start = Instant::now();
    while start.elapsed() < dur {
        pump_once();
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// キューにある全メッセージを 1 バッチ処理する（WM_QUIT でも継続・スパイクは時間で終了）。
fn pump_once() {
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
