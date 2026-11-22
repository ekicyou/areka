//! `resolve_placement` の檻: バルーン暫定 offset（P5）。
//!
//! `resolver_resolve_tests.rs` からの分割（1,000 行規約）。

use super::resolve_test_support::{cfg_of, input, offset_work_area};
use super::test_support::{DPIS, px, work_area};
use super::*;
use crate::config::{Alignment, BalloonSide, ScopeConfig};
use crate::shared_test_support::MEASURE_DPI;

// ------------------------------------------------------------------
// バルーン暫定 offset（P5）
// ------------------------------------------------------------------

/// バルーン構成つき ScopeConfig。
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

/// `balloon.alignment=left`（既定） → `balloon_x = char_x − balloon_w`・
/// `balloon_y = char_y`（上端揃え）。
#[test]
fn balloon_left_places_left_of_char() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let inp = input(0, w, h);
        let bw = inp.balloon_size.w;
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(px(40, dpi)), BalloonSide::Left, None),
        )]);

        let out = resolve_placement(&cfg, wa, &[inp], MEASURE_DPI);

        let cp = out[0].char_pos;
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: cp.x - bw,
                y: cp.y
            },
            "dpi={dpi}: left はキャラ左隣・上端揃え"
        );
    }
}

/// `balloon.alignment=right` → `balloon_x = char_x + w`（キャラ右隣）。
/// scope0 右端密着では `balloon_x = work_area.right`＝work area 外だが、
/// **resolver はバルーンをクランプしない**（P5）＝そのままの幾何値を返す。
/// `windowposition.limit` による作業領域内への補正は下流の関門の所有であり、本檻は関門より上流＝配置式の出力だけを見る。
#[test]
fn balloon_right_places_right_of_char_without_clamp() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(0), BalloonSide::Right, None),
        )]);

        let out = resolve_placement(&cfg, wa, &[input(0, w, h)], MEASURE_DPI);

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
    }
}

/// `balloon.offsetx/offsety` は alignment 由来の幾何値へ加算。
#[test]
fn balloon_offsetx_offsety_added() {
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

        let out = resolve_placement(&cfg, wa, &[inp], MEASURE_DPI);

        let cp = out[0].char_pos;
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: cp.x - bw + ox,
                y: cp.y + oy
            },
            "dpi={dpi}: offsetx/y 加算"
        );
    }
}

/// **配置式（P5）はバルーンをクランプしない**＝キャラが左端クランプされた
/// 状態の left バルーンは work area 左外（負方向）へ素直にはみ出したまま返る。
///
/// # 前提
///
/// 「バルーンは決してクランプされない」という意味ではない——`windowposition.limit`
/// が有効なら、この出力は下流の関門が作業領域内へ補正する（起動時＝
/// `balloon_limit::apply_balloon_limit`（`session/boot_seam.rs` の `restore_merged_placements`）／実行時＝
/// `follow::window_move::enqueue_window_set_pos` の runtime 関門／バルーン単独ドラッグ
/// の解放時＝`follow::balloon_drag::on_balloon_drag_end`・檻は `follow_drag_end_limit_tests.rs`）。補正の所有は関門であって
/// 配置式ではない、という分業を固定するのが本檻である。無クランプは
/// `resolve_placement` の契約としていまも真であり、この assert は反転しない。
#[test]
fn resolver_does_not_clamp_balloon_outside_work_area() {
    for dpi in DPIS {
        let wa = offset_work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let inp = input(0, w, h);
        let bw = inp.balloon_size.w;
        // キャラを左端クランプへ追い込む（`resolver_resolve_tests.rs` の `oversized_defaultx_clamps_to_left_edge` と同型）
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(px(40000, dpi)), BalloonSide::Left, None),
        )]);

        let out = resolve_placement(&cfg, wa, &[inp], MEASURE_DPI);

        assert_eq!(out[0].char_pos.x, wa.left, "dpi={dpi}: 前提＝左端クランプ");
        assert_eq!(
            out[0].balloon_pos.x,
            wa.left - bw,
            "dpi={dpi}: resolver は work area 左外のバルーンもクランプしない（補正は下流の関門）"
        );
    }
}

