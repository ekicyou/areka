//! resolve_tests — `package::resolve` 正常系（happy path）＋失敗系マトリクスの証明テスト。
//!
//! 正常系（3.1）に加え、失敗系（3.2）の 3 経路
//! （`StartPointMissing` / `StartPointUnreadable` / `ShellDirMissing`）を検証する。
//! この 3 経路が **区別できる**（それぞれ固有の variant を返す）ことこそが本題で、
//! `sakura` の `Result` 無し寛容パースとは意図的に非対称である
//! （マウントは物理不在という現実の失敗を持つ・Req 5.1、詳細は `mod.rs` 冒頭）。
//!
//! 外部クレート（tempfile 等）に依存せず、`std::env::temp_dir()` 直下に
//! テスト関数名でユニークな一時ツリーを構築する。区切り文字は `Path::join`
//! に委ね、クロスプラットフォームで決定的に振る舞う。

use std::fs;
use std::path::PathBuf;

use crate::charset::DefaultEncoding;

use super::{resolve, MountError};

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

// ---------------------------------------------------------------------------
// 失敗系マトリクス（3.2）— 3 経路が区別できることを証明する。
//
// `sakura` は `Result` を持たず寛容パースだが、`resolve` は物理不在という
// 現実の致命失敗を観測可能な `Err` として早期 return する（意図的な非対称・
// Req 1.6/3.3/5.1、根拠は `mod.rs` 冒頭のモジュールコメント）。各テストは
// 該当する **固有の** variant を assert し、3 経路の識別可能性を担保する。
// ---------------------------------------------------------------------------

/// 起点 `ghost/master/descript.txt` が不在 → `StartPointMissing`。
///
/// descript が存在しないと `std::fs::read` は `NotFound` を返し、これは
/// 読取失敗（`StartPointUnreadable`）とは区別されねばならない。
#[test]
fn resolve_missing_descript_yields_start_point_missing() {
    // --- Arrange: descript.txt を含まないツリー（ghost/master ディレクトリすら作らない）---
    let root = unique_temp_dir("missing_descript_yields_start_point_missing");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: 固有 variant（StartPointMissing）を返す ---
    match result {
        Err(MountError::StartPointMissing { expected }) => {
            // expected は起点 descript.txt のパスを指す（黙って空を返さない）。
            assert_eq!(expected, root.join("ghost").join("master").join("descript.txt"));
        }
        other => panic!("StartPointMissing を期待したが {other:?} が返った"),
    }

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}

/// 起点 descript.txt は所在するが読取に失敗 → `StartPointUnreadable`。
///
/// `ghost/master/descript.txt` を **ディレクトリ** として作ると、`std::fs::read`
/// は Windows / Unix いずれでも I/O エラー（`NotFound` 以外）で失敗する。これにより
/// 「所在するが読めない」経路を権限トリック無しでクロスプラットフォームに再現でき、
/// `StartPointMissing`（不在）とは区別される。
#[test]
fn resolve_unreadable_descript_yields_start_point_unreadable() {
    // --- Arrange: descript.txt を「ディレクトリ」として作る（read が失敗する）---
    let root = unique_temp_dir("unreadable_descript_yields_start_point_unreadable");
    let _ = fs::remove_dir_all(&root);
    let descript_as_dir = root.join("ghost").join("master").join("descript.txt");
    fs::create_dir_all(&descript_as_dir).expect("create descript.txt as a directory");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: 固有 variant（StartPointUnreadable）を返す ---
    match result {
        Err(MountError::StartPointUnreadable { path, kind }) => {
            assert_eq!(path, descript_as_dir);
            // ディレクトリ読取は NotFound ではない I/O エラー（不在と区別される）。
            assert_ne!(kind, std::io::ErrorKind::NotFound);
        }
        other => panic!("StartPointUnreadable を期待したが {other:?} が返った"),
    }

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}

