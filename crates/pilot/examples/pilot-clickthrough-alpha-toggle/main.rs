//! 先進坑: pilot-clickthrough-alpha-toggle
//!
//! 対応 spec: `.kiro/specs/pilot-clickthrough-alpha-toggle/`
//! 一次記録（動機・概要・検証結果）は隣の README.md を正本とする。
//! T1〜T8 の詳細台帳と REPORT.md はタスク 6.1 で作成する（本ファイルは骨組みのみ）。
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
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{block_on, spawn_local};
use windows::Win32::Foundation::{HMODULE, HWND, LRESULT, POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateDevice, D2D1_ELLIPSE, ID2D1Device, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, ID3D11Device,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice3, IDCompositionDesktopDevice, IDCompositionDevice2,
    IDCompositionSurface, IDCompositionTarget, IDCompositionVisual2,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetClientRect, GetCursorPos, GetWindowLongPtrW, SW_SHOW,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WM_CLOSE, WM_LBUTTONDOWN,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};
// ClientToScreen は Gdi モジュール（read-only な client→screen 座標写像）。
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::core::{Interface, Result};

/// 不透明円の半径（物理ピクセル, R4.1）。
///
/// この定数はタスク 3.2（DComp 描画円）と**共有**する。描画円と判定円が
/// バイト単位で一致する（R2.2/R4.1）よう、円中心の算出（窓矩形中心）と半径を
/// 本定数経由で同一にすること。値や中心算出をここ以外で重複定義しない。
const RADIUS: i32 = 200;

/// α マスク純関数（窓中心・半径 `RADIUS` の円・物理座標）。
///
/// `cursor` が不透明円の**内側**なら `true`（=クリック透過 OFF）、外側なら `false`
/// （=クリック透過 ON）を返す。円中心は `win_rect` の中心
/// `cx=(left+right)/2, cy=(top+bottom)/2`、半径は `RADIUS`。内外判定は
/// `dx*dx + dy*dy <= r*r`（境界は内側扱い）を **i64** で計算し、マルチモニタ仮想
/// スクリーン上の大／負座標での i32 オーバーフローを避ける（R7.3）。
///
/// 純関数（副作用なし）。プライマリモニタ固定座標は前提にせず、円中心を実際の
/// `win_rect` から算出してカーソル物理座標と同一基準で比較する（R4.4/R7.2/R7.3）。
/// 実レンダリング α バッファのサンプリングは行わない（プレースホルダ円の継ぎ目, R4.3。
/// 実装は本坑の責務）。差し替え可能な独立シーム（R4.2）。
///
/// 非テストコードからはカーソル監視ワーカ（タスク 4.1・`spawn_cursor_worker`）が
/// 16ms ごとに呼ぶ（円内外 → desired click-through 判定）。
fn alpha_is_opaque(cursor: POINT, win_rect: RECT) -> bool {
    // 円中心 = 窓矩形の中心（タスク 3.2 の描画円と同一算出, R2.2/R4.1）。
    let cx = (win_rect.left + win_rect.right) / 2;
    let cy = (win_rect.top + win_rect.bottom) / 2;

    // i64 で二乗・比較し、マルチモニタ大／負座標での i32 オーバーフローを防ぐ（R7.3）。
    let dx = (cursor.x - cx) as i64;
    let dy = (cursor.y - cy) as i64;
    let r = RADIUS as i64;

    // 境界は内側扱い（`<=`）。
    dx * dx + dy * dy <= r * r
}

/// プロセスを Per-Monitor Aware v2（PMv2）に設定する（R7.1）。
///
/// PMv2 では `GetCursorPos`/`GetWindowRect` がともに物理スクリーン座標を返すため、
/// 後続タスクの円判定（物理座標一致・T7 の前提）が成立する。失敗は握り潰さず
/// 警告ログを残す（T7 の前提証跡。設計 Error Handling 参照）。
fn init_dpi_awareness() {
    // SAFETY: Win32 境界。`SetProcessDpiAwarenessContext` はプロセスグローバルな
    // DPI awareness をスレッドセーフに設定する。プロセス起動直後・他スレッド／
    // DPI 依存処理の前に一度だけ呼ぶ（main 冒頭）。
    let result = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    match result {
        Ok(()) => println!("[dpi] SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2): Ok"),
        Err(e) => eprintln!(
            "[dpi][warn] SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2) failed: {e}"
        ),
    }
}

/// 不透明円の塗りつぶし色パレット（タスク 3.3 のクリック色トグルで巡回する候補）。
///
/// すべて α = 1.0（完全不透明・R4.1）。トグルは色のみ変え、円の幾何（中心・半径）と
/// クリック透過（`WS_EX_TRANSPARENT`）には一切干渉しない（責務分離・設計「視覚的透過方式」節）。
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

