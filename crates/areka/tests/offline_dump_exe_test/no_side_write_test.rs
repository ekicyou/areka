//! 書かないこと: 書ける根・記憶の置き場・置き場の並び・既定の書ける根（`LOCALAPPDATA`）を
//! `target\` の下の空の根へ向けて回し、ゴーストのフォルダとそれらの木（名前・大きさ・更新時刻）が
//! 前後で同じで、出力先には PNG と CSV だけが書かれることを確かめる（要件 1.2〜1.4・9.1〜9.4・
//! 12.5・12.6）。

use std::collections::BTreeMap;
use std::path::Path;
use std::time::SystemTime;

use temp_path_kit::TempPath;

use crate::common::{BASE, file_names, run_areka_with_env, write_ghost};

/// 面 0 は土台だけ。面 999 は無い（Warning が 1 行＝CSV が書かれる）。
const SURFACES: &[&str] = &["surface0", "{", "element0,overlay,base.png,0,0", "}"];

/// 根からの相対の道 → （ファイルか・大きさ・更新時刻）。フォルダも 1 件に数える。
type Tree = BTreeMap<String, (bool, u64, SystemTime)>;

/// `dir` の下を辿って木を写す。根の直下の `skip` は辿らない。
fn snapshot(dir: &Path, skip: &str) -> Tree {
    let mut tree = Tree::new();
    walk(dir, dir, skip, &mut tree);
    tree
}

fn walk(root: &Path, dir: &Path, skip: &str, tree: &mut Tree) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("一覧の 1 件").path();
        let rel = path
            .strip_prefix(root)
            .expect("根の下")
            .display()
            .to_string();
        if rel == skip {
            continue;
        }
        let meta = std::fs::metadata(&path).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let mtime = meta.modified().expect("更新時刻");
        tree.insert(rel, (meta.is_file(), meta.len(), mtime));
        if meta.is_dir() {
            walk(root, &path, skip, tree);
        }
    }
}

#[test]
fn offline_dump_writes_nothing_but_png_and_csv() {
    let root = TempPath::under_target("offline-exe-no-side-write");
    let ghost = root.child("ghost-dir");
    write_ghost(&ghost, &["name,g_quiet"], SURFACES, &[BASE]);
    // 空の根（中身が増えれば木が変わる）。
    let user = root.child("user-root");
    let profile = root.child("profile");
    let roots = root.child("roots");
    let local_app_data = root.child("local-app-data");
    for dir in [&user, &profile, &roots, &local_app_data] {
        std::fs::create_dir_all(dir).expect("空の根の作成");
    }
    let out = root.child("out");
    let before = snapshot(root.path(), "out");

    let (code, stderr) = run_areka_with_env(
        root.path(),
        &[
            "--offline-dump",
            &ghost.display().to_string(),
            "--dump-surface-list",
            "0,999",
            "--dump-output-dir",
            &out.display().to_string(),
            "--dump-error-log",
            &out.join("errors.csv").display().to_string(),
        ],
        &[
            ("AREKA_USER_ROOT", user.as_os_str()),
            ("AREKA_PROFILE_DIR", profile.as_os_str()),
            ("AREKA_ROOT", roots.as_os_str()),
            ("LOCALAPPDATA", local_app_data.as_os_str()),
        ],
    );
    assert_eq!(code, 1, "Warning まで（無い面 999）なら 1: {stderr}");

    let after = snapshot(root.path(), "out");
    assert_eq!(
        after, before,
        "ゴーストのフォルダ・書ける根・記憶の置き場・置き場の並び・LOCALAPPDATA の木が変わらない"
    );
    assert_eq!(
        file_names(&out),
        ["errors.csv", "surface0.png"],
        "出力先には PNG と CSV だけ"
    );
    assert!(
        std::fs::read_dir(&out).expect("出力先").all(|e| e
            .expect("1 件")
            .file_type()
            .expect("種類")
            .is_file()),
        "出力先にフォルダを作らない"
    );
}
