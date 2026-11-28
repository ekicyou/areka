use super::*;
use crate::persist::withhold::WithholdGuard;
use crate::store::{FsBackend, MemBackend, UNUSABLE_APPLICATION_ID};
use crate::test_log_capture::capture;
use log_capture_kit::CapturedEvent;
use std::sync::Arc;
use temp_path_kit::TempPath;
use tracing::Level;

const ALIAS: &str = "layer";

fn open(
    backend: &dyn StoreBackend,
    root: &Path,
    reported: &mut BTreeSet<PathBuf>,
) -> (Connection, Opened) {
    let conn = open_connection().unwrap();
    let opened = open_layer(&conn, backend, root, ALIAS, reported);
    (conn, opened)
}

fn attached(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT 1 FROM pragma_database_list WHERE name = ?1",
        [ALIAS],
        |_| Ok(()),
    )
    .is_ok()
}

fn layer_value(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        &format!("SELECT value FROM {ALIAS}.kv WHERE key = ?1"),
        [key],
        |r| r.get(0),
    )
    .ok()
}

/// 版 `version` の areka の層の DB を `conn` に作る（`log` は直しの段が当たった跡）。
fn plant(conn: &Connection, version: i32) {
    conn.execute_batch(&format!(
        "PRAGMA application_id = {APPLICATION_ID};
         PRAGMA user_version = {version};
         CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
         CREATE TABLE log (step TEXT);
         INSERT INTO kv VALUES ('a', '1'), ('empty', '');"
    ))
    .unwrap();
}

fn plant_mem(mem: &MemBackend, root: &Path, version: i32) {
    let Located::Uri(uri) = mem.locate(root, Access::Create).unwrap() else {
        panic!("偽の層は作れるはず");
    };
    plant(&Connection::open(uri).unwrap(), version);
}

fn plant_file(root: &Path, version: i32) {
    plant(
        &Connection::open(root.join("sylphya.sqlite")).unwrap(),
        version,
    );
}

fn user_version(conn: &Connection) -> i32 {
    conn.query_row(&format!("PRAGMA {ALIAS}.user_version"), [], |r| r.get(0))
        .unwrap()
}

fn steps_logged(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("SELECT step FROM {ALIAS}.log ORDER BY rowid"))
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// 版 1 → 2: `kv` を列の 1 つ多い表へ作り直す（値は写す）。
fn step_rebuild(conn: &Connection, a: &str) -> rusqlite::Result<()> {
    conn.execute_batch(&format!(
        "CREATE TABLE {a}.kv_next (key TEXT PRIMARY KEY, value TEXT NOT NULL, note TEXT) WITHOUT ROWID;
         INSERT INTO {a}.kv_next (key, value) SELECT key, value FROM {a}.kv;
         DROP TABLE {a}.kv;
         ALTER TABLE {a}.kv_next RENAME TO kv;
         INSERT INTO {a}.log VALUES ('1to2');"
    ))
}

/// 版 2 → 3。
fn step_mark(conn: &Connection, a: &str) -> rusqlite::Result<()> {
    conn.execute_batch(&format!("INSERT INTO {a}.log VALUES ('2to3');"))
}

fn step_fail(_conn: &Connection, _a: &str) -> rusqlite::Result<()> {
    Err(rusqlite::Error::InvalidQuery)
}

const TWO_STEPS: &[Step] = &[step_rebuild, step_mark];

type MakeFile = fn(&Path);

fn events<'a>(log: &'a [CapturedEvent], level: Level, event: &str) -> Vec<&'a CapturedEvent> {
    log.iter()
        .filter(|e| e.level == level && e.field_str("event") == Some(event))
        .collect()
}

fn skip_warnings(log: &[CapturedEvent]) -> usize {
    log.iter()
        .filter(|e| e.level == Level::WARN && e.message().contains("skipping this layer for reads"))
        .count()
}

#[test]
fn connection_keeps_temp_tables_in_memory_and_never_waits_inside_sqlite() {
    let conn = open_connection().unwrap();
    let busy: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
        .unwrap();
    let temp: i64 = conn
        .query_row("PRAGMA temp_store", [], |r| r.get(0))
        .unwrap();
    assert_eq!(busy, 0);
    assert_eq!(temp, 2, "2 は MEMORY");
}

