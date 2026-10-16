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

// ---- 並びを宛先にして入れる（install_into・tasks.md 4.6・要件 5.1・5.3・5.4・5.5・5.7・5.9・12.4） ----

use std::collections::BTreeMap;
use std::os::windows::fs::OpenOptionsExt;

use areka_nar::NarArchive;
use sample_ghost_kit::{NarBuilder, install_txt};

/// ゴースト `directory` の書庫（`balloon` が在れば同梱のバルーン `kaku` つき）を開く。
fn ghost_nar(tmp: &TempPath, directory: &str, balloon: bool) -> NarArchive {
    let mut lines = vec!["type,ghost".to_owned(), "name,テスト".to_owned()];
    lines.push(format!("directory,{directory}"));
    if balloon {
        lines.push("balloon.directory,kaku".to_owned());
        lines.push("balloon.source.directory,kaku".to_owned());
    }
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let mut builder = NarBuilder::new()
        .file("install.txt", &install_txt(&lines))
        .done()
        .file(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\nname,New\r\n",
        )
        .done();
    if balloon {
        builder = builder
            .file("kaku/descript.txt", b"charset,UTF-8\r\nname,K\r\n")
            .done();
    }
    let path = tmp.child(&format!("{directory}.nar"));
    builder.write_to(&path).expect("書庫を置ける");
    NarArchive::open(&path).expect("書庫を開ける")
}

/// 木の全内容（相対パス → バイト列）。無ければ空。
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let bytes = std::fs::read(&path).expect("読める");
                out.insert(path.strip_prefix(base).expect("下").to_path_buf(), bytes);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// 新しいゴーストは先頭の根へ、同梱のバルーンは見えている別の根へ入り、要素ごとに `install_placed`。
#[test]
fn new_ghost_goes_to_head_and_companion_balloon_to_its_own_root() {
    let tmp = TempPath::under_target("install-into-companion");
    let (one, two) = (tmp.child("one"), tmp.child("two"));
    std::fs::create_dir_all(&one).expect("根 1");
    put_balloon(&two, "kaku");
    let archive = ghost_nar(&tmp, "fresh", true);

    let (result, events) =
        capture_logs(|| install_into(&list(&[one.clone(), two.clone()]), &archive, None));
    result.expect("入る");
    assert!(
        one.join("ghost/fresh/ghost/master/descript.txt").is_file(),
        "ゴーストは先頭の根"
    );
    assert!(
        !one.join("balloon").exists(),
        "先頭の根にバルーンを作らない"
    );
    assert_eq!(
        std::fs::read(two.join("balloon/kaku/descript.txt")).expect("上書き"),
        b"charset,UTF-8\r\nname,K\r\n",
        "同梱のバルーンは見えている根で上書き"
    );
    let placed: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_placed"))
        .map(|e| {
            (
                e.field("root").map(str::to_owned),
                e.field("folder").map(str::to_owned),
            )
        })
        .collect();
    assert_eq!(
        placed,
        [
            (Some(one.display().to_string()), Some("fresh".to_owned())),
            (Some(two.display().to_string()), Some("kaku".to_owned())),
        ]
    );
}

/// すでにあるゴーストは見えている上の根で上書きされ、下の根の同じ名前には触らない。
#[test]
fn existing_ghost_is_overwritten_in_its_root_and_the_hidden_one_is_untouched() {
    let tmp = TempPath::under_target("install-into-existing");
    let (one, two) = (tmp.child("one"), tmp.child("two"));
    put_ghost(&one, "Dup");
    put_ghost(&two, "dup");
    let below = tree(&two);
    let archive = ghost_nar(&tmp, "dup", false);

    install_into(&list(&[one.clone(), two.clone()]), &archive, None).expect("入る");
    assert_eq!(
        std::fs::read(one.join("ghost/Dup/ghost/master/descript.txt")).expect("上書き"),
        b"charset,UTF-8\r\nname,New\r\n"
    );
    assert_eq!(tree(&two), below, "下の根の同じ名前は 1 バイトも変わらない");
}

/// 根をまたぐ書庫で 2 つ目の配置（別の根の同梱のバルーン）の確定に失敗すると、全部が元に戻る。
#[test]
fn failure_in_the_second_root_restores_both_roots() {
    let tmp = TempPath::under_target("install-into-rollback");
    let (one, two) = (tmp.child("one"), tmp.child("two"));
    std::fs::create_dir_all(&one).expect("根 1");
    put_balloon(&two, "kaku");
    let (first, second) = (tree(&one), tree(&two));
    let archive = ghost_nar(&tmp, "fresh", true);
    // 読みだけを共有して掴む＝確定の入れ替えが失敗する（areka-nar の檻と同じ型）。
    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1 /* FILE_SHARE_READ */)
        .open(two.join("balloon/kaku/descript.txt"))
        .expect("掴める");

    let error = install_into(&list(&[one.clone(), two.clone()]), &archive, None)
        .expect_err("使用中なので確定できない");
    drop(handle);
    assert!(
        matches!(
            error,
            NarError::Io {
                phase: IoPhase::Commit,
                rolled_back: true,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(
        tree(&one)
            .into_keys()
            .filter(|p| !p.starts_with(".nar-work"))
            .count(),
        first.len(),
        "先頭の根のゴーストも元に戻る"
    );
    assert_eq!(
        tree(&two)
            .into_iter()
            .filter(|(p, _)| !p.starts_with(".nar-work"))
            .collect::<BTreeMap<_, _>>(),
        second,
        "2 つ目の根の宛先は元のまま"
    );
}

/// 見つかる根が無ければ入れずに error `install_no_root` を残し、書く前の失敗として返す。
#[test]
fn no_found_root_fails_before_writing_with_an_error() {
    let tmp = TempPath::under_target("install-into-no-root");
    let archive = ghost_nar(&tmp, "fresh", false);
    let (result, events) =
        capture_logs(|| install_into(&list(&[tmp.child("a"), tmp.child("b")]), &archive, None));
    assert!(
        matches!(
            result,
            Err(NarError::Io {
                phase: IoPhase::Stage,
                rolled_back: true,
                ..
            })
        ),
        "{result:?}"
    );
    let logged = events
        .iter()
        .find(|e| e.field_str("event") == Some("install_no_root"))
        .expect("error が残る");
    assert_eq!(logged.level, tracing::Level::ERROR);
    assert!(!tmp.child("a").exists(), "何も作らない");
}
