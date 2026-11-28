use super::*;
use crate::persist::ScopeRoots;
use crate::store::OP_WAIT;
use crate::store::schema::APPLICATION_ID;
use crate::store::{Access, FsBackend, Located, MemBackend, StoreBackend};
use crate::test_log_capture::capture;
use log_capture_kit::CapturedEvent;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use temp_path_kit::TempPath;
use tracing::Level;

const ORDER: [PersistScope; 4] = [
    PersistScope::Balloon,
    PersistScope::Shell,
    PersistScope::Ghost,
    PersistScope::App,
];

fn asker() -> AskerId {
    AskerId::new("ghost-a")
}

fn fake_roots(tag: &str) -> ScopeRoots {
    let root = |layer: &str| Some(PathBuf::from(format!(r"C:\fake\{tag}\{layer}")));
    ScopeRoots {
        app: root("app"),
        ghost: root("ghost"),
        shell: root("shell"),
        balloon: root("balloon"),
    }
}

fn root_of(roots: &ScopeRoots, layer: PersistScope) -> &Path {
    match layer {
        PersistScope::App => roots.app.as_deref(),
        PersistScope::Ghost => roots.ghost.as_deref(),
        PersistScope::Shell => roots.shell.as_deref(),
        PersistScope::Balloon => roots.balloon.as_deref(),
    }
    .expect("試験の層には置き場が在る")
}

fn layer_conn(backend: &dyn StoreBackend, root: &Path) -> Connection {
    let Located::Uri(uri) = backend.locate(root, Access::Create).unwrap() else {
        panic!("作る開き方では在りかが返るはず");
    };
    let conn = Connection::open(uri).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA application_id = {APPLICATION_ID};
         PRAGMA user_version = 1;
         CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;"
    ))
    .unwrap();
    conn
}

fn plant_file(backend: &dyn StoreBackend, root: &Path, key: &str, value: &str) {
    layer_conn(backend, root)
        .execute("INSERT OR REPLACE INTO kv VALUES (?1, ?2)", [key, value])
        .unwrap();
}

fn unplant_file(backend: &dyn StoreBackend, root: &Path, key: &str) {
    layer_conn(backend, root)
        .execute("DELETE FROM kv WHERE key = ?1", [key])
        .unwrap();
}

fn plant_memory(store: &Store, layer: PersistScope, asker: &AskerId, key: &str, value: &str) {
    match layer {
        PersistScope::App => store.conn.execute(
            "INSERT OR REPLACE INTO appvol.kv VALUES (?1, ?2)",
            [key, value],
        ),
        _ => store.conn.execute(
            "INSERT OR REPLACE INTO main.vol VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![layer_order(layer), asker.as_str(), key, value],
        ),
    }
    .unwrap();
}

fn unplant_memory(store: &Store, layer: PersistScope, asker: &AskerId, key: &str) {
    match layer {
        PersistScope::App => store
            .conn
            .execute("DELETE FROM appvol.kv WHERE key = ?1", [key]),
        _ => store.conn.execute(
            "DELETE FROM main.vol WHERE layer = ?1 AND asker = ?2 AND key = ?3",
            rusqlite::params![layer_order(layer), asker.as_str(), key],
        ),
    }
    .unwrap();
}

fn value_of(layer: PersistScope) -> String {
    format!("{layer:?}")
}

fn present(mask: u8) -> Vec<PersistScope> {
    ORDER
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, l)| *l)
        .collect()
}

