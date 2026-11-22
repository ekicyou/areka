//! 起動の前の書けるかの確かめ。
//!
//! 書けない置き場は、フォルダの在るべき所に普通のファイルを置いて作る。
//! 根はワークツリーの `target\` の下だけ（`TempPath::under_target`）。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::*;
use crate::readonly::tests::{tree, unwritable_places};

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

/// 在るフォルダも、在らない下のフォルダ（親を確かめる）も通り、前後で中身が増えない。
#[test]
fn probe_leaves_nothing_behind() {
    let tmp = TempPath::under_target("write-failure-probe-nothing");
    std::fs::create_dir_all(tmp.child("a")).unwrap();
    std::fs::write(tmp.child("a").join("keep.txt"), "x").unwrap();
    let before = tree(tmp.path());

    probe_dir(&tmp.child("a")).expect("在るフォルダは書ける");
    let absent = tmp.child("a").join("profile").join("areka");
    probe_dir(&absent).expect("在らない下のフォルダは親で確かめる");

    assert_eq!(tree(tmp.path()), before, "確かめが木を変えた");
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
    // ゴーストも書けない置き場なので、アプリが先に確かめられることも見る。
    let (cfg, app) = unwritable_places(&tmp);

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
/// ＝渡された `ConfigInputs` の置き場しか見ない。
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
