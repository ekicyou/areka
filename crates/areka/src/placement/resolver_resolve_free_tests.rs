//! `resolve_placement` の檻: free 配置（T-R4・P3・2.6・DD10）。
//!
//! `resolver_resolve_tests.rs` からの分割（1,000 行規約）。

use super::resolve_test_support::{cfg_of, input, offset_work_area, scope_cfg};
use super::test_support::{DPIS, px};
use super::*;
use crate::placement::config::Alignment;
use crate::placement::shared_test_support::MEASURE_DPI;

// ------------------------------------------------------------------
// T-R4: free 配置（P3・2.6・DD10）
// ------------------------------------------------------------------

/// T-R4: free で `defaultleft`/`defaulttop` 両指定 → work area **左上**原点で
/// `char_x = left + dx`・`char_y = top + dy`（DD10）。原点非 (0,0) の work area
/// で left/top 依存を固定する（bottom の right 基準と混同しない檻）。
#[test]
fn t_r4_free_applies_default_left_top_from_work_area_origin() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let (dx, dy) = (px(120, dpi), px(80, dpi));
        let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, Some(dx), Some(dy)))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)], MEASURE_DPI);

        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.left + dx,
                y: wa.top + dy
            },
            "dpi={dpi}: free は work area 左上原点＋オフセット"
        );
    }
}

/// T-R4: free で Y 未指定 → Y のみ bottom 相当（`bottom − h`）へフォールバック
/// （2.6）。X は左上原点適用のまま。
#[test]
fn t_r4_free_unspecified_y_falls_back_to_bottom() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let dx = px(120, dpi);
        let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, Some(dx), None))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)], MEASURE_DPI);

        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.left + dx,
                y: wa.bottom - h
            },
            "dpi={dpi}: Y 未指定は bottom 相当"
        );
    }
}

/// T-R4: free で X 未指定 → X のみ bottom 相当（P2 連鎖値＝scope0 は右端密着）へ
/// フォールバック（2.6）。Y は左上原点適用のまま。
#[test]
fn t_r4_free_unspecified_x_falls_back_to_bottom_chain() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let dy = px(80, dpi);
        let cfg = cfg_of(vec![(0, scope_cfg(Alignment::Free, None, Some(dy)))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)], MEASURE_DPI);

        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.right - w,
                y: wa.top + dy
            },
            "dpi={dpi}: X 未指定は P2 の bottom 相当値（scope0＝右端密着）"
        );
    }
}

/// T-R4 補: free で両成分未指定 → **幾何**（位置・寸法・バルーン）は Bottom と
/// 完全同一（2.6 の極限）。ただし `anchor` だけは alignment 由来で相違する
/// （free は幾何が bottom へフォールバックしても全方向移動可＝非吸着＝`Anchor::Free`・
/// 4.2。task 8.1 で構造体全体一致から幾何一致＋anchor 相違へ更新）。
#[test]
fn t_r4_free_both_unspecified_equals_bottom() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let inputs = [
            input(0, px(400, dpi), px(600, dpi)),
            input(1, px(320, dpi), px(480, dpi)),
        ];
        let free = cfg_of(vec![
            (0, scope_cfg(Alignment::Free, None, None)),
            (1, scope_cfg(Alignment::Free, None, None)),
        ]);
        let bottom = cfg_of(vec![
            (0, scope_cfg(Alignment::Bottom, None, None)),
            (1, scope_cfg(Alignment::Bottom, None, None)),
        ]);

        let out_free = resolve_placement(&free, wa, &inputs, MEASURE_DPI);
        let out_bottom = resolve_placement(&bottom, wa, &inputs, MEASURE_DPI);
        assert_eq!(out_free.len(), 2, "dpi={dpi}: 空虚一致封じ");
        for (f, b) in out_free.iter().zip(&out_bottom) {
            assert_eq!(f.scope, b.scope, "dpi={dpi}");
            assert_eq!(
                f.char_pos, b.char_pos,
                "dpi={dpi}: 全未指定 free ≡ bottom（幾何）"
            );
            assert_eq!(f.char_size, b.char_size, "dpi={dpi}");
            assert_eq!(f.balloon_pos, b.balloon_pos, "dpi={dpi}");
            assert_eq!(f.balloon_size, b.balloon_size, "dpi={dpi}");
            assert_eq!(f.balloon_offset, b.balloon_offset, "dpi={dpi}");
            // アンカー情報だけは alignment 由来で相違（free＝非吸着・4.2）
            assert_eq!(
                f.anchor,
                Anchor::Free,
                "dpi={dpi}: free は幾何フォールバックでも非吸着（Anchor::Free）"
            );
            assert_eq!(
                b.anchor,
                Anchor::Bottom,
                "dpi={dpi}: bottom は吸着（Anchor::Bottom）"
            );
        }
    }
}

/// T-R4 補: free もキャラ窓クランプ（P4・DD12）の対象（過大 offset は
/// work area 右下で停止＝画面内出現の安全弁は alignment を選ばない）。
#[test]
fn t_r4_free_is_clamped_into_work_area() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let huge = px(40000, dpi);
        let cfg = cfg_of(vec![(
            0,
            scope_cfg(Alignment::Free, Some(huge), Some(huge)),
        )]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)], MEASURE_DPI);

        assert_eq!(
            out[0].char_pos,
            PointPx {
                x: wa.right - w,
                y: wa.bottom - h
            },
            "dpi={dpi}: free も P4 クランプで画面内"
        );
    }
}

/// T-R4 補: free で配置した scope0 の実位置が後続の P2 連鎖基準になる
/// （連鎖は alignment を選ばず「前スコープの実配置の左隣」）。
#[test]
fn t_r4_free_position_feeds_scope_chain() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w0, h0) = (px(400, dpi), px(600, dpi));
        let (w1, h1) = (px(320, dpi), px(480, dpi));
        let (dx, dy) = (px(800, dpi), px(80, dpi));
        let cfg = cfg_of(vec![
            (0, scope_cfg(Alignment::Free, Some(dx), Some(dy))),
            (1, scope_cfg(Alignment::Bottom, Some(0), None)),
        ]);

        let out = resolve_placement(&cfg, wa, &[input(0, w0, h0), input(1, w1, h1)], MEASURE_DPI);

        let x0 = wa.left + dx;
        assert_eq!(out[0].char_pos.x, x0, "dpi={dpi}");
        assert_eq!(
            out[1].char_pos,
            PointPx {
                x: x0 - w1,
                y: wa.bottom - h1
            },
            "dpi={dpi}: base_x(1)=char_x(0)−w1（free 実位置基準・scg 2.1/2.2）"
        );
    }
}
