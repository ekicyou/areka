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

use tracing::{debug, info, warn};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::HiDpi::*;
use windows::core::Result;

use crate::ecs::clickthrough::{
    ClickThroughController, ClickThroughHandle, ClickThroughRegistry, ClickThroughRegistryHandle,
};
use crate::ecs::world::{EcsWorld, EcsWorldSelfRef, FrameFinalize};
use crate::runtime::message_loop::{MessageLoopDriver, ShutdownPolicy};
use crate::runtime::tick_bridge::{AsyncTickTask, VsyncEventBridge};
use crate::runtime::window_registry::{WindowRegistry, reconcile_window_registry};
use crate::runtime::wndproc_bridge::WndState;

/// `WinApp` が World へ確保する本番 `WindowRegistry` の具体型。
///
/// 既定の保持値 `Window<WndState>`（ライブラリ型・`!Send`＝UI スレッド束縛）で単相化した
/// NonSend リソース型。`new()`/`get_non_send` の型推論を確定させるために用いる。
type ProdWindowRegistry = WindowRegistry<crate::executor::util::Window<WndState>>;

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
///
/// `pub(crate)`: `WinApp::run`（task 4.3）が `reconcile_window_registry` を schedule へ
/// 登録する（runtime→World）ため、クレート内から参照可能にする。
pub(crate) mod window_registry;

