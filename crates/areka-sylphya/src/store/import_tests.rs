use crate::asker::AskerId;
use crate::keytable::Home;
use crate::persist::withhold::WithholdGuard;
use crate::persist::{PersistFailureCause, PersistOutcome, PersistScope, ScopeRoots, format};
use crate::store::write::{FileWrite, WriteAttempt};
use crate::store::{Access, FsBackend, Located, MemBackend, Store, StoreBackend};
use crate::test_log_capture::capture;
use log_capture_kit::CapturedEvent;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use temp_path_kit::TempPath;
use tracing::Level;

const GHOST: PersistScope = PersistScope::Ghost;

fn ghost_root(tag: &str) -> PathBuf {
    PathBuf::from(format!(r"C:\偽の置き場\import-{tag}\ghost"))
}

fn roots_at(root: &Path) -> ScopeRoots {
    ScopeRoots {
        ghost: Some(root.to_path_buf()),
        ..ScopeRoots::default()
    }
}

fn open_store(backend: Arc<dyn StoreBackend>, root: &Path) -> Store {
    Store::open(roots_at(root), backend).unwrap()
}

fn read(store: &mut Store, key: &str) -> Option<String> {
    store.read(&AskerId::new("ghost-a"), key, Some(GHOST), Home::File)
}

fn write(root: &Path, pairs: &[(&str, &str)]) -> FileWrite {
    FileWrite {
        layer: GHOST,
        root: Some(root.to_path_buf()),
        puts: pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        removes: Vec::new(),
        typed: Vec::new(),
    }
}

fn saved(attempt: WriteAttempt) {
    assert!(
        matches!(attempt, WriteAttempt::Done(PersistOutcome::Saved)),
        "{attempt:?}"
    );
}

fn unusable(attempt: WriteAttempt) {
    assert!(
        matches!(
            &attempt,
            WriteAttempt::Done(PersistOutcome::Degraded(f))
                if matches!(f.cause, PersistFailureCause::Unusable(_))
        ),
        "{attempt:?}"
    );
}

/// 上げる前の版の読みが同じ `sylphya.toml` から返していた（正規名, 値）の並び。
fn load_scope_reads(toml: &str) -> Vec<(String, String)> {
    super::doc_to_entries(GHOST, &format::parse_toml_str(toml).unwrap().0)
        .into_iter()
        .map(|(key, value)| (key.to_canonical_key(), value))
        .collect()
}

fn events<'a>(log: &'a [CapturedEvent], level: Level, event: &str) -> Vec<&'a CapturedEvent> {
    log.iter()
        .filter(|e| e.level == level && e.field_str("event") == Some(event))
        .collect()
}

fn no_layer_file(mem: &MemBackend, root: &Path) -> bool {
    mem.locate(root, Access::ReadOnly).unwrap() == Located::Missing
}

const KINDS: [&str; 10] = [
    "[window.\"0\"]\nx = \"10\"\ny = \"-20\"\n",
    "[balloon-offset.\"1\"]\nx = \"-5\"\ny = \"7\"\n",
    "[boot]\ncount = \"3\"\n",
    "[vanish]\ncount = \"1\"\n",
    "[last]\nghost = \"emo2\"\n",
    "[last]\nballoon = \"白い吹き出し\"\n",
    "[last]\nshell = \"master\"\n",
    "[last]\nrunning = \"emo2\"\n",
    "[baseware]\nroots = \"\"\n",
    "[window-adjacent]\n\"0\" = \"left\"\n",
];

