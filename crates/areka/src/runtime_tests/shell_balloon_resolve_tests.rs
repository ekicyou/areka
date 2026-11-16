//! 切替先の名前の解決とインストールの控えの決定論テスト（spec: areka-P0-shell-balloon-switch
//! 要件 1.6〜1.11・1.13・12.3〜12.5）。
//!
//! 確かめること: `name` → フォルダ名 → 該当なしの順・大文字小文字の区別・隠しシェルの名指し・
//! 今のものへの切替・`random` の注入の乱数と候補 0・シェルの `lastinstalled` 4 通り（ゴースト切替の
//! 要求 0）・バルーンの `lastinstalled`。目録は一時フォルダに実物の `descript.txt` を置いて読む。

use std::fs;
use std::path::Path;

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use temp_path_kit::TempPath;

use super::test_support::{NO_MEMO, by_name, folder, never};
use super::*;
use crate::session::ghost_switch::SwitchInFlight;

fn write_descript(dir: &Path, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .unwrap();
}

/// 根と、シェル `master`（通常）・`summer`（通常）・`secret`（`menu,hidden`）を持つゴースト `A`。
fn plant_ghost() -> (TempPath, BasewareRoot) {
    let temp = TempPath::under_target("areka-skin-resolve");
    let root = BasewareRoot::new(temp.path().to_path_buf());
    let shell = root.ghost_dir("A").join("shell");
    write_descript(&shell.join("master"), "name,通常");
    write_descript(&shell.join("summer"), "name,夏服");
    write_descript(&shell.join("secret"), "name,ひみつ\r\nmenu,hidden");
    write_descript(&root.balloon_dir("kaku"), "name,かくかく");
    write_descript(&root.balloon_dir("fluffy"), "name,ふわふわ");
    (temp, root)
}

/// シェルの候補は隠しを含み（印つき）、隠しシェルも名指しなら切り替えられる。今のものも候補（要件 1.6・1.8）。
#[test]
fn hidden_shell_by_name_and_the_current_one_are_candidates() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");
    let cands = shell_candidates(&ghost);
    let mut listed: Vec<(&str, bool)> = cands
        .iter()
        .map(|c| (c.folder.as_str(), c.hidden))
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        vec![("master", false), ("secret", true), ("summer", false)]
    );

    let hit = resolve_skin_target(&cands, &by_name("ひみつ"), Some("master"), NO_MEMO, never)
        .expect("隠しシェルも名指しなら解ける");
    assert_eq!(hit.folder, "secret");
    assert_eq!(hit.dir, ghost.join("shell").join("secret"));
    assert_eq!(hit.name.as_deref(), Some("ひみつ"));

    let itself = resolve_skin_target(&cands, &by_name("通常"), Some("master"), NO_MEMO, never);
    assert_eq!(
        folder(itself).as_deref(),
        Ok("master"),
        "今のものへも切り替える"
    );

    let balloons = balloon_candidates(
        &areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
        &ghost,
    );
    let hit = resolve_skin_target(
        &balloons,
        &by_name("かくかく"),
        Some("kaku"),
        NO_MEMO,
        never,
    );
    assert_eq!(
        folder(hit).as_deref(),
        Ok("kaku"),
        "バルーンも今のものが候補"
    );
}

/// 控えを置いた World で、シェルの `lastinstalled` を入口と同じ順（控え → 解決）で解く。
fn shell_lastinstalled(world: &World, ghost: &Path) -> Result<String, NotFoundReason> {
    let installed = installed_for(world, SkinKind::Shell, ghost);
    folder(resolve_skin_target(
        &shell_candidates(ghost),
        &by_name("lastinstalled"),
        Some("master"),
        installed.as_deref().map_err(|r| *r),
        never,
    ))
}

