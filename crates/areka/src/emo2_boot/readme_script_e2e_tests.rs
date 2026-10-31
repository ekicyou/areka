//! 台本の `\![open,readme]` を入れてから偽の OS に開く要求が届くまでを本物の部品で通す
//! テスト（popup-menu-residue 要件 3.1〜3.3・design「説明書の通しのテスト」）。
//!
//! 親（`runtime_boot_tests.rs`）と同じ組み方で emo2 検体を偽の SHIORI で起こし、`Leave` まで
//! 待ってから文脈（ゴーストの枠と開く口）を据え、本番の取り出しを 1 回呼び、送信端を落として
//! 開く処理（`serve`）をテストのスレッドで回す。偽物は SHIORI と OS の 2 つの端だけである。

use std::ffi::OsString;
use std::path::PathBuf;

use super::*;
use crate::emo2_boot::ghost_switch_test_support::standard_script;
use crate::emo2_boot::spine::wait_until;
use crate::emo2_boot::user_break_cue::NoUserBreakSignal;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::readme::os_port::{OsCall, Verb};
use crate::readme::{FakeOs, Opener, drain_readme_requests, serve};
use areka_ghost::dispatcher::DispatcherMsg;
use areka_kanade::MonotonicMs;
use log_capture_kit::{LineFormat, capture_lines};

/// 注入する Tick 1 回で台詞の時計を進める幅（合成の ms）。
const TICK_STEP_MS: u64 = 100;

/// 説明書を開いてから区間に入って出る起動の台本（`Leave` が読み終わりの目印）。
const ON_BOOT: &str = r"\0\![open,readme]\![enter,nouserbreakmode]\![leave,nouserbreakmode]\e";

/// 1 回の通しの結果。
struct Run {
    /// 偽の OS が受けた呼び出し。
    calls: Vec<OsCall>,
    /// 取り出しの呼び出しの間だけ捕まえた記録の行。
    drain_lines: Vec<String>,
    /// 検体の根の `readme.txt`。
    readme: PathBuf,
}

/// emo2 検体を起こし、台本の要求を本番の取り出しから偽の OS まで流す
/// （`remove_readme` なら起動の前に根の `readme.txt` を消す）。
fn run(remove_readme: bool) -> Run {
    com();
    let rig = rig();
    let sample = acquire_emo2();
    let root = sample.folder().to_path_buf();
    let readme = root.join("readme.txt");
    assert!(root.is_absolute(), "検体の根は絶対パス: {}", root.display());
    if remove_readme {
        std::fs::remove_file(&readme).expect("複製の readme.txt を消せる");
    }
    let (backend, handle) = standard_script(ON_BOOT).build();
    let mut world = World::new();
    let inputs = Emo2BootInputs {
        ghost_root: root.clone(),
        balloon_root: sample
            .balloon("emo2-kakukaku")
            .expect("emo2 の同梱バルーン")
            .to_path_buf(),
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        ticker: TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    };
    let runtime = boot_runtime(&mut world, inputs, &rig.sources, mpsc::channel().0, None)
        .expect("偽の SHIORI と emo2 検体で実行系が起きる");

    // 配送は 1 つの cue を全部の受け口へ配ってから次へ進むので、`Leave` の時点で要求は受信端に在る。
    let mut clock_ms = 0;
    let mut left = false;
    let reached = wait_until("OnBoot の Leave", Progress::Unknown, || {
        clock_ms += TICK_STEP_MS;
        let _ = runtime.dispatcher().send(DispatcherMsg::Tick {
            now: MonotonicMs(clock_ms),
        });
        let wiring = world
            .get_non_send::<UserBreakWiring>()
            .expect("成功で中断の持ち物が据わる");
        left |= wiring
            .flag_rx()
            .try_iter()
            .any(|s| s == NoUserBreakSignal::Leave);
        left
    });
    assert!(reached.is_ok(), "Leave まで届かない: {reached:?}");

    // 本番では `ghost_session` と `register_readme_drain` が据える文脈（段ではない）。
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(None, root.clone()))));
    let (tx, rx) = mpsc::channel();
    world.insert_resource(Opener::from_sender(tx));
    let ((), drain_lines) = capture_lines(LineFormat::LevelFields, || {
        drain_readme_requests(&mut world)
    });
    world.remove_resource::<Opener>();
    let mut fake = FakeOs::default();
    serve(rx, &mut fake);

    let calls = move || handle.call_count();
    run_bounded_watching(
        "readme_script ghost shutdown",
        Progress::Count(&calls),
        move || {
            let _ = runtime.shutdown(CloseReason::User { scope: 0 });
        },
    );
    drop(sample);
    Run {
        calls: fake.calls,
        drain_lines,
        readme,
    }
}

/// 台本の説明書の要求が偽の OS へちょうど 1 件、根の `readme.txt` を開く形で届く（3.1・3.3）。
#[test]
fn script_open_readme_reaches_the_fake_os_exactly_once() {
    let run = run(false);
    assert_eq!(run.calls.len(), 1, "開く要求: {:?}", run.calls);
    assert_eq!(run.calls[0].file, OsString::from(run.readme.as_os_str()));
    assert_eq!(run.calls[0].verb, Verb::Open);
}

/// ファイルが無ければ取り出しが `[readme]` の WARN を 1 行だけ残し、偽の OS へは何も届かない（3.2・3.3）。
#[test]
fn script_open_readme_without_the_file_warns_once_and_opens_nothing() {
    let run = run(true);
    let warns: Vec<_> = run
        .drain_lines
        .iter()
        .filter(|l| l.starts_with("level=WARN") && l.contains("[readme]"))
        .collect();
    assert_eq!(warns.len(), 1, "取り出しの行: {:?}", run.drain_lines);
    assert!(
        warns[0].contains("readme_open_skipped_missing"),
        "{warns:?}"
    );
    assert!(run.calls.is_empty(), "開く要求: {:?}", run.calls);
}
