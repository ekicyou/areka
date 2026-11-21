//! シェル名つきの配置の情報源（`load_descript_source_for_shell`）の檻。
//!
//! 配置の値とシェルのフォルダが**名前の先のシェル**から来ること、名前なしは `master` のままであること、
//! 名前の先が無ければ黙って既定へ戻らず失敗することを判定する。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use super::tests::{unique_temp_dir, write_minimal_ghost_descript};
use super::*;
use crate::error::PlacementError;

/// `shell/master`（`seriko.dpi,96`）と `shell/second`（`seriko.dpi,144`）の 2 つを持つ
/// ゴーストを一時ディレクトリへ組む。両シェルの `descript.txt` は中身で見分けられる。
fn two_shell_ghost(tag: &str) -> TempPath {
    let temp = unique_temp_dir(&format!("source-shell-{tag}"));
    let root = temp.path();
    write_minimal_ghost_descript(root);
    write_shell(root, "master", "charset,UTF-8\nname,既定\nseriko.dpi,96\n");
    write_shell(root, "second", "charset,UTF-8\nname,二番\nseriko.dpi,144\n");
    temp
}

fn write_shell(root: &Path, dir: &str, descript: &str) {
    let shell_dir = root.join("shell").join(dir);
    fs::create_dir_all(&shell_dir).expect("create shell dir");
    fs::write(shell_dir.join("descript.txt"), descript.as_bytes()).expect("write shell descript");
}

/// 名前ありの情報源は、シェルのフォルダも生の KV も名前の先のシェルを指す。
#[test]
fn named_shell_source_points_at_named_shell_dir() {
    let temp = two_shell_ghost("named");
    let root = temp.path();

    let src = load_descript_source_for_shell(root, Some("second")).expect("名前の先は在る");

    assert_eq!(src.shell_dir, root.join("shell").join("second"));
    assert_eq!(src.shell_kv.get("name").map(String::as_str), Some("二番"));
    assert_eq!(src.shell_author_dpi(), 144, "名前の先のシェルの seriko.dpi");
}

/// 名前なしは今日の規則（`master`）のまま。
#[test]
fn unnamed_shell_source_keeps_today_rule() {
    let temp = two_shell_ghost("unnamed");
    let root = temp.path();

    let src = load_descript_source_for_shell(root, None).expect("master は在る");

    assert_eq!(src.shell_dir, root.join("shell").join("master"));
    assert_eq!(src.shell_kv.get("name").map(String::as_str), Some("既定"));
}

/// 名前の先が無ければ既定のシェルへ黙って戻らず `Mount` の失敗を返す。
#[test]
fn named_shell_source_missing_dir_is_mount_err() {
    let temp = two_shell_ghost("missing");

    let err = load_descript_source_for_shell(temp.path(), Some("no_such_shell"))
        .expect_err("名前の先が無ければ Err");

    assert!(
        matches!(err, PlacementError::Mount(_)),
        "Mount variant 以外が返った: {err:?}"
    );
}
