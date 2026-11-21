//! runtime レイヤ。UI スレッド基盤の owner となる公開 facade `WinApp` を提供する。
//!
//! `WinApp` はプロセス初期化（`CoInitializeEx(COINIT_MULTITHREADED)`・DPI awareness 設定）と
//! `EcsWorld`（`Rc<RefCell<EcsWorld>>`）生成を統括する。クラス登録はライブラリ
//! （`wintf-winmsg-executor`）が担うため、本層では行わない。
//!
//! 依存方向は COM→ECS→Runtime。runtime は ecs に依存してよいが、ecs は runtime に
//! 依存しない。

use std::cell::RefCell;
use std::rc::Rc;

use tracing::{debug, info, warn};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::HiDpi::*;
use windows::core::Result;

use crate::ecs::clickthrough::{
    ClickThroughHandle, ClickThroughRegistry, ClickThroughRegistryHandle, start_click_through,
};
use crate::ecs::world::{EcsWorld, EcsWorldSelfRef, FrameFinalize};
use crate::runtime::message_loop::ShutdownPolicy;

use crate::runtime::tick_bridge::{VsyncEventBridge, run_async_tick};
use crate::runtime::window_registry::{WindowRegistry, reconcile_window_registry};
use crate::runtime::wndproc_bridge::WndState;
pub use message_loop::AppExit;
pub use window_factory::WindowCreationFailed;

/// `WinApp` が World へ確保する本番 `WindowRegistry` の具体型。
///
/// 既定の保持値 `Window<WndState>`（ライブラリ型・`!Send`＝UI スレッド束縛）で単相化した
/// NonSend リソース型。`new()`/`get_non_send` の型推論を確定させるために用いる。
type ProdWindowRegistry = WindowRegistry<crate::executor::util::Window<WndState>>;

#[cfg(test)]
#[path = "dpi_once_tests.rs"]
mod dpi_once_tests;

/// メッセージループ層。終了の受け口 `AppExit` と、ライブラリ（`wintf-winmsg-executor`）の
/// `block_on` へ渡す shutdown future（`ShutdownPolicy`）を提供する。
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
/// `pub(crate)`: `WinApp::run` が `reconcile_window_registry` を schedule へ
/// 登録する（runtime→World）ため、クレート内から参照可能にする。
pub(crate) mod window_registry;

/// ECS ウィンドウ生成ファクトリ層。宣言的ウィンドウをライブラリの `util::Window::new_ex` で
/// World を借りたまま作り（生成時のメッセージはウィンドウ手続きの借用中の道で読み飛ばす）、
/// 生成後に style/pos/title を反映して `WindowRegistry` へ格納する `EcsWindowFactory` を提供する。
///
/// `pub(crate)`: `create_windows`（ECS 層）が `EcsWindowFactory::create_window` を呼ぶ
/// 唯一の上向きエッジ（ecs→runtime）のため、クレート内から参照可能にする。
pub(crate) mod window_factory;

/// 構築時に選ぶ「窓の数が 0 になったら終了するか」（[`WinApp::with_exit_policy`]）。
///
/// 構築時に消費され、`WinApp` のフィールドにも World にも残らない——実行中に変える口は無い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitPolicy {
    /// 最後の窓が閉じたら終了する（既定・[`WinApp::new`] の選択）。
    OnLastWindowClose,
    /// 明示の指示（[`AppExit::request_exit`]）でだけ終了する。窓 0 になってもループは回り続け、
    /// 後から新しい窓を開ける。
    Explicit,
}

