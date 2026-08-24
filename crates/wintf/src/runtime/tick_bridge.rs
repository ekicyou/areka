//! VSync 起床ブリッジ。
//!
//! 専用 VSync スレッドが `DwmFlush()` で vblank を検出し、共有
//! `event_listener::Event` を `notify(usize::MAX)`（全リスナ起床）で notify する。
//! UI スレッド側の待機タスク（後続タスク 2.3 の `AsyncTickTask`）は
//! `bridge.event().listen().await` で 1 フレームごとに起床できる。
//!
//! # 周期
//! `DwmFlush()` は次の vblank までブロックするため、ループは固定 16.67ms の
//! sleep を持たずモニターのリフレッシュレートに自然追従する（実 vblank 同期・
//! 要件 4.4）。旧 `PostMessageW(WM_VSYNC)` 経路は本ブリッジでは持たない（要件 4.1）。
//!
//! # 生存期間（RAII）
//! 本ユニットは自己完結の RAII で、スレッドの生存期間を `stop_flag → join` の
//! 順序規律で管理する（旧 `WinThreadMgrInner::drop` の規律を継承）。`Drop` で
//! stop フラグを立て、スレッドを `join()` する。
//!
//! 最終的にはこのインスタンスを `WinApp` が所有する（設計: 「`Event` は `WinApp`
//! が所有」）。`WinApp::run` の全結線（後続タスク 4.3）でフィールドとして保持され、
//! `WinApp` の drop に生存期間が委譲される。本タスクの時点では結線は行わず、
//! ブリッジ自身が自前のスレッドを所有し Drop で join する。

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Instant;

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
    /// （`AsyncTickTask`・テスト用）。返す `Arc` を clone して保持してもよい。
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
            if handle.join().is_err() {
                trace!("wintf-vsync thread join returned Err (thread panicked)");
            }
            debug!("VsyncEventBridge stopped (wintf-vsync thread joined)");
        }
    }
}

impl Drop for VsyncEventBridge {
    fn drop(&mut self) {
        self.stop();
    }
}

/// VSync スレッド本体。`DwmFlush()` で vblank を待ち、毎回 `Event` を全リスナ起床
/// で notify する。`stop` が立つまでループする。
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
        // SAFETY: Win32 境界。`DwmFlush` は引数を取らず、DWM 合成の次フレーム
        // （vblank）まで現スレッドをブロックして `windows::core::Result<()>` を返す。
        // 副作用は本スレッドのブロックのみで、共有状態には触れない。
        let flushed = unsafe { DwmFlush() };

        match flushed {
            Ok(()) => {
                // vblank 到来 — 全リスナを起床（複数 tick タスク対応・
                // データは搬送しない起床通知のみ）。
                event.notify(usize::MAX);
            }
            Err(_) => {
                // DWM 無効など稀な失敗。スピンホットを避けるためおおよそ 60Hz
                // 相当だけ待ってからリトライする（本環境では通常到達しない）。
                thread::sleep(std::time::Duration::from_millis(15));
            }
        }
    }
}

/// vblank 駆動の UI スレッド async tick タスク（要件 4.2 / 4.3 / 4.5）。
///
/// `VsyncEventBridge` が共有する `event_listener::Event` の起床通知を待ち、起床ごとに
/// 1 フレーム分の ECS tick（`try_tick_world` の 13 本スケジュール）を実行して再待機する
/// ループを `spawn_local` で UI スレッドに投入する。複数 tick タスクは不要（単一 World）。
///
/// **固定周期ではない。** 起床は `vsync_loop` の `DwmFlush()`（次 vblank まで待つ）が出す
/// ので、実効のフレーム周期は**画面の更新周期**である（120Hz の実機なら約 8.3ms）。
/// 固定 60Hz なのは DWM が使えないときのフォールバック（`vsync_loop` の `Err` 腕）だけで
/// ある。この注記は areka-P0-dpi-transition-atomicity task 4.3 が付けた——判定の上限を
/// 「1 コマ」で置くときに 16.7ms を無条件の値と読む誤りが実際に起きたため。
///
/// # 再入の二重防御（要件 4.3）
/// 本タスクの 1 フレーム実行は [`tick_one_frame`] を通し、`IS_TICK_FLUSH_IN_PROGRESS`
/// 再入ガード（`crate::ecs::world::engage_tick_flush_guard`）を engage する。これは
/// ライブラリ側 wndproc 再入防止（`RefCell` の借用チェック）と二重防御になり、
/// nested-message やレガシー `try_tick_on_vsync` 経路と同一フラグを共有して二重 tick を防ぐ。
///
/// # 結線
/// 本タスクの `WinApp::run` への結線は後続タスク（run 全結線）で行う。本タスクは spawn
/// エントリのみを提供する。
pub(crate) struct AsyncTickTask;