/// DComp 視覚パイプラインの COM インターフェース群（UI スレッド専有・`!Send` 許容, R2.5）。
///
/// これらは**ドロップすると合成ツリーが崩壊する**ため（target/visual/device の解放は
/// composition を破棄する）、窓が生きている間 `AppState` 内で保持し続ける。
/// `desktop` から `IDCompositionDevice2`（CreateVisual/CreateSurface/Commit）へ cast した
/// `device` を描画に用いる。`target`/`visual` は SetRoot/SetContent 済みで保持のみ目的。
///
/// `swapchain` ではなく `IDCompositionSurface` を visual content に用いる（設計「視覚的
/// 透過方式」節が明示的に許可する同等最小の代替。surface.BeginDraw が D2D デバイス
/// コンテキストを直接返すため、swapchain＋D2D-over-DXGI-surface より配線が単純）。
#[allow(dead_code)] // d3d/d2d_device/target/visual は保持専用（ドロップ抑止）。surface/device は描画で使用。
struct DCompPipeline {
    /// D3D11 デバイス（BGRA 対応・DComp/D2D の土台）。保持専用。
    d3d: ID3D11Device,
    /// D2D デバイス（DComp の rendering device 兼 surface 描画系列）。保持専用。
    d2d_device: ID2D1Device,
    /// DComp デスクトップデバイス（CreateTargetForHwnd を持つ）。保持専用。
    desktop: IDCompositionDesktopDevice,
    /// CreateVisual/CreateSurface/Commit を持つデバイス（描画・Commit で使用）。
    device: IDCompositionDevice2,
    /// 窓に紐づく composition target（SetRoot 済み）。保持専用（ドロップで合成破棄）。
    target: IDCompositionTarget,
    /// ルートビジュアル（SetContent(surface) 済み）。保持専用。
    visual: IDCompositionVisual2,
    /// 視覚内容を供給する surface（BeginDraw/EndDraw で D2D 描画）。再描画で使用。
    surface: IDCompositionSurface,
    /// surface のピクセルサイズ（= 窓矩形の物理サイズ）。円中心算出に使用。
    width: u32,
    height: u32,
}

/// 透過トップモスト窓に紐づく共有状態（`!Send` で良い＝`Rc`／`HWND`／COM を内包する, R2.5）。
/// 窓を閉じる契機を `block_on` の future へ伝える shutdown `Event` を保持する。
///
/// 本タスク（3.2）で DComp パイプライン（`pipeline`）と円の現在色インデックス
/// （`color_index`）を追加した。色トグルon-click（3.3）・カーソルワーカ／スタイルトグル
/// （4.x/5.x）は後続で配線される。`pipeline` は `RefCell` 等で包まず、wndproc が `Fn`
/// ゆえ後続 3.3 の再描画は `&DCompPipeline`＋`Cell<usize>` の現在色読みで行える（COM 呼出は
/// `&self` メソッド）。
struct AppState {
    /// 窓クローズ（`WM_CLOSE`）で notify され、`block_on` の await を完了させる。
    shutdown: Event,
    /// 現在の円色インデックス（`COLORS` への添字）。初期色＝0。クリック色トグル（3.3）が
    /// 反転に使う。本タスクでは初期描画の色決定にのみ読む。
    color_index: Cell<usize>,
    /// DComp 視覚パイプライン。窓セットアップ後に `Some` が入り、以降ドロップしない
    /// （ドロップ＝合成ツリー崩壊）。初期化前は `None`。
    pipeline: std::cell::RefCell<Option<DCompPipeline>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            shutdown: Event::new(),
            color_index: Cell::new(0),
            pipeline: std::cell::RefCell::new(None),
        }
    }

    /// 現在の円色（`COLORS[color_index]`）。
    fn current_color(&self) -> D2D1_COLOR_F {
        COLORS[self.color_index.get()]
    }

    /// 円色インデックスを次へ巡回（`COLORS` 長で剰余）し、新しい現在色を返す（R6.3）。
    ///
    /// クリック色トグル（3.3）が `WM_LBUTTONDOWN` で呼ぶ。`Cell` の interior mutability で
    /// `Fn` wndproc から安全に反転できる（`&self`・単一 UI スレッド・再入なし）。
    fn toggle_color(&self) -> D2D1_COLOR_F {
        let next = (self.color_index.get() + 1) % COLORS.len();
        self.color_index.set(next);
        COLORS[next]
    }
}

