//! 準備（`establish_from`）の引き継ぎと、控え・書けるかの読み口の決定論テスト
//! （要件 1.1・2.2・2.4・5.4・5.5・10.5・10.6）。`probe` を通さず、プロセスの環境変数を触らない。
//! 根はすべて `target\` の下。

use super::*;
use crate::app_profile_dir;
use crate::test_support::{env_signals, fixture, logged, profile};
use log_capture_kit::capture;
use std::fs;
use temp_path_kit::TempPath;

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
fn establish_still_migrates_when_the_profile_env_is_empty() {
    let (_tmp, exe, root) = fixture("ud-est-profile-env-empty");
    let empty = OsStr::new("");

    let (layout, events) = capture(|| establish_from(&env_signals(&root), Some(&exe), Some(empty)));

    assert_eq!(
        fs::read_to_string(profile(&root).join("sylphya.toml")).unwrap(),
        "old = 1",
        "空の値でも隣の記憶を写す"
    );
    assert_eq!(
        app_profile_dir(Some(empty), &layout, Some(&exe)),
        Some(profile(&root))
    );
    let errors = logged(&events, "profile_dir_env_unusable");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].level, tracing::Level::ERROR);
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

    let (layout, events) = capture(|| establish_from(&env_signals(&root), Some(&exe), None));
    drop(held);

    let errors = logged(&events, "app_memory_migrate_failed");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].level, tracing::Level::ERROR);

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

    let (layout, events) = capture(|| establish_from(&env_signals(&root), Some(&exe), None));

    let errors = logged(&events, "user_root_unavailable");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].level, tracing::Level::ERROR);
    assert_eq!(errors[0].field_str("reason"), Some("create_failed"));

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
