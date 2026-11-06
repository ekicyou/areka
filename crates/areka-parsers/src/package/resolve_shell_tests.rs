//! resolve_shell_tests — シェル名つきの解決 `resolve_with_shell` の兄弟テスト。
//!
//! 名前なし（`None`）は今日の規則（`seriko.defaultsurfacedirectoryname` → `master`）と
//! 同じマウントを返し、名前あり（`Some("B")`）は `shell/B` と B の `descript.txt` の
//! bindgroup を返し、名前の先が無ければ今日と同じ `ShellDirMissing` になることを固定する。
//! ゴーストの名前だけを読む `read_ghost_names` は単独で名前を返し、起点の不在は
//! `StartPointMissing` になることも固定する。
//! 一時ツリーは共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意・破棄で消える）。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use crate::charset::DefaultEncoding;

use super::{GhostNames, MountError, read_ghost_names, resolve_with_shell, shiori_dir};

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

/// 名前なしは既定のシェル（`seriko.defaultsurfacedirectoryname` の A）と A の bindgroup を返す。
#[test]
fn none_uses_default_named_shell() {
    let temp = ghost_with_two_shells("none_matches_today");
    let root = temp.path();

    let with_none = resolve_with_shell(root, DefaultEncoding::Utf8, None).expect("resolve ok");
    assert_eq!(with_none.shell.dir, root.join("shell").join("A"));
    assert_eq!(with_none.bindgroups[&0].default_on, vec![1100]);
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
    assert_eq!(model.bindgroups[&0].default_on, vec![2200]);
    // シェル以外（名前・SHIORI）は既定のシェルのときと同じ。
    let today = resolve_with_shell(root, DefaultEncoding::Utf8, None).expect("resolve ok");
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

/// ghost/master/descript.txt だけを `body` で書いた一時ツリーを作る（シェルは置かない）。
fn ghost_only(tag: &str, body: &str) -> TempPath {
    let root = TempPath::new(&format!("parsers-ghost-names-{tag}"));
    let master = root.path().join("ghost").join("master");
    fs::create_dir_all(&master).expect("create ghost/master");
    fs::write(master.join("descript.txt"), body.as_bytes()).expect("write ghost descript.txt");
    root
}

/// 名前の在るゴーストは、シェルが無くても descript.txt の名前を単独で返す。
#[test]
fn read_ghost_names_returns_names_without_shell() {
    let temp = ghost_only(
        "present",
        "charset,UTF-8
name,テスト
sakura.name,さくら
sakura.name2,さくら2
kero.name,うにゅう
",
    );

    let names = read_ghost_names(temp.path(), DefaultEncoding::Utf8).expect("read ok");

    assert_eq!(names.name.as_deref(), Some("テスト"));
    assert_eq!(names.sakura_name.as_deref(), Some("さくら"));
    assert_eq!(names.sakura_name2.as_deref(), Some("さくら2"));
    assert_eq!(names.kero_name.as_deref(), Some("うにゅう"));
}

/// `name` の無いゴーストは失敗せず、空の名前（全部 None）を返す。
#[test]
fn read_ghost_names_without_name_is_empty() {
    let temp = ghost_only(
        "empty",
        "charset,UTF-8
shiori,x.dll
",
    );

    let names = read_ghost_names(temp.path(), DefaultEncoding::Utf8).expect("read ok");

    assert_eq!(names, GhostNames::default());
}

/// descript.txt が無ければ `StartPointMissing`。
#[test]
fn read_ghost_names_missing_descript_is_start_point_missing() {
    let temp = TempPath::new("parsers-ghost-names-no-descript");
    let master = temp.path().join("ghost").join("master");
    fs::create_dir_all(&master).expect("create ghost/master");

    let err = read_ghost_names(temp.path(), DefaultEncoding::Utf8).expect_err("must fail");

    assert_eq!(
        err,
        MountError::StartPointMissing {
            expected: master.join("descript.txt"),
        }
    );
}

/// ゴーストのフォルダが無ければ `StartPointMissing`。
#[test]
fn read_ghost_names_missing_folder_is_start_point_missing() {
    let temp = TempPath::new("parsers-ghost-names-no-folder");
    let root = temp.path().join("nothing-here");

    let err = read_ghost_names(&root, DefaultEncoding::Utf8).expect_err("must fail");

    assert_eq!(
        err,
        MountError::StartPointMissing {
            expected: root.join("ghost").join("master").join("descript.txt"),
        }
    );
}

/// `seriko.defaultsurfacedirectoryname` を `value` にした ghost/master と、`shell/master`
/// を持つ一時ツリーを作る。
fn ghost_with_default_dir_name(tag: &str, value: &str) -> TempPath {
    let root = TempPath::new(&format!("parsers-resolve-shell-dirname-{tag}"));
    let master = root.path().join("ghost").join("master");
    fs::create_dir_all(&master).expect("create ghost/master");
    fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\nname,テスト\nseriko.defaultsurfacedirectoryname,{value}\n")
            .as_bytes(),
    )
    .expect("write ghost descript.txt");
    write_shell(root.path(), "master", "charset,UTF-8\n");
    root
}

