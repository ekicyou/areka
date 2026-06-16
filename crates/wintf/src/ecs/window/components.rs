//! ウィンドウコンポーネント定義・ライフサイクルフック
//!
//! - `DpiChangeContext`: WM_DPICHANGED → WM_WINDOWPOSCHANGED 間の DPI 同期伝達
//! - `CompositionMode`: 描画パイプライン選択
//! - `Window`: ウィンドウ作成パラメータ
//! - `WindowHandle`: 作成済みウィンドウのハンドル情報
//! - `WindowStyle`: スタイル・拡張スタイル

use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use std::cell::RefCell;
use tracing::{debug, trace};
use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::api::*;
use crate::ecs::Visual;

use super::dpi::DPI;
use super::window_pos::{SetWindowParentToLayoutRoot, WindowPos};

// ============================================================================
// DpiChangeContext - WM_DPICHANGED → WM_WINDOWPOSCHANGED 間の DPI 同期伝達
// ============================================================================

/// DPI変更コンテキスト
///
/// `WM_DPICHANGED`と`WM_WINDOWPOSCHANGED`間で新DPIを同期的に受け渡す。
/// `DefWindowProcW`内から`SetWindowPos`が呼ばれ、その中で`WM_WINDOWPOSCHANGED`が
/// 同期的に発火するため、スレッドローカルコンテキストで情報を渡す。
#[derive(Debug, Clone)]
pub struct DpiChangeContext {
    /// 新しいDPI値
    pub new_dpi: DPI,
    /// Windowsが推奨するウィンドウRECT（物理座標）
    pub suggested_rect: RECT,
}

thread_local! {
    /// DPI変更コンテキストのスレッドローカルストレージ
    static DPI_CHANGE_CONTEXT: RefCell<Option<DpiChangeContext>> = const { RefCell::new(None) };
}

impl DpiChangeContext {
    /// 新しいDpiChangeContextを作成
    pub fn new(new_dpi: DPI, suggested_rect: RECT) -> Self {
        Self {
            new_dpi,
            suggested_rect,
        }
    }

    /// コンテキストをスレッドローカルに設定
    ///
    /// `WM_DPICHANGED`ハンドラから`DefWindowProcW`を呼ぶ前に呼び出す。
    pub fn set(ctx: DpiChangeContext) {
        trace!(
            dpi_x = ctx.new_dpi.dpi_x,
            dpi_y = ctx.new_dpi.dpi_y,
            suggested_left = ctx.suggested_rect.left,
            suggested_top = ctx.suggested_rect.top,
            suggested_right = ctx.suggested_rect.right,
            suggested_bottom = ctx.suggested_rect.bottom,
            "DpiChangeContext::set"
        );
        DPI_CHANGE_CONTEXT.with(|cell| {
            *cell.borrow_mut() = Some(ctx);
        });
    }

    /// コンテキストを取得・消費
    ///
    /// `WM_WINDOWPOSCHANGED`ハンドラから呼び出し、コンテキストが存在すれば
    /// 取得して消費（Noneにリセット）する。
    pub fn take() -> Option<DpiChangeContext> {
        DPI_CHANGE_CONTEXT.with(|cell| {
            let ctx = cell.borrow_mut().take();
            if let Some(ref c) = ctx {
                trace!(
                    dpi_x = c.new_dpi.dpi_x,
                    dpi_y = c.new_dpi.dpi_y,
                    "DpiChangeContext::take consumed"
                );
            }
            ctx
        })
    }
}

// ============================================================================
// CompositionMode
// ============================================================================

/// 描画パイプライン選択 enum。Window フィールドとして保持。
///
/// ウィンドウ生成時に指定し、以降は不変。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompositionMode {
    /// ULW パイプライン: D2D1 合成 → DIBSection → UpdateLayeredWindow
    /// 透過クリックスルー対応。デフォルト。
    #[default]
    ULW,
    /// DComp パイプライン: IDCompositionTarget → Visual → Surface
    /// 通常ウィンドウUI向け。
    DComp,
}

// ============================================================================
// Window
// ============================================================================

/// Windowコンポーネント - ウィンドウ作成に必要な基本パラメータを保持
/// スタイルや位置・サイズは WindowStyle, WindowPos コンポーネントで指定
#[derive(Component, Debug, Clone)]
#[component(on_add = on_window_add)]
pub struct Window {
    pub title: String,
    pub parent: Option<HWND>,
    /// 描画パイプライン選択。生成後は変更しないこと。
    /// ULW: 透過クリックスルー対応、DComp: 通常ウィンドウUI向け。
    pub composition_mode: CompositionMode,
}