#[test]
fn arekas_mark_is_not_the_foreign_mark_of_the_fake() {
    assert_ne!(APPLICATION_ID, UNUSABLE_APPLICATION_ID);
    assert_eq!(current_version(STEPS), 1);
}

#[test]
fn missing_layer_creates_neither_folder_nor_file_in_both_runs() {
    let dir = TempPath::under_target("sylphya-schema-missing");
    let root = dir.child("無い置き場");
    let fs = FsBackend::with_manual_clock();
    let mut reported = BTreeSet::new();

    let (conn, opened) = open(fs.as_ref(), &root, &mut reported);
    assert_eq!(opened, Opened::Missing);
    assert!(!attached(&conn));

    let _withhold = WithholdGuard::under(dir.path());
    let (conn, opened) = open(fs.as_ref(), &root, &mut reported);
    assert_eq!(opened, Opened::Missing);
    assert!(!attached(&conn));
    assert!(!root.exists());
}

#[test]
fn zero_byte_file_is_a_fresh_empty_layer_in_both_runs() {
    let dir = TempPath::under_target("sylphya-schema-zero");
    let root = dir.path();
    let file = root.join("sylphya.sqlite");
    std::fs::write(&file, b"").unwrap();
    let fs = FsBackend::with_manual_clock();
    let mut reported = BTreeSet::new();

    let (conn, opened) = open(fs.as_ref(), root, &mut reported);
    assert_eq!(opened, Opened::Attached { fresh: true });
    assert!(attached(&conn));
    drop(conn);

    let _withhold = WithholdGuard::under(root);
    let (conn, opened) = open(fs.as_ref(), root, &mut reported);
    assert_eq!(opened, Opened::Attached { fresh: true });
    drop(conn);
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 0);
}

#[test]
fn current_version_layer_is_used_in_both_runs() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\schema-current");
    plant_mem(&mem, root, 1);
    let mut reported = BTreeSet::new();

    let (conn, opened) = open(mem.as_ref(), root, &mut reported);
    assert_eq!(opened, Opened::Attached { fresh: false });
    assert_eq!(layer_value(&conn, "a").as_deref(), Some("1"));
    assert_eq!(layer_value(&conn, "empty").as_deref(), Some(""));

    let _withhold = WithholdGuard::under(root);
    let (conn, opened) = open(mem.as_ref(), root, &mut reported);
    assert_eq!(opened, Opened::Attached { fresh: false });
    assert_eq!(layer_value(&conn, "a").as_deref(), Some("1"));
}

#[test]
fn unusable_files_are_left_byte_for_byte_unchanged_and_not_attached() {
    let foreign_id = |path: &Path| {
        Connection::open(path)
            .unwrap()
            .execute_batch("PRAGMA application_id = 7; CREATE TABLE t(x);")
            .unwrap();
    };
    let newer = |path: &Path| {
        Connection::open(path)
            .unwrap()
            .execute_batch(&format!(
                "PRAGMA application_id = {APPLICATION_ID}; PRAGMA user_version = 2; CREATE TABLE kv(k);"
            ))
            .unwrap();
    };
    let unversioned = |path: &Path| {
        Connection::open(path)
            .unwrap()
            .execute_batch(&format!(
                "PRAGMA application_id = {APPLICATION_ID}; CREATE TABLE kv(k);"
            ))
            .unwrap();
    };
    let not_sqlite =
        |path: &Path| std::fs::write(path, "これは SQLite のファイルではない".repeat(40)).unwrap();
    let cases: [(&str, MakeFile); 4] = [
        ("foreign-id", foreign_id),
        ("newer-version", newer),
        ("version-0", unversioned),
        ("not-sqlite", not_sqlite),
    ];
    let fs = FsBackend::with_manual_clock();
    for (label, make) in cases {
        let dir = TempPath::under_target(&format!("sylphya-schema-unusable-{label}"));
        let root = dir.path();
        let file = root.join("sylphya.sqlite");
        make(&file);
        let before = std::fs::read(&file).unwrap();
        let mut reported = BTreeSet::new();

        let (conn, opened) = open(fs.as_ref(), root, &mut reported);
        let Opened::Unusable(reason) = &opened else {
            panic!("{label}: {opened:?}");
        };
        assert!(reason.contains("sylphya.sqlite"), "{label}: {reason}");
        assert!(!attached(&conn), "{label}");
        drop(conn);

        let withhold = WithholdGuard::under(root);
        let (conn, opened) = open(fs.as_ref(), root, &mut reported);
        assert!(
            matches!(opened, Opened::Unusable(_)),
            "{label} (read-only): {opened:?}"
        );
        assert!(!attached(&conn), "{label}");
        drop((conn, withhold));

        assert_eq!(std::fs::read(&file).unwrap(), before, "{label}");
        let names: Vec<_> = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["sylphya.sqlite"], "{label}");
    }
}