/// NOREDIRECTIONBITMAP・トップモスト・クリック透過の三点セット ex_style で
/// トップレベル窓を生成する（設計 TransparentWindow コンポーネント, R2.1/R2.3）。
///
/// 起動時初期状態は **クリック透過 ON**（`WS_EX_TRANSPARENT` を初期 ex_style に含む）。
/// `WS_EX_NOREDIRECTIONBITMAP` は視覚的透過（redirection surface を持たず GDI/`WM_PAINT`
/// が画面に出ない → DComp 前提）のため、`WS_EX_TOPMOST` は最前面固定のため固定で付す。
///
/// `WS_EX_TRANSPARENT` の動的トグルはタスク 4.2 の責務（ここでは初期値のみ）。
/// `WS_EX_LAYERED` は付けない（R2.3）。`WM_NCHITTEST` は自前処理しない（R2.4）。
///
/// wndproc クロージャは `Fn`（`FnMut` ではない）ため、状態変更は `Cell`/`RefCell` ／
/// `event_listener::Event::notify`（&self）で行う。`WM_CLOSE` で shutdown を notify し、
/// `block_on(async { shutdown.await })` を完了させて清掃終了させる。ライブラリの
/// `wndproc_typed` は `WM_CLOSE` で `DestroyWindow` を呼ばず `LRESULT(0)` を返すため、
/// 窓の実破棄は `Window` の `Drop`（`block_on` 復帰後の drop）が担う。
fn make_window(state: Rc<AppState>) -> Window<Rc<AppState>> {
    // 初期 ex_style = NOREDIRECTIONBITMAP | TOPMOST | TRANSPARENT（= クリック透過 ON）。
    let ex_style = WINDOW_EX_STYLE(
        WS_EX_NOREDIRECTIONBITMAP.0 | WS_EX_TOPMOST.0 | WS_EX_TRANSPARENT.0,
    );

    Window::new_ex(
        WindowType::TopLevel,
        ex_style,
        state,
        move |state: Pin<&Rc<AppState>>, msg: WindowMessage| -> Option<LRESULT> {
            let s: &AppState = state.get_ref();
            match msg.msg {
                WM_CLOSE => {
                    // shutdown を通知して block_on の future を完了させる（清掃終了）。
                    // ライブラリは WM_CLOSE で DestroyWindow を呼ばない＝LRESULT(0) を返す。
                    s.shutdown.notify(usize::MAX);
                    Some(LRESULT(0))
                }
                WM_LBUTTONDOWN => {
                    // 受領ログ＝T3 の証跡（R6.1/R6.2）。期待では不透明円の内側を踏んだクリックだけが
                    // ここへ届く（円外＝WS_EX_TRANSPARENT で背面へ抜ける）。
                    // 検証強化（T2/T3/T6 の一意判定）: クリックのクライアント座標（lParam）を取り、
                    // 描画円（クライアント中心・半径 RADIUS）の内外を判定してログに残す。
                    // 「円外なのに受領」が出れば WS_EX_TRANSPARENT の透過漏れ＝核心 Unknown の否定証跡。
                    let lp = msg.lparam.0;
                    let click_x = (lp & 0xFFFF) as i16 as i32;
                    let click_y = ((lp >> 16) & 0xFFFF) as i16 as i32;
                    let mut cr = RECT::default();
                    // SAFETY: Win32 境界。read-only。msg.hwnd は有効窓。
                    let _ = unsafe { GetClientRect(msg.hwnd, &mut cr) };
                    let ccx = cr.right / 2;
                    let ccy = cr.bottom / 2;
                    let ddx = (click_x - ccx) as i64;
                    let ddy = (click_y - ccy) as i64;
                    let r = RADIUS as i64;
                    let inside = ddx * ddx + ddy * ddy <= r * r;
                    println!(
                        "[click] WM_LBUTTONDOWN 受領 client=({click_x},{click_y}) 円中心=({ccx},{ccy}) → {} → 色トグル＋DComp 再描画",
                        if inside {
                            "円内（正常受領）"
                        } else {
                            "円外（!! WS_EX_TRANSPARENT 透過漏れ !!）"
                        }
                    );

                    // 円色インデックスを反転し（R6.3）、新色で DComp 再描画する。
                    // GDI/WM_PAINT/InvalidateRect は使わず DComp 描画パス（draw_circle）のみ再利用
                    // ＝Clear(透明)＋FillEllipse(新色)＋EndDraw＋Commit（設計「視覚的透過方式」節）。
                    let color = s.toggle_color();
                    match s.pipeline.borrow().as_ref() {
                        Some(pipeline) => {
                            if let Err(e) = draw_circle(pipeline, color) {
                                eprintln!("[dcomp][warn] 色トグル後の再描画に失敗: {e}");
                            }
                        }
                        // パイプライン未構築（初期化失敗時）。色は既に反転済みで次回描画に反映。
                        None => eprintln!(
                            "[dcomp][warn] pipeline 未構築のため再描画をスキップ（色のみ反転）"
                        ),
                    }
                    Some(LRESULT(0))
                }
                // 他メッセージは DefWindowProc にフォールバック（None）。
                // WM_NCHITTEST は自前処理しない（R2.4）。DComp 構築（3.2）・
                // カーソルワーカ／スタイルトグル（4.x/5.x）は本タスクの責務外。
                _ => None,
            }
        },
    )
    .expect("透過トップモスト窓の生成に失敗（Window::new_ex）")
}

