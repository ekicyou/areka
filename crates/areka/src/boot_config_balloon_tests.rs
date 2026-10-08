//! 入力を集める口（`resolve_balloon_for_ghost`）と起動の入口（`resolve_boot_from`）を実ファイルで踏む
//! （`areka-P0-ghost-standard-balloon` タスク 3・要件 2.5・3.2・4.1・5.1・7.4・7.7）。
//!
//! 段の並びと突き合わせの判断は I/O なしの `boot_resolve_balloon_tests.rs` が固定する。ここで見るのは
//! 「読んだ 2 鍵が鎖へ届くこと」「ゴーストの中の `balloon/` を読まないこと」「引数の腕は読まないこと」と
//! 記録の件数だけ。根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下・要件 7.7）。

use crate::boot_config::{RootSource, resolve_balloon_for_ghost, resolve_boot_from};
use crate::boot_resolve::BalloonRoute;
use areka_ghost::BasewareRoot;
use log_capture_kit::{CapturedEvent, capture};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// 呼ばれない添字（候補に既定のバルーンを置くので無作為の段へ届かない）。
fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

/// 根にゴースト `g1`（descript.txt に `extra` を足す）と、バルーン（フォルダ名と `name`）を組む。
/// 既定のバルーン `StayseeBalloon` はいつも置く。ゴーストのフォルダを返す。
fn fixture(tmp: &TempPath, extra: &str, balloons: &[(&str, &str)]) -> PathBuf {
    let ghost = tmp.child("ghost").join("g1");
    let master = ghost.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\n{extra}"),
    )
    .expect("descript");
    for (folder, name) in [("StayseeBalloon", "Staysee")].iter().chain(balloons) {
        put_balloon(&tmp.child("balloon").join(folder), name);
    }
    ghost
}

fn put_balloon(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).expect("フォルダを組む");
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\nname,{name}\n"),
    )
    .expect("descript");
}

fn put_install(ghost: &Path, body: &str) {
    std::fs::write(ghost.join("install.txt"), format!("charset,UTF-8\n{body}"))
        .expect("install.txt");
}

/// そのゴーストの最後のバルーンの記憶（Ghost スコープ・boot が据える場所と同じ）。
fn put_memory(ghost: &Path, folder: &str) {
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

/// 入力を集める口を呼び、(段, フォルダ名) と記録を返す。決まったバルーンの場所が
/// `<根>/balloon/<フォルダ名>` であることもここで確かめる。
fn resolve(tmp: &TempPath, ghost: &Path) -> ((BalloonRoute, String), Vec<CapturedEvent>) {
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let (got, events) = capture(|| resolve_balloon_for_ghost(&root, ghost, no_pick));
    let got = got.expect("決まる");
    let folder = got.folder.expect("引数以外はフォルダ名が在る");
    assert_eq!(got.dir, tmp.child("balloon").join(&folder));
    ((got.route, folder), events)
}

fn named<'e>(events: &'e [CapturedEvent], event: &str) -> Vec<&'e CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

fn warnings(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect()
}

/// `claudia` と同じ形の同梱（番号付きだけ）: 既定のバルーンが在っても、同梱の段で `balloon0` の値。
#[test]
fn numbered_only_companion_wins_over_default() {
    let tmp = TempPath::under_target("cfg-balloon-claudia");
    let ghost = fixture(
        &tmp,
        "",
        &[("claudia", "Claudia"), ("claudia_vertical", "Claudia V")],
    );
    put_install(
        &ghost,
        "balloon0.directory,claudia\nballoon1.directory,claudia_vertical\n",
    );
    let (got, events) = resolve(&tmp, &ghost);
    assert_eq!(got, (BalloonRoute::Companion, "claudia".to_owned()));
    assert!(warnings(&events).is_empty(), "{events:?}");
}

/// descript の `balloon`（`name` で指す）と無印の同梱の両方 → descript の段（2 鍵が鎖へ届く）。
#[test]
fn descript_balloon_wins_over_unnumbered_companion() {
    let tmp = TempPath::under_target("cfg-balloon-descript");
    let ghost = fixture(
        &tmp,
        "balloon,Second Balloon\n",
        &[("b1", "First"), ("b2", "Second Balloon")],
    );
    put_install(&ghost, "balloon.directory,b1\n");
    let (got, events) = resolve(&tmp, &ghost);
    assert_eq!(got, (BalloonRoute::Descript, "b2".to_owned()));
    assert!(warnings(&events).is_empty(), "{events:?}");
}