#[test]
fn fake_unusable_layer_is_unusable() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\schema-fake-unusable");
    mem.mark_unusable(root);
    let (conn, opened) = open(mem.as_ref(), root, &mut BTreeSet::new());
    assert!(matches!(opened, Opened::Unusable(_)), "{opened:?}");
    assert!(!attached(&conn));
}

#[test]
fn unusable_layer_warns_once_per_place() {
    let mem = MemBackend::new();
    let first = Path::new(r"C:\偽の置き場\schema-warn-1");
    let second = Path::new(r"C:\偽の置き場\schema-warn-2");
    mem.mark_unusable(first);
    mem.mark_unusable(second);
    let mut reported = BTreeSet::new();
    let log = capture(|| {
        for _ in 0..3 {
            open(mem.as_ref(), first, &mut reported);
        }
        open(mem.as_ref(), second, &mut reported);
    });
    assert_eq!(skip_warnings(&log), 2);
}

#[test]
fn two_step_migration_lifts_an_old_version_without_losing_values() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\schema-upgrade-from-1");
    plant_mem(&mem, root, 1);
    let mut reported = BTreeSet::new();
    let conn = open_connection().unwrap();

    let log = capture(|| {
        let opened = open_layer_with(&conn, mem.as_ref(), root, ALIAS, &mut reported, TWO_STEPS);
        assert_eq!(opened, Opened::Attached { fresh: false });
    });
    assert_eq!(user_version(&conn), 3);
    assert_eq!(steps_logged(&conn), ["1to2", "2to3"]);
    assert_eq!(layer_value(&conn, "a").as_deref(), Some("1"));
    assert_eq!(layer_value(&conn, "empty").as_deref(), Some(""));
    assert_eq!(
        events(&log, Level::INFO, "persist_schema_upgraded").len(),
        1
    );
    conn.execute_batch(&format!("DETACH {ALIAS}")).unwrap();

    let log = capture(|| {
        let opened = open_layer_with(&conn, mem.as_ref(), root, ALIAS, &mut reported, TWO_STEPS);
        assert_eq!(opened, Opened::Attached { fresh: false });
    });
    assert_eq!(steps_logged(&conn), ["1to2", "2to3"], "今の版は直さない");
    assert!(events(&log, Level::INFO, "persist_schema_upgraded").is_empty());
}

#[test]
fn migration_starts_from_the_files_own_version() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\schema-upgrade-from-2");
    plant_mem(&mem, root, 2);
    let conn = open_connection().unwrap();
    let opened = open_layer_with(
        &conn,
        mem.as_ref(),
        root,
        ALIAS,
        &mut BTreeSet::new(),
        TWO_STEPS,
    );
    assert_eq!(opened, Opened::Attached { fresh: false });
    assert_eq!(user_version(&conn), 3);
    assert_eq!(steps_logged(&conn), ["2to3"]);
}

#[test]
fn failed_step_rolls_back_every_step_and_leaves_the_layer_detached() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\schema-upgrade-fail");
    plant_mem(&mem, root, 1);
    let conn = open_connection().unwrap();
    let opened = open_layer_with(
        &conn,
        mem.as_ref(),
        root,
        ALIAS,
        &mut BTreeSet::new(),
        &[step_rebuild, step_fail],
    );
    assert!(matches!(opened, Opened::Io(_)), "{opened:?}");
    assert!(!attached(&conn));
    assert!(conn.is_autocommit());

    let opened = open_layer_with(&conn, mem.as_ref(), root, ALIAS, &mut BTreeSet::new(), &[]);
    assert_eq!(opened, Opened::Attached { fresh: false }, "版 1 のまま");
    assert!(steps_logged(&conn).is_empty());
    assert_eq!(layer_value(&conn, "a").as_deref(), Some("1"));
}