/// DComp 視覚パイプラインを構築する（設計「視覚的透過方式」節・R2.2/R2.3）。
///
/// チェーン: `D3D11CreateDevice`(BGRA) → `IDXGIDevice` → `D2D1CreateDevice` →
/// `DCompositionCreateDevice3`(=`IDCompositionDesktopDevice`) → cast
/// `IDCompositionDevice2`（CreateVisual/CreateSurface/Commit 系列）→
/// `CreateSurface`(BGRA8・premultiplied) → `CreateVisual` → `SetContent(surface)` →
/// `CreateTargetForHwnd(hwnd, topmost=true)` → `SetRoot(visual)`。Commit は初回描画後に行う。
///
/// surface サイズは窓の物理矩形（PMv2 ゆえ `GetWindowRect` は物理ピクセル, R7.2）。
/// `WS_EX_LAYERED` は使わない（視覚透過は DComp visual の per-pixel α が担う）。GDI/`WM_PAINT`/
/// `DwmExtendFrameIntoClientArea` は一切使わない（NOREDIRECTIONBITMAP 窓では画面に出ない）。
fn build_pipeline(hwnd: HWND) -> Result<DCompPipeline> {
    // 窓の物理矩形（PMv2 ＝物理ピクセル）。surface サイズ＝矩形サイズ。
    let mut rect = RECT::default();
    // SAFETY: Win32 境界。`hwnd` は直前に生成した有効窓。out 引数 `rect` は書き込み専用。
    // DComp の CreateTargetForHwnd は**クライアント領域**に合成するため、surface は client サイズに
    // 合わせる（窓全体だと枠ぶんズレて見える円と判定円が一致しない・R4.1/R7.2）。
    // GetClientRect は (0,0,client_w,client_h) を返す（PMv2 ＝物理ピクセル）。
    unsafe { GetClientRect(hwnd, &mut rect)? };
    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    // 1) D3D11 デバイス（BGRA 対応フラグ・D2D 相互運用に必須）。
    let mut d3d: Option<ID3D11Device> = None;
    // SAFETY: Win32/COM 境界。out 引数 `ppDevice`(Some(&mut d3d)) を非 null で渡すため
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

    // 2) DXGI デバイス（同一 D3D11 デバイスの別インターフェース）。
    let dxgi: IDXGIDevice = d3d.cast()?;

    // 3) D2D デバイス（DComp の rendering device 兼 surface 描画系列）。
    // SAFETY: Win32/COM 境界。`dxgi` は有効。第2引数 None で既定プロパティ。
    let d2d_device: ID2D1Device = unsafe { D2D1CreateDevice(&dxgi, None)? };

    // 4) DComp デスクトップデバイス（rendering device に D2D デバイスを渡す＝repo 準拠）。
    // SAFETY: Win32/COM 境界。`d2d_device` は有効・IUnknown として渡る。
    let desktop: IDCompositionDesktopDevice = unsafe { DCompositionCreateDevice3(&d2d_device)? };
    // CreateVisual/CreateSurface/Commit を持つ基底デバイス（DesktopDevice は Device2 を継承）。
    let device: IDCompositionDevice2 = desktop.cast()?;

    // 5) 視覚内容の surface（BGRA8・premultiplied α）。
    // SAFETY: Win32/COM 境界。引数はすべて値・既定列挙。out は Result で受ける。
    let surface: IDCompositionSurface = unsafe {
        device.CreateSurface(
            width,
            height,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )?
    };

    // 6) ルートビジュアル → surface を content に。
    // SAFETY: Win32/COM 境界。`device`/`surface` は有効。
    let visual: IDCompositionVisual2 = unsafe { device.CreateVisual()? };
    // SAFETY: Win32/COM 境界。`surface` は IUnknown として content に設定可能。
    unsafe { visual.SetContent(&surface)? };

    // 7) 窓に紐づく composition target（topmost=true）→ ルート設定。
    // SAFETY: Win32/COM 境界。`hwnd` は有効窓。topmost で最前面合成。
    let target: IDCompositionTarget = unsafe { desktop.CreateTargetForHwnd(hwnd, true)? };
    // SAFETY: Win32/COM 境界。`visual` は IDCompositionVisual。
    unsafe { target.SetRoot(&visual)? };

    Ok(DCompPipeline {
        d3d,
        d2d_device,
        desktop,
        device,
        target,
        visual,
        surface,
        width,
        height,
    })
}

/// パイプラインの surface に「透明 Clear ＋ 不透明円」を描画し Commit する（R2.2/R4.1/R7.2）。
///
/// 手順: `surface.BeginDraw::<ID2D1DeviceContext>()`（surface 用 D2D コンテキスト＋atlas
/// オフセット POINT を返す）→ `Clear(α=0)`（背面が透ける）→ 現在色で `FillEllipse`
/// （中心＝surface 中心＋atlas オフセット・半径 `RADIUS`・α=1）→ `surface.EndDraw()` →
/// `device.Commit()`。
///
/// 円中心はクライアント中心 `(width/2, height/2)`（物理ピクセル）。これは判定円
/// （`alpha_is_opaque` の `((left+right)/2,(top+bottom)/2)` ＝窓矩形中心）を窓ローカル座標へ
/// 平行移動したものと一致し、描画円と判定円が同一窓の同一中心・同一半径 `RADIUS` を指す
/// （R2.2/R4.1）。atlas オフセットは surface 割当原点の補正で、中心に加算する（SetTransform/
/// Matrix を介さず加算で吸収＝windows-numerics 非依存）。GDI/`WM_PAINT` は使わない。
fn draw_circle(pipeline: &DCompPipeline, color: D2D1_COLOR_F) -> Result<()> {
    // surface 用の D2D デバイスコンテキスト＋割当原点（atlas オフセット）。
    // BeginDraw は描画状態の DC を返す（明示の BeginDraw/EndDraw は surface 側が括る）。
    let mut offset = POINT::default();
    // SAFETY: Win32/COM 境界。`updaterect=None` で全面更新。`offset` は out。返る DC は
    // この surface を対象とする描画用コンテキスト。
    let dc: ID2D1DeviceContext = unsafe { pipeline.surface.BeginDraw(None, &mut offset)? };

    // PMv2 ＝物理ピクセル基準で描くため DPI を 96（1 DIP = 1px）に固定。
    // SAFETY: Win32/COM 境界。`dc` は有効。
    unsafe { dc.SetDpi(96.0, 96.0) };

    // 透明クリア（α=0）＝背面（別プロセス）が透ける（R2.2）。
    let transparent = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    // SAFETY: Win32/COM 境界。`dc` は有効・色は値ポインタ。
    unsafe { dc.Clear(Some(&transparent)) };

    // 不透明円ブラシ（現在色・α=1）。
    // SAFETY: Win32/COM 境界。`color` は値ポインタ。brushproperties=None で既定。
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None)? };

    // 円中心 = surface 中心（物理px）＋ atlas オフセット。半径 = RADIUS。
    // 中心算出は判定円（窓矩形中心）と同一基準のローカル平行移動（R2.2/R4.1）。
    let cx = offset.x as f32 + (pipeline.width as f32) / 2.0;
    let cy = offset.y as f32 + (pipeline.height as f32) / 2.0;
    // windows-numerics を名指しせず構築するため Default + フィールド代入。
    let mut ellipse = D2D1_ELLIPSE::default();
    ellipse.point.X = cx;
    ellipse.point.Y = cy;
    ellipse.radiusX = RADIUS as f32;
    ellipse.radiusY = RADIUS as f32;
    // SAFETY: Win32/COM 境界。`ellipse` は値ポインタ・`brush` は ID2D1Brush。
    unsafe { dc.FillEllipse(&ellipse, &brush) };

    // surface 描画確定 → composition へ反映。
    // SAFETY: Win32/COM 境界。BeginDraw と対。
    unsafe { pipeline.surface.EndDraw()? };
    // SAFETY: Win32/COM 境界。視覚ツリーの変更を DWM へコミット。
    unsafe { pipeline.device.Commit()? };

    Ok(())
}

