use super::*;
use crate::keytable::Home;
use crate::persist::{PersistFailure, PersistFailureCause};
use crate::store::schema::APPLICATION_ID;
use crate::store::{Access, FsBackend, Located, MemBackend, StoreBackend};
use crate::test_log_capture::capture;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;
use tracing::Level;

pub(super) const LAYERS: [PersistScope; 4] = [
    PersistScope::Balloon,
    PersistScope::Shell,
    PersistScope::Ghost,
    PersistScope::App,
];

pub(super) fn asker() -> AskerId {
    AskerId::new("ghost-a")
}

pub(super) fn fake_roots(tag: &str) -> ScopeRoots {
    let root = |layer: &str| Some(PathBuf::from(format!(r"C:\fake\{tag}\{layer}")));
    ScopeRoots {
        app: root("app"),
        ghost: root("ghost"),
        shell: root("shell"),
        balloon: root("balloon"),
    }
}

pub(super) fn put(roots: &ScopeRoots, layer: PersistScope, pairs: &[(&str, &str)]) -> FileWrite {
    FileWrite {
        layer,
        root: roots.root_of(layer).cloned(),
        puts: pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        removes: Vec::new(),
        typed: Vec::new(),
    }
}

pub(super) fn remove(roots: &ScopeRoots, layer: PersistScope, keys: &[&str]) -> FileWrite {
    FileWrite {
        removes: keys.iter().map(|k| (*k).to_owned()).collect(),
        ..put(roots, layer, &[])
    }
}

pub(super) fn attached_layers(store: &Store) -> Vec<String> {
    let mut stmt = store
        .conn
        .prepare(
            "SELECT name FROM pragma_database_list WHERE name NOT IN ('main', 'temp', 'appvol')",
        )
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

pub(super) fn saved(attempt: WriteAttempt) {
    assert!(
        matches!(attempt, WriteAttempt::Done(PersistOutcome::Saved)),
        "{attempt:?}"
    );
}

fn degraded(attempt: WriteAttempt) -> PersistFailure {
    match attempt {
        WriteAttempt::Done(PersistOutcome::Degraded(failure)) => failure,
        other => panic!("書けなかったはず: {other:?}"),
    }
}

fn header(path: &Path) -> (i32, i32) {
    let conn = Connection::open(path).unwrap();
    let id = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap();
    let version = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    (id, version)
}

#[test]
fn first_write_creates_the_folder_tables_and_header() {
    let dir = TempPath::under_target("sylphya-store-write-first");
    let ghost = dir.child("ghost").join("profile").join("areka");
    let roots = ScopeRoots {
        ghost: Some(ghost.clone()),
        ..ScopeRoots::default()
    };
    let mut store = Store::open(roots.clone(), FsBackend::with_manual_clock()).unwrap();
    saved(store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "v")])));
    assert_eq!(
        header(&ghost.join("sylphya.sqlite")),
        (APPLICATION_ID, schema::current_version(schema::STEPS))
    );
    assert_eq!(
        store.read(&asker(), "k", Some(PersistScope::Ghost), Home::File),
        Some("v".to_owned())
    );
    assert!(attached_layers(&store).is_empty());
    let moved = dir.child("ghost-moved");
    std::fs::rename(dir.child("ghost"), &moved).expect("書きの後は層のフォルダを掴んでいない");
}

#[test]
fn zero_byte_file_gets_tables_and_header_on_first_write() {
    let dir = TempPath::under_target("sylphya-store-write-zero");
    let ghost = dir.child("ghost");
    std::fs::create_dir_all(&ghost).unwrap();
    std::fs::write(ghost.join("sylphya.sqlite"), b"").unwrap();
    let roots = ScopeRoots {
        ghost: Some(ghost.clone()),
        ..ScopeRoots::default()
    };
    let mut store = Store::open(roots.clone(), FsBackend::with_manual_clock()).unwrap();
    saved(store.try_write(&remove(&roots, PersistScope::Ghost, &["k"])));
    saved(store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "")])));
    assert_eq!(header(&ghost.join("sylphya.sqlite")).0, APPLICATION_ID);
    assert_eq!(
        store.read(&asker(), "k", Some(PersistScope::Ghost), Home::File),
        Some(String::new())
    );
}

#[test]
fn delete_only_write_on_a_layer_without_file_creates_nothing() {
    let dir = TempPath::under_target("sylphya-store-write-delete-only");
    let ghost = dir.child("ghost");
    let roots = ScopeRoots {
        ghost: Some(ghost.clone()),
        ..ScopeRoots::default()
    };
    let mut store = Store::open(roots.clone(), FsBackend::with_manual_clock()).unwrap();
    saved(store.try_write(&remove(&roots, PersistScope::Ghost, &["k"])));
    assert!(!ghost.exists(), "消しだけではフォルダも作らない");

    let mem = MemBackend::new();
    let roots = fake_roots("delete-only");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    saved(store.try_write(&remove(&roots, PersistScope::Ghost, &["k"])));
    assert_eq!(
        mem.locate(roots.ghost.as_deref().unwrap(), Access::ReadWrite)
            .unwrap(),
        Located::Missing
    );
}

