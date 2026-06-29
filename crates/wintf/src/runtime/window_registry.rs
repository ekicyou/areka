//! ウィンドウ所有・寿命管理層（`WindowRegistry`）。
//!
//! 生成済み `Window<WndState>`（ライブラリ `wintf-winmsg-executor::util::Window`・`!Send`）を
//! Entity キーで保持する **NonSend リソース**を提供する。`Window<S>` は `hwnd` を持ち
//! `Drop` で `DestroyWindow` を呼ぶ `!Send` 型であり、その `!Send` 性こそが
//! 「UI スレッド束縛（`DestroyWindow` のスレッドアフィニティ）」の安全不変条件である。
//! よって `Send` 偽装（`unsafe impl Send`）は行わず、保持値が `!Send` であることにより
//! bevy が自動的に NonSend として扱う（設計 WindowRegistry / State Management）。
//!
//! # 責務分離（配送と寿命）
//! 配送は `WndState{ Weak<World>, Entity }`（[`crate::runtime::wndproc_bridge`]）が担い、
//! 寿命・最後の窓検知・終了起点は本レジストリが担う（設計 Integration Notes）。
//!
//! # リコンサイル（[`reconcile_window_registry`]）
//! `Window` コンポーネント（[`crate::ecs::window::Window`]）の破棄を `RemovedComponents`
//! で検知し、該当 Entity の `Window<WndState>` を `remove` → drop → `DestroyWindow`
//! → `WM_NCDESTROY` させ、ハンドル破棄を Entity ライフサイクルに一致させる。除去後に
//! レジストリが空になったら shutdown フック（終了起点）を発火する。
//!
//! # スケジュール結線は本タスクの領分外
//! 本システムを tick スケジュールへ登録するのは `WinApp`（runtime→World）が後続タスク
//! （4.3）で行う。ここでは登録しない（ECS→runtime の上向き依存を schedule に持ち込まない）。
//! shutdown シグナル本体（ECS 所有の `event_listener::Event`・facade 下向き注入）の結線も
//! 後続タスク（4.2）の領分で、本タスクは「空遷移で発火する差し替え可能なフック接点」のみ
//! 用意する（設計 ShutdownPolicy）。

use std::collections::HashMap;

use bevy_ecs::prelude::{Entity, RemovedComponents};
use bevy_ecs::system::NonSendMut;
use wintf_winmsg_executor::util::Window as LibWindow;

use crate::ecs::window::Window;
use crate::runtime::wndproc_bridge::WndState;

/// 空への遷移時に発火する終了起点フック。
///
/// 実体（ECS 所有の `event_listener::Event` notify）は後続タスク 4.2 で注入される。
/// 本タスクでは「空遷移ちょうどで呼ばれる差し替え可能な接点」だけを提供する。
type ShutdownHook = Box<dyn Fn()>;

/// 生成済みウィンドウハンドルを Entity キーで保持する NonSend リソース。
///
/// 既定の `W = Window<WndState>` が本番型（`!Send`＝UI スレッド束縛）。テスト容易性のため
/// 保持値 `W` を型引数化し、headless テストではドロップ追跡ダミーを差し込む（本番型の形は
/// 不変＝`Window<WndState>` のまま `!Send`/NonSend）。
///
/// `Send` 偽装はしない。`W = Window<WndState>` が `!Send` であることにより本リソースも
/// `!Send` となり、bevy が NonSend として（メインスレッド専用で）強制する。
//
// NOTE: `create_windows`（task 3.4）が insert、`WinApp::run`（task 4.3）が reconcile を
// schedule へ結線するまで lib ビルドでは未使用。兄弟 building block（MessageLoopDriver
// /VsyncEventBridge/AsyncTickTask）と同様に scoped dead_code 許容。
#[allow(dead_code)]
pub(crate) struct WindowRegistry<W = LibWindow<WndState>> {
    /// Entity → 生成済みウィンドウ。`remove`/drop で `DestroyWindow` が走る。
    map: HashMap<Entity, W>,
    /// 空遷移で発火する終了起点フック（未注入なら `None`）。
    shutdown_hook: Option<ShutdownHook>,
}

#[allow(dead_code)]
impl<W> WindowRegistry<W> {
    /// 空のレジストリを生成する（フック未注入）。
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            shutdown_hook: None,
        }
    }

    /// 空遷移で発火する終了起点フックを差し替える。
    ///
    /// 後続タスク 4.2 が ECS 所有の `event_listener::Event` を notify するクロージャを
    /// ここへ注入する。本タスクでは接点のみ。
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

    /// 保持ウィンドウが 0 件か。
    pub(crate) fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// `Window` コンポーネント破棄を検知して registry 要素を drop するリコンサイルシステム。
