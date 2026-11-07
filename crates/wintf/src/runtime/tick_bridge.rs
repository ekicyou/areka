//! VSync 起床ブリッジ。
//!
//! 専用 VSync スレッドが `DwmFlush()` で vblank を検出し、共有
//! `event_listener::Event` を `notify(usize::MAX)`（全リスナ起床）で notify する。
//! UI スレッド側の待機タスク（tick のタスク [`run_async_tick`]・クリック透過の中継 [`relay_vblank`]）は
//! `bridge.event().listen().await` で 1 フレームごとに起床できる。
//!
//! # 周期
//! `DwmFlush()` は次の vblank までブロックするため、ふだんの周期は画面の更新周期に
//! 追従する。消灯・ロック・リモートデスクトップ・DWM の失敗などで `DwmFlush()` が
//! 待たずに戻った回（呼ぶ前から 1ms 未満）だけ、呼ぶ前から数えて約 16ms になるまで
//! 眠ってから起こす（空回りで CPU を張り付かせない）。起こしは失敗の回も含めて毎周
//! 必ず 1 回撃つ（[`vsync_once`]）。旧 `PostMessageW(WM_VSYNC)` 経路は本ブリッジでは
//! 持たない。
//!
//! # 生存期間（RAII）
//! 本ユニットは自己完結の RAII で、スレッドの生存期間を `stop_flag → join` の
//! 順序規律で管理する（旧 `WinThreadMgrInner::drop` の規律を継承）。`Drop` で
//! stop フラグを立て、スレッドを `join()` する。
//!
//! `WinApp::run` が `run` の間だけローカルに所有し、`run` から戻るときの drop で
//! スレッドを止めて join する。

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use event_listener::Event;
use tracing::{debug, error, trace};
use windows::Win32::Graphics::Dwm::DwmFlush;

use crate::ecs::world::EcsWorld;

/// VSync 専用スレッドの vblank 検出を共有 `event_listener::Event` へ橋渡しする
/// RAII ユニット。
///
/// - 共有 `Event`（`Arc` で clone を VSync スレッドへ move）
/// - 停止フラグ（`Arc<AtomicBool>`）
/// - スレッドハンドル（`Option<JoinHandle<()>>`・take は `Drop`/`stop` のみ）
///
/// リスナは [`VsyncEventBridge::event`] で `Arc<Event>` を取得し `listen()` できる。
pub(crate) struct VsyncEventBridge {
    /// 共有 vblank 通知イベント。VSync スレッドが vblank ごとに `notify(usize::MAX)`。
    vblank_event: Arc<event_listener::Event>,
    /// VSync スレッド停止フラグ。`Drop`/`stop` で `true` をストアしループを終える。
    stop_flag: Arc<AtomicBool>,
    /// VSync スレッドのハンドル。`new` で必ず `Some`、take は `stop`/`Drop` のみ。
    handle: Option<JoinHandle<()>>,
}

impl VsyncEventBridge {
    /// VSync スレッドを起動してブリッジを生成する。
    ///
    /// スレッドは `while !stop { DwmFlush(); event.notify(usize::MAX); }` をループし、
    /// vblank ごとに全リスナを起床する。
    pub(crate) fn new() -> Self {
        let vblank_event = Arc::new(event_listener::Event::new());
        let stop_flag = Arc::new(AtomicBool::new(false));

        // clone を VSync スレッドへ move（Event/stop_flag とも Arc で共有）。
        let thread_event = Arc::clone(&vblank_event);
        let thread_stop = Arc::clone(&stop_flag);

        let handle = thread::Builder::new()
            .name("wintf-vsync".to_owned())
            .spawn(move || vsync_loop(thread_event, thread_stop))
            .expect("failed to spawn wintf-vsync thread");

        debug!("VsyncEventBridge started (wintf-vsync thread spawned)");

        Self {
            vblank_event,
            stop_flag,
            handle: Some(handle),
        }
    }

