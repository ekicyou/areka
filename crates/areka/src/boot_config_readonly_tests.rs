//! 読み取りだけの走りの記憶の置き場（readonly-mode タスク 3.4・要件 1.6・4.1・4.3・4.5）。
//!
//! 選び分けは旗を引数で受ける [`app_profile_dir_for`] で両方の腕を見る（プロセスの旗は立てない）。
//! 覚える所は試験の中の `OnceLock` を [`remembered`] へ渡して見る。根は `TempPath::under_target`。

use crate::boot_config::{app_profile_dir_for, remembered};
use areka_user_data::{Layout, Reason, UserRoot, UserRootOrigin};
use log_capture_kit::capture;
use std::path::PathBuf;
use std::sync::OnceLock;
use temp_path_kit::TempPath;

/// 書ける根は在るが中の記憶が無く、exe の隣に記憶が在る形（写すはずの場面）。
struct Fixture {
    _tmp: TempPath,
    layout: Layout,
    user: PathBuf,
    exe: PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let tmp = TempPath::under_target(label);
    let user = tmp.child("user");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&user).expect("書ける根");
    std::fs::create_dir_all(exe.join("profile").join("areka")).expect("隣の記憶");
    let layout = Layout {
        user_root: Some(UserRoot {
            dir: user.clone(),
            origin: UserRootOrigin::EnvVar,
        }),
        reason: Reason::EnvVar,
    };
    Fixture {
        _tmp: tmp,
        layout,
        user,
        exe,
    }
}

fn read_beside_count(events: &[log_capture_kit::CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("app_memory_read_beside"))
        .count()
}

#[test]
fn readonly_reads_the_bundled_memory_beside_the_exe() {
    let f = fixture("bc-ro-pick-on");
    let (dir, events) = capture(|| app_profile_dir_for(true, None, &f.layout, Some(&f.exe)));
    assert_eq!(dir, f.exe.join("profile").join("areka"));
    assert_eq!(read_beside_count(&events), 1);
    assert!(!f.user.join("profile").exists(), "写さない・作らない");
}

#[test]
fn normal_run_keeps_the_user_root_memory() {
    let f = fixture("bc-ro-pick-off");
    let (dir, events) = capture(|| app_profile_dir_for(false, None, &f.layout, Some(&f.exe)));
    assert_eq!(dir, f.user.join("profile").join("areka"));
    assert_eq!(read_beside_count(&events), 0);
}

#[test]
fn undecidable_place_falls_back_under_dot_on_both_branches() {
    let layout = Layout {
        user_root: None,
        reason: Reason::Neither,
    };
    let dot = std::path::Path::new(".").join("profile").join("areka");
    for readonly in [false, true] {
        assert_eq!(app_profile_dir_for(readonly, None, &layout, None), dot);
    }
}

#[test]
fn remembered_place_is_decided_once_and_logged_once() {
    let f = fixture("bc-ro-memo");
    let cell = OnceLock::new();
    let decide = || app_profile_dir_for(true, None, &f.layout, Some(&f.exe));
    let ((first, second), events) =
        capture(|| (remembered(&cell, decide), remembered(&cell, decide)));
    assert_eq!(first, second);
    assert_eq!(first, f.exe.join("profile").join("areka"));
    assert_eq!(read_beside_count(&events), 1);
}
