//! 2 つの根の和の上での切替先の決め方の決定論テスト（areka-P0-baseware-root-list task 4.1）。
//!
//! 確かめること: `\![change,ghost,…]` の照合（`name`・フォルダ名）が両方の根のゴーストに届き、
//! フォルダ名は 1 つの規則（大文字小文字を区別しない）で比べ、`name` は今のまま区別すること。
//! `sequential` と `lastinstalled` も和の並びで引けること（要件 2.8・3.5・4.2・12.3）。

use areka_ghost::catalog_roots::list_ghosts;
use areka_ghost::{BasewareRoot, BasewareRoots};
use temp_path_kit::TempPath;

use super::*;

/// 根 1 に `Alpha`（name＝Alice）、根 2 に `beta`（name＝Bob）と隠れる `ALPHA` を組んだ和。
fn two_root_union(tmp: &TempPath) -> Vec<GhostEntry> {
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
    put("one", "Alpha", "charset,UTF-8\nname,Alice\n");
    put("two", "beta", "charset,UTF-8\nname,Bob\n");
    put("two", "ALPHA", "charset,UTF-8\nname,Hidden\n");
    list_ghosts(&BasewareRoots::new(vec![
        BasewareRoot::new(tmp.child("one")),
        BasewareRoot::new(tmp.child("two")),
    ]))
}

fn folder_of(entries: &[GhostEntry], spec: GhostSpec) -> Option<String> {
    resolve_switch_target(entries, &spec).map(|t| t.folder)
}

#[test]
fn change_ghost_matches_name_and_folder_across_two_roots() {
    let tmp = TempPath::under_target("ghost-switch-two-roots");
    let entries = two_root_union(&tmp);
    let name = |s: &str| GhostSpec::Name(s.to_owned());
    let folder = |s: &str| GhostSpec::Folder(s.to_owned());
    assert_eq!(
        [
            folder_of(&entries, name("Bob")),
            folder_of(&entries, name("Alice")),
            folder_of(&entries, name("BETA")),
            folder_of(&entries, folder("alpha")),
            folder_of(&entries, name("bob")),
            folder_of(&entries, name("Hidden")),
        ],
        [
            Some("beta".to_owned()),
            Some("Alpha".to_owned()),
            Some("beta".to_owned()),
            Some("Alpha".to_owned()),
            None,
            None,
        ],
        "(下の根の name・上の根の name・フォルダ名は大小を問わない・name は区別・隠れは引けない)"
    );
}

#[test]
fn sequential_and_lastinstalled_walk_the_union() {
    let tmp = TempPath::under_target("ghost-switch-two-roots-special");
    let entries = two_root_union(&tmp);
    let special = |name, current, last| {
        resolve_special_name(name, &entries, current, last, |_| {
            unreachable!("乱数は呼ばない")
        })
    };
    let resolved = |folder: &str, position| NameResolution::Resolved {
        folder: folder.to_owned(),
        position,
    };
    assert_eq!(
        [
            special("sequential", Some("Alpha"), None),
            special("sequential", Some("alpha"), None),
            special("sequential", Some("beta"), None),
            special("lastinstalled", Some("Alpha"), Some("BETA")),
        ],
        [
            resolved("beta", Some(0)),
            resolved("beta", Some(0)),
            resolved("Alpha", Some(1)),
            resolved("beta", None),
        ],
        "(根をまたいで次へ・今のゴーストも 1 つの規則・末尾から先頭・lastinstalled も 1 つの規則)"
    );
}
