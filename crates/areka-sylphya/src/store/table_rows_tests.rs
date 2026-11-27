use super::tests::{asker, fake_roots, put, remove, saved};
use super::*;
use crate::keytable::{ByName, Home, KEY_TABLE, KeyMatch, KeyRow, Refusal, rule_of, validate};
use crate::store::MemBackend;

/// 行が当たる見本の鍵（速い住み場所とどこにも書かない行は `None`）。
fn sample_key(row: &KeyRow) -> Option<String> {
    if !matches!(row.home, Some(Home::File | Home::Memory)) {
        return None;
    }
    Some(match row.matches {
        KeyMatch::Shape(shape) => shape.replace("(*)", "(3)"),
        KeyMatch::Root => "system.sample".to_owned(),
        KeyMatch::Leaf => "sample.name".to_owned(),
        KeyMatch::Free => "sample.free".to_owned(),
        KeyMatch::SetEffective => return None,
    })
}

fn write(
    store: &mut Store,
    roots: &ScopeRoots,
    home: Home,
    layer: PersistScope,
    key: &str,
    value: Option<&str>,
) {
    match (home, value) {
        (Home::File, Some(v)) => saved(store.try_write(&put(roots, layer, &[(key, v)]))),
        (Home::File, None) => saved(store.try_write(&remove(roots, layer, &[key]))),
        _ => store.write_volatile(layer, &asker(), key, value).unwrap(),
    }
}

/// 表の行を 1 つずつ: 書く・（ファイルなら）開き直して読む・層を重ねて読む・消す。
fn every_row_writes_reads_overlays_and_removes(table: &'static [KeyRow], tag: &str) -> usize {
    validate(table).unwrap();
    let mut checked = 0;
    for (i, row) in table.iter().enumerate() {
        let Some(key) = sample_key(row) else { continue };
        assert_eq!(rule_of(table, &key), row, "見本の鍵 {key} はこの行に当たる");
        let home = row.home.unwrap();
        let mem = MemBackend::new();
        let roots = fake_roots(&format!("{tag}-{i}"));
        let mut store = Store::open(roots.clone(), mem.clone()).unwrap();

        write(
            &mut store,
            &roots,
            home,
            PersistScope::Ghost,
            &key,
            Some("ghost"),
        );
        if home == Home::File {
            store = Store::open(roots.clone(), mem.clone()).unwrap();
        }
        assert_eq!(
            store.read(&asker(), &key, Some(PersistScope::Ghost), home),
            Some("ghost".to_owned()),
            "{key}"
        );
        write(
            &mut store,
            &roots,
            home,
            PersistScope::Shell,
            &key,
            Some("shell"),
        );
        assert_eq!(
            store.read(&asker(), &key, None, home),
            Some("shell".to_owned()),
            "{key}"
        );
        write(&mut store, &roots, home, PersistScope::Shell, &key, None);
        assert_eq!(
            store.read(&asker(), &key, None, home),
            Some("ghost".to_owned()),
            "{key}"
        );
        write(&mut store, &roots, home, PersistScope::Ghost, &key, None);
        assert_eq!(store.read(&asker(), &key, None, home), None, "{key}");
        checked += 1;
    }
    checked
}

#[test]
fn every_key_table_row_round_trips() {
    let checked = every_row_writes_reads_overlays_and_removes(KEY_TABLE, "rows");
    assert_eq!(checked, 12 + 3, "ファイルの行 12 とメモリーの行 3");
}

#[test]
fn a_table_with_one_added_row_round_trips_the_added_row_too() {
    let mut rows = KEY_TABLE.to_vec();
    rows.insert(
        1,
        KeyRow {
            matches: KeyMatch::Shape("areka.added.scope(*).value"),
            home: Some(Home::File),
            by_name: ByName::Refused(Refusal::FixedWriterOnly),
            default: None,
        },
    );
    let table: &'static [KeyRow] = rows.leak();
    let checked = every_row_writes_reads_overlays_and_removes(table, "added");
    assert_eq!(checked, 12 + 3 + 1);
}
