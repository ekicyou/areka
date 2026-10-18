//! 一時の置き場と置き場の並びの記憶・編集の判断の決定論テスト
//! （areka-P0-startup-options-core 要件 3.4・3.6・3.7・11.2）。
//!
//! 実在する根は `TempPath::under_target`（ワークツリーの `target\` の下）に掘る。

use super::*;
use crate::roots::encode_memory;
use areka_ghost::{BasewareRoot, BasewareRoots};
use std::path::PathBuf;
use temp_path_kit::TempPath;

fn dir(tmp: &TempPath, name: &str) -> PathBuf {
    let d = tmp.path().join(name);
    std::fs::create_dir_all(d.join("ghost")).unwrap();
    d
}

/// 先頭に一時の置き場 `t`、続いて覚える置き場の並び。
fn with_temp(t: &PathBuf, rest: &[&PathBuf]) -> BasewareRoots {
    let mut list = vec![BasewareRoot::temporary(t.clone())];
    list.extend(rest.iter().map(|d| BasewareRoot::new((*d).clone())));
    BasewareRoots::new(list)
}

#[test]
fn temporary_root_is_not_written_to_memory() {
    let tmp = TempPath::under_target("roots-edit-temp-encode");
    let t = dir(&tmp, "t");
    let a = dir(&tmp, "a");
    let b = dir(&tmp, "b");
    assert_eq!(
        encode_memory(&with_temp(&t, &[&a, &b]), None),
        encode_memory(
            &BasewareRoots::new(vec![
                BasewareRoot::new(a.clone()),
                BasewareRoot::new(b.clone())
            ]),
            None
        ),
    );
}

#[test]
fn remove_refuses_temporary_root() {
    let tmp = TempPath::under_target("roots-edit-temp-remove");
    let t = dir(&tmp, "t");
    let a = dir(&tmp, "a");
    let b = dir(&tmp, "b");
    let v = decide_remove(&with_temp(&t, &[&a, &b]), RootsOrigin::Memory, &t);
    assert_eq!(v, RemoveVerdict::Temporary);
}

#[test]
fn remove_counts_last_one_among_remembered_roots() {
    let tmp = TempPath::under_target("roots-edit-temp-last");
    let t = dir(&tmp, "t");
    let a = dir(&tmp, "a");
    let v = decide_remove(&with_temp(&t, &[&a]), RootsOrigin::Memory, &a);
    assert_eq!(v, RemoveVerdict::LastOne);
}

#[test]
fn front_refuses_temporary_root() {
    let tmp = TempPath::under_target("roots-edit-temp-front-refuse");
    let t = dir(&tmp, "t");
    let a = dir(&tmp, "a");
    let v = decide_front(&with_temp(&t, &[&a]), RootsOrigin::Memory, &t);
    assert_eq!(v, FrontVerdict::Temporary);
}

#[test]
fn front_moves_to_head_of_remembered_roots_temporary_stays_first() {
    let tmp = TempPath::under_target("roots-edit-temp-front-move");
    let t = dir(&tmp, "t");
    let a = dir(&tmp, "a");
    let b = dir(&tmp, "b");
    let v = decide_front(&with_temp(&t, &[&a, &b]), RootsOrigin::Memory, &b);
    assert_eq!(
        v,
        FrontVerdict::Moved {
            roots: with_temp(&t, &[&b, &a])
        }
    );
    // 覚える置き場の先頭はすでに先頭。
    let v = decide_front(&with_temp(&t, &[&a, &b]), RootsOrigin::Memory, &a);
    assert_eq!(v, FrontVerdict::AlreadyFront);
}

#[test]
fn adding_same_folder_as_temporary_yields_normal_root() {
    let tmp = TempPath::under_target("roots-edit-temp-add");
    let t = dir(&tmp, "t");
    let a = dir(&tmp, "a");
    let v = decide_add(&with_temp(&t, &[&a]), RootsOrigin::Memory, &t);
    let AddVerdict::Added { roots, empty } = v else {
        panic!("足せていない: {v:?}");
    };
    assert!(!empty);
    let mut expected = with_temp(&t, &[&a]).roots().to_vec();
    expected.push(BasewareRoot::new(t.clone()));
    assert_eq!(roots, BasewareRoots::new(expected));
    assert!(!roots.roots()[2].is_temporary());
    // 足した普通の置き場は記憶に出る。
    assert_eq!(
        encode_memory(&roots, None),
        encode_memory(
            &BasewareRoots::new(vec![
                BasewareRoot::new(a.clone()),
                BasewareRoot::new(t.clone())
            ]),
            None
        ),
    );
}
