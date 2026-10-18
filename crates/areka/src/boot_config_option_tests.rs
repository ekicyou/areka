//! 起動の求め（`cli::BootRequest`）から起動の答えへ（areka-P0-startup-options-core タスク 5.2・
//! 要件 2.2・2.4・2.5・5.3・6.2・6.3・6.8・11.2・11.5）。
//!
//! テストの口 `resolve_boot_from` に引数の列を渡し（中で読み方を通す）、経路・場所・起動中の印の値を見る。
//! 根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下・11.5）。

use crate::boot_config::{BootResolved, resolve_boot_from};
use crate::boot_resolve::{BalloonRoute, DEFAULT_GHOST_FOLDER, GhostRoute, write_session_mark};
use log_capture_kit::capture;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

fn first(_n: usize) -> usize {
    0
}

fn put_ghost(dir: &Path) {
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
}

/// 根にゴースト `g1` と既定ゴースト、バルーン（既定・`b1`）を置き、最後のゴーストの記憶を `g1` にする。
fn fixture(label: &str) -> TempPath {
    use areka_sylphya::persist::FsPersistIo;
    use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, save_scope};
    let tmp = TempPath::under_target(label);
    for g in ["g1", DEFAULT_GHOST_FOLDER] {
        put_ghost(&tmp.child("ghost").join(g));
    }
    for b in ["StayseeBalloon", "b1"] {
        let dir = tmp.child("balloon").join(b);
        std::fs::create_dir_all(&dir).expect("フォルダを組む");
        std::fs::write(dir.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    }
    save_scope(
        PersistScope::App,
        &ScopeRoots {
            app: Some(tmp.child("app-profile")),
            ..ScopeRoots::default()
        },
        &FsPersistIo,
        vec![(PersistKey::LastGhost, "g1".to_owned())],
    );
    tmp
}

fn g1(tmp: &TempPath) -> String {
    tmp.child("ghost").join("g1").display().to_string()
}

fn boot(tmp: &TempPath, rest: &[&str], pick: fn(usize) -> usize) -> BootResolved {
    let args: Vec<String> = std::iter::once("areka.exe")
        .chain(rest.iter().copied())
        .map(str::to_owned)
        .collect();
    resolve_boot_from(
        crate::roots::single_env(tmp.path().to_path_buf()),
        &args,
        &tmp.child("app-profile"),
        pick,
        |_, dir| dir.to_path_buf(),
    )
    .expect("決まる")
}

/// 経路・ゴーストのフォルダ・印の値。
fn answer(got: &BootResolved) -> (GhostRoute, PathBuf, Option<String>) {
    (got.1.route, got.1.dir.clone(), got.3.clone())
}

/// オプションなしは今と同じ答え（2.2）: 引数なしは記憶、位置の引数は `Argv` で印を読まない。
#[test]
fn without_options_the_answer_is_as_today() {
    let tmp = fixture("cfg-opt-none");
    let _ = write_session_mark(&tmp.child("app-profile"), "落ちた子");
    assert_eq!(
        answer(&boot(&tmp, &[], no_pick)),
        (
            GhostRoute::Default,
            tmp.child("ghost").join(DEFAULT_GHOST_FOLDER),
            Some("落ちた子".to_owned())
        ),
        "印が在れば記憶を読まず既定"
    );
    let argv = g1(&tmp);
    assert_eq!(
        answer(&boot(&tmp, &[&argv], no_pick)),
        (GhostRoute::Argv, PathBuf::from(&argv), None),
        "位置の引数は印を読まない"
    );
}

/// 位置の引数と `--ghost` の両方なら位置の引数のゴーストは使わない（2.4）。
/// 指し先が見つからなくても位置の引数へ戻らず、いつもの選び方（記憶）。
#[test]
fn ghost_option_overrides_positional_ghost() {
    let tmp = fixture("cfg-opt-positional-ghost");
    let got = boot(&tmp, &[&g1(&tmp), "--ghost", "nobody"], no_pick);
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Memory, Some("g1"))
    );
}

/// `nobootcheck` なら印を読まない（6.2）: 印が在っても値は返らず、記憶で決まる。
#[test]
fn nobootcheck_does_not_read_the_mark() {
    let tmp = fixture("cfg-opt-nobootcheck");
    let _ = write_session_mark(&tmp.child("app-profile"), "落ちた子");
    assert_eq!(
        answer(&boot(&tmp, &["--option", "nobootcheck"], no_pick)),
        (GhostRoute::Memory, tmp.child("ghost").join("g1"), None)
    );
}

/// `--random` でも印を読み、値を返す（6.8）。選びは無作為の段（記憶・既定を見ない）。
#[test]
fn random_still_returns_the_mark() {
    let tmp = fixture("cfg-opt-random-mark");
    let _ = write_session_mark(&tmp.child("app-profile"), "落ちた子");
    let got = boot(&tmp, &["--random"], first);
    assert_eq!(
        (got.1.route, got.3.as_deref()),
        (GhostRoute::Random, Some("落ちた子"))
    );
}

/// 位置の引数のゴーストと `--random` なら位置の引数を起こし、無作為を使わなかったことを info（5.3）。
#[test]
fn positional_ghost_with_random_logs_unused_random() {
    let tmp = fixture("cfg-opt-positional-random");
    let argv = g1(&tmp);
    let (got, events) = capture(|| answer(&boot(&tmp, &[&argv, "--random"], no_pick)));
    assert_eq!(got, (GhostRoute::Argv, PathBuf::from(&argv), None));
    let unused = events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::INFO && e.field_str("event") == Some("random_unused")
        })
        .count();
    assert_eq!(unused, 1);
}

/// 位置のバルーンは `--ghost` の有無に関わらず今の道で使う（2.5）。
#[test]
fn positional_balloon_is_used_with_or_without_ghost_option() {
    let tmp = fixture("cfg-opt-positional-balloon");
    let b1 = tmp.child("balloon").join("b1");
    let b1s = b1.display().to_string();
    for rest in [
        vec![g1(&tmp), b1s.clone()],
        vec![
            g1(&tmp),
            b1s.clone(),
            "--ghost".to_owned(),
            "nobody".to_owned(),
        ],
    ] {
        let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
        let got = boot(&tmp, &rest, no_pick);
        assert_eq!((got.2.route, &got.2.dir), (BalloonRoute::Argv, &b1));
    }
}
