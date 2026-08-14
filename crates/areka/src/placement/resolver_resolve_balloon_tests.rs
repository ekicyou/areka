//! `resolve_placement` の檻: バルーン暫定 offset（T-R8・P5・4.4・DD7）。
//!
//! `resolver_resolve_tests.rs` からの分割（1,000 行規約）。

use super::resolve_test_support::{cfg_of, input, offset_work_area};
use super::test_support::{DPIS, px, work_area};
use super::*;
use crate::placement::config::{Alignment, BalloonSide, ScopeConfig};

// ------------------------------------------------------------------
// T-R8: バルーン暫定 offset（P5・4.4・DD7）
// ------------------------------------------------------------------

/// バルーン構成つき ScopeConfig（T-R8 用）。
fn scope_cfg_balloon(
    default_x: Option<i32>,
    balloon_alignment: BalloonSide,
    balloon_offset: Option<(i32, i32)>,
) -> ScopeConfig {
    ScopeConfig {
        alignment: Alignment::Bottom,
        default_x,
        default_y: None,
        balloon_alignment,
        balloon_offset,
        ..ScopeConfig::default()
    }
}

/// T-R8: `balloon.alignment=left`（既定） → `balloon_x = char_x − balloon_w`・
/// `balloon_y = char_y`（上端揃え・DD7）。恒等式も確認。
#[test]
fn t_r8_balloon_left_places_left_of_char() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let inp = input(0, w, h);
        let bw = inp.balloon_size.w;
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(px(40, dpi)), BalloonSide::Left, None),
        )]);

        let out = resolve_placement(&cfg, wa, &[inp]);

        let cp = out[0].char_pos;
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: cp.x - bw,
                y: cp.y
            },
            "dpi={dpi}: left はキャラ左隣・上端揃え"
        );
        assert_eq!(
            out[0].balloon_offset,
            PointPx { x: -bw, y: 0 },
            "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
        );
    }
}

/// T-R8: `balloon.alignment=right` → `balloon_x = char_x + w`（キャラ右隣・DD7）。
/// scope0 右端密着では `balloon_x = work_area.right`＝work area 外だが、
/// **resolver はバルーンをクランプしない**（P5）＝そのままの幾何値を返す。
/// `windowposition.limit` による作業領域内への補正は下流の関門の所有であり
/// （windowposition-limit DD6）、本檻は関門より上流＝配置式の出力だけを見る。
#[test]
fn t_r8_balloon_right_places_right_of_char_without_clamp() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let cfg = cfg_of(vec![(0, scope_cfg_balloon(Some(0), BalloonSide::Right, None))]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)]);

        let cp = out[0].char_pos;
        assert_eq!(cp.x, wa.right - w, "dpi={dpi}: 前提＝右端密着");
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: wa.right,
                y: cp.y
            },
            "dpi={dpi}: right はキャラ右隣・resolver は work area 外のまま返す"
        );
        assert_eq!(
            out[0].balloon_offset,
            PointPx { x: w, y: 0 },
            "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
        );
    }
}

/// T-R8: `balloon.offsetx/offsety` は alignment 由来の幾何値へ加算（DD7）。
#[test]
fn t_r8_balloon_offsetx_offsety_added() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let inp = input(0, w, h);
        let bw = inp.balloon_size.w;
        let (ox, oy) = (px(24, dpi), -px(32, dpi));
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(px(40, dpi)), BalloonSide::Left, Some((ox, oy))),
        )]);

        let out = resolve_placement(&cfg, wa, &[inp]);

        let cp = out[0].char_pos;
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: cp.x - bw + ox,
                y: cp.y + oy
            },
            "dpi={dpi}: offsetx/y 加算"
        );
        assert_eq!(
            out[0].balloon_offset,
            PointPx {
                x: -bw + ox,
                y: oy
            },
            "dpi={dpi}: balloon_offset ≡ balloon_pos − char_pos"
        );
    }
}

/// T-R8 補: **配置式（P5）はバルーンをクランプしない**＝キャラが左端クランプされた
/// 状態の left バルーンは work area 左外（負方向）へ素直にはみ出したまま返る。
///
/// # 前提（windowposition-limit 7.3・DD6）
///
/// 「バルーンは決してクランプされない」という意味ではない——`windowposition.limit`
/// が有効なら、この出力は下流の関門が作業領域内へ補正する（起動時＝
/// `balloon_limit::apply_balloon_limit`（`main.rs` の `restore_merged_placements`）／実行時＝
/// `follow::window_move::enqueue_window_set_pos` の runtime 関門／バルーン単独ドラッグ
/// の解放時＝`follow_drag_end_limit_tests.rs` が所有）。補正の所有は関門であって
/// 配置式ではない、という分業を固定するのが本檻である。無クランプは
/// `resolve_placement` の契約としていまも真であり、この assert は反転しない。
#[test]
fn t_r8_resolver_does_not_clamp_balloon_outside_work_area() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let inp = input(0, w, h);
        let bw = inp.balloon_size.w;
        // キャラを左端クランプへ追い込む（T-R6 と同型）
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(px(40000, dpi)), BalloonSide::Left, None),
        )]);

        let out = resolve_placement(&cfg, wa, &[inp]);

        assert_eq!(out[0].char_pos.x, wa.left, "dpi={dpi}: 前提＝左端クランプ");
        assert_eq!(
            out[0].balloon_pos.x,
            wa.left - bw,
            "dpi={dpi}: resolver は work area 左外のバルーンもクランプしない（補正は下流の関門）"
        );
    }
}