/// カーソル監視ワーカを別 `std::thread` で起動する（タスク 4.1・R3.1/R3.2/R3.3/R10.2）。
///
/// UI スレッドでも tokio でもない独立 OS スレッド上で 16ms ごとに以下を回す:
/// 1. `GetCursorPos` + `GetWindowRect`（ともに read-only な Win32 API）で物理座標を読む（R3.4）。
/// 2. `alpha_is_opaque(pt, wr)` で不透明円の内外を判定する。
/// 3. desired click-through を `desired_passthrough` に公開する
///    — 円の**外**（`alpha_is_opaque==false`）= クリック透過 ON（`true`, R5.2）／
///    円の**内**（`alpha_is_opaque==true`）= クリック透過 OFF（`false`, R5.1）。
/// 4. desired が前ループから**変化したとき（および初回）だけ** `state_changed` を notify する
///    （未変化フレームでは notify しない・R3.3／R5.4 を下支え）。変化はログ＋座標で残す（T4 証跡）。
///
/// **スレッド越え HWND**: UI スレッド固有でない read-only API しか触らないため、`HWND` を
/// 生の `isize`（`hwnd.0 as isize`）で渡し、ワーカ内で `HWND(raw as *mut _)` で再構築する。
/// これによりポインタ幅依存（32/64bit）を保ったまま `AppState`（`!Send`）に `unsafe impl Send`
/// を付けずに済む。スタイル変更 API（`SetWindowLongPtr`/`SetWindowPos`）はここでは一切呼ばない
/// （UI スレッド専有・タスク 4.2 の StateApplier の責務）。
///
/// `done` が立つとループを抜けて清掃終了する（R8.2）。`done` の read は `Acquire`、
/// publish は `Release`／count 系は `Relaxed`（reference example の作法に準拠）。
fn spawn_cursor_worker(
    hwnd: HWND,
    desired_passthrough: Arc<AtomicBool>,
    state_changed: Arc<Event>,
    done: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    // HWND を生の isize にしてから move（read-only API 専用・unsafe impl Send 不要）。
    let raw_hwnd: isize = hwnd.0 as isize;

    thread::spawn(move || {
        // 前ループの desired。初回は未設定（None）扱いで必ず publish + notify する。
        let mut last: Option<bool> = None;

        while !done.load(Ordering::Acquire) {
            // ワーカ内で HWND を再構築（生 isize → ポインタ）。read-only API のみに使用。
            let hwnd = HWND(raw_hwnd as *mut _);

            let mut pt = POINT::default();
            let mut cr = RECT::default();
            // SAFETY: Win32 境界。`GetCursorPos`/`GetClientRect` はともに read-only で
            // out 引数へ書き込むのみ。`hwnd` は main が生かし続ける有効窓
            // （done+join で本ワーカ停止後に Window が drop される）。スタイル変更はしない。
            let read_ok = unsafe {
                GetCursorPos(&mut pt).is_ok() && GetClientRect(hwnd, &mut cr).is_ok()
            };

            if read_ok {
                // 判定はクライアント領域基準（R4.1「クライアント中央」）。DComp は client area に
                // 合成するため、見える円（client 中心に描画）と判定円を一致させるには client 矩形を
                // スクリーン座標へ写して中心を求める（窓矩形だと枠ぶんズレる）。GetClientRect は
                // (0,0,cw,ch) ゆえ ClientToScreen で左上・右下をスクリーン座標へ写す。
                let mut tl = POINT { x: cr.left, y: cr.top };
                let mut br = POINT { x: cr.right, y: cr.bottom };
                // SAFETY: Win32 境界。`ClientToScreen` は read-only な座標写像（point を in/out）。
                let map_ok = unsafe {
                    ClientToScreen(hwnd, &mut tl).as_bool() && ClientToScreen(hwnd, &mut br).as_bool()
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

                    // 初回（last==None）または変化時のみ publish + notify（未変化は黙る・R3.3）。
                    if last != Some(desired) {
                        desired_passthrough.store(desired, Ordering::Release);
                        state_changed.notify(usize::MAX);
                        println!(
                            "[cursor] desired click-through = {} (cursor=({}, {}) client_screen=[{},{},{},{}])",
                            if desired { "ON(透過)" } else { "OFF(不透過)" },
                            pt.x, pt.y,
                            client_screen.left, client_screen.top, client_screen.right, client_screen.bottom
                        );
                        last = Some(desired);
                    }
                }
            }

            thread::sleep(Duration::from_millis(16));
        }
    })
}