#[test]
fn layer_without_root_is_degraded_no_root() {
    let mem = MemBackend::new();
    let roots = ScopeRoots {
        shell: None,
        ..fake_roots("no-root")
    };
    let mut store = Store::open(roots.clone(), mem).unwrap();
    let mut write = put(&roots, PersistScope::Shell, &[("areka.boot.count", "1")]);
    write.typed = vec![PersistKey::BootCount];
    let mut attempt = None;
    let log = capture(|| attempt = Some(store.try_write(&write)));
    assert_eq!(
        degraded(attempt.unwrap()),
        PersistFailure {
            scope: PersistScope::Shell,
            dir: None,
            keys: vec![PersistKey::BootCount],
            cause: PersistFailureCause::NoRoot,
        }
    );
    assert_eq!(log.iter().filter(|e| e.level == Level::WARN).count(), 1);
}

#[test]
fn unusable_file_is_not_overwritten_and_reports_unusable() {
    let mem = MemBackend::new();
    let roots = fake_roots("unusable");
    let ghost = roots.ghost.clone().unwrap();
    mem.mark_unusable(&ghost);
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    let mut write = put(&roots, PersistScope::Ghost, &[("areka.boot.count", "1")]);
    write.typed = vec![PersistKey::BootCount];
    let mut attempt = None;
    let log = capture(|| attempt = Some(store.try_write(&write)));
    let failure = degraded(attempt.unwrap());
    assert_eq!(failure.dir, Some(ghost.clone()));
    assert_eq!(failure.keys, vec![PersistKey::BootCount]);
    assert!(
        matches!(failure.cause, PersistFailureCause::Unusable(_)),
        "{failure:?}"
    );
    let errors: Vec<_> = log
        .iter()
        .filter(|e| {
            e.level == Level::ERROR && e.field_str("event") == Some("persist_existing_unusable")
        })
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(attached_layers(&store).is_empty());
    let Located::Uri(uri) = mem.locate(&ghost, Access::ReadWrite).unwrap() else {
        panic!("偽の層は在るはず");
    };
    let conn = Connection::open(uri).unwrap();
    let tables: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'kv'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0, "使えないファイルへは表を作らない");
}

#[test]
fn refused_write_reports_io_and_writes_nothing() {
    let mem = MemBackend::new();
    let roots = fake_roots("os-refused");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    mem.fail_next_write();
    let mut attempt = None;
    let log = capture(|| {
        attempt = Some(store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "v")])))
    });
    let failure = degraded(attempt.unwrap());
    assert!(
        matches!(failure.cause, PersistFailureCause::Io(_)),
        "{failure:?}"
    );
    assert_eq!(log.iter().filter(|e| e.level == Level::ERROR).count(), 1);
    assert!(attached_layers(&store).is_empty());
    assert_eq!(
        store.read(&asker(), "k", Some(PersistScope::Ghost), Home::File),
        None
    );
}

#[test]
fn lock_held_elsewhere_gives_busy_and_leaves_the_file_unchanged() {
    let mem = MemBackend::new();
    let roots = fake_roots("busy");
    let ghost = roots.ghost.clone().unwrap();
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    saved(store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "old")])));
    let hold = mem.hold_writer(&ghost);
    let attempt = store.try_write(&put(
        &roots,
        PersistScope::Ghost,
        &[("k", "new"), ("j", "x")],
    ));
    assert!(matches!(attempt, WriteAttempt::Busy), "{attempt:?}");
    assert!(attached_layers(&store).is_empty());
    drop(hold);
    assert_eq!(
        store.read_layer(PersistScope::Ghost),
        vec![("k".to_owned(), "old".to_owned())]
    );
}

#[test]
fn delete_removes_only_the_named_keys() {
    let mem = MemBackend::new();
    let roots = fake_roots("delete");
    let mut store = Store::open(roots.clone(), mem).unwrap();
    saved(store.try_write(&put(&roots, PersistScope::Ghost, &[("a", "1"), ("b", "2")])));
    saved(store.try_write(&remove(&roots, PersistScope::Ghost, &["a", "missing"])));
    assert_eq!(
        store.read_layer(PersistScope::Ghost),
        vec![("b".to_owned(), "2".to_owned())]
    );
}

