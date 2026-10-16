//! `catalog_roots` の判断の檻（task 1.1: フォルダ名の比べ方・同じ根の比べ方・今見つかる根）。
//!
//! 一時フォルダは `temp-path-kit` の `under_target` の下だけに組む。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{BasewareRoots, fold_key, same_folder_name, same_root};
use crate::catalog::{BasewareRoot, list_balloons, list_ghosts};

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
    assert_eq!(list_ghosts(root)[0].root, dir);
    assert_eq!(list_balloons(root)[0].root, dir);
}