#[test]
fn each_persisted_kind_is_imported_with_the_value_load_scope_reads() {
    for (i, body) in KINDS.iter().enumerate() {
        let toml = format!("format-version = 1\n{body}");
        let expected = load_scope_reads(&toml);
        assert!(!expected.is_empty(), "見本 {i} に値が在るはず");
        let mem = MemBackend::new();
        let root = ghost_root(&format!("kind-{i}"));
        mem.plant_legacy(&root, &toml);

        let mut store = open_store(mem.clone(), &root);
        let log = capture(|| {
            for (key, value) in &expected {
                assert_eq!(read(&mut store, key).as_ref(), Some(value), "見本 {i}");
            }
        });
        let mut sorted = expected.clone();
        sorted.sort();
        assert_eq!(store.read_layer(GHOST), sorted, "見本 {i}");
        assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
        let imported = events(&log, Level::INFO, "persist_imported");
        assert_eq!(imported.len(), 1, "見本 {i}");
        assert_eq!(
            imported[0].field("imported"),
            Some(&*expected.len().to_string())
        );

        let mut reopened = open_store(mem.clone(), &root);
        assert_eq!(reopened.read_layer(GHOST), sorted, "開き直しても同じ");
    }
}

#[test]
fn every_kind_together_reads_as_before_the_upgrade() {
    let toml = "format-version = 1
        [window.\"0\"]
x = \"10\"
y = \"-20\"
        [balloon-offset.\"1\"]
x = \"-5\"
y = \"7\"
        [boot]
count = \"3\"
        [vanish]
count = \"1\"
        [last]
ghost = \"emo2\"
balloon = \"b\"
shell = \"master\"
running = \"emo2\"
        [baseware]
roots = \"r\"
        [window-adjacent]
\"0\" = \"left\"
";
    let expected = load_scope_reads(toml);
    assert_eq!(expected.len(), 12);
    let mem = MemBackend::new();
    let root = ghost_root("all");
    mem.plant_legacy(&root, toml);
    let mut store = open_store(mem, &root);
    for (key, value) in &expected {
        assert_eq!(read(&mut store, key).as_ref(), Some(value));
    }
}

#[test]
fn same_number_in_two_spellings_takes_the_first_spelling() {
    let toml = "format-version = 1\n[window.\"01\"]\nx = \"1\"\n[window.\"1\"]\nx = \"2\"\n";
    let first = load_scope_reads(toml)
        .into_iter()
        .find(|(key, _)| key == "areka.window.scope(1).x")
        .unwrap();
    let mem = MemBackend::new();
    let root = ghost_root("two-spellings");
    mem.plant_legacy(&root, toml);
    let mut store = open_store(mem, &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, &first.0), Some(first.1.clone()));
    });
    let imported = events(&log, Level::INFO, "persist_imported");
    assert_eq!(imported[0].field("imported"), Some("1"));
    assert_eq!(imported[0].field("skipped"), Some("1"));
}

#[test]
fn empty_running_mark_imports_as_absent() {
    let mem = MemBackend::new();
    let root = ghost_root("empty-running");
    mem.plant_legacy(
        &root,
        "format-version = 1\n[last]\nrunning = \"\"\nghost = \"g\"\n",
    );
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(read(&mut store, "areka.last.running"), None);
    assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("g"));
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
}

#[test]
fn unknown_tables_and_values_stay_out_and_are_counted() {
    let toml = "format-version = 1\n\
        [future]\na = \"1\"\nb = \"2\"\n\
        [last]\nghost = \"g\"\nmood = \"x\"\n\
        [window.\"abc\"]\nx = \"1\"\n\
        [boot]\ncount = 5\n";
    let mem = MemBackend::new();
    let root = ghost_root("unknown");
    mem.plant_legacy(&root, toml);
    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("g"));
    });
    assert_eq!(
        store.read_layer(GHOST),
        [("areka.last.ghost".to_owned(), "g".to_owned())]
    );
    let imported = events(&log, Level::INFO, "persist_imported");
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].field("unknown"), Some("5"));
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
}