/// 解決した shell ディレクトリが不在 → `ShellDirMissing`。
///
/// 起点 descript.txt は正常な file として存在し `seriko.defaultsurfacedirectoryname`
/// で `nope` を指すが、`shell/nope` を実在させないため shell 存在確認で失敗する。
/// 起点系の失敗（不在・読取不能）とは区別される。
#[test]
fn resolve_missing_shell_dir_yields_shell_dir_missing() {
    // --- Arrange: 正常な descript.txt（file）だが shell/<名> は作らない ---
    let root = unique_temp_dir("missing_shell_dir_yields_shell_dir_missing");
    let _ = fs::remove_dir_all(&root);

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let descript = ghost_master.join("descript.txt");
    fs::write(
        &descript,
        b"charset,UTF-8\nseriko.defaultsurfacedirectoryname,nope\n",
    )
    .expect("write descript.txt");
    // shell/nope はあえて作らない。

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: 固有 variant（ShellDirMissing）を返す ---
    match result {
        Err(MountError::ShellDirMissing { expected }) => {
            assert_eq!(expected, root.join("shell").join("nope"));
        }
        other => panic!("ShellDirMissing を期待したが {other:?} が返った"),
    }

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// 正常・境界系マトリクス（4.1）— 「推測しない」寛容受理を証明する。
//
// 失敗系 3 経路（不在・読取不能・shell 不在）は上記 3.2 で検証済み。ここでは
// 起点が正常に読める場合の 3 つの境界を assert する：`shiori` 未指定なら
// 既定へ推測せず `None`（Req 2.3）／`seriko.defaultsurfacedirectoryname`
// 未指定なら既定 `master` へフォールバック（Req 3.1）／`type` 欠落でも所在
// ベースで受理し `Ok`（Req 1.2/1.3）。これで 6 シナリオ全マトリクスが揃う。
// ---------------------------------------------------------------------------

/// `shiori` 未指定 → `shiori.file == None`（既定 `"shiori.dll"` へ推測しない・Req 2.3）。
///
/// descript に `shiori,` 行を含めず、shell dir は実在させる。`resolve` は `Ok` を
/// 返し、SHIORI ファイル名は **推測されず** `None` であることを assert する。
#[test]
fn resolve_missing_shiori_yields_file_none() {
    // --- Arrange: shiori 行を持たない正常な descript.txt ＋ 実在 shell dir ---
    let root = unique_temp_dir("missing_shiori_yields_file_none");
    let _ = fs::remove_dir_all(&root);

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let shell_dir = root.join("shell").join("master");
    fs::create_dir_all(&shell_dir).expect("create shell/master");

    let descript = ghost_master.join("descript.txt");
    // shiori 行を **あえて含めない**。
    fs::write(
        &descript,
        "charset,UTF-8\n\
         type,ghost\n\
         name,テスト\n\
         seriko.defaultsurfacedirectoryname,master\n"
            .as_bytes(),
    )
    .expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: Ok かつ shiori.file は推測されず None ---
    let model = result.expect("shiori 未指定でも Ok（マウント点は所在で決まる）");
    assert!(
        model.shiori.file.is_none(),
        "shiori 未指定は None であるべき（\"shiori.dll\" 等へ推測しない）: {:?}",
        model.shiori.file
    );
    // dir 自体は起点の親として確定している。
    assert_eq!(model.shiori.dir, ghost_master);

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}

/// `seriko.defaultsurfacedirectoryname` 未指定 かつ `shell/master` 実在
/// → 既定 `master` フォールバック（Req 3.1）。
///
/// shell 名の指定行を含めず、`shell/master` を実在させる。`resolve` は既定
/// `master` へフォールバックし、`model.shell.dir == root/shell/master` を assert する。
#[test]
fn resolve_missing_shell_name_falls_back_to_master() {
    // --- Arrange: shell 名指定なし ＋ 実在する shell/master ---
    let root = unique_temp_dir("missing_shell_name_falls_back_to_master");
    let _ = fs::remove_dir_all(&root);

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let shell_master = root.join("shell").join("master");
    fs::create_dir_all(&shell_master).expect("create shell/master");

    let descript = ghost_master.join("descript.txt");
    // seriko.defaultsurfacedirectoryname を **あえて含めない**。
    fs::write(
        &descript,
        b"charset,UTF-8\ntype,ghost\nname,fallback\n",
    )
    .expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: Ok かつ既定 master へフォールバック ---
    let model = result.expect("shell 名未指定でも実在 master へフォールバックし Ok");
    assert_eq!(
        model.shell.dir, shell_master,
        "shell 名未指定は既定 master へフォールバックする"
    );
    assert!(model.shell.dir.is_dir(), "フォールバックした shell dir は実在する");

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}

/// `type,ghost` 欠落でも所在ベースで受理 → `Ok`（Req 1.2/1.3）。
///
/// descript に `type` 行を一切含めず（`charset` と `name` のみ）、shell dir は実在
/// させる。`type` 欠落は失敗ではなく、マウントは物理所在で識別されるため `resolve`
/// は `Ok` を返すことを assert する。
#[test]
fn resolve_missing_type_is_accepted() {
    // --- Arrange: type 行を持たない descript.txt ＋ 実在 shell dir ---
    let root = unique_temp_dir("missing_type_is_accepted");
    let _ = fs::remove_dir_all(&root);

    let ghost_master = root.join("ghost").join("master");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let shell_dir = root.join("shell").join("master");
    fs::create_dir_all(&shell_dir).expect("create shell/master");

    let descript = ghost_master.join("descript.txt");
    // type 行を **あえて含めない**（charset と name のみ）。
    fs::write(
        &descript,
        "charset,UTF-8\n\
         name,no-type\n\
         seriko.defaultsurfacedirectoryname,master\n"
            .as_bytes(),
    )
    .expect("write descript.txt");

    // --- Act ---
    let result = resolve(&root, DefaultEncoding::Utf8);

    // --- Assert: type 欠落は失敗ではない（所在ベース識別）→ Ok ---
    let model = result.expect("type 欠落でも所在ベースで受理し Ok を返す");
    // 名前情報は読めており、マウントも所在で確定する。
    assert_eq!(model.names.name, Some("no-type".to_string()));
    assert_eq!(model.shell.dir, shell_dir);

    // --- Cleanup（best-effort）---
    let _ = fs::remove_dir_all(&root);
}
