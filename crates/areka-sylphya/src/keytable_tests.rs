use super::*;
use crate::actor::SetClass;
use crate::persist::{Axis, PersistKey};

const FIXED_WRITER_NAMES: &[&str] = &[
    "areka.window.scope(0).x",
    "areka.window.scope(3).y",
    "areka.balloon.offset.scope(0).x",
    "areka.balloon.offset.scope(1).y",
    "areka.window.scope(2).adjacent",
    "areka.boot.count",
    "areka.vanish.count",
    "areka.last.ghost",
    "areka.last.balloon",
    "areka.last.shell",
    "areka.last.running",
    "areka.baseware.roots",
    "currentghost.scope(0).scaling",
    "currentghost.balloon.scope(1).scaling",
    "currentghost.scope(0).currentmonitor.dpi",
    "currentghost.balloon.scope(1).currentmonitor.dpi",
];

fn every_persist_key() -> Vec<PersistKey> {
    // Exhaustive match: a new variant does not compile until it is listed here.
    let witness = |key: PersistKey| match key {
        PersistKey::WindowPos { .. }
        | PersistKey::BalloonOffset { .. }
        | PersistKey::BootCount
        | PersistKey::VanishCount
        | PersistKey::LastGhost
        | PersistKey::LastBalloon
        | PersistKey::LastShell
        | PersistKey::LastRunning
        | PersistKey::BasewareRoots
        | PersistKey::WindowAdjacent { .. } => key,
    };
    let mut keys = Vec::new();
    for scope in [0, 1, 7] {
        for axis in [Axis::X, Axis::Y] {
            keys.push(witness(PersistKey::WindowPos { scope, axis }));
            keys.push(witness(PersistKey::BalloonOffset { scope, axis }));
        }
        keys.push(witness(PersistKey::WindowAdjacent { scope }));
    }
    keys.extend(
        [
            PersistKey::BootCount,
            PersistKey::VanishCount,
            PersistKey::LastGhost,
            PersistKey::LastBalloon,
            PersistKey::LastShell,
            PersistKey::LastRunning,
            PersistKey::BasewareRoots,
        ]
        .map(witness),
    );
    keys
}

#[test]
fn production_table_validates() {
    assert_eq!(validate(KEY_TABLE), Ok(()));
}

#[test]
fn production_table_has_12_file_rows_and_4_fast_rows() {
    let shapes = |home| {
        KEY_TABLE
            .iter()
            .filter(|r| matches!(r.matches, KeyMatch::Shape(_)) && r.home == Some(home))
            .count()
    };
    assert_eq!(shapes(Home::File), 12);
    assert_eq!(shapes(Home::Fast), 4);
}

#[test]
fn every_persist_key_canonical_name_hits_a_file_row() {
    for key in every_persist_key() {
        let name = key.to_canonical_key();
        let row = rule_of(KEY_TABLE, &name);
        assert!(
            matches!(row.matches, KeyMatch::Shape(s) if s.starts_with("areka.")),
            "{name} が名前の形の行に当たらない: {row:?}"
        );
        assert_eq!(row.home, Some(Home::File), "{name}");
        assert_eq!(
            row.by_name,
            ByName::Refused(Refusal::FixedWriterOnly),
            "{name}"
        );
        assert_eq!(row.default, None, "{name}");
    }
}

#[test]
fn rows_resolve_by_kind() {
    let row = rule_of(KEY_TABLE, "Surface.Num");
    assert_eq!(row.matches, KeyMatch::SetEffective);
    assert_eq!(row.home, None);
    assert_eq!(row.by_name, ByName::Refused(Refusal::RuntimeCommand));

    let row = rule_of(KEY_TABLE, "CurrentGhost.Scope(5).Scaling");
    assert_eq!(row.home, Some(Home::Fast));
    assert_eq!(row.by_name, ByName::Refused(Refusal::FixedWriterOnly));

    let row = rule_of(KEY_TABLE, "system.foo");
    assert_eq!(row.matches, KeyMatch::Root);
    assert_eq!(row.home, Some(Home::Memory));
    assert_eq!(row.by_name, ByName::Refused(Refusal::NotSettable));

    let row = rule_of(KEY_TABLE, "myplugin.name");
    assert_eq!(row.matches, KeyMatch::Leaf);
    assert_eq!(row.home, Some(Home::Memory));
    assert_eq!(row.by_name, ByName::Refused(Refusal::NotSettable));

    for free in [
        "myplugin.customstate",
        "areka.only_a",
        "areka.window.scope(0).z",
        "a..b",
        "",
    ] {
        let row = rule_of(KEY_TABLE, free);
        assert_eq!(row.matches, KeyMatch::Free, "{free}");
        assert_eq!(row.home, Some(Home::Memory), "{free}");
        assert_eq!(row.by_name, ByName::Writable, "{free}");
        assert_eq!(row.default, None, "{free}");
    }
}

