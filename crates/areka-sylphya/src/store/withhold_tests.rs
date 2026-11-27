use crate::asker::AskerId;
use crate::keytable::Home;
use crate::persist::withhold::WithholdGuard;
use crate::persist::{PersistFailureCause, PersistOutcome, PersistScope, ScopeRoots};
use crate::store::write::{FileWrite, WriteAttempt};
use crate::store::{Access, FsBackend, Located, MemBackend, Store, StoreBackend};
use crate::test_log_capture::capture;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use temp_path_kit::TempPath;
use tracing::Level;

const GHOST: PersistScope = PersistScope::Ghost;

fn open_store(backend: Arc<dyn StoreBackend>, root: &Path) -> Store {
    let roots = ScopeRoots {
        ghost: Some(root.to_path_buf()),
        ..ScopeRoots::default()
    };
    Store::open(roots, backend).unwrap()
}

fn read(store: &mut Store, key: &str) -> Option<String> {
    store.read(&AskerId::new("ghost-a"), key, Some(GHOST), Home::File)
}

fn write(root: &Path, puts: &[(&str, &str)], removes: &[&str]) -> FileWrite {
    FileWrite {
        layer: GHOST,
        root: Some(root.to_path_buf()),
        puts: puts
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        removes: removes.iter().map(|k| (*k).to_owned()).collect(),
        typed: Vec::new(),
    }
}

fn outcome(attempt: WriteAttempt) -> PersistOutcome {
    match attempt {
        WriteAttempt::Done(outcome) => outcome,
        WriteAttempt::Busy => panic!("錠に当たらないはず"),
    }
}

/// フォルダの下のパスと中身の全部（フォルダは中身なし）。
fn tree(dir: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                out.push((path.clone(), None));
                out.extend(tree(&path));
            } else {
                out.push((path.clone(), Some(std::fs::read(&path).unwrap())));
            }
        }
    }
    out.sort();
    out
}

/// 範囲の外で、本物のファイルの層に（鍵, 値）を書いておく。
fn real_layer(dir: &TempPath, pairs: &[(&str, &str)]) -> PathBuf {
    let root = dir.child("ghost");
    let backend = FsBackend::with_manual_clock();
    assert_eq!(
        outcome(open_store(backend, &root).try_write(&write(&root, pairs, &[]))),
        PersistOutcome::Saved
    );
    root
}

#[test]
fn write_inside_is_withheld_and_another_core_of_the_run_reads_it() {
    let dir = TempPath::under_target("sylphya-held-another-core");
    let root = real_layer(
        &dir,
        &[("areka.last.ghost", "g"), ("areka.boot.count", "3")],
    );
    let before = tree(dir.path());
    let _withhold = WithholdGuard::under(dir.path());
    let backend: Arc<dyn StoreBackend> = FsBackend::with_manual_clock();

    let mut writer = open_store(backend.clone(), &root);
    let mut result = None;
    let log = capture(|| {
        result = Some(writer.try_write(&write(
            &root,
            &[("areka.last.ghost", "h")],
            &["areka.boot.count"],
        )));
    });
    assert_eq!(outcome(result.unwrap()), PersistOutcome::Withheld);
    assert!(
        log.iter()
            .any(|e| e.level == Level::DEBUG && e.field_str("event") == Some("persist_withheld")),
        "{log:?}"
    );
    assert!(
        !log.iter().any(|e| e.level <= Level::WARN),
        "控えは error にも warn にも出ない: {log:?}"
    );

    let mut other = open_store(backend.clone(), &root);
    assert_eq!(read(&mut other, "areka.last.ghost").as_deref(), Some("h"));
    assert_eq!(read(&mut other, "areka.boot.count"), None);
    assert_eq!(
        other.read_layer(GHOST),
        [("areka.last.ghost".to_owned(), "h".to_owned())]
    );
    assert_eq!(read(&mut writer, "areka.last.ghost").as_deref(), Some("h"));
    drop((writer, other));
    assert_eq!(tree(dir.path()), before, "置き場の一覧と中身は前と同じ");
}