    /// 共有 vblank 通知イベントへの参照を返す。
    ///
    /// 呼び出し側は `bridge.event().listen()` で `EventListener` を arm できる
    /// （[`run_async_tick`]・テスト用）。返す `Arc` を clone して保持してもよい。
    pub(crate) fn event(&self) -> &Arc<event_listener::Event> {
        &self.vblank_event
    }

    /// VSync スレッドを停止し join する（`stop_flag → join` の順序）。
    ///
    /// 冪等。`Drop` からも呼ばれる。
    ///
    /// # Edge
    /// stop フラグを立てた時点でスレッドが `DwmFlush()` 内で次の vblank まで
    /// park していることがあるため、`join()` は最大 1 フレーム（~リフレッシュ
    /// 周期）待つ。これは許容範囲。
    fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop_flag.store(true, Ordering::Release);
            join_vsync(handle);
            debug!("VsyncEventBridge stopped (wintf-vsync thread joined)");
        }
    }
}

impl Drop for VsyncEventBridge {
    fn drop(&mut self) {
        self.stop();
    }
}

/// vblank のスレッドを待ち合わせる。panic で終わっていたら `error!` を 1 行残す。
fn join_vsync(handle: JoinHandle<()>) {
    if handle.join().is_err() {
        error!("[VsyncEventBridge] wintf-vsync スレッドが panic で終わっていた");
    }
}

/// これより短く戻った `DwmFlush` は「待たずに戻った」とみなす。
const WAITED_AT_LEAST: Duration = Duration::from_millis(1);
/// 待たずに戻った回に、呼ぶ前から数えて合わせる 1 周の長さ。
const FALLBACK_PERIOD: Duration = Duration::from_millis(16);

/// 待たずに戻った回だけ、`FALLBACK_PERIOD` に足りない長さを返す（状態を持たない）。
fn compensation(waited: Duration) -> Option<Duration> {
    (waited < WAITED_AT_LEAST).then(|| FALLBACK_PERIOD - waited)
}

/// vblank のスレッドの 1 周。時刻・`DwmFlush`・眠り方を差し込める。
/// `Err` は debug に HRESULT を 1 行、補った回は trace を 1 行。最後に必ず 1 回 notify する。
fn vsync_once(
    now: &mut impl FnMut() -> Instant,
    flush: impl FnOnce() -> windows::core::Result<()>,
    sleep: impl FnOnce(Duration),
    event: &Event,
) {
    let before = now();
    if let Err(e) = flush() {
        // 熱い経路だが間引かない（待てなかった回は補いで約 16ms ごとに抑えられる）。
        debug!(hresult = ?e.code(), "[vsync_loop] DwmFlush が失敗した");
    }
    let waited = now().saturating_duration_since(before);
    if let Some(rest) = compensation(waited) {
        sleep(rest);
        trace!(
            ?waited,
            ?rest,
            "[vsync_loop] DwmFlush が待たずに戻ったので補って眠った"
        );
    }
    // vblank（または補いの周期）ごとに全リスナを起床（データは搬送しない）。
    event.notify(usize::MAX);
}

/// VSync スレッド本体。`stop` が立つまで [`vsync_once`] を回す。
fn vsync_loop(event: Arc<event_listener::Event>, stop: Arc<AtomicBool>) {
    // 走り始めに 1 度だけ、自分の役割を宣言してスレッド名簿へ載せる（要件 2.3）。
    // 失敗しても vblank 検出は続ける——このスレッドの CPU が `unregistered_rest` へ
    // 回るだけで、観測の道具の不調が観測対象を止める理由にはならない。
    if let Err(e) = crate::ecs::world::thread_registry::register_current_thread(
        crate::ecs::world::thread_registry::ROLE_VBLANK,
    ) {
        error!(error = %e, "[vsync_loop] スレッド名簿への登録に失敗した（vblank 検出は継続）");
    }

    while !stop.load(Ordering::Acquire) {
        vsync_once(
            &mut Instant::now,
            // SAFETY: Win32 境界。`DwmFlush` は引数を取らず、DWM 合成の次フレーム
            // （vblank）まで現スレッドをブロックして `windows::core::Result<()>` を返す。
            // 副作用は本スレッドのブロックのみで、共有状態には触れない。
            || unsafe { DwmFlush() },
            thread::sleep,
            &event,
        );
    }
}