#[test]
fn volatile_values_go_to_the_layer_and_can_be_removed() {
    let mem = MemBackend::new();
    let roots = fake_roots("volatile");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    let other = AskerId::new("ghost-b");
    for layer in LAYERS {
        store
            .write_volatile(layer, &asker(), "k", Some(&format!("{layer:?}")))
            .unwrap();
    }
    for layer in LAYERS {
        assert_eq!(
            store.read(&asker(), "k", Some(layer), Home::Memory),
            Some(format!("{layer:?}"))
        );
    }
    assert_eq!(
        store.read(&other, "k", None, Home::Memory),
        Some("App".to_owned()),
        "アプリの層だけがほかの問い手から見える"
    );
    store
        .write_volatile(PersistScope::Balloon, &asker(), "k", None)
        .unwrap();
    assert_eq!(
        store.read(&asker(), "k", None, Home::Memory),
        Some("Shell".to_owned())
    );
    for layer in LAYERS {
        assert_eq!(
            mem.locate(roots.root_of(layer).unwrap(), Access::ReadWrite)
                .unwrap(),
            Located::Missing,
            "揮発の値はファイルを作らない"
        );
    }
}

#[test]
fn flat_names_are_per_asker_and_removable() {
    let mut store = Store::open(fake_roots("flat"), MemBackend::new()).unwrap();
    let other = AskerId::new("ghost-b");
    assert!(store.write_flat(&asker(), "username", Some("a")).unwrap());
    assert!(store.write_flat(&asker(), "empty", Some("")).unwrap());
    assert!(store.write_flat(&other, "username", Some("b")).unwrap());
    assert_eq!(store.read_flat(&asker(), "username"), Some("a".to_owned()));
    assert_eq!(store.read_flat(&asker(), "empty"), Some(String::new()));
    assert_eq!(
        store.flat_snapshot(&asker()),
        BTreeMap::from([
            ("empty".to_owned(), String::new()),
            ("username".to_owned(), "a".to_owned()),
        ])
    );
    assert!(store.write_flat(&asker(), "username", None).unwrap());
    assert!(
        !store.write_flat(&asker(), "username", None).unwrap(),
        "無い名前の消しは何も変えない"
    );
    assert_eq!(store.read_flat(&asker(), "username"), None);
    assert_eq!(store.read_flat(&other, "username"), Some("b".to_owned()));
}

#[test]
fn set_root_discards_that_layers_volatile_values_and_reads_the_new_place() {
    let mem = MemBackend::new();
    let roots = fake_roots("re-root");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    for layer in LAYERS {
        store
            .write_volatile(layer, &asker(), "k", Some(&format!("{layer:?}")))
            .unwrap();
    }
    saved(store.try_write(&put(&roots, PersistScope::Shell, &[("f", "old-shell")])));

    let next_shell = PathBuf::from(r"C:\fake\re-root\shell-2");
    store.set_root(PersistScope::Shell, Some(next_shell.clone()));
    assert_eq!(
        store.read(&asker(), "k", Some(PersistScope::Shell), Home::Memory),
        None
    );
    for layer in [
        PersistScope::Balloon,
        PersistScope::Ghost,
        PersistScope::App,
    ] {
        assert_eq!(
            store.read(&asker(), "k", Some(layer), Home::Memory),
            Some(format!("{layer:?}")),
            "ほかの層の揮発の値は残る"
        );
    }
    assert_eq!(
        store.read(&asker(), "f", Some(PersistScope::Shell), Home::File),
        None
    );
    saved(store.try_write(&FileWrite {
        root: Some(next_shell),
        ..put(&roots, PersistScope::Shell, &[("f", "new-shell")])
    }));
    assert_eq!(
        store.read(&asker(), "f", Some(PersistScope::Shell), Home::File),
        Some("new-shell".to_owned())
    );

    store.set_root(PersistScope::Shell, roots.shell.clone());
    assert_eq!(
        store.read(&asker(), "f", Some(PersistScope::Shell), Home::File),
        Some("old-shell".to_owned()),
        "覚える値はファイルに残り、戻すとまた読める"
    );
}

#[test]
fn lock_met_at_commit_rolls_back_to_busy() {
    let dir = TempPath::under_target("sylphya-store-write-commit-busy");
    let ghost = dir.child("ghost");
    let roots = ScopeRoots {
        ghost: Some(ghost.clone()),
        ..ScopeRoots::default()
    };
    let mut store = Store::open(roots.clone(), FsBackend::with_manual_clock()).unwrap();
    saved(store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "old")])));
    let reader = Connection::open(ghost.join("sylphya.sqlite")).unwrap();
    reader
        .execute_batch("BEGIN; SELECT count(*) FROM kv;")
        .unwrap();
    let attempt = store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "new")]));
    assert!(matches!(attempt, WriteAttempt::Busy), "{attempt:?}");
    assert!(attached_layers(&store).is_empty());
    reader.execute_batch("COMMIT").unwrap();
    assert_eq!(
        store.read_layer(PersistScope::Ghost),
        vec![("k".to_owned(), "old".to_owned())]
    );
    saved(store.try_write(&put(&roots, PersistScope::Ghost, &[("k", "new")])));
}
