//! クリック透過機構の監視対象レジストリ。
//!
//! 監視対象ウィンドウ（window Entity ＋ 対応 HWND）と、その適用済み透過状態
//! （`last_applied`）を保持する。差分ガード（同一状態は再適用しない・R3.2）の
//! 状態基盤であり、初期 `last_applied` は不透過（`Opaque`）。
//!
//! UI スレッド単独所有（`&World` と同居）を前提とし、ワーカスレッドとは共有しない。
//! したがってロック・アトミックを持たない（単一スレッド更新・参照）。

use bevy_ecs::entity::Entity;
use windows::Win32::Foundation::HWND;

/// クリック透過の適用済み／適用予定状態。
///
/// レジストリと（後続タスクの）コントローラで共有する。`Opaque` は
/// 初回トグル前の初期・既定状態（当該窓は自ウィンドウでクリックを受領する）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesiredState {
    /// クリック透過 ON（`WS_EX_TRANSPARENT` 付与相当）。
    Transparent,
    /// クリック透過 OFF（自ウィンドウで受領・既定）。
    Opaque,
}

impl Default for DesiredState {
    fn default() -> Self {
        Self::Opaque
    }
}

/// 監視対象 1 窓分のレコード。window Entity・対応 HWND・適用済み状態を束ねる。
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClickThroughTarget {
    /// 監視対象の window Entity。
    pub(crate) window: Entity,
    /// 対応する HWND（ex-style 適用先）。
    pub(crate) hwnd: HWND,
    /// 直近に適用済みの透過状態（差分ガードの基準・初期 `Opaque`）。
    pub(crate) last_applied: DesiredState,
    /// `WS_EX_LAYERED` 同伴フラグを適用済みか（初期 `false`・pilot 必須条件）。
    ///
    /// DComp 窓は factory（`compute_ex_style`）が生成時に LAYERED を除去するため、
    /// `WS_EX_TRANSPARENT` 単独ではマウス透過が効かない（pilot REPORT 実証）。機構が
    /// 登録窓の初回評価時に `apply_layered_companion` で 1 回立て、成功時に真へ倒す。
    pub(crate) layered_applied: bool,
}

/// 監視対象窓と適用済み状態を保持するレジストリ。
///
/// UI スレッド所有。汎用機構として複数窓を巡回可能（areka は shell/balloon の 2 窓）。
#[derive(Debug, Default)]
pub(crate) struct ClickThroughRegistry {
    targets: Vec<ClickThroughTarget>,
}

impl ClickThroughRegistry {
    /// 空のレジストリを生成する。
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 監視対象窓を登録する。`last_applied` は既定（`Opaque`）、`layered_applied` は
    /// `false` で初期化する。
    ///
    /// 同一 window Entity が既に登録済みの場合は HWND を更新し `last_applied`／
    /// `layered_applied` をリセットして重複させない（dedupe by Entity。HWND が
    /// 変わり得るため同伴フラグも再適用が必要）。
    pub(crate) fn register(&mut self, window: Entity, hwnd: HWND) {
        if let Some(target) = self.targets.iter_mut().find(|t| t.window == window) {
            target.hwnd = hwnd;
            target.last_applied = DesiredState::default();
            target.layered_applied = false;
        } else {
            self.targets.push(ClickThroughTarget {
                window,
                hwnd,
                last_applied: DesiredState::default(),
                layered_applied: false,
            });
        }
    }

    /// 監視対象窓を除去する（ウィンドウライフサイクル追随・R7.2 非破壊）。
    ///
    /// 対象が存在すれば `true`、無ければ `false` を返す。
    pub(crate) fn remove(&mut self, window: Entity) -> bool {
        let before = self.targets.len();
        self.targets.retain(|t| t.window != window);
        self.targets.len() != before
    }

    /// 登録済み対象窓数。
    pub(crate) fn len(&self) -> usize {
        self.targets.len()
    }

    /// 登録済み対象が無いか。
    pub(crate) fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// 指定 window の `last_applied` を読む（未登録なら `None`）。
    pub(crate) fn last_applied(&self, window: Entity) -> Option<DesiredState> {
        self.targets
            .iter()
            .find(|t| t.window == window)
            .map(|t| t.last_applied)
    }

    /// 指定 window の `last_applied` を更新する（未登録なら何もせず `false`）。
    ///
    /// 差分ガードの真実源更新はこの 1 経路に限定する（適用成功後の書き戻し）。
    pub(crate) fn set_last_applied(&mut self, window: Entity, state: DesiredState) -> bool {
        if let Some(target) = self.targets.iter_mut().find(|t| t.window == window) {
            target.last_applied = state;
            true
        } else {
            false
        }
    }

