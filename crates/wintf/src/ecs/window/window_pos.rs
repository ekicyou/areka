//! ウィンドウ位置・サイズ・Z-order 管理
//!
//! - `ZOrder`: Z-order 設定
//! - `WindowPos`: ウィンドウ位置・サイズ・表示オプション（builder pattern）
//! - `SetWindowParentToLayoutRoot`: Window → LayoutRoot 親子関係設定コマンド

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::ecs::layout::LayoutRoot;
use crate::ecs::types::{Point, SizeI};

use super::window_handle::WindowHandle;

// ============================================================================
// ZOrder
// ============================================================================

/// Z-order の設定方法を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZOrder {
    /// Z-orderを変更しない
    #[default]
    NoChange,
    /// 最前面（常に最前面）
    TopMost,
    /// 最前面ではない（通常のウィンドウ）
    NoTopMost,
    /// 最前面に配置
    Top,
    /// 最背面に配置
    Bottom,
    /// 指定したウィンドウの後ろに配置
    InsertAfter(HWND),
}

// SAFETY: `ZOrder::InsertAfter(HWND)` バリアントが `HWND`（windows-rs では
// `*mut c_void` の newtype）を保持するため、自動では Send/Sync が導出されない
// （windows-rs 0.62.2 は HWND に Send/Sync を実装しない）。よってこの手動 impl は
// 冗長ではなく必須である。健全性: HWND は実質的にウィンドウを指す不透明な整数値で
// あり、スレッド間で値として受け渡しても所有権・解放責務を伴わない。`ZOrder` は
// `WindowPos` コンポーネント（ECS は Send+Sync を要求）のフィールドとして
// メインスレッドのシステムからのみ参照・更新される。`drag/context.rs` の
// `WindowDragContext` と同じ crate 標準の HWND 取り扱い方針。
unsafe impl Send for ZOrder {}
unsafe impl Sync for ZOrder {}

// ============================================================================
// WindowPos
// ============================================================================

/// ウィンドウの位置・サイズ・表示オプション
/// ウィンドウ作成時の初期位置・サイズにも使用される
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WindowPos {
    pub zorder: ZOrder,
    pub position: Option<Point>,
    pub size: Option<SizeI>,

    pub no_redraw: bool,        // SWP_NOREDRAW: 再描画しない
    pub no_activate: bool,      // SWP_NOACTIVATE: ウィンドウをアクティブにしない
    pub frame_changed: bool,    // SWP_FRAMECHANGED: フレーム変更を通知
    pub show_window: bool,      // SWP_SHOWWINDOW: ウィンドウを表示
    pub hide_window: bool,      // SWP_HIDEWINDOW: ウィンドウを非表示
    pub no_copy_bits: bool,     // SWP_NOCOPYBITS: クライアント領域をコピーしない
    pub no_owner_zorder: bool,  // SWP_NOOWNERZORDER: オーナーウィンドウのZ-orderを変更しない
    pub no_send_changing: bool, // SWP_NOSENDCHANGING: WM_WINDOWPOSCHANGINGを送信しない
    pub defer_erase: bool,      // SWP_DEFERERASE: WM_SYNCPAINTを送信しない
    pub async_window_pos: bool, // SWP_ASYNCWINDOWPOS: 非同期で処理
}

impl Default for WindowPos {
    fn default() -> Self {
        Self {
            zorder: ZOrder::NoChange,
            position: Some(Point {
                x: CW_USEDEFAULT,
                y: CW_USEDEFAULT,
            }),
            size: Some(SizeI {
                width: CW_USEDEFAULT,
                height: CW_USEDEFAULT,
            }),
            no_redraw: false,
            no_activate: false,
            frame_changed: false,
            show_window: false,
            hide_window: false,
            no_copy_bits: false,
            no_owner_zorder: false,
            no_send_changing: false,
            defer_erase: false,
            async_window_pos: false,
        }
    }
}