/// 1 フレーム分の ECS tick を同期実行する（独立テスト可能なコア・要件 4.3／3.2）。
///
/// 1. `IS_TICK_FLUSH_IN_PROGRESS` 再入ガードを engage する。既に進行中なら（再入）
///    `false` を返して安全側スキップする（二重 tick なし）。
/// 2. World を `try_borrow_mut` する。借用失敗（再入・別経路が借用中）なら tick せず
///    `false` を返す（安全側スキップ・二重 tick なし）。
/// 3. 借用成功時は tick の門（`EcsWorld::decide_tick`）へ 1 度だけ問い、
///    **回す**なら `try_tick_world()`（13 本スケジュール・順序不変）、**省略**なら
///    `note_skipped_tick()`（数を記録するだけ・`FrameCount`／`FrameTime`／`TickStart` は
///    進めない）を実行する。
/// 4. どちらの側でも `flush_window_pos_commands()` は**常に**呼ぶ。積まれている窓書込
///    指令は、この画面更新で 13 本を回したかどうかとは無関係に掃き出す。
///
/// 門の既定は無効なので、既定の運転では毎フレーム `Run(Disabled)` が返り、挙動は門を
/// 入れる前と同じである（`AREKA_TICK_GATE=1` で有効化する）。
///
/// # 戻り値
/// `true` = このフレームを門が処理した（**回した／省略した**のいずれか）。
/// `false` = 再入ガードまたは World 借用に失敗して何もしなかった。
///
/// `flush_window_pos_commands()` は World 借用解放後に呼ぶ（同期 `WM_WINDOWPOSCHANGED`
/// が World を借用しても競合しない）。流しはガードを持ったまま行う
/// （`flush_window_pos_commands_guarded` と同じ規律）。
pub(crate) fn tick_one_frame(world: &Rc<RefCell<EcsWorld>>) -> bool {
    // 門は 1 フレームに 1 度だけ問う（プロセス共有の旗を読んで倒すのはこの 1 箇所）。
    tick_one_frame_with(world, |world| world.decide_tick(Instant::now()))
}

/// [`tick_one_frame`] の本体。門への問い方だけを差し替えられる形にしてある。
///
/// 手順（ガード → 借用 → 問う → 回す／省略 → 常に flush）は本番と完全に同一で、
/// 違うのは `decide` が何を返すかだけである。本番は
/// [`EcsWorld::decide_tick`](crate::ecs::world::EcsWorld::decide_tick)——プロセス共有の
/// 旗を読んで倒す——を渡す。
///
/// # なぜ差し替え口を置くか
///
/// 旗は**プロセスにただ 1 つ**の実体で、本番の経路（窓書込指令の積み上げ・ウィンドウ
/// メッセージの配送・Z 順の要求・ポインタ入力の投入）が立てる。同じテストバイナリの
/// 中で並列に走る他の検査がそれらの経路を通れば、こちらが旗を倒した直後に旗が立つ
/// ——**省略になるはず**を主張する検査は、それだけで赤くなる。錠では防げない: 旗を
/// 立てる側は本番コードであって錠を知らないからである。
///
/// そこで「省略の側」を確かめる検査は、共有の旗を経由せず、読み取り結果を直に渡す。
/// 「回す側」は旗が余分に立っても結論が変わらないので、共有の経路のままでよい。
fn tick_one_frame_with(
    world: &Rc<RefCell<EcsWorld>>,
    decide: impl FnOnce(&mut EcsWorld) -> crate::ecs::world::tick_gate::TickDecision,
) -> bool {
    // ── 再入防止ガード（`flush_window_pos_commands_guarded` と共有・二重防御）─────────
    let Some(_guard) = crate::ecs::world::engage_tick_flush_guard() else {
        trace!("[tick_one_frame] Re-entry blocked by IS_TICK_FLUSH_IN_PROGRESS");
        return false;
    };

    // RefCell 借用を試みる。既に借用中（再入）なら安全側スキップ（false）。
    let processed = match world.try_borrow_mut() {
        Ok(mut world) => {
            // 毎フレームのログはここでは出さない（熱い経路）——省略率は `[tick]` の
            // 観測行（既定 OFF）が 1 秒窓でまとめて出す。
            if decide(&mut world).is_run() {
                world.try_tick_world();
            } else {
                world.note_skipped_tick();
            }
            true
        }
        Err(_) => {
            // 借用失敗（再入時）— 安全にスキップ。エラーではない。
            false
        }
    };
    // World 借用スコープ終了後に SetWindowPos コマンドをフラッシュ（省略の回も必ず）。
    crate::ecs::window::flush_window_pos_commands();

    processed
    // _guard drop で IS_TICK_FLUSH_IN_PROGRESS = false に戻る
}