/// シェルの `lastinstalled` の 4 通り。控えのゴーストが今のゴースト（大文字小文字は問わない）で先が
/// 在るときだけ解け、どの場合もゴースト切替の要求は 0（要件 1.10・1.13・裁定 3）。
#[test]
fn shell_lastinstalled_resolves_only_within_the_current_ghost() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");

    let mut world = World::new();
    assert_eq!(
        shell_lastinstalled(&world, &ghost),
        Err(NotFoundReason::LastInstalledNone),
        "控えなし"
    );

    record_installed_shell(&mut world, Some("a".to_owned()), "summer".to_owned());
    assert_eq!(
        shell_lastinstalled(&world, &ghost).as_deref(),
        Ok("summer"),
        "控えあり"
    );

    record_installed_shell(&mut world, Some("B".to_owned()), "summer".to_owned());
    assert_eq!(
        shell_lastinstalled(&world, &ghost),
        Err(NotFoundReason::LastInstalledOtherGhost),
        "別のゴースト"
    );

    record_installed_shell(&mut world, Some("A".to_owned()), "winter".to_owned());
    assert_eq!(
        shell_lastinstalled(&world, &ghost),
        Err(NotFoundReason::LastInstalledMissing),
        "先が無い"
    );

    assert!(
        world.get_non_send::<SwitchInFlight>().is_none(),
        "ゴースト切替の要求は 0"
    );
    assert_eq!(
        world.get_resource::<LastInstalledShell>(),
        Some(&LastInstalledShell {
            ghost_folder: Some("A".to_owned()),
            folder: "winter".to_owned(),
        }),
        "控えは最後の 1 件"
    );
}

/// バルーンの `lastinstalled`: 控えが目録に在るときだけ解ける（要件 1.11）。
#[test]
fn balloon_lastinstalled_resolves_only_when_in_the_catalog() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");
    let resolve = |world: &World| {
        let installed = installed_for(world, SkinKind::Balloon, &ghost);
        folder(resolve_skin_target(
            &balloon_candidates(
                &areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
                &ghost,
            ),
            &by_name("lastinstalled"),
            Some("kaku"),
            installed.as_deref().map_err(|r| *r),
            never,
        ))
    };
    let mut world = World::new();
    assert_eq!(resolve(&world), Err(NotFoundReason::LastInstalledNone));

    record_installed_balloon(&mut world, "fluffy".to_owned());
    assert_eq!(resolve(&world).as_deref(), Ok("fluffy"));

    record_installed_balloon(&mut world, "gone".to_owned());
    assert_eq!(resolve(&world), Err(NotFoundReason::LastInstalledMissing));
}

/// バルーンの候補は今のゴーストの中が先・根の和が後。同じフォルダ名の根は隠れ、同じ `name` は
/// ゴーストの中が当たる。`random`・`lastinstalled` の候補にもゴーストの中が入る
/// （areka-P0-ghost-inner-balloon 要件 2.1・3.3・10.1）。
#[test]
fn balloon_candidates_put_the_ghost_inner_first() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");
    let inner = ghost.join("balloon");
    write_descript(&inner.join("inner"), "name,かくかく");
    write_descript(&inner.join("Fluffy"), "name,中のふわふわ");
    let cands = balloon_candidates(
        &areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
        &ghost,
    );
    let listed: Vec<(&str, &Path)> = cands
        .iter()
        .map(|c| (c.folder.as_str(), c.dir.as_path()))
        .collect();
    assert_eq!(
        listed,
        vec![
            ("Fluffy", inner.join("Fluffy").as_path()),
            ("inner", inner.join("inner").as_path()),
            ("kaku", root.balloon_dir("kaku").as_path()),
        ],
        "ゴーストの中が先・根の fluffy は隠れる"
    );

    let dir = |spec: &SkinSpec| {
        resolve_skin_target(&cands, spec, Some("kaku"), NO_MEMO, |_| 0).map(|c| c.dir)
    };
    assert_eq!(
        dir(&by_name("かくかく")),
        Ok(inner.join("inner")),
        "同じ name は中"
    );
    assert_eq!(
        dir(&by_name("fluffy")),
        Ok(inner.join("Fluffy")),
        "フォルダ名も中"
    );
    assert_eq!(
        dir(&SkinSpec::Folder("FLUFFY".to_owned())),
        Ok(inner.join("Fluffy")),
        "メニューのフォルダの名指しも中"
    );
    assert_eq!(
        dir(&by_name("random")),
        Ok(inner.join("Fluffy")),
        "random の候補の先頭は中"
    );
    assert_eq!(
        resolve_skin_target(
            &cands,
            &by_name("lastinstalled"),
            Some("kaku"),
            Ok("inner"),
            never
        )
        .map(|c| c.dir),
        Ok(inner.join("inner")),
        "lastinstalled も中を引く"
    );
}
