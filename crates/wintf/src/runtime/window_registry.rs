//! ウィンドウ所有・寿命管理層（`WindowRegistry`）。
//!
//! 生成済み `Window<WndState>`（`Drop` で `DestroyWindow` を呼ぶ `!Send` 型）を Entity キーで
//! 保持する NonSend リソース。`!Send` 性が `DestroyWindow` のスレッドアフィニティ（UI スレッド
//! 束縛）を守る不変条件なので、`unsafe impl Send` で偽装しない。
//!
//! 配送は `WndState`（[`crate::runtime::wndproc_bridge`]）が担い、寿命・最後の窓検知・
//! 終了起点は本レジストリが担う。
//!
//! # スケジュール結線
//! [`reconcile_window_registry`] を tick スケジュールへ登録するのは `WinApp` で、ここでは
//! 登録しない（ECS→runtime の上向き依存を schedule に持ち込まない）。空遷移で発火する
//! フックは `WinApp` が注入し、終了の合図（[`crate::AppExit`]）を撃つ。

use std::collections::HashMap;

use crate::executor::util::Window as LibWindow;
use bevy_ecs::prelude::{Entity, RemovedComponents};
use bevy_ecs::system::NonSendMut;

use crate::ecs::window::Window;
use crate::runtime::wndproc_bridge::WndState;

/// 空への遷移時に発火する終了起点フック。
type ShutdownHook = Box<dyn Fn()>;

/// 生成済みウィンドウハンドルを Entity キーで保持する NonSend リソース。
///
/// 既定の `W = Window<WndState>` が本番型。`W` は headless テストでドロップ追跡ダミーを
/// 差し込むための型引数。
pub(crate) struct WindowRegistry<W = LibWindow<WndState>> {
    /// Entity → 生成済みウィンドウ。`remove`/drop で `DestroyWindow` が走る。
    map: HashMap<Entity, W>,
    /// 空遷移で発火する終了起点フック（未注入なら `None`）。
    shutdown_hook: Option<ShutdownHook>,
}

impl<W> WindowRegistry<W> {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            shutdown_hook: None,
        }
    }

    /// 空遷移で発火する終了起点フックを差し替える。
    pub(crate) fn set_shutdown_hook(&mut self, hook: impl Fn() + 'static) {
        self.shutdown_hook = Some(Box::new(hook));
    }

    /// 生成済みウィンドウを Entity キーで挿入する（同一キーは置換）。
    pub(crate) fn insert(&mut self, entity: Entity, window: W) -> Option<W> {
        self.map.insert(entity, window)
    }

    /// Entity キーで要素を取り出す（呼び出し側が drop すると `DestroyWindow`）。
    pub(crate) fn remove(&mut self, entity: Entity) -> Option<W> {
        self.map.remove(&entity)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 注入済み shutdown hook を発火する。本番型の registry は実 HWND が要り headless で
    /// 空遷移させられないので、`WinApp::new()` の hook 注入をテストで確かめる seam。
    #[cfg(test)]
    pub(crate) fn fire_shutdown_hook(&self) {
        if let Some(hook) = self.shutdown_hook.as_ref() {
            hook();
        }
    }
}

