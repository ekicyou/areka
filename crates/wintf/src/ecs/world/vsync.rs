//! VSYNC優先レンダリング用トレイトと再入防止ガード

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tracing::trace;

use super::EcsWorld;

thread_local! {
    /// tick+flush の再入防止ガード。
    ///
    /// `VsyncTick::try_tick_on_vsync()` のスコープ中のみ `true`。
    /// `flush_window_pos_commands()` → `guarded_set_window_pos()` → 同期 `WM_WINDOWPOSCHANGED`
    /// → 再び `try_tick_on_vsync()` が呼ばれたとき、このフラグが `true` なら
    /// tick をスキップする。これにより VSYNC カウンター更新による
    /// 再帰的 tick-flush-tick ループ（フリーズ）を防止する。
    static IS_TICK_FLUSH_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
}

/// RAII ガード: スコープ離脱時に `IS_TICK_FLUSH_IN_PROGRESS` を `false` にリセット
struct TickFlushGuard;

impl Drop for TickFlushGuard {
    fn drop(&mut self) {
        IS_TICK_FLUSH_IN_PROGRESS.set(false);
    }
}

/// tick+flush 再入ガードの現在値を返す（`true` ならガードスコープ進行中）。
///
/// `try_tick_on_vsync`（レガシー WndProc 経路）と新 `AsyncTickTask`（runtime 層）が
/// 同一の `IS_TICK_FLUSH_IN_PROGRESS` を共有し、二重に tick が走らないことを保証する
/// ための `pub(crate)` アクセサ。既存挙動は変えない（読み取りのみ）。
///
/// 現状の lib 経路では `AsyncTickTask` のテスト・後続結線でのみ参照されるため
/// dead_code を許容する（兄弟 building block と同方針）。
#[allow(dead_code)]
pub(crate) fn is_tick_flush_in_progress() -> bool {
    IS_TICK_FLUSH_IN_PROGRESS.get()
}

/// tick+flush 再入ガードを engage する RAII ハンドルを返す。
///
/// `Some(guard)` を返した場合のみガードを獲得できており（このスコープが
/// `IS_TICK_FLUSH_IN_PROGRESS` の owner）、`guard` の drop で `false` に戻る。
/// 既に進行中（`true`）なら `None` を返し、呼び出し側は安全側スキップする。
///
/// レガシー `try_tick_on_vsync` は自前で同フラグを set/guard しており（挙動不変）、
/// 本ヘルパは新 `AsyncTickTask` が同一フラグを再利用して二重防御するために提供する。
pub(crate) fn engage_tick_flush_guard() -> Option<impl Drop> {
    if IS_TICK_FLUSH_IN_PROGRESS.get() {
        return None;
    }
    IS_TICK_FLUSH_IN_PROGRESS.set(true);
    Some(TickFlushGuard)
}

/// VSYNC駆動のtick実行を提供するトレイト
///
/// `Rc<RefCell<EcsWorld>>`に実装され、WndProcから安全にtickを呼び出せる。
/// 借用失敗時（再入時）は安全にスキップしてfalseを返す。
///
/// # 設計意図
/// - WndProc（特にWM_WINDOWPOSCHANGED）からのVSYNC駆動tick実行を可能にする
/// - モーダルループ（ウィンドウドラッグ等）中でも描画を継続する
/// - RefCellの借用状態を確認し、再入時は安全にスキップする
///
/// # 拡張ポイント
/// 他のモーダルループ関連メッセージで同様の問題が発見された場合、
/// 該当メッセージ処理でこのトレイトのメソッドを呼び出すだけで対応可能。
pub trait VsyncTick {
    /// VSYNCカウンターの変化を検知し、必要に応じてworld tickを実行
    ///
    /// # Returns
    /// - `true`: tickが実行された
    /// - `false`: tickがスキップされた（借用失敗またはカウンター変化なし）
    fn try_tick_on_vsync(&self) -> bool;
}

impl VsyncTick for Rc<RefCell<EcsWorld>> {
    fn try_tick_on_vsync(&self) -> bool {
        // ── 再入防止ガード ──────────────────────────────
        // flush_window_pos_commands() → guarded_set_window_pos() が同期で
        // WM_WINDOWPOSCHANGED を発火し、そこから再び try_tick_on_vsync() が
        // 呼ばれる。VSYNC カウンターは別スレッドで ~16ms ごとに進むため、
        // DPI 変更など重い tick（>16ms）ではカウンターが毎回進行し、
        //   tick → flush → SetWindowPos → WM_WINDOWPOSCHANGED → tick → …
        // という再帰的ループでフリーズが発生する。
        // IS_TICK_FLUSH_IN_PROGRESS フラグで tick+flush スコープ全体を
        // ガードし、再入時は即座にスキップする。
        if IS_TICK_FLUSH_IN_PROGRESS.get() {
            trace!("[try_tick_on_vsync] Re-entry blocked by IS_TICK_FLUSH_IN_PROGRESS");
            return false;
        }
        IS_TICK_FLUSH_IN_PROGRESS.set(true);
        let _guard = TickFlushGuard;

        // RefCellの借用を試みる
        // 既に借用されている場合（再入時）は安全にスキップ
        let result = match self.try_borrow_mut() {
            Ok(mut world) => {
                let result = world.try_tick_on_vsync();

                // デバッグビルドのみ: WndProc経由のtick回数をカウント
                #[cfg(debug_assertions)]
                if result {
                    use crate::win_thread_mgr::DEBUG_WNDPROC_TICK_COUNT;
                    use std::sync::atomic::Ordering;
                    DEBUG_WNDPROC_TICK_COUNT.fetch_add(1, Ordering::Relaxed);
                }

                result
            }
            Err(_) => {
                // 借用失敗（再入時）- 安全にスキップ
                // これは正常な動作であり、エラーではない
                false
            }
        };
        // world借用スコープ終了

        // World借用解放後にSetWindowPosコマンドをフラッシュ
        // これにより、apply_window_pos_changesでキューに追加されたコマンドが
        // 安全に実行される（WM_WINDOWPOSCHANGEDがWorldを借用しても競合しない）
        crate::ecs::window::flush_window_pos_commands();

        result
        // _guard がドロップされ IS_TICK_FLUSH_IN_PROGRESS = false に戻る
    }
}