#[test]
fn broken_or_other_version_legacy_leaves_everything_as_it_was() {
    for (i, toml) in [
        "format-version = [",
        "format-version = 99\n[boot]\ncount = \"1\"\n",
    ]
    .into_iter()
    .enumerate()
    {
        let mem = MemBackend::new();
        let root = ghost_root(&format!("broken-{i}"));
        mem.plant_legacy(&root, toml);
        let mut store = open_store(mem.clone(), &root);
        let log = capture(|| {
            for _ in 0..3 {
                assert_eq!(read(&mut store, "areka.boot.count"), None);
            }
        });
        let failed = events(&log, Level::ERROR, "persist_import_failed");
        assert_eq!(failed.len(), 1, "置き場ごとに 1 回");
        assert!(failed[0].field("root").is_some());
        assert!(failed[0].field("reason").is_some());
        unusable(store.try_write(&write(&root, &[("areka.boot.count", "2")])));
        assert!(no_layer_file(&mem, &root), "新しいファイルを作らない");
        assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
        assert_eq!(mem.read_legacy(&root).unwrap().as_deref(), Some(toml));
    }
}

#[test]
fn broken_legacy_next_to_a_usable_new_file_keeps_reads_and_refuses_writes() {
    let mem = MemBackend::new();
    let root = ghost_root("broken-next-to-new");
    saved(open_store(mem.clone(), &root).try_write(&write(&root, &[("areka.boot.count", "4")])));
    mem.plant_legacy(&root, "format-version = [");

    let mut store = open_store(mem.clone(), &root);
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("4"));
    assert_eq!(
        store.read_layer(GHOST),
        [("areka.boot.count".to_owned(), "4".to_owned())]
    );
    unusable(store.try_write(&write(&root, &[("areka.boot.count", "5")])));
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("4"));
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
}

#[test]
fn unreadable_legacy_leaves_the_layer_unusable() {
    let dir = TempPath::under_target("sylphya-import-unreadable");
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("sylphya.toml")).unwrap();
    let mut store = open_store(FsBackend::with_manual_clock(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.boot.count"), None);
    });
    assert_eq!(events(&log, Level::ERROR, "persist_import_failed").len(), 1);
    unusable(store.try_write(&write(&root, &[("areka.boot.count", "1")])));
    assert!(!root.join("sylphya.sqlite").exists());
    assert!(root.join("sylphya.toml").is_dir());
}

#[test]
fn a_failed_import_is_tried_again_on_the_next_open() {
    let mem = MemBackend::new();
    let root = ghost_root("retry-next-open");
    mem.plant_legacy(&root, "format-version = [");
    assert_eq!(
        read(&mut open_store(mem.clone(), &root), "areka.boot.count"),
        None
    );

    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"7\"\n");
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("7"));
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
}

#[test]
fn unusable_new_file_next_to_legacy_is_not_imported() {
    let mem = MemBackend::new();
    let root = ghost_root("unusable-new");
    mem.mark_unusable(&root);
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"1\"\n");
    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.boot.count"), None);
        unusable(store.try_write(&write(&root, &[("areka.boot.count", "2")])));
    });
    assert!(events(&log, Level::INFO, "persist_imported").is_empty());
    assert!(events(&log, Level::ERROR, "persist_import_failed").is_empty());
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
}

#[test]
fn values_already_in_the_new_file_are_not_overwritten() {
    let mem = MemBackend::new();
    let root = ghost_root("new-wins");
    saved(open_store(mem.clone(), &root).try_write(&write(&root, &[("areka.last.ghost", "new")])));
    mem.plant_legacy(
        &root,
        "format-version = 1\n[last]\nghost = \"old\"\nshell = \"s\"\n",
    );

    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("new"));
    });
    assert_eq!(read(&mut store, "areka.last.shell").as_deref(), Some("s"));
    let imported = events(&log, Level::INFO, "persist_imported");
    assert_eq!(imported[0].field("imported"), Some("1"));
    assert_eq!(imported[0].field("skipped"), Some("1"));
}