#[test]
fn write_inside_to_a_layer_without_a_file_makes_no_folder() {
    let dir = TempPath::under_target("sylphya-held-no-file");
    let root = dir.child("ghost");
    let _withhold = WithholdGuard::under(dir.path());
    let backend: Arc<dyn StoreBackend> = FsBackend::with_manual_clock();
    let mut store = open_store(backend.clone(), &root);
    assert_eq!(
        outcome(store.try_write(&write(&root, &[("areka.last.ghost", "g")], &[]))),
        PersistOutcome::Withheld
    );
    assert_eq!(
        outcome(store.try_write(&write(&root, &[], &["areka.boot.count"]))),
        PersistOutcome::Withheld
    );
    assert_eq!(
        read(&mut open_store(backend, &root), "areka.last.ghost").as_deref(),
        Some("g")
    );
    assert!(!root.exists(), "フォルダもファイルも作らない");
}

#[test]
fn write_inside_to_an_unusable_layer_is_withheld_and_reads_back() {
    let mem = MemBackend::new();
    let root = PathBuf::from(r"C:\偽の置き場\held-unusable\ghost");
    mem.mark_unusable(&root);
    let _withhold = WithholdGuard::under(&root);
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(
        outcome(store.try_write(&write(&root, &[("areka.last.ghost", "g")], &[]))),
        PersistOutcome::Withheld
    );
    assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("g"));
}

#[test]
fn a_layer_without_a_root_is_no_root_even_inside() {
    let dir = TempPath::under_target("sylphya-held-no-root");
    let _withhold = WithholdGuard::under(dir.path());
    let mut store = open_store(FsBackend::with_manual_clock(), &dir.child("ghost"));
    let mut no_root = write(&dir.child("ghost"), &[("areka.last.ghost", "g")], &[]);
    no_root.root = None;
    match outcome(store.try_write(&no_root)) {
        PersistOutcome::Degraded(f) => assert!(matches!(f.cause, PersistFailureCause::NoRoot)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn dropping_the_range_discards_what_was_held_under_it() {
    let dir = TempPath::under_target("sylphya-held-drop");
    let root = real_layer(&dir, &[("areka.last.ghost", "g")]);
    let backend: Arc<dyn StoreBackend> = FsBackend::with_manual_clock();
    let withhold = WithholdGuard::under(dir.path());
    let mut store = open_store(backend.clone(), &root);
    assert_eq!(
        outcome(store.try_write(&write(&root, &[("areka.last.ghost", "h")], &[]))),
        PersistOutcome::Withheld
    );
    drop(store);
    drop(withhold);
    let _again = WithholdGuard::under(dir.path());
    assert_eq!(
        read(&mut open_store(backend, &root), "areka.last.ghost").as_deref(),
        Some("g")
    );
}

#[test]
fn memory_in_an_unwritable_folder_is_read_without_making_files() {
    let dir = TempPath::under_target("sylphya-held-unwritable");
    let root = real_layer(&dir, &[("areka.last.ghost", "g")]);
    let file = root.join("sylphya.sqlite");
    let mut permissions = std::fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&file, permissions.clone()).unwrap();
    let before = tree(dir.path());
    let _withhold = WithholdGuard::under(dir.path());
    let backend: Arc<dyn StoreBackend> = FsBackend::with_manual_clock();

    let mut store = open_store(backend.clone(), &root);
    assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("g"));
    assert_eq!(
        outcome(store.try_write(&write(&root, &[("areka.boot.count", "1")], &[]))),
        PersistOutcome::Withheld
    );
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("1"));
    drop(store);
    let after = tree(dir.path());
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "Windows のファイルの読み取り専用の属性を戻す"
    )]
    permissions.set_readonly(false);
    std::fs::set_permissions(&file, permissions).unwrap();
    assert_eq!(after, before, "ファイルを 1 つも作らない");
}

