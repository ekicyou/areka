//! 測りが各スコープの descript の既定の面で合成することの檻（descript-window-keys 要件 2.1・
//! 2.3・2.4・2.6・7.2）。
//!
//! 3 通り: 検体のシェルに在る別の面を既定にするとその面の大きさ／無い番号なら組み込みの面の
//! 大きさと記録 1 件／書かれていなければ今の期待値。組み込みの面も無い場合（本体側の失敗・
//! 相方側の代わり）は `measure_tests.rs` の今の檻が確かめる。

use std::collections::BTreeMap;

use super::*;
use crate::placement::descript_window_keys::{DefaultSurfaces, read_default_surfaces};
use crate::placement::shared_test_support::{balloon_root, emo2_root, with_com_initialized};
use crate::placement::test_support::{ExpectField, LogEvent, capture_logs, expect_one};

/// emo2 の面 0（surface0.png）の原寸。
const SURFACE0: (i32, i32) = (434, 687);
/// emo2 の面 10（CityPop/surface0010.png）の原寸。
const SURFACE10: (i32, i32) = (336, 400);

fn ghost_kv(pairs: &[(&str, &str)]) -> DefaultSurfaces {
    let kv: BTreeMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    read_default_surfaces(&kv)
}

/// emo2 の本体側・相方側を `defaults` で測り、キャラの大きさの組と記録を返す。
fn measure_emo2(defaults: &DefaultSurfaces) -> (Vec<(i32, i32)>, Vec<LogEvent>) {
    let mut result = None;
    with_com_initialized(|| {
        let (out, events) = capture_logs(|| {
            measure_scope_sizes(
                &emo2_root().join("shell/master"),
                &balloon_root(),
                &[0, 1],
                defaults,
                &MeasureScaling::IDENTITY,
            )
            .expect("emo2 の測りは通る")
        });
        let sizes = out
            .scopes
            .iter()
            .map(|s| (s.char_size.w, s.char_size.h))
            .collect();
        result = Some((sizes, events));
    });
    result.expect("測りを行った")
}

/// 検体のシェルに在る別の面を descript の既定の面にすると、その面の大きさで測る（要件 2.1）。
#[test]
fn descript_default_surface_in_shell_is_measured() {
    let defaults = ghost_kv(&[
        ("sakura.seriko.defaultsurface", "10"),
        ("kero.seriko.defaultsurface", "0"),
    ]);
    let (sizes, events) = measure_emo2(&defaults);
    assert_eq!(sizes, vec![SURFACE10, SURFACE0]);
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains("default_surface_missing")),
        "在る面なら記録しない: {events:?}"
    );
}

/// シェルに無い番号なら組み込みの面の大きさで測り、スコープ・無かった面・代わりの面を 1 件記録する
/// （要件 2.3・2.6）。
#[test]
fn missing_descript_default_surface_falls_back_to_builtin_with_one_log() {
    let defaults = ghost_kv(&[("kero.seriko.defaultsurface", "9999")]);
    let (sizes, events) = measure_emo2(&defaults);
    assert_eq!(sizes, vec![SURFACE0, SURFACE10]);
    let ev = expect_one(&events, "default_surface_missing");
    assert_eq!(ev.level, tracing::Level::WARN);
    assert_eq!(ev.expect_field("scope"), "1");
    assert_eq!(ev.expect_field("missing"), "9999");
    assert_eq!(ev.expect_field("to"), "10");
}

/// 書かれていなければ今の期待値（本体側 0・相方側 10）のまま（要件 2.4・7.2）。
#[test]
fn unwritten_default_surface_keeps_current_sizes() {
    let (sizes, events) = measure_emo2(&ghost_kv(&[("name", "x")]));
    assert_eq!(sizes, vec![SURFACE0, SURFACE10]);
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains("default_surface_missing")),
        "{events:?}"
    );
}
