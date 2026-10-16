//! `catalog_roots` の判断の檻（task 1.1: フォルダ名の比べ方・同じ根の比べ方・今見つかる根／
//! task 1.2: 和・隠れ・引き当て・入れる根）。
//!
//! 一時フォルダは `temp-path-kit` の `under_target` の下だけに組む。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{
    BasewareRoots, Hidden, InstallRoot, StoreKind, find_balloon, find_ghost, fold_key,
    install_root, list_balloons, list_ghosts, same_folder_name, same_root, survey,
};
use crate::catalog::{
    BasewareRoot, GhostEntry, list_balloons as scan_balloons, list_ghosts as scan_ghosts,
};

/// ASCII・全角英字・ギリシアは大文字小文字を区別しない（要件 2.8）。
#[test]
fn folder_names_fold_ascii_fullwidth_greek() {
    assert!(same_folder_name("Emo2", "emo2"));
    assert!(same_folder_name("ＡＢＣ", "ａｂｃ"));
    assert!(same_folder_name("ΣΟΦΙΑ", "σοφια"));
    assert!(!same_folder_name("emo2", "emo3"));
}

/// 2 字以上へ広がる字（`ß` → `SS`）は写さずそのまま残す（NTFS の 1 字 → 1 字の表に合わせる）。
#[test]
fn multi_char_uppercase_is_left_as_is() {
    assert_eq!(fold_key("ß"), "ß");
    assert_eq!(fold_key("straße"), "STRAßE");
    assert!(!same_folder_name("straße", "STRASSE"));
}

/// 末尾の区切りと大文字小文字だけ違う根は同じ根（要件 1.9）。違う根は違う。
#[test]
fn roots_differing_by_trailing_separator_or_case_are_same() {
    assert!(same_root(
        Path::new(r"C:\Areka\Root\"),
        Path::new(r"c:\areka\root")
    ));
    assert!(same_root(
        Path::new(r"C:\Areka\Root/"),
        Path::new(r"C:\Areka\Root")
    ));
    assert!(!same_root(
        Path::new(r"C:\Areka\Root"),
        Path::new(r"C:\Areka\Root2")
    ));
}

/// 見つからない根は「今見つかる根」から外れ、並びには残る。見つかったら呼ぶたびに入り直す（要件 9.3）。
#[test]
fn missing_roots_drop_out_of_present_and_come_back() {
    let tmp = TempPath::under_target("catalog-roots-present");
    let a = tmp.child("a");
    let b = tmp.child("b");
    std::fs::create_dir_all(&a).unwrap();
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(a.clone()),
        BasewareRoot::new(b.clone()),
    ]);

    let present = |r: &BasewareRoots| -> Vec<PathBuf> {
        r.present().map(|x| x.dir().to_path_buf()).collect()
    };
    assert_eq!(roots.roots().len(), 2);
    assert_eq!(present(&roots), std::slice::from_ref(&a));
    assert_eq!(roots.ghost_stores(), [a.join("ghost")]);
    assert_eq!(roots.balloon_stores(), [a.join("balloon")]);

    std::fs::create_dir_all(&b).unwrap();
    assert_eq!(present(&roots), [a.clone(), b.clone()]);
    assert_eq!(roots.ghost_stores(), [a.join("ghost"), b.join("ghost")]);
}

/// 根 1 つの並びと、1 根の走査が項目の `root` を詰めること（要件 3.4）。
#[test]
fn single_root_and_entries_carry_their_root() {
    let tmp = TempPath::under_target("catalog-roots-entry");
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(dir.join(r"ghost\g\ghost\master")).unwrap();
    std::fs::write(dir.join(r"ghost\g\ghost\master\descript.txt"), "name,G\r\n").unwrap();
    std::fs::create_dir_all(dir.join(r"balloon\b")).unwrap();
    std::fs::write(dir.join(r"balloon\b\descript.txt"), "name,B\r\n").unwrap();

    let roots = BasewareRoots::single(dir.clone());
    assert_eq!(roots.roots(), [BasewareRoot::new(dir.clone())]);
    let root = &roots.roots()[0];
    assert_eq!(scan_ghosts(root)[0].root, dir);
    assert_eq!(scan_balloons(root)[0].root, dir);
}

