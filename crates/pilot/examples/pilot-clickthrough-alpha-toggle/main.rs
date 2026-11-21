//! 先進坑: pilot-clickthrough-alpha-toggle
//!
//! 一次記録（動機・概要・検証結果）は隣の README.md、T1〜T8 の詳細台帳は隣の REPORT.md。
//!
//! 実行法: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`
//!
//! 視覚的透過は DirectComposition（DComp）visual tree の per-pixel α を前提とする
//! （窓は `WS_EX_NOREDIRECTIONBITMAP` で生成するため GDI/`WM_PAINT` は画面に出ない）。
//! 葉ノード隔離（examples 配下のみ・inbound 依存ゼロ）は厳守する。

use std::cell::Cell;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use event_listener::Event;
use windows::Win32::Foundation::{HMODULE, HWND, LRESULT, POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1_ELLIPSE, D2D1CreateDevice, ID2D1Device, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice3, IDCompositionDesktopDevice, IDCompositionDevice2,
    IDCompositionSurface, IDCompositionTarget, IDCompositionVisual2,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetCursorPos, GetWindowLongPtrW, GetWindowRect,
    IDC_CROSS, LoadCursorW, SW_SHOW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetCursor, SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOW_EX_STYLE,
    WM_CAPTURECHANGED, WM_CLOSE, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_SETCURSOR, WS_EX_LAYERED, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP,
};
use windows::core::{Interface, Result};
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{block_on, spawn_local};

/// 不透明円の半径（物理ピクセル）。
///
/// 描画円（`draw_circle`）と判定円（`alpha_is_opaque`）が共有する。両者を一致させるため、
/// 半径と円中心の算出（クライアント矩形の中心）をここ以外で重複定義しない。
const RADIUS: i32 = 200;

/// α マスク純関数（矩形中心・半径 `RADIUS` の円・物理座標）。
///
/// `cursor` が不透明円の内側（境界を含む）なら `true`（=クリック透過 OFF）、外側なら `false`
/// （=クリック透過 ON）。`win_rect` はクライアントの矩形で、`cursor` と同じ座標系にそろえる
/// （見張り役はスクリーン座標・クリックの受け取りは左上 0 のクライアント座標）。
///
/// 実レンダリング α バッファのサンプリングは行わない（プレースホルダ円・実装は本坑の責務）。
fn alpha_is_opaque(cursor: POINT, win_rect: RECT) -> bool {
    let cx = (win_rect.left + win_rect.right) / 2;
    let cy = (win_rect.top + win_rect.bottom) / 2;

    // i64 で二乗・比較し、マルチモニタ大／負座標での i32 オーバーフローを防ぐ。
    let dx = (cursor.x - cx) as i64;
    let dy = (cursor.y - cy) as i64;
    let r = RADIUS as i64;

    dx * dx + dy * dy <= r * r
}

/// プロセスを Per-Monitor Aware v2（PMv2）に設定する。
///
/// PMv2 では `GetCursorPos`/`GetWindowRect` がともに物理スクリーン座標を返すので、円判定の
/// 座標がそろう。失敗は警告ログに残す。
fn init_dpi_awareness() {
    // SAFETY: `SetProcessDpiAwarenessContext` はプロセスグローバルな
    // DPI awareness をスレッドセーフに設定する。プロセス起動直後・他スレッド／
    // DPI 依存処理の前に一度だけ呼ぶ（main 冒頭）。
    let result =
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    match result {
        Ok(()) => println!("[dpi] SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2): Ok"),
        Err(e) => {
            eprintln!("[dpi][warn] SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2) failed: {e}")
        }
    }
}

/// 不透明円の塗りつぶし色パレット（クリックの色トグルで巡回する）。
///
/// すべて α = 1.0（完全不透明）。トグルは色のみ変え、円の幾何（中心・半径）と
/// クリック透過（`WS_EX_TRANSPARENT`）には干渉しない。
/// premultiplied surface ゆえ α = 1 では RGB はそのまま使える。
const COLORS: [D2D1_COLOR_F; 2] = [
    // インディゴ（初期色）。
    D2D1_COLOR_F {
        r: 0.30,
        g: 0.20,
        b: 0.80,
        a: 1.0,
    },
    // オレンジ（トグル先）。
    D2D1_COLOR_F {
        r: 0.95,
        g: 0.55,
        b: 0.10,
        a: 1.0,
    },
];

