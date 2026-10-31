//! 起動オプションの段（指し先・`--random`）の純粋な判断の決定論テスト
//! （areka-P0-startup-options-core 要件 3.1・3.9・3.11・4.3・5.1・5.2・5.4・11.2）。
//!
//! 確かめること: 指し先は記憶・列挙・既定より先で経路 `Pointed`・フォルダ名つき。
//! `--random` は記憶・唯一・既定を飛ばし、0 体＝ゴーストなし・1 体＝その 1 体・2 体以上＝注入した
//! 添字で選び、選んだものと候補の数を info `ghost_random_requested` で 1 件残す。
//! 一時の置き場が消えた次の起動（記憶が列挙に無い）は今の `last_ghost_not_found` で次の段へ。
//! callghost の相手（install-before-boot 要件 4.2・4.3・4.7・4.10・9.5）は記憶より先で経路 `CallGhost`、
//! 目録に無ければ warn `callghost_target_not_found` で記憶へ、位置引数・指し先・`--random` が先。
//! プロセス・fs・乱数には触れない。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoot;
use log_capture_kit::{CapturedEvent, capture};

use super::*;

fn root() -> BasewareRoot {
    BasewareRoot::new(PathBuf::from(r"C:\areka-boot-resolve-option-tests"))
}

fn no_pick(n: usize) -> usize {
    panic!("添字を選んではならない（候補数 {n}）")
}

fn last(n: usize) -> usize {
    n - 1
}

fn ghost(
    root: &BasewareRoot,
    pointed: Option<(&Path, &str)>,
    random: bool,
    memory: Option<&str>,
    listed: &[&str],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<GhostDecision, NoGhost>, Vec<CapturedEvent>) {
    ghost_call(root, None, None, pointed, random, memory, listed, pick)
}

#[allow(clippy::too_many_arguments)]
fn ghost_call(
    root: &BasewareRoot,
    argv: Option<&Path>,
    call_ghost: Option<&str>,
    pointed: Option<(&Path, &str)>,
    random: bool,
    memory: Option<&str>,
    listed: &[&str],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<GhostDecision, NoGhost>, Vec<CapturedEvent>) {
    let listed: Vec<GhostEntry> = listed
        .iter()
        .map(|folder| GhostEntry {
            dir: root.ghost_dir(folder),
            root: root.dir().to_path_buf(),
            identity: areka_ghost::catalog::Identity {
                folder: (*folder).to_owned(),
                ..Default::default()
            },
        })
        .collect();
    let inputs = GhostInputs {
        argv,
        pointed,
        random,
        call_ghost,
        memory,
        listed: &listed,
        stores: &[root.ghost_store()],
    };
    capture(|| resolve_ghost(&inputs, pick))
}

fn at(root: &BasewareRoot, route: GhostRoute, folder: &str) -> GhostDecision {
    GhostDecision {
        route,
        dir: root.ghost_dir(folder),
        folder: Some(folder.to_owned()),
    }
}

/// `ghost_random_requested` の 1 件（info・選んだフォルダ名と候補の数）。
fn assert_random_info(events: &[CapturedEvent], folder: &str, candidates: &str) {
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("ghost_random_requested"))
        .collect();
    assert_eq!(hits.len(), 1, "ちょうど 1 件: {events:?}");
    assert_eq!(hits[0].level, tracing::Level::INFO);
    assert_eq!(hits[0].field_str("folder"), Some(folder));
    assert_eq!(hits[0].field("candidates"), Some(candidates));
}

// ---------------------------------------------------------------- 指し先（要件 3.1・3.9・4.3）

/// 指し先は記憶・既定・`--random` より先。場所とフォルダ名をそのまま返し、記憶も列挙も見ない。
#[test]
fn pointed_wins_over_memory_default_and_random() {
    let root = root();
    let dir = PathBuf::from(r"D:\dev\ghosts\my_ghost");
    for random in [false, true] {
        let (got, events) = ghost(
            &root,
            Some((&dir, "my_ghost")),
            random,
            Some("gone"),
            &["a", "emo2"],
            no_pick,
        );
        assert_eq!(
            got,
            Ok(GhostDecision {
                route: GhostRoute::Pointed,
                dir: dir.clone(),
                folder: Some("my_ghost".to_owned()),
            })
        );
        assert!(events.is_empty(), "記憶も列挙も見ない: {events:?}");
    }
}

/// 指し先は列挙が 0 体でも決まる（置き場の外のゴースト）。
#[test]
fn pointed_needs_no_listing() {
    let root = root();
    let dir = PathBuf::from(r"D:\dev\ghosts\my_ghost");
    let (got, _) = ghost(&root, Some((&dir, "my_ghost")), false, None, &[], no_pick);
    assert_eq!(got.map(|g| g.route), Ok(GhostRoute::Pointed));
}

