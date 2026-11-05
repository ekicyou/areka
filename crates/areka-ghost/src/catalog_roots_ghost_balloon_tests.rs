//! ゴーストの中の `balloon\` の走査と根の和との合わせ方（areka-P0-ghost-inner-balloon
//! 要件 1.1〜1.3・1.5・2.2・2.5・3.1・3.4・6.1・6.2・6.4・8.4・10.1〜10.3）。
//!
//! 一時フォルダは `temp-path-kit` の `under_target` の下だけに組む（要件 10.2）。

use std::path::PathBuf;

use temp_path_kit::TempPath;

use super::{BasewareRoots, Hidden, StoreKind, balloons_for, list_balloons, survey};
use crate::catalog::test_support::{
    TARGET, catalog_warns, put, put_balloon, put_ghost_in, two_roots,
};
use crate::catalog::{BalloonEntry, scan_ghost_balloons};
use crate::test_log_capture::capture;

/// `balloon\` の直下の `type` 無し・`type,balloon` は数え、`root` はゴーストのフォルダ・`dir` は
/// `<ゴースト>\balloon\<名>`。ゴーストの直下・`balloon\` そのもの・さらに下は数えない。
/// `type,shell` は除いて `catalog_type_not_balloon` を 1 件だけ残す（要件 1.1〜1.3）。
#[test]
fn ghost_balloon_dir_counts_direct_children_by_root_rules() {
    let tmp = TempPath::under_target("ghost-inner-balloon-scan");
    let g = put_ghost_in(tmp.path(), "ghost_a", "name,G");
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
    assert_eq!(catalog_warns(&events), ["catalog_type_not_balloon"]);
}

/// `balloon\` が無ければ 0 件で記録も 0 件（要件 1.1・10.3）。
#[test]
fn ghost_without_balloon_dir_lists_zero_silently() {
    let tmp = TempPath::under_target("ghost-inner-balloon-none");
    let g = put_ghost_in(tmp.path(), "ghost_a", "name,G");

    let mut listed = vec![];
    let events = capture(|| listed = scan_ghost_balloons(&g));

    assert!(listed.is_empty());
    assert!(events.iter().all(|e| e.target != TARGET));
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
    let g = put_ghost_in(tmp.path(), "ghost_a", "name,G");
    let g_only = put_balloon(&g, "zeta", "Zeta");
    let g_same = put_balloon(&g, "Shared", "Inner");
    let g_case = put_balloon(&g, "mix", "InnerMix");
    let r_only = put_balloon(&r, "alpha", "Alpha");
    let r_same = put_balloon(&r, "Shared", "Outer");
    let r_case = put_balloon(&r, "MIX", "OuterMix");

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
    let (tmp, a, b, roots) = two_roots("ghost-inner-balloon-for-none");
    let g = put_ghost_in(tmp.path(), "ghost_a", "name,G");
    put_balloon(&a, "dup", "Upper");
    put_balloon(&b, "dup", "Lower");
    let installed = put_balloon(&b, "companion_old", "Old");
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
    let g1 = put_ghost_in(tmp.path(), "ghost_1", "name,G");
    let g2 = put_ghost_in(tmp.path(), "ghost_2", "name,G");
    let in1 = put_balloon(&g1, "shared", "One");
    let in2 = put_balloon(&g2, "only2", "Two");
    let r_shared = put_balloon(&r, "shared", "Root");

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