#[test]
fn cannot_rename_keeps_the_imported_values_and_the_next_open_only_renames() {
    let mem = MemBackend::new();
    let root = ghost_root("cannot-rename");
    mem.plant_legacy(
        &root,
        "format-version = 1\n[boot]\ncount = \"1\"\n[vanish]\ncount = \"2\"\n",
    );
    mem.fail_next_retire();

    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("1"));
    });
    assert_eq!(
        events(&log, Level::WARN, "persist_import_rename_failed").len(),
        1
    );
    assert_eq!(events(&log, Level::INFO, "persist_imported").len(), 1);
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
    saved(store.try_write(&write(&root, &[("areka.boot.count", "9")])));
    drop(store);

    let mut next = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut next, "areka.boot.count").as_deref(), Some("9"));
    });
    assert!(events(&log, Level::INFO, "persist_imported").is_empty());
    assert_eq!(events(&log, Level::INFO, "persist_import_skipped").len(), 1);
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
    assert_eq!(
        next.read_layer(GHOST),
        [
            ("areka.boot.count".to_owned(), "9".to_owned()),
            ("areka.vanish.count".to_owned(), "2".to_owned()),
        ]
    );
}

#[test]
fn a_crash_before_the_commit_leaves_nothing_and_the_next_open_imports_all() {
    let toml = "format-version = 1\n[boot]\ncount = \"1\"\n[last]\nghost = \"g\"\n";
    let mem = MemBackend::new();
    let root = ghost_root("crash-before-commit");
    mem.plant_legacy(&root, toml);
    mem.fail_next_write();

    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("1"));
    });
    assert_eq!(events(&log, Level::ERROR, "persist_import_failed").len(), 1);
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
    drop(store);

    let once = MemBackend::new();
    let once_root = ghost_root("crash-before-commit-once");
    once.plant_legacy(&once_root, toml);
    let in_one_go = open_store(once, &once_root).read_layer(GHOST);
    assert_eq!(open_store(mem.clone(), &root).read_layer(GHOST), in_one_go);
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
}

#[test]
fn legacy_again_after_the_import_only_renames_beside_the_kept_name() {
    let mem = MemBackend::new();
    let root = ghost_root("again");
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"1\"\n");
    assert_eq!(
        read(&mut open_store(mem.clone(), &root), "areka.boot.count").as_deref(),
        Some("1")
    );
    mem.plant_legacy(
        &root,
        "format-version = 1\n[boot]\ncount = \"9\"\n[vanish]\ncount = \"4\"\n",
    );

    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("1"));
        assert_eq!(read(&mut store, "areka.vanish.count"), None);
    });
    assert_eq!(events(&log, Level::INFO, "persist_import_skipped").len(), 1);
    assert!(events(&log, Level::INFO, "persist_imported").is_empty());
    assert_eq!(
        mem.legacy_names(&root),
        ["sylphya.toml.imported", "sylphya.toml.imported.1"]
    );
    saved(store.try_write(&write(&root, &[("areka.boot.count", "2")])));
}

#[test]
fn broken_legacy_beside_an_imported_layer_is_not_renamed_and_refuses_writes() {
    let mem = MemBackend::new();
    let root = ghost_root("broken-again");
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"1\"\n");
    assert_eq!(
        read(&mut open_store(mem.clone(), &root), "areka.boot.count").as_deref(),
        Some("1")
    );
    let broken = "format-version = [";
    mem.plant_legacy(&root, broken);

    let mut store = open_store(mem.clone(), &root);
    let log = capture(|| {
        for _ in 0..3 {
            assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("1"));
        }
    });
    assert_eq!(events(&log, Level::ERROR, "persist_import_failed").len(), 1);
    assert!(events(&log, Level::INFO, "persist_import_skipped").is_empty());
    unusable(store.try_write(&write(&root, &[("areka.boot.count", "2")])));
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("1"));
    assert_eq!(
        mem.legacy_names(&root),
        ["sylphya.toml", "sylphya.toml.imported"]
    );
    assert_eq!(mem.read_legacy(&root).unwrap().as_deref(), Some(broken));
}

#[test]
fn unreadable_import_mark_is_recorded_and_reads_go_on() {
    let dir = TempPath::under_target("sylphya-import-mark-unreadable");
    let root = real_layer_with_legacy(&dir, "format-version = 1\n[boot]\ncount = \"1\"\n");
    rusqlite::Connection::open(root.join("sylphya.sqlite"))
        .unwrap()
        .execute_batch("DROP TABLE meta")
        .unwrap();

    let mut store = open_store(FsBackend::with_manual_clock(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut store, "areka.last.ghost").as_deref(), Some("g"));
    });
    let failed = events(&log, Level::ERROR, "persist_import_failed");
    assert_eq!(failed.len(), 1);
    assert!(failed[0].field("root").is_some());
    assert!(failed[0].field("reason").is_some());
    assert!(root.join("sylphya.toml").exists());
}