#[test]
fn set_effective_wins_over_leaf_regardless_of_row_order() {
    static REVERSED: &[KeyRow] = &[
        KeyRow {
            matches: KeyMatch::Free,
            home: Some(Home::Memory),
            by_name: ByName::Writable,
            default: None,
        },
        KeyRow {
            matches: KeyMatch::Leaf,
            home: Some(Home::Memory),
            by_name: ByName::Refused(Refusal::NotSettable),
            default: None,
        },
        KeyRow {
            matches: KeyMatch::SetEffective,
            home: None,
            by_name: ByName::Refused(Refusal::RuntimeCommand),
            default: None,
        },
    ];
    assert_eq!(validate(REVERSED), Ok(()));
    assert_eq!(rule_of(REVERSED, "menu").matches, KeyMatch::SetEffective);
}

#[test]
fn validate_rejects_two_rows_of_the_same_shape() {
    let row = |shape| KeyRow {
        matches: KeyMatch::Shape(shape),
        home: Some(Home::File),
        by_name: ByName::Refused(Refusal::FixedWriterOnly),
        default: None,
    };
    let free = KeyRow {
        matches: KeyMatch::Free,
        home: Some(Home::Memory),
        by_name: ByName::Writable,
        default: None,
    };
    let dup = [row("test.scope(*).x"), free, row("test.scope(*).x")];
    assert!(validate(&dup).is_err());
    let dup_by_spelling = [row("test.scope(*).x"), free, row("TEST.Scope(*).x")];
    assert!(validate(&dup_by_spelling).is_err());
    let distinct = [row("test.scope(*).x"), free, row("test.scope(*).y")];
    assert_eq!(validate(&distinct), Ok(()));
}

#[test]
fn validate_rejects_unparseable_shape_and_missing_free_row() {
    let bad = [
        KeyRow {
            matches: KeyMatch::Shape("a..b"),
            home: Some(Home::File),
            by_name: ByName::Refused(Refusal::FixedWriterOnly),
            default: None,
        },
        KeyRow {
            matches: KeyMatch::Free,
            home: Some(Home::Memory),
            by_name: ByName::Writable,
            default: None,
        },
    ];
    assert!(validate(&bad).is_err());
    assert!(validate(&bad[1..]).is_ok());
    assert!(validate(&[]).is_err());
}

#[test]
fn injected_table_with_an_added_row_resolves_it() {
    static WITH_EXTRA: &[KeyRow] = &[
        KeyRow {
            matches: KeyMatch::Shape("test.extra.scope(*).flag"),
            home: Some(Home::File),
            by_name: ByName::Refused(Refusal::FixedWriterOnly),
            default: Some("0"),
        },
        KeyRow {
            matches: KeyMatch::Free,
            home: Some(Home::Memory),
            by_name: ByName::Writable,
            default: None,
        },
    ];
    assert_eq!(validate(WITH_EXTRA), Ok(()));
    let row = rule_of(WITH_EXTRA, "test.extra.scope(4).flag");
    assert_eq!(row.home, Some(Home::File));
    assert_eq!(row.default, Some("0"));
    assert_eq!(
        rule_of(WITH_EXTRA, "test.extra.flag").matches,
        KeyMatch::Free
    );
}

#[test]
fn classify_set_answers_not_settable_for_the_16_fixed_writer_names() {
    for name in FIXED_WRITER_NAMES {
        assert_eq!(classify_set(name), SetClass::NotSettable, "{name}");
        assert_eq!(
            classify_set(&name.to_ascii_uppercase()),
            SetClass::NotSettable,
            "{name}"
        );
    }
}

#[test]
fn classify_set_keeps_its_answers_for_other_names() {
    let expected: &[(&str, SetClass)] = &[
        ("surface.num", SetClass::RuntimeCommand),
        ("MENU", SetClass::RuntimeCommand),
        ("sakura.bind.menu", SetClass::RuntimeCommand),
        ("baseware.name", SetClass::NotSettable),
        ("CURRENTGHOST.NAME", SetClass::NotSettable),
        ("system.foo", SetClass::NotSettable),
        ("username", SetClass::NotSettable),
        ("myplugin.name", SetClass::NotSettable),
        ("myplugin.customstate", SetClass::StoreWrite),
        ("My.Flag", SetClass::StoreWrite),
        ("areka.only_a", SetClass::StoreWrite),
        ("areka.window.scope(0).z", SetClass::StoreWrite),
        ("a..b", SetClass::StoreWrite),
        ("", SetClass::StoreWrite),
    ];
    for (key, class) in expected {
        assert_eq!(classify_set(key), *class, "{key}");
    }
}