// SAFETY: `WindowPos` は `zorder: ZOrder` を保持し、`ZOrder::InsertAfter(HWND)` が
// `HWND`（`*mut c_void` の newtype）を内包するため、自動では Send/Sync が導出されない。
// この手動 impl は冗長ではなく必須である。健全性は `ZOrder` の impl と同根:
// HWND は不透明なウィンドウ識別子（値渡しで所有権・解放責務を伴わない）であり、
// `WindowPos` は ECS コンポーネントとしてメインスレッドのシステムからのみ
// 参照・更新される。残りのフィールド（Option<Point>/Option<SizeI>/各 bool）は
// プレーンデータで自動的に Send+Sync。
unsafe impl Send for WindowPos {}
unsafe impl Sync for WindowPos {}

impl WindowPos {
    /// 新しい WindowPos を作成
    pub fn new() -> Self {
        Self::default()
    }

    /// Z-order を設定
    pub fn with_zorder(mut self, zorder: ZOrder) -> Self {
        self.zorder = zorder;
        self
    }

    /// Z-orderを変更しない（デフォルト）
    pub fn zorder_no_change(mut self) -> Self {
        self.zorder = ZOrder::NoChange;
        self
    }

    /// 最前面（常に最前面）に配置
    pub fn zorder_topmost(mut self) -> Self {
        self.zorder = ZOrder::TopMost;
        self
    }

    /// 最前面ではない（通常のウィンドウ）に変更
    pub fn zorder_notopmost(mut self) -> Self {
        self.zorder = ZOrder::NoTopMost;
        self
    }

    /// 最前面に配置
    pub fn zorder_top(mut self) -> Self {
        self.zorder = ZOrder::Top;
        self
    }

    /// 最背面に配置
    pub fn zorder_bottom(mut self) -> Self {
        self.zorder = ZOrder::Bottom;
        self
    }

    /// 指定したウィンドウの後ろに配置
    pub fn zorder_insert_after(mut self, hwnd: HWND) -> Self {
        self.zorder = ZOrder::InsertAfter(hwnd);
        self
    }

