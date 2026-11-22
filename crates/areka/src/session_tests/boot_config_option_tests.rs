//! 起動の求め（`areka_session::boot_request::BootRequest`）から起動の答えへ。
//!
//! テストの口 `resolve_boot_from` に引数の列を渡し（中で読み方を通す）、経路・場所・起動中の印の値を見る。
//! 根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下）。

use crate::session_tests::{no_pick, put_ghost, resolve_boot_from, save_scope};
use areka_session::boot_config::BootResolved;
use areka_session::boot_resolve::{
    BalloonRoute, DEFAULT_BALLOON_FOLDER, DEFAULT_GHOST_FOLDER, GhostRoute, write_session_mark,
};
use log_capture_kit::capture;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

fn first(_n: usize) -> usize {
    0
}

/// 根にゴースト `g1` と既定ゴースト、バルーン（既定・`b1`）を置き、最後のゴーストの記憶を `g1` にする。
fn fixture(label: &str) -> TempPath {
    let tmp = TempPath::under_target(label);
    for g in ["g1", DEFAULT_GHOST_FOLDER] {
        put_ghost(&tmp.child("ghost").join(g));
    }
    for b in [DEFAULT_BALLOON_FOLDER, "b1"] {
        let dir = tmp.child("balloon").join(b);
        std::fs::create_dir_all(&dir).expect("フォルダを組む");
        std::fs::write(dir.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    }
    save_scope(&tmp.child("app-profile"), "g1");
    tmp
}

fn g1(tmp: &TempPath) -> String {
    tmp.child("ghost").join("g1").display().to_string()
}

fn boot(tmp: &TempPath, rest: &[&str], pick: fn(usize) -> usize) -> BootResolved {
    boot_with(tmp, rest, pick, |_, dir| dir.to_path_buf())
}

/// 選び手と写しの口を選んで起動を決める。
fn boot_with(
    tmp: &TempPath,
    rest: &[&str],
    pick: fn(usize) -> usize,
    prepare: fn(&areka_ghost::BasewareRoots, &Path) -> PathBuf,
) -> BootResolved {
    let args: Vec<String> = std::iter::once("areka.exe")
        .chain(rest.iter().copied())
        .map(str::to_owned)
        .collect();
    resolve_boot_from(
        areka_session::roots::single_env(tmp.path().to_path_buf()),
        &args,
        &tmp.child("app-profile"),
        pick,
        prepare,
    )
    .expect("決まる")
}

/// 経路・ゴーストのフォルダ・印の値。
fn answer(got: &BootResolved) -> (GhostRoute, PathBuf, Option<String>) {
    (got.1.route, got.1.dir.clone(), got.3.clone())
}

/// オプションなしの答え: 引数なしは印が無ければ記憶・印が在れば既定、位置の引数は `Argv` で印を読まない。
#[test]
fn without_options_the_answer_follows_the_session_mark() {
    let tmp = fixture("cfg-opt-none");
    write_session_mark(&tmp.child("app-profile"), "落ちた子").expect("印を書く");
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

/// 位置の引数と `--ghost` の両方なら位置の引数のゴーストは使わない。
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

/// `nobootcheck` なら印を読まない: 印が在っても値は返らず、記憶で決まる。
#[test]
fn nobootcheck_does_not_read_the_mark() {
    let tmp = fixture("cfg-opt-nobootcheck");
    write_session_mark(&tmp.child("app-profile"), "落ちた子").expect("印を書く");
    assert_eq!(
        answer(&boot(&tmp, &["--option", "nobootcheck"], no_pick)),
        (GhostRoute::Memory, tmp.child("ghost").join("g1"), None)
    );
}

/// `--random` でも印を読み、値を返す。選びは無作為の段（記憶・既定を見ない）。
#[test]
fn random_still_returns_the_mark() {
    let tmp = fixture("cfg-opt-random-mark");
    write_session_mark(&tmp.child("app-profile"), "落ちた子").expect("印を書く");
    let got = boot(&tmp, &["--random"], first);
    assert_eq!(
        (got.1.route, got.3.as_deref()),
        (GhostRoute::Random, Some("落ちた子"))
    );
}

/// 位置の引数のゴーストと `--random` なら位置の引数を起こし、無作為を使わなかったことを info。
#[test]
fn positional_ghost_with_random_logs_unused_random() {
    let tmp = fixture("cfg-opt-positional-random");
    let argv = g1(&tmp);
    let (got, events) = capture(|| answer(&boot(&tmp, &[&argv, "--random"], no_pick)));
    assert_eq!(got, (GhostRoute::Argv, PathBuf::from(&argv), None));
    assert_eq!(
        super::levels_of(&events, "random_unused"),
        vec![tracing::Level::INFO]
    );
}

/// 位置のバルーンは `--ghost` の有無に関わらず通常の道で使う。
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

// ---------------------------------------------------------------------------
// `--ghost` の指し先
// ---------------------------------------------------------------------------

/// 写しの口が呼ばれたら分かるように、フォルダの下の `copied` を返す。
fn mark_copy(_: &areka_ghost::BasewareRoots, dir: &Path) -> PathBuf {
    dir.join("copied")
}

/// `name` を付けられる `put_ghost`。
fn put_named_ghost(dir: &Path, name: &str) {
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\nname,{name}\n"),
    )
    .expect("descript");
}

/// 絶対パスで置き場の外のゴーストを指すと、親が一時の置き場として並びと状態の先頭に足され、
/// そのフォルダが `Pointed` で起きる。info `temporary_root_added` が 1 件。
#[test]
fn absolute_path_adds_temporary_root_at_head() {
    let tmp = fixture("cfg-opt-ghost-abs");
    let dev = tmp.child("ext").join("dev");
    put_ghost(&dev);
    let dev_s = dev.display().to_string();
    let (got, events) = capture(|| boot(&tmp, &["--ghost", &dev_s], no_pick));
    assert_eq!(
        (got.1.route, &got.1.dir, got.1.folder.as_deref()),
        (GhostRoute::Pointed, &dev, Some("dev"))
    );
    assert_eq!(got.0.ghost_root, dev, "構成入力も指されたフォルダ");
    let roots = got.4.roots.roots();
    assert_eq!(roots.len(), 2);
    assert!(roots[0].is_temporary());
    assert_eq!(roots[0].dir(), tmp.child("ext"));
    assert!(!roots[1].is_temporary());
    assert_eq!(got.4.status.len(), 2, "状態と並びの数が同じ");
    assert_eq!(
        (got.4.status[0].dir.as_path(), got.4.status[0].found),
        (tmp.child("ext").as_path(), true)
    );
    assert_eq!(
        super::levels_of(&events, "temporary_root_added"),
        vec![tracing::Level::INFO]
    );
}

/// 親がすでに置き場の `ghost\` なら一時の置き場を足さず、そのゴーストとして起こす。
#[test]
fn absolute_path_under_root_ghost_store_adds_no_temporary_root() {
    let tmp = fixture("cfg-opt-ghost-abs-in-root");
    let argv = g1(&tmp);
    let (got, events) = capture(|| boot(&tmp, &["--ghost", &argv], no_pick));
    assert_eq!(
        (got.1.route, got.1.dir.clone()),
        (GhostRoute::Pointed, PathBuf::from(&argv))
    );
    assert_eq!(got.4.roots.roots().len(), 1);
    assert!(!got.4.roots.roots()[0].is_temporary());
    assert_eq!(
        super::levels_of(&events, "temporary_root_added"),
        Vec::<tracing::Level>::new()
    );
}

/// 同じフォルダ名のゴーストが置き場にあっても、写しの口があっても、指されたフォルダそのもの。
#[test]
fn absolute_path_boots_the_pointed_folder_itself() {
    let tmp = fixture("cfg-opt-ghost-abs-same-name");
    let ext_g1 = tmp.child("ext").join("g1");
    put_ghost(&ext_g1);
    let s = ext_g1.display().to_string();
    let got = boot_with(&tmp, &["--ghost", &s], no_pick, mark_copy);
    assert_eq!(
        (got.1.route, &got.1.dir, got.1.folder.as_deref()),
        (GhostRoute::Pointed, &ext_g1, Some("g1"))
    );
}

/// 絶対パスだがゴーストのフォルダでない: 一時の置き場を足さず warn 1 件・いつもの選び方。
#[test]
fn absolute_path_not_a_ghost_folder_falls_back_to_usual() {
    let tmp = fixture("cfg-opt-ghost-abs-not-ghost");
    let plain = tmp.child("ext").join("plain");
    std::fs::create_dir_all(&plain).expect("フォルダを組む");
    let s = plain.display().to_string();
    let (got, events) = capture(|| boot(&tmp, &["--ghost", &s], no_pick));
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Memory, Some("g1"))
    );
    assert_eq!(got.4.roots.roots().len(), 1);
    assert_eq!(
        super::levels_of(&events, "ghost_option_not_ghost_dir"),
        vec![tracing::Level::WARN]
    );
}