/// `balloon.alignment=none`（自動調整） → キャラ窓の中心が作業領域の中央より左なら
/// 右隣、それ以外は左隣（ukadoc「バルーンの位置情報」）。左右の両方を踏ませる。
#[test]
fn balloon_none_picks_the_side_facing_the_area_center() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let (w, h) = (px(400, dpi), px(600, dpi));
        let area_center_x = (wa.left + wa.right) / 2;
        // (default_x, キャラが中央より左に居るか)。右端密着と、左端へクランプされる大きな値。
        for (default_x, char_is_left) in [(0, false), (px(40000, dpi), true)] {
            let inp = input(0, w, h);
            let bw = inp.balloon_size.w;
            let cfg = cfg_of(vec![(
                0,
                scope_cfg_balloon(Some(default_x), BalloonSide::Auto, None),
            )]);

            let out = resolve_placement(&cfg, wa, &[inp], MEASURE_DPI);

            let cp = out[0].char_pos;
            assert_eq!(
                cp.x + w / 2 < area_center_x,
                char_is_left,
                "dpi={dpi} default_x={default_x}: 前提（キャラの左右）が崩れている"
            );
            let expected_x = if char_is_left { cp.x + w } else { cp.x - bw };
            assert_eq!(
                out[0].balloon_pos,
                PointPx {
                    x: expected_x,
                    y: cp.y
                },
                "dpi={dpi} default_x={default_x}: none は中央を向く側に置く"
            );
        }
    }
}

/// `balloon.alignment=none` でキャラ窓の中心がちょうど作業領域の中央の
/// ときは左側に付く（「中央より左なら右側」の境は左側＝`<` を `<=` に変えると赤）。
#[test]
fn balloon_none_at_exact_center_goes_left() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let w = px(400, dpi);
        let inp = input(0, w, px(600, dpi));
        let bw = inp.balloon_size.w;
        // right − w − 760 + w/2 ＝ 1920 − 400 − 760 + 200 ＝ 960（作業領域の中央）
        let cfg = cfg_of(vec![(
            0,
            scope_cfg_balloon(Some(px(760, dpi)), BalloonSide::Auto, None),
        )]);

        let out = resolve_placement(&cfg, wa, &[inp], MEASURE_DPI);

        let cp = out[0].char_pos;
        assert_eq!(
            cp.x + w / 2,
            (wa.left + wa.right) / 2,
            "dpi={dpi}: 前提（中心がちょうど中央）が崩れている"
        );
        assert_eq!(
            out[0].balloon_pos,
            PointPx {
                x: cp.x - bw,
                y: cp.y
            },
            "dpi={dpi}: ちょうど中央は左側"
        );
    }
}

// ------------------------------------------------------------------
// 右に出す腕の descript の分は左向きが＋
// ------------------------------------------------------------------

/// descript の分 `d`・合流後の総量 `total` を持つ構成（右の腕の檻用）。
fn cfg_descript(side: BalloonSide, total_x: i32, descript_x: i32) -> ScopeConfig {
    ScopeConfig {
        balloon_offset_descript_x: descript_x,
        ..scope_cfg_balloon(Some(0), side, Some((total_x, 0)))
    }
}

/// キャラ窓の右端と、その scope のバルーン x を返す。
fn right_edge_and_balloon_x(cfg: ScopeConfig) -> (i32, i32) {
    let dpi = 96;
    let out = resolve_placement(
        &cfg_of(vec![(0, cfg)]),
        work_area(dpi),
        &[input(0, px(400, dpi), px(600, dpi))],
        MEASURE_DPI,
    );
    (out[0].char_pos.x + out[0].char_size.w, out[0].balloon_pos.x)
}

/// 右に出す＋descript 30（合流後 30）→ 右端 − 30。
#[test]
fn right_arm_descript_offset_goes_toward_the_shell() {
    let (edge, bx) = right_edge_and_balloon_x(cfg_descript(BalloonSide::Right, 30, 30));
    assert_eq!(bx, edge - 30);
}

/// 左に出す腕は変えない（descript 30 → キャラ左隣 + 30）。
#[test]
fn left_arm_descript_offset_unchanged() {
    let dpi = 96;
    let inp = input(0, px(400, dpi), px(600, dpi));
    let bw = inp.balloon_size.w;
    let out = resolve_placement(
        &cfg_of(vec![(0, cfg_descript(BalloonSide::Left, 30, 30))]),
        work_area(dpi),
        &[inp],
        MEASURE_DPI,
    );
    assert_eq!(out[0].balloon_pos.x, out[0].char_pos.x - bw + 30);
}

/// `Auto` で右に決まる形（キャラが作業領域の中央より左）でも右端 − 30。
#[test]
fn auto_resolving_right_descript_offset_goes_toward_the_shell() {
    let dpi = 96;
    let mut cfg = cfg_descript(BalloonSide::Auto, 30, 30);
    cfg.default_x = Some(px(40000, dpi)); // 左端へクランプ＝中央より左
    let (edge, bx) = right_edge_and_balloon_x(cfg);
    assert_eq!(bx, edge - 30);
}

/// 右・descript 0・`windowposition.x` の分 −237（合流後 −237）→ 右端 − 237（今の式と同一）。
#[test]
fn right_arm_windowposition_part_only_keeps_current_formula() {
    let (edge, bx) = right_edge_and_balloon_x(cfg_descript(BalloonSide::Right, -237, 0));
    assert_eq!(bx, edge - 237);
}
