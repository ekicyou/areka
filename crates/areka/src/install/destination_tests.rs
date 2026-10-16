//! 並びでインストールの宛先の根に答える部品の決定論テスト（areka-P0-baseware-root-list task 4.5）。
//!
//! 確かめること: 新しいものは先頭の根・すでにあるものはその根・先頭の根が見つからないときは
//! 見つかるいちばん上と warn `install_head_root_skipped`・見つかる根が無いときは作れない
//! （要件 5.1・5.2・5.3・5.6・12.4）。答えは並びの写しの綴り（tasks.md Implementation Notes 1.5）。

use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_nar::InstallRoots;
use temp_path_kit::TempPath;

use super::*;
use crate::placement::test_support::capture_logs;

fn put_ghost(root: &Path, folder: &str) {
    let master = root.join("ghost").join(folder).join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\nname,X\n").expect("descript");
}

fn put_balloon(root: &Path, folder: &str) {
    let dir = root.join("balloon").join(folder);
    std::fs::create_dir_all(&dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), "charset,UTF-8\nname,B\n").expect("descript");
}

fn list(dirs: &[PathBuf]) -> BasewareRoots {
    BasewareRoots::new(dirs.iter().cloned().map(BasewareRoot::new).collect())
}

fn skipped_warns(events: &[log_capture_kit::CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_head_root_skipped"))
        .count()
}

#[test]
fn new_items_go_to_head_root_and_existing_ones_to_their_root() {
    let tmp = TempPath::under_target("install-destination-two-roots");
    let (one, two) = (tmp.child("one"), tmp.child("two"));
    std::fs::create_dir_all(&one).expect("根 1");
    put_ghost(&two, "Below");
    put_balloon(&two, "bln");
    let dest = CatalogDestinations::new(list(&[one.clone(), two.clone()])).expect("作れる");
    let (answers, events) = capture_logs(|| {
        [
            dest.ghost_root("Fresh"),
            dest.ghost_root("below"),
            dest.balloon_root("Fresh"),
            dest.balloon_root("BLN"),
        ]
    });
    assert_eq!(
        answers,
        [one.clone(), two.clone(), one, two],
        "(新しいゴースト→先頭・すでにあるゴースト→その根・新しいバルーン→先頭・すでにあるバルーン→その根)"
    );
    assert_eq!(skipped_warns(&events), 0, "先頭が見つかるなら warn は無い");
}

#[test]
fn missing_head_root_answers_topmost_found_and_warns() {
    let tmp = TempPath::under_target("install-destination-head-missing");
    let (gone, two, three) = (tmp.child("gone"), tmp.child("two"), tmp.child("three"));
    std::fs::create_dir_all(&two).expect("根 2");
    std::fs::create_dir_all(&three).expect("根 3");
    let dest = CatalogDestinations::new(list(&[gone.clone(), two.clone(), three])).expect("作れる");
    let (answer, events) = capture_logs(|| dest.ghost_root("Fresh"));
    assert_eq!(answer, two, "見つかるいちばん上");
    let warn = events
        .iter()
        .find(|e| e.field_str("event") == Some("install_head_root_skipped"))
        .expect("warn が残る");
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(
        (warn.field("head_root"), warn.field("root")),
        (
            Some(gone.display().to_string().as_str()),
            Some(two.display().to_string().as_str())
        )
    );
}

#[test]
fn cannot_be_built_without_any_found_root() {
    let tmp = TempPath::under_target("install-destination-none");
    assert!(CatalogDestinations::new(list(&[tmp.child("a"), tmp.child("b")])).is_none());
}

#[test]
fn answers_built_root_when_all_roots_vanish_after_build() {
    let tmp = TempPath::under_target("install-destination-vanish");
    let one = tmp.child("one");
    std::fs::create_dir_all(&one).expect("根 1");
    let dest = CatalogDestinations::new(list(std::slice::from_ref(&one))).expect("作れる");
    std::fs::remove_dir_all(&one).expect("根を消す");
    assert_eq!(dest.balloon_root("Fresh"), one);
}