/// 次の起動で一時の置き場が無く、記憶が一時の置き場のゴーストを指す: 見つからないを記録して次の段へ。
#[test]
fn next_boot_does_not_find_the_temporary_ghost_in_memory() {
    let tmp = fixture("cfg-opt-ghost-next-boot");
    put_ghost(&tmp.child("ext").join("dev"));
    save_scope(&tmp.child("app-profile"), "dev");
    let (got, events) = capture(|| boot(&tmp, &[], no_pick));
    assert_eq!(got.1.route, GhostRoute::Default);
    assert_eq!(got.4.roots.roots().len(), 1, "一時の置き場は残らない");
    assert_eq!(
        super::levels_of(&events, "last_ghost_not_found"),
        vec![tracing::Level::WARN]
    );
}

/// 名前は `name` が先・一致が無ければフォルダ名。名前の指し先は写す。
#[test]
fn name_matches_descript_name_before_folder_name() {
    let tmp = fixture("cfg-opt-ghost-name");
    // フォルダ名 `Alpha` のゴーストと、`name,Alpha` のフォルダ `named`。
    put_named_ghost(&tmp.child("ghost").join("Alpha"), "Other");
    put_named_ghost(&tmp.child("ghost").join("named"), "Alpha");
    let got = boot_with(&tmp, &["--ghost", "Alpha"], no_pick, mark_copy);
    assert_eq!(
        (got.1.route, &got.1.dir, got.1.folder.as_deref()),
        (
            GhostRoute::Pointed,
            &tmp.child("ghost").join("named").join("copied"),
            Some("named")
        ),
        "name が先・名前の指し先は写す"
    );
    let got = boot(&tmp, &["--ghost", "g1"], no_pick);
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Pointed, Some("g1")),
        "name に無ければフォルダ名"
    );
}

