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