/// DComp 視覚パイプラインの COM インターフェース群（UI スレッド専有・`!Send` 許容）。
///
/// ドロップすると合成ツリーが崩壊する（target/visual/device の解放は composition を破棄する）
/// ため、窓が生きている間 `AppState` 内で保持し続ける。
///
/// swapchain でなく `IDCompositionSurface` を visual content に用いる（surface.BeginDraw が
/// D2D デバイスコンテキストを直接返すため、swapchain＋D2D-over-DXGI-surface より配線が単純）。
struct DCompPipeline {
    /// D3D11 デバイス（BGRA 対応・DComp/D2D の土台）。保持専用。
    _d3d: ID3D11Device,
    /// D2D デバイス（DComp の rendering device 兼 surface 描画系列）。保持専用。
    _d2d_device: ID2D1Device,
    /// DComp デスクトップデバイス（CreateTargetForHwnd を持つ）。保持専用。
    _desktop: IDCompositionDesktopDevice,
    /// CreateVisual/CreateSurface/Commit を持つデバイス（描画・Commit で使用）。
    device: IDCompositionDevice2,
    /// 窓に紐づく composition target（SetRoot 済み）。保持専用（ドロップで合成破棄）。
    _target: IDCompositionTarget,
    /// ルートビジュアル（SetContent(surface) 済み）。保持専用。
    _visual: IDCompositionVisual2,
    /// 視覚内容を供給する surface（BeginDraw/EndDraw で D2D 描画）。再描画で使用。
    surface: IDCompositionSurface,
    /// surface のピクセルサイズ（= クライアント矩形の物理サイズ）。円中心算出に使用。
    width: u32,
    height: u32,
}

/// 透過トップモスト窓に紐づく共有状態（`!Send` で良い＝`Rc`／`HWND`／COM を内包する）。
///
/// wndproc が `Fn` ゆえ状態は `Cell`/`RefCell`/atomic で持つ。
struct AppState {
    /// 窓クローズ（`WM_CLOSE`）で notify され、`block_on` の await を完了させる。
    shutdown: Event,
    /// 現在の円色インデックス（`COLORS` への添字）。初期色＝0。
    color_index: Cell<usize>,
    /// DComp 視覚パイプライン。窓セットアップ後に `Some` が入り、以降ドロップしない
    /// （ドロップ＝合成ツリー崩壊）。初期化前は `None`。
    pipeline: std::cell::RefCell<Option<DCompPipeline>>,
    /// StateApplier を起こす wake イベント（ワーカと共有）。ドラッグ終了時に wndproc から
    /// notify して透過状態を再収束させる。
    state_changed: Arc<Event>,
    /// ドラッグ中フラグ（wndproc が設定・StateApplier が参照）。true の間は applier が
    /// `WS_EX_TRANSPARENT` トグルを抑止し、表示位置に関わらずクリック透過 OFF を維持する
    /// （透過 ON へ戻すと SetCapture を失いドラッグが崩壊する）。
    dragging: Arc<AtomicBool>,
    /// ドラッグ開始時の「カーソル − 窓原点」オフセット（物理スクリーン座標）。移動追従に使う。
    drag_anchor: Cell<(i32, i32)>,
}

impl AppState {
    fn new(state_changed: Arc<Event>, dragging: Arc<AtomicBool>) -> Self {
        Self {
            shutdown: Event::new(),
            color_index: Cell::new(0),
            pipeline: std::cell::RefCell::new(None),
            state_changed,
            dragging,
            drag_anchor: Cell::new((0, 0)),
        }
    }

    /// 現在の円色（`COLORS[color_index]`）。
    fn current_color(&self) -> D2D1_COLOR_F {
        COLORS[self.color_index.get()]
    }

    /// 円色インデックスを次へ巡回し、新しい現在色を返す。
    fn toggle_color(&self) -> D2D1_COLOR_F {
        let next = (self.color_index.get() + 1) % COLORS.len();
        self.color_index.set(next);
        COLORS[next]
    }
}

