//! アプリケーション全体の状態を管理するリソース

use bevy_ecs::prelude::*;
use tracing::{debug, info};

/// アプリケーション全体の状態を管理するリソース
#[derive(Resource, Default)]
pub struct App {
    window_count: usize,
    display_configuration_changed: bool,
}

impl App {
    /// 新しいAppリソースを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// ディスプレイ構成が変更されたことをマーク
    pub fn mark_display_change(&mut self) {
        // 表示構成が変わった＝幾何をやり直す仕事がある（設計 C16 の `WM_GEOMETRY`）。
        crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::WM_GEOMETRY);
        self.display_configuration_changed = true;
        info!("[App] Display configuration changed");
    }

    /// ディスプレイ構成変更フラグをリセット
    pub fn reset_display_change(&mut self) {
        self.display_configuration_changed = false;
    }

    /// ディスプレイ構成が変更されたかどうかを取得
    pub fn display_configuration_changed(&self) -> bool {
        self.display_configuration_changed
    }

    /// ウィンドウが作成されたときに呼ばれる
    pub fn on_window_created(&mut self, entity: Entity) {
        self.window_count += 1;
        debug!(
            entity = ?entity,
            total_windows = self.window_count,
            "[App] Window created"
        );
    }

    /// ウィンドウが破棄されたときに呼ばれる
    /// 最後のウィンドウが閉じられた場合はtrueを返す
    ///
    /// NOTE: 新 `WinApp` 経路のアプリ終了は `WindowRegistry` の空遷移
    /// （`reconcile_window_registry` → `ShutdownPolicy::notify_shutdown`）が駆動する。
    /// 旧 `WM_LAST_WINDOW_DESTROYED` を message_window へ PostMessage する経路は撤去した
    /// （task 4.5）。本メソッドは window_count の追跡と「最後のウィンドウか」の判定のみを担う。
    pub fn on_window_destroyed(&mut self, entity: Entity) -> bool {
        self.window_count = self.window_count.saturating_sub(1);
        debug!(
            entity = ?entity,
            remaining_windows = self.window_count,
            "[App] Window destroyed"
        );

        if self.window_count == 0 {
            info!("[App] Last window closed.");
            true
        } else {
            false
        }
    }

    /// 現在のウィンドウ数を取得
    pub fn window_count(&self) -> usize {
        self.window_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用のダミー Entity（World 不要）。
    fn entity(idx: u32) -> Entity {
        Entity::from_raw_u32(idx).expect("valid entity index")
    }

    /// `App::default` / `App::new` の初期状態を固定する。
    /// window_count=0・display フラグ false（message_window は private のため
    /// 外部観測可能な display_configuration_changed / window_count で検証）。
    #[test]
    fn test_default_and_new_start_with_zero_windows_and_no_display_change() {
        let from_default = App::default();
        assert_eq!(from_default.window_count(), 0);
        assert!(!from_default.display_configuration_changed());

        let from_new = App::new();
        assert_eq!(from_new.window_count(), 0);
        assert!(!from_new.display_configuration_changed());
    }

    /// `on_window_created` が呼び出しごとに window_count を 1 ずつ増やす。
    #[test]
    fn test_on_window_created_increments_window_count() {
        let mut app = App::new();

        app.on_window_created(entity(1));
        assert_eq!(app.window_count(), 1);

        app.on_window_created(entity(2));
        assert_eq!(app.window_count(), 2);

        app.on_window_created(entity(3));
        assert_eq!(app.window_count(), 3);
    }

    /// 残りウィンドウがある間の `on_window_destroyed` は count を 1 減らし false を返す
    /// （= 最後のウィンドウではない）。
    #[test]
    fn test_on_window_destroyed_decrements_and_returns_false_while_windows_remain() {
        let mut app = App::new();
        app.on_window_created(entity(1));
        app.on_window_created(entity(2));
        assert_eq!(app.window_count(), 2);

        let was_last = app.on_window_destroyed(entity(2));
        assert!(!was_last, "破棄後も1ウィンドウ残るため false");
        assert_eq!(app.window_count(), 1);
    }

    /// 最後のウィンドウ破棄で count が 0 になり true を返す（最後のウィンドウ検出）。
    #[test]
    fn test_on_window_destroyed_returns_true_when_last_window_closed() {
        let mut app = App::new();
        app.on_window_created(entity(1));
        assert_eq!(app.window_count(), 1);

        let was_last = app.on_window_destroyed(entity(1));
        assert!(was_last, "最後のウィンドウ破棄で true");
        assert_eq!(app.window_count(), 0);
    }

    /// window_count=0 での `on_window_destroyed` は saturating_sub により 0 に留まり
    /// （アンダーフローしない）、count==0 のため true を返す。
    #[test]
    fn test_on_window_destroyed_saturates_at_zero_and_returns_true() {
        let mut app = App::new();
        assert_eq!(app.window_count(), 0);

        let was_last = app.on_window_destroyed(entity(7));
        assert!(
            was_last,
            "0 から破棄しても window_count==0 のため true（最後のウィンドウ扱い）"
        );
        assert_eq!(
            app.window_count(),
            0,
            "saturating_sub でアンダーフローしない"
        );
    }

    /// 作成・破棄の混在シーケンスで window_count が正しく増減し、
    /// 0 到達時のみ true を返す（増減ロジックの累積検証）。
    #[test]
    fn test_window_count_tracks_mixed_create_destroy_sequence() {
        let mut app = App::new();

        app.on_window_created(entity(1));
        app.on_window_created(entity(2));
        app.on_window_created(entity(3));
        assert_eq!(app.window_count(), 3);

        assert!(!app.on_window_destroyed(entity(3)));
        assert_eq!(app.window_count(), 2);

        // 破棄途中の再作成で count は再び増える
        app.on_window_created(entity(4));
        assert_eq!(app.window_count(), 3);

        assert!(!app.on_window_destroyed(entity(4)));
        assert!(!app.on_window_destroyed(entity(2)));
        assert_eq!(app.window_count(), 1);

        // 最後の1つの破棄で 0 到達・true
        assert!(app.on_window_destroyed(entity(1)));
        assert_eq!(app.window_count(), 0);
    }
}