/// `default.balloon.path` と同じ名前のフォルダがゴーストの中の `balloon/` にだけ在る → 当たらず
/// 警告 1 件で次の段へ（入力を集める口は根の置き場だけを列挙する・要件 2.5）。
#[test]
fn default_balloon_path_does_not_see_ghost_inner_balloon() {
    let tmp = TempPath::under_target("cfg-balloon-inner");
    let ghost = fixture(&tmp, "default.balloon.path,inner\n", &[("b1", "First")]);
    put_balloon(&ghost.join("balloon").join("inner"), "Inner");
    let (got, events) = resolve(&tmp, &ghost);
    assert_eq!(got, (BalloonRoute::Default, "StayseeBalloon".to_owned()));
    let warned = warnings(&events);
    assert_eq!(warned.len(), 1, "{events:?}");
    let store = tmp.child("balloon").display().to_string();
    assert_eq!(
        (
            warned[0].field_str("event"),
            warned[0].field_str("key"),
            warned[0].field_str("value"),
            warned[0].field("balloon_store"),
        ),
        (
            Some("descript_balloon_not_found"),
            Some("default.balloon.path"),
            Some("inner"),
            Some(store.as_str()),
        )
    );
}

/// 起動の入口を呼ぶ（記憶の置き場は空のフォルダ・印も最後のゴーストも無い）。
fn boot(tmp: &TempPath, rest: &[&Path]) -> (BalloonRoute, PathBuf, Vec<CapturedEvent>) {
    let args: Vec<String> = std::iter::once("areka.exe".to_owned())
        .chain(rest.iter().map(|p| p.display().to_string()))
        .collect();
    let app_dir = tmp.child("app-profile");
    let (got, events) = capture(|| {
        resolve_boot_from(
            Ok((tmp.path().to_path_buf(), RootSource::EnvVar)),
            &args,
            &app_dir,
            no_pick,
        )
    });
    let (cfg, _, balloon, _, _) = got.expect("決まる");
    assert_eq!(cfg.balloon_root, balloon.dir);
    (balloon.route, balloon.dir, events)
}

/// 引数なしで descript の指定が当たる起動 → 「バルーンを決めました」がちょうど 1 件で段が descript（要件 5.1）。
#[test]
fn boot_without_argv_records_descript_route_once() {
    let tmp = TempPath::under_target("cfg-balloon-boot-descript");
    fixture(
        &tmp,
        "default.balloon.path,b2\n",
        &[("b1", "First"), ("b2", "Second")],
    );
    let (route, dir, events) = boot(&tmp, &[]);
    assert_eq!(
        (route, dir),
        (BalloonRoute::Descript, tmp.child("balloon").join("b2"))
    );
    let resolved = named(&events, "balloon_resolved");
    assert_eq!(resolved.len(), 1, "{events:?}");
    assert_eq!(resolved[0].field("route"), Some("Descript"));
}

/// 共有なしで開いて、握っている間は誰も読めなくする（読めば「読めない」の warn が出る）。
fn hold_exclusive(path: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .expect("共有なしで開く")
}

/// 引数でバルーンを渡した起動は、当たらない descript・同梱・記憶を置いても読まない（要件 3.2）:
/// 引数の段・「当たらなかった」の記録 0 件・「バルーンを決めました」1 件。読みの有無は、
/// ゴーストの descript.txt・install.txt・記憶を共有なしで握ったまま起動し、warn が 0 件である
/// ことで判定する（読めば `catalog_descript_unreadable` などが出る。ゴーストかの検査は
/// ファイルの有無だけを見るので、握っていても起動は決まる）。
#[test]
fn boot_with_argv_balloon_reads_no_ghost_balloon_keys() {
    let tmp = TempPath::under_target("cfg-balloon-boot-argv");
    let ghost = fixture(
        &tmp,
        "default.balloon.path,gone\nballoon,gone\n",
        &[("b1", "First")],
    );
    put_install(&ghost, "balloon.directory,gone\n");
    put_memory(&ghost, "gone");
    let master = ghost.join("ghost").join("master");
    let _held = [
        hold_exclusive(&master.join("descript.txt")),
        hold_exclusive(&ghost.join("install.txt")),
        hold_exclusive(&master.join("profile").join("areka").join("sylphya.toml")),
    ];
    let argv_balloon = tmp.child("balloon").join("b1");
    let (route, dir, events) = boot(&tmp, &[&ghost, &argv_balloon]);
    assert_eq!((route, dir), (BalloonRoute::Argv, argv_balloon));
    assert!(
        warnings(&events).is_empty(),
        "引数の腕がゴーストのファイルを読んだ: {events:?}"
    );
    let missed: Vec<_> = [
        "last_balloon_not_found",
        "descript_balloon_not_found",
        "companion_balloon_not_found",
    ]
    .iter()
    .flat_map(|event| named(&events, event))
    .collect();
    assert!(missed.is_empty(), "引数の腕が読んだ痕跡: {missed:?}");
    assert_eq!(named(&events, "balloon_resolved").len(), 1, "{events:?}");
}
