//! 起動解決の純粋な判断の決定論テスト。
//!
//! 確かめること: ゴースト 6 分岐（argv／記憶／唯一／既定／無作為／0 体）・バルーン 7 分岐
//! （argv／記憶／同梱／唯一／既定／無作為／0）と、記憶の指す先が無い 2 通り・同梱の指す先が
//! 無い 1 通り・古い既定の記憶が新しい既定に負けないこと。無作為は固定の添字（`|n| n - 1`）を
//! 注入し、選ばれたものが列挙に含まれることと経路が `Random` であることを見る。既定の分岐は
//! `listed` に `"emo2"`／[`DEFAULT_BALLOON_FOLDER`] を並べるだけ（検体の登記に依存しない）。
//! 本番の `pick_index` が候補の範囲に収まることを見る 1 本のほかは、プロセス・fs・乱数に触れない。
//! 記憶の書き手と直読みは兄弟の `boot_resolve_last_used_tests.rs` にある。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoot;
use log_capture_kit::{CapturedEvent, capture};

use super::*;

// ---------------------------------------------------------------- 道具立て

/// 実在を問わない根（判断は fs を見ないので実在は要らない）。
pub(super) fn root() -> BasewareRoot {
    BasewareRoot::new(PathBuf::from(r"C:\areka-boot-resolve-tests"))
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

/// 呼ばれてはならない添字（無作為の段に届かない分岐で使う）。
fn no_pick(n: usize) -> usize {
    panic!("無作為の段に届いてはならない（候補数 {n}）")
}

/// 最後の候補を選ぶ固定の添字（先頭を選ぶ実装と区別するため末尾）。
fn last(n: usize) -> usize {
    n - 1
}

/// 記録の中から `event` が一致するものを数える。
pub(super) fn count_event(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// `event` の記録がちょうど 1 件あり、そのレベルが `level` であることを確かめる。
pub(super) fn assert_one_event(events: &[CapturedEvent], event: &str, level: tracing::Level) {
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect();
    assert_eq!(hits.len(), 1, "{event} はちょうど 1 件: {events:?}");
    assert_eq!(hits[0].level, level, "{event} のレベル");
}

fn ghost(
    root: &BasewareRoot,
    argv: Option<&Path>,
    memory: Option<&str>,
    listed: &[String],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<GhostDecision, NoGhost>, Vec<CapturedEvent>) {
    let listed: Vec<GhostEntry> = listed
        .iter()
        .map(|folder| GhostEntry {
            dir: root.ghost_dir(folder),
            root: root.dir().to_path_buf(),
            identity: areka_ghost::catalog::Identity {
                folder: folder.clone(),
                ..Default::default()
            },
        })
        .collect();
    let inputs = GhostInputs {
        argv,
        pointed: None,
        random: false,
        call_ghost: None,
        memory,
        listed: &listed,
        stores: &[root.ghost_store()],
    };
    capture(|| resolve_ghost(&inputs, pick))
}

fn balloon(
    root: &BasewareRoot,
    argv: Option<&Path>,
    memory: Option<&str>,
    companion: Option<&str>,
    listed: &[String],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<BalloonDecision, NoBalloon>, Vec<CapturedEvent>) {
    // フォルダ名の列を一覧の形（`name` 無し）へ直す。descript の 2 鍵は無し。
    let listed: Vec<areka_ghost::catalog::BalloonEntry> = listed
        .iter()
        .map(|folder| areka_ghost::catalog::BalloonEntry {
            dir: root.balloon_dir(folder),
            root: root.dir().to_path_buf(),
            identity: areka_ghost::catalog::Identity {
                folder: folder.clone(),
                ..Default::default()
            },
        })
        .collect();
    let inputs = BalloonInputs {
        argv,
        memory,
        default_balloon_path: None,
        balloon_name: None,
        companion,
        listed: &listed,
        stores: &[root.balloon_store()],
    };
    capture(|| resolve_balloon(&inputs, pick))
}

pub(super) fn ghost_at(root: &BasewareRoot, route: GhostRoute, folder: &str) -> GhostDecision {
    GhostDecision {
        route,
        dir: root.ghost_dir(folder),
        folder: Some(folder.to_owned()),
    }
}

pub(super) fn balloon_at(
    root: &BasewareRoot,
    route: BalloonRoute,
    folder: &str,
) -> BalloonDecision {
    BalloonDecision {
        route,
        dir: root.balloon_dir(folder),
        folder: Some(folder.to_owned()),
    }
}

// ---------------------------------------------------------------- ゴースト 6 分岐

/// argv: 渡されたパスそのもの・名前は持たない。記憶の指す先が無くても列挙が 0 でも見ない
/// （`warn!` も出ない）。
#[test]
fn ghost_argv_wins_without_looking_at_memory_or_listing() {
    let root = root();
    let argv = PathBuf::from(r"C:\somewhere\outside\my_ghost");
    for listed in [names(&[]), names(&["a", "b"])] {
        let (got, events) = ghost(&root, Some(&argv), Some("gone"), &listed, no_pick);
        assert_eq!(
            got,
            Ok(GhostDecision {
                route: GhostRoute::Argv,
                dir: argv.clone(),
                folder: None,
            })
        );
        assert!(events.is_empty(), "argv では記憶も列挙も見ない: {events:?}");
    }
}

/// 記憶: 列挙に在れば記憶のゴースト（唯一・既定より先）。
#[test]
fn ghost_memory_listed_wins_over_default() {
    let root = root();
    let listed = names(&["a", "emo2", "z"]);
    let (got, events) = ghost(&root, None, Some("z"), &listed, no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Memory, "z")));
    assert!(events.is_empty(), "{events:?}");
}