/// プロセスの既定の DPI の扱いを、モニタごとの v2 に 1 回だけ立てる（2 回目からは何もしない）。
///
/// 成否はどちらも 1 行残す。起動の前に画面を出すときは UI を作る前にこれを呼び、
/// [`WinApp::with_exit_policy`] の呼び出しは何もしない（2 回目の `ERROR_ACCESS_DENIED` の
/// warn を出さない）。
pub fn ensure_process_dpi_awareness() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // SAFETY: Win32 境界。SetProcessDpiAwarenessContext はプロセスグローバルな
        // DPI awareness をスレッドセーフに設定する。既に設定済み／不可の場合はエラーを
        // 返すが、レガシー同様にプロセス起動は止めない（致命的ではない）。
        //
        // 戻り値は握り潰さず、成否のどちらも 1 行残す。残さないと per-monitor v2 の設定失敗が
        // 観測できず、`WM_DPICHANGED` が来ない原因の切り分けができない。
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
    });
}

/// UI スレッドの COM を MTA で初期化する。
///
/// 同じアパートメントで初期化済み（`S_FALSE`）は黙って `Ok`。別のアパートメントで初期化済み
/// （`RPC_E_CHANGED_MODE`）なら `warn!` を 1 行残して `Ok`（WUC と D2D の物はそのアパートメントの
/// 上で作られる）。ほかの失敗は `Err`。
fn init_ui_com() -> Result<()> {
    // SAFETY: Win32 境界。CoInitializeEx は呼び出しスレッドの COM を初期化するのみで、
    // 引数 None・COINIT_MULTITHREADED は定数。
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if hr == RPC_E_CHANGED_MODE {
        warn!(
            "[WinApp::new] UI スレッドは別の COM の部屋で初期化済み（MTA にできない・その部屋のまま続ける）"
        );
        return Ok(());
    }
    hr.ok()
}

#[cfg(test)]
#[path = "ui_com_init_tests.rs"]
mod ui_com_init_tests;

/// UI スレッド基盤の owner（公開 facade）。
///
/// COM 初期化・DPI awareness 設定・`EcsWorld` 生成を統括し、共有 World ハンドルの
/// 唯一の strong 所有者となる。VSync スレッド・メッセージループの駆動は [`WinApp::run`] が持つ。
pub struct WinApp {
    /// 共有 ECS world。`WinApp` が唯一の strong 所有者（`world()` は strong clone を返す）。
    world: Rc<RefCell<EcsWorld>>,
    /// 終了の指示の受け口（「指示済み」の 1 ビット＋終了シグナル・**runtime=WinApp 所有**）。
    ///
    /// `WindowRegistry`/`reconcile_window_registry` は runtime 配置ゆえ、
    /// 空遷移 hook からの指示も runtime→runtime で完結し上向き依存が生じない。
    ///
    /// `with_exit_policy()` で生成し、clone を World へ NonSend として据え、既定の選択なら
    /// `WindowRegistry` の shutdown hook に `request_exit()` を仕込む。`run()` が
    /// `block_on(ShutdownPolicy::shutdown_future(self.exit.clone()))` で待つ。
    exit: AppExit,
}

impl WinApp {
    /// 既定の選択（[`ExitPolicy::OnLastWindowClose`]＝最後の窓が閉じたら終了）で構築する。
    ///
    /// [`WinApp::with_exit_policy`] への委譲。
    pub fn new() -> Result<Self> {
        Self::with_exit_policy(ExitPolicy::OnLastWindowClose)
    }

