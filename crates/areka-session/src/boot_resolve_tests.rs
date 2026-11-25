//! 起動解決の純粋な判断の決定論テスト。
//!
//! 確かめること: ゴーストの経路のうち列挙から決める 6 分岐（argv／記憶／唯一／既定／無作為／0 体）・バルーン 7 分岐
//! （argv／記憶／同梱／唯一／既定／無作為／0）と、記憶の指す先が無い 2 通り・同梱の指す先が
//! 無い 1 通り・記憶が並んだ既定より先なこと。無作為は固定の添字（`|n| n - 1`）を
//! 注入し、選ばれたものが列挙に含まれることと経路が `Random` であることを見る。既定の分岐は
//! `listed` に `"emo2"`／[`DEFAULT_BALLOON_FOLDER`] を並べるだけ（検体の登記に依存しない）。
//! 本番の `pick_index` が候補の範囲に収まることを見る 1 本のほかは、プロセス・fs・乱数に触れない。
//! 例外は起動中の印の文の 2 本と印に書く名前の 1 本で、`target\` の下の根で fs を踏む。
//! 記憶の書き手と直読みは兄弟の `boot_resolve_last_used_tests.rs` にある。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoot;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

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
pub(crate) fn no_pick(n: usize) -> usize {
    panic!("無作為の段に届いてはならない（候補数 {n}）")
}

/// 最後の候補を選ぶ固定の添字（先頭を選ぶ実装と区別するため末尾）。
pub(crate) fn last(n: usize) -> usize {
    n - 1
}

/// `event` の記録（`event` 欄で引く）。
pub(crate) fn events_named<'a>(events: &'a [CapturedEvent], event: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// 記録の中から `event` が一致するものを数える。
pub(crate) fn count_event(events: &[CapturedEvent], event: &str) -> usize {
    events_named(events, event).len()
}

/// `event` の記録がちょうど 1 件あり、そのレベルが `level` であることを確かめる。
pub(crate) fn assert_one_event(events: &[CapturedEvent], event: &str, level: tracing::Level) {
    let hits = events_named(events, event);
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

// ---------------------------------------------------------------- ゴーストの列挙から決める 6 分岐

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

/// 記憶のバルーン `StayseeBalloon` は、既定のバルーンが並んでいても記憶の段で決まる（記録 0 件）。
#[test]
fn balloon_memory_wins_over_listed_default() {
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

// ---------------------------------------------------------------- 記憶の印の文と印の名前

/// `event` の記録がちょうど 1 件あり、その文を返す。
fn one_message(events: &[CapturedEvent], event: &str) -> String {
    let hits = events_named(events, event);
    assert_eq!(hits.len(), 1, "{event} はちょうど 1 件: {events:?}");
    hits[0].message().to_owned()
}

/// 置き場を普通のファイルに替える（書けない）。
fn unwritable(dir: &Path) {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir).expect("置き場のフォルダを外す");
    }
    std::fs::write(dir, "x").expect("置き場の位置に普通のファイルを置く");
}

/// 印の書き手が書いた記憶のファイル（置き場にただ 1 つ在るファイル）。
fn mark_file(app_dir: &Path) -> std::path::PathBuf {
    let files: Vec<_> = std::fs::read_dir(app_dir)
        .expect("置き場を読む")
        .map(|e| e.expect("置き場の項目").path())
        .filter(|p| p.is_file())
        .collect();
    assert_eq!(files.len(), 1, "置き場の記憶のファイル: {files:?}");
    files[0].clone()
}

/// 印を消せない文は、記憶に印が残るときだけ「前回落ちたとして」を含む。
#[test]
fn clear_degraded_foretells_crash_only_when_mark_remains() {
    let tmp = TempPath::under_target("mark-text-clear");

    // 印が残る: 印を書いた後、書き換えの一時ファイルの位置にフォルダを置いて置き換えを失敗させる
    // （本体は読めるまま）。
    let kept = tmp.child("kept");
    write_session_mark(&kept, "動いている子").expect("印を書く");
    let file = mark_file(&kept);
    let mut temp = file.clone().into_os_string();
    temp.push(".tmp");
    std::fs::create_dir_all(&temp).expect("一時ファイルの位置にフォルダを置く");
    let (cleared, kept_events) = capture(|| clear_session_mark(&kept));
    assert!(cleared.is_err(), "印を消せないはず");
    assert_eq!(read_session_mark(&kept), Some("動いている子".to_owned()));
    let kept_text = one_message(&kept_events, "session_mark_clear_degraded");

    // 印が無い: 置き場が普通のファイル（読めず書けない）。
    let gone = tmp.child("gone");
    unwritable(&gone);
    let (cleared, gone_events) = capture(|| clear_session_mark(&gone));
    assert!(cleared.is_err(), "印を消せないはず");
    assert_eq!(read_session_mark(&gone), None);
    let gone_text = one_message(&gone_events, "session_mark_clear_degraded");

    assert_eq!(
        (
            kept_text.contains("前回落ちたとして"),
            gone_text.contains("前回落ちたとして"),
        ),
        (true, false),
        "印が残る・無いの文: {kept_text} / {gone_text}"
    );
}

/// 印を残す文は、記憶に印が在るときだけ「Ref6/7 付き」を含む。
#[test]
fn keep_mark_foretells_ref6_7_only_when_mark_exists() {
    let tmp = TempPath::under_target("mark-text-keep");

    let marked = tmp.child("marked");
    write_session_mark(&marked, "動いている子").expect("印を書く");
    let ((), marked_events) = capture(|| keep_session_mark(&marked, "run_failed", None));
    let marked_text = one_message(&marked_events, "session_mark_kept");

    let empty = tmp.child("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let ((), empty_events) = capture(|| keep_session_mark(&empty, "run_failed", None));
    let empty_text = one_message(&empty_events, "session_mark_kept");

    assert_eq!(
        (
            marked_text.contains("Ref6/7 付き"),
            empty_text.contains("Ref6/7 付き"),
        ),
        (true, false),
        "印が在る・無いの文: {marked_text} / {empty_text}"
    );
}

/// 根の `ghost\<folder>` に descript.txt を置き、ゴーストのフォルダを返す。
pub(crate) fn put_ghost(root: &Path, folder: &str, descript: &str) -> PathBuf {
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

/// 起動中の印に書く名前は和を引かず、決まったゴーストの場所の descript から。
/// 同じフォルダ名が上の根にも在っても、決まった下の根のゴーストの名前を書く。
#[test]
fn running_name_reads_the_decided_location_not_the_union() {
    let tmp = TempPath::under_target("boot-resolve-running-name-decided");
    let (top, low) = (tmp.child("top"), tmp.child("low"));
    put_ghost(&top, "g", "name,上の子\n");
    let lower = put_ghost(&low, "g", "name,下の子\n");
    let ghost = GhostDecision {
        route: GhostRoute::Memory,
        dir: lower,
        folder: Some("g".to_owned()),
    };
    assert_eq!(running_name(&ghost), "下の子");
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
