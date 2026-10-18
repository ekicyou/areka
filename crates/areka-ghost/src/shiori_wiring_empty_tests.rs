//! 何を聞かれても中身なしで答える SHIORI（`EmptyShiori`・`--option dontloadmodule`）の
//! 決定論テスト（spec `areka-P0-startup-options-core` タスク 4.1・要件 7.2〜7.5・11.4）。
//!
//! 窓も 32bit 成果物も実 SHIORI も要さない。起動のテストは `shiori` の行の無いゴーストを
//! 差し替え口（`ShioriWiring::Custom(Box::new(empty_connect()))`）で起こし、起動と終了の
//! 問い合わせが中身なしで返り、「終了」の 204 で終わる（`unload` まで進む）ことを見る。
//! ゴーストの置き場はワークツリーの `target\` の下だけ（`TempPath::under_target`）。
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use areka_kanade::{CloseReason, KanadeMsg, MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::contract::{CueSink, SystemVarSnapshot, TalkCue};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use temp_path_kit::TempPath;

use super::{EmptyShiori, empty_connect};
use crate::dispatcher::DispatcherMsg;
use crate::runtime::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};

/// 4 つの答え（要件 7.2・7.5）: 問い合わせ→中身なし・知らせ→成功・終わり→きれいな終了・
/// 状態→動いている。問い合わせの名前や Reference に依らない。
#[test]
fn empty_shiori_answers_no_content_success_clean_and_running() {
    let mut shiori = EmptyShiori;
    let refs = vec!["a".to_string(), "b".to_string()];
    for id in [
        "OnBoot",
        "OnClose",
        "username",
        "OnTranslate",
        "OnSecondChange",
    ] {
        assert!(
            matches!(shiori.get(id, &refs, Some("talking")), Ok(None)),
            "{id}"
        );
        assert!(matches!(shiori.get(id, &[], None), Ok(None)), "{id}");
        assert!(matches!(shiori.notify(id, &refs, None), Ok(())), "{id}");
    }
    assert_eq!(shiori.status(), HelperStatus::Running);
    assert!(matches!(shiori.unload(), Ok(ExitKind::Clean)));
}

/// 差し替え口の閉包は呼ぶたびに `EmptyShiori` を返し、失敗しない（窓もプロセスも作らない）。
#[test]
fn empty_connect_yields_empty_backend() {
    let mut backend = empty_connect()().expect("empty_connect never fails");
    assert!(matches!(backend.get("OnBoot", &[], None), Ok(None)));
    assert_eq!(backend.status(), HelperStatus::Running);
}

/// 問い合わせとその答え・`unload` の到達を記録へ流す包み（中身は `empty_connect` のもの）。
struct Recording {
    inner: Box<dyn ShioriBackend>,
    seen: Sender<Seen>,
}

#[derive(Debug, PartialEq)]
enum Seen {
    Get(String, Option<String>),
    /// `unload` の答え（`Err` は `None`）。
    Unload(Option<ExitKind>),
}

impl ShioriBackend for Recording {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        let answer = self.inner.get(id, references, status);
        if let Ok(value) = &answer {
            let _ = self.seen.send(Seen::Get(id.to_string(), value.clone()));
        }
        answer
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.inner.notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        let result = self.inner.unload();
        let _ = self.seen.send(Seen::Unload(result.as_ref().ok().copied()));
        result
    }

    fn status(&mut self) -> HelperStatus {
        self.inner.status()
    }
}

#[derive(Clone)]
struct NoopSink;

impl CueSink for NoopSink {
    fn emit(&mut self, _cue: TalkCue) {}
}

/// `shiori` の行の無いゴースト（要件 7.4）を差し替え口で起こすと起動は成功し、起動の
/// 問い合わせ（`OnBoot`）は中身なし（要件 7.3）、「終了」の問い合わせ（`OnClose`）も中身
/// なしで、その 204 で終了の系列へ進み `unload` がきれいな終了を返す（要件 7.5）。
#[test]
fn ghost_without_shiori_line_boots_and_closes_through_empty_connect() {
    let temp = TempPath::under_target("ghost-shiori-wiring-empty-boot");
    let root = temp.path();
    let ghost_master = root.join("ghost").join("master");
    let shell_master = root.join("shell").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::create_dir_all(&shell_master).expect("create shell/master");
    // `shiori,` の行を持たない（読める SHIORI のファイルが無い）ゴースト。
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,NoShioriGhost\n",
    )
    .expect("write ghost descript.txt");
    std::fs::write(
        shell_master.join("descript.txt"),
        "charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");

    let (seen_tx, seen_rx) = channel();
    let connect = empty_connect();
    let options = GhostBootOptions {
        ghost_root: root.to_path_buf(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            let inner = connect()?;
            Ok(Box::new(Recording {
                inner,
                seen: seen_tx,
            }) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(Box::new(SystemVarSnapshot::default)),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot must succeed without a shiori line");
    // 起動の問い合わせが答えられたら「終了」を頼み、終了の系列が `unload` へ届くまで、
    // 合成の時刻で Tick を注入しつつ記録を集める（壁時計の期限は宙吊りを防ぐ上限だけ）。
    let boot_answered = Seen::Get("OnBoot".to_string(), None);
    let mut seen = Vec::new();
    let mut close_sent = false;
    let mut now = 1;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !seen.iter().any(|s| matches!(s, Seen::Unload(_))) && Instant::now() < deadline {
        while let Ok(s) = seen_rx.try_recv() {
            seen.push(s);
        }
        if !close_sent && seen.contains(&boot_answered) {
            runtime
                .kanade()
                .send(KanadeMsg::CloseRequest {
                    reason: CloseReason::User { scope: 0 },
                })
                .expect("kanade alive after boot");
            close_sent = true;
        }
        // 初回の起動は起動の記録だけのトークを再生するので、再生の側にも時刻を送る。
        let _ = runtime.dispatcher().send(DispatcherMsg::Tick {
            now: MonotonicMs(now),
        });
        let _ = runtime.kanade().send(KanadeMsg::Tick {
            now: MonotonicMs(now),
        });
        now += 1000;
        std::thread::sleep(Duration::from_millis(5));
    }

    assert!(
        seen.contains(&boot_answered),
        "OnBoot must be asked and answered with no content: {seen:?}"
    );
    assert!(
        seen.contains(&Seen::Get("OnClose".to_string(), None)),
        "OnClose must be asked and answered with no content: {seen:?}"
    );
    assert!(
        seen.iter().all(|s| !matches!(s, Seen::Get(_, Some(_)))),
        "every request must return no content: {seen:?}"
    );
    assert_eq!(
        seen.last(),
        Some(&Seen::Unload(Some(ExitKind::Clean))),
        "OnClose 204 must lead to a clean unload: {seen:?}"
    );

    let (done_tx, done_rx) = channel();
    std::thread::spawn(move || {
        let _ = done_tx.send(runtime.shutdown(CloseReason::System).is_ok());
    });
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(10)),
        Ok(true),
        "shutdown should complete within 10s"
    );
}
