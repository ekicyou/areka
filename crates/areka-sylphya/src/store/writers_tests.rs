use super::tests::{LAYERS, asker, attached_layers, fake_roots, put, saved};
use super::*;
use crate::keytable::Home;
use crate::store::{Access, Located, MemBackend, StoreBackend};
use std::path::Path;

/// `store` の接続で層のファイルを付けて書きの錠を取り、確定せずに返す（書きの途中で止めた核）。
fn begin_holding(store: &Store, backend: &dyn StoreBackend, root: &Path, key: &str, value: &str) {
    let Located::Uri(uri) = backend.locate(root, Access::ReadWrite).unwrap() else {
        panic!("書いた後の層は在るはず");
    };
    store.conn.execute("ATTACH ?1 AS ghost", [uri]).unwrap();
    store.conn.execute_batch("BEGIN").unwrap();
    store
        .conn
        .execute(
            "INSERT INTO ghost.kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )
        .unwrap();
}

fn finish_holding(store: &Store) {
    store.conn.execute_batch("COMMIT; DETACH ghost").unwrap();
}

#[test]
fn three_cores_step_through_one_lock_and_every_saved_value_is_read() {
    let mem = MemBackend::new();
    let roots = fake_roots("three-cores");
    let ghost = roots.ghost.clone().unwrap();
    let mut cores: Vec<Store> = (0..3)
        .map(|_| Store::open(roots.clone(), mem.clone()).unwrap())
        .collect();
    let layer = PersistScope::Ghost;

    saved(cores[0].try_write(&put(&roots, layer, &[("own.0", "a"), ("same", "0")])));

    let writes = [
        put(&roots, layer, &[("own.1", "b"), ("same", "1")]),
        put(&roots, layer, &[("own.2", "c"), ("same", "2")]),
    ];
    begin_holding(&cores[0], mem.as_ref(), &ghost, "same", "held-by-0");
    for (core, write) in cores[1..].iter_mut().zip(&writes) {
        let attempt = core.try_write(write);
        assert!(matches!(attempt, WriteAttempt::Busy), "{attempt:?}");
        assert!(attached_layers(core).is_empty());
    }
    finish_holding(&cores[0]);
    assert_eq!(
        cores[1].read(&asker(), "same", None, Home::File),
        Some("held-by-0".to_owned())
    );

    for (core, write) in cores[1..].iter_mut().zip(&writes) {
        saved(core.try_write(write));
    }

    for core in &mut cores {
        assert_eq!(
            core.read_layer(layer),
            vec![
                ("own.0".to_owned(), "a".to_owned()),
                ("own.1".to_owned(), "b".to_owned()),
                ("own.2".to_owned(), "c".to_owned()),
                ("same".to_owned(), "2".to_owned()),
            ],
            "別々の鍵は全部・同じ鍵は最後に確定した値"
        );
        assert_eq!(
            core.read(&asker(), "same", None, Home::File),
            Some("2".to_owned())
        );
    }
}

#[test]
fn two_keys_of_one_write_land_together_or_not_at_all() {
    let mem = MemBackend::new();
    let roots = fake_roots("atomic");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    let layer = PersistScope::Ghost;
    saved(store.try_write(&put(&roots, layer, &[("x", "1"), ("y", "1")])));

    mem.fail_next_write();
    let attempt = store.try_write(&put(&roots, layer, &[("x", "2"), ("y", "2")]));
    assert!(
        matches!(attempt, WriteAttempt::Done(PersistOutcome::Degraded(_))),
        "{attempt:?}"
    );
    assert_eq!(
        store.read_layer(layer),
        vec![
            ("x".to_owned(), "1".to_owned()),
            ("y".to_owned(), "1".to_owned())
        ],
        "1 つも入らない"
    );

    saved(store.try_write(&put(&roots, layer, &[("x", "3"), ("y", "3")])));
    assert_eq!(
        store.read_layer(layer),
        vec![
            ("x".to_owned(), "3".to_owned()),
            ("y".to_owned(), "3".to_owned())
        ],
        "両方入る"
    );
}

#[test]
fn a_write_leaves_the_other_layers_files_unchanged() {
    let mem = MemBackend::new();
    let roots = fake_roots("other-layers");
    let mut store = Store::open(roots.clone(), mem.clone()).unwrap();
    for layer in LAYERS {
        saved(store.try_write(&put(&roots, layer, &[("k", &format!("{layer:?}"))])));
    }
    let before: Vec<_> = LAYERS.iter().map(|&l| store.read_layer(l)).collect();
    saved(store.try_write(&put(
        &roots,
        PersistScope::Ghost,
        &[("k", "new"), ("j", "1")],
    )));
    for (i, &layer) in LAYERS.iter().enumerate() {
        if layer != PersistScope::Ghost {
            assert_eq!(store.read_layer(layer), before[i], "{layer:?} は変わらない");
        }
    }

    let fresh = fake_roots("other-layers-fresh");
    let mut store = Store::open(fresh.clone(), mem.clone()).unwrap();
    saved(store.try_write(&put(&fresh, PersistScope::Ghost, &[("k", "v")])));
    for layer in [
        PersistScope::App,
        PersistScope::Shell,
        PersistScope::Balloon,
    ] {
        assert_eq!(
            mem.locate(fresh.root_of(layer).unwrap(), Access::ReadWrite)
                .unwrap(),
            Located::Missing,
            "{layer:?} のファイルは作らない"
        );
    }
}
