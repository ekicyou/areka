//! 利用者ごとの置き場への写し（`copy_tree_atomic`）が、新しい記憶のファイルの値を欠かさず運ぶことを確かめる試験。
//!
//! 写しの元に記憶のファイルだけが在る場合と、別の接続が書きの途中でジャーナルが在る場合を回す。
//! 写した先で「書けた」の値が全部読め、写しの元は名前も中身も変わらない。

use areka_sylphya::persist::PersistScope;
use areka_sylphya::store::{FsBackend, StoreBackend};
use areka_sylphya::{Axis, PersistKey, PersistOutcome, ScopeRoots, load_scope, save_scope};
use areka_user_data::{CopyDone, copy_tree_atomic};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use temp_path_kit::TempPath;

fn backend() -> Arc<dyn StoreBackend> {
    FsBackend::with_manual_clock()
}

fn app_roots(dir: &Path) -> ScopeRoots {
    ScopeRoots {
        app: Some(dir.to_path_buf()),
        ..ScopeRoots::default()
    }
}

fn written() -> Vec<(PersistKey, String)> {
    let mut entries = vec![
        (PersistKey::BootCount, "3".to_owned()),
        (PersistKey::LastGhost, "emo2".to_owned()),
        (
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            "120".to_owned(),
        ),
    ];
    entries.sort_by_key(|(key, _)| key.to_canonical_key());
    entries
}

/// 写しの元 `exe\profile\areka` に、口を通して値を書く。
fn source_with_memory(tmp: &TempPath) -> PathBuf {
    let from = tmp.child("exe").join("profile").join("areka");
    std::fs::create_dir_all(&from).expect("写しの元を作る");
    let outcome = save_scope(PersistScope::App, &app_roots(&from), &backend(), written());
    assert_eq!(outcome, PersistOutcome::Saved);
    from
}

/// フォルダの下の全部のファイルの、相対の名前と中身。
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("フォルダを読む") {
            let path = entry.expect("項目を読む").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).expect("ファイルを読む");
                out.insert(path.strip_prefix(dir).unwrap().to_path_buf(), bytes);
            }
        }
    }
    out
}

fn copy_to_user_root(tmp: &TempPath, from: &Path) -> PathBuf {
    let user_root = tmp.child("user");
    let to = user_root.join("profile").join("areka");
    let done = copy_tree_atomic(from, &to, &user_root).expect("写せる");
    assert!(matches!(done, CopyDone::Copied(_)), "{done:?}");
    to
}

#[test]
fn copied_memory_file_reads_every_written_value_and_leaves_the_source_alone() {
    let tmp = TempPath::under_target("memcopy-plain");
    let from = source_with_memory(&tmp);
    let names: Vec<_> = snapshot(&from).into_keys().collect();
    assert_eq!(names, [PathBuf::from("sylphya.sqlite")]);
    let before = snapshot(&from);

    let to = copy_to_user_root(&tmp, &from);

    assert_eq!(snapshot(&from), before);
    assert_eq!(
        load_scope(PersistScope::App, &app_roots(&to), &backend()),
        written()
    );
}

#[test]
fn copy_while_another_connection_is_mid_write_reads_only_the_committed_values() {
    let tmp = TempPath::under_target("memcopy-journal");
    let from = source_with_memory(&tmp);
    let writer = rusqlite::Connection::open(from.join("sylphya.sqlite")).unwrap();
    writer
        .execute_batch(
            "BEGIN IMMEDIATE; \
             UPDATE kv SET value = '999' WHERE key = 'areka.boot.count'; \
             INSERT INTO kv (key, value) VALUES ('areka.last.running', 'half');",
        )
        .unwrap();
    assert!(from.join("sylphya.sqlite-journal").is_file());
    let before = snapshot(&from);

    let to = copy_to_user_root(&tmp, &from);

    assert_eq!(snapshot(&from), before);
    assert!(to.join("sylphya.sqlite-journal").is_file());
    assert_eq!(
        load_scope(PersistScope::App, &app_roots(&to), &backend()),
        written()
    );
    let more = vec![(PersistKey::VanishCount, "1".to_owned())];
    let outcome = save_scope(PersistScope::App, &app_roots(&to), &backend(), more.clone());
    assert_eq!(outcome, PersistOutcome::Saved);
    assert!(!to.join("sylphya.sqlite-journal").exists());
    let mut expected = [written(), more].concat();
    expected.sort_by_key(|(key, _)| key.to_canonical_key());
    assert_eq!(
        load_scope(PersistScope::App, &app_roots(&to), &backend()),
        expected
    );
    writer.execute_batch("ROLLBACK").unwrap();
}