    /// 位置を設定
    pub fn with_position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }

    /// サイズを設定
    pub fn with_size(mut self, size: SizeI) -> Self {
        self.size = Some(size);
        self
    }

    /// SWP_NOREDRAW フラグを設定
    pub fn no_redraw(mut self, value: bool) -> Self {
        self.no_redraw = value;
        self
    }

    /// SWP_NOACTIVATE フラグを設定
    pub fn no_activate(mut self, value: bool) -> Self {
        self.no_activate = value;
        self
    }

    /// SWP_FRAMECHANGED フラグを設定
    pub fn frame_changed(mut self, value: bool) -> Self {
        self.frame_changed = value;
        self
    }

    /// SWP_SHOWWINDOW フラグを設定
    pub fn show_window(mut self, value: bool) -> Self {
        self.show_window = value;
        self
    }

    /// SWP_HIDEWINDOW フラグを設定
    pub fn hide_window(mut self, value: bool) -> Self {
        self.hide_window = value;
        self
    }

    /// SWP_NOCOPYBITS フラグを設定
    pub fn no_copy_bits(mut self, value: bool) -> Self {
        self.no_copy_bits = value;
        self
    }

    /// SWP_NOOWNERZORDER フラグを設定
    pub fn no_owner_zorder(mut self, value: bool) -> Self {
        self.no_owner_zorder = value;
        self
    }

    /// SWP_NOSENDCHANGING フラグを設定
    pub fn no_send_changing(mut self, value: bool) -> Self {
        self.no_send_changing = value;
        self
    }

    /// SWP_DEFERERASE フラグを設定
    pub fn defer_erase(mut self, value: bool) -> Self {
        self.defer_erase = value;
        self
    }

    /// SWP_ASYNCWINDOWPOS フラグを設定
    pub fn async_window_pos(mut self, value: bool) -> Self {
        self.async_window_pos = value;
        self
    }

    /// bool値からフラグを生成
    fn build_flags(&self) -> SET_WINDOW_POS_FLAGS {
        let mut flags = SET_WINDOW_POS_FLAGS(0);

        // 自動判定: position が None なら SWP_NOMOVE
        if self.position.is_none() {
            flags |= SWP_NOMOVE;
        }

        // 自動判定: size が None なら SWP_NOSIZE
        if self.size.is_none() {
            flags |= SWP_NOSIZE;
        }

        // 自動判定: zorder が NoChange なら SWP_NOZORDER
        if self.zorder == ZOrder::NoChange {
            flags |= SWP_NOZORDER;
        }

        if self.no_redraw {
            flags |= SWP_NOREDRAW;
        }
        if self.no_activate {
            flags |= SWP_NOACTIVATE;
        }
        if self.frame_changed {
            flags |= SWP_FRAMECHANGED;
        }
        if self.show_window {
            flags |= SWP_SHOWWINDOW;
        }
        if self.hide_window {
            flags |= SWP_HIDEWINDOW;
        }
        if self.no_copy_bits {
            flags |= SWP_NOCOPYBITS;
        }
        if self.no_owner_zorder {
            flags |= SWP_NOOWNERZORDER;
        }
        if self.no_send_changing {
            flags |= SWP_NOSENDCHANGING;
        }
        if self.defer_erase {
            flags |= SWP_DEFERERASE;
        }
        if self.async_window_pos {
            flags |= SWP_ASYNCWINDOWPOS;
        }

        flags
    }

    /// システム用フラグビルダー（公開用）
    /// apply_window_pos_changesシステムから利用
    pub fn build_flags_for_system(&self) -> SET_WINDOW_POS_FLAGS {
        self.build_flags()
    }

    /// ZOrderに基づくhwnd_insert_afterを取得（公開用）
    /// apply_window_pos_changesシステムから利用
    pub fn get_hwnd_insert_after(&self) -> Option<HWND> {
        match self.zorder {
            ZOrder::NoChange => None,
            ZOrder::TopMost => Some(HWND_TOPMOST),
            ZOrder::NoTopMost => Some(HWND_NOTOPMOST),
            ZOrder::Top => Some(HWND_TOP),
            ZOrder::Bottom => Some(HWND_BOTTOM),
            ZOrder::InsertAfter(hwnd) => Some(hwnd),
        }
    }

    /// SetWindowPos を呼び出す
    pub fn set_window_pos(&self, hwnd: HWND) -> windows::core::Result<()> {
        let (x, y) = if let Some(pos) = self.position {
            (pos.x, pos.y)
        } else {
            (0, 0)
        };

        let (width, height) = if let Some(size) = self.size {
            (size.width, size.height)
        } else {
            (0, 0)
        };

        let flags = self.build_flags();
        let hwnd_insert_after = self.get_hwnd_insert_after();

        unsafe { SetWindowPos(hwnd, hwnd_insert_after, x, y, width, height, flags) }
    }

    /// クライアント領域の座標・サイズをウィンドウ全体の座標・サイズに変換する。
    ///
    /// # Arguments
    /// * `window_handle` - 変換対象のウィンドウハンドル
    ///
    /// # Returns
    /// * `Ok((x, y, width, height))` - 変換後のウィンドウ全体座標（左上x, 左上y, 幅, 高さ）
    /// * `Err(String)` - Win32 API呼び出し失敗時のエラーメッセージ
    ///
    /// # Notes
    /// - `WindowHandle::client_to_window_coords`に委譲
    /// - Windows 11専用実装（DPIフォールバック不要）
    pub fn to_window_coords(
        &self,
        window_handle: &WindowHandle,
    ) -> Result<(i32, i32, i32, i32), String> {
        // position/sizeがNoneの場合はデフォルト値(0, 0)を使用
        let position = self.position.unwrap_or(Point { x: 0, y: 0 });
        let size = self.size.unwrap_or(SizeI {
            width: 0,
            height: 0,
        });

        window_handle.client_to_window_coords(position, size)
    }

    /// ウィンドウ作成時に使用する座標変換（HWNDなしでスタイル情報から変換）
    ///
    /// # Arguments
    /// * `style` - ウィンドウスタイル
    /// * `ex_style` - 拡張ウィンドウスタイル
    /// * `dpi` - DPI値（0の場合はデフォルト96を使用）
    ///
    /// # Returns
    /// * `(x, y, width, height)` - 変換後のウィンドウ全体座標
    ///
    /// # Notes
    /// - CW_USEDEFAULTが含まれる場合は変換せずそのまま返す
    pub fn to_window_coords_for_creation(
        &self,
        style: WINDOW_STYLE,
        ex_style: WINDOW_EX_STYLE,
        dpi: u32,
    ) -> (i32, i32, i32, i32) {
        let position = self.position.unwrap_or(Point {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
        });
        let size = self.size.unwrap_or(SizeI {
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
        });

        // CW_USEDEFAULTが含まれる場合は変換をスキップ
        if position.x == CW_USEDEFAULT || size.width == CW_USEDEFAULT {
            return (position.x, position.y, size.width, size.height);
        }

        // 実際のシステムDPIを使用してウィンドウ枠サイズを計算する。
        // DPI=96 固定だと高DPI環境でフレームサイズが不一致となり、
        // CreateWindowExW 後の実クライアント領域サイズが意図値とずれる。
        // 0が渡された場合はフォールバックとして96を使用する。
        let dpi = if dpi == 0 { 96 } else { dpi };

        // クライアント領域からRECT構造体を構築
        let mut rect = RECT {
            left: position.x,
            top: position.y,
            right: position.x + size.width,
            bottom: position.y + size.height,
        };

        // AdjustWindowRectExForDpiでウィンドウ全体の矩形を計算
        // dpi=96により、物理ピクセル単位でウィンドウ枠のサイズが追加される
        let result = unsafe { AdjustWindowRectExForDpi(&mut rect, style, false, ex_style, dpi) };

        if result.is_err() {
            // 変換失敗時は元の座標を返す
            warn!(
                error = ?result,
                "AdjustWindowRectExForDpi failed during window creation, using original values"
            );
            return (position.x, position.y, size.width, size.height);
        }

        (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    }
}

