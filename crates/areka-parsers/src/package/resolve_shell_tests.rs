//! resolve_shell_tests — シェル名つきの解決 `resolve_with_shell` の兄弟テスト。
//!
//! 名前なし（`None`）は今日の規則（`seriko.defaultsurfacedirectoryname` → `master`）と
//! 同じマウントを返し、名前あり（`Some("B")`）は `shell/B` と B の `descript.txt` の
//! bindgroup を返し、名前の先が無ければ今日と同じ `ShellDirMissing` になることを固定する。
//! 一時ツリーは共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意・破棄で消える）。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use crate::charset::DefaultEncoding;

use super::{MountError, resolve, resolve_with_shell};

/// ghost/master の descript.txt と、`shell/A`（bindgroup 1100 が既定オン）・
/// `shell/B`（bindgroup 2200 が既定オン）の 2 つのシェルを持つ一時ツリーを作る。
///
/// 既定のシェルは `seriko.defaultsurfacedirectoryname,A` で A を指す。
fn ghost_with_two_shells(tag: &str) -> TempPath {
    // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
    let root = TempPath::new(&format!("parsers-resolve-shell-{}", tag.replace('_', "-")));
    let master = root.path().join("ghost").join("master");
    fs::create_dir_all(&master).expect("create ghost/master");
    fs::write(
        master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nseriko.defaultsurfacedirectoryname,A\n".as_bytes(),
    )
    .expect("write ghost descript.txt");
    write_shell(
        root.path(),
        "A",
        "charset,UTF-8\nsakura.bindgroup1100.default,1\n",
    );
    write_shell(
        root.path(),
        "B",
        "charset,UTF-8\nsakura.bindgroup2200.default,1\n",
    );
    root
}

/// `shell/<name>/descript.txt` を書く。
fn write_shell(root: &Path, name: &str, descript_body: &str) {
    let dir = root.join("shell").join(name);
    fs::create_dir_all(&dir).expect("create shell/<name>");
    fs::write(dir.join("descript.txt"), descript_body.as_bytes()).expect("write shell descript");
}

/// 名前なしは今日の `resolve` と同じマウント（既定のシェル A と A の bindgroup）を返す。
#[test]
fn none_matches_todays_resolve() {
    let temp = ghost_with_two_shells("none_matches_today");
    let root = temp.path();

    let with_none = resolve_with_shell(root, DefaultEncoding::Utf8, None).expect("resolve ok");
    let today = resolve(root, DefaultEncoding::Utf8).expect("resolve ok");

    assert_eq!(with_none, today);
    assert_eq!(with_none.shell.dir, root.join("shell").join("A"));
    assert_eq!(with_none.bindgroups.sakura_default_on, vec![1100]);
}

/// 名前なしで `defaultsurfacedirectoryname` が無ければ `master` を選ぶ（今日の規則）。
#[test]
fn none_without_default_name_falls_back_to_master() {
    let temp = TempPath::new("parsers-resolve-shell-none-master");
    let root = temp.path();
    let master = root.join("ghost").join("master");
    fs::create_dir_all(&master).expect("create ghost/master");
    fs::write(
        master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\n".as_bytes(),
    )
    .expect("write ghost descript.txt");
    write_shell(root, "master", "charset,UTF-8\n");

    let model = resolve_with_shell(root, DefaultEncoding::Utf8, None).expect("resolve ok");

    assert_eq!(model.shell.dir, root.join("shell").join("master"));
}

/// 名前ありは `shell/B` と、B の `descript.txt` から読んだ bindgroup を返す。
#[test]
fn named_shell_mounts_its_dir_and_bindgroups() {
    let temp = ghost_with_two_shells("named_b");
    let root = temp.path();

    let model = resolve_with_shell(root, DefaultEncoding::Utf8, Some("B")).expect("resolve ok");

    assert_eq!(model.shell.dir, root.join("shell").join("B"));
    assert_eq!(model.bindgroups.sakura_default_on, vec![2200]);
    // シェル以外（名前・SHIORI）は既定のシェルのときと同じ。
    let today = resolve(root, DefaultEncoding::Utf8).expect("resolve ok");
    assert_eq!(model.names, today.names);
    assert_eq!(model.shiori, today.shiori);
}

/// 名前の先のフォルダが無ければ今日と同じ `ShellDirMissing`（既定のシェルへ黙って戻らない）。
#[test]
fn named_shell_missing_dir_is_shell_dir_missing() {
    let temp = ghost_with_two_shells("named_missing");
    let root = temp.path();

    let err = resolve_with_shell(root, DefaultEncoding::Utf8, Some("Z")).expect_err("must fail");

    assert_eq!(
        err,
        MountError::ShellDirMissing {
            expected: root.join("shell").join("Z"),
        }
    );
}
