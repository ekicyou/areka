//! ドラッグコンテキスト（配りから窓の手続きへの受け渡し用）
//!
//! dispatch_drag_events がドラッグ開始時にWindowDragContextを書き込み、
//! update_dragging がJustStarted→Dragging遷移時に読み取る（どちらも同じ UI スレッド）。

use bevy_ecs::prelude::*;
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::HWND;

use super::DragConstraint;
use crate::ecs::Point;

/// ドラッグに必要なWindow情報（配りが書き、update_dragging が読む）
#[derive(Debug, Clone, Default)]
pub struct WindowDragContext {
    /// Window の Win32 ハンドル
    pub hwnd: Option<HWND>,
    /// ドラッグ開始時のウィンドウ位置（ウィンドウ枠を含むスクリーン座標、SetWindowPos用）
    pub initial_window_pos: Option<Point>,
    /// DragConfig.move_window のキャッシュ
    pub move_window: bool,
    /// DragConstraint のキャッシュ
    pub constraint: Option<DragConstraint>,
}

// HWND は *mut c_void を含むため自動的に Send/Sync を実装しないが、
// Resource に置くには Send/Sync が要る。ウィンドウハンドルは実質的にただの整数値で、読み書きはどれも同じ UI スレッド。
// Arc<Mutex> でラップされているため、データ競合も発生しない。
unsafe impl Send for WindowDragContext {}
unsafe impl Sync for WindowDragContext {}

impl WindowDragContext {
    /// コンテキストをクリアする
    pub fn clear(&mut self) {
        self.hwnd = None;
        self.initial_window_pos = None;
        self.move_window = false;
        self.constraint = None;
    }
}

/// ドラッグ開始時のWindow情報を配りから窓の手続きへ渡すリソース（同じ UI スレッド）
///
/// # ライフサイクル
/// - dispatch_drag_events が Started 処理時に更新
/// - update_dragging が JustStarted → Dragging 遷移時に読み取り
#[derive(Resource, Clone)]
pub struct WindowDragContextResource {
    inner: Arc<Mutex<WindowDragContext>>,
}

impl WindowDragContextResource {
    /// 新しいWindowDragContextResourceを作成
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(WindowDragContext::default())),
        }
    }

    /// コンテキストを設定（配りの dispatch_drag_events から呼ばれる）
    pub fn set(&self, context: WindowDragContext) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner = context;
        }
    }

    /// コンテキストを読み取る（update_dragging から呼ばれる。窓の手続きの WM_MOUSEMOVE と配りの両方が呼ぶ）
    pub fn get(&self) -> Option<WindowDragContext> {
        self.inner.lock().ok().map(|inner| inner.clone())
    }

    /// コンテキストをクリアする
    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.clear();
        }
    }
}

impl Default for WindowDragContextResource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default は全フィールド「未設定」状態（hwnd/initial_window_pos=None, move_window=false, constraint=None）。
    #[test]
    fn test_default_is_unset() {
        let ctx = WindowDragContext::default();
        assert!(ctx.hwnd.is_none());
        assert!(ctx.initial_window_pos.is_none());
        assert!(!ctx.move_window);
        assert!(ctx.constraint.is_none());
    }

    /// clear() は設定済みコンテキストを Default 相当に戻す。
    #[test]
    fn test_clear_resets_all_fields() {
        let mut ctx = WindowDragContext {
            hwnd: Some(HWND::default()),
            initial_window_pos: Some(Point { x: 10, y: 20 }),
            move_window: true,
            constraint: Some(DragConstraint {
                min_x: Some(0),
                max_x: Some(100),
                min_y: Some(0),
                max_y: Some(100),
            }),
        };
        ctx.clear();
        assert!(ctx.hwnd.is_none());
        assert!(ctx.initial_window_pos.is_none());
        assert!(!ctx.move_window);
        assert!(ctx.constraint.is_none());
    }

    /// Resource: set した値が get で読み取れる（initial_window_pos/move_window の往復）。
    #[test]
    fn test_resource_set_then_get_roundtrip() {
        let res = WindowDragContextResource::new();
        res.set(WindowDragContext {
            hwnd: None,
            initial_window_pos: Some(Point { x: 300, y: 400 }),
            move_window: true,
            constraint: None,
        });

        let got = res.get().expect("get は Some を返すべき");
        assert_eq!(got.initial_window_pos, Some(Point { x: 300, y: 400 }));
        assert!(got.move_window);
    }

    /// Resource: 新規作成直後は Default（move_window=false 等）が読み出される。
    #[test]
    fn test_resource_new_returns_default() {
        let res = WindowDragContextResource::new();
        let got = res.get().expect("get は Some を返すべき");
        assert!(got.hwnd.is_none());
        assert!(!got.move_window);
        assert!(got.initial_window_pos.is_none());
    }

    /// Resource: clear() 後に get するとフィールドが未設定へ戻る。
    #[test]
    fn test_resource_clear_resets() {
        let res = WindowDragContextResource::new();
        res.set(WindowDragContext {
            hwnd: Some(HWND::default()),
            initial_window_pos: Some(Point { x: 1, y: 2 }),
            move_window: true,
            constraint: None,
        });
        res.clear();
        let got = res.get().expect("get は Some を返すべき");
        assert!(got.initial_window_pos.is_none());
        assert!(!got.move_window);
    }
}
