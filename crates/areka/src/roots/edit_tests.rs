//! 置き場の追加と外すの判断の決定論テスト（要件 1.5・6.4・6.6〜6.8・7.2・7.3・8.6・12.5）。
//!
//! 実在する根は `TempPath::under_target`（ワークツリーの `target\` の下）に掘る。

use super::*;
use crate::roots::{EXE_TOKEN, encode_memory};
use areka_ghost::{BasewareRoot, BasewareRoots};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// `target\` の下に実在する根を掘る（`ghost\` を持たせるかどうか）。
fn root(tmp: &TempPath, name: &str, with_ghost: bool) -> PathBuf {
    let dir = tmp.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    if with_ghost {
        std::fs::create_dir_all(dir.join("ghost")).unwrap();
    }
    dir
}

fn roots_of(dirs: &[&Path]) -> BasewareRoots {
    BasewareRoots::new(
        dirs.iter()
            .map(|d| BasewareRoot::new(d.to_path_buf()))
            .collect(),
    )
}

#[test]
fn add_appends_at_end() {
    let tmp = TempPath::under_target("roots-edit-add-append");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let v = decide_add(&roots_of(&[&a]), RootsOrigin::Memory, &b);
    assert_eq!(
        v,
        AddVerdict::Added {
            roots: roots_of(&[&a, &b]),
            empty: false
        }
    );
}

#[test]
fn add_already_listed_is_unchanged() {
    let tmp = TempPath::under_target("roots-edit-add-listed");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    // 大文字小文字と末尾の区切りだけ違う綴りも同じ根（same_root）。
    let picked = PathBuf::from(format!("{}\\", b.to_string_lossy().to_uppercase()));
    let v = decide_add(&roots_of(&[&a, &b]), RootsOrigin::Memory, &picked);
    assert_eq!(v, AddVerdict::AlreadyListed { existing: b });
}

#[test]
fn add_empty_root_is_added_and_marked() {
    let tmp = TempPath::under_target("roots-edit-add-empty");
    let a = root(&tmp, "a", true);
    let e = root(&tmp, "e", false);
    let v = decide_add(&roots_of(&[&a]), RootsOrigin::Memory, &e);
    assert_eq!(
        v,
        AddVerdict::Added {
            roots: roots_of(&[&a, &e]),
            empty: true
        }
    );
    // balloon\ だけでも空ではない。
    let bl = root(&tmp, "bl", false);
    std::fs::create_dir_all(bl.join("balloon")).unwrap();
    assert!(matches!(
        decide_add(&roots_of(&[&a]), RootsOrigin::Memory, &bl),
        AddVerdict::Added { empty: false, .. }
    ));
}

#[test]
fn add_exe_dir_enters_as_absolute_and_memory_writes_exe_token() {
    let tmp = TempPath::under_target("roots-edit-add-exe");
    let exe = root(&tmp, "exe", true);
    let other = root(&tmp, "other", true);
    // 既定でない並び（記憶の並びから実行ファイルの隣を外した後）へ実行ファイルのフォルダを足す。
    let v = decide_add(&roots_of(&[&other]), RootsOrigin::Memory, &exe);
    let AddVerdict::Added { roots, empty } = v else {
        panic!("足せるはず: {v:?}");
    };
    assert!(!empty);
    assert_eq!(roots, roots_of(&[&other, &exe]));
    assert!(roots.roots()[1].dir().is_absolute());
    assert_eq!(
        encode_memory(&roots, Some(&exe)),
        format!("{}|{EXE_TOKEN}", other.to_string_lossy())
    );
}

#[test]
fn add_env_var_is_locked() {
    let tmp = TempPath::under_target("roots-edit-add-env");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_add(&roots_of(&[&a]), RootsOrigin::EnvVar, &b),
        AddVerdict::EnvLocked
    );
}

#[test]
fn remove_takes_out_and_keeps_folder() {
    let tmp = TempPath::under_target("roots-edit-remove");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let c = root(&tmp, "c", true);
    let picked = PathBuf::from(b.to_string_lossy().to_uppercase());
    let v = decide_remove(&roots_of(&[&a, &b, &c]), RootsOrigin::Memory, &picked);
    assert_eq!(
        v,
        RemoveVerdict::Removed {
            roots: roots_of(&[&a, &c]),
            removed: b.clone()
        }
    );
    // フォルダは消さず動かさない（要件 7.2）。
    assert!(b.join("ghost").is_dir());
}

#[test]
fn remove_last_one_is_refused() {
    let tmp = TempPath::under_target("roots-edit-remove-last");
    let a = root(&tmp, "a", true);
    assert_eq!(
        decide_remove(&roots_of(&[&a]), RootsOrigin::Default, &a),
        RemoveVerdict::LastOne
    );
}

#[test]
fn remove_env_var_is_locked() {
    let tmp = TempPath::under_target("roots-edit-remove-env");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_remove(&roots_of(&[&a, &b]), RootsOrigin::EnvVar, &b),
        RemoveVerdict::EnvLocked
    );
}

#[test]
fn remove_not_listed_is_stale() {
    let tmp = TempPath::under_target("roots-edit-remove-stale");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let gone = root(&tmp, "gone", true);
    assert_eq!(
        decide_remove(&roots_of(&[&a, &b]), RootsOrigin::Memory, &gone),
        RemoveVerdict::Stale
    );
}
