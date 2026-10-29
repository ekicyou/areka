//! 記憶の置き場を決めるだけの口（`locate_app_profile`）の決定論テスト（spec: bridge-store-user-root
//! 要件 2.1〜2.9・4.4・6.1・6.4・6.7）。`Signals` を注入し、プロセスの環境変数を書き換えない。
//! 根は `target\` の下の偽の `LOCALAPPDATA` の `areka`、入力は winget の控え在りの形。
//! どの場面も、呼ぶ前と後で木（フォルダとファイルの中身）が同じことを確かめる。

use crate::{
    AppProfile, Decision, Layout, Signals, Writability, app_profile_dir, decide, locate_app_profile,
};
use log_capture_kit::{CapturedEvent, capture};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// 実行ファイルのフォルダ（隣に記憶 `profile\areka\sylphya.toml` 在り）・偽の `LOCALAPPDATA`・
/// まだ無い書ける根（`LOCALAPPDATA\areka`）。
fn fixture(label: &str) -> (TempPath, PathBuf, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(label);
    let exe = tmp.child("exe");
    fs::create_dir_all(profile(&exe)).unwrap();
    fs::write(profile(&exe).join("sylphya.toml"), "old = 1").unwrap();
    let lad = tmp.child("lad");
    fs::create_dir_all(&lad).unwrap();
    let root = lad.join("areka");
    (tmp, exe, lad, root)
}

/// winget の控え在りの入力（書ける根は `LOCALAPPDATA\areka`）。
fn winget_signals(lad: Option<&Path>) -> Signals {
    Signals {
        user_root_env: None,
        local_app_data: lad.map(OsString::from),
        winget_marker: Some("Areka.Areka.Portable_winget.db".to_owned()),
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

/// ログを捕まえながら 1 回呼び、呼ぶ前と後で木が同じことを確かめる。
fn run(
    tmp: &TempPath,
    signals: &Signals,
    exe: &Path,
    profile_env: Option<&OsStr>,
) -> (AppProfile, Vec<CapturedEvent>) {
    let before = tree(tmp.path());
    let (answer, events) = capture(|| locate_app_profile(signals, exe, profile_env));
    assert_eq!(tree(tmp.path()), before, "作らない・写さない・書き換えない");
    (answer, events)
}

/// 同じ入力での今の記憶の置き場（決めた根をそのまま `Layout` に入れる）。
fn current(signals: &Signals, exe: &Path, profile_env: Option<&OsStr>) -> Option<PathBuf> {
    let layout = match decide(signals) {
        Decision::UserRoot { root, reason } => Layout {
            user_root: Some(root),
            reason,
        },
        Decision::Beside { reason } | Decision::Fallback { reason, .. } => Layout {
            user_root: None,
            reason,
        },
    };
    app_profile_dir(profile_env, &layout, Some(exe))
}

/// `event` が `name` の記録。
fn logged<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

#[test]
fn absent_root_is_root_missing_and_not_created() {
    let (tmp, exe, lad, root) = fixture("ud-loc-absent");
    let (answer, _) = run(&tmp, &winget_signals(Some(&lad)), &exe, None);
    assert_eq!(answer, AppProfile::RootMissing { root: root.clone() });
    assert!(!root.exists(), "書ける根は作らない");
}

#[test]
fn present_root_without_memory_is_migration_pending_and_not_copied() {
    let (tmp, exe, lad, root) = fixture("ud-loc-pending");
    fs::create_dir_all(&root).unwrap();
    let (answer, _) = run(&tmp, &winget_signals(Some(&lad)), &exe, None);
    assert_eq!(answer, AppProfile::MigrationPending { root: root.clone() });
    assert!(!profile(&root).exists(), "根の profile\\areka は作らない");
}

#[test]
fn root_memory_present_is_ready_under_root() {
    let (tmp, exe, lad, root) = fixture("ud-loc-ready");
    fs::create_dir_all(profile(&root)).unwrap();
    fs::write(profile(&root).join("sylphya.toml"), "new = 2").unwrap();
    let signals = winget_signals(Some(&lad));
    let (answer, _) = run(&tmp, &signals, &exe, None);
    assert_eq!(answer, AppProfile::Ready(profile(&root)));
    assert_eq!(Some(profile(&root)), current(&signals, &exe, None));
}

#[test]
fn present_root_without_memory_anywhere_is_ready_under_root() {
    let (tmp, exe, lad, root) = fixture("ud-loc-ready-empty");
    fs::remove_dir_all(exe.join("profile")).unwrap();
    fs::create_dir_all(&root).unwrap();
    let signals = winget_signals(Some(&lad));
    let (answer, _) = run(&tmp, &signals, &exe, None);
    assert_eq!(answer, AppProfile::Ready(profile(&root)));
    assert_eq!(Some(profile(&root)), current(&signals, &exe, None));
    assert!(!profile(&root).exists(), "根の profile\\areka は作らない");
}

#[test]
fn zip_layout_is_ready_beside_exe() {
    let (tmp, exe, lad, root) = fixture("ud-loc-zip");
    let signals = Signals {
        winget_marker: None,
        ..winget_signals(Some(&lad))
    };
    let (answer, _) = run(&tmp, &signals, &exe, None);
    assert_eq!(answer, AppProfile::Ready(profile(&exe)));
    assert_eq!(Some(profile(&exe)), current(&signals, &exe, None));
    assert!(!root.exists());
}

#[test]
fn profile_env_is_ready_without_touching_root() {
    let (tmp, exe, lad, root) = fixture("ud-loc-profile-env");
    let dir = tmp.child("profile-env");
    let signals = winget_signals(Some(&lad));
    let (answer, _) = run(&tmp, &signals, &exe, Some(dir.as_os_str()));
    assert_eq!(answer, AppProfile::Ready(dir.clone()));
    assert_eq!(
        Some(dir.clone()),
        current(&signals, &exe, Some(dir.as_os_str()))
    );
    assert!(!root.exists(), "書ける根は作らない");
    assert!(!dir.exists(), "記憶の置き場も作らない");
}

#[test]
fn every_fallback_reason_is_unlocatable_with_the_current_logs() {
    let (tmp, exe, _lad, _root) = fixture("ud-loc-unlocatable");
    let cases = [
        (
            "env_empty",
            Signals {
                user_root_env: Some(OsString::new()),
                ..winget_signals(None)
            },
        ),
        (
            "env_relative",
            Signals {
                user_root_env: Some(OsString::from("rel\\root")),
                ..winget_signals(None)
            },
        ),
        ("local_app_data_missing", winget_signals(None)),
        (
            "local_app_data_relative",
            winget_signals(Some(Path::new("rel\\lad"))),
        ),
    ];
    for (kind, signals) in cases {
        assert!(
            matches!(decide(&signals), Decision::Fallback { .. }),
            "{kind}"
        );
        let (answer, events) = run(&tmp, &signals, &exe, None);
        assert_eq!(answer, AppProfile::Unlocatable, "{kind}");
        let errors = logged(&events, "user_root_unavailable");
        assert_eq!(errors.len(), 1, "{kind}");
        assert_eq!(errors[0].level, tracing::Level::ERROR, "{kind}");
        assert_eq!(errors[0].field_str("error"), Some(kind));
        let layouts = logged(&events, "user_data_layout");
        assert_eq!(layouts.len(), 1, "{kind}");
        assert_eq!(layouts[0].level, tracing::Level::INFO, "{kind}");
        assert_eq!(layouts[0].field_str("mode"), Some("beside"), "{kind}");
    }
}
