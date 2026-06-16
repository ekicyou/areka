//! SetWindowPos コマンドキュー・ガード
//!
//! - `is_self_initiated`: 自アプリ由来の SetWindowPos 呼び出し判定
//! - `guarded_set_window_pos`: RAII ガード付き SetWindowPos ラッパー
//! - `SetWindowPosCommand`: SetWindowPos 遅延実行キュー
//! - `find_owner_window`: エンティティの所属ウィンドウ特定

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicI32, Ordering};
use tracing::{trace, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::components::Window;

// ============================================================================
// SELF_INITIATED_DEPTH - SetWindowPos ネストカウンタ (AtomicI32)
// ============================================================================

/// `guarded_set_window_pos` のネスト深度カウンタ。
///
/// 0 より大きい場合、自アプリ由来の `SetWindowPos` 呼び出し中であることを示す。
/// `SetWindowPos` → `WM_WINDOWPOSCHANGED` は同期的に発火するため、
/// Relaxed ordering で十分。
///
/// ## ライフサイクル
/// 1. `guarded_set_window_pos()` 呼び出し → カウンタ +1
/// 2. `SetWindowPos` Win32 API 呼び出し（同期的に `WM_WINDOWPOSCHANGED` が発火）
/// 3. ハンドラ内で `is_self_initiated()` を参照 → カウンタ > 0 なら echo
/// 4. `SetWindowPosGuard` の Drop でカウンタ -1（RAII 保証）
static SELF_INITIATED_DEPTH: AtomicI32 = AtomicI32::new(0);

/// 現在 `guarded_set_window_pos` 呼び出しスコープ内かどうかを返す。
///
/// `WM_WINDOWPOSCHANGED` ハンドラ内で echo 判定に使用する。
/// `true` の場合、自アプリの `guarded_set_window_pos()` 経由の呼び出しであり、
/// `apply_window_pos_changes` での再送信は不要。
pub fn is_self_initiated() -> bool {
    SELF_INITIATED_DEPTH.load(Ordering::Relaxed) > 0
}

/// RAII ガード: スコープ終了時にネストカウンタをデクリメントする。
///
/// `guarded_set_window_pos()` 内で使用され、正常終了・`?` early return・
/// パニック時のいずれでもカウンタが確実に復元されることを保証する。
struct SetWindowPosGuard;

impl SetWindowPosGuard {
    fn new() -> Self {
        SELF_INITIATED_DEPTH.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for SetWindowPosGuard {
    fn drop(&mut self) {
        let prev = SELF_INITIATED_DEPTH.fetch_sub(1, Ordering::Relaxed);
        trace!(
            depth = prev - 1,
            "SELF_INITIATED_DEPTH decremented by guard"
        );
    }
}

/// `SetWindowPos` をラッパー付きで呼び出す。
///
/// RAII Drop guard により、正常終了・`?` early return・パニック時も
/// ネストカウンタが確実にデクリメントされる。
/// `SetWindowPos` → `WM_WINDOWPOSCHANGED` は同期発火のため、
/// ハンドラ内で `is_self_initiated()` を参照して echo を判定できる。
///
/// # Safety
/// `SetWindowPos` Win32 API の unsafe 呼び出しを内包する。
///
/// # Arguments
/// * `hwnd` - 対象ウィンドウハンドル
/// * `hwnd_insert_after` - Z-order 挿入位置（`None` で変更なし）
/// * `x`, `y` - ウィンドウ左上座標
/// * `cx`, `cy` - ウィンドウ幅・高さ
/// * `flags` - `SET_WINDOW_POS_FLAGS`
pub unsafe fn guarded_set_window_pos(
    hwnd: HWND,
    hwnd_insert_after: Option<HWND>,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    flags: SET_WINDOW_POS_FLAGS,
) -> windows::core::Result<()> {
    let _guard = SetWindowPosGuard::new(); // Drop でカウンタ -1

    trace!(
        hwnd = format!("0x{:X}", hwnd.0 as usize),
        x = x, y = y, cx = cx, cy = cy,
        flags = ?flags,
        "[guarded_set_window_pos] Calling SetWindowPos"
    );

    unsafe { SetWindowPos(hwnd, hwnd_insert_after, x, y, cx, cy, flags) }?;
    Ok(())
}

// ============================================================================
// SetWindowPosCommand - SetWindowPos 遅延実行キュー
// ============================================================================

/// SetWindowPosコマンド
///
/// `apply_window_pos_changes`システムから直接`SetWindowPos`を呼び出さず、
/// キューに追加して`tick`後に遅延実行することで、World借用競合を防止する。
#[derive(Debug, Clone)]
pub struct SetWindowPosCommand {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub flags: SET_WINDOW_POS_FLAGS,
    pub hwnd_insert_after: Option<HWND>,
}

thread_local! {
    /// SetWindowPosコマンドキュー
    static WINDOW_POS_COMMANDS: RefCell<Vec<SetWindowPosCommand>> = const { RefCell::new(Vec::new()) };
}

impl SetWindowPosCommand {
    /// 新しいSetWindowPosCommandを作成
    pub fn new(
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: SET_WINDOW_POS_FLAGS,
        hwnd_insert_after: Option<HWND>,
    ) -> Self {
        Self {
            hwnd,
            x,
            y,
            width,
            height,
            flags,
            hwnd_insert_after,
        }
    }

    /// コマンドをキューに追加
    pub fn enqueue(cmd: SetWindowPosCommand) {
        trace!(
            hwnd = ?cmd.hwnd,
            x = cmd.x,
            y = cmd.y,
            width = cmd.width,
            height = cmd.height,
            "SetWindowPosCommand::enqueue"
        );
        WINDOW_POS_COMMANDS.with(|cell| {
            cell.borrow_mut().push(cmd);
        });
    }

    /// キュー内の全コマンドを実行し、キューをクリア
    ///
    /// World借用解放後に呼び出すこと。
    /// 内部で `guarded_set_window_pos()` を使用し、TLS フラグによるフィードバック防止を適用する。
    pub fn flush() {
        WINDOW_POS_COMMANDS.with(|cell| {
            let commands: Vec<_> = cell.borrow_mut().drain(..).collect();
            if commands.is_empty() {
                return;
            }
            trace!(
                count = commands.len(),
                "SetWindowPosCommand::flush processing"
            );
            for cmd in commands {
                let result = unsafe {
                    guarded_set_window_pos(
                        cmd.hwnd,
                        cmd.hwnd_insert_after,
                        cmd.x,
                        cmd.y,
                        cmd.width,
                        cmd.height,
                        cmd.flags,
                    )
                };
                if let Err(e) = result {
                    warn!(
                        hwnd = ?cmd.hwnd,
                        error = ?e,
                        "SetWindowPos failed"
                    );
                } else {
                    trace!(hwnd = ?cmd.hwnd, "SetWindowPos succeeded");
                }
            }
        });
    }
}

/// SetWindowPosコマンドキューをフラッシュする便利関数
pub fn flush_window_pos_commands() {
    SetWindowPosCommand::flush();
}

// ============================================================================
// find_owner_window - エンティティの所属ウィンドウ特定
// ============================================================================

/// エンティティが所属する Window エンティティを返す。
///
/// ChildOf チェーンを辿り、Window コンポーネントを持つ最初の祖先で停止する。
/// エンティティ自身が Window の場合は `Some(entity)` を返す。
/// ChildOf を持たないエンティティ（LayoutRoot 等）に到達した場合は `None` を返す。
///
/// # Arguments
/// * `world` - ECS World 参照（読み取り専用）
/// * `entity` - 所属ウィンドウを検索するエンティティ
///
/// # Returns
/// 所属する Window エンティティ。Window が見つからない場合は `None`。
pub fn find_owner_window(world: &World, entity: Entity) -> Option<Entity> {
    // エンティティ自身が Window を持つ場合は自身を返す
    if world.get::<Window>(entity).is_some() {
        return Some(entity);
    }

    // ChildOf チェーンを辿る
    let mut current = entity;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        let parent = child_of.parent();
        if world.get::<Window>(parent).is_some() {
            return Some(parent);
        }
        current = parent;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_window_pos_command_new_stores_all_fields() {
        let hwnd = HWND(0x1000 as *mut _);
        let after = HWND(0x2000 as *mut _);
        let cmd = SetWindowPosCommand::new(
            hwnd,
            10,
            20,
            300,
            400,
            SWP_NOACTIVATE,
            Some(after),
        );
        assert_eq!(cmd.hwnd, hwnd);
        assert_eq!(cmd.x, 10);
        assert_eq!(cmd.y, 20);
        assert_eq!(cmd.width, 300);
        assert_eq!(cmd.height, 400);
        assert_eq!(cmd.flags, SWP_NOACTIVATE);
        assert_eq!(cmd.hwnd_insert_after, Some(after));
    }

    #[test]
    fn test_set_window_pos_command_new_allows_none_insert_after() {
        let cmd = SetWindowPosCommand::new(
            HWND(0x1 as *mut _),
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(0),
            None,
        );
        assert_eq!(cmd.hwnd_insert_after, None);
    }

    #[test]
    fn test_is_self_initiated_false_at_rest() {
        // guarded_set_window_pos スコープ外（ネストカウンタ 0）では false
        // 注: SELF_INITIATED_DEPTH はプロセス共有の AtomicI32 だが、
        // テストスレッドで guarded_set_window_pos を呼ばない限り 0 のまま。
        assert!(!is_self_initiated());
    }

    #[test]
    fn test_flush_empty_queue_is_noop() {
        // 空キューの flush は early-return で SetWindowPos を呼ばずパニックしない。
        // （このテスト内では enqueue していないため WINDOW_POS_COMMANDS は空。
        //  thread_local かつ同一テストスレッドのため他テストの enqueue 残留はない）
        SetWindowPosCommand::flush();
        // 便利関数経由でも同様に no-op
        flush_window_pos_commands();
        assert!(!is_self_initiated());
    }
}
