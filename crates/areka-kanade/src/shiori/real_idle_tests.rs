use super::*;
use crate::msg::EventId;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use areka_actor::{ReplyError, reply_channel};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// 1 本のテストが待つ上限（受信の上限はこの内側に収める）。
const BOUND: Duration = Duration::from_secs(5);
/// 手空きの証拠を待つ上限（保守周期の 4 倍）。
const IDLE_WAIT: Duration = Duration::from_secs(2);

/// 手空きの回数を数え、他スレッドから死活状態を切り替えられる fake backend。
///
/// `get` は id をそのまま返し、`status` は `exited` が立っていれば異常終了を返す。
struct IdleProbeBackend {
    exited: Arc<AtomicBool>,
    idles: Arc<AtomicU32>,
}

impl ShioriBackend for IdleProbeBackend {
    fn get(
        &mut self,
        id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        Ok(Some(id.to_string()))
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
        if self.exited.load(Ordering::SeqCst) {
            HelperStatus::Exited(ExitKind::Terminated)
        } else {
            HelperStatus::Running
        }
    }
    fn on_idle(&mut self) {
        self.idles.fetch_add(1, Ordering::SeqCst);
    }
}

/// 本番の受信ループを素のスレッドで走らせた 1 式（後片付けは `finish`）。
struct Runner {
    tx: Sender<ShioriMsg>,
    on_down_rx: Receiver<KanadeMsg>,
    idles: Arc<AtomicU32>,
    exited: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

fn start_runner() -> Runner {
    let (tx, rx) = mpsc::channel::<ShioriMsg>();
    let (on_down_tx, on_down_rx) = mpsc::channel::<KanadeMsg>();
    let exited = Arc::new(AtomicBool::new(false));
    let idles = Arc::new(AtomicU32::new(0));
    let probe = IdleProbeBackend {
        exited: exited.clone(),
        idles: idles.clone(),
    };
    let handle = std::thread::spawn(move || run_shiori_loop(rx, Box::new(probe), on_down_tx));
    Runner {
        tx,
        on_down_rx,
        idles,
        exited,
        handle,
    }
}

impl Runner {
    /// `idles` が `target` 以上になるまで最長 `limit` 待つ。戻り値は（到達したか・観測値・待った長さ）。
    fn wait_idles(&self, target: u32, limit: Duration) -> (bool, u32, Duration) {
        let start = Instant::now();
        loop {
            let seen = self.idles.load(Ordering::SeqCst);
            if seen >= target {
                return (true, seen, start.elapsed());
            }
            if start.elapsed() >= limit {
                return (false, seen, start.elapsed());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// いま届いている `ShioriDown` を取り出し切って通数を返す（待たない）。
    fn drain_down_count(&self) -> usize {
        std::iter::from_fn(|| self.on_down_rx.try_recv().ok())
            .filter(|m| matches!(m, KanadeMsg::ShioriDown { .. }))
            .count()
    }

    /// 全送信端を drop して受信ループを抜けさせ、有界に join する。
    fn finish(self) {
        drop(self.tx);
        let deadline = Instant::now() + BOUND;
        while !self.handle.is_finished() {
            assert!(
                Instant::now() < deadline,
                "受信ループが送信端の drop 後 {BOUND:?} 以内に終わらなかった"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        self.handle.join().expect("runner joins");
    }
}

#[test]
fn on_idle_is_called_while_idle() {
    let runner = start_runner();
    let (reached, seen, waited) = runner.wait_idles(1, IDLE_WAIT);
    let down = runner.on_down_rx.try_recv().is_ok();
    runner.finish();
    assert!(
        reached,
        "何も送らずに {waited:?} 待ったが手空きの通知が届かなかった（観測した手空き回数={seen}・\
         上限={IDLE_WAIT:?}・保守周期={IDLE_INTERVAL:?}）"
    );
    assert!(!down, "死活は正常なのに ShioriDown が届いた");
}

#[test]
fn requests_after_idle_are_served_in_order() {
    let runner = start_runner();
    let (reached, seen, waited) = runner.wait_idles(1, IDLE_WAIT);
    if !reached {
        runner.finish();
        panic!(
            "要求を送る前に手空きの証拠を {waited:?} 待ったが届かなかった（観測した手空き回数={seen}・\
             上限={IDLE_WAIT:?}）——手空きを挟んだ順序を主張できない"
        );
    }

    let ids = ["OnIdleOrderA", "OnIdleOrderB", "OnIdleOrderC"];
    let receivers: Vec<_> = ids
        .into_iter()
        .map(|id| {
            let (reply, receiver) = reply_channel::<ShioriOutcome>();
            let call = ShioriCall::Get {
                id: EventId::Static(id),
                references: Vec::new(),
                status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
            };
            runner
                .tx
                .send(ShioriMsg::Request { call, reply })
                .expect("send Request");
            receiver
        })
        .collect();

    // 3 件の応答待ちは 1 つの期限を共有する（全体を BOUND 以内に収める）。
    let deadline = Instant::now() + BOUND;
    let echoed: Vec<Result<String, String>> = receivers
        .into_iter()
        .map(|receiver| {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match receiver.recv_timeout(remaining) {
                Ok(ShioriOutcome::Value(v)) => Ok(v),
                Ok(_) => Err("Value 以外の応答".to_string()),
                Err(ReplyError::Timeout) => Err(format!("{BOUND:?} 以内に応答が届かなかった")),
                Err(ReplyError::Dropped) => Err("応答されずに切断された".to_string()),
            }
        })
        .collect();
    runner.finish();

    let expected: Vec<Result<String, String>> = ids.iter().map(|id| Ok(id.to_string())).collect();
    assert_eq!(
        echoed, expected,
        "手空き（観測回数={seen}・{waited:?} 待機）を挟んだ後の連投 3 件が到着順に処理されなかった"
    );
}

#[test]
fn helper_exit_during_idle_reports_shiori_down_once() {
    let runner = start_runner();
    runner.exited.store(true, Ordering::SeqCst);

    // 何も送らずに 1 通目を待つ。
    let start = Instant::now();
    let first = runner.on_down_rx.recv_timeout(IDLE_WAIT);
    let first_waited = start.elapsed();
    let first_is_down = matches!(first, Ok(KanadeMsg::ShioriDown { .. }));
    if !first_is_down {
        let idles = runner.idles.load(Ordering::SeqCst);
        runner.finish();
        panic!(
            "helper を異常終了させ何も送らずに {first_waited:?} 待ったが ShioriDown が届かなかった\
             （受信結果={:?}・観測した手空き回数={idles}・上限={IDLE_WAIT:?}・保守周期={IDLE_INTERVAL:?}）",
            first.map(|_| "ShioriDown 以外の通知")
        );
    }

    // 手空きの腕が 2 回以上回った後で 2 通目が無いことを確かめる。
    let idles_before = runner.idles.load(Ordering::SeqCst);
    let (reached, idles_after, waited) = runner.wait_idles(idles_before + 2, IDLE_WAIT);
    let extra_down = runner.drain_down_count();
    runner.finish();
    assert!(
        reached,
        "1 通目の後、手空きが 2 回回る証拠を {waited:?} 待ったが得られなかった\
         （idles_before={idles_before}・idles_after={idles_after}・上限={IDLE_WAIT:?}）——2 通目が無いことを主張できない"
    );
    assert_eq!(
        extra_down, 0,
        "死活報告が 1 通に留まらなかった（1 通目は {first_waited:?} で到着・追加の ShioriDown={extra_down} 通・\
         idles_before={idles_before}・idles_after={idles_after}）"
    );
}

#[test]
fn no_liveness_report_after_clean_unload_even_when_idle() {
    let runner = start_runner();

    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    runner
        .tx
        .send(ShioriMsg::Unload { reply })
        .expect("send Unload");
    let unloaded = receiver.recv_timeout(BOUND);
    if !matches!(unloaded, Ok(ShioriOutcome::Unloaded)) {
        runner.finish();
        panic!(
            "Unload の往復で Unloaded が得られなかった（受信結果が Unloaded 以外・上限={BOUND:?}）"
        );
    }

    // 正規終了の後に helper が消えても、手空きの腕は黙っていなければならない。
    runner.exited.store(true, Ordering::SeqCst);
    let idles_before = runner.idles.load(Ordering::SeqCst);
    let (reached, idles_after, waited) = runner.wait_idles(idles_before + 2, IDLE_WAIT);
    let down = runner.drain_down_count();
    runner.finish();
    assert!(
        reached,
        "正規終了の後、手空きが 2 回回る証拠を {waited:?} 待ったが得られなかった\
         （idles_before={idles_before}・idles_after={idles_after}・ShioriDown={down} 通・上限={IDLE_WAIT:?}）\
         ——報告が来ないことを主張できない"
    );
    assert_eq!(
        down, 0,
        "正規終了の後なのに手空き中に死活報告が届いた（ShioriDown={down} 通・\
         idles_before={idles_before}・idles_after={idles_after}・{waited:?} 待機）"
    );
}