#[test]
fn read_only_run_skips_an_old_version_with_one_error_and_changes_nothing() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\schema-readonly-upgrade");
    plant_mem(&mem, root, 1);
    let _withhold = WithholdGuard::under(root);
    let mut reported = BTreeSet::new();
    let conn = open_connection().unwrap();

    let log = capture(|| {
        for _ in 0..3 {
            let opened =
                open_layer_with(&conn, mem.as_ref(), root, ALIAS, &mut reported, TWO_STEPS);
            assert_eq!(opened, Opened::NeedsUpgrade);
            assert!(!attached(&conn));
        }
    });
    assert_eq!(
        events(&log, Level::ERROR, "persist_readonly_needs_upgrade").len(),
        1
    );
    assert!(events(&log, Level::INFO, "persist_schema_upgraded").is_empty());

    drop(_withhold);
    let opened = open_layer_with(&conn, mem.as_ref(), root, ALIAS, &mut BTreeSet::new(), &[]);
    assert_eq!(opened, Opened::Attached { fresh: false });
    assert_eq!(user_version(&conn), 1);
    assert!(steps_logged(&conn).is_empty());
}

/// 書きの途中で落ちた形（`sylphya.sqlite` と、中身の在るジャーナル）を `dest` に写す。
fn plant_hot_journal(dest: &Path) {
    let source = TempPath::under_target("sylphya-schema-journal-source");
    plant_file(source.path(), 1);
    let writer = Connection::open(source.path().join("sylphya.sqlite")).unwrap();
    // 小さい page cache で書きをディスクへあふれさせ、ジャーナルの頭を確定させる（落ちた後と同じ形）。
    writer
        .execute_batch(
            "PRAGMA cache_size = 1;
             BEGIN;
             UPDATE kv SET value = 'half-written';
             INSERT INTO kv VALUES ('b', '2');
             WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 2000)
             INSERT INTO kv SELECT 'filler-' || i, printf('%0200d', i) FROM n;",
        )
        .unwrap();
    for name in ["sylphya.sqlite", "sylphya.sqlite-journal"] {
        std::fs::copy(source.path().join(name), dest.join(name)).unwrap();
    }
    drop(writer);
    let journal = std::fs::read(dest.join("sylphya.sqlite-journal")).unwrap();
    assert!(
        journal.first().is_some_and(|b| *b != 0),
        "ジャーナルに中身が在るはず"
    );
}

#[test]
fn read_only_run_skips_a_hot_journal_with_one_error_and_changes_nothing() {
    let dir = TempPath::under_target("sylphya-schema-readonly-journal");
    let root = dir.path();
    plant_hot_journal(root);
    let read_all = |name: &str| std::fs::read(root.join(name)).unwrap();
    let before = (
        read_all("sylphya.sqlite"),
        read_all("sylphya.sqlite-journal"),
    );
    let fs: Arc<FsBackend> = FsBackend::with_manual_clock();
    let _withhold = WithholdGuard::under(root);
    let mut reported = BTreeSet::new();

    let log = capture(|| {
        for _ in 0..3 {
            let (conn, opened) = open(fs.as_ref(), root, &mut reported);
            assert_eq!(opened, Opened::NeedsRecovery);
            assert!(!attached(&conn));
        }
    });
    assert_eq!(
        events(&log, Level::ERROR, "persist_readonly_needs_recovery").len(),
        1
    );
    assert_eq!(
        (
            read_all("sylphya.sqlite"),
            read_all("sylphya.sqlite-journal")
        ),
        before
    );
}

#[test]
fn normal_run_rolls_a_hot_journal_back_on_open() {
    let dir = TempPath::under_target("sylphya-schema-normal-journal");
    let root = dir.path();
    plant_hot_journal(root);
    let fs = FsBackend::with_manual_clock();

    let (conn, opened) = open(fs.as_ref(), root, &mut BTreeSet::new());
    assert_eq!(opened, Opened::Attached { fresh: false });
    assert_eq!(layer_value(&conn, "a").as_deref(), Some("1"));
    assert_eq!(layer_value(&conn, "b"), None);
    drop(conn);
    assert!(!root.join("sylphya.sqlite-journal").exists());
}