/// `Window` コンポーネント破棄を検知して registry 要素を drop するリコンサイルシステム
/// （drop → `DestroyWindow` → `WM_NCDESTROY` で、ハンドル破棄を Entity 寿命に一致させる）。
///
/// 1 件以上を実際に除去し、その結果 registry が空になった場合にのみ shutdown フックを
/// 発火する（元から空での no-op 除去では発火しない）。
pub(crate) fn reconcile_window_registry<W: 'static>(
    mut removed: RemovedComponents<Window>,
    mut registry: NonSendMut<WindowRegistry<W>>,
) {
    let mut removed_any = false;
    for entity in removed.read() {
        if registry.remove(entity).is_some() {
            removed_any = true;
        }
    }

    if removed_any && registry.is_empty() {
        if let Some(hook) = registry.shutdown_hook.as_ref() {
            hook();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    use bevy_ecs::world::World;

    use crate::ecs::window::Window;

    /// drop を観測するダミー保持値（実 HWND 不要・headless）。
    struct DropTracker {
        dropped: Rc<Cell<bool>>,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    /// insert + remove が Entity キーで round-trip する。
    #[test]
    fn insert_remove_roundtrips_by_entity() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg: WindowRegistry<DropTracker> = WindowRegistry::new();
        let flag = Rc::new(Cell::new(false));
        reg.insert(
            e,
            DropTracker {
                dropped: flag.clone(),
            },
        );
        assert!(!reg.is_empty());
        let got = reg.remove(e);
        assert!(
            got.is_some(),
            "remove は Entity キーで挿入済み要素を返すべき"
        );
        assert!(reg.is_empty());
        assert!(!flag.get());
        drop(got);
        assert!(flag.get(), "remove で取り出した値の drop が観測されるべき");
    }

    /// entity の remove で保持値が drop される。
    #[test]
    fn remove_drops_held_value() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg: WindowRegistry<DropTracker> = WindowRegistry::new();
        let flag = Rc::new(Cell::new(false));
        reg.insert(
            e,
            DropTracker {
                dropped: flag.clone(),
            },
        );
        assert!(!flag.get());
        drop(reg.remove(e));
        assert!(flag.get(), "registry 内要素の remove で Drop が走るべき");
    }

    fn run_reconcile(world: &mut World) {
        use bevy_ecs::schedule::Schedule;
        let mut sched = Schedule::default();
        sched.add_systems(reconcile_window_registry::<DropTracker>);
        sched.run(world);
    }

    /// reconcile が破棄済み `Window` Entity の registry 要素を除去・drop し、
    /// shutdown フックは空への遷移ちょうどでのみ発火する（非空中・既に空での空振りでは発火しない）。
    #[test]
    fn reconcile_removes_entries_and_fires_hook_only_on_empty_transition() {
        let mut world = World::new();

        let e1 = world.spawn(Window::default()).id();
        let e2 = world.spawn(Window::default()).id();

        let mut reg: WindowRegistry<DropTracker> = WindowRegistry::new();
        let d1 = Rc::new(Cell::new(false));
        let d2 = Rc::new(Cell::new(false));
        reg.insert(
            e1,
            DropTracker {
                dropped: d1.clone(),
            },
        );
        reg.insert(
            e2,
            DropTracker {
                dropped: d2.clone(),
            },
        );

        let fired = Rc::new(Cell::new(0u32));
        let f = fired.clone();
        reg.set_shutdown_hook(move || f.set(f.get() + 1));
        world.insert_non_send(reg);

        world.entity_mut(e1).remove::<Window>();
        run_reconcile(&mut world);
        {
            let reg = world.get_non_send::<WindowRegistry<DropTracker>>().unwrap();
            assert!(!reg.is_empty(), "1 件残るので非空のまま");
        }
        assert!(
            d1.get(),
            "破棄 Entity の保持値が drop されるべき（DestroyWindow 相当）"
        );
        assert!(!d2.get(), "残存 Entity の保持値は drop されない");
        assert_eq!(fired.get(), 0, "非空のままなら shutdown フックは発火しない");

        world.entity_mut(e2).remove::<Window>();
        run_reconcile(&mut world);
        {
            let reg = world.get_non_send::<WindowRegistry<DropTracker>>().unwrap();
            assert!(reg.is_empty(), "全 Window 破棄で registry は空になる");
        }
        assert!(d2.get(), "最後の保持値も drop される");
        assert_eq!(
            fired.get(),
            1,
            "空への遷移ちょうどで shutdown フックが発火する"
        );

        run_reconcile(&mut world);
        assert_eq!(
            fired.get(),
            1,
            "既に空での空振り reconcile では再発火しない"
        );
    }
}
