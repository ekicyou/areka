//! `resolve` の決定論テスト（要件 3.9 の全場合）。
//!
//! ⑴ 解決の判断（名前・フルパス・省略・空・0 体・`Reject`）
//! ⑵ 一覧の値（名前あり・`name` 無し）が同じゴーストへ戻ること（4.2・4.4）
//! ⑶ World からの読み取り（GhostSlot 無し・置き場が空・実行系の無い単位は無し・3.8）

use std::path::PathBuf;

use bevy_ecs::world::World;

use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};

const ROOT: &str = r"C:\ssp\ghost\emily4";

fn named() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(ROOT),
    }
}

fn unnamed() -> ActiveGhost {
    ActiveGhost {
        name: None,
        root: PathBuf::from(ROOT),
    }
}

/// 名前ありの 1 体へ `ghost_name` を任意の扱いで解いた結果。
fn solve(ghost_name: Option<&str>) -> Result<ActiveGhost, &'static str> {
    let g = named();
    resolve(Some(&g), ghost_name, Omitted::UseActive).cloned()
}

// ---- ⑴ 解決の判断 ---------------------------------------------------------

#[test]
fn name_exact_match_resolves() {
    assert_eq!(solve(Some("Emily/Phase4.5")), Ok(named()));
}

#[test]
fn name_differing_only_in_case_does_not_resolve() {
    assert_eq!(solve(Some("emily/phase4.5")), Err(CANNOT_FIND));
}

#[test]
fn full_path_resolves() {
    assert_eq!(solve(Some(ROOT)), Ok(named()));
}

#[test]
fn full_path_with_case_separator_and_trailing_differences_resolves() {
    for given in [
        r"c:\SSP\Ghost\EMILY4",
        "C:/ssp/ghost/emily4",
        r"C:\ssp\ghost\emily4\",
        "c:/SSP/ghost/Emily4/",
        r"C:\ssp/ghost\emily4\\",
    ] {
        assert_eq!(solve(Some(given)), Ok(named()), "{given}");
    }
}

#[test]
fn folder_name_only_does_not_resolve() {
    assert_eq!(solve(Some("emily4")), Err(CANNOT_FIND));
}

#[test]
fn relative_path_does_not_resolve() {
    for given in [r"ghost\emily4", "ghost/emily4", r".\ghost\emily4"] {
        assert_eq!(solve(Some(given)), Err(CANNOT_FIND), "{given}");
    }
}

#[test]
fn sakura_name_does_not_resolve() {
    // descript の sakura.name に当たる別名は見ない。
    assert_eq!(solve(Some("エミリ")), Err(CANNOT_FIND));
}

#[test]
fn empty_or_omitted_with_use_active_resolves_to_the_active_one() {
    assert_eq!(solve(Some("")), Ok(named()));
    assert_eq!(solve(None), Ok(named()));
}

#[test]
fn empty_or_omitted_with_reject_is_not_active() {
    let g = named();
    assert_eq!(resolve(Some(&g), None, Omitted::Reject), Err(NOT_ACTIVE));
    assert_eq!(
        resolve(Some(&g), Some(""), Omitted::Reject),
        Err(NOT_ACTIVE)
    );
}

#[test]
fn no_active_ghost_omitted_is_not_active() {
    for omitted in [Omitted::UseActive, Omitted::Reject] {
        assert_eq!(resolve(None, None, omitted), Err(NOT_ACTIVE));
        assert_eq!(resolve(None, Some(""), omitted), Err(NOT_ACTIVE));
    }
}

#[test]
fn no_active_ghost_with_name_cannot_find() {
    for omitted in [Omitted::UseActive, Omitted::Reject] {
        assert_eq!(
            resolve(None, Some("Emily/Phase4.5"), omitted),
            Err(CANNOT_FIND)
        );
        assert_eq!(resolve(None, Some(ROOT), omitted), Err(CANNOT_FIND));
    }
}

#[test]
fn reject_with_name_still_resolves() {
    let g = named();
    assert_eq!(
        resolve(Some(&g), Some("Emily/Phase4.5"), Omitted::Reject),
        Ok(&g)
    );
}

// ---- ⑵ 一覧の値 -----------------------------------------------------------

#[test]
fn listed_value_is_name_when_present() {
    assert_eq!(listed_value(&named()), "Emily/Phase4.5");
}

#[test]
fn listed_value_is_full_path_without_trailing_separator_when_name_absent() {
    assert_eq!(listed_value(&unnamed()), ROOT);
    let trailing = ActiveGhost {
        name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4\"),
    };
    assert_eq!(listed_value(&trailing), ROOT);
}

#[test]
fn listed_value_resolves_back_to_the_same_ghost() {
    for g in [named(), unnamed()] {
        for omitted in [Omitted::UseActive, Omitted::Reject] {
            let value = listed_value(&g);
            assert_eq!(resolve(Some(&g), Some(&value), omitted), Ok(&g), "{value}");
        }
    }
}

// ---- ⑶ World からの読み取り ------------------------------------------------

#[test]
fn active_is_none_without_ghost_slot() {
    assert_eq!(active(&World::new()), None);
}

#[test]
fn active_is_none_when_slot_is_empty() {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(None));
    assert_eq!(active(&world), None);
}

#[test]
fn active_is_none_when_session_has_no_runtime() {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        None,
        PathBuf::from(ROOT),
    ))));
    assert_eq!(active(&world), None);
}
