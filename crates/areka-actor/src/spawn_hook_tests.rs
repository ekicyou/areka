//! スレッド開始フック（[`super::install_thread_start_hook`]）の決定論テスト。
//!
//! フックはプロセス共有で **1 度しか導入できない**（最初が勝つ）。導入の検査と、フックが
//! 「生成されたスレッドの中で」走ることの検査を 1 本のテストに畳んでいるのは、テストが
//! 並列に走る中で導入順を取り合わないためである。

use super::*;
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::Duration;

/// フックが観測した（アクター名, 走ったスレッド）の記録。
static OBSERVED: Mutex<Vec<(String, ThreadId)>> = Mutex::new(Vec::new());

/// フックの中の記録の宛先（このファイル専用・ほかの試験の記録と混ざらない）。
const TARGET: &str = "areka_actor::spawn_hook_tests";

/// 検査用フック。名前と「今どのスレッドで走っているか」を記録し、追跡の記録を 1 件出す。
fn probe_hook(name: &str) {
    tracing::trace!(target: TARGET, actor = name, "フックの中の追跡");
    let mut seen = OBSERVED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    seen.push((name.to_owned(), thread::current().id()));
}

/// 2 度目の導入が `Err` になることの対照に使う別のフック（呼ばれない）。
fn never_installed_hook(_name: &str) {}

#[test]
fn hook_runs_on_the_spawned_thread_and_only_the_first_install_wins() {
    install_thread_start_hook(probe_hook).expect("最初の導入は成功するはず");

    let caller = thread::current().id();
    let (tx, handle) = spawn_actor::<(), _>("hook-probe", |_rx| {});
    drop(tx); // 全 Sender drop → body は即座に終わる。
    handle.join().expect("body は正常終了するはず");

    let seen = OBSERVED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let hit = seen
        .iter()
        .find(|(name, _)| name == "hook-probe")
        .expect("フックはアクター名を受け取って呼ばれるはず");
    assert_ne!(
        hit.1, caller,
        "フックは生成された側のスレッドの中で走るはず（呼び出し側では走らない）"
    );

    assert!(
        install_thread_start_hook(never_installed_hook).is_err(),
        "2 度目の導入は Err（最初の導入が勝つ）"
    );

    // 導入済みでもアクターの挙動は変わらない（body が走り、join できる）。
    let (tx2, handle2) = spawn_actor::<(), _>("hook-probe-2", |rx| {
        // 全 Sender drop まで待つ受信ループ相当（切断で正常終了）。
        while rx.recv_timeout(Duration::from_millis(50)).is_ok() {}
    });
    drop(tx2);
    handle2.join().expect("2 本目も正常終了するはず");

    // 窓の中で起こしたアクターでは、フックの中の記録も窓に入る（受け手を据えるのがフックより前）。
    let ((), events) = log_capture_kit::capture(|| {
        let (tx3, handle3) = spawn_actor::<(), _>("hook-probe-window", |_rx| {});
        drop(tx3);
        handle3.join().expect("3 本目も正常終了するはず");
    });
    assert!(
        events
            .iter()
            .any(|e| e.target == TARGET && e.field_str("actor") == Some("hook-probe-window")),
        "窓の中で起こしたアクターのフックの記録が窓に入るはず: {events:?}"
    );
}