/// 名前が見つからない: warn 1 件・いつもの選び方。
#[test]
fn name_not_found_falls_back_to_usual() {
    let tmp = fixture("cfg-opt-ghost-name-miss");
    let (got, events) = capture(|| boot(&tmp, &["--ghost", "nobody"], no_pick));
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Memory, Some("g1"))
    );
    assert_eq!(
        super::levels_of(&events, "ghost_option_not_found"),
        vec![tracing::Level::WARN]
    );
}

/// 値が 2 つ以上なら最初の値のゴーストだけ。
#[test]
fn extra_ghost_values_boot_only_the_first() {
    let tmp = fixture("cfg-opt-ghost-extra");
    let got = boot(&tmp, &["--ghost", DEFAULT_GHOST_FOLDER, "g1"], no_pick);
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Pointed, Some(DEFAULT_GHOST_FOLDER))
    );
}

/// 値が無ければいつもの選び方。
#[test]
fn ghost_without_value_uses_usual_choice() {
    let tmp = fixture("cfg-opt-ghost-empty");
    let got = boot(&tmp, &["--ghost"], no_pick);
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Memory, Some("g1"))
    );
}

/// `--ghost` でも起動中の印を読み、値を返す。起こすのは指し先（印に関わらず）。
#[test]
fn ghost_option_still_returns_the_mark() {
    let tmp = fixture("cfg-opt-ghost-mark");
    write_session_mark(&tmp.child("app-profile"), "落ちた子").expect("印を書く");
    let dev = tmp.child("ext").join("dev");
    put_ghost(&dev);
    let dev_s = dev.display().to_string();
    for (value, folder) in [("g1", "g1"), (dev_s.as_str(), "dev")] {
        let got = boot(&tmp, &["--ghost", value], no_pick);
        assert_eq!(
            (got.1.route, got.1.folder.as_deref(), got.3.as_deref()),
            (GhostRoute::Pointed, Some(folder), Some("落ちた子"))
        );
    }
}

/// 指し先と `--random`: 指し先を起こし、無作為を使わなかったことを info 1 件。
#[test]
fn pointed_ghost_with_random_logs_unused_random() {
    let tmp = fixture("cfg-opt-ghost-random");
    let (got, events) = capture(|| boot(&tmp, &["--ghost", "g1", "--random"], no_pick));
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Pointed, Some("g1"))
    );
    assert_eq!(
        super::levels_of(&events, "random_unused"),
        vec![tracing::Level::INFO]
    );
}