/// 唯一: 記憶が無く列挙が 1 体。
#[test]
fn ghost_only_one_listed() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&["solo"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Only, "solo")));
}

/// 唯一は既定より先: 列挙が `emo2` 1 体だけなら経路は `Only`（既定の段は 2 体以上のとき）。
#[test]
fn ghost_only_emo2_is_only_not_default() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&["emo2"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Only, "emo2")));
}

/// 既定: 2 体以上で `emo2` が列挙される。
#[test]
fn ghost_default_emo2_when_listed_among_many() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&["a", "emo2", "z"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Default, "emo2")));
}

/// 無作為: 2 体以上で `emo2` が無い。注入した添字の候補が選ばれ、info を 1 件残す。
#[test]
fn ghost_random_picks_from_listing_and_logs_info() {
    let root = root();
    let listed = names(&["a", "b", "c"]);
    let (got, events) = ghost(&root, None, None, &listed, last);
    let got = got.expect("無作為で決まる");
    assert_eq!(got, ghost_at(&root, GhostRoute::Random, "c"));
    assert!(
        listed.contains(got.folder.as_ref().unwrap()),
        "列挙に含まれる"
    );
    assert_one_event(&events, "ghost_picked_randomly", tracing::Level::INFO);
}

/// 0 体: ゴーストの格納フォルダを載せて返す。
#[test]
fn ghost_none_listed_returns_store() {
    let root = root();
    let (got, _) = ghost(&root, None, None, &names(&[]), no_pick);
    assert_eq!(
        got,
        Err(NoGhost {
            ghost_stores: vec![root.ghost_store()],
        })
    );
}

/// 記憶の指す先が無い: `warn!` を 1 件残して次の段（ここでは唯一）へ。
#[test]
fn ghost_memory_not_listed_warns_then_falls_through() {
    let root = root();
    let (got, events) = ghost(&root, None, Some("gone"), &names(&["solo"]), no_pick);
    assert_eq!(got, Ok(ghost_at(&root, GhostRoute::Only, "solo")));
    assert_one_event(&events, "last_ghost_not_found", tracing::Level::WARN);
}

// ---------------------------------------------------------------- バルーン 7 分岐

/// argv: 渡されたパスそのもの。記憶・同梱の指す先が無くても列挙が 0 でも見ない。
#[test]
fn balloon_argv_wins_without_looking_at_anything_else() {
    let root = root();
    let argv = PathBuf::from(r"C:\somewhere\outside\my_balloon");
    for listed in [names(&[]), names(&["a", "b"])] {
        let (got, events) = balloon(
            &root,
            Some(&argv),
            Some("gone"),
            Some("also_gone"),
            &listed,
            no_pick,
        );
        assert_eq!(
            got,
            Ok(BalloonDecision {
                route: BalloonRoute::Argv,
                dir: argv.clone(),
                folder: None,
            })
        );
        assert!(events.is_empty(), "argv では他を見ない: {events:?}");
    }
}

/// 記憶: 列挙に在れば同梱より先。
#[test]
fn balloon_memory_wins_over_companion() {
    let root = root();
    let listed = names(&["mine", "bundled", DEFAULT_BALLOON_FOLDER]);
    let (got, events) = balloon(&root, None, Some("mine"), Some("bundled"), &listed, no_pick);
    assert_eq!(got, Ok(balloon_at(&root, BalloonRoute::Memory, "mine")));
    assert!(events.is_empty(), "{events:?}");
}

/// 同梱: 記憶が無く、`install.txt` の名が列挙に在る（唯一・既定より先）。
#[test]
fn balloon_companion_wins_over_default() {
    let root = root();
    let listed = names(&["bundled", DEFAULT_BALLOON_FOLDER]);
    let (got, _) = balloon(&root, None, None, Some("bundled"), &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Companion, "bundled"))
    );
}

