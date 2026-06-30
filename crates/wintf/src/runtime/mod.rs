//! 新 runtime レイヤ。UI スレッド基盤の owner となる公開 facade `WinApp` を提供する。
//!
//! `WinApp` は旧 `WinThreadMgr` を置換する新公開 facade であり、プロセス初期化
//! （`CoInitializeEx(COINIT_MULTITHREADED)`・DPI awareness 設定）と `EcsWorld`
//! （`Rc<RefCell<EcsWorld>>`）生成を統括する。クラス登録はライブラリ
//! （`wintf-winmsg-executor`）が担うため、本層では行わない。
//!
//! 依存方向は COM→ECS→Runtime。runtime は ecs に依存してよいが、ecs は runtime に
//! 依存しない。

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use tracing::debug;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::HiDpi::*;
use windows::core::Result;

use crate::ecs::world::EcsWorld;
use crate::runtime::wndproc_bridge::WndState;
use window_registry::WindowRegistry;

/// `WinApp` が World へ確保する本番 `WindowRegistry` の具体型。
///
/// 既定の保持値 `Window<WndState>`（ライブラリ型・`!Send`＝UI スレッド束縛）で単相化した
/// NonSend リソース型。`new()`/`get_non_send_resource` の型推論を確定させるために用いる。
type ProdWindowRegistry =
    WindowRegistry<wintf_winmsg_executor::util::Window<WndState>>;

/// メッセージループ層。ライブラリ（`wintf-winmsg-executor`）の `block_on` /
/// `MessageLoop::run` へ委譲する `MessageLoopDriver` を提供する。
mod message_loop;

/// VSync 起床ブリッジ層。専用スレッドの `DwmFlush` vblank 検出を共有
/// `event_listener::Event` で UI スレッドへ通知する `VsyncEventBridge` を提供する。
mod tick_bridge;

/// ウィンドウ手続きブリッジ層。ライブラリの wndproc クロージャから `dispatch_window_message`
/// 純関数へ Entity 配送を橋渡しする `WndState`/`make_wndproc` を提供する。
mod wndproc_bridge;

/// ウィンドウ所有・寿命管理層。生成済み `Window<WndState>`（`!Send`）を Entity キーで
/// 保持する NonSend リソース `WindowRegistry` と、`Window` コンポーネント破棄を検知して
/// 該当要素を drop（`DestroyWindow`）するリコンサイル `reconcile_window_registry` を提供する。
mod window_registry;

/// ECS ウィンドウ生成ファクトリ層。宣言的ウィンドウ生成をライブラリの再入安全な
/// `util::Window::new_checked_ex` 経由へ置換し、生成後に style/pos/title を反映して
/// `WindowRegistry` へ格納する `EcsWindowFactory` を提供する（live cutover は 4.1/4.3）。
mod window_factory;

/// UI スレッド基盤の owner。旧 `WinThreadMgr` を置換する新公開 facade。
///
/// COM 初期化・DPI awareness 設定・`EcsWorld` 生成を統括し、共有 World ハンドルの
/// 唯一の strong 所有者となる。VSync スレッド・message_window・メッセージループ駆動の
/// 結線は後続タスクで追加する。
pub struct WinApp {
    /// 共有 ECS world。`WinApp` が唯一の strong 所有者（`world()` は strong clone を返す）。
    world: Rc<RefCell<EcsWorld>>,
    /// 終了シグナル（`event_listener::Event`・**runtime=WinApp 所有**）。
    ///
    /// 設計 design:387 は「ECS 層所有」だが、その根拠（ecs→runtime の上向き依存回避）は
    /// reconcile が ECS にある場合のみ。本坑は `WindowRegistry`/`reconcile_window_registry`
    /// を runtime 配置（task 3.3 決定）ゆえ notify が runtime→runtime で完結し上向き依存が
    /// 生じない。よって WinApp が `Event` を所有するのが一貫かつ最小（4.2 解釈・開発者決定）。
    ///
    /// `new()` で生成し、`WindowRegistry` の shutdown hook に `notify(usize::MAX)` を仕込む。
    /// `run()` の `block_on(ShutdownPolicy::shutdown_future(...))` 結線は task 4.3。
    #[allow(dead_code)]
    shutdown: Rc<event_listener::Event>,
}

impl WinApp {
    /// COM/DPI 初期化・World 生成を行う。
    ///
    /// COM は `COINIT_MULTITHREADED` で初期化する。既に別モデルで初期化済みの場合
    /// （`S_FALSE` / `RPC_E_CHANGED_MODE`）はレガシー（`WinThreadMgrInner::new`）同様に
    /// 成功扱いとする。DPI awareness は per-monitor v2 を設定する。
    ///
    /// NOTE(W1-V): CoInitializeEx の成功に対し Drop で CoUninitialize を呼ばない現行方針
    /// （P30）を維持する。プロセス常駐の単一インスタンス運用では実害なし。
    pub fn new() -> Result<Self> {
        // SAFETY: Win32 境界。CoInitializeEx はプロセス/スレッドの COM ランタイムを
        // 初期化するのみで、引数 None・COINIT_MULTITHREADED は定数。S_FALSE
        // （同一スレッドで初期化済み）・RPC_E_CHANGED_MODE（別アパートメントモデルで
        // 初期化済み）は成功とみなす（レガシー踏襲）。
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_err() && hr != RPC_E_CHANGED_MODE {
                return Err(hr.into());
            }
        }

        // SAFETY: Win32 境界。SetProcessDpiAwarenessContext はプロセスグローバルな
        // DPI awareness をスレッドセーフに設定する。既に設定済み／不可の場合はエラーを
        // 返すが、レガシー同様に無視する（致命的ではない）。
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let world = Rc::new(RefCell::new(EcsWorld::new()));

