//! 一時の置き場を `.nar` の入れ先にしない決定論テスト（areka-P0-startup-options-core task 3.4）。
//!
//! 確かめること: 一時の置き場のゴーストが動いていても入れ先は覚える置き場・同じフォルダ名の相手は
//! 覚える置き場の方を見る・一時の置き場しか見つからなければ作れない（要件 3.5・3.9）。
//! 一時の置き場ではゴーストは `<一時の置き場>\<フォルダ>` にいる（`ghost\` を挟まない）。

use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_nar::InstallRoots;
use temp_path_kit::TempPath;

use super::*;
use crate::placement::test_support::capture_logs;

/// 一時の置き場（フォルダそのものがゴーストの置き場）にゴーストを置く。
fn put_temporary_ghost(temp: &Path, folder: &str) -> PathBuf {
    let dir = temp.join(folder);
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\nname,T\n").expect("descript");
    dir
}

fn put_ghost(root: &Path, folder: &str) {
    put_temporary_ghost(&root.join("ghost"), folder);
}

fn list_with_temporary(temp: &Path, dirs: &[PathBuf]) -> BasewareRoots {
    let mut roots = vec![BasewareRoot::temporary(temp.to_path_buf())];
    roots.extend(dirs.iter().cloned().map(BasewareRoot::new));
    BasewareRoots::new(roots)
}

/// 動いているゴーストが一時の置き場にいるとき、その名前でも「2 つ上が根」とせず覚える置き場へ入れる。
#[test]
fn running_ghost_in_temporary_root_installs_into_remembered_root() {
    let tmp = TempPath::under_target("install-destination-temporary-running");
    let (temp, one) = (tmp.child("pointed"), tmp.child("one"));
    let running = put_temporary_ghost(&temp, "Emo");
    std::fs::create_dir_all(&one).expect("根 1");
    let dest = CatalogDestinations::new(list_with_temporary(&temp, std::slice::from_ref(&one)))
        .expect("作れる")
        .with_running(Some(running));
    let (answers, events) = capture_logs(|| [dest.ghost_root("emo"), dest.ghost_root("Fresh")]);
    assert_eq!(
        answers,
        [one.clone(), one],
        "(動いている名前・新しい名前とも覚える置き場)"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.field_str("event") == Some("install_running_ghost_root_kept")),
        "一時の置き場の 2 つ上を根とみなさない"
    );
}

/// 同じフォルダ名のゴーストが覚える置き場にあれば、先頭の一時の置き場ではなくその根へ入れる。
#[test]
fn same_folder_counterpart_skips_temporary_root() {
    let tmp = TempPath::under_target("install-destination-temporary-counterpart");
    let (temp, one, two) = (tmp.child("pointed"), tmp.child("one"), tmp.child("two"));
    put_temporary_ghost(&temp, "Emo");
    std::fs::create_dir_all(&one).expect("根 1");
    put_ghost(&two, "emo");
    let dest =
        CatalogDestinations::new(list_with_temporary(&temp, &[one, two.clone()])).expect("作れる");
    assert_eq!(dest.ghost_root("EMO"), two, "覚える置き場の同じフォルダ名");
}

/// 一時の置き場しか見つからなければ入れ先は作れない（控えの根にもしない）。
#[test]
fn only_temporary_root_found_cannot_be_built() {
    let tmp = TempPath::under_target("install-destination-temporary-only");
    let temp = tmp.child("pointed");
    put_temporary_ghost(&temp, "Emo");
    assert!(CatalogDestinations::new(list_with_temporary(&temp, &[tmp.child("gone")])).is_none());
}
