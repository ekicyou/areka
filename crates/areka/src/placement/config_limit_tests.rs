//! `ScopeConfig` の limit / 水平配置モード保持枠の檻（要件 1.2・1.4／設計 C3）。
//!
//! 本タスク（1.2）が固定するのは**保持枠の既定値と縮退経路**のみである。
//! `windowposition.limit` の語彙分類・警告・幾何・runtime 関門は後続タスク
//! （2.1／2.2／2.3）の所有であり、ここでは扱わない。

use std::collections::BTreeMap;

use super::{BalloonSide, BalloonXMode, PlacementConfig, ScopeConfig, build_placement_config};

/// テスト補助: `(key, value)` ペア列から `parse_kv` 出力相当の `BTreeMap` を組む。
fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn empty() -> BTreeMap<String, String> {
    BTreeMap::new()
}

/// `resolver::resolve_placement`（resolver.rs の未収載 scope フォールバック）と
/// **同一の解決式**。未収載 scope は表を生やさず `ScopeConfig::default()` で
/// 配置されるという既存契約をテスト側で逐語再現する。
fn scope_config_for(cfg: &PlacementConfig, scope: usize) -> ScopeConfig {
    let default_scope_cfg = ScopeConfig::default();
    cfg.scopes.get(&scope).unwrap_or(&default_scope_cfg).clone()
}

// ------------------------------------------------------------------
// T-L1: 保持枠の既定値（1.2）
// ------------------------------------------------------------------

/// T-L1: `ScopeConfig::default()` の limit は正典既定＝有効（1.2）。
/// 水平配置モードの既定は `Side`＝現行の左右分岐と同一挙動（設計 C3）。
#[test]
fn t_l1_scope_config_default_enables_limit_and_side_mode() {
    let d = ScopeConfig::default();
    assert!(
        d.balloon_limit,
        "limit の既定は正典既定 1（有効）でなければならない（要件 1.2）"
    );
    assert_eq!(
        d.balloon_x_mode,
        BalloonXMode::Side,
        "水平配置モードの既定は現行の左右分岐（Side）"
    );
    // 既存フィールドの既定は不変（additive 追加であることの固定）
    assert_eq!(d.balloon_alignment, BalloonSide::Left);
    assert_eq!(d.balloon_offset, None);
    assert_eq!(d.default_x, None);
    assert_eq!(d.default_y, None);
}

/// T-L1: enum 自身の `Default` も `Side`（`#[default]` バリアントの固定）。
#[test]
fn t_l1_balloon_x_mode_default_is_side() {
    assert_eq!(BalloonXMode::default(), BalloonXMode::Side);
}

/// T-L1: 3 バリアントは互いに区別される（`PartialEq` の意味的固定）。
#[test]
fn t_l1_balloon_x_mode_variants_are_distinct() {
    let all = [
        BalloonXMode::Side,
        BalloonXMode::CenterTop,
        BalloonXMode::CenterBottom,
    ];
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            assert_eq!(a == b, i == j, "variant {i} vs {j} の同値判定が不正");
        }
    }
}

// ------------------------------------------------------------------
// T-L2: descript KV 経路では保持枠が既定のまま（1.2・本タスクは未結線）
// ------------------------------------------------------------------

/// T-L2: descript KV（ghost/shell）には `windowposition` 系キーが存在しないため、
/// `build_placement_config` が返す全 scope の limit は既定＝有効・モードは `Side`。
/// バルーン定義側からの反映は task 2.2 の取得経路が所有する。
#[test]
fn t_l2_build_placement_config_keeps_canonical_defaults_for_all_scopes() {
    // emo2 実測相当（scope0/1）＋ char2 で 3 scope
    let ghost = kv(&[("kero.name", "エモ"), ("char2.name", "三人目")]);
    let shell = kv(&[
        ("seriko.alignmenttodesktop", "bottom"),
        ("sakura.defaultx", "0"),
        ("kero.defaultx", "0"),
        ("sakura.balloon.alignment", "left"),
        ("kero.balloon.alignment", "right"),
    ]);
    let cfg = build_placement_config(&ghost, &shell);

    assert_eq!(
        cfg.scopes.keys().copied().collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    for (scope, sc) in &cfg.scopes {
        assert!(
            sc.balloon_limit,
            "scope {scope} の limit が既定で無効になっている"
        );
        assert_eq!(sc.balloon_x_mode, BalloonXMode::Side, "scope {scope}");
    }
}

/// T-L2: KV が空（＝配置キーを一切読めなかった）でも scope0 の limit は有効。
#[test]
fn t_l2_empty_kv_scope0_keeps_limit_enabled() {
    let cfg = build_placement_config(&empty(), &empty());
    assert!(cfg.scopes[&0].balloon_limit);
    assert_eq!(cfg.scopes[&0].balloon_x_mode, BalloonXMode::Side);
}

// ------------------------------------------------------------------
// T-L3: バルーン定義が読めず既定構成へ落ちた scope（1.2・1.4）
// ------------------------------------------------------------------

/// T-L3: 配置表に**未収載**の scope（＝バルーン定義が読めずスロットが生えなかった
/// scope）は `ScopeConfig::default()` で配置されるという既存契約の下で limit が
/// 有効になる（要件 1.2）。scope 単位の解決である（要件 1.4）ことも、収載済み
/// scope と未収載 scope が独立に解けることで固定する。
#[test]
fn t_l3_unlisted_scope_falls_back_to_default_with_limit_enabled() {
    // scope0 のみ検出される KV（kero 信号なし・char{n} なし）
    let cfg = build_placement_config(&empty(), &kv(&[("sakura.defaultx", "0")]));
    assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0]);

    // 収載済み scope0: 表の値が使われる
    let s0 = scope_config_for(&cfg, 0);
    assert_eq!(s0.default_x, Some(0));
    assert!(s0.balloon_limit);

    // 未収載 scope 1／5: 既定構成へ落ちてもなお limit は有効
    for scope in [1usize, 5] {
        assert!(
            !cfg.scopes.contains_key(&scope),
            "前提: scope {scope} は未収載でなければならない"
        );
        let sc = scope_config_for(&cfg, scope);
        assert!(
            sc.balloon_limit,
            "未収載 scope {scope} の既定構成で limit が無効になっている（要件 1.2）"
        );
        assert_eq!(sc.balloon_x_mode, BalloonXMode::Side, "scope {scope}");
        assert_eq!(sc.default_x, None, "既定構成なので配置値は持たない");
    }
}

/// T-L3: limit は scope 単位で保持される枠である（要件 1.4）——同一 `PlacementConfig`
/// 内で scope ごとに異なる値を保持でき、他 scope を汚染しない。
#[test]
fn t_l3_limit_is_held_per_scope() {
    let ghost = kv(&[("kero.name", "エモ")]);
    let mut cfg = build_placement_config(&ghost, &empty());
    assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0, 1]);

    // scope1 のみ無効化しても scope0 は既定（有効）のまま
    cfg.scopes
        .get_mut(&1)
        .expect("scope1 は検出済み")
        .balloon_limit = false;
    assert!(cfg.scopes[&0].balloon_limit);
    assert!(!cfg.scopes[&1].balloon_limit);

    // 水平配置モードも同様に scope 単位
    cfg.scopes
        .get_mut(&1)
        .expect("scope1 は検出済み")
        .balloon_x_mode = BalloonXMode::CenterBottom;
    assert_eq!(cfg.scopes[&0].balloon_x_mode, BalloonXMode::Side);
    assert_eq!(cfg.scopes[&1].balloon_x_mode, BalloonXMode::CenterBottom);
}
