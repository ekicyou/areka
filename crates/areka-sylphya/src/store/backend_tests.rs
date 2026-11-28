use super::*;
use rusqlite::Connection;
use std::time::Duration;
use temp_path_kit::TempPath;

fn uri_of(located: Located) -> String {
    match located {
        Located::Uri(uri) => uri,
        Located::Missing => panic!("在りかが返るはず"),
    }
}

fn attach(uri: &str) -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("ATTACH ?1 AS layer", [uri]).unwrap();
    conn
}

#[test]
fn fs_retire_claims_a_fresh_name_and_never_overwrites() {
    let dir = TempPath::under_target("sylphya-backend-retire");
    let backend = fs_backend();
    let root = dir.path();

    assert_eq!(backend.read_legacy(root).unwrap(), None);
    backend.retire_legacy(root).unwrap();

    std::fs::write(root.join("sylphya.toml"), "new").unwrap();
    std::fs::write(root.join("sylphya.toml.imported"), "old").unwrap();
    assert_eq!(backend.read_legacy(root).unwrap().as_deref(), Some("new"));

    backend.retire_legacy(root).unwrap();
    assert!(!root.join("sylphya.toml").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("sylphya.toml.imported")).unwrap(),
        "old"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("sylphya.toml.imported.1")).unwrap(),
        "new"
    );
    assert_eq!(backend.read_legacy(root).unwrap(), None);
}

#[test]
fn fs_read_legacy_does_not_create_the_root() {
    let dir = TempPath::under_target("sylphya-backend-absent");
    let root = dir.child("無い置き場");
    assert_eq!(fs_backend().read_legacy(&root).unwrap(), None);
    fs_backend().retire_legacy(&root).unwrap();
    assert!(!root.exists());
}

#[test]
fn manual_clocks_advance_without_sleeping() {
    let fs = FsBackend::with_manual_clock();
    assert_eq!(fs.now(), Duration::ZERO);
    fs.pause(Duration::from_secs(3600));
    assert_eq!(fs.now(), Duration::from_secs(3600));

    let mem = MemBackend::new();
    mem.pause(Duration::from_millis(10));
    mem.advance(Duration::from_secs(1));
    assert_eq!(mem.now(), Duration::from_millis(1010));
}

#[test]
fn real_clock_is_monotonic() {
    let backend = fs_backend();
    let a = backend.now();
    let b = backend.now();
    assert!(b >= a);
}

#[test]
fn mem_layer_is_missing_until_created_and_survives_closed_connections() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽の置き場\ghost");
    assert!(matches!(
        mem.locate(root, Access::ReadWrite).unwrap(),
        Located::Missing
    ));

    let create = uri_of(mem.locate(root, Access::Create).unwrap());
    {
        let conn = attach(&create);
        conn.execute_batch(
            "CREATE TABLE layer.kv(k TEXT, v TEXT); INSERT INTO layer.kv VALUES('a','1');",
        )
        .unwrap();
    }
    let rw = uri_of(mem.locate(root, Access::ReadWrite).unwrap());
    let v: String = attach(&rw)
        .query_row("SELECT v FROM layer.kv", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, "1");

    let other = MemBackend::new();
    assert!(matches!(
        other.locate(root, Access::ReadWrite).unwrap(),
        Located::Missing
    ));
    let other_create = uri_of(other.locate(root, Access::Create).unwrap());
    assert_ne!(other_create, create);
}

#[test]
fn mem_fail_next_write_fails_exactly_once() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽");
    mem.before_write(root).unwrap();
    mem.fail_next_write();
    let err = mem.before_write(root).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    mem.before_write(root).unwrap();
}

#[test]
fn mem_writer_hold_blocks_other_writers_until_dropped() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽");
    let uri = uri_of(mem.locate(root, Access::Create).unwrap());
    let conn = attach(&uri);
    conn.execute_batch("CREATE TABLE layer.kv(k TEXT)").unwrap();

    let hold = mem.hold_writer(root);
    let busy = conn.execute_batch("BEGIN IMMEDIATE");
    assert!(matches!(
        busy,
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::DatabaseBusy
    ));
    drop(hold);
    conn.execute_batch("BEGIN IMMEDIATE; INSERT INTO layer.kv VALUES('a'); COMMIT")
        .unwrap();
}

#[test]
fn mem_mark_unusable_gives_a_foreign_application_id() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽");
    mem.mark_unusable(root);
    let uri = uri_of(mem.locate(root, Access::ReadWrite).unwrap());
    let id: i64 = attach(&uri)
        .query_row("PRAGMA layer.application_id", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id, i64::from(UNUSABLE_APPLICATION_ID));
}

#[test]
fn mem_legacy_is_planted_read_and_retired_without_overwrite() {
    let mem = MemBackend::new();
    let root = Path::new(r"C:\偽");
    assert_eq!(mem.read_legacy(root).unwrap(), None);
    assert!(mem.legacy_names(root).is_empty());

    mem.plant_legacy(root, "first");
    mem.retire_legacy(root).unwrap();
    mem.plant_legacy(root, "second");
    assert_eq!(mem.read_legacy(root).unwrap().as_deref(), Some("second"));
    mem.retire_legacy(root).unwrap();

    assert_eq!(mem.read_legacy(root).unwrap(), None);
    assert_eq!(
        mem.legacy_names(root),
        vec!["sylphya.toml.imported", "sylphya.toml.imported.1"]
    );
}

#[test]
fn volatile_and_held_dbs_live_while_the_backend_lives() {
    for backend in [
        FsBackend::with_manual_clock() as Arc<dyn StoreBackend>,
        MemBackend::new(),
    ] {
        let app = Path::new(r"C:\アプリ");
        let uri = backend.volatile_uri(Some(app));
        assert_eq!(uri, backend.volatile_uri(Some(app)));
        assert_ne!(uri, backend.volatile_uri(None));
        assert_ne!(uri, backend.held_uri());

        attach(&uri)
            .execute_batch("CREATE TABLE layer.kv(k TEXT); INSERT INTO layer.kv VALUES('x');")
            .unwrap();
        let n: i64 = attach(&uri)
            .query_row("SELECT count(*) FROM layer.kv", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}

#[test]
fn separate_backends_do_not_share_volatile_dbs() {
    let a = MemBackend::new();
    let b = MemBackend::new();
    assert_ne!(a.volatile_uri(None), b.volatile_uri(None));
    assert_ne!(a.held_uri(), b.held_uri());
}
