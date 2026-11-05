//! `plan` の兄弟テストの続き——計画は根へ何も書かない（要件 4.1・設計「読む → 入れる」）。
//!
//! 助手と固定入力は [`super`]（`plan_tests.rs`）から借りる。根の木は公開面のテストと
//! 共有の `tree`（相対パスとバイト列そのもの）で写すので、同じ長さの書き換えも見える。

use super::*;
use crate::tests::tree;

/// `build_plan` は根を 1 バイトも書き換えない（読むのは宛先ゴーストの実在だけ）。
#[test]
fn build_plan_writes_nothing_under_the_root() {
    let root = root_with_ghost("test-ghost");
    std::fs::write(
        root.path()
            .join("ghost")
            .join("test-ghost")
            .join("install.txt"),
        b"x",
    )
    .expect("宛先ゴーストに 1 件置ける");
    let before = tree(root.path());
    assert!(!before.is_empty(), "根が空なら突合は恒真で緑になる");

    let free = InstallRequest {
        roots: &SingleRoot(root.path()),
        target_ghost: None,
    };
    let targeted = InstallRequest {
        roots: &SingleRoot(root.path()),
        target_ghost: Some("test-ghost"),
    };
    let missing = InstallRequest {
        roots: &SingleRoot(root.path()),
        target_ghost: Some("not-installed"),
    };
    let mut walked = 0;
    for (archive, request) in [
        (ghost_archive(), &free),
        (balloon_archive(), &free),
        (shell_archive(), &targeted),
        (supplement_archive(), &targeted),
        (empty_folder_archive(), &free),
        (no_folder_entry_archive(), &free),
        (shell_archive(), &missing),
        (supplement_archive(), &free),
    ] {
        let _ = planned(archive, request);
        walked += 1;
    }
    assert_eq!(walked, 8, "固定入力を 1 本も通していなければ突合は恒真");
    assert_eq!(before, tree(root.path()), "build_plan が根の中身を変えた");
}

/// 上の突合の較正。書き込みを 3 通り持ち込めば必ず差が出る（中身の変化は同じ長さで試す）。
#[test]
fn the_tree_notices_every_shape_of_write() {
    let root = root_with_ghost("test-ghost");
    let ghost = root.path().join("ghost").join("test-ghost");

    let before = tree(root.path());
    std::fs::write(ghost.join("x.txt"), b"1").expect("ファイルを作れる");
    let after_create = tree(root.path());
    assert_ne!(before, after_create, "ファイルの追加を見落とす");

    std::fs::write(ghost.join("x.txt"), b"2").expect("ファイルを書き替えられる");
    let after_write = tree(root.path());
    assert_ne!(after_create, after_write, "中身の変化を見落とす");

    std::fs::create_dir(root.path().join("balloon")).expect("フォルダを作れる");
    assert_ne!(after_write, tree(root.path()), "空フォルダの追加を見落とす");
}
