//! 測りが各スコープの descript の既定の面で合成することの檻。
//!
//! 3 通り: 検体のシェルに在る別の面を既定にするとその面の大きさ／無い番号なら組み込みの面の
//! 大きさと記録 1 件／組み込みの面も無ければ「既定の面が無い」を記録せず合成の失敗の扱い（相方側は
//! 本体側の大きさ）へ委ねる。本体側の失敗は `measure_tests.rs` が確かめる。

use std::path::Path;

use super::tests::surface0_only_shell;
use super::*;
use crate::descript_window_keys::{DefaultSurfaces, read_default_surfaces};
use crate::sample_test_support::{SCOPE0_H, SCOPE0_W, SCOPE1_H, SCOPE1_W, balloon_root, emo2_root};
use crate::test_support::{kv, with_com_initialized};
use log_capture_kit::{CapturedEvent, ExpectField, capture, expect_one};

fn ghost_kv(pairs: &[(&str, &str)]) -> DefaultSurfaces {
    read_default_surfaces(&kv(pairs))
}

/// emo2 の本体側・相方側を `defaults` で測り、キャラの大きさの組と記録を返す。
fn measure_emo2(defaults: &DefaultSurfaces) -> (Vec<(i32, i32)>, Vec<CapturedEvent>) {
    measure_chars(&emo2_root().join("shell/master"), defaults)
}

/// `shell` の本体側・相方側を `defaults` で測り、キャラの大きさの組と記録を返す。
fn measure_chars(
    shell: &Path,
    defaults: &DefaultSurfaces,
) -> (Vec<(i32, i32)>, Vec<CapturedEvent>) {
    let mut result = None;
    with_com_initialized(|| {
        let (out, events) = capture(|| {
            measure_scope_sizes(
                shell,
                &balloon_root(),
                &[0, 1],
                defaults,
                &MeasureScaling::IDENTITY,
            )
            .expect("測りは通る")
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

/// 検体のシェルに在る別の面を descript の既定の面にすると、その面の大きさで測る。
#[test]
fn descript_default_surface_in_shell_is_measured() {
    let defaults = ghost_kv(&[
        ("sakura.seriko.defaultsurface", "10"),
        ("kero.seriko.defaultsurface", "0"),
    ]);
    let (sizes, events) = measure_emo2(&defaults);
    assert_eq!(sizes, vec![(SCOPE1_W, SCOPE1_H), (SCOPE0_W, SCOPE0_H)]);
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains("default_surface_missing")),
        "在る面なら記録しない: {events:?}"
    );
}

/// シェルに無い番号なら組み込みの面の大きさで測り、スコープ・無かった面・代わりの面を 1 件記録する。
#[test]
fn missing_descript_default_surface_falls_back_to_builtin_with_one_log() {
    let defaults = ghost_kv(&[("kero.seriko.defaultsurface", "9999")]);
    let (sizes, events) = measure_emo2(&defaults);
    assert_eq!(sizes, vec![(SCOPE0_W, SCOPE0_H), (SCOPE1_W, SCOPE1_H)]);
    let ev = expect_one(&events, "default_surface_missing");
    assert_eq!(ev.level, tracing::Level::WARN);
    assert_eq!(ev.expect_field("scope"), "1");
    assert_eq!(ev.expect_field("missing"), "9999");
    assert_eq!(ev.expect_field("to"), "10");
}

/// descript の面も組み込みの面もシェルに無いときは「既定の面が無い」を記録しない。相方側は合成の
/// 失敗の扱い（本体側の大きさで代える・記録 1 件）へ委ねる。
#[test]
fn no_candidate_in_shell_leaves_it_to_the_compose_failure() {
    // 相方側の候補 12・10 はどちらも無い。
    let shell = surface0_only_shell(b"surface0\n{\nelement0,overlay,surface0.png,0,0\n}\n");

    let defaults = ghost_kv(&[("kero.seriko.defaultsurface", "12")]);
    let (sizes, events) = measure_chars(shell.path(), &defaults);
    assert_eq!(sizes, vec![(SCOPE0_W, SCOPE0_H), (SCOPE0_W, SCOPE0_H)]);
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains("default_surface_missing")),
        "候補がどれも無いなら記録しない: {events:?}"
    );
    expect_one(&events, "スコープの採寸合成に失敗");
}
