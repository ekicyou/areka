//! LogSink 側の起動でも停止通知が終了の指示まで届くことの確認（areka-P0-shiori-fault-notice
//! タスク 3.3・要件 6.4・7.6・8.5）。
//!
//! 実 sink 結線（`Emo2Wiring`）の無い World に受け口 `KanadeStopRx` だけを挿し、接続に失敗する
//! SHIORI（`ShioriWiring::Custom` が `Err`）を停止通知の送出端つきで起動する。kanade が自分で
//! 止まって送る停止通知を、終了相（`run_ghost_quit_phase`）を有界に回して受け取り、終了が
//! 指示され、最初の出所が「接続できなかった」の Fault になることを見る。GPU も実窓も要らない。

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use areka_ghost::sink::{DiscardSink, LogSink};
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode};
use areka_kanade::{CloseReason, KanadeStopCause, KanadeStopped, ShioriFaultKind};
use areka_parsers::charset::DefaultEncoding;
use temp_path_kit::TempPath;

use super::*;
use crate::app_exit::{ExitOrigin, FirstExit, fault_of};

/// 停止通知を待つ上限（有界・宙吊りを防ぐ）。
const BOUND: Duration = Duration::from_secs(30);

/// 注入する接続失敗の理由。
const CONNECT_ERR: &str = "logsink-side simulated connect failure";

/// 最小の解決可能なゴーストの木（`ghost/master/descript.txt`＋`shell/master/descript.txt`）。
/// areka-ghost の接続失敗の e2e（`spine_e2e_test_s2_connect_failure.rs`）と同じ組み方。
fn write_ghost_fixture(root: &Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,LogSinkQuitGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");
    let shell_master = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_master).expect("create shell/master");
    std::fs::write(
        shell_master.join("descript.txt"),
        b"charset,UTF-8\nname,LogSinkQuitShell\n",
    )
    .expect("write shell descript.txt");
}

/// 実 sink 結線なしの World＋受け口で、接続失敗の停止通知が `quit_app` まで届く（要件 6.4・7.6）。
///
/// # 空振りしないこと
/// 受け口を挿さなければ終了相は何もせず、期限切れで落ちる。停止通知の送出端を渡さなければ
/// 通知が来ず、同じく落ちる。
#[test]
fn connect_failure_reaches_quit_app_without_real_sink_wiring() {
    let temp = TempPath::new("areka-ghost-quit-logsink");
    write_ghost_fixture(temp.path());

    // 実 sink 結線（`Emo2Wiring`）は挿さない。終了の受け口と停止通知の受け口だけ。
    let (stop_tx, stop_rx) = mpsc::channel::<KanadeStopped>();
    let mut world = World::new();
    world.insert_non_send(wintf::AppExit::new());
    world.insert_non_send(KanadeStopRx(stop_rx));

    // LogSink 側の起動と同じ sink の組（`boot_config::ghost_boot_options`）で、SHIORI だけ接続失敗にする。
    let options = GhostBootOptions {
        ghost_root: temp.path().to_path_buf(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| Err(CONNECT_ERR.to_string()))),
        sinks: vec![Box::new(LogSink::new()), Box::new(DiscardSink::new())],
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };
    let ghost = areka_ghost::boot_with_kanade_stop(options, Some(stop_tx))
        .expect("接続の失敗はアクターの中で起こるので起動そのものは成功する");

    // 終了相を有界に回す（本番は毎フレーム 1 回）。
    let deadline = Instant::now() + BOUND;
    let mut consumed = false;
    while !consumed && Instant::now() < deadline {
        consumed = run_ghost_quit_phase(&mut world);
        if !consumed {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    // kanade は停止済みなので後始末は冪等に終わる（判定より先に畳んでスレッドを残さない）。
    let shutdown = ghost.shutdown(CloseReason::User { scope: 0 });

    assert!(consumed, "停止通知が {BOUND:?} 以内に終了相へ届く");
    assert!(
        world
            .get_non_send::<wintf::AppExit>()
            .expect("挿してある")
            .is_requested(),
        "終了が指示される"
    );
    let origin = &world
        .get_resource::<FirstExit>()
        .expect("最初の出所が残る")
        .0;
    assert!(
        matches!(origin, ExitOrigin::KanadeStopped(KanadeStopCause::Fault(_))),
        "最初の出所は kanade の停止（Fault）: {origin:?}"
    );
    // 理由の文言は着順で揺れる（`shiori handshake failure: ` の前置きの有無）ので種類で判定し、理由は含有で見る。
    let fault = fault_of(origin).expect("Fault なら中身が取れる");
    assert_eq!(fault.kind, ShioriFaultKind::ConnectFailed, "{origin:?}");
    assert!(fault.reason.contains(CONNECT_ERR), "{origin:?}");
    assert!(shutdown.is_ok(), "後始末が失敗しない: {shutdown:?}");
}