/// NOREDIRECTIONBITMAP・トップモスト・クリック透過の ex_style でトップレベル窓を生成する。
///
/// 起動時の初期状態はクリック透過 ON（`WS_EX_TRANSPARENT` を初期 ex_style に含む）。
/// `WS_EX_NOREDIRECTIONBITMAP` の窓は redirection surface を持たず GDI/`WM_PAINT` が画面に
/// 出ない（DComp 前提）。`WS_EX_TRANSPARENT` の動的トグルは StateApplier が行う。
/// `WM_NCHITTEST` は自前処理しない。
///
/// 【wakagomo 記事】`WS_EX_TRANSPARENT` は単独ではマウス透過が効かず（合成窓では窓がイベントを
/// 吸う）、`WS_EX_LAYERED` をフラグとして併設して初めて効く、との経験則を検証する。
/// `WS_EX_LAYERED` はフラグのみ（`UpdateLayeredWindow`/`SetLayeredWindowAttributes` は呼ばない）。
/// DComp と共存できるか＝描画が消えないか／クリックが透過するかを実測する（結果は REPORT.md）。
///
/// ライブラリの `wndproc_typed` は `WM_CLOSE` で `DestroyWindow` を呼ばず `LRESULT(0)` を返すため、
/// 窓の実破棄は `Window` の `Drop`（`block_on` 復帰後の drop）が担う。
fn make_window(state: Rc<AppState>) -> Window<Rc<AppState>> {
    let ex_style = WINDOW_EX_STYLE(
        WS_EX_NOREDIRECTIONBITMAP.0 | WS_EX_TOPMOST.0 | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0,
    );

    Window::new_ex(
        WindowType::TopLevel,
        ex_style,
        state,
        move |state: Pin<&Rc<AppState>>, msg: WindowMessage| -> Option<LRESULT> {
            let s: &AppState = state.get_ref();
            match msg.msg {
                WM_SETCURSOR => {
                    // 診断用: 十字カーソルにする。この窓は透過 OFF のときだけ WM_SETCURSOR を
                    // 受け取るので、十字が出ている範囲＝反応する領域が一目で分かる。
                    // 返り値 TRUE(=1) で DefWindowProc に既定カーソルへ戻させない。
                    if let Ok(cross) = unsafe { LoadCursorW(None, IDC_CROSS) } {
                        unsafe {
                            let _ = SetCursor(Some(cross));
                        }
                    }
                    Some(LRESULT(1))
                }
                WM_CLOSE => {
                    s.shutdown.notify(usize::MAX);
                    Some(LRESULT(0))
                }
                WM_LBUTTONDOWN => {
                    // 期待では不透明円の内側を踏んだクリックだけがここへ届く（円外は
                    // WS_EX_TRANSPARENT で背面へ抜ける）。「円外なのに受領」が出れば透過漏れ。
                    let lp = msg.lparam.0;
                    let click_x = (lp & 0xFFFF) as i16 as i32;
                    let click_y = ((lp >> 16) & 0xFFFF) as i16 as i32;
                    let mut cr = RECT::default();
                    // SAFETY: read-only。msg.hwnd は有効窓。
                    let _ = unsafe { GetClientRect(msg.hwnd, &mut cr) };
                    // cr は左上 0 のクライアント矩形ゆえ中心は (right/2, bottom/2)＝判定と同じ。
                    let (ccx, ccy) = (cr.right / 2, cr.bottom / 2);
                    let inside = alpha_is_opaque(POINT { x: click_x, y: click_y }, cr);
                    println!(
                        "[click] WM_LBUTTONDOWN 受領 client=({click_x},{click_y}) 円中心=({ccx},{ccy}) → {} → 色トグル＋DComp 再描画",
                        if inside {
                            "円内（正常受領）"
                        } else {
                            "円外（!! WS_EX_TRANSPARENT 透過漏れ !!）"
                        }
                    );

                    // GDI/WM_PAINT/InvalidateRect は使わず DComp 描画パス（draw_circle）で描き直す。
                    let color = s.toggle_color();
                    match s.pipeline.borrow().as_ref() {
                        Some(pipeline) => {
                            if let Err(e) = draw_circle(pipeline, color) {
                                eprintln!("[dcomp][warn] 色トグル後の再描画に失敗: {e}");
                            }
                        }
                        None => eprintln!(
                            "[dcomp][warn] pipeline 未構築のため再描画をスキップ（色のみ反転）"
                        ),
                    }

                    // ドラッグ移動開始: 不透明部を掴んで窓を動かす。ドラッグ中は表示位置に関わらず
                    // WS_EX_TRANSPARENT を外したまま維持する（外れるとマウスキャプチャが切れドラッグが
                    // 崩壊する）。
                    let mut cur = POINT::default();
                    let mut win = RECT::default();
                    // SAFETY: read-only。msg.hwnd は有効窓。
                    let anchored = unsafe {
                        GetCursorPos(&mut cur).is_ok() && GetWindowRect(msg.hwnd, &mut win).is_ok()
                    };
                    if anchored {
                        s.drag_anchor.set((cur.x - win.left, cur.y - win.top));
                        // SAFETY: 窓所有スレッド（UI）から自窓へキャプチャ。
                        unsafe {
                            let _ = SetCapture(msg.hwnd);
                        }
                        s.dragging.store(true, Ordering::Release);
                        println!(
                            "[drag] 開始（SetCapture・dragging=true・以降は位置に関わらず WS_EX_TRANSPARENT を外したまま維持）"
                        );
                    }
                    Some(LRESULT(0))
                }
                WM_MOUSEMOVE => {
                    // ドラッグ中のみ窓を追従移動（DComp 合成内容・判定円も窓と一緒に動く）。
                    if s.dragging.load(Ordering::Acquire) {
                        let mut cur = POINT::default();
                        // SAFETY: read-only。
                        if unsafe { GetCursorPos(&mut cur) }.is_ok() {
                            let (ax, ay) = s.drag_anchor.get();
                            // SAFETY: 窓所有スレッド（UI）から自窓を移動。サイズ/Z順/
                            // アクティブ化は変えない。
                            unsafe {
                                let _ = SetWindowPos(
                                    msg.hwnd,
                                    None,
                                    cur.x - ax,
                                    cur.y - ay,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                                );
                            }
                        }
                        Some(LRESULT(0))
                    } else {
                        None
                    }
                }
                WM_LBUTTONUP | WM_CAPTURECHANGED => {
                    // ドラッグ終了: ボタンを離した／捕まえを失った（Alt+Tab 等で離す合図が他窓へ行く）。
                    // 片付けは印を下ろせたときだけ 1 回（ReleaseCapture が同期で送る
                    // WM_CAPTURECHANGED は印が既に下りているので素通り）。
                    if s.dragging.swap(false, Ordering::AcqRel) {
                        // SAFETY: キャプチャ解除。
                        unsafe {
                            let _ = ReleaseCapture();
                        }
                        s.state_changed.notify(usize::MAX);
                        let why = if msg.msg == WM_LBUTTONUP {
                            "ボタンを離した"
                        } else {
                            "捕まえを失った"
                        };
                        println!(
                            "[drag] 終了（{why}・ReleaseCapture・dragging=false・透過状態を再収束）"
                        );
                        Some(LRESULT(0))
                    } else {
                        None
                    }
                }
                WM_LBUTTONDBLCLK => {
                    // ダブルクリックで終了する（枠なし窓で閉じる UI が無いため）。WM_CLOSE と
                    // 同じ経路。円（不透明部）上でのみ届く。
                    println!("[window] ダブルクリック → shutdown（正常終了）");
                    s.shutdown.notify(usize::MAX);
                    Some(LRESULT(0))
                }
                _ => None,
            }
        },
    )
    .expect("透過トップモスト窓の生成に失敗（Window::new_ex）")
}