    /// 「窓 0 で終了するか」を選んで COM/DPI 初期化・World 生成を行う。
    ///
    /// 選択は構築時に消費し、以後は参照しない（実行中に変える口は無い）。
    ///
    /// COM は `COINIT_MULTITHREADED` で初期化する。初期化済みの場合は成功扱いで、別の
    /// アパートメント（`RPC_E_CHANGED_MODE`）なら `warn!` を 1 行残す（`init_ui_com`）。DPI awareness は
    /// per-monitor v2 を設定する。
    ///
    /// NOTE: CoInitializeEx の成功に対し Drop で CoUninitialize を呼ばない。
    /// プロセス常駐の単一インスタンス運用では実害なし。
    pub fn with_exit_policy(policy: ExitPolicy) -> Result<Self> {
        init_ui_com()?;

        ensure_process_dpi_awareness();

        // 呼び出しスレッド＝UI スレッド（メッセージループを回す本体）の役割をここで 1 度だけ
        // 宣言してスレッド名簿へ載せる。COM/DPI の初期化の後・World 生成の前に置くのは、
        // `new()` が返る前に必ず 1 度通る点として最も早いため。
        // 失敗しても起動は止めない——このスレッドの CPU が `unregistered_rest` へ回るだけで、
        // 観測の道具の不調が観測対象を止める理由にはならない。
        if let Err(e) = crate::ecs::world::thread_registry::register_current_thread(
            crate::ecs::world::thread_registry::ROLE_UI,
        ) {
            tracing::error!(error = %e, "[WinApp::new] スレッド名簿への登録に失敗した（起動は継続）");
        }

        let world = Rc::new(RefCell::new(EcsWorld::new()));

        let exit = AppExit::new();

        Self::wire_shutdown_hook(&world, &exit, policy);

        debug!(
            ?policy,
            "WinApp initialized (COM/DPI ready, world created, exit wired per policy)"
        );

        Ok(Self { world, exit })
    }

    /// `WindowRegistry` と終了の指示の受け口（NonSend）を World へ確保し、既定の選択なら
    /// 空遷移 hook に終了の指示を注入する。
    ///
    /// 本番型 `WindowRegistry<Window<WndState>>`（`runtime/window_registry.rs` 既定）が未挿入
    /// なら default を挿入し（`create_windows` より前でも安全）、受け口の clone を World へ
    /// 据える（**両方の選択で**・利用側が `&mut World` から明示の指示を出す口）。
    /// [`ExitPolicy::OnLastWindowClose`] のときだけ shutdown hook に `exit.request_exit()` を
    /// 呼ぶクロージャを仕込む（既定の「最後の窓が閉じたら終了」も明示の指示と同じ口を通る＝
    /// 終了の完了機構は 1 本）。[`ExitPolicy::Explicit`] では hook を差さないので、空遷移の
    /// reconcile は何もしない。
    fn wire_shutdown_hook(world: &Rc<RefCell<EcsWorld>>, exit: &AppExit, policy: ExitPolicy) {
        let mut ecs = world.borrow_mut();
        let w = ecs.world_mut();
        if w.get_non_send::<ProdWindowRegistry>().is_none() {
            w.insert_non_send(ProdWindowRegistry::new());
        }
        w.insert_non_send(exit.clone());
        if policy == ExitPolicy::Explicit {
            return;
        }
        let on_empty = exit.clone();
        let mut reg = w
            .get_non_send_mut::<ProdWindowRegistry>()
            .expect("WindowRegistry was just ensured present");
        reg.set_shutdown_hook(move || {
            on_empty.request_exit();
        });
    }

    /// 共有 ECS world ハンドルを返す。
    ///
    /// 返す `Rc` は単一 World（複数ウィンドウで共有）への strong clone。
    pub fn world(&self) -> Rc<RefCell<EcsWorld>> {
        Rc::clone(&self.world)
    }

    /// 新経路の結線（self-ref 注入 + reconcile 登録）を World へ施す。
    ///
    /// `run()` 開始時に 1 度呼ぶ（冪等ではない・下の「重複登録について」）:
    /// - `EcsWorldSelfRef`（外側 `Weak<RefCell<EcsWorld>>`）を NonSend リソースとして
    ///   insert する（`create_windows` がこれを読んで新経路/旧経路を分岐）。既存なら上書き
    ///   （同一 World への Weak ゆえ等価）。
    /// - `reconcile_window_registry::<Window<WndState>>`（本番単相化）を **遅め schedule**
    ///   `FrameFinalize` へ登録する。
    ///
    /// # reconcile を `FrameFinalize` に置く理由
    /// `try_tick_world`（`ecs/world/mod.rs`）は 13 本スケジュールを `Input … FrameFinalize`
    /// の順で回し、本クレートは `clear_trackers`/`clear_all` を **どこでも呼ばない**。bevy の `RemovedComponents<Window>` は `clear_trackers` まで
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
        w.insert_non_send(EcsWorldSelfRef(weak));
        drop(ecs);