/// 状態変化適用タスク（StateApplier・UI スレッド専有・タスク 4.2・R5.3/R5.4/R5.5）。
///
/// `spawn_local`（**別 `std::thread` ではない**）で UI スレッド上に async タスクを起こす。
/// スタイル変更 API（`SetWindowLongPtrW`/`SetWindowPos`）は窓所有スレッド（UI）でのみ
/// 呼べるため、ワーカ（4.1・別スレッド）ではなくここで適用する（設計 Architecture
/// 「判定・適用の分離」）。ワーカ（4.1）は一切変更しない。
///
/// ループ（**取りこぼし防止＋起動時初期状態収束**・listener-first poll-then-await・タスク 5.1）:
/// 1. `let listener = state_changed.listen();`（await **前**に張る → notify を逃さない）。
/// 2. `applied_done`（＝block_on 復帰後に立つ shutdown フラグ）が立っていれば break。
/// 3. **無条件の初期 poll**: `desired_passthrough` を `Acquire` で読み、ローカル `applied` と
///    比較して差分時のみ適用する（下記）。これにより、await の前に一度必ず希望状態へ収束し、
///    起動時にカーソルが円**内**で始まった場合の初回 OFF を確実に一度だけ適用する
///    （notify が pending でなくても収束する・設計「起動時初期状態確定」）。
/// 4. `listener.await`（ワーカの notify か終了 notify で起床）。poll と await の間に届いた
///    notify は手順 1 で張った listener が確実に捕捉する（lost wakeup なし）。
/// 5. 起床後はループ先頭へ戻り、再び listener を張り直してから `applied_done` を確認し
///    （終了 notify への即応）、次の poll を行う。
///
/// **差分時（`desired != applied`）のみ**:
///   - 現在の `GetWindowLongPtrW(hwnd, GWL_EXSTYLE)` を読み、`WS_EX_TRANSPARENT` ビット
///     **だけ**を加除（desired==ON で立てる・OFF で落とす）して `new_ex` を算出する。
///     `WS_EX_NOREDIRECTIONBITMAP`/`WS_EX_TOPMOST`（および他ビット）は保存（R5.3）。
///   - `SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex)` ＋
///     `SetWindowPos(hwnd, None, 0,0,0,0, SWP_FRAMECHANGED|SWP_NOMOVE|SWP_NOSIZE|
///     SWP_NOZORDER|SWP_NOACTIVATE)`（R5.3）。
///   - `applied = desired` を更新し、遷移（"ON→OFF"／"OFF→ON"）をログ（R5.5・T4 証跡）。
///
/// **非差分時（`desired == applied`）**: スタイル API を一切呼ばず、遷移ログも出さない
/// （R5.4・T5 証跡＝no-op パスは style 呼出ゼロ）。ワーカの「変化時のみ notify」と
/// この `applied` ガードの二重防御は意図的（設計「判定・適用フロー」）。
///
/// `applied` の初期値は窓初期 ex_style（`WS_EX_TRANSPARENT` 含む＝クリック透過 ON）と
/// 一致させて `true`（設計 State Management）。スレッド越え HWND はワーカと同様に生 `isize`
/// で渡し、UI スレッド内で `HWND(raw as *mut _)` に再構成する（このタスクは UI スレッド上で
/// 動くため所有スレッドからの呼出になり、スタイル変更 API を正当に呼べる）。
///
/// 終了: `applied_done`（block_on 復帰後の shutdown 後始末で立てて notify される）で
/// ループを抜ける。最終的なライフサイクル配線（初期状態収束・done 立て＋join 順序）は
/// タスク 5.1（本関数のループ順序＋main の配線）で確定する。
fn spawn_state_applier(
    hwnd: HWND,
    desired_passthrough: Arc<AtomicBool>,
    state_changed: Arc<Event>,
    applied_done: Arc<AtomicBool>,
) {
    // HWND を生の isize にしてから move（ポインタ幅 32/64bit 保持）。
    let raw_hwnd: isize = hwnd.0 as isize;

    spawn_local(async move {
        // ローカル applied。窓初期 ex_style（TRANSPARENT 含む＝クリック透過 ON）と一致＝true。
        let mut applied: bool = true;

        loop {
            // await 前に listener を張る（notify 取りこぼし防止）。次の poll と await の間に
            // 届く notify を確実に捕捉する。
            let listener = state_changed.listen();
            if applied_done.load(Ordering::Acquire) {
                break;
            }

            // 無条件の初期 poll（listener-first poll-then-await・タスク 5.1）。
            // listener を張った後・await の前に希望状態を読んで差分収束する。これにより
            // 起動初回は notify の有無に関わらず一度必ず収束し（円内始動なら初回 OFF を一度だけ
            // 適用）、かつ poll〜await 間に来た notify も上の listener が捕捉する（lost wakeup なし）。
            let desired = desired_passthrough.load(Ordering::Acquire);

            // 差分時のみスタイル適用＋ログ。非差分時は style API を一切呼ばない（R5.4）。
            if desired != applied {
                // UI スレッド内で HWND を再構成（所有スレッドゆえスタイル変更 API を呼べる）。
                let hwnd = HWND(raw_hwnd as *mut _);

                // 現在の ex_style を読み、WS_EX_TRANSPARENT ビットだけを加除して new_ex を算出。
                // NOREDIRECTIONBITMAP/TOPMOST（および他ビット）は保存（R5.3）。
                // SAFETY: Win32 境界。`hwnd` は main が生かす有効窓。`GetWindowLongPtrW` は
                // read-only。ポインタ幅（32/64bit）は windows クレートの isize で吸収。
                let cur = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
                let transparent_bit = WS_EX_TRANSPARENT.0 as isize;
                let new_ex = if desired {
                    cur | transparent_bit // ON＝WS_EX_TRANSPARENT を立てる（R5.2）。
                } else {
                    cur & !transparent_bit // OFF＝WS_EX_TRANSPARENT を落とす（R5.1）。
                };

                // ex_style を実適用 → SWP_FRAMECHANGED で OS にスタイル変更を反映（R5.3）。
                // SAFETY: Win32 境界。`hwnd` は UI スレッドが所有する有効窓。`SetWindowLongPtrW`/
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

                // 遷移ログ（R5.5・T4 証跡）。変化したときのみ出す。
                println!(
                    "[applier] click-through {} (WS_EX_TRANSPARENT {} ・ex_style 0x{:X}→0x{:X})",
                    if applied { "ON→OFF" } else { "OFF→ON" },
                    if desired { "add" } else { "remove" },
                    cur as u32,
                    new_ex as u32,
                );

                applied = desired;
            }
            // desired == applied のときは何もしない（style API 非呼出・ログ非出力・R5.4）。

            // poll で収束させた後に await（手順 1 で張った listener で起床）。次の起床は
            // ワーカの変化 notify か終了 notify。起床後はループ先頭へ戻り、listener を張り直して
            // applied_done を確認してから次の poll を行う（終了 notify への即応）。
            listener.await;
        }
    });
}