/// 本物のファイルの層（取り込みの前の値を 1 つ持つ）と、その隣の `sylphya.toml`。
fn real_layer_with_legacy(dir: &TempPath, toml: &str) -> PathBuf {
    let root = dir.child("ghost");
    saved(
        open_store(FsBackend::with_manual_clock(), &root)
            .try_write(&write(&root, &[("areka.last.ghost", "g")])),
    );
    std::fs::write(root.join("sylphya.toml"), toml).unwrap();
    root
}

/// 別の書き手として、層のファイルの書きの錠を取る。
fn hold_writer(root: &Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(root.join("sylphya.sqlite")).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    conn
}

#[test]
fn two_cores_importing_at_once_lose_nothing() {
    let dir = TempPath::under_target("sylphya-import-two-cores");
    let root = real_layer_with_legacy(
        &dir,
        "format-version = 1
[boot]
count = \"1\"
[vanish]
count = \"2\"
",
    );
    let backend = FsBackend::with_manual_clock();
    let mut a = open_store(backend.clone(), &root);
    let mut b = open_store(backend.clone(), &root);

    let hold = hold_writer(&root);
    for core in [&mut a, &mut b] {
        let log = capture(|| {
            assert_eq!(read(core, "areka.boot.count").as_deref(), Some("1"));
        });
        assert!(events(&log, Level::INFO, "persist_imported").is_empty());
        assert_eq!(
            core.read_layer(GHOST),
            [
                ("areka.boot.count".to_owned(), "1".to_owned()),
                ("areka.last.ghost".to_owned(), "g".to_owned()),
                ("areka.vanish.count".to_owned(), "2".to_owned()),
            ]
        );
        assert!(matches!(
            core.try_write(&write(&root, &[("areka.boot.count", "x")])),
            WriteAttempt::Busy
        ));
    }
    assert!(
        root.join("sylphya.toml").exists(),
        "錠が取れない間は名前を替えない"
    );
    drop(hold);

    saved(a.try_write(&write(&root, &[("areka.boot.count", "10")])));
    assert_eq!(read(&mut b, "areka.boot.count").as_deref(), Some("10"));
    saved(b.try_write(&write(&root, &[("areka.vanish.count", "20")])));
    assert_eq!(read(&mut a, "areka.vanish.count").as_deref(), Some("20"));
    assert_eq!(read(&mut a, "areka.last.ghost").as_deref(), Some("g"));
    assert!(!root.join("sylphya.toml").exists());
    assert!(root.join("sylphya.toml.imported").exists());
}

#[test]
fn a_core_that_saw_the_legacy_values_drops_them_once_another_core_imported() {
    let dir = TempPath::under_target("sylphya-import-drop-overlay");
    let root = real_layer_with_legacy(
        &dir,
        "format-version = 1
[vanish]
count = \"2\"
",
    );
    let backend = FsBackend::with_manual_clock();
    let mut a = open_store(backend.clone(), &root);
    let mut b = open_store(backend.clone(), &root);
    let hold = hold_writer(&root);
    assert_eq!(read(&mut b, "areka.vanish.count").as_deref(), Some("2"));
    drop(hold);

    assert_eq!(read(&mut a, "areka.vanish.count").as_deref(), Some("2"));
    let remove = FileWrite {
        removes: vec!["areka.vanish.count".to_owned()],
        ..write(&root, &[])
    };
    saved(a.try_write(&remove));
    assert_eq!(read(&mut b, "areka.vanish.count"), None);
}

#[test]
fn set_root_forgets_that_the_layer_was_imported() {
    let mem = MemBackend::new();
    let root = ghost_root("set-root");
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(read(&mut store, "areka.boot.count"), None);
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"3\"\n");
    assert_eq!(
        read(&mut store, "areka.boot.count"),
        None,
        "同じ置き場は 1 回だけ見る"
    );

    store.set_root(GHOST, Some(root.clone()));
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("3"));
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
}