// ============================================================================
// SetWindowParentToLayoutRoot
// ============================================================================

/// Window追加時にLayoutRootの子として設定するためのコマンド
pub(super) struct SetWindowParentToLayoutRoot {
    pub(super) entity: Entity,
}

impl Command for SetWindowParentToLayoutRoot {
    fn apply(self, world: &mut World) {
        // LayoutRootを検索
        let mut query = world.query_filtered::<Entity, With<LayoutRoot>>();
        let layout_root = query.iter(world).next();

        if let Some(root) = layout_root {
            // Windowエンティティがまだ親を持っていない場合のみChildOfを設定
            if let Ok(mut entity_mut) = world.get_entity_mut(self.entity) {
                if !entity_mut.contains::<ChildOf>() {
                    debug!(
                        root = ?root,
                        entity = ?self.entity,
                        "Setting ChildOf for Window entity"
                    );
                    entity_mut.insert(ChildOf(root));
                }
            }
        }
        // LayoutRootが見つからない場合は何もしない
        // 後のフレームで ensure_window_parent_system が処理する
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== unsafe impl Send/Sync の不変条件（W7a-V） =====

    /// `ZOrder` / `WindowPos` は HWND（`*mut c_void` newtype・非 Send/Sync）を内包する
    /// ため手動 `unsafe impl Send/Sync` で Send+Sync を表明している。本テストはその
    /// 不変条件をコンパイル時に固定する: 将来フィールドが追加され（かつ手動 impl が
    /// 撤去され）て Send/Sync が壊れた場合に検出する回帰検知器。device 非依存（型のみ）。
    #[test]
    fn test_window_pos_types_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ZOrder>();
        assert_send_sync::<WindowPos>();
    }

    // ===== ZOrder =====

    #[test]
    fn test_zorder_default_is_no_change() {
        assert_eq!(ZOrder::default(), ZOrder::NoChange);
    }

    // ===== WindowPos::default / new =====

    #[test]
    fn test_default_uses_cw_usedefault_position_and_size() {
        let pos = WindowPos::default();
        assert_eq!(pos.zorder, ZOrder::NoChange);
        assert_eq!(
            pos.position,
            Some(Point {
                x: CW_USEDEFAULT,
                y: CW_USEDEFAULT
            })
        );
        assert_eq!(
            pos.size,
            Some(SizeI {
                width: CW_USEDEFAULT,
                height: CW_USEDEFAULT
            })
        );
        // 全ての SWP bool フラグは既定で false
        assert!(!pos.no_redraw);
        assert!(!pos.no_activate);
        assert!(!pos.frame_changed);
        assert!(!pos.show_window);
        assert!(!pos.hide_window);
        assert!(!pos.no_copy_bits);
        assert!(!pos.no_owner_zorder);
        assert!(!pos.no_send_changing);
        assert!(!pos.defer_erase);
        assert!(!pos.async_window_pos);
    }

    #[test]
    fn test_new_equals_default() {
        assert_eq!(WindowPos::new(), WindowPos::default());
    }

