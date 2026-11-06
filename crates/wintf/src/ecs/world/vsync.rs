//! tick+flush の再入防止ガードと、錠つきの窓の書き込みの流し

use std::cell::Cell;
use tracing::trace;

thread_local! {
    /// tick+flush の再入防止ガード。
    ///
    /// [`flush_window_pos_commands_guarded`] と `runtime::tick_bridge::tick_one_frame` の
    /// スコープ中のみ `true`。`flush_window_pos_commands()` → `guarded_set_window_pos()` →
    /// 同期 `WM_WINDOWPOSCHANGED` → 再び流しが呼ばれたとき、このフラグが `true` なら
    /// スキップする。これにより再帰的な tick-flush-tick ループ（フリーズ）を防ぐ。
    static IS_TICK_FLUSH_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
}

/// RAII ガード: スコープ離脱時に `IS_TICK_FLUSH_IN_PROGRESS` を `false` にリセット
struct TickFlushGuard;

impl Drop for TickFlushGuard {
    fn drop(&mut self) {
        IS_TICK_FLUSH_IN_PROGRESS.set(false);
    }
}

/// tick+flush 再入ガードの現在値を返す（`true` ならガードスコープ進行中）。テスト専用。
#[cfg(test)]
pub(crate) fn is_tick_flush_in_progress() -> bool {
    IS_TICK_FLUSH_IN_PROGRESS.get()
}

/// tick+flush 再入ガードを engage する RAII ハンドルを返す。
///
/// `Some(guard)` を返した場合のみガードを獲得できており（このスコープが
/// `IS_TICK_FLUSH_IN_PROGRESS` の owner）、`guard` の drop で `false` に戻る。
/// 既に進行中（`true`）なら `None` を返し、呼び出し側は安全側スキップする。
pub(crate) fn engage_tick_flush_guard() -> Option<impl Drop> {
    if IS_TICK_FLUSH_IN_PROGRESS.get() {
        return None;
    }
    IS_TICK_FLUSH_IN_PROGRESS.set(true);
    Some(TickFlushGuard)
}

/// 再入防止ガードを取って、キューに溜まった窓の書き込み（SetWindowPos）を流す。
///
/// WM_WINDOWPOSCHANGED から World の借用を解いた後に呼ぶ。流しが同期で
/// WM_WINDOWPOSCHANGED を起こして再びここへ来たときは、ガードが立っているので
/// 何もせずに返る。
pub fn flush_window_pos_commands_guarded() {
    let Some(_guard) = engage_tick_flush_guard() else {
        trace!("[flush_window_pos_commands_guarded] Re-entry blocked by IS_TICK_FLUSH_IN_PROGRESS");
        return;
    };
    crate::ecs::window::flush_window_pos_commands();
}