/// DComp 視覚パイプラインを構築する。Commit は初回描画の後に行う。
///
/// surface サイズはクライアント矩形（PMv2 ゆえ `GetClientRect` は物理ピクセル・WS_POPUP で client==window）。
/// GDI/`WM_PAINT`/`DwmExtendFrameIntoClientArea` は使わない（NOREDIRECTIONBITMAP 窓では画面に出ない）。
fn build_pipeline(hwnd: HWND) -> Result<DCompPipeline> {
    let mut rect = RECT::default();
    // SAFETY: `hwnd` は直前に生成した有効窓。out 引数 `rect` は書き込み専用。
    // DComp の CreateTargetForHwnd はウィンドウ原点から合成する。窓は WS_POPUP で client==window
    // にしてあるので、surface を client サイズにすれば見える円と判定円が一致する。
    unsafe { GetClientRect(hwnd, &mut rect)? };
    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    // BGRA 対応フラグは D2D 相互運用に必須。
    let mut d3d: Option<ID3D11Device> = None;
    // SAFETY: out 引数 `ppDevice`(Some(&mut d3d)) を非 null で渡すため
    // HRESULT 成功時に必ず書き込まれる（`?` が失敗経路を先行リターン）。他 out は None。
    unsafe {
        D3D11CreateDevice(
            None, // 既定アダプタ。
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None, // 既定のフィーチャレベル列。
            D3D11_SDK_VERSION,
            Some(&mut d3d),
            None, // 取得したフィーチャレベルは不要。
            None, // immediate context は不要（D2D 経由で描く）。
        )?;
    }
    let d3d = d3d.expect("D3D11CreateDevice 成功時は ppDevice が書き込まれる");

    let dxgi: IDXGIDevice = d3d.cast()?;

    // SAFETY: `dxgi` は有効。第2引数 None で既定プロパティ。
    let d2d_device: ID2D1Device = unsafe { D2D1CreateDevice(&dxgi, None)? };

    // rendering device に D2D デバイスを渡す（repo 準拠）。
    // SAFETY: `d2d_device` は有効・IUnknown として渡る。
    let desktop: IDCompositionDesktopDevice = unsafe { DCompositionCreateDevice3(&d2d_device)? };
    // CreateVisual/CreateSurface/Commit を持つ基底デバイス（DesktopDevice は Device2 を継承）。
    let device: IDCompositionDevice2 = desktop.cast()?;

    // SAFETY: 引数はすべて値・既定列挙。out は Result で受ける。
    let surface: IDCompositionSurface = unsafe {
        device.CreateSurface(
            width,
            height,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )?
    };

    // SAFETY: `device`/`surface` は有効。
    let visual: IDCompositionVisual2 = unsafe { device.CreateVisual()? };
    // SAFETY: `surface` は IUnknown として content に設定可能。
    unsafe { visual.SetContent(&surface)? };

    // SAFETY: `hwnd` は有効窓。topmost で最前面合成。
    let target: IDCompositionTarget = unsafe { desktop.CreateTargetForHwnd(hwnd, true)? };
    // SAFETY: `visual` は IDCompositionVisual。
    unsafe { target.SetRoot(&visual)? };

    Ok(DCompPipeline {
        _d3d: d3d,
        _d2d_device: d2d_device,
        _desktop: desktop,
        device,
        _target: target,
        _visual: visual,
        surface,
        width,
        height,
    })
}

