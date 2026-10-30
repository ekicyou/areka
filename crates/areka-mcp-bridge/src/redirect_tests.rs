//! 箱の見立て（`redirect`）の決定論テスト（spec: bridge-store-user-root 要件 3.5〜3.9・6.6・6.7）。
//!
//! `classify` は注入の値だけで全部の腕を固める（本物の箱はテストで作れない）。
//! `observe` は `target\` の下に組んだ偽の `LOCALAPPDATA` で 4 つの場面を確かめる。
//! プロセスの環境変数は書き換えない。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;

const B1: &str = "[bridge] writes under LOCALAPPDATA are redirected into the app's private folder (Microsoft Store app)";
const B2: &str = "[bridge] cannot tell whether writes under LOCALAPPDATA are redirected";

const LOCAL: &str = r"\\?\C:\Users\u\AppData\Local";
const ASKED: &str = r"C:\Users\u\AppData\Local\areka\mcp-bridge";
const BOX: &str = r"\\?\C:\Users\u\AppData\Local\Packages\Claude_x\LocalCache\Local";

fn rel() -> PathBuf {
    Path::new("areka").join("mcp-bridge")
}

fn sight(local: io::Result<PathBuf>, resolved: io::Result<PathBuf>) -> BoxSight {
    classify(&rel(), Path::new(ASKED), local, resolved)
}

fn ok(path: &str) -> io::Result<PathBuf> {
    Ok(PathBuf::from(path))
}

fn err(kind: io::ErrorKind) -> io::Result<PathBuf> {
    Err(io::Error::from(kind))
}

/// 記録を捕まえ、文と決まった語の欄が ASCII であることを確かめて返す（パスの欄は見ない・要件 3.8）。
fn reported(sight: &BoxSight) -> Vec<CapturedEvent> {
    let ((), events) = capture(|| report(sight));
    for event in &events {
        assert!(event.message().is_ascii(), "{}", event.message());
        if let Some(why) = event.field_str("why") {
            assert!(why.is_ascii(), "{why}");
        }
    }
    events
}

fn unknown(why: &'static str) -> BoxSight {
    BoxSight::Unknown {
        asked: Some(PathBuf::from(ASKED)),
        why,
    }
}

/// B2 が info で 1 件・`why` の欄が `why`・B1 は 0 件（要件 3.9）。
fn assert_b2(sight: &BoxSight, why: &str, asked: bool) {
    let events = reported(sight);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].level, Level::INFO);
    assert_eq!(events[0].message(), B2);
    assert_eq!(events[0].field_str("why"), Some(why));
    assert_eq!(events[0].field("asked").is_some(), asked);
}

#[test]
fn resolved_at_the_real_place_is_outside_and_reports_nothing() {
    let real = format!(r"{LOCAL}\areka\mcp-bridge");
    let got = sight(ok(LOCAL), ok(&real));
    assert_eq!(got, BoxSight::Outside);
    assert!(reported(&got).is_empty());
}

#[test]
fn resolved_into_the_box_is_inside_with_the_box_dir() {
    let resolved = format!(r"{BOX}\areka\mcp-bridge");
    let got = sight(ok(LOCAL), ok(&resolved));
    assert_eq!(
        got,
        BoxSight::Inside {
            asked: PathBuf::from(ASKED),
            resolved: PathBuf::from(&resolved),
            box_dir: Some(PathBuf::from(BOX)),
        }
    );
    let events = reported(&got);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].level, Level::INFO);
    assert_eq!(events[0].message(), B1);
    assert!(events[0].field("asked").is_some());
    assert!(events[0].field("resolved").is_some());
    assert_eq!(
        events[0].field("box_dir"),
        Some(Path::new(BOX).display().to_string().as_str())
    );
}

#[test]
fn box_dir_survives_case_only_differences_in_the_tail() {
    let resolved = format!(r"{BOX}\Areka\MCP-Bridge");
    let BoxSight::Inside { box_dir, .. } = sight(ok(LOCAL), ok(&resolved)) else {
        panic!("not inside");
    };
    assert_eq!(box_dir, Some(PathBuf::from(BOX)));
}