/// `<根>/ghost/<folder>/ghost/master/descript.txt` を置く（一覧に出せるゴースト）。
fn put_ghost(root: &Path, folder: &str) -> PathBuf {
    let dir = root.join("ghost").join(folder);
    std::fs::create_dir_all(dir.join(r"ghost\master")).unwrap();
    std::fs::write(dir.join(r"ghost\master\descript.txt"), "name,G\r\n").unwrap();
    dir
}

/// `<根>/balloon/<folder>/descript.txt` を置く（一覧に出せるバルーン）。
fn put_balloon(root: &Path, folder: &str) -> PathBuf {
    let dir = root.join("balloon").join(folder);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("descript.txt"), "name,B\r\n").unwrap();
    dir
}

/// 見つかる 2 つの根 `a`（上）・`b`（下）の並び。
fn two_roots(label: &str) -> (TempPath, PathBuf, PathBuf, BasewareRoots) {
    let tmp = TempPath::under_target(label);
    let a = tmp.child("a");
    let b = tmp.child("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(a.clone()),
        BasewareRoot::new(b.clone()),
    ]);
    (tmp, a, b, roots)
}

fn ghost_dirs(entries: &[GhostEntry]) -> Vec<PathBuf> {
    entries.iter().map(|e| e.dir.clone()).collect()
}

/// 2 つの根の同じフォルダ名は上だけが見え、下は隠れとして控えられる。ゴーストとバルーンは
/// 別々に判断し、フォルダ名 `x` のゴーストと `x` のバルーンは互いを隠さない（要件 2.1〜2.4・3.1〜3.3）。
#[test]
fn union_hides_lower_same_name_per_kind_and_sorts_by_folder() {
    let (_tmp, a, b, roots) = two_roots("catalog-roots-union");
    let ga = put_ghost(&a, "g");
    let gb = put_ghost(&b, "g");
    let only_b = put_ghost(&b, "a_only_b");
    let ba = put_balloon(&a, "bal");
    let bb = put_balloon(&b, "bal");
    // フォルダ名 x: a にゴースト・b にバルーン → 互いを隠さない。
    let xg = put_ghost(&a, "x");
    let xb = put_balloon(&b, "x");

    let s = survey(&roots);
    assert_eq!(ghost_dirs(&s.ghosts), [only_b, ga.clone(), xg.clone()]);
    let balloon_dirs: Vec<PathBuf> = s.balloons.iter().map(|e| e.dir.clone()).collect();
    assert_eq!(balloon_dirs, [ba.clone(), xb.clone()]);
    assert_eq!(s.ghosts[0].root, b);
    assert_eq!(
        s.hidden,
        [
            Hidden {
                kind: StoreKind::Ghost,
                folder: "g".into(),
                hidden: gb,
                by: ga.clone(),
            },
            Hidden {
                kind: StoreKind::Balloon,
                folder: "bal".into(),
                hidden: bb,
                by: ba.clone(),
            },
        ]
    );
    assert_eq!(list_ghosts(&roots), s.ghosts);
    assert_eq!(list_balloons(&roots), s.balloons);

    assert_eq!(find_ghost(&roots, "g").unwrap().dir, ga);
    assert_eq!(find_balloon(&roots, "bal").unwrap().dir, ba);
    assert_eq!(find_ghost(&roots, "x").unwrap().dir, xg);
    assert_eq!(find_balloon(&roots, "x").unwrap().dir, xb);
    assert!(find_ghost(&roots, "none").is_none());
}

/// descript の無い上の `x` は一覧に出せないので、下の根の `x` を隠さない（要件 2.7）。
#[test]
fn unlistable_upper_folder_does_not_hide_lower() {
    let (_tmp, a, b, roots) = two_roots("catalog-roots-unlistable");
    std::fs::create_dir_all(a.join(r"ghost\x")).unwrap();
    std::fs::create_dir_all(a.join(r"balloon\x")).unwrap();
    let gx = put_ghost(&b, "x");
    let bx = put_balloon(&b, "x");

    assert!(survey(&roots).hidden.is_empty());
    assert_eq!(find_ghost(&roots, "x").unwrap().dir, gx);
    assert_eq!(find_balloon(&roots, "x").unwrap().dir, bx);
}

