//! `ScopeConfig` の limit / 水平配置モード保持枠の檻。
//!
//! ここで固定するのは**保持枠の既定値と縮退経路**のみである。
//! `windowposition.limit` の語彙分類・警告・幾何・runtime 関門は、取得経路（`lib.rs` の
//! `merge_scope_windowpositions`）とその先が持ち、ここでは扱わない。

use super::{BalloonSide, BalloonXMode, ScopeConfig, build_placement_config};
use crate::test_support::kv;

// ------------------------------------------------------------------
// 保持枠の既定値
// ------------------------------------------------------------------

/// `ScopeConfig::default()` の limit は正典既定＝有効。
/// 水平配置モードの既定は `Side`＝現行の左右分岐と同一挙動。
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

// ------------------------------------------------------------------
// descript KV 経路では保持枠が既定のまま
// ------------------------------------------------------------------

/// descript KV（ghost/shell）には `windowposition` 系キーが存在しないため、
/// `build_placement_config` が返す全 scope の limit は既定＝有効・モードは `Side`。
/// バルーン定義側からの反映は `build_placement_config` の外、`lib.rs` の `scale_then_merge` が呼ぶ
/// 取得経路（`merge_scope_windowpositions`）が行う。
#[test]
fn t_l2_build_placement_config_keeps_canonical_defaults_for_all_scopes() {
    // emo2 実測相当（scope0/1）＋ char2（scope は足さない）
    let ghost = kv(&[("kero.name", "エモ"), ("char2.name", "三人目")]);
    let shell = kv(&[
        ("seriko.alignmenttodesktop", "bottom"),
        ("sakura.defaultx", "0"),
        ("kero.defaultx", "0"),
        ("sakura.balloon.alignment", "left"),
        ("kero.balloon.alignment", "right"),
    ]);
    let cfg = build_placement_config(&ghost, &shell);

    assert_eq!(cfg.scopes.keys().copied().collect::<Vec<_>>(), vec![0, 1]);
    for (scope, sc) in &cfg.scopes {
        assert!(
            sc.balloon_limit,
            "scope {scope} の limit が既定で無効になっている"
        );
        assert_eq!(sc.balloon_x_mode, BalloonXMode::Side, "scope {scope}");
    }
}