/// 唯一: 記憶・同梱が無く列挙が 1 つ。
#[test]
fn balloon_only_one_listed() {
    let root = root();
    let (got, _) = balloon(&root, None, None, None, &names(&["solo"]), no_pick);
    assert_eq!(got, Ok(balloon_at(&root, BalloonRoute::Only, "solo")));
}

/// 唯一は既定より先: 列挙が既定の名前 1 つだけなら経路は `Only`。
#[test]
fn balloon_only_default_name_is_only_not_default() {
    let root = root();
    let (got, _) = balloon(
        &root,
        None,
        None,
        None,
        &names(&[DEFAULT_BALLOON_FOLDER]),
        no_pick,
    );
    assert_eq!(
        got,
        Ok(balloon_at(
            &root,
            BalloonRoute::Only,
            DEFAULT_BALLOON_FOLDER
        ))
    );
}

/// 既定: 2 つ以上で `areka-balloon` が列挙される。既定の名前を文字で比べるのは
/// このテストだけ（ほかは定数を参照するので、定数の値を誤ったときに赤になるのはここ）。
#[test]
fn balloon_default_areka_balloon_when_listed_among_many() {
    let root = root();
    let listed = names(&["a", "areka-balloon", "z"]);
    let (got, _) = balloon(&root, None, None, None, &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Default, "areka-balloon"))
    );
}

/// 既定を替えた後も、記憶が `StayseeBalloon`（2026-10 までの既定）の利用者は記憶の段で
/// `StayseeBalloon` に決まり、新しい既定へ移らない（記録 0 件）。
#[test]
fn balloon_memory_of_old_default_wins_over_new_default() {
    let root = root();
    let listed = names(&[DEFAULT_BALLOON_FOLDER, "StayseeBalloon"]);
    let (got, events) = balloon(&root, None, Some("StayseeBalloon"), None, &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Memory, "StayseeBalloon"))
    );
    assert!(events.is_empty(), "{events:?}");
}

/// 無作為: 2 つ以上で既定が無い。注入した添字の候補が選ばれ、info を 1 件残す。
#[test]
fn balloon_random_picks_from_listing_and_logs_info() {
    let root = root();
    let listed = names(&["a", "b", "c"]);
    let (got, events) = balloon(&root, None, None, None, &listed, last);
    let got = got.expect("無作為で決まる");
    assert_eq!(got, balloon_at(&root, BalloonRoute::Random, "c"));
    assert!(
        listed.contains(got.folder.as_ref().unwrap()),
        "列挙に含まれる"
    );
    assert_one_event(&events, "balloon_picked_randomly", tracing::Level::INFO);
}

/// 0: バルーンの格納フォルダを載せて返す。
#[test]
fn balloon_none_listed_returns_store() {
    let root = root();
    let (got, _) = balloon(&root, None, None, None, &names(&[]), no_pick);
    assert_eq!(
        got,
        Err(NoBalloon {
            balloon_stores: vec![root.balloon_store()],
        })
    );
}

/// 記憶の指す先が無い: `warn!` を 1 件残して次の段（同梱）へ。
#[test]
fn balloon_memory_not_listed_warns_then_companion() {
    let root = root();
    let listed = names(&["bundled", "other"]);
    let (got, events) = balloon(&root, None, Some("gone"), Some("bundled"), &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(&root, BalloonRoute::Companion, "bundled"))
    );
    assert_one_event(&events, "last_balloon_not_found", tracing::Level::WARN);
    assert_eq!(count_event(&events, "companion_balloon_not_found"), 0);
}

/// 同梱の指す先が無い: `warn!` を 1 件残して次の段（既定）へ。
#[test]
fn balloon_companion_not_listed_warns_then_default() {
    let root = root();
    let listed = names(&["a", DEFAULT_BALLOON_FOLDER]);
    let (got, events) = balloon(&root, None, None, Some("gone"), &listed, no_pick);
    assert_eq!(
        got,
        Ok(balloon_at(
            &root,
            BalloonRoute::Default,
            DEFAULT_BALLOON_FOLDER
        ))
    );
    assert_one_event(&events, "companion_balloon_not_found", tracing::Level::WARN);
    assert_eq!(count_event(&events, "last_balloon_not_found"), 0);
}

// ---------------------------------------------------------------- 本番の添字

/// `pick_index` は候補の範囲 `0..n` に収まる（質は問わない）。
#[test]
fn pick_index_stays_in_range() {
    for n in 2..=7 {
        for _ in 0..32 {
            assert!(pick_index(n) < n);
        }
    }
}