    /// 指定 window の `layered_applied` を真へ倒す（同伴フラグ適用成功後の書き戻し・
    /// 未登録なら何もせず `false`）。
    pub(crate) fn mark_layered_applied(&mut self, window: Entity) -> bool {
        if let Some(target) = self.targets.iter_mut().find(|t| t.window == window) {
            target.layered_applied = true;
            true
        } else {
            false
        }
    }

    /// 全対象を巡回参照する（コントローラが各窓を順次判定するため）。
    pub(crate) fn iter(&self) -> impl Iterator<Item = &ClickThroughTarget> {
        self.targets.iter()
    }

    /// 述語が `false` を返す対象を一括除去する（窓破棄追随のバルク版・R7.2 非破壊）。
    ///
    /// `remove` は Entity 指定の単発除去だが、本メソッドは「World 上に生存しない窓」を
    /// まとめて刈り取る（`prune_dead_targets` が World 参照で述語を与える）。除去件数を返す。
    pub(crate) fn retain<F: FnMut(&ClickThroughTarget) -> bool>(&mut self, mut keep: F) -> usize {
        let before = self.targets.len();
        self.targets.retain(|t| keep(t));
        before - self.targets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;
    use windows::Win32::Foundation::HWND;

    fn hwnd(v: isize) -> HWND {
        HWND(v as *mut core::ffi::c_void)
    }

    #[test]
    fn desired_state_default_is_opaque() {
        assert_eq!(DesiredState::default(), DesiredState::Opaque);
    }

    #[test]
    fn freshly_registered_target_is_opaque() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg = ClickThroughRegistry::new();
        reg.register(e, hwnd(1));

        assert_eq!(reg.last_applied(e), Some(DesiredState::Opaque));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn register_then_remove() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg = ClickThroughRegistry::new();

        assert!(reg.is_empty());
        reg.register(e, hwnd(1));
        assert!(!reg.is_empty());
        assert!(reg.last_applied(e).is_some());

        assert!(reg.remove(e));
        assert!(reg.is_empty());
        assert_eq!(reg.last_applied(e), None);
        // 二重除去は false。
        assert!(!reg.remove(e));
    }

    #[test]
    fn re_registering_same_entity_does_not_duplicate() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg = ClickThroughRegistry::new();

        reg.register(e, hwnd(1));
        // 状態を進めてから再登録。
        assert!(reg.set_last_applied(e, DesiredState::Transparent));
        assert!(reg.mark_layered_applied(e));
        reg.register(e, hwnd(2));

        assert_eq!(reg.len(), 1);
        // 再登録は HWND を更新し last_applied を Opaque へ・layered_applied を false へリセットする。
        assert_eq!(reg.last_applied(e), Some(DesiredState::Opaque));
        assert_eq!(reg.iter().next().map(|t| t.hwnd), Some(hwnd(2)));
        assert_eq!(
            reg.iter().next().map(|t| t.layered_applied),
            Some(false),
            "再登録（HWND 更新）で同伴フラグは再適用が必要"
        );
    }

    #[test]
    fn layered_applied_defaults_false_and_marks_true() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg = ClickThroughRegistry::new();
        reg.register(e, hwnd(1));

        assert_eq!(
            reg.iter().next().map(|t| t.layered_applied),
            Some(false),
            "初期は未適用（false）"
        );
        assert!(reg.mark_layered_applied(e));
        assert_eq!(reg.iter().next().map(|t| t.layered_applied), Some(true));

        // 未登録 Entity への mark は false。
        let other = world.spawn_empty().id();
        assert!(!reg.mark_layered_applied(other));
    }

    #[test]
    fn set_last_applied_is_reflected_on_read() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg = ClickThroughRegistry::new();
        reg.register(e, hwnd(1));

        assert!(reg.set_last_applied(e, DesiredState::Transparent));
        assert_eq!(reg.last_applied(e), Some(DesiredState::Transparent));

        // 未登録 Entity への更新は false。
        let other = world.spawn_empty().id();
        assert!(!reg.set_last_applied(other, DesiredState::Transparent));
    }

    #[test]
    fn iterates_multiple_targets() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let mut reg = ClickThroughRegistry::new();
        reg.register(a, hwnd(10));
        reg.register(b, hwnd(20));

        assert_eq!(reg.len(), 2);
        let windows: Vec<Entity> = reg.iter().map(|t| t.window).collect();
        assert!(windows.contains(&a));
        assert!(windows.contains(&b));
    }
}