#[test]
fn read_only_run_reads_the_legacy_values_without_making_or_renaming_files() {
    let mem = MemBackend::new();
    let root = ghost_root("read-only");
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"5\"\n");
    let _withhold = WithholdGuard::under(&root);
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(read(&mut store, "areka.boot.count").as_deref(), Some("5"));
    assert!(no_layer_file(&mem, &root));
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
}

#[test]
fn import_on_real_files_renames_and_never_overwrites_the_kept_name() {
    let dir = TempPath::under_target("sylphya-import-real");
    let root = dir.child("ghost");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("sylphya.toml.imported"), "前に残したもの").unwrap();
    std::fs::write(
        root.join("sylphya.toml"),
        "format-version = 1\n[last]\nghost = \"emo2\"\n",
    )
    .unwrap();
    let mut store = open_store(FsBackend::with_manual_clock(), &root);
    assert_eq!(
        read(&mut store, "areka.last.ghost").as_deref(),
        Some("emo2")
    );
    assert!(!root.join("sylphya.toml").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("sylphya.toml.imported")).unwrap(),
        "前に残したもの"
    );
    assert!(
        std::fs::read_to_string(root.join("sylphya.toml.imported.1"))
            .unwrap()
            .contains("emo2")
    );
    let moved = dir.child("moved");
    std::fs::rename(&root, &moved).expect("取り込みの後は層のフォルダを掴んでいない");
    drop(store);
}

#[test]
fn read_only_run_beside_an_imported_layer_reads_what_a_normal_run_reads() {
    let toml = "format-version = 1\n[last]\nrunning = \"emo2\"\nghost = \"g\"\n";
    let mem = MemBackend::new();
    let root = ghost_root("read-only-after-import");
    mem.plant_legacy(&root, toml);
    mem.fail_next_retire();
    let mut store = open_store(mem.clone(), &root);
    assert_eq!(
        read(&mut store, "areka.last.running").as_deref(),
        Some("emo2")
    );
    let remove = FileWrite {
        removes: vec!["areka.last.running".to_owned()],
        ..write(&root, &[("areka.last.ghost", "h")])
    };
    saved(store.try_write(&remove));
    drop(store);
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);

    let _withhold = WithholdGuard::under(&root);
    let mut read_only = open_store(mem.clone(), &root);
    let log = capture(|| {
        assert_eq!(read(&mut read_only, "areka.last.running"), None);
        assert_eq!(
            read(&mut read_only, "areka.last.ghost").as_deref(),
            Some("h")
        );
    });
    assert_eq!(
        read_only.read_layer(GHOST),
        [("areka.last.ghost".to_owned(), "h".to_owned())]
    );
    assert!(events(&log, Level::INFO, "persist_imported").is_empty());
    assert!(events(&log, Level::ERROR, "persist_import_failed").is_empty());
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml"]);
    assert_eq!(mem.read_legacy(&root).unwrap().as_deref(), Some(toml));
}

#[test]
fn a_layer_file_that_cannot_be_made_for_the_import_fails_with_the_os_reason() {
    use crate::guardian::wait_tests::{Hooked, INJECTED_DIR_FAILURE};
    let mem = MemBackend::new();
    let root = ghost_root("cannot-make-dir");
    mem.plant_legacy(&root, "format-version = 1\n[boot]\ncount = \"1\"\n");
    let backend = Arc::new(Hooked {
        inner: mem,
        volatile_uri: None,
        panic_on_write: false,
        fail_ensure_dir: true,
    });
    let mut store = open_store(backend, &root);
    let attempt = store.try_write(&write(&root, &[("areka.boot.count", "2")]));
    assert!(
        matches!(
            &attempt,
            WriteAttempt::Done(PersistOutcome::Degraded(f))
                if f.cause == PersistFailureCause::Io(INJECTED_DIR_FAILURE.to_owned())
        ),
        "{attempt:?}"
    );
}
