//! 一時の置き場が並びに在るときの「置き場を外す ▸」「置き場を先頭へ ▸」の決定論テスト
//! （areka-P0-startup-options-core 要件 3.6）。
//!
//! 確かめること: 一時の項目の子は外す・先頭へとも選べない・「最後の 1 つ」と見出しの判定は
//! 覚える置き場の数で数える・「先頭へ」は覚える置き場の先頭の子を選べない。項目の動作は呼ばない。

use std::path::PathBuf;

use areka_ghost::BasewareRoot;
use temp_path_kit::TempPath;

use super::tests::{children, item, label, wired_world_roots};
use super::*;
use crate::roots::RootsOrigin;

/// 先頭に一時の置き場 `t`、続いて覚える置き場 `rest` の並びで据えた World。
fn world_with_temp(t: &PathBuf, rest: &[&PathBuf]) -> World {
    let mut list = vec![BasewareRoot::temporary(t.clone())];
    list.extend(rest.iter().map(|d| BasewareRoot::new((*d).clone())));
    wired_world_roots(list, RootsOrigin::Memory)
}

fn dirs(tmp: &TempPath, names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|n| {
            let d = tmp.path().join(n);
            std::fs::create_dir_all(d.join("ghost")).unwrap();
            d
        })
        .collect()
}

#[test]
fn remove_disables_temporary_child_only() {
    let tmp = TempPath::under_target("roots_frame_temp_remove");
    let [t, a, b] = <[PathBuf; 3]>::try_from(dirs(&tmp, &["t", "a", "b"])).unwrap();
    let remove = item(&world_with_temp(&t, &[&a, &b]), Frame::RootRemove);
    assert_eq!(
        (remove.enabled, children(&remove)),
        (
            true,
            vec![(label(&t), false), (label(&a), true), (label(&b), true)]
        )
    );
}

#[test]
fn remove_counts_last_one_among_remembered_roots() {
    let tmp = TempPath::under_target("roots_frame_temp_remove_last");
    let [t, a] = <[PathBuf; 2]>::try_from(dirs(&tmp, &["t", "a"])).unwrap();
    let remove = item(&world_with_temp(&t, &[&a]), Frame::RootRemove);
    assert_eq!(
        children(&remove),
        vec![(label(&t), false), (label(&a), false)]
    );
}

#[test]
fn front_disables_temporary_child_and_remembered_head() {
    let tmp = TempPath::under_target("roots_frame_temp_front");
    let [t, a, b] = <[PathBuf; 3]>::try_from(dirs(&tmp, &["t", "a", "b"])).unwrap();
    let front = item(&world_with_temp(&t, &[&a, &b]), Frame::RootFront);
    assert_eq!(
        (front.enabled, children(&front)),
        (
            true,
            vec![(label(&t), false), (label(&a), false), (label(&b), true)]
        )
    );
}

#[test]
fn front_header_counts_remembered_roots() {
    let tmp = TempPath::under_target("roots_frame_temp_front_header");
    let [t, a] = <[PathBuf; 2]>::try_from(dirs(&tmp, &["t", "a"])).unwrap();
    let front = item(&world_with_temp(&t, &[&a]), Frame::RootFront);
    assert_eq!(
        (front.enabled, children(&front)),
        (false, vec![(label(&t), false), (label(&a), false)])
    );
}
