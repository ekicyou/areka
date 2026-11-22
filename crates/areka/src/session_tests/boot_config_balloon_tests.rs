//! 起動の入口（`resolve_boot_from`）のバルーンの解決を実ファイルで踏む。
//!
//! 1 ゴースト分を決める口そのものは `areka-session` の `boot_config_balloon_tests.rs` が見る。ここで見るのは
//! 「引数なしの起動で記録が 1 件」「引数の腕は読まないこと」「引数のゴーストの中の `balloon\` も数えること」だけ。
//! 根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下）。

use crate::session_tests::{no_pick, resolve_boot_from};
use areka_session::boot_resolve::{BalloonRoute, DEFAULT_BALLOON_FOLDER};
use log_capture_kit::{CapturedEvent, capture};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// 根にゴースト `g1`（descript.txt に `extra` を足す）と、バルーン（フォルダ名と `name`）を組む。
/// 既定のバルーン（[`DEFAULT_BALLOON_FOLDER`]）はいつも置く。ゴーストのフォルダを返す。
fn fixture(tmp: &TempPath, extra: &str, balloons: &[(&str, &str)]) -> PathBuf {
    let ghost = tmp.child("ghost").join("g1");
    let master = ghost.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\n{extra}"),
    )
    .expect("descript");
    for (folder, name) in [(DEFAULT_BALLOON_FOLDER, "Staysee")].iter().chain(balloons) {
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

/// 起動の入口を呼ぶ（記憶の置き場は空のフォルダ・印も最後のゴーストも無い）。
fn boot(tmp: &TempPath, rest: &[&Path]) -> (BalloonRoute, PathBuf, Vec<CapturedEvent>) {
    let args: Vec<String> = std::iter::once("areka.exe".to_owned())
        .chain(rest.iter().map(|p| p.display().to_string()))
        .collect();
    let app_dir = tmp.child("app-profile");
    let (got, events) = capture(|| {
        resolve_boot_from(
            areka_session::roots::single_env(tmp.path().to_path_buf()),
            &args,
            &app_dir,
            no_pick,
            |_, dir| dir.to_path_buf(),
        )
    });
    let (cfg, _, balloon, _, _) = got.expect("決まる");
    assert_eq!(cfg.balloon_root, balloon.dir);
    (balloon.route, balloon.dir, events)
}

/// 引数なしで descript の指定が当たる起動 → 「バルーンを決めました」がちょうど 1 件で段が descript。
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

/// 引数でバルーンを渡した起動は、当たらない descript・同梱・記憶を置いても読まない:
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

/// 引数で渡したゴースト（位置の引数・`--ghost`）でも、そのゴーストのフォルダの中のバルーンを数える。
#[test]
fn argv_ghost_counts_its_inner_balloons() {
    for (case, flag) in [("pos", None), ("opt", Some(Path::new("--ghost")))] {
        let tmp = TempPath::under_target(&format!("cfg-balloon-inner-argv-{case}"));
        fixture(&tmp, "", &[("b1", "First")]);
        let dev = tmp.child("ext").join("dev");
        std::fs::create_dir_all(dev.join("ghost").join("master")).expect("フォルダを組む");
        std::fs::write(
            dev.join("ghost").join("master").join("descript.txt"),
            "charset,UTF-8\ndefault.balloon.path,inner\n",
        )
        .expect("descript");
        put_balloon(&dev.join("balloon").join("inner"), "Inner");
        let args: Vec<&Path> = flag.into_iter().chain([dev.as_path()]).collect();
        let (route, dir, events) = boot(&tmp, &args);
        assert_eq!(
            (route, dir),
            (BalloonRoute::Descript, dev.join("balloon").join("inner")),
            "{case}"
        );
        assert!(warnings(&events).is_empty(), "{case}: {events:?}");
    }
}
