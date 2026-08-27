//! `resolve_placement` の檻: anchor 伝搬と出力の事後条件。
//!
//! `resolver_resolve_tests.rs` からの分割（1,000 行規約）。

use super::resolve_test_support::{cfg_of, input, scope_cfg};
use super::test_support::{DPIS, px, work_area};
use super::*;
use crate::placement::config::{Alignment, build_placement_config};
use crate::placement::shared_test_support::MEASURE_DPI;

// ------------------------------------------------------------------
// anchor 伝搬（4.2・DD15 基盤・task 1.2）
//
// 旧 `bottom_snap`（二値）伝搬テストを 5 値アンカー検証へ意味を保って差し替え
// （単一真実源＝`ScopePlacement.anchor`・Req1.6）。二値の吸着フラグは
// `!anchor.is_free()` で導出可能ゆえ格納しない。
// ------------------------------------------------------------------

/// 4.2: cascade 解決済み `alignment` が 5 値アンカーとして `ScopePlacement.anchor`
/// へ伝搬する（`Bottom`→`Bottom`・`Seam("top"/"left"/"right")`→対応辺・
/// `Seam(未知)`→`Bottom`（フォールバック）・`Free`→`Free`）。吸着ドラッグ／
/// リサイズの射影 T（後続 task）が消費する情報伝搬の檻。
#[test]
fn anchor_propagates_five_values_from_resolved_alignment() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let cases = [
            (Alignment::Bottom, Anchor::Bottom),
            (Alignment::Seam("top".to_owned()), Anchor::Top),
            (Alignment::Seam("left".to_owned()), Anchor::Left),
            (Alignment::Seam("right".to_owned()), Anchor::Right),
            (Alignment::Seam("unknown-value".to_owned()), Anchor::Bottom),
            (Alignment::Free, Anchor::Free),
        ];
        for (alignment, expected) in cases {
            let cfg = cfg_of(vec![(0, scope_cfg(alignment.clone(), Some(0), None))]);
            let out = resolve_placement(&cfg, wa, &[input(0, w, h)], MEASURE_DPI);
            assert_eq!(
                out[0].anchor, expected,
                "dpi={dpi} alignment={alignment:?}: anchor の伝搬"
            );
        }
    }
}

/// 4.2 補: 混在スコープでスコープごとに独立に伝搬し、`cfg.scopes` 未収載
/// スコープは既定 `ScopeConfig`（＝Bottom）ゆえ `Anchor::Bottom`。
#[test]
fn anchor_mixed_scopes_and_missing_config_defaults_to_bottom() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let cfg = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, Some(0), None)),
            (
                1,
                scope_cfg(Alignment::Free, Some(px(100, dpi)), Some(px(80, dpi))),
            ),
        ]);
        let out = resolve_placement(
            &cfg,
            wa,
            &[
                input(0, px(400, dpi), px(600, dpi)),
                input(1, px(320, dpi), px(480, dpi)),
                input(2, px(200, dpi), px(400, dpi)), // 未収載 → 既定 Bottom
            ],
            MEASURE_DPI,
        );
        assert_eq!(
            out[0].anchor,
            Anchor::Bottom,
            "dpi={dpi}: Bottom → Anchor::Bottom"
        );
        assert_eq!(
            out[1].anchor,
            Anchor::Free,
            "dpi={dpi}: Free → Anchor::Free"
        );
        assert_eq!(
            out[2].anchor,
            Anchor::Bottom,
            "dpi={dpi}: 未収載＝既定 Bottom → Anchor::Bottom"
        );
    }
}

