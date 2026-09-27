//! 受信ループが見張り部品（[`ShioriProbe`]）へ今の呼び出しを書く位置と、アクターの起動が解く手を
//! 据えることの判定（areka-P0-session-mark-residue 要件 1.3・2.2・2.4）。
//!
//! 書く位置は、backend の往復の内側で見張り部品を読み（往復の最中の値）、応答を受け取った後に
//! もう一度読む（往復の後の値）ことで確かめる。受信ループは応答を送る前に書き終えるので、
//! 応答の受け取りの後の読みは時刻に依らない。

use super::*;
use crate::msg::EventId;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use areka_actor::reply_channel;
use shiori_host32_host::ShutdownError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::shiori::probe::{ShioriCut, WaitBudget};

const BOUND: Duration = Duration::from_secs(5);

/// 往復の最中に見張り部品を読んで残す偽の backend。
struct BusyRecordingBackend {
    probe: ShioriProbe,
    seen: Arc<Mutex<Vec<ShioriBusy>>>,
    unload_ok: bool,
    unblock: Option<ShioriUnblock>,
}

impl BusyRecordingBackend {
    fn record(&self) {
        self.seen.lock().unwrap().push(self.probe.busy());
    }
}

impl ShioriBackend for BusyRecordingBackend {
    fn get(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.record();
        Ok(None)
    }
    fn notify(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.record();
        Ok(())
    }
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.record();
        if self.unload_ok {
            Ok(ExitKind::Clean)
        } else {
            Err(ShutdownError::ExitTimeout)
        }
    }
    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        self.unblock.clone()
    }
}

/// 解く手の既定（`None`）のまま上書きしない偽の backend（既存の backend の実装と同じ形）。
struct NoUnblockBackend;

impl ShioriBackend for NoUnblockBackend {
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
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        Ok(())
    }
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }
    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
}

fn get_call(id: &'static str) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static(id),
        references: Vec::new(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

fn notify_call(id: &'static str) -> ShioriCall {
    ShioriCall::Notify {
        id: EventId::Static(id),
        references: Vec::new(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

fn request(tx: &Sender<ShioriMsg>, call: ShioriCall) {
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Request { call, reply })
        .expect("send Request");
    receiver.recv_timeout(BOUND).expect("要求に応答が届く");
}

fn unload(tx: &Sender<ShioriMsg>) {
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Unload { reply }).expect("send Unload");
    receiver.recv_timeout(BOUND).expect("Unload に応答が届く");
}

/// 呼ばれた回数を数える解く手。
fn counting_unblock() -> (ShioriUnblock, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let unblock: ShioriUnblock = Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    (unblock, calls)
}

/// 見張りの決め手をテストのスレッドで直に呼び、その結果を返す（上限は十分に遠い）。
fn cut_by_hand(probe: &ShioriProbe) -> Option<ShioriCut> {
    probe.fire_now(WaitBudget {
        started: Instant::now(),
        limit: Duration::from_secs(60),
    })
}

/// 要求の前後・降ろす前・降ろした後で今の呼び出しを書く。降ろす処理は成否を問わず
/// `Unloaded` を残す。
#[test]
fn the_loop_writes_the_current_call_around_each_round_trip() {
    for unload_ok in [true, false] {
        let probe = ShioriProbe::default();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let backend = BusyRecordingBackend {
            probe: probe.clone(),
            seen: Arc::clone(&seen),
            unload_ok,
            unblock: None,
        };
        let (tx, rx) = mpsc::channel::<ShioriMsg>();
        let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
        let loop_probe = probe.clone();
        let handle = std::thread::spawn(move || {
            run_shiori_loop(rx, Box::new(backend), on_down_tx, loop_probe)
        });

        assert_eq!(
            probe.busy(),
            ShioriBusy::Idle,
            "unload_ok={unload_ok}: 始まり"
        );
        request(&tx, get_call("OnBoot"));
        assert_eq!(
            probe.busy(),
            ShioriBusy::Idle,
            "unload_ok={unload_ok}: GET の後"
        );
        request(&tx, notify_call("OnClose"));
        assert_eq!(
            probe.busy(),
            ShioriBusy::Idle,
            "unload_ok={unload_ok}: NOTIFY の後"
        );
        unload(&tx);
        assert_eq!(
            probe.busy(),
            ShioriBusy::Unloaded,
            "unload_ok={unload_ok}: 降ろした後は成否を問わず Unloaded"
        );

        drop(tx);
        handle.join().expect("runner joins");
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                ShioriBusy::Request("OnBoot".into()),
                ShioriBusy::Request("OnClose".into()),
                ShioriBusy::Unload,
            ],
            "unload_ok={unload_ok}: 往復の最中の値"
        );
    }
}

/// 接続の成功後、backend が差し出した解く手が見張り部品に据わり、発火で 1 回呼ばれる。
#[test]
fn the_actor_installs_the_backends_unblock_handle_after_connecting() {
    let (unblock, calls) = counting_unblock();
    let backend = BusyRecordingBackend {
        probe: ShioriProbe::default(),
        seen: Arc::new(Mutex::new(Vec::new())),
        unload_ok: true,
        unblock: Some(unblock),
    };
    let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle, probe) = spawn_shiori_actor(
        move || Ok(Box::new(backend) as Box<dyn ShioriBackend>),
        on_down_tx,
    );
    // 往復が 1 回返れば、受信ループの前にある据える処理は済んでいる。
    request(&tx, get_call("OnBoot"));

    let cut = cut_by_hand(&probe);
    let _ = tx.send(ShioriMsg::Close);
    handle.join().expect("shiori actor joins");
    assert_eq!(
        cut,
        Some(ShioriCut {
            stage: "idle",
            unblocked: true
        })
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1, "解く手は 1 回呼ばれる");
}

/// 解く手を持たない backend（既定の `None`）では「解く手なし」が据わる。後から別の手を据えようと
/// しても効かないことで、据わっていないのではなく「なし」が据わったことを確かめる。
#[test]
fn a_backend_without_an_unblock_handle_installs_none() {
    let (on_down_tx, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle, probe) = spawn_shiori_actor(
        || Ok(Box::new(NoUnblockBackend) as Box<dyn ShioriBackend>),
        on_down_tx,
    );
    request(&tx, get_call("OnBoot"));

    let (late, calls) = counting_unblock();
    probe.install_unblock(Some(late));
    let cut = cut_by_hand(&probe);
    let _ = tx.send(ShioriMsg::Close);
    handle.join().expect("shiori actor joins");
    assert_eq!(
        cut,
        Some(ShioriCut {
            stage: "idle",
            unblocked: false
        })
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "後から据えた手は呼ばれない"
    );
}

/// 接続に失敗したら「解く手なし」が据わる（死活報告より前に据える）。
#[test]
fn a_connect_failure_installs_none() {
    let (on_down_tx, on_down_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle, probe) = spawn_shiori_actor(|| Err("boom".to_string()), on_down_tx);
    assert!(
        matches!(
            on_down_rx.recv_timeout(BOUND),
            Ok(KanadeMsg::ShioriDown { .. })
        ),
        "接続の失敗の死活報告が届く"
    );

    let (late, calls) = counting_unblock();
    probe.install_unblock(Some(late));
    let cut = cut_by_hand(&probe);
    drop(tx);
    handle.join().expect("shiori actor joins");
    assert_eq!(
        cut,
        Some(ShioriCut {
            stage: "idle",
            unblocked: false
        })
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "後から据えた手は呼ばれない"
    );
}
