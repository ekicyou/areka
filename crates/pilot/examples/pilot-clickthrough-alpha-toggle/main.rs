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

use std::pin::Pin;
use std::rc::Rc;

use event_listener::Event;
use wintf_winmsg_executor::block_on;
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use windows::Win32::Foundation::{LRESULT, POINT, RECT};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    WINDOW_EX_STYLE, WM_CLOSE, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};

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
/// 非テストコードからの呼び出しはタスク 4.x（カーソルワーカ）が配線するまで無いため
/// `#[allow(dead_code)]` を付す。
#[allow(dead_code)]
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

/// 透過トップモスト窓に紐づく共有状態（`!Send` で良い＝`Rc`／`HWND` を内包する後続
/// タスクの拡張点, R2.5）。窓を閉じる契機を `block_on` の future へ伝える shutdown
/// `Event` を保持する。
///
/// 本タスク（3.1）では shutdown 配線のみ。DComp デバイス／ビジュアルツリー（3.2）・
/// 描画円の色（3.3）・カーソルワーカ／スタイルトグル（4.x/5.x）はここに追加される。
struct AppState {
    /// 窓クローズ（`WM_CLOSE`）で notify され、`block_on` の await を完了させる。
    shutdown: Event,
}

impl AppState {
    fn new() -> Self {
        Self {
            shutdown: Event::new(),
        }
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
                // 他メッセージは DefWindowProc にフォールバック（None）。
                // WM_NCHITTEST は自前処理しない（R2.4）。WM_LBUTTONDOWN（3.3）・
                // DComp 構築（3.2）・スタイルトグル（4.2）は本タスクの責務外。
                _ => None,
            }
        },
    )
    .expect("透過トップモスト窓の生成に失敗（Window::new_ex）")
}

fn main() {
    init_dpi_awareness();
    println!("=== pilot: clickthrough-alpha-toggle 先進坑 ===");

    let state = Rc::new(AppState::new());
    // 窓を生成（初期: NOREDIRECTIONBITMAP|TOPMOST|TRANSPARENT = クリック透過 ON）。
    // ハンドルは block_on 復帰まで生かす（Drop で DestroyWindow される）。
    let _window = make_window(state.clone());
    println!("[window] NOREDIRECTIONBITMAP|TOPMOST|TRANSPARENT 窓を生成（初期クリック透過 ON）");

    // メッセージループ：WM_CLOSE → shutdown notify → この future 完了でループ終了。
    let shutdown = state.shutdown.listen();
    block_on(async move {
        shutdown.await;
    });
    println!("[window] WM_CLOSE 受領 → shutdown 完了・清掃終了");

    // DComp パイプライン（3.2）・描画円の色（3.3）・カーソルワーカ／スタイルトグル
    // （4.x）・ワーカ join／初期状態収束（5.1）は後続タスクで実装する。
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