    // ===== builder pattern =====

    #[test]
    fn test_builder_with_position_and_size() {
        let pos = WindowPos::new()
            .with_position(Point { x: 10, y: 20 })
            .with_size(SizeI {
                width: 300,
                height: 400,
            });
        assert_eq!(pos.position, Some(Point { x: 10, y: 20 }));
        assert_eq!(
            pos.size,
            Some(SizeI {
                width: 300,
                height: 400
            })
        );
    }

    #[test]
    fn test_builder_zorder_helpers() {
        assert_eq!(WindowPos::new().zorder_no_change().zorder, ZOrder::NoChange);
        assert_eq!(WindowPos::new().zorder_topmost().zorder, ZOrder::TopMost);
        assert_eq!(
            WindowPos::new().zorder_notopmost().zorder,
            ZOrder::NoTopMost
        );
        assert_eq!(WindowPos::new().zorder_top().zorder, ZOrder::Top);
        assert_eq!(WindowPos::new().zorder_bottom().zorder, ZOrder::Bottom);
        let hwnd = HWND(0x1234 as *mut _);
        assert_eq!(
            WindowPos::new().zorder_insert_after(hwnd).zorder,
            ZOrder::InsertAfter(hwnd)
        );
        assert_eq!(
            WindowPos::new().with_zorder(ZOrder::Top).zorder,
            ZOrder::Top
        );
    }

    #[test]
    fn test_builder_bool_flag_setters() {
        let pos = WindowPos::new()
            .no_redraw(true)
            .no_activate(true)
            .frame_changed(true)
            .show_window(true)
            .hide_window(true)
            .no_copy_bits(true)
            .no_owner_zorder(true)
            .no_send_changing(true)
            .defer_erase(true)
            .async_window_pos(true);
        assert!(pos.no_redraw);
        assert!(pos.no_activate);
        assert!(pos.frame_changed);
        assert!(pos.show_window);
        assert!(pos.hide_window);
        assert!(pos.no_copy_bits);
        assert!(pos.no_owner_zorder);
        assert!(pos.no_send_changing);
        assert!(pos.defer_erase);
        assert!(pos.async_window_pos);
    }

    // ===== build_flags_for_system (auto-detect NOMOVE/NOSIZE/NOZORDER) =====

    #[test]
    fn test_build_flags_default_sets_nozorder_only_when_pos_size_present() {
        // 既定は position/size とも Some なので NOMOVE/NOSIZE は立たず、NoChange により NOZORDER のみ
        let flags = WindowPos::default().build_flags_for_system();
        assert_eq!(flags & SWP_NOMOVE, SET_WINDOW_POS_FLAGS(0));
        assert_eq!(flags & SWP_NOSIZE, SET_WINDOW_POS_FLAGS(0));
        assert_eq!(flags & SWP_NOZORDER, SWP_NOZORDER);
    }

    #[test]
    fn test_build_flags_position_none_sets_nomove() {
        let mut pos = WindowPos::new();
        pos.position = None;
        let flags = pos.build_flags_for_system();
        assert_eq!(flags & SWP_NOMOVE, SWP_NOMOVE);
    }

    #[test]
    fn test_build_flags_size_none_sets_nosize() {
        let mut pos = WindowPos::new();
        pos.size = None;
        let flags = pos.build_flags_for_system();
        assert_eq!(flags & SWP_NOSIZE, SWP_NOSIZE);
    }

    #[test]
    fn test_build_flags_nonzero_zorder_clears_nozorder() {
        let flags = WindowPos::new().zorder_top().build_flags_for_system();
        // NoChange 以外では NOZORDER は立たない
        assert_eq!(flags & SWP_NOZORDER, SET_WINDOW_POS_FLAGS(0));
    }

