//! 1 ゴースト分のバルーンを決める口（`resolve_balloon_for_ghost`）を実ファイルで踏む。
//!
//! 段の並びと突き合わせの判断は I/O なしの `boot_resolve_balloon_tests.rs` が固定する。ここで見るのは
//! 「読んだ 2 鍵が鎖へ届くこと」「ゴーストの中の `balloon\` が根より先に候補へ入ること」と記録の件数だけ。
//! 根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下）。

use crate::boot_config::resolve_balloon_for_ghost;
use crate::boot_resolve::boot_resolve_tests::no_pick;
use crate::boot_resolve::{BalloonRoute, DEFAULT_BALLOON_FOLDER};
use crate::session_end::test_support::put_memory;
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

/// 入力を集める口を呼び、(段, フォルダ名) と記録を返す。決まったバルーンの場所が
/// `<根>/balloon/<フォルダ名>` であることもここで確かめる。
fn resolve(tmp: &TempPath, ghost: &Path) -> ((BalloonRoute, String), Vec<CapturedEvent>) {
    let ((route, dir), events) = resolve_dir(tmp, ghost);
    let folder = dir.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(dir, tmp.child("balloon").join(&folder));
    ((route, folder), events)
}

/// 入力を集める口を呼び、(段, 決まったバルーンの場所) と記録を返す（ゴーストの中に当たる場面用）。
fn resolve_dir(tmp: &TempPath, ghost: &Path) -> ((BalloonRoute, PathBuf), Vec<CapturedEvent>) {
    let root = areka_ghost::BasewareRoots::single(tmp.path().to_path_buf());
    let (got, events) = capture(|| resolve_balloon_for_ghost(&root, ghost, no_pick));
    let got = got.expect("決まる");
    assert!(got.folder.is_some(), "引数以外はフォルダ名が在る");
    ((got.route, got.dir), events)
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

/// `default.balloon.path` と同じ名前のフォルダがゴーストの中の `balloon\` にだけ在る → descript の段で
/// ゴーストの中に当たり、警告は 0 件。
#[test]
fn default_balloon_path_hits_ghost_inner_balloon() {
    let tmp = TempPath::under_target("cfg-balloon-inner");
    let ghost = fixture(&tmp, "default.balloon.path,inner\n", &[("b1", "First")]);
    put_balloon(&ghost.join("balloon").join("inner"), "Inner");
    let (got, events) = resolve_dir(&tmp, &ghost);
    assert_eq!(
        got,
        (BalloonRoute::Descript, ghost.join("balloon").join("inner"))
    );
    assert!(warnings(&events).is_empty(), "{events:?}");
}

/// `balloon/plain` は `<ゴースト>\balloon\plain` が在っても当たらない（フォルダ名の突き合わせ）。
/// 名前は検体と無関係にしてある（検体の在処を綴るのは窓口 `sample_ghost_kit::SampleRoot` だけ）。
/// `descript_balloon_not_found` が 1 件（`balloon_store` は根の置き場だけ）。
#[test]
fn balloon_prefixed_path_misses_even_with_ghost_inner_folder() {
    let tmp = TempPath::under_target("cfg-balloon-inner-prefixed");
    let ghost = fixture(
        &tmp,
        "default.balloon.path,balloon/plain\n",
        &[("b1", "First")],
    );
    put_balloon(&ghost.join("balloon").join("plain"), "Plain");
    let (got, events) = resolve_dir(&tmp, &ghost);
    assert_eq!(
        got,
        (
            BalloonRoute::Default,
            tmp.child("balloon").join(DEFAULT_BALLOON_FOLDER)
        )
    );
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
            Some("balloon/plain"),
            Some(store.as_str()),
        )
    );
}

/// 同梱の段（`install.txt` の `balloon.directory`）がゴーストの中に当たる。
#[test]
fn companion_hits_ghost_inner_balloon() {
    let tmp = TempPath::under_target("cfg-balloon-inner-companion");
    let ghost = fixture(&tmp, "", &[("b1", "First")]);
    put_balloon(&ghost.join("balloon").join("bundled"), "Bundled");
    put_install(&ghost, "balloon.directory,bundled\n");
    let (got, events) = resolve_dir(&tmp, &ghost);
    assert_eq!(
        got,
        (
            BalloonRoute::Companion,
            ghost.join("balloon").join("bundled")
        )
    );
    assert!(warnings(&events).is_empty(), "{events:?}");
}

/// 根の置き場にバルーンが 1 つも無くても、ゴーストの中に在ればそれで決まり `NoBalloon` にならない。
#[test]
fn empty_root_store_resolves_from_ghost_inner_balloon() {
    let tmp = TempPath::under_target("cfg-balloon-inner-only");
    let ghost = fixture(&tmp, "", &[]);
    std::fs::remove_dir_all(tmp.child("balloon")).expect("根のバルーンを消す");
    put_balloon(&ghost.join("balloon").join("inner"), "Inner");
    let (got, events) = resolve_dir(&tmp, &ghost);
    assert_eq!(
        got,
        (BalloonRoute::Only, ghost.join("balloon").join("inner"))
    );
    assert!(warnings(&events).is_empty(), "{events:?}");
}