/// vblank 駆動の UI スレッド async tick タスク（要件 4.2 / 4.3 / 4.5）。
///
/// `VsyncEventBridge` が共有する `event_listener::Event` の起床通知を待ち、起床ごとに
/// 1 フレーム分の ECS tick（`try_tick_world` の 13 本スケジュール）を実行して再待機する
/// ループ。`WinApp::run` が `spawn_local` で UI スレッドに投入する。複数 tick タスクは不要（単一 World）。
///
/// **固定周期ではない。** 起床は `vsync_loop` の `DwmFlush()`（次 vblank まで待つ）が出す
/// ので、実効のフレーム周期は**画面の更新周期**である（120Hz の実機なら約 8.3ms）。
/// `DwmFlush()` が待たずに戻った回（消灯・ロック・失敗など）だけ、呼ぶ前から約 16ms に
/// なるまで眠ってから起こす（[`vsync_once`]）。この注記は areka-P0-dpi-transition-atomicity task 4.3 が付けた——判定の上限を
/// 「1 コマ」で置くときに 16.7ms を無条件の値と読む誤りが実際に起きたため。
///
/// # 再入の二重防御（要件 4.3）
/// 本タスクの 1 フレーム実行は [`tick_one_frame`] を通し、`IS_TICK_FLUSH_IN_PROGRESS`
/// 再入ガード（`crate::ecs::world::engage_tick_flush_guard`）を engage する。これは
/// ライブラリ側 wndproc 再入防止（`RefCell` の借用チェック）と二重防御になり、
/// 窓の書き込みの流し（`crate::ecs::world::flush_window_pos_commands_guarded`・
/// `WM_WINDOWPOSCHANGED` の後に呼ばれる）と同一フラグを共有し、流しの中から入れ子で
/// 届いた tick と、tick の中から入れ子で届いた流しを読み飛ばす（二重 tick を防ぐ）。
///
/// notify 取りこぼし防止規律（設計）: await の **前** に `listener = event.listen()` を
/// arm する。これにより `tick_one_frame` 実行中に届いた notify も次の `listener.await`
/// で捕捉でき、1 フレームを取りこぼさない。
///
/// 起きたらまず `running`（`WinApp::run` のループがまだ回っているか）を見る。`run()` は
/// `block_on` から戻った直後にこれを下ろし、その後で窓の登録表を World から取り除く。
/// 戻った後も起床のメッセージが残っていれば、利用側のモーダルなダイアログ等がそれを配る
/// ——下りていればフレームを回さずに終える（登録表の無い World を回さない）。
///
/// World は `Weak` のまま待ち、起きてから `upgrade()` する（待ちの間に強参照を握らない）。
/// `None`（strong 所有者が drop 済み）でも終える。
///
/// 自分で終わるのは起床が来たときだけである。正常な終了では `run()` の戻りで vblank の
/// スレッドが止まり起床が来ないので、タスクは待ちのまま残る（`run` 1 回につき 1 度の
/// 小さな残り・World は `Weak` なので握らない）。
pub(crate) async fn run_async_tick(
    event: Arc<Event>,
    world: Weak<RefCell<EcsWorld>>,
    running: Rc<Cell<bool>>,
) {
    debug!("tick task started (vblank-driven UI-thread tick loop)");
    loop {
        // 先に listen() を arm（処理中に届く notify を落とさない）。
        let listener = event.listen();

        // 起床を待機（次 vblank notify まで UI スレッドを譲る）。
        listener.await;

        if !running.get() {
            debug!("tick task stopping (message loop has returned)");
            return;
        }
        let Some(world) = world.upgrade() else {
            debug!("tick task stopping (world dropped — shutdown)");
            return;
        };

        // 1 フレーム実行（13 本スケジュール）。再入・借用失敗は安全側スキップ。
        tick_one_frame(&world);
    }
}

