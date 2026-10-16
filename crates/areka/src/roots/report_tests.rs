//! 並び・見つからない根・隠れの記録の決定論テスト（要件 1.15・2.5・8.3・9.1・12.2）。
//!
//! 値だけを組んで渡し、ログの捕捉（`log-capture-kit`）で行を数える。fs は見ない。

use super::*;
use crate::roots::{EXE_TOKEN, RootStatus};
use areka_ghost::{BasewareRoot, BasewareRoots, Hidden, StoreKind};
use log_capture_kit::{CapturedEvent, capture};
use std::path::PathBuf;

fn of<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn hidden(folder: &str) -> Hidden {
    Hidden {
        kind: StoreKind::Ghost,
        folder: folder.to_owned(),
        hidden: PathBuf::from(format!(r"C:\b\ghost\{folder}")),
        by: PathBuf::from(format!(r"C:\a\ghost\{folder}")),
    }
}

fn resolved(found: &[(&str, bool)], skipped: Vec<RootSkip>, fell_back: bool) -> ResolvedRoots {
    ResolvedRoots {
        roots: BasewareRoots::new(
            found
                .iter()
                .map(|(d, _)| BasewareRoot::new(PathBuf::from(d)))
                .collect(),
        ),
        origin: RootsOrigin::Memory,
        status: found
            .iter()
            .map(|(d, f)| RootStatus {
                dir: PathBuf::from(d),
                found: *f,
            })
            .collect(),
        skipped,
        memory_fell_back: fell_back,
    }
}

#[test]
fn hidden_items_one_line_each_plus_count() {
    let ((), events) = capture(|| log_hidden(&[hidden("a"), hidden("b")], ReportOccasion::Change));
    let items = of(&events, "root_hidden_item");
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|e| e.level == tracing::Level::INFO));
    assert_eq!(items[0].field_str("folder"), Some("a"));
    assert_eq!(items[0].field_str("kind"), Some("ghost"));
    assert_eq!(items[0].field_str("occasion"), Some("change"));
    let count = of(&events, "roots_hidden_count");
    assert_eq!(count.len(), 1);
    assert_eq!(count[0].field("count"), Some("2"));
}

#[test]
fn hidden_count_line_even_when_zero() {
    let ((), events) = capture(|| log_hidden(&[], ReportOccasion::Boot));
    assert!(of(&events, "root_hidden_item").is_empty());
    let count = of(&events, "roots_hidden_count");
    assert_eq!(count.len(), 1);
    assert_eq!(count[0].level, tracing::Level::INFO);
    assert_eq!(count[0].field("count"), Some("0"));
    assert_eq!(count[0].field_str("occasion"), Some("boot"));
}

#[test]
fn each_root_listed_and_missing_root_warns() {
    let r = resolved(&[(r"C:\a", true), (r"C:\b", false)], Vec::new(), false);
    let ((), events) = capture(|| log_roots(&r, ReportOccasion::Boot));
    let listed = of(&events, "root_listed");
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|e| e.level == tracing::Level::INFO));
    assert_eq!(listed[1].field("path"), Some(r"C:\b"));
    assert_eq!(listed[1].field("found"), Some("false"));
    assert_eq!(listed[1].field_str("origin"), Some("memory"));
    let missing = of(&events, "root_missing");
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].level, tracing::Level::WARN);
    assert_eq!(missing[0].field("path"), Some(r"C:\b"));
    assert!(of(&events, "root_skipped").is_empty());
    assert!(of(&events, "roots_memory_fell_back").is_empty());
}

#[test]
fn skips_and_fell_back_warn() {
    let skipped = vec![
        RootSkip::Empty { index: 1 },
        RootSkip::Unreadable {
            item: "rel".into(),
            reason: "relative",
        },
        RootSkip::Duplicate {
            dir: PathBuf::from(r"C:\A\"),
            kept: PathBuf::from(r"C:\a"),
        },
    ];
    let r = resolved(&[(r"C:\a", true)], skipped, true);
    let ((), events) = capture(|| log_roots(&r, ReportOccasion::Boot));
    let skips = of(&events, "root_skipped");
    assert_eq!(skips.len(), 3);
    assert!(skips.iter().all(|e| e.level == tracing::Level::WARN));
    assert_eq!(skips[1].field_str("reason"), Some("relative"));
    assert_eq!(skips[2].field_str("reason"), Some("duplicate"));
    let fell = of(&events, "roots_memory_fell_back");
    assert_eq!(fell.len(), 1);
    assert_eq!(fell[0].level, tracing::Level::WARN);
}

#[test]
fn failure_value_still_warns_skips_and_fell_back() {
    let err = RootError::ExeLocationUnavailable {
        tried: Vec::new(),
        origin: RootsOrigin::Default,
        skipped: vec![RootSkip::Unreadable {
            item: EXE_TOKEN.into(),
            reason: "exe_unavailable",
        }],
        memory_fell_back: true,
    };
    let ((), events) = capture(|| log_root_error(&err, ReportOccasion::Boot));
    let skips = of(&events, "root_skipped");
    assert_eq!(skips.len(), 1);
    assert_eq!(skips[0].level, tracing::Level::WARN);
    assert_eq!(skips[0].field_str("reason"), Some("exe_unavailable"));
    assert_eq!(of(&events, "roots_memory_fell_back").len(), 1);
}

#[test]
fn only_marked_roots_carry_user_root_origin() {
    let dirs = [r"C:\u", r"C:\e", r"C:\p"];
    let r = ResolvedRoots {
        roots: BasewareRoots::new(vec![
            BasewareRoot::user_root(PathBuf::from(dirs[0]), false),
            BasewareRoot::user_root(PathBuf::from(dirs[1]), true),
            BasewareRoot::new(PathBuf::from(dirs[2])),
        ]),
        origin: RootsOrigin::Default,
        status: dirs
            .iter()
            .map(|d| RootStatus {
                dir: PathBuf::from(d),
                found: true,
            })
            .collect(),
        skipped: Vec::new(),
        memory_fell_back: false,
    };
    let ((), events) = capture(|| log_roots(&r, ReportOccasion::Change));
    let listed = of(&events, "root_listed");
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].field_str("user_root"), Some("default"));
    assert_eq!(listed[1].field_str("user_root"), Some("env_var"));
    assert_eq!(listed[2].field_str("user_root"), Some(""));
}