/// 4.2 完了条件: **4 層優先度カスケード**（`build_placement_config`）で解決された
/// 各アンカー設定（bottom・free・未指定＝既定 bottom・混在スコープ）が、
/// `resolve_placement` を通って対応する 5 値アンカーとして `ScopePlacement.anchor`
/// へ正しく伝搬する（KV → config → resolver → anchor の実経路）。
///
/// - scope0: shell scope 接頭層 `sakura.seriko.alignmenttodesktop=bottom` が
///   ghost scope 接頭層 `sakura.seriko.alignmenttodesktop=free` を**上書き**
///   （shell ＞ ghost の層優先で解決）→ `Bottom`
/// - scope1: shell scope 接頭層 `kero.seriko.alignmenttodesktop=free` → `Free`
///   （スコープごとに独立伝搬）
/// - scope2: `char2.` 接頭キーでスコープ検出のみ・alignment はどの層にも不在
///   → 既定 `Bottom`（未指定＝既定）
///
/// cascade 解決自体は config.rs の領分（改変しない・Req6.3）。ここでは解決済み
/// alignment が anchor として正しく運ばれることのみを固定する。
#[test]
fn anchor_propagates_through_build_placement_config_cascade() {
    use std::collections::BTreeMap;

    fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    let ghost_kv = kv(&[
        ("kero.name", "エモ"),                        // scope1 検出
        ("sakura.seriko.alignmenttodesktop", "free"), // scope0 の弱層（shell に負ける）
    ]);
    let shell_kv = kv(&[
        ("sakura.seriko.alignmenttodesktop", "bottom"), // scope0 強層（ghost free を上書き）
        ("kero.seriko.alignmenttodesktop", "free"),     // scope1
        ("char2.defaultx", "0"),                        // scope2 検出のみ（alignment 未指定）
    ]);
    let cfg = build_placement_config(&ghost_kv, &shell_kv);

    for dpi in DPIS {
        let wa = work_area(dpi);
        let out = resolve_placement(
            &cfg,
            wa,
            &[
                input(0, px(400, dpi), px(600, dpi)),
                input(1, px(320, dpi), px(480, dpi)),
                input(2, px(200, dpi), px(400, dpi)),
            ],
            MEASURE_DPI,
        );
        assert_eq!(out.len(), 3, "dpi={dpi}: 3 スコープ解決（空虚一致封じ）");
        assert_eq!(
            out[0].anchor,
            Anchor::Bottom,
            "dpi={dpi}: scope0 は shell 強層 bottom が ghost 弱層 free を上書き → Anchor::Bottom"
        );
        assert_eq!(
            out[1].anchor,
            Anchor::Free,
            "dpi={dpi}: scope1 接頭層 free → Anchor::Free（スコープ独立伝搬）"
        );
        assert_eq!(
            out[2].anchor,
            Anchor::Bottom,
            "dpi={dpi}: scope2 alignment どの層にも不在＝既定 → Anchor::Bottom"
        );
    }
}

// ------------------------------------------------------------------
// 事後条件（design Postconditions・resolve_placement の出力時点で成立する条件）
// ------------------------------------------------------------------

/// 事後条件: 出力長＝入力長・入力順保存・寸法転記・
/// `balloon_offset ≡ balloon_pos − char_pos`。
///
/// 恒等式は [`resolve_placement`] の**出力時点**の事後条件であり、以降ずっと成り立つ
/// 不変量ではない（windowposition-limit DD6）——下流の関門は `balloon_pos`（表示位置）
/// だけを補正し `balloon_offset`（論理相対位置）を生値のまま残すため、関門通過後は
/// 両者の差が恒等式を満たすとは限らない。本檻は配置式の出力だけを見る。
#[test]
fn postconditions_order_length_and_offset_identity() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let inputs = [
            input(0, px(400, dpi), px(600, dpi)),
            input(1, px(320, dpi), px(480, dpi)),
        ];
        let cfg = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, Some(0), None)),
            (1, scope_cfg(Alignment::Bottom, Some(0), None)),
        ]);

        let out = resolve_placement(&cfg, wa, &inputs, MEASURE_DPI);

        assert_eq!(out.len(), inputs.len(), "dpi={dpi}: 出力長＝入力長");
        for (o, i) in out.iter().zip(&inputs) {
            assert_eq!(o.scope, i.scope, "dpi={dpi}: 入力順保存");
            assert_eq!(o.char_size, i.char_size, "dpi={dpi}");
            assert_eq!(o.balloon_size, i.balloon_size, "dpi={dpi}");
            assert_eq!(
                o.balloon_offset,
                PointPx {
                    x: o.balloon_pos.x - o.char_pos.x,
                    y: o.balloon_pos.y - o.char_pos.y
                },
                "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
            );
        }
    }
}
