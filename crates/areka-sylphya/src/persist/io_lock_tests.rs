//! 更新の間の錠（#3）の実のファイルのテスト。根は共有の窓口から取る（ワークツリーの `target` の下）。
//! `io.rs` の一時のパスの入口の行を増やさないため兄弟のファイルに置く（`io` の子＝非公開の口も見える）。

use super::*;
use std::time::Duration;
use temp_path_kit::TempPath;

/// 1 本目を持つ間は 2 本目が期限切れになり、捨てた後は取れて、錠のファイルが消える。
/// 同じプロセスの 2 つの持ち手でも、共有の決まりは持ち手ごとなので、ほかのプロセスとぶつかる場面と同じ。
#[test]
fn the_update_lock_shuts_out_a_second_holder_until_dropped() {
    let root = TempPath::new("sylphya-update-lock");
    let path = root.child("sylphya.toml");
    let lock_file = root.child("sylphya.toml.lock");

    let first = FsPersistIo::lock_within(&path, Duration::ZERO).expect("1 本目は取れる");
    assert!(lock_file.exists(), "持つ間は錠のファイルが在る");

    let second = FsPersistIo::lock_within(&path, Duration::from_millis(50));
    let err = second.expect_err("1 本目を持つ間、2 本目は取れない");
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut, "{err}");

    drop(first);
    let third = FsPersistIo::lock_within(&path, Duration::ZERO).expect("捨てた後は取れる");
    drop(third);
    assert!(!lock_file.exists(), "捨てた後は錠のファイルが消える");
}

/// 控える道（読み取りだけの走り）は錠を持たず、錠のファイルもフォルダも作らない。
#[test]
fn a_withheld_path_takes_no_lock_file() {
    let root = TempPath::new("sylphya-update-lock-withheld");
    let dir = root.child("profile");
    let path = dir.join("sylphya.toml");
    let _guard = super::super::withhold::WithholdGuard::under(root.path());

    assert!(FsPersistIo.lock_for_update(&path).unwrap().is_none());
    assert!(!dir.exists(), "控える道ではフォルダを作らない");
    assert!(!dir.join("sylphya.toml.lock").exists());
}

/// 一時の掴み（共有違反）は取り直して通り、ほかの失敗は待たずに返す。
#[test]
fn retry_transient_retries_a_sharing_violation_only() {
    let mut calls = 0;
    let got = retry_transient(|| {
        calls += 1;
        if calls == 1 {
            Err(std::io::Error::from_raw_os_error(32))
        } else {
            Ok(calls)
        }
    });
    assert_eq!(got.unwrap(), 2);

    let mut calls = 0;
    let got: std::io::Result<()> = retry_transient(|| {
        calls += 1;
        Err(std::io::Error::other("not transient"))
    });
    assert!(got.is_err());
    assert_eq!(calls, 1, "一時でない失敗は 1 回で返す");
}