impl Window {
    /// 描画パイプラインを返す。生成後は変更不可。
    pub fn composition_mode(&self) -> CompositionMode {
        self.composition_mode
    }
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: "Window".to_string(),
            parent: None,
            composition_mode: CompositionMode::default(), // ULW
        }
    }
}

// SAFETY: `Window` は `parent: Option<HWND>` を保持し、`HWND`（windows-rs では
// `*mut c_void` の newtype）は自動では Send/Sync を実装しない（windows-rs 0.62.2）
// ため、自動導出されない。よってこの手動 impl は冗長ではなく必須である。
// 健全性: 親 HWND は不透明なウィンドウ識別子（値渡しで所有権・解放責務を伴わない）
// であり、`Window` は ECS コンポーネントとしてメインスレッドのシステム・フックから
// のみ参照される。`title: String` / `composition_mode: CompositionMode` は
// プレーンデータで自動的に Send+Sync。`drag/context.rs` の `WindowDragContext` と
// 同じ crate 標準の HWND 取り扱い方針。
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

// ============================================================================
// WindowStyle
// ============================================================================

/// Window Style / Ex Style
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WindowStyle {
    pub style: WINDOW_STYLE,
    pub ex_style: WINDOW_EX_STYLE,
}

impl Default for WindowStyle {
    fn default() -> Self {
        Self {
            // ULW（UpdateLayeredWindow）透過ウィンドウはフレームを持たないため
            // WS_POPUP を使用する。WS_OVERLAPPEDWINDOW だと AdjustWindowRectExForDpi が
            // タイトルバー・ボーダー分（約17px）を client↔window 変換時に加減算し、
            // ドラッグのたびにサイズが縮小するバグを引き起こす。
            style: WS_POPUP | WS_VISIBLE,
            // Phase 3: WS_EX_NOREDIRECTIONBITMAP → WS_EX_LAYERED
            // ULW 方式による alpha 透過描画に必要
            ex_style: WS_EX_LAYERED,
        }
    }
}

impl WindowStyle {
    /// 新しい WindowStyle を作成
    pub fn from_hwnd(hwnd: HWND) -> windows::core::Result<Self> {
        let style = WINDOW_STYLE(get_window_long_ptr(hwnd, GWL_STYLE)? as u32);
        let ex_style = WINDOW_EX_STYLE(get_window_long_ptr(hwnd, GWL_EXSTYLE)? as u32);
        Ok(Self { style, ex_style })
    }
}

// ============================================================================
// on_window_add hook
// ============================================================================

