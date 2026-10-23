//! ゴーストの中の `balloon\` の走査と根の和との合わせ方（areka-P0-ghost-inner-balloon
//! 要件 1.1〜1.3・1.5・2.2・2.5・3.1・3.4・6.1・6.2・6.4・8.4・10.1〜10.3）。
//!
//! 一時フォルダは `temp-path-kit` の `under_target` の下だけに組む（要件 10.2）。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;
use tracing::Level;

use super::{BasewareRoots, Hidden, StoreKind, balloons_for, list_balloons, survey};
use crate::catalog::{BalloonEntry, BasewareRoot, scan_ghost_balloons};
use crate::test_log_capture::{CapturedEvent, capture};

const TARGET: &str = "areka_ghost::catalog";

/// `path` に `body` を書く（親フォルダも作る）。
fn put(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().expect("親")).unwrap();
    std::fs::write(path, body).unwrap();
}

/// 一時フォルダの下にゴーストのフォルダを作って返す。
fn ghost(tmp: &TempPath) -> PathBuf {
    let g = tmp.child("ghost_a");
    put(&g.join(r"ghost\master\descript.txt"), "name,G\r\n");
    g
}

fn warns(events: &[CapturedEvent]) -> Vec<&str> {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::WARN)
        .map(|e| e.event.as_deref().unwrap_or("<event 無し>"))
        .collect()
}

/// `balloon\` の直下の `type` 無し・`type,balloon` は数え、`root` はゴーストのフォルダ・`dir` は
/// `<ゴースト>\balloon\<名>`。ゴーストの直下・`balloon\` そのもの・さらに下は数えない。
/// `type,shell` は除いて `catalog_type_not_balloon` を 1 件だけ残す（要件 1.1〜1.3）。
#[test]
fn ghost_balloon_dir_counts_direct_children_by_root_rules() {
    let tmp = TempPath::under_target("ghost-inner-balloon-scan");
    let g = ghost(&tmp);
    put(&g.join(r"balloon\plain\descript.txt"), "name,Plain\r\n");
    put(
        &g.join(r"balloon\typed\descript.txt"),
        "name,Typed\r\ntype,balloon\r\n",
    );
    put(
        &g.join(r"balloon\shell_like\descript.txt"),
        "type,shell\r\n",
    );
    // 数えない場所: ゴーストの直下・`balloon\` そのもの・`balloon\<名>\<下>\`。
    put(&g.join(r"direct\descript.txt"), "type,balloon\r\n");
    put(&g.join(r"balloon\descript.txt"), "type,balloon\r\n");
    put(
        &g.join(r"balloon\outer\inner\descript.txt"),
        "type,balloon\r\n",
    );

    let mut listed = Vec::new();
    let events = capture(|| listed = scan_ghost_balloons(&g));

    let folders: Vec<&str> = listed.iter().map(|b| b.identity.folder.as_str()).collect();
    assert_eq!(folders, ["plain", "typed"]);
    for b in &listed {
        assert_eq!(b.root, g);
        assert_eq!(b.dir, g.join("balloon").join(&b.identity.folder));
    }
    assert_eq!(listed[0].identity.name.as_deref(), Some("Plain"));
    assert_eq!(warns(&events), ["catalog_type_not_balloon"]);
}

/// `balloon\` が無ければ 0 件で記録も 0 件（要件 1.1・10.3）。
#[test]
fn ghost_without_balloon_dir_lists_zero_silently() {
    let tmp = TempPath::under_target("ghost-inner-balloon-none");
    let g = ghost(&tmp);

    let mut listed = vec![];
    let events = capture(|| listed = scan_ghost_balloons(&g));

    assert!(listed.is_empty());
    assert!(events.iter().all(|e| e.target != TARGET));
}

/// 一時フォルダの下に名前 `label` のゴーストのフォルダを作って返す。
fn ghost_named(tmp: &TempPath, label: &str) -> PathBuf {
    let g = tmp.child(label);
    put(&g.join(r"ghost\master\descript.txt"), "name,G\r\n");
    g
}

/// `store` の下にバルーン `folder`（`name` 付き）を置いて、その場所を返す。
fn balloon(store: &Path, folder: &str, name: &str) -> PathBuf {
    let dir = store.join("balloon").join(folder);
    put(&dir.join("descript.txt"), &format!("name,{name}\r\n"));
    dir
}

/// 根 1 つの並び（作って返す）。
fn one_root(tmp: &TempPath, label: &str) -> (PathBuf, BasewareRoots) {
    let r = tmp.child(label);
    std::fs::create_dir_all(&r).unwrap();
    (r.clone(), BasewareRoots::single(r))
}