fn attached_layers(store: &Store) -> Vec<String> {
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

fn read_busy_errors(log: &[CapturedEvent]) -> usize {
    log.iter()
        .filter(|e| e.level == Level::ERROR && e.field_str("event") == Some("persist_read_busy"))
        .count()
}

/// 値の在る層の組 `mask` で、重ね読み・名指しの読みを確かめ、上の層から消して次の層へ進むのを見る。
fn check_combination(
    mask: u8,
    home: Home,
    store: &mut Store,
    plant: &mut dyn FnMut(&Store, PersistScope, &str),
    unplant: &mut dyn FnMut(&Store, PersistScope),
) {
    const KEY: &str = "areka.window.scope(0).x";
    let layers = present(mask);
    for &layer in &layers {
        plant(store, layer, &value_of(layer));
    }
    for &layer in &ORDER {
        let expected = layers.contains(&layer).then(|| value_of(layer));
        assert_eq!(
            store.read(&asker(), KEY, Some(layer), home),
            expected,
            "組 {mask:04b} の {layer:?} を名指しした読み"
        );
    }
    let mut remaining = layers.clone();
    loop {
        let expected = remaining.first().map(|l| value_of(*l));
        assert_eq!(
            store.read(&asker(), KEY, None, home),
            expected,
            "組 {mask:04b} の残り {remaining:?} の重ね読み"
        );
        assert!(attached_layers(store).is_empty());
        if remaining.is_empty() {
            break;
        }
        unplant(store, remaining.remove(0));
    }
}

#[test]
fn file_keys_overlay_in_all_fifteen_combinations() {
    for mask in 1..16u8 {
        let mem = MemBackend::new();
        let roots = fake_roots(&format!("file-{mask}"));
        let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
        check_combination(
            mask,
            Home::File,
            &mut store,
            &mut |_, layer, value| {
                plant_file(
                    mem.as_ref(),
                    root_of(&roots, layer),
                    "areka.window.scope(0).x",
                    value,
                )
            },
            &mut |_, layer| {
                unplant_file(
                    mem.as_ref(),
                    root_of(&roots, layer),
                    "areka.window.scope(0).x",
                )
            },
        );
    }
}

#[test]
fn memory_keys_overlay_in_all_fifteen_combinations() {
    for mask in 1..16u8 {
        let mem = MemBackend::new();
        let mut store = Store::open(fake_roots(&format!("mem-{mask}")), mem.clone()).unwrap();
        check_combination(
            mask,
            Home::Memory,
            &mut store,
            &mut |store, layer, value| {
                plant_memory(store, layer, &asker(), "areka.window.scope(0).x", value)
            },
            &mut |store, layer| unplant_memory(store, layer, &asker(), "areka.window.scope(0).x"),
        );
        let roots = fake_roots(&format!("mem-{mask}"));
        for layer in ORDER {
            assert_eq!(
                mem.locate(root_of(&roots, layer), Access::ReadWrite)
                    .unwrap(),
                Located::Missing,
                "メモリーの値は層のファイルを作らない"
            );
        }
    }
}

#[test]
fn layer_without_root_is_skipped() {
    let mem = MemBackend::new();
    let full = fake_roots("rootless");
    plant_file(
        mem.as_ref(),
        root_of(&full, PersistScope::Ghost),
        "k",
        "ghost",
    );
    plant_file(mem.as_ref(), root_of(&full, PersistScope::App), "k", "app");
    let roots = ScopeRoots {
        ghost: None,
        ..full
    };
    let mut store = Store::open(roots, mem.clone()).unwrap();
    assert_eq!(
        store.read(&asker(), "k", None, Home::File),
        Some("app".to_owned())
    );
    assert_eq!(
        store.read(&asker(), "k", Some(PersistScope::Ghost), Home::File),
        None
    );
}

#[test]
fn empty_string_is_a_value_not_absence() {
    let mem = MemBackend::new();
    let roots = fake_roots("empty");
    plant_file(
        mem.as_ref(),
        root_of(&roots, PersistScope::Balloon),
        "k",
        "",
    );
    plant_file(mem.as_ref(), root_of(&roots, PersistScope::Ghost), "k", "x");
    let mut store = Store::open(roots, mem.clone()).unwrap();
    plant_memory(&store, PersistScope::Shell, &asker(), "k", "");
    plant_memory(&store, PersistScope::App, &asker(), "k", "x");
    assert_eq!(
        store.read(&asker(), "k", None, Home::File),
        Some(String::new())
    );
    assert_eq!(
        store.read(&asker(), "k", None, Home::Memory),
        Some(String::new())
    );
}

#[test]
fn names_are_folded_to_the_canonical_key() {
    let mem = MemBackend::new();
    let roots = fake_roots("fold");
    plant_file(
        mem.as_ref(),
        root_of(&roots, PersistScope::Ghost),
        "areka.window.scope(0).x",
        "10",
    );
    let mut store = Store::open(roots, mem.clone()).unwrap();
    plant_memory(&store, PersistScope::Ghost, &asker(), "foo.bar(ab)", "v");
    assert_eq!(
        store.read(&asker(), "AREKA.Window.Scope(0).X", None, Home::File),
        Some("10".to_owned())
    );
    assert_eq!(
        store.read(&asker(), "Foo.BAR(Ab)", None, Home::Memory),
        Some("v".to_owned())
    );
    assert_eq!(
        store.read(&asker(), "ｆoo.bar(ab)", None, Home::Memory),
        None
    );
}

#[test]
fn unresolvable_name_warns_and_reads_no_value() {
    let mem = MemBackend::new();
    let roots = fake_roots("unresolvable");
    plant_file(
        mem.as_ref(),
        root_of(&roots, PersistScope::Ghost),
        "a..b",
        "x",
    );
    let mut store = Store::open(roots, mem.clone()).unwrap();
    plant_memory(&store, PersistScope::Ghost, &asker(), "a..b", "x");
    let mut answers = Vec::new();
    let log = capture(|| {
        answers.push(store.read(&asker(), "a..b", None, Home::File));
        answers.push(store.read(&asker(), "a..b", None, Home::Memory));
    });
    assert_eq!(answers, vec![None, None]);
    let warns = log
        .iter()
        .filter(|e| e.level == Level::WARN && e.target == "areka_sylphya::store")
        .count();
    assert_eq!(warns, 2);
}

#[test]
fn ghost_layer_volatile_values_belong_to_the_asker_and_app_values_are_shared() {
    let mem = MemBackend::new();
    let roots = fake_roots("asker");
    let mut first = Store::open(roots.clone(), mem.clone()).unwrap();
    let mut second = Store::open(roots, mem.clone()).unwrap();
    let other = AskerId::new("ghost-b");
    plant_memory(&first, PersistScope::Ghost, &asker(), "free", "mine");
    plant_memory(&first, PersistScope::App, &asker(), "shared", "all");
    assert_eq!(
        first.read(&asker(), "free", None, Home::Memory),
        Some("mine".to_owned())
    );
    assert_eq!(first.read(&other, "free", None, Home::Memory), None);
    assert_eq!(second.read(&asker(), "free", None, Home::Memory), None);
    for store in [&mut first, &mut second] {
        assert_eq!(
            store.read(&other, "shared", None, Home::Memory),
            Some("all".to_owned())
        );
    }
}

#[test]
fn value_written_by_another_connection_is_seen_on_the_next_read() {
    let mem = MemBackend::new();
    let roots = fake_roots("other-conn");
    let ghost = root_of(&roots, PersistScope::Ghost).to_path_buf();
    let mut store = Store::open(roots, mem.clone()).unwrap();
    assert_eq!(store.read(&asker(), "k", None, Home::File), None);
    plant_file(mem.as_ref(), &ghost, "k", "1");
    assert_eq!(
        store.read(&asker(), "k", None, Home::File),
        Some("1".to_owned())
    );
    plant_file(mem.as_ref(), &ghost, "k", "2");
    assert_eq!(
        store.read(&asker(), "k", None, Home::File),
        Some("2".to_owned())
    );
}

#[test]
fn real_file_written_by_another_connection_is_seen_and_not_held_open() {
    let dir = TempPath::under_target("sylphya-store-layered-read");
    let ghost = dir.child("ghost");
    std::fs::create_dir_all(&ghost).unwrap();
    let fs: Arc<FsBackend> = FsBackend::with_manual_clock();
    let roots = ScopeRoots {
        ghost: Some(ghost.clone()),
        ..ScopeRoots::default()
    };
    let mut store = Store::open(roots, fs.clone()).unwrap();
    assert_eq!(store.read(&asker(), "k", None, Home::File), None);
    assert!(
        !ghost.join("sylphya.sqlite").exists(),
        "読みはファイルを作らない"
    );
    plant_file(fs.as_ref(), &ghost, "k", "written");
    assert_eq!(
        store.read(&asker(), "k", None, Home::File),
        Some("written".to_owned())
    );
    let moved = dir.child("ghost-moved");
    std::fs::rename(&ghost, &moved).expect("読みの後は層のフォルダを掴んでいない");
}

#[test]
fn read_that_meets_a_lock_retries_then_gives_up_after_the_wait() {
    let mem = MemBackend::new();
    let roots = fake_roots("busy");
    let ghost = root_of(&roots, PersistScope::Ghost).to_path_buf();
    plant_file(mem.as_ref(), &ghost, "k", "v");
    let mut store = Store::open(roots, mem.clone()).unwrap();
    let locker = layer_conn(mem.as_ref(), &ghost);
    locker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let start = mem.now();
    let mut answer = Some(String::new());
    let log = capture(|| answer = store.read(&asker(), "k", None, Home::File));
    assert_eq!(answer, None);
    assert_eq!(read_busy_errors(&log), 1);
    assert!(
        mem.now() - start > OP_WAIT,
        "待ちは置き場の口の時計で 1 秒を越える"
    );
    assert!(attached_layers(&store).is_empty());

    locker.execute_batch("COMMIT").unwrap();
    assert_eq!(
        store.read(&asker(), "k", None, Home::File),
        Some("v".to_owned())
    );
}

#[test]
fn open_that_meets_the_app_volatile_lock_retries_until_the_wait() {
    let mem = MemBackend::new();
    let roots = fake_roots("open-busy");
    let first = Store::open(roots.clone(), mem.clone()).unwrap();
    let locker = Connection::open(mem.volatile_uri(roots.app.as_deref())).unwrap();
    locker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let start = mem.now();
    assert!(Store::open(roots.clone(), mem.clone()).is_err());
    assert!(
        mem.now() - start > OP_WAIT,
        "開くのも置き場の口の時計で 1 秒を越えるまで取り直す"
    );

    locker.execute_batch("COMMIT").unwrap();
    assert!(Store::open(roots, mem.clone()).is_ok());
    drop(first);
}

#[test]
fn read_layer_returns_every_pair_of_that_layer_only() {
    let mem = MemBackend::new();
    let roots = fake_roots("whole");
    let ghost = root_of(&roots, PersistScope::Ghost);
    plant_file(mem.as_ref(), ghost, "b", "2");
    plant_file(mem.as_ref(), ghost, "a", "");
    plant_file(mem.as_ref(), root_of(&roots, PersistScope::App), "c", "3");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    assert_eq!(
        store.read_layer(PersistScope::Ghost),
        vec![
            ("a".to_owned(), String::new()),
            ("b".to_owned(), "2".to_owned())
        ]
    );
    assert!(store.read_layer(PersistScope::Shell).is_empty());
    assert!(attached_layers(&store).is_empty());
}