/// パイプラインの surface に「透明 Clear ＋ 不透明円」を描画し Commit する。
///
/// 円中心はクライアント中心 `(width/2, height/2)`（物理ピクセル）で、判定円（`alpha_is_opaque`
/// のクライアント矩形の中心）と一致する。`BeginDraw` が返す atlas オフセットは surface 割当原点の
/// 補正で、中心に加算して吸収する（SetTransform/Matrix を介さない＝windows-numerics 非依存）。
fn draw_circle(pipeline: &DCompPipeline, color: D2D1_COLOR_F) -> Result<()> {
    // BeginDraw は描画状態の DC を返す（明示の BeginDraw/EndDraw は surface 側が括る）。
    let mut offset = POINT::default();
    // SAFETY: `updaterect=None` で全面更新。`offset` は out。返る DC は
    // この surface を対象とする描画用コンテキスト。
    let dc: ID2D1DeviceContext = unsafe { pipeline.surface.BeginDraw(None, &mut offset)? };

    // PMv2 ＝物理ピクセル基準で描くため DPI を 96（1 DIP = 1px）に固定。
    // SAFETY: `dc` は有効。
    unsafe { dc.SetDpi(96.0, 96.0) };

    let transparent = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    // SAFETY: `dc` は有効・色は値ポインタ。
    unsafe { dc.Clear(Some(&transparent)) };

    // SAFETY: `color` は値ポインタ。brushproperties=None で既定。
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None)? };

    let cx = offset.x as f32 + (pipeline.width as f32) / 2.0;
    let cy = offset.y as f32 + (pipeline.height as f32) / 2.0;
    // windows-numerics を名指しせず構築するため Default + フィールド代入。
    let mut ellipse = D2D1_ELLIPSE::default();
    ellipse.point.X = cx;
    ellipse.point.Y = cy;
    ellipse.radiusX = RADIUS as f32;
    ellipse.radiusY = RADIUS as f32;
    // SAFETY: `ellipse` は値ポインタ・`brush` は ID2D1Brush。
    unsafe { dc.FillEllipse(&ellipse, &brush) };

    // SAFETY: BeginDraw と対。
    unsafe { pipeline.surface.EndDraw()? };
    // SAFETY: 視覚ツリーの変更を DWM へコミット。
    unsafe { pipeline.device.Commit()? };

    Ok(())
}

