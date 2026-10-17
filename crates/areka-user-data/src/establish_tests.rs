//! 準備（`establish_from`）の引き継ぎと、控え・書けるかの読み口の決定論テスト
//! （要件 1.1・2.2・2.4・5.4・5.5・10.5・10.6）。`probe` を通さず、プロセスの環境変数を触らない。
//! 根はすべて `target\` の下。

use super::*;
use crate::app_profile_dir;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use temp_path_kit::TempPath;

/// 実行ファイルのフォルダ（隣に記憶 `profile\areka\sylphya.toml` 在り）と、まだ無い書ける根。
fn fixture(label: &str) -> (TempPath, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(label);
    let exe = tmp.child("exe");
    let beside = exe.join("profile").join("areka");
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

fn beside_unchanged(exe: &Path) {
    assert_eq!(
        fs::read_to_string(profile(exe).join("sylphya.toml")).unwrap(),
        "old = 1"
    );
    assert!(profile(exe).join("mcp-bridge").is_dir());
}

#[test]
fn beside_memory_is_migrated_once_and_beside_kept() {
    let (_tmp, exe, root) = fixture("ud-est-migrate");

    let layout = establish_from(&env_signals(&root), Some(&exe), None);

    assert_eq!(layout.user_root.as_ref().map(|r| &r.dir), Some(&root));
    assert_eq!(
        fs::read_to_string(profile(&root).join("sylphya.toml")).unwrap(),
        "old = 1"
    );
    assert!(profile(&root).join("mcp-bridge").is_dir());
    beside_unchanged(&exe);
    assert_eq!(
        app_profile_dir(None, &layout, Some(&exe)),
        Some(profile(&root))
    );
}

#[test]
fn memory_already_in_user_root_is_not_overwritten() {
    let (_tmp, exe, root) = fixture("ud-est-present");
    fs::create_dir_all(profile(&root)).unwrap();
    fs::write(profile(&root).join("sylphya.toml"), "new = 2").unwrap();

    establish_from(&env_signals(&root), Some(&exe), None);

    assert_eq!(
        fs::read_to_string(profile(&root).join("sylphya.toml")).unwrap(),
        "new = 2"
    );
    assert!(!profile(&root).join("mcp-bridge").exists());
    beside_unchanged(&exe);
}

#[test]
fn profile_env_skips_migration() {
    let (tmp, exe, root) = fixture("ud-est-profile-env");
    let elsewhere = tmp.child("elsewhere");

    let layout = establish_from(&env_signals(&root), Some(&exe), Some(elsewhere.as_os_str()));

    assert!(layout.user_root.is_some());
    assert!(root.is_dir(), "書ける根は作る");
    assert!(!profile(&root).exists());
    beside_unchanged(&exe);
}

#[test]
fn migration_failure_keeps_user_root_profile() {
    let (_tmp, exe, root) = fixture("ud-est-copy-fail");
    // 隣の記憶を共有なしで掴み、読めなくする。
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(profile(&exe).join("sylphya.toml"))
        .unwrap();

    let layout = establish_from(&env_signals(&root), Some(&exe), None);
    drop(held);

    assert_eq!(layout.user_root.as_ref().map(|r| &r.dir), Some(&root));
    assert_eq!(layout.reason, Reason::EnvVar);
    assert!(!profile(&root).exists(), "半端な写しは残らない");
    assert_eq!(
        app_profile_dir(None, &layout, Some(&exe)),
        Some(profile(&root))
    );
    beside_unchanged(&exe);
}

#[test]
fn uncreatable_user_root_falls_back_beside() {
    let (tmp, exe, _root) = fixture("ud-est-uncreatable");
    // ファイルの下にはフォルダを作れない。
    let file = tmp.child("a-file");
    fs::write(&file, "x").unwrap();
    let root = file.join("user");

    let layout = establish_from(&env_signals(&root), Some(&exe), None);

    assert_eq!(layout.user_root, None);
    assert_eq!(layout.reason, Reason::EnvVar);
    assert_eq!(
        app_profile_dir(None, &layout, Some(&exe)),
        Some(profile(&exe))
    );
    beside_unchanged(&exe);
}

#[test]
fn winget_marker_found_case_insensitively_files_only() {
    let tmp = TempPath::under_target("ud-est-marker");
    let exe = tmp.child("exe");
    fs::create_dir_all(exe.join("Areka.Areka.Portable_dir.db")).unwrap();
    fs::write(exe.join("areka.exe"), "").unwrap();
    assert_eq!(find_winget_marker(&exe), None);

    let marker = "areka.areka.portable__DefaultSource.DB";
    fs::write(exe.join(marker), "").unwrap();
    assert_eq!(find_winget_marker(&exe).as_deref(), Some(marker));

    assert_eq!(find_winget_marker(&tmp.child("missing")), None);
}

#[test]
fn writability_maps_os_result_and_creates_nothing() {
    let tmp = TempPath::under_target("ud-est-writable");
    let dir = tmp.child("exe");
    fs::create_dir_all(&dir).unwrap();

    assert_eq!(exe_dir_writability(&dir), Writability::Writable);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    assert!(matches!(
        exe_dir_writability(&tmp.child("missing")),
        Writability::Unknown(_)
    ));
}