/// vblank ごとにクリック透過の起こしを中継する。World が落ちたか run が戻った後は終わる。
///
/// 先に `listen()` を arm してから待つ（処理中に届く vblank を落とさない）。`wake` を
/// vblank の `Event` とは別の `Arc` に保つので、カーソル移動の起こしが tick タスクを
/// 余計に起こさない。[`run_async_tick`] と同じく、終わるのは起床が来たときだけで、
/// 正常な終了の後は待ちのまま残る（`run` 1 回につき 1 度の小さな残り）。
pub(crate) async fn relay_vblank(
    vblank: Arc<Event>,
    world: Weak<RefCell<EcsWorld>>,
    wake: Arc<Event>,
    running: Rc<Cell<bool>>,
) {
    loop {
        let listener = vblank.listen();
        if world.upgrade().is_none() {
            debug!("ClickThrough VSync relay stopping (world dropped — shutdown)");
            return;
        }
        listener.await;
        // ループが戻った後の起床は中継しない（tick タスクと同じ守り）。
        if !running.get() {
            debug!("ClickThrough VSync relay stopping (message loop has returned)");
            return;
        }
        wake.notify(usize::MAX);
    }
}

#[cfg(test)]
#[path = "tick_bridge_vsync_tests.rs"]
mod vsync_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::world::FrameCount;
    use crate::ecs::world::thread_registry::{self, ROLE_VBLANK, ThreadEntry};
    use crate::ecs::world::tick_gate::TICK_GATE_WARMUP_FRAMES;
    use crate::ecs::world::tick_wake::WakeSnapshot;
    use event_listener::Listener;
    use std::time::Duration;

    /// 実 vblank の壁時計期限に依る 1 本を明示実行へ限る門の環境変数。
    ///
    /// 本ワークスペースの実行時環境変数は `AREKA_` 名前空間である（同じ流儀の先例:
    /// `crates/log-capture-kit/tests/capture_calibration_test.rs` の `MODE_ENV`）。
    /// 隣の実窓の重なり順の門とは別の変数にしてある——赤くなる機序も、走らせるために
    /// 要る条件も違うためである（あちらは他プロセスの可視窓が無いこと、こちらは機械が
    /// 空いていて DWM が動いていること）。
    const VBLANK_DEADLINE_ENV: &str = "AREKA_WINTF_VBLANK_DEADLINE";

    /// 門が開いていなければ、判定を 1 行も実行せずに落とす。
    ///
    /// # なぜ門を付けたか
    ///
    /// 下の 1 本は「500ms 以内に vblank の通知が届く」という壁時計の期限そのものが判定で
    /// ある。機械が混んでいると期限が飢えて赤くなるが、待つ形へ書き換えると判定の意味が
    /// 変わる（＝根治にあたる）。よって既定の `cargo test` からは外し、狙って走らせる
    /// ときだけ走る形にした。
    ///
    /// **判定そのものは 1 行も変えていない**——期限も期待値も隔離前のままである。根治の
    /// 引受先は `.kiro/specs/zorder-chain-residue/brief.md` の A-2（壁時計期限の
    /// 飢餓）である。
    ///
    /// # 門が閉じたまま呼ばれたら緑にしない
    ///
    /// `--ignored` で無差別に呼ばれたときに素通りで緑を返すと、「実 DWM の通知を測った」
    /// という記録に化ける。既存の手本（`crates/areka/src/placement_tests/transition_signoff_tests.rs`
    /// の実機ログ判定）と同じく、条件が揃っていない呼び出しは失敗として落とす。
    fn require_explicit_vblank_run() {
        assert!(
            std::env::var_os(VBLANK_DEADLINE_ENV).is_some(),
            "{VBLANK_DEADLINE_ENV} が未設定である。実 vblank の壁時計期限は負荷で飢えて\
             間欠的に赤くなるため明示実行へ隔離してある（判定は 1 行も走っていない）。\
             走らせるには環境変数を与えて \
             `cargo test -p wintf --lib <テスト名> -- --ignored --exact` とすること"
        );
    }

    /// 名簿に当該役割名の項目が現れるまで待ち、現れた項目を返す（現れなければ `None`）。
    ///
    /// 待ち時間は合否の対象ではなく、登録が起きなかったときに無限に待たないための上限で
    /// ある（要件 6.5 が禁じる「実時間の閾値による判定」ではない）。
    fn wait_for_role(role: &str, limit: Duration) -> Option<ThreadEntry> {
        let deadline = std::time::Instant::now() + limit;
        loop {
            let found = thread_registry::snapshot()
                .into_iter()
                .find(|entry| entry.role == role);
            if found.is_some() {
                return found;
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// vblank 検出スレッドは走り始めに自分をスレッド名簿へ登録する（役割名 `vblank`・要件 2.3）。
    ///
    /// 名簿はプロセス共有で、テストは並列に走る——**全体の件数は当てにせず**、登録された
    /// 役割名の項目だけを検査する。観測はスレッドを生かしたまま行う（終了した TID が別の
    /// スレッドへ再利用されて項目が置き換わる余地を残さないため）。
    #[test]
    fn vsync_thread_registers_itself_with_the_vblank_role() {
        let bridge = VsyncEventBridge::new();

        let found = wait_for_role(ROLE_VBLANK, Duration::from_secs(5))
            .expect("wintf-vsync スレッドは走り始めに vblank として名簿へ載るはず");
        assert_eq!(
            found.name.as_deref(),
            Some("wintf-vsync"),
            "名簿の OS 名は生成時に付けたスレッド名であるはず"
        );
        assert!(
            thread_registry::is_known_role(ROLE_VBLANK),
            "登録した役割名は固定語彙に含まれるはず"
        );

        drop(bridge);
    }

    /// 完了状態の検証: vblank ごとに Event が notify され、待機リスナを起床できる。
    /// さらに drop でスレッドが clean に join することを確認する（ハングしない）。
    #[test]
    #[ignore = "実 vblank の 500ms 期限が負荷で飢えて間欠的に赤くなる（AREKA_WINTF_VBLANK_DEADLINE を与えて明示実行する）"]
    fn vblank_notifies_listener_then_joins_on_drop() {
        require_explicit_vblank_run();

        let bridge = VsyncEventBridge::new();

        // notify/listen レースを避けるため、待機前に listen() を arm する。
        let listener = bridge.event().listen();

        // 500ms 以内に vblank notify が届けば Some（通常 Win10/11 で DWM 常時 ON）。
        let got = listener.wait_timeout(Duration::from_millis(500));
        assert!(
            got.is_some(),
            "expected a vblank notify within 500ms (is DWM running?)"
        );

        // drop で stop→join。スレッドが clean に終われば本テストはハングせず復帰する。
        drop(bridge);
    }

    // ── tick のタスク (task 2.3) ──────────────────────────────────

    /// 再入安全スキップ（要件 4.3）: World が既に `borrow_mut` 済みのとき
    /// `tick_one_frame` は tick せず `false` を返す（二重 tick なし・panic なし）。
    ///
    /// この経路は World 借用に失敗するため 13 本スケジュールを実行せず、
    /// GPU/window 不要でヘッドレス実行できる。
    #[test]
    fn tick_one_frame_safe_skips_when_world_borrowed() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let world = Rc::new(RefCell::new(crate::ecs::world::EcsWorld::new()));

        // World を借用したまま tick_one_frame を呼ぶ → try_borrow_mut が失敗。
        let _held = world.borrow_mut();
        let ticked = tick_one_frame(&world);

        assert!(
            !ticked,
            "borrow_mut が握られている間は安全スキップで false を返すべき"
        );
        // ガードは借用失敗後に確実に解放されていること（リーク無し）。
        assert!(
            !crate::ecs::world::is_tick_flush_in_progress(),
            "安全スキップ後は再入ガードが false に戻っているべき"
        );
    }

    // ── tick の門の結線 (task 3.3) ────────────────────────────────

    /// 起動直後の全走ぶんだけ判定を進める（回さずに数だけ進める）。
    fn fast_forward_warmup(world: &Rc<RefCell<crate::ecs::world::EcsWorld>>) {
        let mut world = world.borrow_mut();
        for _ in 0..TICK_GATE_WARMUP_FRAMES {
            let decision = world.decide_tick_with(WakeSnapshot {
                bits: 0,
                deadline_due: false,
            });
            assert!(decision.is_run(), "起動直後は無条件で回すべき");
        }
    }

    /// (要件 3.2) 門が無効（既定）なら、1 フレームは今までどおり 13 本を回す
    /// ——フレーム番号がちょうど 1 進む。
    #[test]
    fn tick_one_frame_ticks_the_world_when_the_gate_is_disabled() {
        // 門が無効でも `decide_tick` は共有の旗を読んで**倒す**——自分の合否は旗に
        // 依らないが、倒した旗は他の検査から奪ったことになる。だから読む側も錠を取る。
        let _lock = crate::ecs::world::TICK_WAKE_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let world = Rc::new(RefCell::new(crate::ecs::world::EcsWorld::new()));
        assert!(
            !world.borrow().tick_gate_enabled(),
            "門の既定は無効であるべき"
        );

        let before = world.borrow().world().resource::<FrameCount>().0;
        let processed = tick_one_frame(&world);

        assert!(processed, "借用できたフレームは処理済みとして true を返す");
        assert_eq!(
            world.borrow().world().resource::<FrameCount>().0,
            before + 1,
            "門が無効なら 13 本を回してフレーム番号が 1 進む"
        );
    }

    /// (要件 3.2/3.4) 門が有効で回す理由が 1 つも無ければ、1 フレームは 13 本を回さない
    /// ——フレーム番号は進まないが、関数は「このフレームは処理した」として true を返す。
    ///
    /// # 共有の旗を経由しない理由
    ///
    /// 旗はプロセスにただ 1 つで、**本番の経路**（窓書込指令の積み上げ・ウィンドウ
    /// メッセージの配送・Z 順の要求・ポインタ入力の投入）が立てる。同じテストバイナリの
    /// 中で並列に走る他の検査がそれらを通れば、こちらが旗を倒した直後に旗が立ち得る
    /// ——錠を取っても防げない（立てる側は本番コードで、錠を知らない）。「省略になる
    /// はず」を主張できるのは読み取り結果を直に渡したときだけなので、ここは
    /// [`tick_one_frame_with`] へ旗ゼロ・期限なしの読み取り結果を渡す。手順（ガード →
    /// 借用 → 問う → 回す／省略 → 常に flush）は本番と同一である。
    #[test]
    fn tick_one_frame_skips_the_world_tick_when_the_gate_finds_no_reason() {
        let world = Rc::new(RefCell::new(crate::ecs::world::EcsWorld::new()));
        world.borrow_mut().set_tick_gate(true);
        fast_forward_warmup(&world);

        let before = world.borrow().world().resource::<FrameCount>().0;
        // 省略の回にも掃き出しが走ることを、積んだ 1 本がキューから消えることで見る
        // （HWND は空なので書込は失敗し `warn!` が出るだけ・キューはスレッドごと）。
        crate::ecs::window::SetWindowPosCommand::enqueue(
            crate::ecs::window::SetWindowPosCommand::new(
                windows::Win32::Foundation::HWND::default(),
                0,
                0,
                1,
                1,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
                None,
            ),
        );
        let processed = tick_one_frame_with(&world, |world| {
            world.decide_tick_with(WakeSnapshot {
                bits: 0,
                deadline_due: false,
            })
        });

        assert!(
            processed,
            "省略も「このフレームは門が処理した」なので true を返す"
        );
        assert_eq!(
            world.borrow().world().resource::<FrameCount>().0,
            before,
            "省略の回は 13 本を回さないのでフレーム番号は進まない"
        );
        assert!(
            crate::ecs::window::drain_window_pos_commands().is_empty(),
            "省略の回も窓書込の掃き出しが走り、積んだ指令が残らないべき（D-3）"
        );
    }

    /// 再入ガードのトグル（要件 4.3）: `engage_tick_flush_guard` のスコープ中は
    /// `IS_TICK_FLUSH_IN_PROGRESS` が true、drop 後に false へ戻る。さらに
    /// 進行中の再 engage は `None`（安全スキップ）になる。
    #[test]
    fn reentry_guard_toggles_and_blocks_nested_engage() {
        assert!(!crate::ecs::world::is_tick_flush_in_progress());

        let guard = crate::ecs::world::engage_tick_flush_guard()
            .expect("最初の engage はガードを獲得できる");
        assert!(
            crate::ecs::world::is_tick_flush_in_progress(),
            "engage スコープ中は true"
        );

        // 再入: 進行中は None を返す（二重 tick 防止）。
        assert!(
            crate::ecs::world::engage_tick_flush_guard().is_none(),
            "進行中の再 engage は None（安全スキップ）であるべき"
        );

        drop(guard);
        assert!(
            !crate::ecs::world::is_tick_flush_in_progress(),
            "guard drop 後は false へ戻る"
        );
    }

    // ── run() 復帰後に回さない（areka-P0-shiori-fault-notice task 7.2） ──

    /// `run()` が戻った後に届いた起床では、tick タスクはフレームを回さずに終わる。
    /// 待ちの間は World の強参照を握らない。
    ///
    /// 本番では `run()` 復帰後に利用側が出したモーダルなダイアログが、残っていた起床の
    /// メッセージを配り、登録表の抜けた World でフレームが回って panic した（2026-09-25 実機）。
    /// `block_on` は投入順（メッセージの到着順）にタスクを回すので、通知の後の
    /// `block_on(async {})` は「起床したタスクを 1 度回してから戻る」になる。
    #[test]
    fn tick_task_stops_without_a_frame_once_the_loop_has_returned() {
        let _lock = crate::ecs::world::TICK_WAKE_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let world = Rc::new(RefCell::new(crate::ecs::world::EcsWorld::new()));
        let event = Arc::new(Event::new());
        let running = Rc::new(Cell::new(true));
        let frame = || world.borrow().world().resource::<FrameCount>().0;

        let _task = crate::executor::spawn_local(run_async_tick(
            Arc::clone(&event),
            Rc::downgrade(&world),
            Rc::clone(&running),
        ));
        crate::executor::block_on(async {}); // 待ちを立てさせる
        let held_while_waiting = Rc::strong_count(&world) - 1;

        // 対照: ループが回っている間の起床は 1 フレーム回す（起床が届く仕掛けの確認）。
        let before = frame();
        event.notify(usize::MAX);
        crate::executor::block_on(async {});
        assert_eq!(
            frame(),
            before + 1,
            "ループが回っている間の起床は 1 フレーム回すはず"
        );

        // ループが戻った後の起床。
        running.set(false);
        let before = frame();
        event.notify(usize::MAX);
        crate::executor::block_on(async {});
        let advanced_after_return = frame() - before;

        assert_eq!(
            (held_while_waiting, advanced_after_return),
            (0, 0),
            "(待ちの間に握っていた World の強参照の数, ループが戻った後に回ったフレーム数) は両方 0 のはず"
        );
    }
}
