//! 使えない記憶のファイルを上書きしない檻（#1・要件 4.1・4.2・4.5）。
//!
//! - 新しい版・壊れた TOML・読みの失敗 → 保存は `Degraded(Io(理由))`・中身はバイトで同じ・
//!   ERROR `persist_existing_unusable` が 1 件。
//! - 1 回目だけ共有違反（os error 32）→ 読み直して `Saved`・既存の key も残る。
//! - 空のファイルは上書きし、無いファイルは今どおり作る。

use super::*;
use crate::test_log_capture::capture;
use std::sync::atomic::{AtomicBool, Ordering};

fn ghost_roots() -> ScopeRoots {
    ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    }
}

fn ghost_file() -> PathBuf {
    ghost_roots().file_of(PersistScope::Ghost).unwrap()
}

fn save_boot_count(io: &dyn PersistIo) -> PersistOutcome {
    save_scope(
        PersistScope::Ghost,
        &ghost_roots(),
        io,
        vec![(PersistKey::BootCount, "7".into())],
    )
}

/// 使えない中身を置いた上の保存が縮退し、中身を変えず、ERROR を 1 件出すことを見る。
fn assert_kept_and_degraded(io: &FakePersistIo, content: &str, reason_needle: &str) {
    let mut outcome = None;
    let events = capture(|| outcome = Some(save_boot_count(io)));
    let outcome = outcome.unwrap();
    let PersistOutcome::Degraded(failure) = &outcome else {
        panic!("使えないファイルの上の保存は Degraded になる: {outcome:?}");
    };
    let PersistFailureCause::Io(reason) = &failure.cause else {
        panic!("理由は Io で運ぶ: {failure:?}");
    };
    assert!(
        reason.contains(reason_needle),
        "理由に {reason_needle:?}: {reason}"
    );
    assert_eq!(
        io.read(&ghost_file()).unwrap().as_deref(),
        Some(content),
        "中身はバイトで同じ"
    );
    let unusable = events
        .iter()
        .filter(|e| {
            e.target == LOG_TARGET
                && e.level == tracing::Level::ERROR
                && e.message().contains("persist existing file unusable")
        })
        .count();
    assert_eq!(
        unusable, 1,
        "ERROR persist_existing_unusable が 1 件: {events:?}"
    );
}

#[test]
fn a_newer_format_version_is_kept_and_the_save_degrades() {
    let io = FakePersistIo::new();
    let content = "format-version = 2\n\n[last]\nghost = \"emo2\"\n";
    io.commit(&ghost_file(), content).unwrap();
    assert_kept_and_degraded(&io, content, "format-version");
}

#[test]
fn a_broken_toml_is_kept_and_the_save_degrades() {
    let io = FakePersistIo::new();
    let content = "format-version = 1\n[window";
    io.commit(&ghost_file(), content).unwrap();
    assert_kept_and_degraded(&io, content, "TOML");
}

#[test]
fn a_read_failure_keeps_the_file_and_the_save_degrades() {
    let io = FakePersistIo::new();
    save_scope(
        PersistScope::Ghost,
        &ghost_roots(),
        &io,
        vec![(PersistKey::LastGhost, "emo2".into())],
    );
    let content = io.read(&ghost_file()).unwrap().unwrap();
    io.fail_next_read();
    // 共有違反でない読みの失敗は待たずに縮退する。
    assert_kept_and_degraded(&io, &content, "読めない");
}

/// 1 回目の読みだけ共有違反（os error 32）を返す、共有の偽の IO の包み。
struct SharingViolationOnce {
    inner: FakePersistIo,
    violated: AtomicBool,
}

impl PersistIo for SharingViolationOnce {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        if !self.violated.swap(true, Ordering::SeqCst) {
            return Err(std::io::Error::from_raw_os_error(32));
        }
        self.inner.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.inner.commit(path, content)
    }
}

#[test]
fn a_transient_sharing_violation_is_retried() {
    let inner = FakePersistIo::new();
    save_scope(
        PersistScope::Ghost,
        &ghost_roots(),
        &inner,
        vec![(PersistKey::LastGhost, "emo2".into())],
    );
    let io = SharingViolationOnce {
        inner,
        violated: AtomicBool::new(false),
    };
    assert_eq!(save_boot_count(&io), PersistOutcome::Saved);
    let loaded = load_scope(PersistScope::Ghost, &ghost_roots(), &io);
    assert!(
        loaded.contains(&(PersistKey::LastGhost, "emo2".into())),
        "既存の key が残る: {loaded:?}"
    );
    assert!(
        loaded.contains(&(PersistKey::BootCount, "7".into())),
        "保存した値: {loaded:?}"
    );
}

#[test]
fn a_transient_sharing_violation_on_load_is_retried() {
    let inner = FakePersistIo::new();
    save_scope(
        PersistScope::Ghost,
        &ghost_roots(),
        &inner,
        vec![(PersistKey::LastGhost, "emo2".into())],
    );
    let io = SharingViolationOnce {
        inner,
        violated: AtomicBool::new(false),
    };
    // 起動の読みも一時の掴みで覚えていた値を忘れない。
    assert_eq!(
        load_scope(PersistScope::Ghost, &ghost_roots(), &io),
        vec![(PersistKey::LastGhost, "emo2".into())]
    );
}

#[test]
fn an_empty_file_is_overwritten() {
    let io = FakePersistIo::new();
    io.commit(&ghost_file(), "").unwrap();
    assert_eq!(save_boot_count(&io), PersistOutcome::Saved);
    assert_eq!(
        load_scope(PersistScope::Ghost, &ghost_roots(), &io),
        vec![(PersistKey::BootCount, "7".into())]
    );
}

#[test]
fn a_missing_file_is_still_created() {
    let io = FakePersistIo::new();
    assert_eq!(save_boot_count(&io), PersistOutcome::Saved);
    assert_eq!(
        load_scope(PersistScope::Ghost, &ghost_roots(), &io),
        vec![(PersistKey::BootCount, "7".into())]
    );
}

/// 版が 1 のままの知らない表・知っている表の知らない key・型の違う値は、保存で消えない。
#[test]
fn unknown_tables_and_values_survive_a_save() {
    let io = FakePersistIo::new();
    let content = "format-version = 1\n[foo]\nbar = \"x\"\n[last]\nghost = \"emo2\"\nnote = \"n\"\n[vanish]\ncount = 7\n";
    io.commit(&ghost_file(), content).unwrap();
    assert_eq!(save_boot_count(&io), PersistOutcome::Saved);
    let saved: toml::Table = io.read(&ghost_file()).unwrap().unwrap().parse().unwrap();
    let expected: toml::Table = "format-version = 1\n[foo]\nbar = \"x\"\n[last]\nghost = \"emo2\"\nnote = \"n\"\n[vanish]\ncount = 7\n[boot]\ncount = \"7\"\n"
        .parse()
        .unwrap();
    assert_eq!(saved, expected);
}