/// 大文字小文字だけ違うフォルダ名は同じ名前: 上の `Emo2` が下の `emo2` を隠し、どちらの綴りでも
/// 上が引ける（要件 2.8）。
#[test]
fn case_only_difference_hides_and_finds() {
    let (_tmp, a, b, roots) = two_roots("catalog-roots-case");
    let upper = put_ghost(&a, "Emo2");
    let lower = put_ghost(&b, "emo2");

    let s = survey(&roots);
    assert_eq!(ghost_dirs(&s.ghosts), std::slice::from_ref(&upper));
    assert_eq!(
        s.hidden,
        [Hidden {
            kind: StoreKind::Ghost,
            folder: "emo2".into(),
            hidden: lower,
            by: upper.clone(),
        }]
    );
    assert_eq!(find_ghost(&roots, "emo2").unwrap().dir, upper);
    assert_eq!(find_ghost(&roots, "EMO2").unwrap().dir, upper);
}

/// 見つからない根は 0 件として読み、下の根のものが見える（要件 9.3・3.1）。
#[test]
fn missing_root_reads_as_zero_items() {
    let tmp = TempPath::under_target("catalog-roots-missing");
    let gone = tmp.child("gone");
    let b = tmp.child("b");
    let g = put_ghost(&b, "g");
    let roots = BasewareRoots::new(vec![BasewareRoot::new(gone), BasewareRoot::new(b)]);

    let s = survey(&roots);
    assert_eq!(ghost_dirs(&s.ghosts), [g]);
    assert!(s.balloons.is_empty());
    assert!(s.hidden.is_empty());
}

/// 入れる根の 4 つの分岐（要件 5.1・5.2・5.3）。
#[test]
fn install_root_branches() {
    let tmp = TempPath::under_target("catalog-roots-install");
    let gone = tmp.child("gone");
    let a = tmp.child("a");
    let b = tmp.child("b");
    std::fs::create_dir_all(&a).unwrap();
    put_ghost(&b, "g");
    put_balloon(&b, "bal");
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(a.clone()),
        BasewareRoot::new(b.clone()),
    ]);
    let at = |root: &PathBuf| {
        Some(InstallRoot {
            root: root.clone(),
            head_skipped: None,
        })
    };

    // ① 見えているものがあればその根（種類ごとに・フォルダ名は大文字小文字を区別しない）。
    assert_eq!(install_root(&roots, StoreKind::Ghost, "G"), at(&b));
    assert_eq!(install_root(&roots, StoreKind::Balloon, "bal"), at(&b));
    // ② 無ければ先頭の根（バルーンの `g` は無い）。
    assert_eq!(install_root(&roots, StoreKind::Ghost, "new"), at(&a));
    assert_eq!(install_root(&roots, StoreKind::Balloon, "g"), at(&a));

    // ③ 先頭が見つからなければ見つかるいちばん上の根へ、飛ばした先頭を添える。
    let skipped = BasewareRoots::new(vec![
        BasewareRoot::new(gone.clone()),
        BasewareRoot::new(a.clone()),
        BasewareRoot::new(b.clone()),
    ]);
    assert_eq!(
        install_root(&skipped, StoreKind::Ghost, "new"),
        Some(InstallRoot {
            root: a.clone(),
            head_skipped: Some(gone.clone()),
        })
    );
    // 見えているものがあれば、先頭を飛ばしていても添えない。
    assert_eq!(install_root(&skipped, StoreKind::Ghost, "g"), at(&b));

    // ④ 見つかる根が無ければ無し。
    let none = BasewareRoots::new(vec![BasewareRoot::new(gone)]);
    assert_eq!(install_root(&none, StoreKind::Ghost, "new"), None);
}
