//! `resolve` の決定論テスト（mcp-tool-entrances 要件 3.9・mcp-ghost-name-match 要件 5.1 の全場合）。
//!
//! ⑴ 解決の判断（名前・本体側名・英字の大小・前後の空白・フルパス・省略・空と空白だけ・0 体・`Reject`）
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
        sakura_name: Some("エミリ".to_string()),
        root: PathBuf::from(ROOT),
    }
}

fn unnamed() -> ActiveGhost {
    ActiveGhost {
        name: None,
        sakura_name: None,
        root: PathBuf::from(ROOT),
    }
}

/// 名前が無く本体側名だけを持つ 1 体。
fn sakura_only() -> ActiveGhost {
    ActiveGhost {
        name: None,
        sakura_name: Some("エミリ".to_string()),
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
fn name_differing_only_in_ascii_case_resolves() {
    for given in ["emily/phase4.5", "EMILY/PHASE4.5"] {
        assert_eq!(solve(Some(given)), Ok(named()), "{given}");
    }
}

#[test]
fn sakura_name_resolves() {
    assert_eq!(solve(Some("エミリ")), Ok(named()));
    // `name` が無くても本体側名で解決する（要件 1.2）。
    let g = sakura_only();
    assert_eq!(resolve(Some(&g), Some("エミリ"), Omitted::Reject), Ok(&g));
    // 本体側名も名前と同じ規則で、半角の英字の大小を畳む（要件 1.2）。
    let g = ActiveGhost {
        sakura_name: Some("Emi".to_string()),
        ..sakura_only()
    };
    assert_eq!(resolve(Some(&g), Some("eMI"), Omitted::Reject), Ok(&g));
}

#[test]
fn non_name_strings_do_not_resolve() {
    // 相方の名前・sakura.name2 に当たる文字列も、ActiveGhost が持たないのでただの外れ（要件 1.4）。
    for g in [named(), sakura_only()] {
        for given in ["エモ", "エミリー", "Emily", "Nobody"] {
            assert_eq!(
                resolve(Some(&g), Some(given), Omitted::UseActive),
                Err(CANNOT_FIND),
                "{given}"
            );
        }
    }
}

#[test]
fn kana_width_and_fullwidth_case_differences_do_not_resolve() {
    let g = ActiveGhost {
        name: Some("えもＡＢＣ".to_string()),
        sakura_name: None,
        root: PathBuf::from(ROOT),
    };
    assert_eq!(
        resolve(Some(&g), Some("えもＡＢＣ"), Omitted::UseActive),
        Ok(&g)
    );
    // かなの違い・全角の英字の大小・全角と半角（要件 1.3）。
    for given in ["エモＡＢＣ", "えもａｂｃ", "えもABC"] {
        assert_eq!(
            resolve(Some(&g), Some(given), Omitted::UseActive),
            Err(CANNOT_FIND),
            "{given}"
        );
    }
}

#[test]
fn name_with_surrounding_whitespace_resolves() {
    for name in ["Emily/Phase4.5", "エミリ"] {
        for given in [
            format!(" {name} "),
            format!("\t{name}\t"),
            format!("\u{3000}{name}\u{3000}"),
            format!(" \t\u{3000}{name}"),
            format!("{name}\u{3000}\t "),
        ] {
            assert_eq!(solve(Some(&given)), Ok(named()), "{given:?}");
        }
    }
}

#[test]
fn inner_whitespace_difference_does_not_resolve() {
    let g = ActiveGhost {
        name: Some("Emily Phase".to_string()),
        sakura_name: None,
        root: PathBuf::from(ROOT),
    };
    assert_eq!(
        resolve(Some(&g), Some("Emily Phase"), Omitted::UseActive),
        Ok(&g)
    );
    // 途中の空白は除かない（要件 2.2）。
    for given in ["Emily  Phase", "Emily\u{3000}Phase", "EmilyPhase"] {
        assert_eq!(
            resolve(Some(&g), Some(given), Omitted::UseActive),
            Err(CANNOT_FIND),
            "{given:?}"
        );
    }
}

#[test]
fn blank_or_empty_cannot_find() {
    // 空文字・空白だけは省略ではなく、何にも当たらない名前（要件 2.4・0 体も）。
    let (n, u, s) = (named(), unnamed(), sakura_only());
    for active in [Some(&n), Some(&u), Some(&s), None] {
        for omitted in [Omitted::UseActive, Omitted::Reject] {
            for given in ["", "   ", "\t", "\u{3000}"] {
                assert_eq!(
                    resolve(active, Some(given), omitted),
                    Err(CANNOT_FIND),
                    "{given:?} {omitted:?} {}",
                    active.is_some()
                );
            }
        }
    }
}

#[test]
fn full_path_with_surrounding_whitespace_does_not_resolve() {
    // フルパスとの照合では前後の空白を除かない（要件 2.3）。
    for g in [named(), unnamed()] {
        for given in [
            r" C:\ssp\ghost\emily4",
            r"C:\ssp\ghost\emily4 ",
            " C:\\ssp\\ghost\\emily4\\ ",
            "\tC:/ssp/ghost/emily4/\t",
            "\u{3000}C:/ssp/ghost/emily4",
        ] {
            assert_eq!(
                resolve(Some(&g), Some(given), Omitted::UseActive),
                Err(CANNOT_FIND),
                "{given:?}"
            );
        }
    }
}

#[test]
fn every_spelling_resolves_to_the_same_ghost() {
    // 綴りは返り値に残らない。どの綴りでも起動中のゴーストの値そのもの（要件 1.5・4.4）。
    let g = named();
    for given in [
        "Emily/Phase4.5",
        "emily/PHASE4.5",
        " Emily/Phase4.5\u{3000}",
        "エミリ",
        "\tエミリ ",
        ROOT,
        "c:/ssp/ghost/emily4/",
    ] {
        let got = resolve(Some(&g), Some(given), Omitted::Reject);
        assert_eq!(got, Ok(&g), "{given:?}");
        let got = got.unwrap();
        assert!(std::ptr::eq(got, &g), "{given:?}");
        assert_eq!(listed_value(got), "Emily/Phase4.5", "{given:?}");
    }
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
fn omitted_with_use_active_resolves_to_the_active_one() {
    assert_eq!(solve(None), Ok(named()));
}

#[test]
fn omitted_with_reject_is_not_active() {
    let g = named();
    assert_eq!(resolve(Some(&g), None, Omitted::Reject), Err(NOT_ACTIVE));
}

#[test]
fn no_active_ghost_omitted_is_not_active() {
    for omitted in [Omitted::UseActive, Omitted::Reject] {
        assert_eq!(resolve(None, None, omitted), Err(NOT_ACTIVE));
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
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4\"),
    };
    assert_eq!(listed_value(&trailing), ROOT);
}

#[test]
fn listed_value_resolves_back_to_the_same_ghost() {
    for g in [named(), unnamed(), sakura_only()] {
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
