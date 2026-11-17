use areka_seriko::{NamedBindResult, ScopeStates, apply_named_bind};

use super::*;
use crate::scope_bind_fixture_test_support::{
    BLUE, CHAR2_CATEGORY, GREEN, KERO_CATEGORY, RED, SAKURA_CATEGORY, scope_bind_ghost,
};

// ── スコープごとの既定の着せ替えと名前の表（areka-P0-offline-surface-dump 要件 14.1・14.2・14.4）──

/// 画面の側: 共有の検体（areka-P0-offline-surface-dump の research.md §10.3 の `g_scope` の形）を画面の
/// 道（シェルの資産づくり → `ScopeStates` → `\![bind]` の決め方）に通すと、初めの組はスコープ
/// 0・1・2・3 で赤（10）・緑（11）・青（12）・空になり、スコープ 1・2 への名前の着せ替えは
/// そのスコープの名前の表だけで当たる。
#[test]
fn screen_binds_split_per_scope_on_shared_scope_fixture() {
    let ghost = scope_bind_ghost();
    let decoder = WicDecoderArm::new().expect("WIC デコーダ");
    let shell = match build_shell_assets(
        ghost.path(),
        None,
        &[0, 1, 2, 3],
        96,
        &areka_placement::DefaultSurfaces::builtin(),
        &decoder,
    ) {
        Ok(shell) => shell,
        Err(e) => panic!("シェルの資産づくりは成功する: {e:?}"),
    };
    assert!(shell.bake_failures.is_empty(), "{:?}", shell.bake_failures);

    let mut states = ScopeStates::new(shell.static_binds);
    let key = |s: &str| areka_sakura::ActorKey::from(s);
    let ids = |states: &ScopeStates, scope: &str| states.current_binds(&key(scope)).ids().to_vec();
    assert_eq!(
        ids(&states, "0"),
        vec![RED],
        "スコープ 0 は sakura.* の既定＝赤"
    );
    assert_eq!(
        ids(&states, "1"),
        vec![GREEN],
        "スコープ 1 は kero.* の既定＝緑"
    );
    assert_eq!(
        ids(&states, "2"),
        vec![BLUE],
        "スコープ 2 は char2.* の既定＝青"
    );
    assert!(
        ids(&states, "3").is_empty(),
        "char3.* の無いスコープ 3 は何も着ない"
    );

    let r = &shell.bind_resolver;
    // スコープ 1 は kero.* の名前で当たり、同じカテゴリの緑は外れる（排他）。
    let hit = apply_named_bind(&mut states, r, &key("1"), KERO_CATEGORY, "青", true);
    assert!(
        matches!(hit, NamedBindResult::Applied { id: BLUE, .. }),
        "{hit:?}"
    );
    assert_eq!(ids(&states, "1"), vec![BLUE]);
    // スコープ 2 は char2.* の名前で当たる。
    let hit = apply_named_bind(&mut states, r, &key("2"), CHAR2_CATEGORY, "赤", true);
    assert!(
        matches!(hit, NamedBindResult::Applied { id: RED, .. }),
        "{hit:?}"
    );
    assert_eq!(ids(&states, "2"), vec![RED]);
    // ほかのスコープの名前では当たらない。
    for (scope, other) in [("1", SAKURA_CATEGORY), ("2", KERO_CATEGORY)] {
        let miss = apply_named_bind(&mut states, r, &key(scope), other, "緑", true);
        assert!(
            matches!(miss, NamedBindResult::NotFound),
            "{scope}/{other}: {miss:?}"
        );
    }
    assert_eq!(ids(&states, "0"), vec![RED], "スコープ 0 は触られない");
}

// ── descript の既定の面（descript-window-keys 要件 1.7・3.1）──

/// シェルの資産の各スコープの最初の面は、渡した descript の
/// 既定の面になる（本体側・相方側・スコープ 2 は書いた値、書いていないスコープ 3 は組み込みの 10）。
/// 面がシェルに在るかは問わない（在るかの判断は測り・装着の相が持つ）。
#[test]
fn shell_assets_take_initial_surfaces_from_descript_defaults() {
    use areka_placement::descript_window_keys::read_default_surfaces;
    let ghost = scope_bind_ghost();
    let decoder = WicDecoderArm::new().expect("WIC デコーダ");
    let ghost_kv = [
        ("sakura.seriko.defaultsurface", "3"),
        ("kero.seriko.defaultsurface", "12"),
        ("char2.seriko.defaultsurface", "7"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let defaults = read_default_surfaces(&ghost_kv);
    let shell = match build_shell_assets(ghost.path(), None, &[0, 1, 2, 3], 96, &defaults, &decoder)
    {
        Ok(shell) => shell,
        Err(e) => panic!("シェルの資産づくりは成功する: {e:?}"),
    };
    let initial: Vec<(u32, u32)> = shell
        .shells
        .iter()
        .map(|s| (s.scope, s.initial_surface_id))
        .collect();
    assert_eq!(initial, vec![(0, 3), (1, 12), (2, 7), (3, 10)]);
}
