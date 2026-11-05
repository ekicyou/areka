#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # Taffy Flexbox Demo - Tunnel/Bubbleフェーズのイベント伝播デモ + ドラッグ移動
//!
//! このサンプルは、wintfフレームワークのポインターイベントシステムにおける
//! Tunnel（親→子）とBubble（子→親）の2フェーズイベント伝播を実演します。
//! また、ウィンドウドラッグ移動機能も実装されています。
//!
//! ## イベントフェーズの概念
//!
//! wintfのイベントシステムは、WinUI3/WPF/DOMイベントモデルと同様の2フェーズを実装:
//!
//! - **Tunnelフェーズ** (親→子): イベント発生前に親が介入可能
//! - **Bubbleフェーズ** (子→親): イベント発生後に親が処理可能
//!
//! ## 他のフレームワークとの対応表
//!
//! | wintf             | WinUI3           | WPF              | DOM Level 3      |
//! |-------------------|------------------|------------------|------------------|
//! | Phase::Tunnel     | PreviewMouseDown | PreviewMouseDown | Capture Phase    |
//! | Phase::Bubble     | MouseDown        | MouseDown        | Bubble Phase     |
//! | handler return    | e.Handled = true | e.Handled = true | stopPropagation()|
//! | sender引数        | e.OriginalSource | e.OriginalSource | event.target     |
//! | entity引数        | sender引数       | sender引数       | currentTarget    |
//!
//! ## デモの操作例
//!
//! 1. **FlexDemo-Container（灰色背景）を左クリック＆ドラッグ**
//!    - 期待: ウィンドウがドラッグ移動する
//!    - ログ: `[Drag] DragStart/Drag/DragEnd` が出力される
//!
//! 2. **GreenBoxChild（黄色矩形）を左クリック**
//!    - 期待: `[Tunnel] FlexContainer: Passing through` → `[Tunnel] GreenBox: Button pressed` → `[Tunnel] GreenBox: Captured event`
//!    - GreenBoxChildのログは出ない（親がTunnelでキャプチャ）
//!
//! 3. **GreenBoxChild（黄色矩形）を右クリック**
//!    - 期待: `[Tunnel] FlexContainer` → `[Tunnel] GreenBox` → `[Tunnel] GreenBoxChild` → `[Bubble] GreenBoxChild`
//!    - 親はキャプチャしない。子が Bubble で止めるので `[Bubble] GreenBox` は出ない
//!
//! 4. **Ctrl+左クリックでRedBox**
//!    - 期待: `[Tunnel] FlexContainer: Event stopped` のみ出力
//!    - RedBoxのログは出ない（Containerで停止）
//!
//! ## 実装パターン
//!
//! ```rust
//! fn handler(world: &mut World, sender: Entity, entity: Entity, ev: &Phase<PointerState>) -> bool {
//!     match ev {
//!         Phase::Tunnel(state) => {
//!             if state.ctrl_down && state.left_down {
//!                 // 親で事前処理してイベントを停止
//!                 return true; // stopPropagation相当
//!             }
//!             false
//!         }
//!         Phase::Bubble(state) => {
//!             // 通常のイベント処理
//!             if state.right_down {
//!                 // 処理...
//!                 return true;
//!             }
//!             false
//!         }
//!     }
//! }
//! ```

mod diagnostics;
mod drag;
mod handlers;
mod region;
mod setup;

pub(crate) use bevy_ecs::name::Name;
pub(crate) use bevy_ecs::prelude::*;
pub(crate) use std::time::Duration;
pub(crate) use tracing::{debug, info};
pub(crate) use tracing_subscriber::EnvFilter;
pub(crate) use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
pub(crate) use windows::core::Result;
pub(crate) use wintf::ecs::Visual;
pub(crate) use wintf::ecs::drag::{
    DragConfig, DragEndEvent, DragEvent, DragStartEvent, OnDrag, OnDragEnd, OnDragStart,
};
pub(crate) use wintf::ecs::layout::hit_region::HitRegionMap;
pub(crate) use wintf::ecs::layout::{BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension};
pub(crate) use wintf::ecs::layout::{
    GlobalArrangement, HitTest, PhysicalPoint, hit_test_in_window_ex,
};
pub(crate) use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed, Phase, PointerState};
pub(crate) use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
pub(crate) use wintf::ecs::widget::brushes::Brushes;
pub(crate) use wintf::ecs::widget::shapes::Rectangle;
pub(crate) use wintf::ecs::window::find_owner_window;
pub(crate) use wintf::ecs::{Point, Window, WindowPos};
pub(crate) use wintf::*;

#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub(crate) struct FlexDemoWindow;

/// イベントハンドラ用ウィンドウ識別文字列を返す
pub(crate) fn window_label(world: &World, entity: Entity) -> String {
    match find_owner_window(world, entity) {
        Some(win) => {
            let title = world
                .get::<Window>(win)
                .map(|w| w.title.as_str())
                .unwrap_or("?");
            format!("win={:?}({})", win, title)
        }
        None => "win=None".to_string(),
    }
}

fn main() -> Result<()> {
    human_panic::setup_panic!();

    // tracing-subscriber 初期化
    // RUST_LOG環境変数で制御: 例 RUST_LOG=wintf=debug,info
    // 環境変数未設定時はデフォルトでinfoレベル
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // 非同期タスクでFlexboxデモを実行
    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\nTaffy Flexboxレイアウトのデモ:");
    println!("  [上段] イベントシステムデモ（既存）");
    println!("    - 赤い矩形 (固定200x100) / 緑の矩形 (grow=1) / 青い矩形 (grow=2)");
    println!("  [下段] 名前付きヒット領域テスト");
    println!("    - 矩形リージョン (top-left/top-right/bottom-left/bottom-right)");
    println!("    - 多角形リージョン (同じ 4 つの角を四角形の多角形で)");
    println!("    - 混在 (矩形と多角形で 4 つの角)");
    println!("    - カラーマップ (4 色→4 つの角)");
    println!("    - フォールバック (NamedRegions without HitRegionMap)");
    println!("\n2秒後、プライマリ以外のモニターがあれば Window 1 をそこへ移動します。");
    println!("約64秒後に自動的にWindowを閉じてアプリ終了します。");

    // メッセージループを開始
    mgr.run()?;

    Ok(())
}