/// カーソル監視ワーカを別 `std::thread` で起動する。
///
/// 16ms ごとにカーソルとクライアント矩形をスクリーン座標で読み、円の外＝クリック透過 ON（`true`）
/// を `desired_passthrough` に公開する。desired が前ループから変化したとき（および初回）だけ
/// `state_changed` を notify する。
///
/// UI スレッド固有でない read-only API しか触らないため、`HWND` を生の `isize` で渡して
/// ワーカ内で再構築し、`AppState`（`!Send`）に `unsafe impl Send` を付けずに済ませる。
/// スタイル変更 API（`SetWindowLongPtr`/`SetWindowPos`）は窓所有スレッドの StateApplier が呼ぶ。
fn spawn_cursor_worker(
    hwnd: HWND,
    desired_passthrough: Arc<AtomicBool>,
    state_changed: Arc<Event>,
    done: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let raw_hwnd: isize = hwnd.0 as isize;

    thread::spawn(move || {
        // 初回は None 扱いで必ず publish + notify する。
        let mut last: Option<bool> = None;

        while !done.load(Ordering::Acquire) {
            let hwnd = HWND(raw_hwnd as *mut _);

            let mut pt = POINT::default();
            let mut cr = RECT::default();
            // SAFETY: `GetCursorPos`/`GetClientRect` はともに read-only で
            // out 引数へ書き込むのみ。`hwnd` は main が生かし続ける有効窓
            // （done+join で本ワーカ停止後に Window が drop される）。スタイル変更はしない。
            let read_ok =
                unsafe { GetCursorPos(&mut pt).is_ok() && GetClientRect(hwnd, &mut cr).is_ok() };

            if read_ok {
                // DComp はウィンドウ原点から合成するが、窓は WS_POPUP で client==window にしてあるので、
                // 見える円と判定円は client 矩形の中心で一致する。
                let mut tl = POINT {
                    x: cr.left,
                    y: cr.top,
                };
                let mut br = POINT {
                    x: cr.right,
                    y: cr.bottom,
                };
                // SAFETY: `ClientToScreen` は read-only な座標写像（point を in/out）。
                let map_ok = unsafe {
                    ClientToScreen(hwnd, &mut tl).as_bool()
                        && ClientToScreen(hwnd, &mut br).as_bool()
                };

                if map_ok {
                    let client_screen = RECT {
                        left: tl.x,
                        top: tl.y,
                        right: br.x,
                        bottom: br.y,
                    };
                    // 円内 = 不透明 = クリック透過 OFF（false）／円外 = 透明 = ON（true）。
                    let desired = !alpha_is_opaque(pt, client_screen);

                    if last != Some(desired) {
                        desired_passthrough.store(desired, Ordering::Release);
                        state_changed.notify(usize::MAX);
                        println!(
                            "[cursor] desired click-through = {} (cursor=({}, {}) client_screen=[{},{},{},{}])",
                            if desired {
                                "ON(透過)"
                            } else {
                                "OFF(不透過)"
                            },
                            pt.x,
                            pt.y,
                            client_screen.left,
                            client_screen.top,
                            client_screen.right,
                            client_screen.bottom
                        );
                        last = Some(desired);
                    }
                }
            }

            thread::sleep(Duration::from_millis(16));
        }
    })
}

/// 状態変化適用タスク（StateApplier・UI スレッド専有）。
///
/// スタイル変更 API（`SetWindowLongPtrW`/`SetWindowPos`）は窓所有スレッド（UI）でのみ呼べるため、
/// 別スレッドのワーカでなく、`spawn_local` で UI スレッド上に起こしたここで適用する。
///
/// ループは listener-first poll-then-await: await の前に listener を張り、無条件に一度 poll して
/// 差分を収束させてから await する。notify が pending でなくても起動時に一度必ず収束し（カーソルが
/// 円内で始まれば初回 OFF を一度だけ適用）、poll と await の間に届いた notify も取りこぼさない。
///
/// 差分時（`desired != applied`）だけ `WS_EX_TRANSPARENT` ビットを加除し（他ビットは保存）、
/// `SWP_FRAMECHANGED` で反映して遷移をログに出す。非差分時はスタイル API を呼ばない。
/// ワーカの「変化時のみ notify」とこの `applied` ガードの二重防御は意図的。
///
/// `applied` の初期値は窓の初期 ex_style（クリック透過 ON）に合わせて `true`。
///
/// 終了: 抜ける道は持たない。`block_on` が戻ると `spawn_local` のタスクは進まない（`wintf-winmsg-executor` の約束）。
fn spawn_state_applier(
    hwnd: HWND,
    desired_passthrough: Arc<AtomicBool>,
    state_changed: Arc<Event>,
    dragging: Arc<AtomicBool>,
) {
    let raw_hwnd: isize = hwnd.0 as isize;

    spawn_local(async move {
        let mut applied: bool = true;

        loop {
            let listener = state_changed.listen();
            let desired = desired_passthrough.load(Ordering::Acquire);

            // ドラッグ中は透過トグルを抑止する（ドラッグ開始は円内＝OFF。ここで ON へ戻すと
            // SetCapture が切れドラッグが崩壊する）。終了時に WM_LBUTTONUP／WM_CAPTURECHANGED が
            // state_changed を notify して本ループを起こし、改めて再収束させる。
            if desired != applied && !dragging.load(Ordering::Acquire) {
                let hwnd = HWND(raw_hwnd as *mut _);

                // SAFETY: `hwnd` は main が生かす有効窓。`GetWindowLongPtrW` は
                // read-only。ポインタ幅（32/64bit）は windows クレートの isize で吸収。
                let cur = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
                let transparent_bit = WS_EX_TRANSPARENT.0 as isize;
                let new_ex = if desired {
                    cur | transparent_bit
                } else {
                    cur & !transparent_bit
                };

                // SAFETY: `hwnd` は UI スレッドが所有する有効窓。`SetWindowLongPtrW`/
                // `SetWindowPos` は窓所有スレッドからの呼出（本タスクは UI スレッド上で動く）。
                unsafe {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex);
                    if let Err(e) = SetWindowPos(
                        hwnd,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    ) {
                        eprintln!("[applier][warn] SetWindowPos(SWP_FRAMECHANGED) 失敗: {e}");
                    }
                }

                println!(
                    "[applier] click-through {} (WS_EX_TRANSPARENT {} ・ex_style 0x{:X}→0x{:X})",
                    if applied { "ON→OFF" } else { "OFF→ON" },
                    if desired { "add" } else { "remove" },
                    cur as u32,
                    new_ex as u32,
                );

                applied = desired;
            }

            listener.await;
        }
    });
}

