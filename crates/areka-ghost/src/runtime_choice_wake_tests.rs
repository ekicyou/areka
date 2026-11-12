//! 選択待ちの起こしを kanade → 起こしの中継 → 本物の刻み役 → `KanadeMsg::ChoiceTimeoutDue` と
//! 1 本で追うテスト（audit-kanade #7・要件 9.5）。
//!
//! 刻み役の時刻は注入した値だけで、毎秒の `Tick` の周期は試験中に届かないほど長くする。だから
//! `OnChoiceTimeout` が出るのは起こしの中継が刻み役へ頼みを渡したときだけ。待ちは SHIORI が
//! 受けた GET の知らせを期限付きで待つ 1 か所だけ。
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use temp_path_kit::TempPath;

use crate::test_support::{fake_options, shutdown_bounded, write_ghost};

/// 起動の挨拶に選択肢 1 つの台本を返し、GET の ID を知らせる偽の SHIORI。ほかは 204。
struct ChoiceShiori(mpsc::Sender<String>);

impl ShioriBackend for ChoiceShiori {
    fn get(
        &mut self,
        id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        let _ = self.0.send(id.to_string());
        Ok(matches!(id, "OnBoot" | "OnFirstBoot").then(|| r"\0\q[a,OnA]\e".to_string()))
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

/// 時刻を 1 回に進める幅（既定の選択の期限 30 秒より長い＝頼みの後の 1 歩で期限を越える）。
const STEP_MS: u64 = 60_000;

/// **起こしの中継が刻み役へ頼みを渡すと、時刻を進めるだけで `OnChoiceTimeout` が出る**。
///
/// # 弁別
/// - 起こしの中継をつながない（線を作らない・頼みを刻み役へ渡さない）と、`Tick` が来ないので
///   `OnChoiceTimeout` は出ず、期限で落ちる。
#[test]
fn choice_timeout_reaches_kanade_through_the_relay_and_the_real_ticker() {
    let temp = TempPath::new("ghost-runtime-choice-timeout-through-the-relay");
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let now = Arc::new(AtomicU64::new(1_000));
    let clock_now = Arc::clone(&now);
    let (ids_tx, ids_rx) = mpsc::channel::<String>();
    let options = GhostBootOptions {
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(ChoiceShiori(ids_tx)) as Box<dyn ShioriBackend>)
        })),
        ticker: TickerMode::Real(crate::ticker::TickerConfig {
            // 毎秒の Tick は試験中の時刻（高々 数時間）では来ない。
            kanade_interval: Duration::from_secs(30 * 24 * 60 * 60),
            clock: Box::new(move || areka_kanade::MonotonicMs(clock_now.load(Ordering::SeqCst))),
            ..crate::ticker::TickerConfig::default()
        }),
        ..fake_options(root)
    };
    let runtime = boot(options).expect("boot (Real)");

    // 頼みがいつ刻み役へ着いても、その後の 1 歩で期限を越える。
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = Vec::new();
    while !seen.iter().any(|id| id == "OnChoiceTimeout") {
        assert!(
            Instant::now() < deadline,
            "OnChoiceTimeout が出ない（起こしの中継がつながっていない？）: {seen:?}"
        );
        now.fetch_add(STEP_MS, Ordering::SeqCst);
        match ids_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(id) => seen.push(id),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => panic!("SHIORI が先に止まった: {seen:?}"),
        }
    }

    shutdown_bounded(runtime);
    drop(temp);
}
