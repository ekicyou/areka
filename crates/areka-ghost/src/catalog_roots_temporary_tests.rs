//! 一時の置き場の目録（startup-options-core 要件 3.2・3.5・3.9）。
//!
//! 一時フォルダは `temp-path-kit` の `under_target` の下だけに組む。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{BasewareRoots, StoreKind, find_ghost, install_root, list_balloons, survey};
use crate::catalog::{BasewareRoot, RootMark, scan_balloons};

/// `<store>/<folder>/ghost/master/descript.txt` を置く。
fn put_ghost_in(store: &Path, folder: &str) -> PathBuf {
    let dir = store.join(folder);
    std::fs::create_dir_all(dir.join(r"ghost\master")).unwrap();
    std::fs::write(dir.join(r"ghost\master\descript.txt"), "name,G\r\n").unwrap();
    dir
}

/// 一時の置き場 `t`（ゴーストを直に並べる）を先頭に、普通の根 `a` を後ろに置いた並び。
fn temp_then_plain(label: &str) -> (TempPath, PathBuf, PathBuf, BasewareRoots) {
    let tmp = TempPath::under_target(label);
    let t = tmp.child("t");
    let a = tmp.child("a");
    std::fs::create_dir_all(&t).unwrap();
    std::fs::create_dir_all(&a).unwrap();
    let roots = BasewareRoots::new(vec![
        BasewareRoot::temporary(t.clone()),
        BasewareRoot::new(a.clone()),
    ]);
    (tmp, t, a, roots)
}

/// 一時の項目は印 `Temporary` で、ゴーストの置き場はフォルダそのもの（要件 3.2）。
#[test]
fn temporary_root_ghost_store_is_the_folder_itself() {
    let dir = PathBuf::from(r"C:\x\ghosts");
    let root = BasewareRoot::temporary(dir.clone());
    assert_eq!(root.mark(), RootMark::Temporary);
    assert_eq!(root.ghost_store(), dir);
    assert_eq!(root.ghost_dir("g"), dir.join("g"));
}

/// 先頭の一時の置き場のゴーストが、同じフォルダ名のほかの根のゴーストより先に当たる（要件 3.9）。
#[test]
fn head_temporary_ghost_wins_over_same_folder_name() {
    let (_tmp, t, a, roots) = temp_then_plain("catalog-roots-temp-wins");
    let in_t = put_ghost_in(&t, "G");
    let in_a = put_ghost_in(&a.join("ghost"), "g");

    let hit = find_ghost(&roots, "g").expect("当たる");
    assert_eq!(hit.dir, in_t);
    assert_eq!(hit.root, t);
    let s = survey(&roots);
    assert_eq!(s.ghosts.len(), 1);
    assert_eq!(s.hidden.len(), 1);
    assert_eq!(s.hidden[0].hidden, in_a);
    assert_eq!(s.hidden[0].by, in_t);
    // ゴーストの置き場の列には入る。
    assert_eq!(roots.ghost_stores(), [t.clone(), a.join("ghost")]);
}

/// 一時の項目では `balloon\` を探さず、バルーンの置き場の列にも入れない（要件 3.2）。
#[test]
fn temporary_root_does_not_look_for_balloons() {
    let (_tmp, t, a, roots) = temp_then_plain("catalog-roots-temp-balloon");
    let bal = t.join(r"balloon\b");
    std::fs::create_dir_all(&bal).unwrap();
    std::fs::write(bal.join("descript.txt"), "name,B\r\n").unwrap();

    assert!(scan_balloons(&roots.roots()[0]).is_empty());
    assert!(list_balloons(&roots).is_empty());
    assert_eq!(roots.balloon_stores(), [a.join("balloon")]);
}

/// 新しく入れるものの入れ先は一時の項目を飛ばして決める（要件 3.5）。
#[test]
fn temporary_root_is_never_install_destination() {
    let (_tmp, t, a, roots) = temp_then_plain("catalog-roots-temp-install");
    put_ghost_in(&t, "g");

    // 見えている項目が一時の置き場にあっても、そこへは入れない。
    let seen = install_root(&roots, StoreKind::Ghost, "g").expect("入れ先");
    assert_eq!(seen.root, a);
    assert_eq!(seen.head_skipped, None);
    // 見えないものも、いちばん上の一時の置き場ではなく次の根へ。
    let new = install_root(&roots, StoreKind::Ghost, "new").expect("入れ先");
    assert_eq!(new.root, a);
    assert_eq!(
        install_root(&roots, StoreKind::Balloon, "b").unwrap().root,
        a
    );

    // 一時の置き場しか無ければ入れ先は無い。
    let only = BasewareRoots::new(vec![BasewareRoot::temporary(t.clone())]);
    assert_eq!(install_root(&only, StoreKind::Ghost, "new"), None);
}
