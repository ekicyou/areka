//! descript の既定の面の読みと戻り先の候補の並びの決定論のテスト（descript-window-keys 要件 7.1）。

use std::collections::BTreeMap;

use super::*;
use crate::placement::test_support::{ExpectField, capture_logs, expect_one};

fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// 書かれていなければ組み込みの既定の面（本体側 0・相方側とスコープ 2 以降 10）で、記録も残さない（要件 1.1・1.2・1.4）。
#[test]
fn unwritten_scopes_use_builtin_default_surfaces_silently() {
    let (ds, events) = capture_logs(|| read_default_surfaces(&kv(&[("name", "x")])));
    assert_eq!(ds, DefaultSurfaces::builtin());
    assert_eq!(ds.surface(0), 0);
    assert_eq!(ds.surface(1), 10);
    assert_eq!(ds.surface(2), 10);
    assert_eq!(ds.surface(7), 10);
    assert!(
        events.is_empty(),
        "書かれていないのは異常ではない: {events:?}"
    );
}

/// `sakura.`・`kero.` に書かれた値を読む。前後の空白は除く（要件 1.1・1.2）。
#[test]
fn written_values_are_read_per_scope() {
    let ds = read_default_surfaces(&kv(&[
        ("sakura.seriko.defaultsurface", "5"),
        ("kero.seriko.defaultsurface", " 12 "),
    ]));
    assert_eq!(ds.surface(0), 5);
    assert_eq!(ds.surface(1), 12);
}

/// 別名 `char0.`・`char1.` は配置のほかのキーと同じ優先（正典の綴り ＞ 別名）で扱う（要件 1.3）。
#[test]
fn aliases_follow_the_same_precedence_as_other_placement_keys() {
    let alias_only = read_default_surfaces(&kv(&[
        ("char0.seriko.defaultsurface", "3"),
        ("char1.seriko.defaultsurface", "13"),
    ]));
    assert_eq!(alias_only.surface(0), 3);
    assert_eq!(alias_only.surface(1), 13);

    let both = read_default_surfaces(&kv(&[
        ("sakura.seriko.defaultsurface", "5"),
        ("char0.seriko.defaultsurface", "3"),
        ("kero.seriko.defaultsurface", "12"),
        ("char1.seriko.defaultsurface", "13"),
    ]));
    assert_eq!(both.surface(0), 5, "sakura. が char0. に勝つ");
    assert_eq!(both.surface(1), 12, "kero. が char1. に勝つ");
}

/// `char2.`・`char7.` だけ書かれていれば、そのスコープだけ読み、ほかは組み込みのまま（要件 1.4）。
#[test]
fn scopes_two_and_up_are_read_only_where_written() {
    let ds = read_default_surfaces(&kv(&[
        ("char2.seriko.defaultsurface", "20"),
        ("char7.seriko.defaultsurface", "70"),
    ]));
    assert_eq!(ds.surface(0), 0);
    assert_eq!(ds.surface(1), 10);
    assert_eq!(ds.surface(2), 20);
    assert_eq!(ds.surface(3), 10);
    assert_eq!(ds.surface(7), 70);
}

/// 読めない値（負・数でない・空・桁あふれ）はそのスコープを組み込みの面にし、スコープ・キー・値の記録を 1 件残す（要件 1.5）。
#[test]
fn unreadable_values_fall_back_to_builtin_with_one_warning_each() {
    for raw in ["-1", "abc", "", "4294967296"] {
        for (scope, key) in [
            (0usize, "sakura.seriko.defaultsurface"),
            (1, "char1.seriko.defaultsurface"),
            (2, "char2.seriko.defaultsurface"),
        ] {
            let (ds, events) = capture_logs(|| read_default_surfaces(&kv(&[(key, raw)])));
            assert_eq!(
                ds.surface(scope),
                builtin_default_surface(scope),
                "raw={raw:?} key={key}"
            );
            assert_eq!(events.len(), 1, "raw={raw:?} key={key}: {events:?}");
            let ev = expect_one(&events, "descript_value_unreadable");
            assert_eq!(ev.level, tracing::Level::WARN);
            assert_eq!(ev.expect_field("scope"), scope.to_string());
            assert_eq!(ev.expect_field("key"), key);
            assert_eq!(ev.expect_field("value"), raw);
        }
    }
}

/// 既定の面はゴーストの descript だけから読む。シェルの KV に同じ名前の鍵があっても使わない（要件 1.6）。
///
/// 読みの関数はシェルの KV を引数に持たない（型で守る）。ここではシェルの KV を渡す道が無いことを、
/// ゴーストの KV だけを渡したときに組み込みの値になることで確かめる。
#[test]
fn shell_kv_with_same_key_is_not_used() {
    let shell_kv = kv(&[
        ("sakura.seriko.defaultsurface", "5"),
        ("kero.seriko.defaultsurface", "12"),
    ]);
    let ghost_kv = kv(&[("name", "x")]);
    let ds = read_default_surfaces(&ghost_kv);
    assert_eq!(ds, DefaultSurfaces::builtin());
    assert_ne!(
        ds.surface(0),
        5,
        "シェルの値 {shell_kv:?} を拾ってはならない"
    );
}

/// 候補の並びは descript の面、違えば続けて組み込みの面で、重ならない（要件 2.6）。
#[test]
fn fallback_candidates_do_not_overlap() {
    assert_eq!(fallback_candidates(0, 0), vec![0]);
    assert_eq!(fallback_candidates(1, 10), vec![10]);
    assert_eq!(fallback_candidates(0, 5), vec![5, 0]);
    assert_eq!(fallback_candidates(1, 12), vec![12, 10]);
    assert_eq!(fallback_candidates(3, 10), vec![10]);
    assert_eq!(fallback_candidates(3, 0), vec![0, 10]);

    let ds = read_default_surfaces(&kv(&[("kero.seriko.defaultsurface", "12")]));
    assert_eq!(ds.candidates(0), vec![0]);
    assert_eq!(ds.candidates(1), vec![12, 10]);
}

/// 前から最初に在る面: 先頭が在る・先頭が無く次が在る・どれも無い の 3 通り。
#[test]
fn first_existing_picks_the_first_present_candidate() {
    let shell = [0u32, 10];
    let exists = |id: u32| shell.contains(&id);
    assert_eq!(first_existing(&[10, 0], exists), Some(10));
    assert_eq!(first_existing(&[12, 10], exists), Some(10));
    assert_eq!(first_existing(&[12, 13], exists), None);
}