#[test]
fn read_only_import_holds_the_legacy_values_for_the_run() {
    let dir = TempPath::under_target("sylphya-held-import");
    let root = real_layer(&dir, &[("areka.last.ghost", "g")]);
    std::fs::write(
        root.join("sylphya.toml"),
        "format-version = 1\n[last]\nghost = \"old\"\nshell = \"s\"\nballoon = \"b\"\n[boot]\ncount = \"5\"\n",
    )
    .unwrap();
    let before = tree(dir.path());
    let _withhold = WithholdGuard::under(dir.path());
    let backend: Arc<dyn StoreBackend> = FsBackend::with_manual_clock();

    let mut writer = open_store(backend.clone(), &root);
    assert_eq!(
        outcome(writer.try_write(&write(
            &root,
            &[("areka.last.shell", "t")],
            &["areka.last.balloon"]
        ))),
        PersistOutcome::Withheld
    );
    let mut first = open_store(backend.clone(), &root);
    let mut second = open_store(backend.clone(), &root);
    for store in [&mut first, &mut second] {
        assert_eq!(
            read(store, "areka.last.ghost").as_deref(),
            Some("g"),
            "ファイルの値が勝つ"
        );
        assert_eq!(read(store, "areka.boot.count").as_deref(), Some("5"));
        assert_eq!(
            read(store, "areka.last.shell").as_deref(),
            Some("t"),
            "書いた値が勝つ"
        );
        assert_eq!(read(store, "areka.last.balloon"), None, "消した鍵は値なし");
    }
    assert_eq!(
        second.read_layer(GHOST),
        [
            ("areka.boot.count".to_owned(), "5".to_owned()),
            ("areka.last.ghost".to_owned(), "g".to_owned()),
            ("areka.last.shell".to_owned(), "t".to_owned()),
        ]
    );
    drop((writer, first, second));
    assert_eq!(tree(dir.path()), before, "ファイルを作らず名前も替えない");
}

#[test]
fn read_only_import_into_a_layer_without_a_file_makes_nothing() {
    let mem = MemBackend::new();
    let root = PathBuf::from(r"C:\偽の置き場\held-import-mem\ghost");
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"5\"\n");
    let _withhold = WithholdGuard::under(&root);
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("5"));
    assert_eq!(
        read(&mut open_store(mem.clone(), &root), "areka.boot.count").as_deref(),
        Some("5")
    );
    assert_eq!(
        mem.locate(&root, Access::ReadOnly).unwrap(),
        Located::Missing
    );
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
}

#[test]
fn unreadable_import_mark_inside_is_recorded_and_reads_go_on() {
    let dir = TempPath::under_target("sylphya-held-mark-unreadable");
    let root = real_layer(&dir, &[("areka.last.ghost", "g")]);
    rusqlite::Connection::open(root.join("sylphya.sqlite"))
        .unwrap()
        .execute_batch("DROP TABLE meta")
        .unwrap();
    std::fs::write(
        root.join("sylphya.toml"),
        "format-version = 1\n[boot]\ncount = \"5\"\n",
    )
    .unwrap();
    let before = tree(dir.path());
    let _withhold = WithholdGuard::under(dir.path());
    let mut store = open_store(FsBackend::with_manual_clock(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("g"));
    });
    let failed: Vec<_> = log
        .iter()
        .filter(|e| {
            e.level == Level::ERROR && e.field_str("event") == Some("persist_import_failed")
        })
        .collect();
    assert_eq!(failed.len(), 1, "{log:?}");
    drop(store);
    assert_eq!(tree(dir.path()), before);
}

#[test]
fn deleting_a_pending_legacy_key_inside_hides_the_legacy_value() {
    let dir = TempPath::under_target("sylphya-held-delete-pending");
    let root = real_layer(&dir, &[("areka.last.ghost", "g")]);
    rusqlite::Connection::open(root.join("sylphya.sqlite"))
        .unwrap()
        .execute_batch("DROP TABLE meta")
        .unwrap();
    std::fs::write(
        root.join("sylphya.toml"),
        "format-version = 1
[boot]
count = \"5\"
",
    )
    .unwrap();
    let _withhold = WithholdGuard::under(dir.path());
    let mut store = open_store(FsBackend::with_manual_clock(), &root);
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("5"));
    assert_eq!(
        outcome(store.try_write(&write(&root, &[], &["areka.boot.count"]))),
        PersistOutcome::Withheld
    );
    assert_eq!(read(&mut store, "areka.boot.count"), None);
    assert!(
        !store
            .read_layer(GHOST)
            .iter()
            .any(|(k, _)| k == "areka.boot.count")
    );
}