/// 名前なしの解決を記録つきで回し、選んだシェルの dir と `warn!` の件を返す。
fn resolve_default_capturing(
    root: &Path,
) -> (std::path::PathBuf, Vec<log_capture_kit::CapturedEvent>) {
    let (model, events) =
        log_capture_kit::capture(|| resolve_with_shell(root, DefaultEncoding::Utf8, None));
    let warns = events
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    (model.expect("resolve ok").shell.dir, warns)
}

/// 既定のシェル名が親へ逃げる値なら、逃げた先が在っても使わず `warn!` を残して `master`。
#[test]
fn default_dir_name_parent_escape_falls_back_to_master() {
    let temp = ghost_with_default_dir_name("parent", "../x");
    let root = temp.path();
    fs::create_dir_all(root.join("x")).expect("create escape target");

    let (dir, warns) = resolve_default_capturing(root);

    assert_eq!(dir, root.join("shell").join("master"));
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert_eq!(warns[0].field("value"), Some("../x"), "{warns:?}");
    assert_eq!(
        warns[0].field("ghost_root"),
        Some(root.display().to_string().as_str()),
        "{warns:?}"
    );
}

/// 既定のシェル名が絶対パスなら、その先が在っても使わず `warn!` を残して `master`。
#[test]
fn default_dir_name_absolute_falls_back_to_master() {
    let probe = TempPath::new("parsers-resolve-shell-dirname-absolute-target");
    fs::create_dir_all(probe.path()).expect("create absolute target");
    let absolute = probe.path().display().to_string();
    let temp = ghost_with_default_dir_name("absolute", &absolute);
    let root = temp.path();

    let (dir, warns) = resolve_default_capturing(root);

    assert_eq!(dir, root.join("shell").join("master"));
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert_eq!(
        warns[0].field("value"),
        Some(absolute.as_str()),
        "{warns:?}"
    );
}

/// 空の値は「無し」と同じに黙って `master`、`.` は確かめに通らず `warn!` を残して `master`。
#[test]
fn default_dir_name_empty_or_dot_falls_back_to_master() {
    let empty = ghost_with_default_dir_name("empty", "");
    let (dir, warns) = resolve_default_capturing(empty.path());
    assert_eq!(dir, empty.path().join("shell").join("master"));
    assert!(warns.is_empty(), "{warns:?}");

    let dot = ghost_with_default_dir_name("dot", ".");
    let (dir, warns) = resolve_default_capturing(dot.path());
    assert_eq!(dir, dot.path().join("shell").join("master"));
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert_eq!(warns[0].field("value"), Some("."), "{warns:?}");
}

/// ゴーストの descript の鍵は大文字と小文字を区別しない（名前・SHIORI・既定のシェル名）。
#[test]
fn ghost_descript_keys_ignore_case() {
    let temp = ghost_only(
        "case",
        "charset,UTF-8
Name,テスト
Sakura.Name,さくら
KERO.NAME,うにゅう
Shiori,x.dll
SERIKO.DefaultSurfaceDirectoryName,B
",
    );
    write_shell(temp.path(), "B", "charset,UTF-8\n");

    let model = resolve_with_shell(temp.path(), DefaultEncoding::Utf8, None).expect("resolve ok");

    assert_eq!(model.names.name.as_deref(), Some("テスト"));
    assert_eq!(model.names.sakura_name.as_deref(), Some("さくら"));
    assert_eq!(model.names.kero_name.as_deref(), Some("うにゅう"));
    assert_eq!(model.shiori.file.as_deref(), Some("x.dll"));
    assert_eq!(model.shell.dir, temp.path().join("shell").join("B"));
}

/// ゴーストの descript の空の値は「無し」と同じ（`Some("")` を返さない）。
#[test]
fn ghost_descript_empty_values_are_absent() {
    let temp = ghost_only(
        "empty-values",
        "charset,UTF-8
name,
sakura.name,
shiori,
shiori.encoding,
readme,
",
    );
    write_shell(temp.path(), "master", "charset,UTF-8\n");

    let model = resolve_with_shell(temp.path(), DefaultEncoding::Utf8, None).expect("resolve ok");

    assert_eq!(model.names, GhostNames::default());
    assert_eq!(model.shiori.file, None);
    assert_eq!(model.shiori.encoding, None);
    assert_eq!(model.readme, None);
}