/// 次の起動で一時の置き場が無い: 記憶は列挙に無く、今の warn で次の段（既定）へ（要件 3.11）。
#[test]
fn next_boot_memory_of_temporary_ghost_falls_through() {
    let root = root();
    let (got, events) = ghost(
        &root,
        None,
        false,
        Some("my_ghost"),
        &["a", "emo2"],
        no_pick,
    );
    assert_eq!(got, Ok(at(&root, GhostRoute::Default, "emo2")));
    let warns = events
        .iter()
        .filter(|e| e.field_str("event") == Some("last_ghost_not_found"))
        .count();
    assert_eq!(warns, 1, "{events:?}");
}

// ---------------------------------------------------------------- `--random`（要件 5.1・5.2・5.4）

/// 2 体以上: 記憶・既定に関わらず注入した添字で選び、選んだものと候補の数を info に残す。
#[test]
fn random_two_or_more_picks_ignoring_memory_and_default() {
    let root = root();
    let (got, events) = ghost(&root, None, true, Some("emo2"), &["a", "emo2", "z"], last);
    assert_eq!(got, Ok(at(&root, GhostRoute::Random, "z")));
    assert_random_info(&events, "z", "3");
    assert!(
        events
            .iter()
            .all(|e| e.field_str("event") != Some("ghost_picked_randomly")),
        "いつもの無作為の段の記録は出ない: {events:?}"
    );
}

/// 1 体: 添字を選ばずその 1 体（経路は `Random`）。info は出す。
#[test]
fn random_one_is_that_one() {
    let root = root();
    let (got, events) = ghost(&root, None, true, Some("gone"), &["solo"], no_pick);
    assert_eq!(got, Ok(at(&root, GhostRoute::Random, "solo")));
    assert_random_info(&events, "solo", "1");
}

/// 0 体: ゴーストなし（今の知らせと終了）。記録は出さない。
#[test]
fn random_zero_is_no_ghost() {
    let root = root();
    let (got, events) = ghost(&root, None, true, Some("gone"), &[], no_pick);
    assert_eq!(
        got,
        Err(NoGhost {
            ghost_stores: vec![root.ghost_store()],
        })
    );
    assert!(events.is_empty(), "{events:?}");
}

// ---------------------------------------------------------------- callghost（install-before-boot 要件 4.2・4.3・4.7・4.10・9.5）

/// 相手は記憶（列挙に在る）・既定より先に立つ。経路は `CallGhost`・記録は出さない。
#[test]
fn call_ghost_wins_over_memory_and_default() {
    let root = root();
    let (got, events) = ghost_call(
        &root,
        None,
        Some("new_ghost"),
        None,
        false,
        Some("a"),
        &["a", "emo2", "new_ghost"],
        no_pick,
    );
    assert_eq!(got, Ok(at(&root, GhostRoute::CallGhost, "new_ghost")));
    assert!(events.is_empty(), "{events:?}");
}

/// 前回がきれいに終わらなかった（記憶を読まない＝None）ときも相手が立つ（要件 4.7）。
#[test]
fn call_ghost_stands_without_memory() {
    let root = root();
    let (got, _) = ghost_call(
        &root,
        None,
        Some("new_ghost"),
        None,
        false,
        None,
        &["emo2", "new_ghost"],
        no_pick,
    );
    assert_eq!(got, Ok(at(&root, GhostRoute::CallGhost, "new_ghost")));
}

/// 相手が目録に無い: warn `callghost_target_not_found` を 1 件残して記憶の段へ。
#[test]
fn call_ghost_not_listed_warns_and_falls_to_memory() {
    let root = root();
    let (got, events) = ghost_call(
        &root,
        None,
        Some("gone"),
        None,
        false,
        Some("a"),
        &["a", "emo2"],
        no_pick,
    );
    assert_eq!(got, Ok(at(&root, GhostRoute::Memory, "a")));
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("callghost_target_not_found"))
        .collect();
    assert_eq!(hits.len(), 1, "{events:?}");
    assert_eq!(hits[0].level, tracing::Level::WARN);
    assert_eq!(hits[0].field_str("folder"), Some("gone"));
}

/// 位置引数・指し先・`--random` は相手より先（相手は見ない＝記録も出ない）。
#[test]
fn argv_pointed_and_random_win_over_call_ghost() {
    let root = root();
    let listed = ["a", "new_ghost", "z"];
    let argv = PathBuf::from(r"D:\dev\argv_ghost");
    let (got, _) = ghost_call(
        &root,
        Some(&argv),
        Some("new_ghost"),
        None,
        false,
        None,
        &listed,
        no_pick,
    );
    assert_eq!(got.map(|g| g.route), Ok(GhostRoute::Argv));
    let dir = PathBuf::from(r"D:\dev\ghosts\my_ghost");
    let (got, _) = ghost_call(
        &root,
        None,
        Some("new_ghost"),
        Some((&dir, "my_ghost")),
        false,
        None,
        &listed,
        no_pick,
    );
    assert_eq!(got.map(|g| g.route), Ok(GhostRoute::Pointed));
    let (got, events) = ghost_call(
        &root,
        None,
        Some("new_ghost"),
        None,
        true,
        None,
        &listed,
        last,
    );
    assert_eq!(got, Ok(at(&root, GhostRoute::Random, "z")));
    assert_random_info(&events, "z", "3");
}
