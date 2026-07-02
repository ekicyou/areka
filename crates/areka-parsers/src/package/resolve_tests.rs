//! resolve_tests — `package::resolve` 正常系（happy path）の証明テスト。
//!
//! 本タスク（3.1）が担うのは **正常系** の 1 本のみ。失敗系（`StartPointMissing`
//! / `StartPointUnreadable` / `ShellDirMissing`）の網羅マトリクスと `sakura`
//! 非対称ドキュメントはタスク 3.2 / 4.1 の `resolve_tests` が担う（重複回避）。
//!
//! 外部クレート（tempfile 等）に依存せず、`std::env::temp_dir()` 直下に
//! テスト関数名でユニークな一時ツリーを構築する。区切り文字は `Path::join`
//! に委ね、クロスプラットフォームで決定的に振る舞う。

use std::fs;
use std::path::PathBuf;

use crate::charset::DefaultEncoding;

use super::resolve;

/// このテスト専用の一意な一時ディレクトリを返す（関数名でユニーク化・衝突回避）。
fn unique_temp_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_package_resolve_tests_{tag}"));
    dir
}

#[test]
fn resolve_happy_path_builds_mount_model() {
    // --- Arrange: 正常なゴーストツリーを一時ディレクトリに構築 ---
    let root = unique_temp_dir("happy_path_builds_mount_model");
    // 前回残骸があれば掃除（best-effort）。
    let _ = fs::remove_dir_all(&root);

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    // seriko.defaultsurfacedirectoryname を明示指定し、shell/<名> を実在させる。
    let shell_name = "master";
    let shell_dir = root.join("shell").join(shell_name);
    fs::create_dir_all(&shell_dir).expect("create shell/<name>");

    let descript = ghost_master.join("descript.txt");
    let contents = "charset,UTF-8\n\
         type,ghost\n\
         name,テスト\n\
         sakura.name,さくら\n\
         kero.name,けろ\n\
         shiori,pasta.dll\n\
         seriko.defaultsurfacedirectoryname,master\n";
    fs::write(&descript, contents.as_bytes()).expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert ---
    let model = result.expect("正常ツリーは Ok(MountModel) を返す");

    // SHIORI マウント: dir = root/ghost/master、file = Some("pasta.dll")（推測なし）。
    assert_eq!(model.shiori.dir, ghost_master);
    assert_eq!(model.shiori.file, Some("pasta.dll".to_string()));

    // shell マウント: dir = root/shell/master（存在確認済み）。
    assert_eq!(model.shell.dir, shell_dir);
    assert!(model.shell.dir.is_dir(), "解決した shell dir は実在する");

    // 名前情報（欠落なし）。
    assert_eq!(model.names.name, Some("テスト".to_string()));
    assert_eq!(model.names.sakura_name, Some("さくら".to_string()));
    assert_eq!(model.names.kero_name, Some("けろ".to_string()));

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}
