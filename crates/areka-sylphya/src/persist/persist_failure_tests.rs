//! 書けなかった 1 回の中身（[`PersistFailure`]）を保存の結果が運ぶことの檻（要件 1.4・6.3）。
//!
//! - 書けない置き場（置き場の位置に普通のファイルを置く）→ 置き場・key の並び・OS の理由。
//! - 置き場の無いスコープ → `NoRoot`・`dir: None`。
//! - `persist commit failed` の記録に欄 `keys` が載る（名前は変えない）。

use super::*;
use temp_path_kit::TempPath;

/// 置き場（root）の位置に普通のファイルを置き、`<root>/sylphya.toml` を書けなくする。
fn unwritable_root(label: &str) -> (TempPath, PathBuf) {
    let tmp = TempPath::under_target(label);
    let root = tmp.child("areka");
    std::fs::write(&root, "not a directory").expect("置き場の位置に普通のファイルを置く");
    (tmp, root)
}

fn entries() -> Vec<(PersistKey, String)> {
    vec![
        (PersistKey::LastGhost, "emo2".into()),
        (PersistKey::BootCount, "3".into()),
    ]
}

#[test]
fn unwritable_dir_returns_dir_keys_and_os_reason() {
    let (_tmp, root) = unwritable_root("sylphya-persist-failure-io");
    let roots = ScopeRoots {
        ghost: Some(root.clone()),
        ..ScopeRoots::default()
    };
    // OS の理由は同じ置き場へ同じ操作をしたときの表示と一致する。
    let expected_reason = std::fs::create_dir_all(&root)
        .expect_err("前提: 普通のファイルの位置にはディレクトリを作れない")
        .to_string();

    let outcome = save_scope(PersistScope::Ghost, &roots, &FsPersistIo, entries());

    let PersistOutcome::Degraded(failure) = outcome else {
        panic!("書けない置き場は Degraded になる: {outcome:?}");
    };
    assert_eq!(
        failure,
        PersistFailure {
            scope: PersistScope::Ghost,
            dir: Some(root),
            keys: vec![PersistKey::LastGhost, PersistKey::BootCount],
            cause: PersistFailureCause::Io(expected_reason),
        }
    );
}

#[test]
fn scope_without_root_returns_no_root() {
    let outcome = save_scope(
        PersistScope::Shell,
        &ScopeRoots::default(),
        &FakePersistIo::new(),
        entries(),
    );
    assert_eq!(
        outcome,
        PersistOutcome::Degraded(PersistFailure {
            scope: PersistScope::Shell,
            dir: None,
            keys: vec![PersistKey::LastGhost, PersistKey::BootCount],
            cause: PersistFailureCause::NoRoot,
        })
    );
}

#[test]
fn commit_failed_log_carries_keys_field() {
    let (_tmp, root) = unwritable_root("sylphya-persist-failure-log");
    let roots = ScopeRoots {
        app: Some(root),
        ..ScopeRoots::default()
    };
    let (_, events) = log_capture_kit::capture(|| {
        save_scope(PersistScope::App, &roots, &FsPersistIo, entries());
    });
    let ev = events
        .iter()
        .find(|e| {
            e.target == LOG_TARGET
                && e.level == tracing::Level::ERROR
                && e.message().contains("persist commit failed")
        })
        .unwrap_or_else(|| panic!("persist commit failed が出ていない: {events:?}"));
    let keys = ev
        .field("keys")
        .unwrap_or_else(|| panic!("欄 keys が無い: {ev:?}"));
    assert!(
        keys.contains("areka.last.ghost") && keys.contains("areka.boot.count"),
        "欄 keys に覚えられなかった key が並ぶ: {keys}"
    );
}