fn dirs(entries: &[BalloonEntry]) -> Vec<PathBuf> {
    entries.iter().map(|e| e.dir.clone()).collect()
}

/// ゴーストの中だけ・根だけ・両方に同じフォルダ名（大文字小文字だけ違う綴りを含む）。
/// `listed` はゴーストの中（フォルダ名のバイト順）が先、根の残りが後。`hidden` はゴーストの中が
/// 隠した根の項目だけで、どちらも記録を残さない（要件 1.5・2.2・2.5・3.1・3.4）。
#[test]
fn ghost_inner_comes_first_and_hides_same_folder_in_root() {
    let tmp = TempPath::under_target("ghost-inner-balloon-for");
    let (r, roots) = one_root(&tmp, "root");
    let g = ghost_named(&tmp, "ghost_a");
    let g_only = balloon(&g, "zeta", "Zeta");
    let g_same = balloon(&g, "Shared", "Inner");
    let g_case = balloon(&g, "mix", "InnerMix");
    let r_only = balloon(&r, "alpha", "Alpha");
    let r_same = balloon(&r, "Shared", "Outer");
    let r_case = balloon(&r, "MIX", "OuterMix");

    let mut got = None;
    let events = capture(|| got = Some(balloons_for(&roots, &g)));
    let got = got.unwrap();

    // ゴーストの中はバイト順（大文字が先）: Shared・mix・zeta。根の残りは alpha だけ。
    assert_eq!(
        dirs(&got.listed),
        [g_same.clone(), g_case.clone(), g_only, r_only]
    );
    assert_eq!(got.listed[0].identity.name.as_deref(), Some("Inner"));
    assert_eq!(got.listed[0].root, g);
    assert_eq!(got.listed[3].root, r);
    assert_eq!(
        got.hidden,
        [
            Hidden {
                kind: StoreKind::Balloon,
                folder: "MIX".into(),
                hidden: r_case,
                by: g_case,
            },
            Hidden {
                kind: StoreKind::Balloon,
                folder: "Shared".into(),
                hidden: r_same,
                by: g_same,
            },
        ]
    );
    assert!(events.is_empty(), "記録は残さない: {events:?}");
}

/// ゴーストの中が 0 件なら `listed` は根の和と同じで `hidden` は空。根どうしの隠れの答え
/// （`survey`）は変わらず、`hidden` へは入らない。以前に根へ入れた同名でないバルーンは根の
/// 項目のまま出る（要件 6.1・6.2・6.4・8.4）。
#[test]
fn ghost_without_inner_matches_root_union_and_keeps_root_hiding() {
    let tmp = TempPath::under_target("ghost-inner-balloon-for-none");
    let a = tmp.child("a");
    let b = tmp.child("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(a.clone()),
        BasewareRoot::new(b.clone()),
    ]);
    let g = ghost_named(&tmp, "ghost_a");
    balloon(&a, "dup", "Upper");
    balloon(&b, "dup", "Lower");
    let installed = balloon(&b, "companion_old", "Old");
    let before = survey(&roots);

    let got = balloons_for(&roots, &g);

    assert_eq!(got.listed, list_balloons(&roots));
    assert!(got.hidden.is_empty());
    assert_eq!(survey(&roots), before);
    assert_eq!(before.hidden.len(), 1, "根どうしの隠れは survey にだけ在る");
    let old = got.listed.iter().find(|e| e.dir == installed).unwrap();
    assert_eq!(old.root, b);
}

/// 2 つのゴーストで呼び分けると他方の中は出ず、他方が隠した根のものは出る。ゴーストの
/// フォルダを消した後は、その中のものは出ない（呼ぶたびに今のファイルから作る・要件 1.5・3.4）。
#[test]
fn each_ghost_sees_only_its_own_inner_and_deleted_ghost_drops_out() {
    let tmp = TempPath::under_target("ghost-inner-balloon-for-two");
    let (r, roots) = one_root(&tmp, "root");
    let g1 = ghost_named(&tmp, "ghost_1");
    let g2 = ghost_named(&tmp, "ghost_2");
    let in1 = balloon(&g1, "shared", "One");
    let in2 = balloon(&g2, "only2", "Two");
    let r_shared = balloon(&r, "shared", "Root");

    let for1 = balloons_for(&roots, &g1);
    let for2 = balloons_for(&roots, &g2);

    assert_eq!(dirs(&for1.listed), [in1.clone()]);
    assert_eq!(dirs(&for2.listed), [in2, r_shared.clone()]);
    assert!(for2.hidden.is_empty());

    std::fs::remove_dir_all(&g1).unwrap();
    let after = balloons_for(&roots, &g1);
    assert_eq!(dirs(&after.listed), [r_shared]);
    assert!(after.hidden.is_empty());
}