#[test]
fn inside_with_an_unmatched_tail_has_no_box_dir_field() {
    let resolved = format!(r"{BOX}\other\place");
    let got = sight(ok(LOCAL), ok(&resolved));
    let BoxSight::Inside { box_dir, .. } = &got else {
        panic!("not inside: {got:?}");
    };
    assert_eq!(*box_dir, None);
    let events = reported(&got);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].message(), B1);
    assert!(events[0].field("box_dir").is_none());
    assert!(events[0].field("resolved").is_some());
}

#[test]
fn case_only_differences_are_outside() {
    let got = sight(
        ok(LOCAL),
        ok(r"\\?\c:\USERS\u\appdata\LOCAL\Areka\MCP-bridge"),
    );
    assert_eq!(got, BoxSight::Outside);
}

#[test]
fn not_found_is_not_there_yet() {
    let got = sight(ok(LOCAL), err(io::ErrorKind::NotFound));
    assert_eq!(got, unknown("not_there_yet"));
    assert_b2(&got, "not_there_yet", true);
}

#[test]
fn other_failures_cannot_resolve() {
    let asked_fails = sight(ok(LOCAL), err(io::ErrorKind::PermissionDenied));
    assert_eq!(asked_fails, unknown("cannot_resolve"));
    assert_b2(&asked_fails, "cannot_resolve", true);
    let real = format!(r"{LOCAL}\areka\mcp-bridge");
    let local_fails = sight(err(io::ErrorKind::PermissionDenied), ok(&real));
    assert_eq!(local_fails, unknown("cannot_resolve"));
}

#[test]
fn no_cache_dir_reports_without_the_asked_field() {
    let got = observe(Some(Path::new(r"C:\Users\u\AppData\Local")), None);
    assert_eq!(
        got,
        BoxSight::Unknown {
            asked: None,
            why: "no_cache_dir"
        }
    );
    assert_b2(&got, "no_cache_dir", false);
}

/// 偽の `LOCALAPPDATA`（`target\` の下）と、その下の控えのフォルダ（まだ作らない）。
fn fake_local(label: &str) -> (TempPath, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(label);
    let lad = tmp.child("lad");
    fs::create_dir_all(&lad).unwrap();
    let cache = lad.join("areka").join("mcp-bridge");
    (tmp, lad, cache)
}

#[test]
fn observe_existing_cache_dir_is_outside() {
    let (_tmp, lad, cache) = fake_local("redirect-observe-outside");
    fs::create_dir_all(&cache).unwrap();
    assert_eq!(observe(Some(&lad), Some(&cache)), BoxSight::Outside);
}

#[test]
fn observe_missing_cache_dir_is_not_there_yet_and_creates_nothing() {
    let (_tmp, lad, cache) = fake_local("redirect-observe-not-yet");
    assert_eq!(
        observe(Some(&lad), Some(&cache)),
        BoxSight::Unknown {
            asked: Some(cache.clone()),
            why: "not_there_yet"
        }
    );
    assert!(!cache.exists());
    assert!(!lad.join("areka").exists());
}

#[test]
fn observe_cache_dir_outside_local_app_data_is_not_under() {
    let (tmp, lad, _) = fake_local("redirect-observe-not-under");
    let elsewhere = tmp.child("elsewhere").join("mcp-bridge");
    let want = BoxSight::Unknown {
        asked: Some(elsewhere.clone()),
        why: "not_under_local_app_data",
    };
    assert_eq!(observe(Some(&lad), Some(&elsewhere)), want);
    assert_eq!(observe(None, Some(&elsewhere)), want);
    assert_eq!(observe(Some(Path::new("lad")), Some(&elsewhere)), want);
    // 相対の部分が空（控えのフォルダが `LOCALAPPDATA` そのもの）も下ではない。
    assert_eq!(
        observe(Some(&lad), Some(&lad)),
        BoxSight::Unknown {
            asked: Some(lad.clone()),
            why: "not_under_local_app_data",
        }
    );
    assert_b2(&want, "not_under_local_app_data", true);
}
