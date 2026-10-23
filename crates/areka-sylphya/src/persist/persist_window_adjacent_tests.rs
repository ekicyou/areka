//! 隣り合いの印（`PersistKey::WindowAdjacent`・表 `[window-adjacent]`）の檻
//! （spec dpi-realign-remembered-chain 要件 6.2）。
//!
//! - 正準 key `areka.window.scope(ID).adjacent` が `parse_dotted` → `to_canonical_string` で往復する。
//! - 書く → 読むで値が戻る・表 `[window-adjacent]` に載る・書き出しの版は 1 のまま。
//! - マージで他の鍵が残る（位置 → 印・印 → 位置の両方の順）。
//! - 文字列でない値・数でない ID は無しとして読む（寛容）。表の無い古いファイルも読める。

use super::*;
use crate::key::parse_dotted;

fn roots() -> ScopeRoots {
    ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    }
}

fn written(io: &FakePersistIo) -> String {
    io.read(Path::new("/g/sylphya.toml"))
        .unwrap()
        .expect("書いたファイルがある")
}

fn window_x(scope: u32) -> PersistKey {
    PersistKey::WindowPos {
        scope,
        axis: Axis::X,
    }
}

#[test]
fn adjacent_canonical_key_exact_and_round_trips() {
    for scope in [0, 1, 7] {
        let s = PersistKey::WindowAdjacent { scope }.to_canonical_key();
        assert_eq!(s, format!("areka.window.scope({scope}).adjacent"));
        assert_eq!(parse_dotted(&s).unwrap().to_canonical_string(), s);
    }
}

#[test]
fn adjacent_save_load_round_trip_under_its_table_with_version_1() {
    let io = FakePersistIo::new();
    let entries = vec![
        (PersistKey::WindowAdjacent { scope: 1 }, "left".to_string()),
        (PersistKey::WindowAdjacent { scope: 2 }, "right".to_string()),
        (PersistKey::WindowAdjacent { scope: 3 }, "none".to_string()),
    ];
    assert_eq!(
        save_scope(PersistScope::Ghost, &roots(), &io, entries.clone()),
        PersistOutcome::Saved
    );
    let content = written(&io);
    assert!(content.contains("format-version = 1"), "{content}");
    assert!(content.contains("[window-adjacent]"), "{content}");
    assert!(content.contains("1 = \"left\""), "{content}");
    assert_eq!(load_scope(PersistScope::Ghost, &roots(), &io), entries);
}

#[test]
fn merge_keeps_other_keys_both_orders() {
    // 位置 → 印
    let io = FakePersistIo::new();
    save_scope(
        PersistScope::Ghost,
        &roots(),
        &io,
        vec![(window_x(1), "1032".into())],
    );
    save_scope(
        PersistScope::Ghost,
        &roots(),
        &io,
        vec![(PersistKey::WindowAdjacent { scope: 1 }, "left".into())],
    );
    let loaded = load_scope(PersistScope::Ghost, &roots(), &io);
    assert!(loaded.contains(&(window_x(1), "1032".into())), "{loaded:?}");
    assert!(
        loaded.contains(&(PersistKey::WindowAdjacent { scope: 1 }, "left".into())),
        "{loaded:?}"
    );

    // 印 → 位置・起動記録（印が消えない）
    let io = FakePersistIo::new();
    save_scope(
        PersistScope::Ghost,
        &roots(),
        &io,
        vec![(PersistKey::WindowAdjacent { scope: 2 }, "right".into())],
    );
    save_scope(
        PersistScope::Ghost,
        &roots(),
        &io,
        vec![
            (window_x(2), "500".into()),
            (PersistKey::BootCount, "4".into()),
        ],
    );
    let loaded = load_scope(PersistScope::Ghost, &roots(), &io);
    assert!(
        loaded.contains(&(PersistKey::WindowAdjacent { scope: 2 }, "right".into())),
        "{loaded:?}"
    );
    assert!(loaded.contains(&(window_x(2), "500".into())), "{loaded:?}");
    assert!(
        loaded.contains(&(PersistKey::BootCount, "4".into())),
        "{loaded:?}"
    );
}

#[test]
fn non_string_value_and_non_numeric_id_are_absent() {
    let io = FakePersistIo::new();
    io.commit(
        Path::new("/g/sylphya.toml"),
        "format-version = 1\n[window-adjacent]\n\"1\" = 3\n\"2\" = \"right\"\nabc = \"left\"\n\"3\" = { x = \"1\" }\n",
    )
    .unwrap();
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots(), &io),
        vec![(PersistKey::WindowAdjacent { scope: 2 }, "right".to_string())]
    );
}

#[test]
fn old_file_without_table_still_reads() {
    let io = FakePersistIo::new();
    io.commit(
        Path::new("/g/sylphya.toml"),
        "format-version = 1\n[window.\"1\"]\nx = \"1032\"\n",
    )
    .unwrap();
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots(), &io),
        vec![(window_x(1), "1032".to_string())]
    );
}

#[test]
fn adjacent_only_doc_is_not_all_absent() {
    let mut doc = FormatDoc::default();
    doc.window_adjacent.insert("1".into(), "left".into());
    assert!(!doc.is_all_absent());
}
