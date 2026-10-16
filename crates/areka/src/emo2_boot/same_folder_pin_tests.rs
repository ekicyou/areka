//! 同じフォルダ名の差し替えの決定論テスト（areka-P0-baseware-root-list task 4.2・要件 8.4・8.7・12.5）。

use areka_ghost::catalog_roots::list_ghosts;
use areka_ghost::{BasewareRoot, BasewareRoots};
use temp_path_kit::TempPath;

use super::*;
use crate::boot_resolve::GhostRoute;
use crate::emo2_boot::ghost_switch::{GhostSpec, resolve_switch_target};

/// 根 `one` に今のゴースト `Alpha`、根 `two` に同じフォルダ名の `ALPHA` と別の `beta` を組む。
fn put_two_roots(tmp: &TempPath) {
    let put = |root: &str, folder: &str, descript: &str| {
        let master = tmp
            .child(root)
            .join("ghost")
            .join(folder)
            .join("ghost")
            .join("master");
        std::fs::create_dir_all(&master).expect("フォルダを組む");
        std::fs::write(master.join("descript.txt"), descript).expect("descript");
    };
    put(
        "one",
        "Alpha",
        "charset,UTF-8\nname,Alice\nsakura.name,アリス\n",
    );
    put(
        "two",
        "ALPHA",
        "charset,UTF-8\nname,Other\nsakura.name,ほか\n",
    );
    put("two", "beta", "charset,UTF-8\nname,Bob\nsakura.name,ボブ\n");
}

fn current(tmp: &TempPath) -> GhostDecision {
    GhostDecision {
        route: GhostRoute::Memory,
        dir: tmp.child("one").join("ghost").join("Alpha"),
        folder: Some("Alpha".to_owned()),
    }
}

/// 今のゴーストの根を外した和（下の根の `ALPHA` が見える）から選ぶ。
fn pick_after_removing_current_root(tmp: &TempPath, want: &str) -> SwitchTarget {
    let entries = list_ghosts(&BasewareRoots::new(vec![BasewareRoot::new(
        tmp.child("two"),
    )]));
    resolve_switch_target(&entries, &GhostSpec::Folder(want.to_owned())).expect("和に在る")
}

#[test]
fn same_folder_from_another_root_pins_to_current_ghost() {
    let tmp = TempPath::under_target("same-folder-pin-kept");
    put_two_roots(&tmp);
    let now = current(&tmp);
    let seen = pick_after_removing_current_root(&tmp, "alpha");
    assert_eq!(seen.dir, tmp.child("two").join("ghost").join("ALPHA"));
    let pinned = pin_to_current(seen, &now);
    assert_eq!(
        (
            pinned.dir,
            pinned.name,
            pinned.sakura_name,
            running_name(&now)
        ),
        (
            now.dir.clone(),
            "Alice".to_owned(),
            Some("アリス".to_owned()),
            "Alice".to_owned()
        ),
        "(場所・名前・さくら側の名前は今のゴースト・起動中の印の名前も今のゴースト)"
    );
}

#[test]
fn other_folder_or_same_place_is_untouched() {
    let tmp = TempPath::under_target("same-folder-pin-untouched");
    put_two_roots(&tmp);
    let now = current(&tmp);
    let other = pick_after_removing_current_root(&tmp, "beta");
    let itself = SwitchTarget {
        dir: now.dir.clone(),
        folder: "Alpha".to_owned(),
        name: "Alice".to_owned(),
        sakura_name: None,
    };
    let argv = GhostDecision {
        folder: None,
        ..now.clone()
    };
    let seen = pick_after_removing_current_root(&tmp, "alpha");
    assert_eq!(
        [
            pin_to_current(other.clone(), &now),
            pin_to_current(itself.clone(), &now),
            pin_to_current(seen.clone(), &argv),
        ],
        [other, itself, seen],
        "(別のフォルダ名・同じ場所・フォルダ名の無い今のゴーストは手を触れない)"
    );
}