/// Windowコンポーネントが追加されたときに呼ばれるフック
/// WindowをLayoutRootの子として自動的に設定し、Visual/WindowPosコンポーネントを自動挿入する
fn on_window_add(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;

    // LayoutRootの子として設定
    world
        .commands()
        .queue(SetWindowParentToLayoutRoot { entity });

    // Visual自動挿入（既に存在する場合はスキップ）
    if world.get::<Visual>(entity).is_none() {
        world.commands().entity(entity).insert(Visual::default());
    }

    // WindowPos自動挿入（既に存在する場合はスキップ）
    // CreateWindow実行前にWindowPosが存在する必要があるため、on_window_addで追加
    if world.get::<WindowPos>(entity).is_none() {
        world.commands().entity(entity).insert(WindowPos::default());
        debug!(
            entity = ?entity,
            "WindowPos component inserted in on_window_add"
        );
    }

    // DPI事前セット: CreateWindowExW前にシステムDPIで初期化
    // これにより、最初のレイアウト計算からDPIスケーリングが有効になり、
    // Frame 2でのサイズジャンプ（800x700 → 1000x875等）を防ぐ。
    // ウィンドウ作成後にon_window_handle_addで実際のGetDpiForWindow()値に更新される。
    if world.get::<DPI>(entity).is_none() {
        let system_dpi = unsafe { GetDpiForSystem() } as u16;
        let dpi = if system_dpi > 0 {
            DPI::from_dpi(system_dpi, system_dpi)
        } else {
            DPI::default()
        };
        world.commands().entity(entity).insert(dpi);
        debug!(
            entity = ?entity,
            dpi_x = dpi.dpi_x,
            dpi_y = dpi.dpi_y,
            "DPI pre-initialized with system DPI in on_window_add"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== unsafe impl Send/Sync の不変条件（W7a-V） =====

    /// `Window` は `parent: Option<HWND>`（HWND は `*mut c_void` newtype・非 Send/Sync）
    /// を内包するため手動 `unsafe impl Send/Sync` で Send+Sync を表明している。本テストは
    /// その不変条件をコンパイル時に固定する回帰検知器（device 非依存）。将来 parent 以外に
    /// 非 Send なフィールドが追加され手動 impl が撤去された場合に検出する。
    #[test]
    fn test_window_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Window>();
    }

    // ===== Window::default =====

    #[test]
    fn test_window_default_fields() {
        let w = Window::default();
        assert_eq!(w.title, "Window");
        assert!(w.parent.is_none());
        // composition_mode は ULW（CompositionMode::default）
        assert_eq!(w.composition_mode(), CompositionMode::ULW);
    }

    // ===== WindowStyle::default =====

    #[test]
    fn test_window_style_default_is_popup_visible_layered() {
        // ULW 透過ウィンドウは WS_POPUP | WS_VISIBLE / WS_EX_LAYERED を使用する
        let style = WindowStyle::default();
        assert_eq!(style.style, WS_POPUP | WS_VISIBLE);
        assert_eq!(style.ex_style, WS_EX_LAYERED);
    }

    #[test]
    fn test_window_style_default_does_not_use_overlappedwindow() {
        // WS_OVERLAPPEDWINDOW を含まないこと（ドラッグ時サイズ縮小バグ回避のコメント根拠）
        let style = WindowStyle::default();
        assert_eq!(
            style.style & WS_OVERLAPPEDWINDOW,
            WS_POPUP & WS_OVERLAPPEDWINDOW,
            "既定スタイルは WS_OVERLAPPEDWINDOW のキャプション/ボーダービットを含まない"
        );
        // 念のため WS_CAPTION（タイトルバー）が立っていないことを確認
        assert_eq!(style.style & WS_CAPTION, WINDOW_STYLE(0));
    }

    #[test]
    fn test_window_style_partial_eq() {
        let a = WindowStyle::default();
        let b = WindowStyle::default();
        assert_eq!(a, b);
        let c = WindowStyle {
            style: WS_POPUP,
            ex_style: WS_EX_LAYERED,
        };
        assert_ne!(a, c);
    }

    // ===== CompositionMode =====

    #[test]
    fn test_composition_mode_eq_distinguishes_variants() {
        // 既定値・Debug は composition_mode_test.rs で固定済み。
        // ここでは 2 バリアントの非等価性のみ補完する。
        assert_ne!(CompositionMode::ULW, CompositionMode::DComp);
    }

    // ===== DpiChangeContext (thread_local set/take) =====

    #[test]
    fn test_dpi_change_context_new_stores_fields() {
        let dpi = DPI::from_dpi(144, 144);
        let rect = RECT {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        let ctx = DpiChangeContext::new(dpi, rect);
        assert_eq!(ctx.new_dpi, dpi);
        assert_eq!(ctx.suggested_rect.left, 1);
        assert_eq!(ctx.suggested_rect.bottom, 4);
    }

    #[test]
    fn test_dpi_change_context_take_returns_none_when_unset() {
        // set していない状態（または直前 take 済み）では None
        // 同一テストスレッドで set しない限り空。先に drain して前提を揃える。
        let _ = DpiChangeContext::take();
        assert!(DpiChangeContext::take().is_none());
    }

    #[test]
    fn test_dpi_change_context_set_then_take_consumes() {
        // 前提を揃えるため先に drain
        let _ = DpiChangeContext::take();

        let dpi = DPI::from_dpi(120, 120);
        let rect = RECT {
            left: 10,
            top: 20,
            right: 110,
            bottom: 120,
        };
        DpiChangeContext::set(DpiChangeContext::new(dpi, rect));

        // 1回目の take は設定値を返す
        let taken = DpiChangeContext::take().expect("context should be present after set");
        assert_eq!(taken.new_dpi, dpi);
        assert_eq!(taken.suggested_rect.right, 110);

        // 2回目の take は消費済みで None
        assert!(
            DpiChangeContext::take().is_none(),
            "take は消費的（take 後は None に戻る）"
        );
    }
}
