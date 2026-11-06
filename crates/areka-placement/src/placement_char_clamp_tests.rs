//! `char2` 以降の鍵を持つゴーストでも、配置の scope は 0・1 だけ
//! （spec: areka-P0-emily-ghost-verification 要件 4.7・6.7）。
//!
//! 3 人目以降の窓は作らない。作ると debug 版の `debug_assert!(scope <= 1)`
//! （`input_events::char_scope`・`menu::captions::visible_resource_for`）で落ちる。

use std::fs;

use super::shared_test_support::{
    EMILY_SHAPED_GHOST_DESCRIPT, EMILY_SHAPED_SHELL_DESCRIPT, TempDir, emily_shaped_scopes,
    emo2_root, synth_balloon_dir, with_com_initialized,
};
use super::*;

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
        let ghost_master = root.path().join("ghost").join("master");
        let shell_master = root.path().join("shell").join("master");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        fs::create_dir_all(&shell_master).expect("create shell/master");
        fs::write(
            ghost_master.join("descript.txt"),
            EMILY_SHAPED_GHOST_DESCRIPT,
        )
        .expect("ghost descript");
        fs::write(
            shell_master.join("descript.txt"),
            EMILY_SHAPED_SHELL_DESCRIPT,
        )
        .expect("shell descript");
        fs::write(
            shell_master.join("surfaces.txt"),
            "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\n",
        )
        .expect("surfaces.txt");
        for png in ["surface0.png", "surface10.png"] {
            fs::copy(
                emo2_root().join("shell/master").join(png),
                shell_master.join(png),
            )
            .unwrap_or_else(|e| panic!("{png} 複写: {e}"));
        }
        let balloon_dir = synth_balloon_dir(&root, "balloon", "");

        let prepared = prepare_ghost_windows_for_shell(root.path(), &balloon_dir, Some("master"))
            .expect("えみりの形のゴーストの準備は成功する");

        let scopes: Vec<usize> = prepared.placements.iter().map(|p| p.scope).collect();
        assert_eq!(scopes, vec![0, 1]);
    });
}
