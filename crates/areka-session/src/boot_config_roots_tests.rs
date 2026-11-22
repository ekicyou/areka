//! 1 ゴースト分のバルーンを決める口（`resolve_balloon_for_ghost`）を 2 つの根の和の上で踏む。
//!
//! 確かめること: 同梱のバルーンの引き当て・記憶のフォルダ名からの引き当て（大文字小文字だけ違う
//! 綴りでも当たる）・どの根にも無いときの次の候補と記録（値は格納フォルダの列を `; ` でつないだもの）。
//! 根は `TempPath::under_target`。

use super::balloon_tests::put_memory;
use crate::boot_config::resolve_balloon_for_ghost;
use crate::boot_resolve::boot_resolve_tests::no_pick;
use crate::boot_resolve::{BalloonRoute, DEFAULT_BALLOON_FOLDER};
use areka_ghost::{BasewareRoot, BasewareRoots};
use log_capture_kit::{CapturedEvent, capture};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// 上の根 `top` と下の根 `low`（どちらも `ghost\`・`balloon\` を持つ）。
fn two_roots(tmp: &TempPath) -> (PathBuf, PathBuf, BasewareRoots) {
    let (top, low) = (tmp.child("top"), tmp.child("low"));
    for root in [&top, &low] {
        std::fs::create_dir_all(root.join("ghost")).expect("フォルダを組む");
        std::fs::create_dir_all(root.join("balloon")).expect("フォルダを組む");
    }
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(top.clone()),
        BasewareRoot::new(low.clone()),
    ]);
    (top, low, roots)
}

fn put_ghost(root: &Path, folder: &str, descript: &str) -> PathBuf {
    let dir = root.join("ghost").join(folder);
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\n{descript}"),
    )
    .expect("descript");
    dir
}

fn put_balloon(root: &Path, folder: &str) -> PathBuf {
    let dir = root.join("balloon").join(folder);
    std::fs::create_dir_all(&dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    dir
}

fn named<'e>(events: &'e [CapturedEvent], event: &str) -> Vec<&'e CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// 同梱のバルーンが下の根にだけ在る → 同梱の段で下の根の項目。
#[test]
fn companion_balloon_is_found_in_the_lower_root() {
    let tmp = TempPath::under_target("cfg-roots-companion");
    let (top, low, roots) = two_roots(&tmp);
    let ghost = put_ghost(&top, "g", "");
    std::fs::write(
        ghost.join("install.txt"),
        "charset,UTF-8\nballoon.directory,bundled\n",
    )
    .expect("install.txt");
    put_balloon(&top, DEFAULT_BALLOON_FOLDER);
    let bundled = put_balloon(&low, "bundled");
    let (got, events) = capture(|| resolve_balloon_for_ghost(&roots, &ghost, no_pick));
    let got = got.expect("決まる");
    assert_eq!(
        (got.route, got.dir, got.folder.as_deref()),
        (BalloonRoute::Companion, bundled, Some("bundled"))
    );
    assert!(named(&events, "companion_balloon_not_found").is_empty());
}

/// 記憶のフォルダ名が大文字小文字だけ違っても下の根の項目に当たり、フォルダ名は項目の綴り。
#[test]
fn memory_folder_name_hits_by_the_one_rule_across_roots() {
    let tmp = TempPath::under_target("cfg-roots-memory");
    let (top, low, roots) = two_roots(&tmp);
    let ghost = put_ghost(&top, "g", "");
    put_memory(&ghost, "MINE");
    put_balloon(&top, DEFAULT_BALLOON_FOLDER);
    let mine = put_balloon(&low, "mine");
    let (got, events) = capture(|| resolve_balloon_for_ghost(&roots, &ghost, no_pick));
    let got = got.expect("決まる");
    assert_eq!(
        (got.route, got.dir, got.folder.as_deref()),
        (BalloonRoute::Memory, mine, Some("mine"))
    );
    assert!(named(&events, "last_balloon_not_found").is_empty());
}

/// どの根にも無い記憶 → warn `last_balloon_not_found` 1 件（`balloon_store` は各根の `balloon\` を
/// `; ` でつないだもの）で、次の候補（既定）へ。
#[test]
fn memory_in_no_root_warns_with_store_list_and_falls_through() {
    let tmp = TempPath::under_target("cfg-roots-memory-gone");
    let (top, low, roots) = two_roots(&tmp);
    let ghost = put_ghost(&top, "g", "");
    put_memory(&ghost, "gone");
    put_balloon(&top, "x");
    let default_balloon = put_balloon(&low, DEFAULT_BALLOON_FOLDER);
    let (got, events) = capture(|| resolve_balloon_for_ghost(&roots, &ghost, no_pick));
    let got = got.expect("決まる");
    assert_eq!(
        (got.route, got.dir),
        (BalloonRoute::Default, default_balloon)
    );
    let warned = named(&events, "last_balloon_not_found");
    assert_eq!(warned.len(), 1, "{events:?}");
    let stores = format!(
        "{}; {}",
        top.join("balloon").display(),
        low.join("balloon").display()
    );
    assert_eq!(warned[0].field("balloon_store"), Some(stores.as_str()));
    assert_eq!(warned[0].level, tracing::Level::WARN);
}
