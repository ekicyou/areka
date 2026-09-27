//! `HelperTerminator` の兄弟テスト（要件 2.4・3.5）。
//!
//! 長命の子を `Child` を持たない別スレッドから終わらせ、締切内に子の終了が観測でき、
//! 終わった後にもう一度呼んでも成功する（冪等）ことを実プロセスで確かめる。

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::HelperTerminator;
use crate::lifecycle::{HelperLifecycle, HelperStatus};
use crate::process_host::{ExitKind, HelperHandle, poll_exit_kind, spawn_command};

/// 約 60 秒生きる子（`ping` を直接起こす。`cmd /c` を挟むと孫が残るため挟まない）。
/// 終わらせなければ締切（10 秒）を大きく超えて生き続ける。
fn long_lived() -> Command {
    let mut command = Command::new("ping.exe");
    command
        .args(["-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// 締切（10 秒）内に子の終了を非ブロッキングで待ち、終了の種別を返す。
fn wait_exit(handle: &mut HelperHandle) -> ExitKind {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(kind) = poll_exit_kind(handle) {
            return kind;
        }
        assert!(
            Instant::now() < deadline,
            "terminate の後も子が締切内に終わらなかった"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 型の約束: 複製でき、スレッドをまたいで渡せ、共有できる。
#[test]
fn terminator_is_clone_send_sync() {
    fn assert_bounds<T: Clone + Send + Sync + 'static>() {}
    assert_bounds::<HelperTerminator>();
}

/// `HelperHandle::terminator` の複製を別スレッドから `terminate` すると子が締切内に終わり
/// （終了コード 1＝`Abnormal(1)`）、終わった後に呼んでも（複製元・複製先どちらからでも）成功する。
#[test]
fn terminate_from_another_thread_ends_child_and_is_idempotent() {
    let mut handle = spawn_command(long_lived()).expect("長命の子を spawn できる");
    let terminator = handle.terminator().expect("取っ手を複製できる");
    // 対照: 終わらせる前は生きている。
    assert_eq!(
        poll_exit_kind(&mut handle),
        None,
        "終わらせる前は生きている"
    );

    let remote = terminator.clone();
    std::thread::spawn(move || remote.terminate())
        .join()
        .expect("別スレッドが panic しない")
        .expect("1 度目の terminate は成功");

    assert_eq!(wait_exit(&mut handle), ExitKind::Abnormal(1));
    terminator
        .terminate()
        .expect("終わった後の 2 度目の terminate も成功（冪等）");
    terminator
        .clone()
        .terminate()
        .expect("複製からの 3 度目も成功（冪等）");
}

/// 1 度目の直後、終了を待たずに 2 度目を呼んでも成功する（終わりかけのプロセスでも冪等）。
/// 終わりかけの間だけ開く隙間を踏むため、別々の子で数回くり返す。
#[test]
fn terminate_twice_back_to_back_is_ok() {
    for round in 0..10 {
        let mut handle = spawn_command(long_lived()).expect("長命の子を spawn できる");
        let terminator = handle.terminator().expect("取っ手を複製できる");
        terminator
            .terminate()
            .unwrap_or_else(|e| panic!("{round} 回目の組の 1 度目が失敗: {e:?}"));
        terminator
            .terminate()
            .unwrap_or_else(|e| panic!("{round} 回目の組の直後の 2 度目が失敗: {e:?}"));
        assert_eq!(wait_exit(&mut handle), ExitKind::Abnormal(1));
    }
}

/// `HelperLifecycle::terminator` は保持する `HelperHandle` へ委譲し、同じ子を終わらせる。
#[test]
fn lifecycle_terminator_delegates_to_handle() {
    let handle = spawn_command(long_lived()).expect("長命の子を spawn できる");
    let mut lc = HelperLifecycle::new(handle);
    assert_eq!(
        lc.status(),
        HelperStatus::Running,
        "終わらせる前は生きている"
    );

    let terminator = lc.terminator().expect("取っ手を複製できる");
    terminator.terminate().expect("1 度目の terminate は成功");

    let deadline = Instant::now() + Duration::from_secs(10);
    let kind = loop {
        if let HelperStatus::Exited(kind) = lc.status() {
            break kind;
        }
        assert!(
            Instant::now() < deadline,
            "terminate の後も子が締切内に終わらなかった"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(kind, ExitKind::Abnormal(1));
    terminator
        .terminate()
        .expect("終わった後の 2 度目の terminate も成功（冪等）");
}