/// 根の同じフォルダ名を隠した 1 件ごとに info `ghost_balloon_hides_root` が 1 件（欄は
/// `folder`・`hidden`＝根の場所・`by`＝ゴーストの中の場所）。根に無いゴーストの中のバルーンは数えない。
#[test]
fn ghost_balloon_hides_root_once_per_hidden_entry() {
    let tmp = TempPath::under_target("cfg-balloon-inner-hides");
    let ghost = fixture(&tmp, "", &[("b1", "First"), ("b2", "Second")]);
    for folder in ["b1", "B2", "own"] {
        put_balloon(&ghost.join("balloon").join(folder), folder);
    }
    let (_, events) = resolve_dir(&tmp, &ghost);
    let mut got: Vec<_> = named(&events, "ghost_balloon_hides_root")
        .iter()
        .map(|e| {
            assert_eq!(e.level, tracing::Level::INFO);
            (
                e.field_str("folder").map(str::to_owned),
                e.field("hidden").map(str::to_owned),
                e.field("by").map(str::to_owned),
            )
        })
        .collect();
    got.sort();
    let row = |root_folder: &str, inner_folder: &str| {
        (
            Some(root_folder.to_owned()),
            Some(tmp.child("balloon").join(root_folder).display().to_string()),
            Some(
                ghost
                    .join("balloon")
                    .join(inner_folder)
                    .display()
                    .to_string(),
            ),
        )
    };
    let mut want = vec![row("b1", "b1"), row("b2", "B2")];
    want.sort();
    assert_eq!(got, want, "{events:?}");
}

/// 切替先のバルーンを argv 無しの分岐で解く（記憶・同梱・既定の順）。
mod switch_balloon {
    use super::{no_pick, put_memory};
    use crate::boot_config::resolve_balloon_for_ghost;
    use crate::boot_resolve::{BalloonRoute, DEFAULT_BALLOON_FOLDER};
    use std::path::{Path, PathBuf};
    use temp_path_kit::TempPath;

    /// 根にゴースト `g1` とバルーン 3 つ（既定・b1・b2）を組み、ゴーストのフォルダを返す。
    fn fixture(tmp: &TempPath) -> PathBuf {
        let ghost = tmp.child("ghost").join("g1");
        let master = ghost.join("ghost").join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        for folder in [DEFAULT_BALLOON_FOLDER, "b1", "b2"] {
            let dir = tmp.child("balloon").join(folder);
            std::fs::create_dir_all(&dir).expect("フォルダを組む");
            std::fs::write(dir.join("descript.txt"), "charset,UTF-8\n").expect("descript");
        }
        ghost
    }

    fn put_companion(ghost: &Path, folder: &str) {
        std::fs::write(
            ghost.join("install.txt"),
            format!("charset,UTF-8\nballoon.directory,{folder}\n"),
        )
        .expect("install.txt");
    }

    fn resolve(tmp: &TempPath, ghost: &Path) -> (BalloonRoute, Option<String>) {
        let root = areka_ghost::BasewareRoots::single(tmp.path().to_path_buf());
        let got = resolve_balloon_for_ghost(&root, ghost, no_pick).expect("決まる");
        assert_eq!(
            got.dir,
            tmp.child("balloon").join(got.folder.as_deref().unwrap())
        );
        (got.route, got.folder)
    }

    /// 同梱バルーンがあればそれ（記憶が無いとき）。
    #[test]
    fn companion_is_used_when_no_memory() {
        let tmp = TempPath::new("switch-balloon-companion");
        let ghost = fixture(&tmp);
        put_companion(&ghost, "b1");
        assert_eq!(
            resolve(&tmp, &ghost),
            (BalloonRoute::Companion, Some("b1".to_owned()))
        );
    }

    /// そのゴーストの最後のバルーンの記憶があれば、同梱より先にそれ（起動解決と同じ順）。
    #[test]
    fn memory_of_that_ghost_is_used() {
        let tmp = TempPath::new("switch-balloon-memory");
        let ghost = fixture(&tmp);
        put_companion(&ghost, "b1");
        put_memory(&ghost, "b2");
        assert_eq!(
            resolve(&tmp, &ghost),
            (BalloonRoute::Memory, Some("b2".to_owned()))
        );
    }

    /// 同梱も記憶も無ければ既定のバルーン。
    #[test]
    fn default_when_neither_companion_nor_memory() {
        let tmp = TempPath::new("switch-balloon-default");
        let ghost = fixture(&tmp);
        assert_eq!(
            resolve(&tmp, &ghost),
            (
                BalloonRoute::Default,
                Some(DEFAULT_BALLOON_FOLDER.to_owned())
            )
        );
    }
}