        // 終了シグナルを生成（runtime=WinApp 所有・上向き依存なし／4.2 解釈）。
        let shutdown = Rc::new(event_listener::Event::new());

        // WindowRegistry（NonSend）を World へ確保し、空遷移で終了シグナルを notify する
        // hook を **facade から下向きに注入**する。reconcile が最後の窓を除去して registry が
        // 空になると hook が発火し、`run()`（4.3）が待つ shutdown future を完了させる。
        Self::wire_shutdown_hook(&world, &shutdown);

        debug!("WinApp initialized (COM/DPI ready, world created, shutdown hook wired)");

        Ok(Self { world, shutdown })
    }

    /// `WindowRegistry`（NonSend）を World へ確保し、空遷移 hook に終了 notify を注入する。
    ///
    /// 本番型 `WindowRegistry<Window<WndState>>`（`runtime/window_registry.rs` 既定）が未挿入
    /// なら default を挿入し（`create_windows`(4.3) より前でも安全）、その shutdown hook に
    /// `shutdown.notify(usize::MAX)` を撃つクロージャ（`Rc` clone を capture）を仕込む。
    /// これが「facade が下向きに注入」する結線で、reconcile の空遷移発火→Event notify→
    /// shutdown future 完了（4.3）へ繋がる。
    fn wire_shutdown_hook(world: &Rc<RefCell<EcsWorld>>, shutdown: &Rc<event_listener::Event>) {
        let mut ecs = world.borrow_mut();
        let w = ecs.world_mut();
        if w.get_non_send_resource::<ProdWindowRegistry>().is_none() {
            w.insert_non_send_resource(ProdWindowRegistry::new());
        }
        // capture 用に Event の Rc clone を hook へ move（registry が空遷移で発火）。
        let signal = Rc::clone(shutdown);
        let mut reg = w
            .get_non_send_resource_mut::<ProdWindowRegistry>()
            .expect("WindowRegistry was just ensured present");
        reg.set_shutdown_hook(move || {
            // 最後の窓が消えた瞬間に全リスナ起床で notify（shutdown future を完了させる）。
            signal.notify(usize::MAX);
        });
    }

    /// 共有 ECS world ハンドルを返す（旧 `WinThreadMgr::world` 相当）。
    ///
    /// 返す `Rc` は単一 World（複数ウィンドウで共有）への strong clone。
    pub fn world(&self) -> Rc<RefCell<EcsWorld>> {
        Rc::clone(&self.world)
    }

    /// UI スレッドのメッセージループを開始する（旧 `WinThreadMgr::run` 相当）。
    ///
    /// 最小スタブ。`AsyncTickTask` の spawn・`ShutdownPolicy` future の `block_on` 等の
    /// 完全な結線は後続タスク（run の全結線）で実装する。現状は即時復帰する。
    pub fn run(&self) -> Result<()> {
        Ok(())
    }

    /// UI スレッド単一の async タスクを投入する（旧 `spawn_normal` 相当・`!Send` 可）。
    ///
    /// ライブラリの `spawn_local` へ委譲する。実行にはメッセージループ（`run`/`block_on`）が
    /// 必要だが、投入自体はループ非実行下でも可能。
    pub fn spawn_ui_local<T: 'static>(
        &self,
        fut: impl Future<Output = T> + 'static,
    ) -> wintf_winmsg_executor::JoinHandle<T> {
        wintf_winmsg_executor::spawn_local(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_listener::Listener;

    /// `WinApp::new()` の shutdown 結線（task 4.2・要件 1.3/1.4/1.5）:
    /// 1. `WindowRegistry`（NonSend）が World に確保される。
    /// 2. その shutdown hook が発火すると WinApp 所有の終了シグナル `Event` が notify され、
    ///    `listen()` した待機が起床する（= run()(4.3) が待つ shutdown future が完了する経路）。
    ///
    /// 「最後の窓が消えた瞬間に hook 発火→Event notify」を hook を直接発火して検証する
    /// （本番型 `Window<WndState>` は実 HWND が要るため headless では reconcile を直接駆動
    /// せず hook 発火 seam で wiring を実証する）。full close→destroy→reconcile→hook→
    /// future 完了→block_on 復帰の E2E は 4.3/5.3 の領分。
    #[test]
    fn new_wires_registry_shutdown_hook_to_notify_event() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");

        // 終了シグナルへ先に listen() を arm（hook 発火 notify を取りこぼさない）。
        let listener = app.shutdown.listen();

        // World 内の WindowRegistry を取得し、注入済み hook を発火（= 空遷移相当）。
        {
            let world = app.world();
            let ecs = world.borrow();
            let reg = ecs
                .world()
                .get_non_send_resource::<ProdWindowRegistry>()
                .expect("WinApp::new should ensure a WindowRegistry NonSend resource");
            reg.fire_shutdown_hook();
        }

        // hook が WinApp 所有の Event を notify したので arm 済み listener が起床する
        // （ハングしない = shutdown future が完了する経路が結線されている）。
        listener.wait();
    }

    /// 終了シグナルは `WinApp` が strong 所有する（`shutdown` フィールド存在の確認も兼ねる）。
    /// 構築直後は未 notify ゆえ、新規 arm した listener は即起床しない（false-positive 防止）。
    #[test]
    fn new_owns_unfired_shutdown_signal() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");
        // まだ hook 未発火なら notify されていない。新規 listener は即起床しないはず。
        let listener = app.shutdown.listen();
        assert!(
            listener
                .wait_timeout(std::time::Duration::from_millis(20))
                .is_none(),
            "構築直後（hook 未発火）は終了シグナルが notify されていないべき"
        );
    }
}