/// SHIORI の置き場は `ghost` と `master` をネイティブの区切りで組む（`ghost/master` の
/// 1 成分で継がない）。バイト列で比べる（`Path` の `==` は区切りの違いを見逃す・指摘 10）。
#[test]
fn shiori_dir_uses_native_separators() {
    let root = Path::new("root");
    assert_eq!(
        shiori_dir(root).as_os_str(),
        root.join("ghost").join("master").as_os_str()
    );

    let temp = ghost_only("native-separators", "charset,UTF-8\nname,テスト\n");
    write_shell(temp.path(), "master", "charset,UTF-8\n");
    let model = resolve_with_shell(temp.path(), DefaultEncoding::Utf8, None).expect("resolve ok");
    assert_eq!(
        model.shiori.dir.as_os_str(),
        temp.path().join("ghost").join("master").as_os_str()
    );
}

/// ghost/master の descript.txt を `body` にし、`shell/master` を持つ一時ツリーで名前なしの解決を
/// 記録つきで回し、マウントと `warn!` の件を返す。
fn resolve_descript_capturing(
    tag: &str,
    body: &str,
) -> (
    TempPath,
    super::MountModel,
    Vec<log_capture_kit::CapturedEvent>,
) {
    let root = TempPath::new(&format!("parsers-resolve-shell-outside-{tag}"));
    let master = root.path().join("ghost").join("master");
    fs::create_dir_all(&master).expect("create ghost/master");
    fs::write(master.join("descript.txt"), body.as_bytes()).expect("write ghost descript.txt");
    write_shell(root.path(), "master", "charset,UTF-8\n");
    let (model, events) =
        log_capture_kit::capture(|| resolve_with_shell(root.path(), DefaultEncoding::Utf8, None));
    let warns = events
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    (root, model.expect("resolve ok"), warns)
}

/// R2: ゴーストのフォルダの外を指す `readme`（`..`・絶対パス・ドライブ）は値が無いのと同じに
/// 扱い（説明書は既定の `readme.txt` へ落ちる）、値を `warn!` に残す。
#[test]
fn readme_pointing_outside_ghost_is_dropped_with_warn() {
    for (tag, value) in [
        ("parent", r"..\..\secret.txt"),
        ("parent-slash", "../x.txt"),
        ("rooted", r"\Windows\win.ini"),
        ("drive", r"C:\Windows\win.ini"),
        ("unc", r"\\server\share\x.txt"),
    ] {
        let (_root, model, warns) =
            resolve_descript_capturing(tag, &format!("charset,UTF-8\nreadme,{value}\n"));

        assert_eq!(model.readme, None, "{value}");
        assert_eq!(warns.len(), 1, "{value}: {warns:?}");
        assert_eq!(warns[0].field("value"), Some(value), "{warns:?}");
    }
}

/// R2: ゴーストのフォルダの中のサブフォルダを指す `readme` はそのまま使い、記録しない。
#[test]
fn readme_inside_ghost_subfolder_is_kept() {
    let (_root, model, warns) =
        resolve_descript_capturing("inside", "charset,UTF-8\nreadme,doc/manual.txt\n");

    assert_eq!(model.readme.as_deref(), Some("doc/manual.txt"));
    assert!(warns.is_empty(), "{warns:?}");
}

/// R2: ghost/master の外を指す `shiori` は記録だけ——値はそのまま使う（共有の DLL を持つ
/// ゴーストを起動できなくしない）。
#[test]
fn shiori_pointing_outside_is_kept_with_warn() {
    let value = r"..\shared\x.dll";
    let (_root, model, warns) =
        resolve_descript_capturing("shiori", &format!("charset,UTF-8\nshiori,{value}\n"));

    assert_eq!(model.shiori.file.as_deref(), Some(value));
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert_eq!(warns[0].field("value"), Some(value), "{warns:?}");
}

/// R2: ghost/master の中の `shiori` は記録しない。
#[test]
fn shiori_inside_is_not_warned() {
    let (_root, model, warns) =
        resolve_descript_capturing("shiori-inside", "charset,UTF-8\nshiori,yaya.dll\n");

    assert_eq!(model.shiori.file.as_deref(), Some("yaya.dll"));
    assert!(warns.is_empty(), "{warns:?}");
}
