//! 起動の鎖を 2 つの根の和の上で踏む（areka-P0-baseware-root-list タスク 3.2・要件 2.1・2.8・
//! 4.1〜4.5・9.6・9.7・12.3）。
//!
//! 確かめること: 既定ゴーストの採用・同梱のバルーンの引き当て・記憶のフォルダ名からの引き当て
//! （大文字小文字だけ違う綴りでも当たる）・どの根にも無いときの今と同じ次の候補と記録（値は格納
//! フォルダの列を `; ` でつないだもの）・0 体の失敗が各根の格納フォルダを持つこと・起動中の印に
//! 書く名前が和を引かず決まったゴーストの場所から引けること。根は `TempPath::under_target`。

use crate::alert::AlertScene;
use crate::boot_config::{resolve_balloon_for_ghost, resolve_boot_from};
use crate::boot_resolve::{BalloonRoute, GhostDecision, GhostRoute, running_name};
use crate::roots::{ResolvedRoots, RootStatus, RootsOrigin};
use areka_ghost::{BasewareRoot, BasewareRoots};
use log_capture_kit::{CapturedEvent, capture};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

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

fn resolved(roots: &BasewareRoots) -> ResolvedRoots {
    ResolvedRoots {
        roots: roots.clone(),
        origin: RootsOrigin::EnvVar,
        status: roots
            .roots()
            .iter()
            .map(|r| RootStatus {
                dir: r.dir().to_path_buf(),
                found: true,
            })
            .collect(),
        skipped: Vec::new(),
        memory_fell_back: false,
    }
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

/// そのゴーストの最後のバルーンの記憶（Ghost スコープ・boot が据える場所と同じ）。
fn put_balloon_memory(ghost: &Path, folder: &str) {
    let dir = ghost
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    std::fs::create_dir_all(&dir).expect("フォルダを組む");
    std::fs::write(
        dir.join("sylphya.toml"),
        format!("format-version = 1\n[last]\nballoon = \"{folder}\"\n"),
    )
    .expect("記憶");
}

fn named<'e>(events: &'e [CapturedEvent], event: &str) -> Vec<&'e CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// 既定ゴーストが下の根にだけ在る → 和の上で既定の段が当たり、場所は下の根の項目。
/// 既定バルーンも下の根にだけ在る → 既定の段が当たる。
#[test]
fn default_ghost_and_balloon_are_adopted_from_the_lower_root() {
    let tmp = TempPath::under_target("cfg-roots-default");
    let (top, low, roots) = two_roots(&tmp);
    put_ghost(&top, "a", "");
    put_ghost(&low, "b", "");
    let emo2 = put_ghost(&low, "emo2", "");
    put_balloon(&top, "x");
    let staysee = put_balloon(&low, "StayseeBalloon");
    let app_dir = tmp.child("app-profile");
    let (got, _) = capture(|| {
        resolve_boot_from(
            Ok(resolved(&roots)),
            &["areka.exe".to_owned()],
            &app_dir,
            no_pick,
        )
    });
    let (cfg, ghost, balloon, _, _) = got.expect("決まる");
    assert_eq!((ghost.route, ghost.dir), (GhostRoute::Default, emo2));
    assert_eq!(
        (balloon.route, balloon.dir),
        (BalloonRoute::Default, staysee)
    );
    assert_eq!(cfg.ghost_root, tmp.child("low").join("ghost").join("emo2"));
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
    put_balloon(&top, "StayseeBalloon");
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
    put_balloon_memory(&ghost, "MINE");
    put_balloon(&top, "StayseeBalloon");
    let mine = put_balloon(&low, "mine");
    let (got, events) = capture(|| resolve_balloon_for_ghost(&roots, &ghost, no_pick));
    let got = got.expect("決まる");
    assert_eq!(
        (got.route, got.dir, got.folder.as_deref()),
        (BalloonRoute::Memory, mine, Some("mine"))
    );
    assert!(named(&events, "last_balloon_not_found").is_empty());
}

/// どの根にも無い記憶 → warn 1 件（event 名と欄名は今のまま・値は各根の `balloon\` を `; ` で
/// つないだもの）で、今と同じ次の候補（既定）へ。
#[test]
fn memory_in_no_root_warns_with_store_list_and_falls_through() {
    let tmp = TempPath::under_target("cfg-roots-memory-gone");
    let (top, low, roots) = two_roots(&tmp);
    let ghost = put_ghost(&top, "g", "");
    put_balloon_memory(&ghost, "gone");
    put_balloon(&top, "x");
    let staysee = put_balloon(&low, "StayseeBalloon");
    let (got, events) = capture(|| resolve_balloon_for_ghost(&roots, &ghost, no_pick));
    let got = got.expect("決まる");
    assert_eq!((got.route, got.dir), (BalloonRoute::Default, staysee));
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

/// 和にゴーストが 0 体 → 告知の場面は各根の `ghost\` の列を持つ（要件 9.6）。
#[test]
fn no_ghost_in_any_root_carries_every_ghost_store() {
    let tmp = TempPath::under_target("cfg-roots-no-ghost");
    let (top, low, roots) = two_roots(&tmp);
    let app_dir = tmp.child("app-profile");
    let got = resolve_boot_from(
        Ok(resolved(&roots)),
        &["areka.exe".to_owned()],
        &app_dir,
        no_pick,
    );
    assert_eq!(
        got.map(|_| ()),
        Err(AlertScene::GhostMissing {
            ghost_stores: vec![top.join("ghost"), low.join("ghost")],
            argv: None,
        })
    );
}

/// 起動中の印に書く名前は和を引かず、決まったゴーストの場所の descript から。
/// 同じフォルダ名が上の根にも在っても、決まった下の根のゴーストの名前を書く。
#[test]
fn running_name_reads_the_decided_location_not_the_union() {
    let tmp = TempPath::under_target("cfg-roots-running-name");
    let (top, low, _) = two_roots(&tmp);
    put_ghost(&top, "g", "name,上の子\n");
    let lower = put_ghost(&low, "g", "name,下の子\n");
    let ghost = GhostDecision {
        route: GhostRoute::Memory,
        dir: lower,
        folder: Some("g".to_owned()),
    };
    assert_eq!(running_name(&ghost), "下の子");
}
