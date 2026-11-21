//! `char2` 以降の鍵を持つゴーストでも、配置の scope は 0・1 だけ
//! （spec: areka-P0-emily-ghost-verification 要件 4.7・6.7）。
//!
//! 3 人目以降の窓は作らない。作ると debug 版の `debug_assert!(scope <= 1)`
//! （`areka_input::char_scope`・`menu::captions::visible_resource_for`）で落ちる。

use std::fs;

use super::shared_test_support::TempDir;
use super::*;
use crate::sample_test_support::{balloon_root, emo2_root};
use crate::test_support::{
    EMILY_SHAPED_GHOST_DESCRIPT, EMILY_SHAPED_SHELL_DESCRIPT, emily_shaped_scopes,
    synth_declared_dpi_ghost, with_com_initialized,
};

/// えみりの形の固定入力から、配置の設定は scope 0・1 だけを返す。
#[test]
fn emily_shaped_config_has_scopes_zero_and_one_only() {
    assert_eq!(emily_shaped_scopes(), vec![0, 1]);
}

/// シェルの切替の準備（`prepare_ghost_windows_for_shell`）でも、配置は scope 0・1 だけ。
#[test]
fn emily_shaped_shell_switch_prepare_places_scopes_zero_and_one_only() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) =
            synth_declared_dpi_ghost(root.path(), &emo2_root(), &balloon_root(), "96", "96", None);
        fs::write(
            ghost_root.join("ghost/master/descript.txt"),
            EMILY_SHAPED_GHOST_DESCRIPT,
        )
        .expect("ghost descript");
        fs::write(
            ghost_root.join("shell/master/descript.txt"),
            EMILY_SHAPED_SHELL_DESCRIPT,
        )
        .expect("shell descript");

        let prepared = prepare_ghost_windows_for_shell(&ghost_root, &balloon_dir, Some("master"))
            .expect("えみりの形のゴーストの準備は成功する");

        let scopes: Vec<usize> = prepared.placements.iter().map(|p| p.scope).collect();
        assert_eq!(scopes, vec![0, 1]);
    });
}