///
/// `RemovedComponents<Window>` が積む「`Window` を失った Entity」を順に `remove` し、
/// 取り出した `Window<WndState>` をその場で drop する（→ `DestroyWindow` →
/// `WM_NCDESTROY`）。これによりハンドル破棄を Entity ライフサイクルに一致させる。
///
/// 1 件以上を実際に除去し、その結果 registry が空になった場合にのみ shutdown フックを
/// 発火する（「空への遷移ちょうど」。元から空での no-op 除去では発火しない）。
///
/// 保持値 `W` で総称化する（本番は既定 `W = Window<WndState>`）。bevy が system として
/// 登録できるのは具体型のみのため、`'static` 境界を付し本番では `W = Window<WndState>` で
/// 単相化される。headless テストはダミー `W` で同じ反復ロジックを検証する。
///
/// 本システムは schedule へ自動登録しない。登録は `WinApp`（task 4.3）が行う。
//
// NOTE: dead_code 許容理由は `WindowRegistry` と同じ（4.3 結線まで未使用）。
#[allow(dead_code)]
pub(crate) fn reconcile_window_registry<W: 'static>(
    mut removed: RemovedComponents<Window>,
    mut registry: NonSendMut<WindowRegistry<W>>,
) {
    let mut removed_any = false;
    for entity in removed.read() {
        // remove の戻り値はここで drop され、`Window<WndState>::drop` が
        // `DestroyWindow` を呼ぶ（ハンドル破棄＝Entity 寿命一致）。
        if registry.remove(entity).is_some() {
            removed_any = true;
        }
    }

    // 「空への遷移ちょうど」でのみ終了起点を発火する。実際に除去が起き、かつ結果が空
    // のときに限る（元から空＝窓ゼロでの空振りリコンサイルでは発火しない）。
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

    /// (1) insert + remove が Entity キーで round-trip する。
    #[test]
    fn insert_remove_roundtrips_by_entity() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg: WindowRegistry<DropTracker> = WindowRegistry::new();
        let flag = Rc::new(Cell::new(false));
        reg.insert(e, DropTracker { dropped: flag.clone() });
        assert!(!reg.is_empty());
        let got = reg.remove(e);
        assert!(got.is_some(), "remove は Entity キーで挿入済み要素を返すべき");
        assert!(reg.is_empty());
        // 取り出した値はまだ生存（drop 未発火）。
        assert!(!flag.get());
        drop(got);
        assert!(flag.get(), "remove で取り出した値の drop が観測されるべき");
    }

    /// (2) entity の remove で保持値が drop される（drop-tracker が観測）。
    #[test]
    fn remove_drops_held_value() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        let mut reg: WindowRegistry<DropTracker> = WindowRegistry::new();
        let flag = Rc::new(Cell::new(false));
        reg.insert(e, DropTracker { dropped: flag.clone() });
        assert!(!flag.get());
        // remove の戻り値を即 drop（receiver 不在）→ Drop が走る。
        drop(reg.remove(e));
        assert!(flag.get(), "registry 内要素の remove で Drop が走るべき");
    }

    /// reconcile を 1 度走らせる headless ヘルパ。
    fn run_reconcile(world: &mut World) {
        use bevy_ecs::schedule::Schedule;
        let mut sched = Schedule::default();
        sched.add_systems(reconcile_window_registry::<DropTracker>);
        sched.run(world);
    }

    /// (3)+(4) reconcile が破棄済み `Window` Entity の registry 要素を除去・drop し、
    ///         shutdown フックは「空への遷移ちょうど」でのみ発火する（非空中は発火しない・
    ///         既に空での空振りでは再発火しない）。
    #[test]
    fn reconcile_removes_entries_and_fires_hook_only_on_empty_transition() {
        let mut world = World::new();

        // Window コンポーネント付き Entity を 2 つ作る（headless: 実窓は作らない）。
        let e1 = world.spawn(Window::default()).id();
        let e2 = world.spawn(Window::default()).id();

        // registry を NonSend リソースとして world に挿入。
        let mut reg: WindowRegistry<DropTracker> = WindowRegistry::new();
        let d1 = Rc::new(Cell::new(false));
        let d2 = Rc::new(Cell::new(false));
        reg.insert(e1, DropTracker { dropped: d1.clone() });
        reg.insert(e2, DropTracker { dropped: d2.clone() });

        let fired = Rc::new(Cell::new(0u32));
        let f = fired.clone();
        reg.set_shutdown_hook(move || f.set(f.get() + 1));
        world.insert_non_send_resource(reg);

        // 1 つ目の Window を破棄 → reconcile で除去・drop されるが registry は非空のまま。
        world.entity_mut(e1).remove::<Window>();
        run_reconcile(&mut world);
        {
            let reg = world
                .get_non_send_resource::<WindowRegistry<DropTracker>>()
                .unwrap();
            assert!(!reg.is_empty(), "1 件残るので非空のまま");
        }
        assert!(d1.get(), "破棄 Entity の保持値が drop されるべき（DestroyWindow 相当）");
        assert!(!d2.get(), "残存 Entity の保持値は drop されない");
        assert_eq!(fired.get(), 0, "非空のままなら shutdown フックは発火しない");

        // 2 つ目（最後）の Window を破棄 → reconcile で空へ遷移しフックがちょうど 1 回発火。
        world.entity_mut(e2).remove::<Window>();
        run_reconcile(&mut world);
        {
            let reg = world
                .get_non_send_resource::<WindowRegistry<DropTracker>>()
                .unwrap();
            assert!(reg.is_empty(), "全 Window 破棄で registry は空になる");
        }
        assert!(d2.get(), "最後の保持値も drop される");
        assert_eq!(fired.get(), 1, "空への遷移ちょうどで shutdown フックが発火する");

        // 既に空で reconcile（除去なし）→ 再発火しない。
        run_reconcile(&mut world);
        assert_eq!(fired.get(), 1, "既に空での空振り reconcile では再発火しない");
    }
}