/// ECS ウィンドウ生成ファクトリ層。宣言的ウィンドウ生成をライブラリの再入安全な
/// `util::Window::new_checked_ex` 経由へ置換し、生成後に style/pos/title を反映して
/// `WindowRegistry` へ格納する `EcsWindowFactory` を提供する（live cutover は 4.3）。
///
/// `pub(crate)`: `create_windows`（ECS 層）が `EcsWindowFactory::create_window` を呼ぶ
/// 設計公認の単一上向きエッジ（ecs→runtime）のため、クレート内から参照可能にする。
pub(crate) mod window_factory;

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
    /// `run()` が `block_on(ShutdownPolicy::shutdown_future(self.shutdown.clone()))` で待つ
    /// （task 4.3 で結線済み）。
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
        // 返すが、レガシー同様にプロセス起動は止めない（致命的ではない）。
        //
        // Req 7.4: **戻り値を握り潰さない**。旧実装は `let _ =` で捨てていたため、
        // per-monitor v2 の設定が失敗していても観測できず、`WM_DPICHANGED` が
        // 実機で 0 件だった機序（診断レポート §2.7）の切り分けができなかった。
        // 成否のどちらも必ず 1 行残す——「点灯しない観測点を『発生 0 回』の根拠に
        // 用いない」（Req 1.5）の直接適用である。
        unsafe {
            match SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) {
                Ok(()) => info!(
                    context = "PER_MONITOR_AWARE_V2",
                    "[SetProcessDpiAwarenessContext] DPI awareness set"
                ),
                Err(e) => warn!(
                    context = "PER_MONITOR_AWARE_V2",
                    error = ?e,
                    "[SetProcessDpiAwarenessContext] failed (process continues with the inherited awareness)"
                ),
            }
        }

        // 呼び出しスレッド＝UI スレッド（メッセージループを回す本体）である。ここで 1 度だけ
        // 役割を宣言してスレッド名簿へ載せる（要件 2.3）。COM/DPI の初期化を済ませた後・
        // World 生成の前に置くのは、`new()` が返る前に必ず 1 度通る点として最も早いため。
        // 失敗しても起動は止めない——このスレッドの CPU が `unregistered_rest` へ回るだけで、
        // 観測の道具の不調が観測対象を止める理由にはならない。
        if let Err(e) = crate::ecs::world::thread_registry::register_current_thread(
            crate::ecs::world::thread_registry::ROLE_UI,
        ) {
            tracing::error!(error = %e, "[WinApp::new] スレッド名簿への登録に失敗した（起動は継続）");
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
        if w.get_non_send::<ProdWindowRegistry>().is_none() {
            w.insert_non_send(ProdWindowRegistry::new());
        }
        // capture 用に Event の Rc clone を hook へ move（registry が空遷移で発火）。
        let signal = Rc::clone(shutdown);
        let mut reg = w
            .get_non_send_mut::<ProdWindowRegistry>()
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

    /// 新経路の結線（self-ref 注入 + reconcile 登録）を World へ施す（冪等・task 4.3）。
    ///
    /// `run()` 開始時に呼ぶ。複数回呼んでも安全:
    /// - `EcsWorldSelfRef`（外側 `Weak<RefCell<EcsWorld>>`）を NonSend リソースとして
    ///   insert する（`create_windows` がこれを読んで新経路/旧経路を分岐）。既存なら上書き
    ///   （同一 World への Weak ゆえ等価）。
    /// - `reconcile_window_registry::<Window<WndState>>`（本番単相化）を **遅め schedule**
    ///   `FrameFinalize` へ登録する。
    ///
    /// # reconcile を `FrameFinalize` に置く理由（task 4.3 解釈 (3)）
    /// `try_tick_world`（`ecs/world/mod.rs`）は 13 本スケジュールを `Input … FrameFinalize`
    /// の順で回し、本クレートは `clear_trackers`/`clear_all` を **どこでも呼ばない**
    /// （grep 済み・確認）。bevy の `RemovedComponents<Window>` は `clear_trackers` まで
    /// バッファに残り、各登録システムは自分の read カーソルで「各除去を一度だけ」観測する。
    /// よって配置による取りこぼしは構造的に起きないが、WM_CLOSE の despawn が
    /// `UISetup`（create と同じ）等の前段で起きても **同一 tick の最終段**で確実に
    /// reconcile される `FrameFinalize`（13 本目）を選ぶ（破棄→registry drop→空遷移 hook→
    /// shutdown notify を 1 フレーム内で閉じる）。
    ///
    /// # 重複登録について
    /// `add_systems` を複数回呼ぶと同じ system 関数が重複登録され得る。`run()` を複数回
    /// 呼ぶ運用は想定しない（メッセージループは 1 度）ため実害はないが、念のため呼び出しは
    /// `run()` 冒頭の 1 箇所に集約する。
    fn wire_new_path(&self) {
        let weak = Rc::downgrade(&self.world);
        let mut ecs = self.world.borrow_mut();
        let w = ecs.world_mut();
        // self-ref（外側 Weak）を注入（create_windows の新経路分岐スイッチ）。
        w.insert_non_send(EcsWorldSelfRef(weak));
        drop(ecs);

        // reconcile を遅め schedule（FrameFinalize）へ登録（runtime→World・上向き依存なし）。
        // 本番単相化 `Window<WndState>` で turbofish して登録する。
        self.world.borrow_mut().add_systems(
            FrameFinalize,
            reconcile_window_registry::<crate::executor::util::Window<WndState>>,
        );
    }

    /// クリック透過機構を起動し、登録面（NonSend リソース）を World へ据える（task 3.2）。
    ///
    /// `run()` の結線点（既存 tick/vsync 結線に相乗り）から呼ぶ、テスト可能な最小フック。
    /// `wire_new_path` がフル `run()` ループと分離してテストされるのと同じ方針で、本結線も
    /// メッセージループ非実行下（headless）で単体検証できるよう **ヘルパーに切り出す**。
    ///
    /// 手順:
    /// 1. 共有レジストリ `Rc<RefCell<ClickThroughRegistry>>` と **専用** wake `Arc<Event>` を生成。
    /// 2. [`ClickThroughController::start`] を呼び、ワーカ＋判定ループを起動（`self.world` の
    ///    `Weak` を渡す＝shutdown で `upgrade()` None → ループ終了、ワーカは返り値 handle drop で
    ///    stop/join）。返る [`ClickThroughHandle`] は `run()` がローカル保持し、`run()` 復帰
    ///    （＝shutdown）で drop される（`bridge`/`_tick` と同じ寿命規律）。
    /// 3. 共有レジストリを [`ClickThroughRegistryHandle`]（`pub` NonSend リソース）に包んで World へ
    ///    挿入する。areka の `run_setup`（task 4.1・`&mut World` を持つ）が
    ///    `get_non_send::<ClickThroughRegistryHandle>()` で取得し 2 窓を登録する登録面。
    ///
    /// # wake event を VSync とは別に専用化する理由（起床契機・post-tick）
    /// 機構は二重起床する: (a) カーソル移動 notify（ワーカが `start` 内で `wake_event` を叩く）、
    /// (b) VSync tick 毎の再評価（静止カーソルでも表示更新＝αマスク変化に追随／R2.4、`JustEnded`
    /// 再収束／R5.2）。ここで wake_event を **VSync event とは別 `Arc`** に保ち、VSync 起床は
    /// [`Self::run`] の relay タスクが「vblank を待って `wake_event.notify`」する形で中継する
    /// （下記 relay 参照）。こうすることで既存 tick cadence を一切変えない（カーソル移動 notify が
    /// `AsyncTickTask` を余計に起こして ECS tick を増やすことがない＝入力と描画 cadence を非結合）。
    ///
    /// # post-tick の構造保証
    /// UI 全体が単一スレッドで、判定コア（`evaluate_targets`）は `&World` を **同期借用**する
    /// （await を跨がず、tick が `borrow_mut` を保持する間は借用しない）。よって評価は常に
    /// settled な World（tick 途中の中間状態ではない）を読む。起床順序に関わらず最悪でも
    /// 1 フレーム遅延に留まり、これが design「post-tick 評価」の構造的裏付けである。
    ///
    /// 戻り値: `(handle, wake_event)`。`handle` は `run()` がローカル保持（drop=停止/join）、
    /// `wake_event` は VSync relay タスクの結線に使う。
    fn wire_click_through(&self) -> (ClickThroughHandle, std::sync::Arc<event_listener::Event>) {
        // 1. 共有レジストリ（UI スレッド単独所有・`!Send`）と専用 wake event。
        let registry = Rc::new(RefCell::new(ClickThroughRegistry::new()));
        let wake_event = std::sync::Arc::new(event_listener::Event::new());

        // 2. 機構起動（ワーカ生成・event 共有・async ループ spawn_local）。
        let handle = ClickThroughController::start(
            Rc::downgrade(&self.world),
            Rc::clone(&registry),
            std::sync::Arc::clone(&wake_event),
        );

        // 3. 登録面（NonSend リソース）を World へ据える（areka task 4.1 が取得・登録）。
        {
            let mut ecs = self.world.borrow_mut();
            ecs.world_mut()
                .insert_non_send(ClickThroughRegistryHandle::new(Rc::clone(&registry)));
        }

        debug!("ClickThrough wired (worker + eval loop started, registry handle inserted)");
        (handle, wake_event)
    }

    /// UI スレッドのメッセージループを開始する（旧 `WinThreadMgr::run` 相当・task 4.3 全結線）。
    ///
    /// 最小フロー `WinApp::new → world → run` でメッセージループ・tick・終了が一体動作する
    /// 心臓部。手順:
    /// 1. 新経路を結線（[`wire_new_path`](Self::wire_new_path)・self-ref 注入 + reconcile 登録）。
    /// 2. `VsyncEventBridge` をローカルに生成（`run` の間 `WinApp` 相当で所有・Drop で
    ///    VSync スレッド stop→join）。
    /// 3. `AsyncTickTask::spawn` で 60Hz tick ループを UI スレッドへ投入（JoinHandle は
    ///    `run` の間ローカル保持・World は `Weak`）。
    /// 4. `MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(self.shutdown.clone()))`
    ///    で、最後のウィンドウ破棄（registry 空遷移 hook→Event notify）まで OS メッセージ
    ///    ループを駆動する。future 完了でループが **先行 quit せず** 正常復帰する
    ///    （"received unexpected quit message" panic を構造的に回避）。
    /// 5. 復帰時、ローカルの `VsyncEventBridge` が drop（VSync stop→join）し、tick の
    ///    `JoinHandle` も drop される。tail race 補填として終了時に防御的 notify を撃つ。
    ///
    /// # 戻り値
    /// 正常終了時 `Ok(())`。COM/DPI は `new()` で初期化済み。
    pub fn run(&self) -> Result<()> {
        // 1. 新経路結線（self-ref 注入 + reconcile 登録・冪等）。
        self.wire_new_path();

        // 2. VSync ブリッジを run の間ローカル所有（Drop で stop→join）。
        let bridge = VsyncEventBridge::new();

        // 3. 60Hz tick ループを UI スレッドへ投入（World は Weak・shutdown で upgrade None 終了）。
        let _tick = AsyncTickTask::spawn(bridge.event().clone(), Rc::downgrade(&self.world));

        // 3.5. クリック透過機構を起動（ワーカ＋判定ループ）＋登録面を World へ据える（task 3.2）。
        //      `_click_through` を run の間ローカル保持し、run 復帰（shutdown）で drop
        //      （ワーカ stop/join・async ループは world Weak upgrade None で自然終了）。
        let (_click_through, click_wake) = self.wire_click_through();

        // 3.6. VSync tick → クリック透過 wake の relay タスク（既存 tick cadence を変えない）。
        //      vblank 毎に click_wake を notify し、静止カーソル時も post-tick 再評価を起こす
        //      （R2.4 表示更新追随／R5.2 JustEnded 再収束）。listen-before-work 規律で notify を
        //      取りこぼさず、`self.world` の Weak が upgrade 不能（shutdown）になればループ終了。
        //      wake_event を VSync event とは **別 Arc** に保つことで、カーソル移動 notify が
        //      `AsyncTickTask` を余計に起こさない（入力と描画 cadence の非結合）。
        {
            let vblank = bridge.event().clone();
            let world_weak = Rc::downgrade(&self.world);
            let wake = std::sync::Arc::clone(&click_wake);
            let _relay = crate::executor::spawn_local(async move {
                loop {
                    // 先に listen() を arm（処理中に届く vblank を落とさない）。
                    let listener = vblank.listen();
                    // strong 所有者が生存しているか（shutdown で None → 終了）。
                    if world_weak.upgrade().is_none() {
                        debug!("ClickThrough VSync relay stopping (world dropped — shutdown)");
                        return;
                    }
                    // 次 vblank まで待機し、クリック透過ループを起床（post-tick 再評価）。
                    listener.await;
                    wake.notify(usize::MAX);
                }
            });
            // `_relay` はこのスコープ末で JoinHandle が drop されるが、spawn 済みタスクは
            // executor 上で走り続け、world Weak upgrade None（shutdown）で自ら終了する
            // （`AsyncTickTask` と同じ「JoinHandle を保持せず self-terminate」規律）。
        }

        // 4. shutdown future が完了するまでメッセージループを駆動（先行 quit せず復帰）。
        MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(Rc::clone(&self.shutdown)));

        // 5. tail race 補填の防御的 notify（冪等・arm 済みリスナのみ起床）。
        ShutdownPolicy::notify_shutdown(&self.shutdown);

        // bridge / _tick はここで drop（VSync stop→join・tick JoinHandle 破棄）。
        Ok(())
    }

    /// UI スレッド単一の async タスクを投入する（旧 `spawn_normal` 相当・`!Send` 可）。
    ///
    /// ライブラリの `spawn_local` へ委譲する。実行にはメッセージループ（`run`/`block_on`）が
    /// 必要だが、投入自体はループ非実行下でも可能。
    pub fn spawn_ui_local<T: 'static>(
        &self,
        fut: impl Future<Output = T> + 'static,
    ) -> crate::executor::JoinHandle<T> {
        crate::executor::spawn_local(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_listener::Listener;

    /// `WinApp::new()` は呼び出しスレッド（＝UI スレッド）を役割名 `ui` で名簿へ登録する
    /// （要件 2.3）。
    ///
    /// 登録は同期的で、対象は自分自身のスレッド——待ちも件数の当てにも要らない。
    #[test]
    fn new_registers_the_calling_thread_as_the_ui_thread() {
        use crate::ecs::world::thread_registry::{self, ROLE_UI, get_current_thread_id};

        let app = WinApp::new().expect("WinApp::new should succeed headless");

        let tid = get_current_thread_id();
        let found = thread_registry::snapshot()
            .into_iter()
            .find(|entry| entry.tid == tid)
            .expect("WinApp::new は呼び出しスレッドを名簿へ載せるはず");
        assert_eq!(
            found.role, ROLE_UI,
            "WinApp を建てたスレッドの役割名は ui であるはず"
        );
        assert!(
            thread_registry::is_known_role(ROLE_UI),
            "登録した役割名は固定語彙に含まれるはず"
        );

        drop(app);
    }

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
                .get_non_send::<ProdWindowRegistry>()
                .expect("WinApp::new should ensure a WindowRegistry NonSend resource");
            reg.fire_shutdown_hook();
        }

        // hook が WinApp 所有の Event を notify したので arm 済み listener が起床する
        // （ハングしない = shutdown future が完了する経路が結線されている）。
        listener.wait();
    }

    /// task 4.3: `wire_new_path` 後の World 状態を検証する（headless）。
    /// 1. `EcsWorldSelfRef`（NonSend・外側 Weak）が World に注入される（create_windows の
    ///    新経路分岐スイッチ）。その Weak は WinApp の strong World を upgrade できる。
    /// 2. `reconcile_window_registry` が `FrameFinalize` へ登録され、空 World の 1 tick が
    ///    panic せず回る（reconcile 登録が schedule を壊さない＝結線が健全）。
    ///
    /// 実 block_on ループ（実窓 + close イベント要）は回さない。full new→world→run の E2E は
    /// 4.4（examples 切替）+ 5.3（手動）の領分。
    #[test]
    fn wire_new_path_injects_self_ref_and_schedules_reconcile() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");
        app.wire_new_path();

        {
            let world = app.world();
            let ecs = app.world.borrow();
            // (1) EcsWorldSelfRef が注入され、外側 World を upgrade できる。
            let self_ref = ecs
                .world()
                .get_non_send::<EcsWorldSelfRef>()
                .expect("wire_new_path should inject EcsWorldSelfRef");
            assert!(
                self_ref.0.upgrade().is_some(),
                "注入された Weak は WinApp の strong World を upgrade できるべき"
            );
            drop(ecs);
            let _ = world; // strong clone（self-ref upgrade の生存保証は self.world）。
        }

        // (2) reconcile 登録後、空 World で 1 tick 回しても panic しない（schedule 健全）。
        app.world.borrow_mut().try_tick_world();
    }

    /// 統合テスト（task 5.2・要件 1.3/1.5/2.1）: **close→reconcile→shutdown チェーン end-to-end**。
    ///
    /// `new_wires_registry_shutdown_hook_to_notify_event` は hook を直接発火する seam テスト。
    /// 本テストは実 `Window<WndState>`（実 HWND）を WinApp の World へ生成し、その `Window`
    /// コンポーネントを除去 → 実 `reconcile_window_registry`（schedule 駆動）が registry を
    /// 空へ遷移させ → 注入済み shutdown hook が WinApp 所有 `Event` を notify し → arm 済み
    /// listener が **ハングせず** 起床する、という close→shutdown の本番チェーンを実 reconcile
    /// で貫通検証する（要件 1.3「終了要求で清掃終了」/ 1.5「ハング・panic なし」）。
    ///
    /// これは `run()` の `block_on(shutdown_future)` が最後の窓クローズで **先行 quit せず**
    /// future 完了で正常復帰する経路の核（最後の窓 close → Event notify）を結線実証する。
    /// full `run()` の block_on 復帰（メッセージループ実駆動＋OS close イベント）は E2E 5.3。
    ///
    /// headless 制約: DComp で不可視のまま実 HWND を生成（GDI 非表示）、メッセージループは
    /// 回さず reconcile を直接 schedule 実行する。
    #[test]
    fn close_to_reconcile_to_shutdown_chain_wakes_listener() {
        use crate::ecs::window::{Window, WindowHandle, WindowStyle};
        use crate::ecs::world::EcsWorldSelfRef;
        use crate::runtime::window_factory::EcsWindowFactory;
        use bevy_ecs::schedule::Schedule;
        use windows::Win32::UI::WindowsAndMessaging::{WS_EX_LAYERED, WS_POPUP};

        let app = WinApp::new().expect("WinApp::new should succeed headless");
        // 新経路結線（self-ref 注入 + reconcile を FrameFinalize へ登録 + shutdown hook は
        // new() で既に WinApp 所有 Event へ結線済み）。
        app.wire_new_path();

        // 終了シグナルへ先に listen() を arm（reconcile 空遷移の notify を取りこぼさない）。
        let listener = app.shutdown.listen();

        let weak = Rc::downgrade(&app.world);

        // 実 Window<WndState>（実 HWND）を 1 枚生成し registry へ格納（DComp で不可視）。
        let entity = {
            let world = app.world();
            let mut ecs = world.borrow_mut();
            let w = ecs.world_mut();
            // self-ref は wire_new_path で注入済みだが、明示確認（factory 分岐の前提）。
            assert!(
                w.get_non_send::<EcsWorldSelfRef>().is_some(),
                "wire_new_path が EcsWorldSelfRef を注入しているべき"
            );
            let entity = w
                .spawn((
                    Window {
                        title: "ShutdownChain".to_string(),
                        parent: None,
                    },
                    WindowStyle {
                        style: WS_POPUP,
                        ex_style: WS_EX_LAYERED,
                    },
                ))
                .id();
            EcsWindowFactory::create_window(w, entity, weak.clone());
            // 生成済み・registry 非空・ハンドル付与を確認（最後の窓が在る状態）。
            assert!(
                w.get::<WindowHandle>(entity).is_some(),
                "factory 生成後に WindowHandle が付与されるべき"
            );
            let reg = w
                .get_non_send::<ProdWindowRegistry>()
                .expect("ProdWindowRegistry が存在するべき");
            assert!(!reg.is_empty(), "生成直後の registry は非空（窓が在る）");
            entity
        };

        // 最後の窓を close 相当（`Window` コンポーネント除去）→ 実 reconcile を 1 周。
        {
            let world = app.world();
            let mut ecs = world.borrow_mut();
            let w = ecs.world_mut();
            w.entity_mut(entity).remove::<Window>();

            // 本番 reconcile（`Window<WndState>` 単相）を schedule 実行（FrameFinalize 相当）。
            // registry が空へ遷移し、注入済み hook が WinApp 所有 Event を notify する。
            let mut sched = Schedule::default();
            sched.add_systems(reconcile_window_registry::<crate::executor::util::Window<WndState>>);
            sched.run(w);

            // 空遷移を確認（最後の窓が消えた）。
            let reg = w
                .get_non_send::<ProdWindowRegistry>()
                .expect("ProdWindowRegistry が存在するべき");
            assert!(
                reg.is_empty(),
                "最後の Window 除去後 reconcile で registry は空へ遷移するべき"
            );
        }

        // 空遷移 hook が WinApp 所有 Event を notify したので arm 済み listener が起床する。
        // ハングしない = run() の shutdown future が完了して block_on が正常復帰できる経路。
        listener.wait();
    }

    /// task 3.2: `wire_click_through` の結線を headless で検証する。
    /// 1. クリック透過機構が起動し（ワーカ＋判定ループ）、返る `ClickThroughHandle` を得る。
    /// 2. 登録面 `ClickThroughRegistryHandle`（NonSend リソース）が World へ挿入される
    ///    （areka task 4.1 が `get_non_send` で取得し窓登録する登録面）。
    /// 3. 登録面経由の register/remove が反映される（共有レジストリ）。
    /// 4. `ClickThroughHandle` を drop するとワーカが stop/join されハングしない（RAII）。
    ///
    /// フル `run()` のメッセージループ（実 vblank relay 駆動）は回さず、`wire_new_path` と同様に
    /// 結線ヘルパーを単体検証する（executor 非駆動でも spawn_local タスクは投入されるだけで安全）。
    #[test]
    fn wire_click_through_starts_and_inserts_registry_handle() {
        use crate::ecs::clickthrough::ClickThroughRegistryHandle;

        let app = WinApp::new().expect("WinApp::new should succeed headless");
        let (handle, _wake) = app.wire_click_through();

        // ダミー window Entity（登録面の反映確認用・World 生存は問わない）。
        let e = bevy_ecs::entity::Entity::from_raw_u32(1).expect("valid test entity index");

        // 登録面が NonSend リソースとして World に据えられていること。
        {
            let world = app.world();
            let ecs = world.borrow();
            let reg_handle = ecs
                .world()
                .get_non_send::<ClickThroughRegistryHandle>()
                .expect("wire_click_through は ClickThroughRegistryHandle を World へ挿入すべき");
            assert!(reg_handle.is_empty(), "初期の登録面は空");

            // 登録面経由の register が共有レジストリへ反映される（areka 4.1 の登録経路）。
            reg_handle.register(e, windows::Win32::Foundation::HWND::default());
            assert_eq!(reg_handle.len(), 1, "登録面 register が反映される");
            assert!(reg_handle.remove(e), "登録面 remove が反映される");
            assert!(reg_handle.is_empty());
        }

        // handle drop でワーカ stop/join。ハングせず復帰すれば RAII 健全。
        drop(handle);
    }

    /// task 3.2: `run()` の結線が既存 tick/vsync/shutdown チェーンを壊さないことの回帰確認。
    /// `wire_click_through` を呼んだ後でも空 World の 1 tick が panic せず回る（schedule 健全）。
    /// `wire_new_path`（reconcile 登録）と `wire_click_through`（登録面挿入）を併用しても
    /// World の tick が健全であること＝既存機能の非破壊（R7.2）。
    #[test]
    fn wire_click_through_does_not_break_tick() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");
        app.wire_new_path();
        let (handle, _wake) = app.wire_click_through();

        // 併用後も空 World の 1 tick が panic しない（既存 schedule 非破壊）。
        app.world.borrow_mut().try_tick_world();

        drop(handle);
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
