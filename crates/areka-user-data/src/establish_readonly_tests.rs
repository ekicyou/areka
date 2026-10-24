//! 読み取りだけの準備（`establish_readonly_from`）と記憶の置き場（`readonly_app_profile_dir`）の
//! 決定論テスト（spec: readonly-mode 要件 4.1〜4.3・4.5・10.5・11.2）。`Signals` を注入し、
//! 本物の `%LOCALAPPDATA%\areka` と実行ファイルの隣に触れない。根はすべて `target\` の下。
//! どの腕も、呼ぶ前と後で木（フォルダとファイルの中身）が同じことを確かめる。

use crate::{
    Layout, Reason, Signals, Writability, app_profile_dir, establish_readonly_from,
    readonly_app_profile_dir,
};
use log_capture_kit::{CapturedEvent, capture};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use temp_path_kit::TempPath;

/// 実行ファイルのフォルダ（隣に記憶 `profile\areka\sylphya.toml` 在り）と、まだ無い書ける根。
fn fixture(label: &str) -> (TempPath, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(label);
    let exe = tmp.child("exe");
    let beside = profile(&exe);
    fs::create_dir_all(beside.join("mcp-bridge")).unwrap();
    fs::write(beside.join("sylphya.toml"), "old = 1").unwrap();
    let root = tmp.child("user");
    (tmp, exe, root)
}

/// 書ける根を環境変数で指した入力（ほかの入力は空）。
fn env_signals(root: &Path) -> Signals {
    Signals {
        user_root_env: Some(OsString::from(root)),
        local_app_data: None,
        winget_marker: None,
        writability: Writability::Writable,
    }
}

fn profile(dir: &Path) -> PathBuf {
    dir.join("profile").join("areka")
}

/// 木の写し（相対パスと、ファイルなら中身）。並びは名前順。
fn tree(dir: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(dir).unwrap().to_path_buf();
            if path.is_dir() {
                stack.push(path);
                out.push((rel, None));
            } else {
                out.push((rel, Some(fs::read(&path).unwrap())));
            }
        }
    }
    out.sort();
    out
}

/// 準備と記憶の置き場を、ログを捕まえながら 1 回ずつ呼ぶ。
fn run(
    signals: &Signals,
    exe: &Path,
    profile_env: Option<&OsStr>,
) -> (Layout, Option<PathBuf>, Vec<CapturedEvent>) {
    let ((layout, dir), events) = capture(|| {
        let layout = establish_readonly_from(signals, Some(exe), profile_env);
        let dir = readonly_app_profile_dir(profile_env, &layout, Some(exe));
        (layout, dir)
    });
    (layout, dir, events)
}

/// `event` が `name` の記録。
fn logged<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 作らなかった・写さなかったの記録の数（`user_root_withheld`, `app_memory_read_beside`）。
fn withheld_counts(events: &[CapturedEvent]) -> (usize, usize) {
    (
        logged(events, "user_root_withheld").len(),
        logged(events, "app_memory_read_beside").len(),
    )
}

#[test]
fn absent_root_is_not_created_and_stays_beside() {
    let (tmp, exe, root) = fixture("ud-ro-absent");
    let before = tree(tmp.path());

    let (layout, dir, events) = run(&env_signals(&root), &exe, None);

    assert_eq!(layout.user_root, None);
    assert_eq!(layout.reason, Reason::EnvVar);
    assert!(!root.exists(), "書ける根は作らない");
    assert_eq!(withheld_counts(&events), (1, 0));
    let withheld = logged(&events, "user_root_withheld");
    assert_eq!(withheld[0].level, tracing::Level::INFO);
    assert_eq!(
        withheld[0].field("dir"),
        Some(root.display().to_string().as_str())
    );
    assert_eq!(dir, Some(profile(&exe)));
    assert_eq!(dir, app_profile_dir(None, &layout, Some(&exe)));
    assert_eq!(tree(tmp.path()), before);
}

#[test]
fn present_root_with_memory_is_used() {
    let (tmp, exe, root) = fixture("ud-ro-present");
    fs::create_dir_all(profile(&root)).unwrap();
    fs::write(profile(&root).join("sylphya.toml"), "new = 2").unwrap();
    let before = tree(tmp.path());

    let (layout, dir, events) = run(&env_signals(&root), &exe, None);

    assert_eq!(layout.user_root.as_ref().map(|r| &r.dir), Some(&root));
    assert_eq!(dir, Some(profile(&root)));
    assert_eq!(dir, app_profile_dir(None, &layout, Some(&exe)));
    assert_eq!(withheld_counts(&events), (0, 0));
    assert_eq!(tree(tmp.path()), before);
}

#[test]
fn present_root_without_memory_reads_beside_without_copying() {
    let (tmp, exe, root) = fixture("ud-ro-read-beside");
    fs::create_dir_all(&root).unwrap();
    let before = tree(tmp.path());

    let (layout, dir, events) = run(&env_signals(&root), &exe, None);

    assert_eq!(layout.user_root.as_ref().map(|r| &r.dir), Some(&root));
    assert_eq!(dir, Some(profile(&exe)), "写す元をそのまま読む");
    assert!(!profile(&root).exists(), "写さない");
    assert_eq!(withheld_counts(&events), (0, 1));
    let read = logged(&events, "app_memory_read_beside");
    assert_eq!(read[0].level, tracing::Level::INFO);
    assert_eq!(
        read[0].field("from"),
        Some(profile(&exe).display().to_string().as_str())
    );
    assert_eq!(
        read[0].field("to"),
        Some(profile(&root).display().to_string().as_str())
    );
    assert_eq!(tree(tmp.path()), before);
}

#[test]
fn neither_memory_gives_the_normal_place() {
    let tmp = TempPath::under_target("ud-ro-neither");
    let exe = tmp.child("exe");
    let root = tmp.child("user");
    fs::create_dir_all(&exe).unwrap();
    fs::create_dir_all(&root).unwrap();
    let before = tree(tmp.path());

    let (layout, dir, events) = run(&env_signals(&root), &exe, None);

    assert_eq!(dir, Some(profile(&root)));
    assert_eq!(dir, app_profile_dir(None, &layout, Some(&exe)));
    assert_eq!(withheld_counts(&events), (0, 0));
    assert_eq!(tree(tmp.path()), before);
}

#[test]
fn profile_env_wins() {
    let (tmp, exe, root) = fixture("ud-ro-profile-env");
    fs::create_dir_all(&root).unwrap();
    let elsewhere = tmp.child("elsewhere");
    let before = tree(tmp.path());

    let (_layout, dir, events) = run(&env_signals(&root), &exe, Some(elsewhere.as_os_str()));

    assert_eq!(dir, Some(elsewhere));
    assert_eq!(withheld_counts(&events), (0, 0));
    assert_eq!(tree(tmp.path()), before);
}

#[test]
fn copy_condition_is_shared() {
    let (_tmp, exe, root) = fixture("ud-ro-source");
    assert_eq!(
        super::migrate_source(&root, &exe),
        Some((profile(&exe), profile(&root)))
    );
    fs::create_dir_all(profile(&root)).unwrap();
    assert_eq!(
        super::migrate_source(&root, &exe),
        None,
        "写し先が在れば写さない"
    );
    assert_eq!(
        super::migrate_source(&root, &root.join("no-exe")),
        None,
        "写す元が無ければ写さない"
    );
}
