//! 期限つきで降ろす入口（[`GhostRuntime::shutdown_within`]）のテスト（要件 1.1・1.6・2.1・3.3）。
//!
//! 偽の SHIORI は `OnClose` の通知で「解かれるまで固まる」ことができ、解く手の呼ばれた回数を
//! 数える。見張りは手動の口（`cut_now`）で起こすので、結果は時刻に依らない。
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use areka_kanade::{CloseReason, ShioriBusy, ShioriCut, ShioriUnblock, WaitBudget};
use areka_sakura::contract::{CueSink, TalkCue};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use temp_path_kit::TempPath;

/// 解く手と固まる往復が共有する置き場（解かれたか・解く手の回数）。
#[derive(Default)]
struct Gate {
    released: Mutex<bool>,
    wake: Condvar,
    unblock_calls: AtomicUsize,
}

/// `hold_on_close` なら `OnClose` の通知で解かれるまで固まり、解かれたら期限切れの失敗を返す
/// （補助プロセスを終わらせた往復が kanade に見える形）。解く手を差し出すと `ready` へ知らせる。
struct HoldingShiori {
    gate: Arc<Gate>,
    hold_on_close: bool,
    ready: Sender<()>,
    holding: Sender<()>,
}

impl ShioriBackend for HoldingShiori {
    fn get(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        Ok(None)
    }

    fn notify(
        &mut self,
        id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        if !(self.hold_on_close && id == "OnClose") {
            return Ok(());
        }
        let _ = self.holding.send(());
        let mut released = self.gate.released.lock().expect("gate poisoned");
        while !*released {
            released = self.gate.wake.wait(released).expect("gate poisoned");
        }
        Err(RequestError::Timeout)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }

    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }

    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        let gate = Arc::clone(&self.gate);
        let unblock: ShioriUnblock = Arc::new(move || {
            gate.unblock_calls.fetch_add(1, Ordering::SeqCst);
            *gate.released.lock().expect("gate poisoned") = true;
            gate.wake.notify_all();
            Ok(())
        });
        let _ = self.ready.send(());
        Some(unblock)
    }
}

#[derive(Clone)]
struct NoopSink;

impl CueSink for NoopSink {
    fn emit(&mut self, _cue: TalkCue) {}
}

fn write_fixture(root: &std::path::Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");
    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");
}

struct Booted {
    runtime: GhostRuntime,
    gate: Arc<Gate>,
    holding: Receiver<()>,
    _temp: TempPath,
}

/// 偽の SHIORI で起動し、解く手が据わるまで待ってから返す（見張りが解く手を必ず見つける）。
fn boot_holding(tag: &str, hold_on_close: bool) -> Booted {
    let temp = TempPath::new(&format!("ghost-runtime-within-{tag}"));
    write_fixture(temp.path());
    let gate = Arc::new(Gate::default());
    let (ready_tx, ready_rx) = mpsc::channel();
    let (holding_tx, holding_rx) = mpsc::channel();
    let backend_gate = Arc::clone(&gate);
    let options = GhostBootOptions {
        ghost_root: temp.path().to_path_buf(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(HoldingShiori {
                gate: backend_gate,
                hold_on_close,
                ready: ready_tx,
                holding: holding_tx,
            }) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(Box::new(SystemVarSnapshot::default)),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };
    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    ready_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the shiori actor should install the unblock handle after connecting");
    Booted {
        runtime,
        gate,
        holding: holding_rx,
        _temp: temp,
    }
}

fn budget(limit: Duration) -> WaitBudget {
    WaitBudget {
        started: Instant::now(),
        limit,
    }
}

/// 別スレッドで走らせ、期限内に戻らなければ失敗にする（固まったままテスト全体を止めない）。
fn bounded<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(20))
        .unwrap_or_else(|_| panic!("'{what}' did not return within 20s (hang)"))
}

/// 上限の中で降り切れば今日どおり `Ok` で、打ち切りは無く、解く手も呼ばれない（要件 1.6・2.6）。
#[test]
fn shutdown_within_returns_no_cut_when_teardown_finishes_inside_the_limit() {
    let Booted { runtime, gate, .. } = boot_holding("inside-limit", false);

    let (result, cut) = bounded("shutdown_within", move || {
        runtime.shutdown_within(CloseReason::System, budget(Duration::from_secs(60)))
    });

    assert!(
        result.is_ok(),
        "teardown should succeed as today, got {result:?}"
    );
    assert_eq!(
        cut, None,
        "no cut when the teardown finishes inside the limit"
    );
    assert_eq!(gate.unblock_calls.load(Ordering::SeqCst), 0);
}

/// `OnClose` の通知で固まった SHIORI を見張りが解き、残りの後始末を今日どおり走らせて戻る
/// （要件 1.1・2.1）。段の語は `on_close_notify`、解く手は 1 回。
#[test]
fn shutdown_within_cuts_a_held_on_close_and_finishes_the_teardown() {
    let Booted {
        runtime,
        gate,
        holding,
        ..
    } = boot_holding("held-on-close", true);
    let probe = runtime.shiori_probe().clone();
    std::thread::spawn(move || {
        if holding.recv_timeout(Duration::from_secs(10)).is_ok() {
            probe.cut_now();
        }
    });

    let (result, cut) = bounded("shutdown_within", move || {
        runtime.shutdown_within(CloseReason::System, budget(Duration::from_secs(60)))
    });

    assert!(
        result.is_ok(),
        "the remaining teardown should still join cleanly, got {result:?}"
    );
    assert_eq!(
        cut,
        Some(ShioriCut {
            stage: "on_close_notify",
            unblocked: true,
        })
    );
    assert_eq!(gate.unblock_calls.load(Ordering::SeqCst), 1);
}

/// 既存の降ろし方は見張りを張らない（要件 3.3）: 手動の口の予約を置いてから `shutdown` しても
/// 解く手は呼ばれず、予約は消費されずに残る。残った予約は、後から張った見張りを即発火させる
/// （張っていれば予約はその場で消費されるので、この発火は起きない）。
#[test]
fn plain_shutdown_never_arms_the_watchdog() {
    let Booted { runtime, gate, .. } = boot_holding("plain-never-arms", false);
    let probe = runtime.shiori_probe().clone();
    probe.cut_now();

    let result = bounded("shutdown", move || runtime.shutdown(CloseReason::System));
    assert!(
        result.is_ok(),
        "shutdown should succeed as today, got {result:?}"
    );
    assert_eq!(
        gate.unblock_calls.load(Ordering::SeqCst),
        0,
        "plain shutdown must not arm the watchdog, so the reserved cut never fires"
    );

    // 予約がまだ残っていること: 張った時点で即発火し、解く手が呼ばれるまで待てる。
    probe.set_busy(ShioriBusy::Idle);
    let guard = probe.arm(budget(Duration::from_secs(60)));
    let released = gate.released.lock().expect("gate poisoned");
    let (released, _) = gate
        .wake
        .wait_timeout_while(released, Duration::from_secs(10), |released| !*released)
        .expect("gate poisoned");
    assert!(
        *released,
        "the reservation left by plain shutdown should fire as soon as the watchdog is armed"
    );
    drop(released);
    assert_eq!(
        guard.finish(),
        Some(ShioriCut {
            stage: "idle",
            unblocked: true,
        })
    );
    assert_eq!(gate.unblock_calls.load(Ordering::SeqCst), 1);
}
