//! 起動の前の書けるかの確かめ（areka-P0-write-failure-notice task 5.4・要件 2.1・2.9・7.2）。
//!
//! 書けない置き場は、フォルダの在るべき所に普通のファイルを置いて作る。
//! 根はワークツリーの `target\` の下だけ（`TempPath::under_target`）。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::*;

/// フォルダの下の物の数（ファイルとフォルダ・下の階層まで全部）。
fn entries(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let path = e.unwrap().path();
            1 + if path.is_dir() { entries(&path) } else { 0 }
        })
        .sum()
}

/// `<root>\ghost\master` まで作ったゴーストのフォルダ。
fn ghost_root(base: &TempPath, name: &str) -> PathBuf {
    let root = base.child(name);
    std::fs::create_dir_all(root.join("ghost").join("master")).unwrap();
    root
}

fn ghost_profile(root: &Path) -> PathBuf {
    root.join("ghost")
        .join("master")
        .join("profile")
        .join("areka")
}

/// 在るフォルダも、在らない下のフォルダ（親を確かめる）も通り、前後で中身が増えない（要件 2.9）。
#[test]
fn probe_leaves_nothing_behind() {
    let tmp = TempPath::under_target("write-failure-probe-nothing");
    std::fs::create_dir_all(tmp.child("a")).unwrap();
    std::fs::write(tmp.child("a").join("keep.txt"), "x").unwrap();
    let before = entries(tmp.path());

    probe_dir(&tmp.child("a")).expect("在るフォルダは書ける");
    let absent = tmp.child("a").join("profile").join("areka");
    probe_dir(&absent).expect("在らない下のフォルダは親で確かめる");

    assert_eq!(entries(tmp.path()), before, "確かめで物が増えた");
    assert!(!absent.exists(), "確かめがフォルダを作った");
}

/// 普通のファイルそのものも、その下も書けない。
#[test]
fn probe_under_a_regular_file_is_unwritable() {
    let tmp = TempPath::under_target("write-failure-probe-file");
    let file = tmp.child("profile");
    std::fs::write(&file, "x").unwrap();

    assert!(probe_dir(&file).is_err());
    assert!(probe_dir(&file.join("areka")).is_err());
}

/// アプリの置き場が書けない → スコープ App・置き場はアプリの記憶の置き場（ゴーストより先）。
#[test]
fn app_place_unwritable_is_scope_app() {
    let tmp = TempPath::under_target("write-failure-probe-app");
    let app = tmp.child("app-profile");
    std::fs::write(&app, "x").unwrap();
    let root = ghost_root(&tmp, "ghost-a");
    // ゴーストも書けなくして、アプリが先に確かめられることも見る。
    std::fs::write(root.join("ghost").join("master").join("profile"), "x").unwrap();
    let cfg = ConfigInputs {
        ghost_root: root,
        balloon_root: tmp.child("balloon"),
    };

    let failure = check_boot_dirs(&cfg, &app).expect_err("アプリの置き場は書けない");

    assert_eq!(failure.scope, PersistScope::App);
    assert_eq!(failure.dir, Some(app));
    assert!(failure.keys.is_empty());
    assert!(matches!(failure.cause, PersistFailureCause::Io(_)));
}

/// ゴーストの置き場が書けない → スコープ Ghost・置き場は `<ghost_root>\ghost\master\profile\areka`。
#[test]
fn ghost_place_unwritable_is_scope_ghost() {
    let tmp = TempPath::under_target("write-failure-probe-ghost");
    let app = tmp.child("app-profile");
    std::fs::create_dir_all(&app).unwrap();
    let root = ghost_root(&tmp, "ghost-a");
    std::fs::write(root.join("ghost").join("master").join("profile"), "x").unwrap();
    let cfg = ConfigInputs {
        ghost_root: root.clone(),
        balloon_root: tmp.child("balloon"),
    };

    let failure = check_boot_dirs(&cfg, &app).expect_err("ゴーストの置き場は書けない");

    assert_eq!(failure.scope, PersistScope::Ghost);
    assert_eq!(failure.dir, Some(ghost_profile(&root)));
    assert!(failure.keys.is_empty());
    assert!(matches!(failure.cause, PersistFailureCause::Io(_)));
}

/// 写す前の元の置き場を書けなくしても、渡した置き場（写した先）が書ければ通る
/// ＝渡された `ConfigInputs` の置き場しか見ない（要件 2.1・7.2）。
#[test]
fn only_the_passed_place_is_looked_at() {
    let tmp = TempPath::under_target("write-failure-probe-passed");
    let app = tmp.child("app-profile");
    std::fs::create_dir_all(&app).unwrap();
    let original = ghost_root(&tmp, "original");
    std::fs::write(original.join("ghost").join("master").join("profile"), "x").unwrap();
    let copied = ghost_root(&tmp, "copied");
    let cfg = ConfigInputs {
        ghost_root: copied,
        balloon_root: tmp.child("balloon"),
    };

    check_boot_dirs(&cfg, &app).expect("写した先は書ける");
}