        self.world.borrow_mut().add_systems(
            FrameFinalize,
            reconcile_window_registry::<crate::executor::util::Window<WndState>>,
        );
    }

    /// クリック透過機構（ワーカ＋判定ループ）を起動し、登録面（NonSend リソース）を World へ据える。
    ///
    /// `run()` の結線点から呼ぶ。`wire_new_path` と同じく、メッセージループ非実行下（headless）で
    /// 単体検証できるよう **ヘルパーに切り出す**。
    ///
    /// [`start_click_through`] には `self.world` の `Weak` を渡す（shutdown で `upgrade()` が
    /// None になりループ終了、ワーカは返り値 [`ClickThroughHandle`] の drop で stop/join）。
    /// 登録面は [`ClickThroughRegistryHandle`]（`pub` NonSend リソース）で、areka の `run_setup` が
    /// `get_non_send::<ClickThroughRegistryHandle>()` で取得して 2 窓を登録する。
    ///
    /// # wake event を VSync とは別に専用化する理由（起床契機・post-tick）
    /// 機構は二重起床する: (a) カーソル移動 notify（ワーカが `start` 内で `wake_event` を叩く）、
    /// (b) VSync tick 毎の再評価（静止カーソルでも表示更新＝αマスク変化に追随、`JustEnded`
    /// 再収束）。ここで wake_event を **VSync event とは別 `Arc`** に保ち、VSync 起床は
    /// [`Self::run`] の relay タスクが「vblank を待って `wake_event.notify`」する形で中継する
    /// （下記 relay 参照）。こうすることで既存 tick cadence を一切変えない（カーソル移動 notify が
    /// tick のタスクを余計に起こして ECS tick を増やすことがない＝入力と描画 cadence を非結合）。
    ///
    /// # post-tick の構造保証
    /// UI 全体が単一スレッドで、判定コア（`evaluate_targets`）は `&World` を **同期借用**する
    /// （await を跨がず、tick が `borrow_mut` を保持する間は借用しない）。よって評価は常に
    /// settled な World（tick 途中の中間状態ではない）を読む。起床順序に関わらず最悪でも
    /// 1 フレーム遅延に留まる。
    ///
    /// 戻り値: `(handle, wake_event)`。`handle` は `run()` がローカル保持（drop=停止/join）、
    /// `wake_event` は VSync relay タスクの結線に使う。
    fn wire_click_through(
        &self,
        running: Rc<std::cell::Cell<bool>>,
    ) -> (ClickThroughHandle, std::sync::Arc<event_listener::Event>) {
        let registry = Rc::new(RefCell::new(ClickThroughRegistry::new()));
        let wake_event = std::sync::Arc::new(event_listener::Event::new());

        let handle = start_click_through(
            Rc::downgrade(&self.world),
            Rc::clone(&registry),
            std::sync::Arc::clone(&wake_event),
            running,
        );

        {
            let mut ecs = self.world.borrow_mut();
            ecs.world_mut()
                .insert_non_send(ClickThroughRegistryHandle::new(Rc::clone(&registry)));
        }

        debug!("ClickThrough wired (worker + eval loop started, registry handle inserted)");
        (handle, wake_event)
    }

    /// UI スレッドのメッセージループを開始する。
    ///
    /// 新経路の結線（[`wire_new_path`](Self::wire_new_path)）・`VsyncEventBridge` と vblank 駆動の
    /// tick ループの起動・クリック透過機構の起動を済ませ、
    /// `block_on(ShutdownPolicy::shutdown_future(self.exit.clone()))` で、終了の指示
    /// （利用側の [`AppExit::request_exit`]、または registry 空遷移 hook）まで OS メッセージループを
    /// 駆動する。ループ開始前に出た指示でもただちに戻る。future 完了でループが **先行 quit せず**
    /// 正常復帰する（"received unexpected quit message" panic を構造的に回避）。
    ///
    /// 戻った直後に「ループはまだ回っているか」の旗を下ろす。以後に残っていた起床が配られても、
    /// tick タスク・VSync 中継・クリック透過の判定ループはフレームも判定も回さずに終わる。
    /// 続けて窓の登録表を World から取り出して破棄し、残っていた窓を壊す（残っていたときだけ
    /// `info!` を 1 行残す）。ローカルの `VsyncEventBridge`（VSync スレッド stop→join）と tick の
    /// `JoinHandle` は復帰時に drop される。
    ///
    /// # 契約: 指示で戻るとき
    /// 終了の指示で戻るときは、登録表に残っていた窓を壊してから戻る。戻った後の `run()` は
    /// 再入できない（登録表が World に無い）。残っていた窓の entity の資源（`WindowHandle`・
    /// WUC）は `WinApp` の drop まで World に残るので、窓は閉じてから指示すること
    /// （[`AppExit`] の契約）。
    ///
    /// # 戻り値
    /// 正常終了時 `Ok(())`。COM/DPI は `new()` で初期化済み。
    pub fn run(&self) -> Result<()> {
        // このスレッドを UI スレッドとして控える（ほかのスレッドの窓の書き込みは捨てる）。
        crate::ecs::window::record_ui_thread();
        self.wire_new_path();

        let bridge = VsyncEventBridge::new();

        // `running` は block_on から戻った直後に下ろす旗。
        let running = Rc::new(std::cell::Cell::new(true));
        let _tick = crate::executor::spawn_local(run_async_tick(
            bridge.event().clone(),
            Rc::downgrade(&self.world),
            Rc::clone(&running),
        ));

        // `_click_through` は run の間ローカル保持し、run 復帰（shutdown）で drop する
        // （ワーカ stop/join・判定ループは `running` が下りた後の起床で終了）。
        let (_click_through, click_wake) = self.wire_click_through(Rc::clone(&running));

        // vblank 毎に click_wake を notify する中継（静止カーソル時も post-tick 再評価を起こす）。
        // JoinHandle は捨てても executor 上で走り続ける。
        let _relay = crate::executor::spawn_local(crate::runtime::tick_bridge::relay_vblank(
            bridge.event().clone(),
            Rc::downgrade(&self.world),
            std::sync::Arc::clone(&click_wake),
            Rc::clone(&running),
        ));

        crate::executor::block_on(ShutdownPolicy::shutdown_future(self.exit.clone()));

        // 以後の起床ではフレームを回さない。戻った後に利用側のモーダルなダイアログ等が
        // 残っていた起床のメッセージを配っても、登録表を抜いた World を回さない
        // （クリック透過の判定も同じ）。登録表を取り除く前に下ろすこと。
        running.set(false);

        // 残存窓の破棄。tick の外で指示が出ると、reconcile より先にここへ来て窓が
        // 残り得る。登録表ごと取り出して drop し、`Window<WndState>` の drop で
        // `DestroyWindow` を呼ぶ。World を借りたまま壊すのは reconcile と同じ条件で、
        // ウィンドウ手続きは `try_borrow` 失敗で読み飛ばす。
        {
            let mut ecs = self.world.borrow_mut();
            if let Some(registry) = ecs.world_mut().remove_non_send::<ProdWindowRegistry>()
                && !registry.is_empty()
            {
                info!(
                    "[WinApp::run] exit requested while windows remained open — destroying them before returning"
                );
                drop(registry);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_listener::Listener;

    /// 作った窓を確かめる前に隠す（`create_window` は `SW_SHOW` で出し、前面を奪いうる）。
    fn hide_created_window(w: &bevy_ecs::world::World, entity: bevy_ecs::entity::Entity) {
        use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
        if let Some(handle) = w.get::<crate::ecs::window::WindowHandle>(entity) {
            // SAFETY: 作ったばかりの生きた HWND を隠すだけ（戻り値は前の可視の状態）。
            let _ = unsafe { ShowWindow(handle.hwnd, SW_HIDE) };
        }
    }

    /// `WinApp::new()` は呼び出しスレッド（＝UI スレッド）を役割名 `ui` で名簿へ登録する。
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

    /// `WinApp::new()` の shutdown 結線:
    /// 1. `WindowRegistry`（NonSend）が World に確保される。
    /// 2. その shutdown hook が発火すると WinApp 所有の終了シグナル `Event` が notify され、
    ///    `listen()` した待機が起床する（= run() が待つ shutdown future が完了する経路）。
    ///
    /// 本番型 `Window<WndState>` は実 HWND が要るため、headless では reconcile を直接駆動せず
    /// hook を直接発火して結線を確かめる。
    #[test]
    fn new_wires_registry_shutdown_hook_to_notify_event() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");

        // 終了シグナルへ先に listen() を arm（hook 発火 notify を取りこぼさない）。
        let listener = app.exit.signal().listen();

        {
            let world = app.world();
            let ecs = world.borrow();
            let reg = ecs
                .world()
                .get_non_send::<ProdWindowRegistry>()
                .expect("WinApp::new should ensure a WindowRegistry NonSend resource");
            reg.fire_shutdown_hook();
        }

        assert!(
            listener
                .wait_timeout(std::time::Duration::from_secs(5))
                .is_some(),
            "空遷移 hook が終了シグナルを notify するべき（止まったままにせず赤にする）"
        );
    }

    /// `wire_new_path` 後の World 状態を検証する（headless）。
    /// 1. `EcsWorldSelfRef`（NonSend・外側 Weak）が World に注入される（create_windows の
    ///    新経路分岐スイッチ）。その Weak は WinApp の strong World を upgrade できる。
    /// 2. `reconcile_window_registry` が `FrameFinalize` へ登録され、空 World の 1 tick が
    ///    panic せず回る（reconcile 登録が schedule を壊さない＝結線が健全）。
    ///
    /// 実 block_on ループ（実窓 + close イベント要）は回さない。
    #[test]
    fn wire_new_path_injects_self_ref_and_schedules_reconcile() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");
        app.wire_new_path();

        {
            let world = app.world();
            let ecs = app.world.borrow();
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

        app.world.borrow_mut().try_tick_world();
    }

    /// 統合テスト: **close→reconcile→shutdown チェーン end-to-end**。
    ///
    /// `new_wires_registry_shutdown_hook_to_notify_event` は hook を直接発火する seam テスト。
    /// 本テストは実 `Window<WndState>`（実 HWND）を WinApp の World へ生成し、その `Window`
    /// コンポーネントを除去 → 実 `reconcile_window_registry`（schedule 駆動）が registry を
    /// 空へ遷移させ → 注入済み shutdown hook が WinApp 所有 `Event` を notify し → arm 済み
    /// listener が **ハングせず** 起床する、という close→shutdown の本番チェーンを実 reconcile
    /// で貫通検証する。
    ///
    /// `run()` の `block_on(shutdown_future)` が最後の窓クローズで **先行 quit せず**
    /// future 完了で正常復帰する経路の核（最後の窓 close → Event notify）の結線を確かめる。
    ///
    /// 実 HWND の生成は窓を出す（`create_window` は `ShowWindow(SW_SHOW)` を呼ぶ）ので、
    /// 作った直後に `SW_HIDE` で隠す。メッセージループは回さず reconcile を直接 schedule 実行する。
    #[test]
    fn close_to_reconcile_to_shutdown_chain_wakes_listener() {
        use crate::ecs::window::{Window, WindowHandle, WindowStyle};
        use crate::ecs::world::EcsWorldSelfRef;
        use crate::runtime::window_factory::EcsWindowFactory;
        use bevy_ecs::schedule::Schedule;
        use windows::Win32::UI::WindowsAndMessaging::{WS_EX_LAYERED, WS_POPUP};

        let app = WinApp::new().expect("WinApp::new should succeed headless");
        app.wire_new_path();

        // 終了シグナルへ先に listen() を arm（reconcile 空遷移の notify を取りこぼさない）。
        let listener = app.exit.signal().listen();

        let weak = Rc::downgrade(&app.world);

        let entity = {
            let world = app.world();
            let mut ecs = world.borrow_mut();
            let w = ecs.world_mut();
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
            hide_created_window(w, entity);
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

        // 最後の窓を close 相当（`Window` コンポーネント除去）にして、実 reconcile を 1 周回す。
        {
            let world = app.world();
            let mut ecs = world.borrow_mut();
            let w = ecs.world_mut();
            w.entity_mut(entity).remove::<Window>();

            let mut sched = Schedule::default();
            sched.add_systems(reconcile_window_registry::<crate::executor::util::Window<WndState>>);
            sched.run(w);

            let reg = w
                .get_non_send::<ProdWindowRegistry>()
                .expect("ProdWindowRegistry が存在するべき");
            assert!(
                reg.is_empty(),
                "最後の Window 除去後 reconcile で registry は空へ遷移するべき"
            );
        }

        assert!(
            listener
                .wait_timeout(std::time::Duration::from_secs(5))
                .is_some(),
            "空遷移 hook が終了シグナルを notify するべき（止まったままにせず赤にする）"
        );
    }

    /// 「明示の指示でだけ終了」の選択: 最後の窓が閉じても終了しない。
    ///
    /// `close_to_reconcile_to_shutdown_chain_wakes_listener` と同じ構築（実 HWND を 1 枚 →
    /// `Window` 除去 → 実 `reconcile_window_registry`）で、選択だけを `Explicit` に変える。
    /// 登録表は空へ遷移するが、通知は来ず指示済みにもならない。続けて新しい窓を開ける
    /// （登録表は空でも `insert` を受ける＝「後で窓を開ける」が成り立つ）。
    /// 既定の選択で同じ構築をする既存テストが listener の起床を固定しているので、
    /// 両方が緑なら空遷移の仕掛けを差すか否かの分岐が効いている。
    #[test]
    fn explicit_policy_keeps_the_loop_alive_when_the_last_window_closes() {
        use crate::ecs::window::{Window, WindowHandle, WindowStyle};
        use crate::runtime::window_factory::EcsWindowFactory;
        use bevy_ecs::schedule::Schedule;
        use windows::Win32::UI::WindowsAndMessaging::{WS_EX_LAYERED, WS_POPUP};

        let app = WinApp::with_exit_policy(ExitPolicy::Explicit)
            .expect("WinApp::with_exit_policy should succeed headless");
        app.wire_new_path();

        assert!(
            app.world
                .borrow()
                .world()
                .get_non_send::<AppExit>()
                .is_some(),
            "Explicit でも受け口は World に据えられるべき"
        );

        let listener = app.exit.signal().listen();
        let weak = Rc::downgrade(&app.world);

        let spawn_window = |title: &str| {
            let world = app.world();
            let mut ecs = world.borrow_mut();
            let w = ecs.world_mut();
            let entity = w
                .spawn((
                    Window {
                        title: title.to_string(),
                        parent: None,
                    },
                    WindowStyle {
                        style: WS_POPUP,
                        ex_style: WS_EX_LAYERED,
                    },
                ))
                .id();
            EcsWindowFactory::create_window(w, entity, weak.clone());
            hide_created_window(w, entity);
            assert!(
                w.get::<WindowHandle>(entity).is_some(),
                "factory 生成後に WindowHandle が付与されるべき"
            );
            entity
        };

        let entity = spawn_window("ExplicitFirst");
        {
            let world = app.world();
            let mut ecs = world.borrow_mut();
            let w = ecs.world_mut();
            w.entity_mut(entity).remove::<Window>();
            let mut sched = Schedule::default();
            sched.add_systems(reconcile_window_registry::<crate::executor::util::Window<WndState>>);
            sched.run(w);
            let reg = w
                .get_non_send::<ProdWindowRegistry>()
                .expect("ProdWindowRegistry が存在するべき");
            assert!(
                reg.is_empty(),
                "最後の Window 除去後 registry は空へ遷移するべき"
            );
        }

        assert!(
            listener
                .wait_timeout(std::time::Duration::from_millis(20))
                .is_none(),
            "Explicit では最後の窓が閉じても終了シグナルが notify されないべき"
        );
        assert!(
            !app.exit.is_requested(),
            "Explicit では最後の窓が閉じても終了を指示されないべき"
        );

        let reopened = spawn_window("ExplicitReopened");
        let world = app.world();
        let ecs = world.borrow();
        let reg = ecs
            .world()
            .get_non_send::<ProdWindowRegistry>()
            .expect("ProdWindowRegistry が存在するべき");
        assert!(!reg.is_empty(), "窓 0 の後に開いた窓が registry に載るべき");
        assert!(ecs.world().get::<WindowHandle>(reopened).is_some());
    }

    /// `wire_click_through` の結線を headless で検証する。
    /// 1. クリック透過機構が起動し（ワーカ＋判定ループ）、返る `ClickThroughHandle` を得る。
    /// 2. 登録面 `ClickThroughRegistryHandle`（NonSend リソース）が World へ挿入される。
    /// 3. 登録面経由の register が反映される（共有レジストリ）。
    /// 4. `ClickThroughHandle` を drop するとワーカが stop/join されハングしない（RAII）。
    ///
    /// フル `run()` のメッセージループ（実 vblank relay 駆動）は回さない（executor 非駆動でも
    /// spawn_local タスクは投入されるだけで安全）。
    #[test]
    fn wire_click_through_starts_and_inserts_registry_handle() {
        use crate::ecs::clickthrough::ClickThroughRegistryHandle;

        let app = WinApp::new().expect("WinApp::new should succeed headless");
        let (handle, _wake) = app.wire_click_through(Rc::new(std::cell::Cell::new(true)));

        // ダミー window Entity（登録面の反映確認用・World 生存は問わない）。
        let e = bevy_ecs::entity::Entity::from_raw_u32(1).expect("valid test entity index");

        {
            let world = app.world();
            let ecs = world.borrow();
            let reg_handle = ecs
                .world()
                .get_non_send::<ClickThroughRegistryHandle>()
                .expect("wire_click_through は ClickThroughRegistryHandle を World へ挿入すべき");
            assert!(reg_handle.is_empty(), "初期の登録面は空");

            reg_handle.register(e, windows::Win32::Foundation::HWND::default());
            assert_eq!(reg_handle.len(), 1, "登録面 register が反映される");
        }

        drop(handle);
    }

    /// `wire_new_path`（reconcile 登録）と `wire_click_through`（登録面挿入）を併用しても、
    /// 空 World の 1 tick が panic せず回る。
    #[test]
    fn wire_click_through_does_not_break_tick() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");
        app.wire_new_path();
        let (handle, _wake) = app.wire_click_through(Rc::new(std::cell::Cell::new(true)));

        app.world.borrow_mut().try_tick_world();

        drop(handle);
    }

    /// 終了シグナルは `WinApp` が strong 所有する（`exit` フィールド存在の確認も兼ねる）。
    /// 構築直後は未 notify ゆえ、新規 arm した listener は即起床しない（false-positive 防止）。
    #[test]
    fn new_owns_unfired_shutdown_signal() {
        let app = WinApp::new().expect("WinApp::new should succeed headless");
        assert!(
            !app.exit.is_requested(),
            "構築直後は終了を指示されていないべき"
        );
        let listener = app.exit.signal().listen();
        assert!(
            listener
                .wait_timeout(std::time::Duration::from_millis(20))
                .is_none(),
            "構築直後（hook 未発火）は終了シグナルが notify されていないべき"
        );
    }
}