fn main() {
    init_dpi_awareness();
    println!("=== pilot: clickthrough-alpha-toggle 先進坑 ===");

    let state_changed = Arc::new(Event::new());
    let dragging = Arc::new(AtomicBool::new(false));
    let state = Rc::new(AppState::new(state_changed.clone(), dragging.clone()));
    // ハンドルは block_on 復帰まで生かす（Drop で DestroyWindow される）。
    let _window = make_window(state.clone());
    println!(
        "[window] NOREDIRECTIONBITMAP|TOPMOST|LAYERED|TRANSPARENT 窓を生成（初期クリック透過 ON・LAYERED はフラグのみ/実験）"
    );

    // 枠なし（WS_POPUP）にして client==window にする。非クライアント領域（タイトルバー/枠）が
    // あると、DComp（CreateTargetForHwnd はウィンドウ原点から合成）と GetClientRect（クライアント
    // 原点）がずれ、見える円より反応領域がタイトルバーぶん下にずれる。
    // SAFETY: 窓所有スレッド（UI）から自窓へ。GWL_STYLE/GWL_EXSTYLE は独立で LAYERED/
    // TRANSPARENT 等 ex_style には触れない。SWP_FRAMECHANGED で非クライアント再計算を反映。
    unsafe {
        let _ = SetWindowLongPtrW(_window.hwnd(), GWL_STYLE, WS_POPUP.0 as isize);
        let _ = SetWindowPos(
            _window.hwnd(),
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    println!("[window] WS_POPUP 化（枠なし＝client==window でタイトルバー由来の縦ズレを解消）");

    // Window::new_ex は WS_VISIBLE を立てず ShowWindow も呼ばないため、明示的に SW_SHOW しないと
    // DComp 描画が済んでいても画面に何も出ない。
    // SAFETY: 窓所有スレッド（UI）から自窓の HWND に対して呼ぶ。
    unsafe {
        let _ = ShowWindow(_window.hwnd(), SW_SHOW);
    }
    println!("[window] ShowWindow(SW_SHOW) で可視化");

    match build_pipeline(_window.hwnd()) {
        Ok(pipeline) => {
            let color = state.current_color();
            if let Err(e) = draw_circle(&pipeline, color) {
                eprintln!("[dcomp][warn] 初回描画に失敗: {e}");
            } else {
                println!(
                    "[dcomp] パイプライン構築＋初回描画 完了（surface {}x{}・透明 Clear ＋ 中心に不透明円 r={RADIUS}）",
                    pipeline.width, pipeline.height
                );
            }
            // ドロップ＝合成ツリー崩壊。block_on 復帰時の state ドロップまで生かす。
            *state.pipeline.borrow_mut() = Some(pipeline);
        }
        Err(e) => eprintln!("[dcomp][warn] DComp パイプライン構築に失敗: {e}"),
    }

    // 初期 true は起動時のクリック透過 ON に合わせる。
    let desired_passthrough = Arc::new(AtomicBool::new(true));
    let worker_done = Arc::new(AtomicBool::new(false));
    let worker = spawn_cursor_worker(
        _window.hwnd(),
        desired_passthrough.clone(),
        state_changed.clone(),
        worker_done.clone(),
    );
    println!(
        "[cursor] 監視ワーカ起動（別 std::thread・16ms ポーリング・desired 公開＋変化時 notify）"
    );

    // spawn 順（worker → applier）には依らない: applier は listener-first poll-then-await なので、
    // worker の初回 notify が applier の最初の listen より先に発火しても取りこぼさない。
    spawn_state_applier(
        _window.hwnd(),
        desired_passthrough.clone(),
        state_changed.clone(),
        dragging.clone(),
    );
    println!(
        "[applier] 状態変化適用タスク起動（spawn_local・UI スレッド・差分時のみ WS_EX_TRANSPARENT 加除）"
    );

    let shutdown = state.shutdown.listen();
    block_on(async move {
        shutdown.await;
    });
    println!("[window] WM_CLOSE 受領 → shutdown 完了・清掃終了");

    // ── ライフサイクル終了（唯一の確定 shutdown 経路）──
    // StateApplier は止めない: `block_on` が戻ると `spawn_local` のタスクは進まない（`wintf-winmsg-executor` の約束）。
    // ワーカは 16ms 間隔で done を見るので、join は最大 ~16ms 待つだけ。panic 終了時も join の Err を
    // ログに残してプロセスは正常終了させる。
    worker_done.store(true, Ordering::Release);
    if worker.join().is_err() {
        eprintln!("[cursor][warn] 監視ワーカの join に失敗（パニック終了）");
    } else {
        println!("[cursor] 監視ワーカ停止（done → join 完了）");
    }

    // ここで _window が drop され DestroyWindow される。ワーカは既に join 済みで HWND を触らない。
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 中心 (cx, cy)・幅 w・高さ h の矩形を作るヘルパ。
    fn rect_centered(cx: i32, cy: i32, w: i32, h: i32) -> RECT {
        RECT {
            left: cx - w / 2,
            top: cy - h / 2,
            right: cx + w / 2,
            bottom: cy + h / 2,
        }
    }

    fn pt(x: i32, y: i32) -> POINT {
        POINT { x, y }
    }

    #[test]
    fn center_is_opaque() {
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(alpha_is_opaque(pt(1000, 1000), win));
    }

    #[test]
    fn just_inside_radius_is_opaque() {
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(alpha_is_opaque(pt(1000 + (RADIUS - 1), 1000), win));
        assert!(alpha_is_opaque(pt(1000, 1000 - (RADIUS - 1)), win));
    }

    #[test]
    fn exactly_on_boundary_is_opaque() {
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(alpha_is_opaque(pt(1000 + RADIUS, 1000), win));
        assert!(alpha_is_opaque(pt(1000, 1000 + RADIUS), win));
    }

    #[test]
    fn just_outside_radius_is_transparent() {
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(!alpha_is_opaque(pt(1000 + (RADIUS + 1), 1000), win));
        assert!(!alpha_is_opaque(pt(1000, 1000 - (RADIUS + 1)), win));
    }

    #[test]
    fn judged_relative_to_window_center_not_fixed_coords() {
        let win = rect_centered(-5000, -3000, 400, 400);
        assert!(alpha_is_opaque(pt(-5000, -3000), win));
        // 固定座標 (960,540) は窓から遠く離れており不透明扱いされてはならない。
        assert!(!alpha_is_opaque(pt(960, 540), win));
    }

    #[test]
    fn asymmetric_rect_uses_rect_center() {
        // left=100,right=1100 -> cx=600 / top=200,bottom=400 -> cy=300。
        let win = RECT {
            left: 100,
            top: 200,
            right: 1100,
            bottom: 400,
        };
        assert!(alpha_is_opaque(pt(600, 300), win)); // 中心
        assert!(alpha_is_opaque(pt(600 + RADIUS, 300), win)); // 境界
        assert!(!alpha_is_opaque(pt(600 + RADIUS + 1, 300), win)); // 直後＝外
        // 矩形の幾何中心ではない点（例: left 寄り）は円外。
        assert!(!alpha_is_opaque(pt(100, 300), win));
    }

    #[test]
    fn large_coords_no_overflow() {
        let win = rect_centered(2_000_000, 2_000_000, 400, 400);
        assert!(alpha_is_opaque(pt(2_000_000, 2_000_000), win));
        assert!(!alpha_is_opaque(pt(2_000_000 + RADIUS + 1, 2_000_000), win));
    }
}

#[cfg(test)]
#[path = "main_drag_tests.rs"]
mod drag_tests;