fn main() {
    init_dpi_awareness();
    println!("=== pilot: clickthrough-alpha-toggle 先進坑 ===");

    let state = Rc::new(AppState::new());
    // 窓を生成（初期: NOREDIRECTIONBITMAP|TOPMOST|TRANSPARENT = クリック透過 ON）。
    // ハンドルは block_on 復帰まで生かす（Drop で DestroyWindow される）。
    let _window = make_window(state.clone());
    println!("[window] NOREDIRECTIONBITMAP|TOPMOST|TRANSPARENT 窓を生成（初期クリック透過 ON）");

    // 窓を可視化する（R2.1「ウィンドウを表示する」）。Window::new_ex は WS_VISIBLE を
    // 立てず ShowWindow も呼ばないため、明示的に SW_SHOW しないと画面に何も出ない
    // （NOREDIRECTIONBITMAP ＋ DComp 描画が完了していても窓自体が非表示なら不可視）。
    // SAFETY: 窓所有スレッド（UI）から自窓の HWND に対して呼ぶ。
    unsafe {
        let _ = ShowWindow(_window.hwnd(), SW_SHOW);
    }
    println!("[window] ShowWindow(SW_SHOW) で可視化");

    // DComp パイプライン構築＋初回描画（透明 Clear ＋ 中心に不透明円）。
    // 失敗は握り潰さず警告（NOREDIRECTIONBITMAP 窓は描画なしだと画面に何も出ない＝T1 で気付く）。
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
            // COM 群を保持（ドロップ＝合成ツリー崩壊）。block_on 復帰時の state ドロップまで生かす。
            *state.pipeline.borrow_mut() = Some(pipeline);
        }
        Err(e) => eprintln!("[dcomp][warn] DComp パイプライン構築に失敗: {e}"),
    }

    // カーソル監視ワーカ（タスク 4.1）を別 std::thread で起動。
    // desired_passthrough = ワーカが公開する希望クリック透過状態（初期 true = 起動時 ON と整合）。
    // state_changed = 変化時に UI を起こす wake イベント（消費側 StateApplier は 4.2 で配線）。
    // done = ワーカ停止フラグ（block_on 復帰後に立てて join → 清掃終了）。
    let desired_passthrough = Arc::new(AtomicBool::new(true));
    let state_changed = Arc::new(Event::new());
    let worker_done = Arc::new(AtomicBool::new(false));
    let worker = spawn_cursor_worker(
        _window.hwnd(),
        desired_passthrough.clone(),
        state_changed.clone(),
        worker_done.clone(),
    );
    println!("[cursor] 監視ワーカ起動（別 std::thread・16ms ポーリング・desired 公開＋変化時 notify）");

    // 状態変化適用タスク（StateApplier・タスク 4.2／ループ順序確定は 5.1）を UI スレッドへ
    // spawn_local。ワーカの desired/state_changed を消費し、差分時のみ WS_EX_TRANSPARENT を
    // 加除する（UI スレッド専有・スタイル変更は窓所有スレッドのみ）。block_on より前に spawn
    // して executor に駆動させる。
    //
    // 起動時初期状態収束（タスク 5.1・R8.1/R8.2 前提）: applier のループは listener-first
    // poll-then-await（listener を張る → 無条件 poll で差分収束 → await）なので、worker の
    // 初回 notify が applier の最初の listen より先に発火しても取りこぼさない。block_on が
    // executor を駆動した最初のパスで applier は desired_passthrough を必ず poll し、起動時に
    // カーソルが円**内**なら初回 OFF を一度だけ適用、円**外**なら applied(ON) と一致して
    // style API を一切呼ばない。worker は初回判定を無条件に publish+notify する（4.1）ため、
    // applier の初回 poll 時点で desired_passthrough は有効値を保持する。spawn 順
    // （worker → applier）はこの idiom により頑健（順序非依存）。applier_done は終了用フラグ。
    let applier_done = Arc::new(AtomicBool::new(false));
    spawn_state_applier(
        _window.hwnd(),
        desired_passthrough.clone(),
        state_changed.clone(),
        applier_done.clone(),
    );
    println!("[applier] 状態変化適用タスク起動（spawn_local・UI スレッド・差分時のみ WS_EX_TRANSPARENT 加除）");

    // メッセージループ：WM_CLOSE → shutdown notify → この future 完了でループ終了。
    let shutdown = state.shutdown.listen();
    block_on(async move {
        shutdown.await;
    });
    println!("[window] WM_CLOSE 受領 → shutdown 完了・清掃終了");

    // ── ライフサイクル終了（唯一の確定 shutdown 経路・タスク 5.1・R8.1/R8.2）──
    // WM_CLOSE → shutdown.notify → 上の block_on(shutdown.await) 復帰 → ここでテアダウン。
    //
    // (1) StateApplier（spawn_local タスク）終了: applied_done を立て（Release）、
    //     state_changed を notify して applier の pending な listener.await を即起床させる。
    //     起床後 applier はループ先頭で applied_done を見て break しタスク終了する
    //     （これで spawn_local タスクがプロセスを生かし続けない）。block_on 復帰後は executor
    //     が回らないが、明示フラグ＋notify で確実にループを畳む（drop 任せにしない）。
    applier_done.store(true, Ordering::Release);
    state_changed.notify(usize::MAX);

    // (2) カーソルワーカ（別 std::thread）終了: done を立て（Release）→ join で確実に合流する
    //     （R8.2）。ワーカは 16ms スリープ間隔で done(Acquire) を観測しループを抜けるため、
    //     join は最大 ~16ms 待つだけ（許容）。二重 join はせず、panic 終了時も join の Err を
    //     ログに残してプロセスは正常終了させる（hang/panic なし）。
    worker_done.store(true, Ordering::Release);
    if worker.join().is_err() {
        eprintln!("[cursor][warn] 監視ワーカの join に失敗（パニック終了）");
    } else {
        println!("[cursor] 監視ワーカ停止（done → join 完了）");
    }

    // ここで _window が drop され DestroyWindow される（ライブラリの Window::Drop）。
    // ワーカは既に join 済みで HWND を触らない＝use-after-free なし。残留スレッドなしで
    // プロセスは正常終了する（R8.1）。
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
        // 円中心は常に不透明（R4.1）。
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(alpha_is_opaque(pt(1000, 1000), win));
    }

    #[test]
    fn just_inside_radius_is_opaque() {
        // 半径ちょうど手前（199）は内側（R4.1）。
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(alpha_is_opaque(pt(1000 + (RADIUS - 1), 1000), win));
        assert!(alpha_is_opaque(pt(1000, 1000 - (RADIUS - 1)), win));
    }

    #[test]
    fn exactly_on_boundary_is_opaque() {
        // 境界（==RADIUS）は内側扱い（`<=`, R4.1）。
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(alpha_is_opaque(pt(1000 + RADIUS, 1000), win));
        assert!(alpha_is_opaque(pt(1000, 1000 + RADIUS), win));
    }

    #[test]
    fn just_outside_radius_is_transparent() {
        // 半径直後（201）は外側＝透過（R4.1）。
        let win = rect_centered(1000, 1000, 800, 600);
        assert!(!alpha_is_opaque(pt(1000 + (RADIUS + 1), 1000), win));
        assert!(!alpha_is_opaque(pt(1000, 1000 - (RADIUS + 1)), win));
    }

    #[test]
    fn judged_relative_to_window_center_not_fixed_coords() {
        // 負／オフセット座標の窓。判定は窓中心基準で、旧 (960,540) 固定中心ではない
        // （R4.4/R7.3）。窓中心は (-5000, -3000)。
        let win = rect_centered(-5000, -3000, 400, 400);
        // 窓中心は不透明。
        assert!(alpha_is_opaque(pt(-5000, -3000), win));
        // 旧固定中心 (960,540) は窓から遠く離れており不透明扱いされてはならない。
        assert!(!alpha_is_opaque(pt(960, 540), win));
    }

    #[test]
    fn asymmetric_rect_uses_rect_center() {
        // 非正方（横長）矩形でも中心は (left+right)/2, (top+bottom)/2。
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
        // マルチモニタ仮想スクリーンの大座標で i32 二乗オーバーフローしない（i64 計算, R7.3）。
        let win = rect_centered(2_000_000, 2_000_000, 400, 400);
        assert!(alpha_is_opaque(pt(2_000_000, 2_000_000), win));
        assert!(!alpha_is_opaque(pt(2_000_000 + RADIUS + 1, 2_000_000), win));
    }
}