/// プライマリ以外のモニター中央付近の物理ピクセル座標を返す。
///
/// マルチモニター環境で DPI が異なるスクリーンへのウィンドウ配置テスト用。
/// 非プライマリモニターが存在しない場合は None を返す。
fn find_non_primary_monitor_origin() -> Option<windows::Win32::Foundation::POINT> {
    use std::cell::Cell;
    use windows::Win32::Foundation::{LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows_core::BOOL;

    // スレッドローカルで結果を収集（EnumDisplayMonitors コールバックから返す手段として）
    thread_local! {
        static RESULT: Cell<Option<POINT>> = const { Cell::new(None) };
    }

    RESULT.with(|r| r.set(None));

    unsafe extern "system" fn monitor_enum_proc(
        hmonitor: HMONITOR,
        _hdc: HDC,
        _lprc: *mut RECT,
        _lparam: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(hmonitor, &mut info) }.as_bool() {
            let is_primary = (info.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1
            if !is_primary {
                // モニター中央付近（左上から10%内側）を初期位置とする
                let rect = info.rcMonitor;
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                let pt = POINT {
                    x: rect.left + w / 10,
                    y: rect.top + h / 10,
                };
                RESULT.with(|r| r.set(Some(pt)));
                return BOOL(0); // 最初の非プライマリを見つけたら列挙停止
            }
        }
        BOOL(1) // 継続
    }

    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(monitor_enum_proc), LPARAM(0));
    }

    RESULT.with(|r| r.get())
}

/// 非同期デモ実行
///
/// タイムライン: 0s→create → 1s→dump① → 2s→change_layout → 3s→dump② → 64s→close
async fn run_demo(tx: CommandSender) {
    // === 0秒: ウィンドウ作成（2つ） ===
    info!("[Async] 0s: Creating Flexbox demo windows (multi-window)");
    let _ = tx.send(Box::new(|world: &mut World| {
        setup::create_flexbox_window(
            world,
            "wintf - Taffy Flexbox Demo (Window 1)",
            Point { x: 100, y: 100 },
        );
        setup::create_flexbox_window(
            world,
            "wintf - Taffy Flexbox Demo (Window 2)",
            find_non_primary_monitor_origin()
                .map(Point::from)
                .unwrap_or(Point { x: 1200, y: 100 }),
        );
    }));

    // === 1秒待機（DPI確定＋レイアウト安定のため） ===
    async_io::Timer::after(Duration::from_secs(1)).await;

    // === 1秒: dump① — 初期レイアウト状態の DPI/GA/BoxStyle ダンプ ===
    info!("[Async] 1s: Running DPI layout dump ① (initial)");
    let _ = tx.send(Box::new(diagnostics::dump_all_windows_dpi));

    // === 1秒待機 ===
    async_io::Timer::after(Duration::from_secs(1)).await;

    // === 2秒: レイアウトパラメーター変更 ===
    info!("[Async] 2s: Changing layout parameters");
    let _ = tx.send(Box::new(change_layout_parameters));

    // === 1秒待機 ===
    async_io::Timer::after(Duration::from_secs(1)).await;

    // === 3秒: dump② — 変更後レイアウト状態の DPI/GA/BoxStyle ダンプ ===
    info!("[Async] 3s: Running DPI layout dump ② (after change)");
    let _ = tx.send(Box::new(diagnostics::dump_all_windows_dpi));

    // === 1秒待機 ===
    async_io::Timer::after(Duration::from_secs(1)).await;

    // === 64秒: 終了（手で操作する時間を残す） ===
    async_io::Timer::after(Duration::from_secs(60)).await;
    info!("[Async] 64s: Closing windows");
    let _ = tx.send(Box::new(close_window));
}

/// レイアウトパラメーターを変更
fn change_layout_parameters(world: &mut World) {
    // WindowPos.position を変更してウィンドウを別モニターへ移動（サイズ・内部レイアウトは維持）
    // NOTE: BoxStyle.size および子要素のレイアウトパラメーターは一切変更しない。
    // ウィンドウの論理サイズと内部レイアウトを保持したままモニター間移動することで、
    // DPIスケーリング動作を正しく検証できる。
    let Some(target) = find_non_primary_monitor_origin().map(Point::from) else {
        info!("[Test] No non-primary monitor; Window 1 stays where it is");
        return;
    };
    // 2 枚とも FlexDemoWindow を持つので、題で Window 1 を選ぶ
    let mut wp_query = world.query_filtered::<(&Window, &mut WindowPos), With<FlexDemoWindow>>();
    if let Some((_, mut wp)) = wp_query
        .iter_mut(world)
        .find(|(w, _)| w.title.ends_with("(Window 1)"))
    {
        wp.position = Some(target);
        info!(
            "[Test] Window 1 moved to non-primary monitor ({}, {}) via WindowPos",
            target.x, target.y
        );
    }
}

/// ウィンドウを閉じる
fn close_window(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<FlexDemoWindow>>();
    let windows: Vec<Entity> = query.iter(world).collect();
    for window in windows {
        println!("[Test] Removing Window entity {:?}", window);
        world.despawn(window);
    }
}