    #[test]
    fn test_build_flags_maps_each_bool_to_swp_flag() {
        let pos = WindowPos::new()
            .no_redraw(true)
            .no_activate(true)
            .frame_changed(true)
            .show_window(true)
            .hide_window(true)
            .no_copy_bits(true)
            .no_owner_zorder(true)
            .no_send_changing(true)
            .defer_erase(true)
            .async_window_pos(true);
        let flags = pos.build_flags_for_system();
        assert_eq!(flags & SWP_NOREDRAW, SWP_NOREDRAW);
        assert_eq!(flags & SWP_NOACTIVATE, SWP_NOACTIVATE);
        assert_eq!(flags & SWP_FRAMECHANGED, SWP_FRAMECHANGED);
        assert_eq!(flags & SWP_SHOWWINDOW, SWP_SHOWWINDOW);
        assert_eq!(flags & SWP_HIDEWINDOW, SWP_HIDEWINDOW);
        assert_eq!(flags & SWP_NOCOPYBITS, SWP_NOCOPYBITS);
        assert_eq!(flags & SWP_NOOWNERZORDER, SWP_NOOWNERZORDER);
        assert_eq!(flags & SWP_NOSENDCHANGING, SWP_NOSENDCHANGING);
        assert_eq!(flags & SWP_DEFERERASE, SWP_DEFERERASE);
        assert_eq!(flags & SWP_ASYNCWINDOWPOS, SWP_ASYNCWINDOWPOS);
    }

    // ===== get_hwnd_insert_after (ZOrder enum mapping) =====

    #[test]
    fn test_get_hwnd_insert_after_maps_each_zorder() {
        assert_eq!(
            WindowPos::new().zorder_no_change().get_hwnd_insert_after(),
            None
        );
        assert_eq!(
            WindowPos::new().zorder_topmost().get_hwnd_insert_after(),
            Some(HWND_TOPMOST)
        );
        assert_eq!(
            WindowPos::new().zorder_notopmost().get_hwnd_insert_after(),
            Some(HWND_NOTOPMOST)
        );
        assert_eq!(
            WindowPos::new().zorder_top().get_hwnd_insert_after(),
            Some(HWND_TOP)
        );
        assert_eq!(
            WindowPos::new().zorder_bottom().get_hwnd_insert_after(),
            Some(HWND_BOTTOM)
        );
        let hwnd = HWND(0xABCD as *mut _);
        assert_eq!(
            WindowPos::new()
                .zorder_insert_after(hwnd)
                .get_hwnd_insert_after(),
            Some(hwnd)
        );
    }

    // ===== to_window_coords_for_creation (CW_USEDEFAULT passthrough — device-independent) =====

    #[test]
    fn test_to_window_coords_for_creation_passes_through_cw_usedefault() {
        // CW_USEDEFAULT を含む既定 WindowPos は AdjustWindowRectExForDpi を呼ばず素通し
        let pos = WindowPos::default();
        let (x, y, w, h) = pos.to_window_coords_for_creation(
            WINDOW_STYLE(0),
            WINDOW_EX_STYLE(0),
            96,
        );
        assert_eq!(x, CW_USEDEFAULT);
        assert_eq!(y, CW_USEDEFAULT);
        assert_eq!(w, CW_USEDEFAULT);
        assert_eq!(h, CW_USEDEFAULT);
    }

    #[test]
    fn test_to_window_coords_for_creation_passes_through_when_position_x_is_cw_usedefault() {
        // position.x のみ CW_USEDEFAULT でも素通し（size は具体値）
        let mut pos = WindowPos::new();
        pos.position = Some(Point {
            x: CW_USEDEFAULT,
            y: 0,
        });
        pos.size = Some(SizeI {
            width: 100,
            height: 100,
        });
        let (x, y, w, h) =
            pos.to_window_coords_for_creation(WINDOW_STYLE(0), WINDOW_EX_STYLE(0), 96);
        assert_eq!((x, y, w, h), (CW_USEDEFAULT, 0, 100, 100));
    }

    #[test]
    fn test_to_window_coords_for_creation_no_frame_style_is_identity() {
        // WS_POPUP（フレームなし）+ ex_style 0 では AdjustWindowRectExForDpi の調整量が 0 となり、
        // クライアント矩形＝ウィンドウ矩形（座標素通し）。実 API を呼ぶが結果は決定的。
        let mut pos = WindowPos::new();
        pos.position = Some(Point { x: 50, y: 60 });
        pos.size = Some(SizeI {
            width: 200,
            height: 150,
        });
        let (x, y, w, h) =
            pos.to_window_coords_for_creation(WS_POPUP, WINDOW_EX_STYLE(0), 96);
        assert_eq!((x, y, w, h), (50, 60, 200, 150));
    }
}