impl AsyncTickTask {
    /// vblank 駆動の tick ループを UI スレッドの `spawn_local` で投入する。
    ///
    /// - `event`: `VsyncEventBridge::event()` から得た共有 vblank 通知（`Arc<Event>`）。
    /// - `world`: 共有 World への `Weak`（強参照を持たない・設計 State Management）。
    ///   shutdown 時は `upgrade()` が `None` を返し、ループは安全に終了する。
    ///
    /// 投入のみ行い実行はメッセージループ（`block_on`/`MessageLoop::run`）に委ねる。
    // `WinApp::run`（task 4.3 結線済み）が呼ぶ。
    pub(crate) fn spawn(
        event: Arc<Event>,
        world: Weak<RefCell<EcsWorld>>,
    ) -> crate::executor::JoinHandle<()> {
        crate::executor::spawn_local(run_async_tick(event, world))
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
/// が World を借用しても競合しない）。レガシー `try_tick_on_vsync` と同じ規律。
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
    // ── 再入防止ガード（レガシー vsync.rs と共有・二重防御）─────────
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

/// vblank 駆動の tick ループ本体（`spawn_local` で UI スレッドに駆動される async タスク）。
///
/// notify 取りこぼし防止規律（設計）: await の **前** に `listener = event.listen()` を
/// arm する。これにより `tick_one_frame` 実行中に届いた notify も次の `listener.await`
/// で捕捉でき、1 フレームを取りこぼさない。World の `Weak` は毎フレーム `upgrade()` し、
/// `None`（shutdown で strong 所有者が drop 済み）なら安全にループを終了する。
async fn run_async_tick(event: Arc<Event>, world: Weak<RefCell<EcsWorld>>) {
    debug!("AsyncTickTask started (vblank-driven UI-thread tick loop)");
    loop {
        // 先に listen() を arm（処理中に届く notify を落とさない）。
        let listener = event.listen();

        // strong 所有者が生存しているか確認。None なら shutdown — 終了。
        let Some(world) = world.upgrade() else {
            debug!("AsyncTickTask stopping (world dropped — shutdown)");
            return;
        };

        // 起床を待機（次 vblank notify まで UI スレッドを譲る）。
        listener.await;

        // 1 フレーム実行（13 本スケジュール）。再入・借用失敗は安全側スキップ。
        tick_one_frame(&world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::world::FrameCount;
    use crate::ecs::world::thread_registry::{self, ROLE_VBLANK, ThreadEntry};
    use crate::ecs::world::tick_gate::TICK_GATE_WARMUP_FRAMES;
    use crate::ecs::world::tick_wake::WakeSnapshot;
    use event_listener::Listener;
    use std::time::Duration;

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
    fn vblank_notifies_listener_then_joins_on_drop() {
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

    // ── AsyncTickTask (task 2.3) ──────────────────────────────────

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
        // 窓書込の掃き出しはガードが外れる**前**に呼ばれる。ガードが戻っていることは
        // 「省略の回でも関数が flush まで通り抜けた」ことの証跡である（D-3）。
        assert!(
            !crate::ecs::world::is_tick_flush_in_progress(),
            "省略の回も flush まで進んでから再入ガードが外れているべき"
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
}
