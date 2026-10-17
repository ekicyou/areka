// =============================================================================
// `\![move]` の移動先の計算の決定論檻（design.md Testing Strategy「move_cue_geometry_tests.rs」・
// 要件 1.3・1.5・2.1〜2.4・2.6・3.4・4.1〜4.3・6.1・6.2）
// =============================================================================

use super::*;
use crate::emo2_boot::move_cue::{
    Axis, AxisSpec, MoveBase, MoveDirective, RefPoint, RefX, RefY, parse_move_directive,
};
use wintf::ecs::SizeI;

const K1: ScaleRatio = ScaleRatio::ONE;

fn k2() -> ScaleRatio {
    ScaleRatio::new(2, 1).expect("2/1")
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
    RectPx {
        left,
        top,
        right,
        bottom,
    }
}

fn pt(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

fn directive(
    x: AxisSpec,
    y: AxisSpec,
    base_offset: RefPoint,
    move_offset: RefPoint,
) -> MoveDirective {
    MoveDirective {
        scope: 1,
        x,
        y,
        duration_ms: 0,
        base: MoveBase::Scope(0),
        base_offset,
        move_offset,
        ignore_sticky_window: false,
    }
}

fn parse(items: &[&str]) -> MoveDirective {
    let tokens: Vec<String> = items.iter().map(|s| s.to_string()).collect();
    let parsed = parse_move_directive(1, &tokens);
    assert!(
        parsed.notes.is_empty(),
        "検体の台本は読みの置き換えが無い: {:?}",
        parsed.notes
    );
    parsed.directive
}

/// 参照点の語を軸ごとに同じ順で並べる（left↔top・right↔bottom・center↔center・base↔base）。
const WORDS: [(RefX, RefY); 4] = [
    (RefX::Left, RefY::Top),
    (RefX::Right, RefY::Bottom),
    (RefX::Center, RefY::Center),
    (RefX::Base, RefY::Base),
];

// -------------------------------------------------------------------------
// 参照点 4×4 の全組（要件 2.1・2.2・2.3・4.3）
// -------------------------------------------------------------------------

/// 窓の枠: 枠の上の点 4 語 × 動かす窓の中の点 4 語の全組。奇数の幅・高さ（半分は切り捨て）。
///
/// 枠 (100,200)-(501,901)（幅 401・高さ 701）・basepos (50,30):
/// 横 left 100・right 501・center 300・base 150／縦 top 200・bottom 901・center 550・base 230。
/// 動かす窓 (1000,1000)-(1301,1501)（幅 301・高さ 501）・basepos (10,20):
/// 横 left 0・right 301・center 150・base 10／縦 top 0・bottom 501・center 250・base 20。
#[test]
fn window_frame_all_ref_point_pairs() {
    let frame = BaseFrame::Window {
        rect: rect(100, 200, 501, 901),
        basepos: pt(50, 30),
    };
    let target = rect(1000, 1000, 1301, 1501);
    let target_bp = pt(10, 20);
    let frame_x = [100, 501, 300, 150];
    let frame_y = [200, 901, 550, 230];
    let inner_x = [0, 301, 150, 10];
    let inner_y = [0, 501, 250, 20];

    for (b, &(bx, by)) in WORDS.iter().enumerate() {
        for (m, &(mx, my)) in WORDS.iter().enumerate() {
            let d = directive(
                AxisSpec::Px(7),
                AxisSpec::Px(-3),
                RefPoint { x: bx, y: by },
                RefPoint { x: mx, y: my },
            );
            let got = compute_move_target(&frame, target, target_bp, &d, K1);
            assert_eq!(
                got.position,
                pt(frame_x[b] + 7 - inner_x[m], frame_y[b] - 3 - inner_y[m]),
                "base={bx:?}.{by:?} move={mx:?}.{my:?}"
            );
            assert_eq!(got.base_offset_used, RefPoint { x: bx, y: by });
            assert!(
                got.notes.is_empty(),
                "窓の枠は置き換えない: {:?}",
                got.notes
            );
        }
    }
}

/// 作業領域の枠: `base` は軸ごとに `center` へ（要件 4.1）。動かす窓の `base` は置き換えない（4.3）。
///
/// 作業領域 (0,0)-(2880,1704): 横 left 0・right 2880・center 1440・base→1440／
/// 縦 top 0・bottom 1704・center 852・base→852。
#[test]
fn area_frame_all_ref_point_pairs() {
    let frame = BaseFrame::Area(rect(0, 0, 2880, 1704));
    let target = rect(1000, 1000, 1301, 1501);
    let target_bp = pt(10, 20);
    let frame_x = [0, 2880, 1440, 1440];
    let frame_y = [0, 1704, 852, 852];
    let inner_x = [0, 301, 150, 10];
    let inner_y = [0, 501, 250, 20];

    for (b, &(bx, by)) in WORDS.iter().enumerate() {
        for (m, &(mx, my)) in WORDS.iter().enumerate() {
            let d = directive(
                AxisSpec::Px(7),
                AxisSpec::Px(-3),
                RefPoint { x: bx, y: by },
                RefPoint { x: mx, y: my },
            );
            let got = compute_move_target(&frame, target, target_bp, &d, K1);
            assert_eq!(
                got.position,
                pt(frame_x[b] + 7 - inner_x[m], frame_y[b] - 3 - inner_y[m]),
                "base={bx:?}.{by:?} move={mx:?}.{my:?}"
            );
            let substituted = bx == RefX::Base;
            let expected_notes = if substituted {
                vec![
                    FrameNote::AreaBaseToCenter { axis: Axis::X },
                    FrameNote::AreaBaseToCenter { axis: Axis::Y },
                ]
            } else {
                vec![]
            };
            assert_eq!(
                got.notes, expected_notes,
                "記録は基準の参照点だけ（動かす窓は置き換えない）"
            );
            let used = if substituted {
                RefPoint {
                    x: RefX::Center,
                    y: RefY::Center,
                }
            } else {
                RefPoint { x: bx, y: by }
            };
            assert_eq!(got.base_offset_used, used);
        }
    }
}

/// `base` が `CanonDefaultBasepos` の値（幅÷2・下端）になる（要件 2.3）。
#[test]
fn base_word_reads_canon_default_basepos() {
    let frame_rect = rect(726, 610, 1490, 1704);
    let frame = BaseFrame::Window {
        rect: frame_rect,
        basepos: CanonDefaultBasepos.basepos(SizeI::new(764, 1094)),
    };
    let target = rect(0, 0, 672, 800);
    let target_bp = CanonDefaultBasepos.basepos(SizeI::new(672, 800));
    let d = directive(
        AxisSpec::Px(0),
        AxisSpec::Px(0),
        RefPoint::BASE_BASE,
        RefPoint::BASE_BASE,
    );
    let got = compute_move_target(&frame, target, target_bp, &d, K1);
    // 枠の基準位置 (726+382, 610+1094) に動かす窓の基準位置 (336, 800) がそろう。
    assert_eq!(got.position, pt(1108 - 336, 1704 - 800));
}

// -------------------------------------------------------------------------
// k 倍・Fix（要件 2.4・1.5）
// -------------------------------------------------------------------------

/// k=2 で台本の X・Y だけが 2 倍になる（枠と窓の点は物理 px のまま）。
#[test]
fn k2_scales_script_offsets_only() {
    let frame = BaseFrame::Window {
        rect: rect(100, 200, 501, 901),
        basepos: pt(50, 30),
    };
    let target = rect(1000, 1000, 1301, 1501);
    let d = directive(
        AxisSpec::Px(7),
        AxisSpec::Px(-3),
        RefPoint {
            x: RefX::Right,
            y: RefY::Bottom,
        },
        RefPoint {
            x: RefX::Center,
            y: RefY::Center,
        },
    );
    let got = compute_move_target(&frame, target, pt(10, 20), &d, k2());
    assert_eq!(got.position, pt(501 + 14 - 150, 901 - 6 - 250));
}

/// `Fix` の軸は今の左端（上端）のまま。他方の軸は計算される（要件 1.5）。
#[test]
fn fix_axis_keeps_current_edge() {
    let frame = BaseFrame::Area(rect(0, 0, 2880, 1704));
    let target = rect(1000, 500, 1672, 1300);
    let both = directive(
        AxisSpec::Fix,
        AxisSpec::Fix,
        RefPoint::BASE_BASE,
        RefPoint::BASE_BASE,
    );
    assert_eq!(
        compute_move_target(&frame, target, pt(336, 800), &both, k2()).position,
        pt(1000, 500)
    );
    let x_only = directive(
        AxisSpec::Px(10),
        AxisSpec::Fix,
        RefPoint::LEFT_TOP,
        RefPoint::LEFT_TOP,
    );
    assert_eq!(
        compute_move_target(&frame, target, pt(336, 800), &x_only, K1).position,
        pt(10, 500)
    );
}

// -------------------------------------------------------------------------
// 置き換えの記録（要件 4.1・4.2・4.3）
// -------------------------------------------------------------------------

/// 作業領域の `left.base` は縦だけ `center` へ置き換え、記録も縦の 1 件だけ（要件 4.1）。
#[test]
fn area_base_substitution_is_per_axis() {
    let frame = BaseFrame::Area(rect(0, 0, 2880, 1704));
    let d = directive(
        AxisSpec::Px(0),
        AxisSpec::Px(0),
        RefPoint {
            x: RefX::Left,
            y: RefY::Base,
        },
        RefPoint::LEFT_TOP,
    );
    let got = compute_move_target(&frame, rect(1440, 852, 2112, 1652), pt(336, 800), &d, K1);
    assert_eq!(got.position, pt(0, 852));
    assert_eq!(
        got.notes,
        vec![FrameNote::AreaBaseToCenter { axis: Axis::Y }]
    );
    assert_eq!(
        got.base_offset_used,
        RefPoint {
            x: RefX::Left,
            y: RefY::Center
        }
    );
}

/// 原点の基準の参照点が `left.top` 以外なら原点そのもの・記録 1 件（要件 4.2・3.4）。
/// `left.top` なら記録しない。動かす窓の参照点は効く（要件 4.3）。
#[test]
fn origin_ignores_non_left_top_offset() {
    let target = rect(1000, 500, 1672, 1300);
    for (bx, by) in WORDS {
        let offset = RefPoint { x: bx, y: by };
        let d = directive(
            AxisSpec::Px(100),
            AxisSpec::Px(100),
            offset,
            RefPoint {
                x: RefX::Right,
                y: RefY::Bottom,
            },
        );
        let got = compute_move_target(&BaseFrame::Origin, target, pt(336, 800), &d, K1);
        assert_eq!(got.position, pt(100 - 672, 100 - 800), "offset={offset:?}");
        assert_eq!(got.base_offset_used, RefPoint::LEFT_TOP);
        let expected = if offset == RefPoint::LEFT_TOP {
            vec![]
        } else {
            vec![FrameNote::GlobalOffsetIgnored { offset }]
        };
        assert_eq!(got.notes, expected);
    }
}

// -------------------------------------------------------------------------
// SSP 2.9.18 の実測表（research.md §8.3・k=1 で比べる）
// -------------------------------------------------------------------------

/// 測る前の机: `\0` (726,610)-(1490,1704)・主モニタの作業領域 (0,0)-(2880,1704)。
const SCOPE0: RectPx = RectPx {
    left: 726,
    top: 610,
    right: 1490,
    bottom: 1704,
};
const WORK: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 2880,
    bottom: 1704,
};

fn canon(r: RectPx) -> PointPx {
    CanonDefaultBasepos.basepos(SizeI::new(r.right - r.left, r.bottom - r.top))
}

/// 実測の机で基準の枠を選ぶ（適用の側が World から選ぶものの代わり）。
fn ssp_frame(base: &MoveBase, before: RectPx) -> BaseFrame {
    match base {
        MoveBase::Scope(0) => BaseFrame::Window {
            rect: SCOPE0,
            basepos: canon(SCOPE0),
        },
        MoveBase::Me => BaseFrame::Window {
            rect: before,
            basepos: canon(before),
        },
        MoveBase::Screen | MoveBase::PrimaryScreen => BaseFrame::Area(WORK),
        MoveBase::Global => BaseFrame::Origin,
        other => panic!("実測表に無い基準: {other:?}"),
    }
}

/// (番号, 台本の引数, 前の `` の左上, 後の左上)。
type SspRow = (
    &'static str,
    &'static [&'static str],
    (i32, i32),
    (i32, i32),
);

#[test]
fn ssp_measurement_table_positions_match() {
    // (番号, 台本の引数, 前の `\1` の矩形, 後の左上)。`\1` は 672×800。
    let rows: &[SspRow] = &[
        (
            "M1-c",
            &["--X=0", "--Y=-900", "--base=0", "--base-offset=left.bottom"],
            (726, 904),
            (726, 804),
        ),
        (
            "M1-e",
            &["--X=0", "--Y=0", "--base=0", "--move-offset=right.bottom"],
            (726, 1704),
            (54, -190),
        ),
        (
            "M1-f",
            &[
                "--X=0",
                "--Y=0",
                "--base=0",
                "--base-offset=left.bottom",
                "--move-offset=right.bottom",
            ],
            (54, -190),
            (54, 904),
        ),
        ("M2-a", &["100", "0", "0", "0"], (54, 904), (826, 610)),
        (
            "M2-b",
            &["-353", "", "", "0", "base", "base"],
            (826, 610),
            (419, 610),
        ),
        (
            "M3-b",
            &[
                "--base=screen",
                "--base-offset=right.bottom",
                "--move-offset=right.bottom",
                "--X=0",
                "--Y=0",
            ],
            (419, 610),
            (2208, 904),
        ),
        (
            "M3-c",
            &[
                "--X=-100",
                "--Y=-100",
                "--base=primaryscreen",
                "--base-offset=right.bottom",
                "--move-offset=right.bottom",
            ],
            (2208, 904),
            (2108, 804),
        ),
        ("M3-d", &["--X=0", "--Y=0"], (2108, 804), (0, 0)),
        (
            "M4-a",
            &["--X=0", "--Y=0", "--base=screen", "--base-offset=base.base"],
            (1000, 500),
            (1440, 852),
        ),
        (
            "M4-b",
            &[
                "--X=0",
                "--Y=0",
                "--base=primaryscreen",
                "--base-offset=base.base",
            ],
            (1000, 500),
            (1440, 852),
        ),
        (
            "M4-c",
            &["--X=0", "--Y=0", "--base=screen", "--base-offset=left.base"],
            (1440, 852),
            (0, 852),
        ),
        (
            "M4-d",
            &[
                "--X=0",
                "--Y=0",
                "--base=global",
                "--base-offset=right.bottom",
            ],
            (1000, 500),
            (0, 0),
        ),
        (
            "M4-e",
            &[
                "--X=0",
                "--Y=0",
                "--base=global",
                "--base-offset=center.center",
            ],
            (1000, 500),
            (0, 0),
        ),
        (
            "M4-f",
            &["--X=0", "--Y=0", "--base=global", "--base-offset=base.base"],
            (1000, 500),
            (0, 0),
        ),
        (
            "M5",
            &["--base=global", "--X=100", "--Y=100"],
            (0, 0),
            (100, 100),
        ),
        (
            "M6-a",
            &["--base=me", "--X=10", "--Y=0"],
            (726, 610),
            (736, 610),
        ),
        (
            "M6-b",
            &["--X=fix", "--Y=50", "--base=0"],
            (736, 610),
            (736, 660),
        ),
    ];
    for &(id, args, (bx, by), (ax, ay)) in rows {
        let d = parse(args);
        let before = rect(bx, by, bx + 672, by + 800);
        let frame = ssp_frame(&d.base, before);
        let got = compute_move_target(&frame, before, canon(before), &d, K1);
        assert_eq!(got.position, pt(ax, ay), "{id}");
    }
}

// -------------------------------------------------------------------------
// emo2（要件 6.1・6.2・2.6）
// -------------------------------------------------------------------------

/// emo2 の位置調整は k=1・2 で「エモの右下＝むらさきの左下」（k に依らない）。
#[test]
fn emo2_adjust_aligns_bottom_right_to_bottom_left() {
    let d = parse(&[
        "--X=0",
        "--Y=0",
        "--base=0",
        "--base-offset=left.bottom",
        "--move-offset=right.bottom",
    ]);
    let (x0, y0, w0, h0) = (1483, 377, 435, 687);
    let (w1, h1) = (279, 401);
    let frame_rect = rect(x0, y0, x0 + w0, y0 + h0);
    let target = rect(-50, 900, -50 + w1, 900 + h1);
    for k in [K1, k2()] {
        let frame = BaseFrame::Window {
            rect: frame_rect,
            basepos: canon(frame_rect),
        };
        let got = compute_move_target(&frame, target, canon(target), &d, k);
        assert_eq!(got.position, pt(x0 - w1, y0 + h0 - h1), "k={k:?}");
        assert!(got.notes.is_empty());
    }
}

/// 旧い emo2（`-353,,,0,base,base`）の着地は本機能の前（基準位置どうしの式
/// `x' = x0 + w0/2 + k·(−353) − w1/2`・Y は `fix` で今のまま）と同じ値（要件 2.6・2.4）。
/// 本機能の前の計算（`resolve_move_target_position`）の檻から移した値で固定する。
#[test]
fn legacy_emo2_lands_where_it_did_before() {
    let d = parse(&["-353", "", "", "0", "base", "base"]);
    // (基準の窓, 動かす窓, k, 後の左上)
    let rows = [
        // 1000 + 200 − 353 − 150 = 697
        (
            rect(1000, 500, 1400, 1187),
            rect(1200, 800, 1500, 1234),
            K1,
            pt(697, 800),
        ),
        // 1000 + 200 − 706 − 150 = 344（台本の値だけが k 倍）
        (
            rect(1000, 500, 1400, 1187),
            rect(1200, 800, 1500, 1234),
            k2(),
            pt(344, 800),
        ),
        // 1483 + 217 − 353 − 139 = 1208
        (
            rect(1483, 377, 1918, 1064),
            rect(1200, 800, 1479, 1234),
            K1,
            pt(1208, 800),
        ),
        // 1483 + 217 − 706 − 139 = 855
        (
            rect(1483, 377, 1918, 1064),
            rect(1200, 800, 1479, 1234),
            k2(),
            pt(855, 800),
        ),
    ];
    for (base, target, k, want) in rows {
        let frame = BaseFrame::Window {
            rect: base,
            basepos: canon(base),
        };
        let got = compute_move_target(&frame, target, canon(target), &d, k);
        assert_eq!(got.position, want, "k={k:?}");
        assert!(got.notes.is_empty());
    }
}

/// 基準位置どうしで Y も書くと、両軸とも符号を保って k 倍される（Y が `fix` に固定されて
/// いないことの檻・本機能の前の檻から移した値）。
#[test]
fn legacy_base_base_scales_both_axes_with_sign() {
    let d = parse(&["100", "50", "", "0", "base", "base"]);
    let base = rect(1000, 500, 1400, 1187);
    let target = rect(1200, 800, 1500, 1234);
    let frame = BaseFrame::Window {
        rect: base,
        basepos: canon(base),
    };
    // x' = 1000 + 200 + 200 − 150 = 1250・y' = 500 + 687 + 100 − 434 = 853
    assert_eq!(
        compute_move_target(&frame, target, canon(target), &d, k2()).position,
        pt(1250, 853)
    );
}

/// 今の既定の基準位置は（幅÷2・下端）。奇数の幅は切り捨て（要件 2.3）。
#[test]
fn canon_default_basepos_is_half_width_and_bottom() {
    assert_eq!(
        CanonDefaultBasepos.basepos(SizeI::new(400, 687)),
        pt(200, 687)
    );
    assert_eq!(
        CanonDefaultBasepos.basepos(SizeI::new(435, 100)),
        pt(217, 100)
    );
}

// -------------------------------------------------------------------------
// 丸めと飽和
// -------------------------------------------------------------------------

/// 奇数の幅・高さと負の座標: 半分は切り捨て（幅 2559 → 1279・高さ 1601 → 800・幅 7 → 3・高さ 5 → 2）。
#[test]
fn odd_sizes_and_negative_coords_round_down() {
    let frame = BaseFrame::Area(rect(-2561, 195, -2, 1796));
    let center = RefPoint {
        x: RefX::Center,
        y: RefY::Center,
    };
    let d = directive(AxisSpec::Px(0), AxisSpec::Px(0), center, center);
    let got = compute_move_target(&frame, rect(10, 10, 17, 15), pt(3, 5), &d, K1);
    assert_eq!(got.position, pt(-2561 + 1279 - 3, 195 + 800 - 2));

    // 負の座標の窓の基準位置: 左端 −435・幅 435 → basepos.x 217 → 枠の点 −218。
    let win_rect = rect(-435, -100, 0, 0);
    let frame = BaseFrame::Window {
        rect: win_rect,
        basepos: canon(win_rect),
    };
    let d = directive(
        AxisSpec::Px(0),
        AxisSpec::Px(0),
        RefPoint::BASE_BASE,
        RefPoint::LEFT_TOP,
    );
    assert_eq!(
        compute_move_target(&frame, rect(10, 10, 17, 15), pt(3, 5), &d, K1).position,
        pt(-218, 0)
    );
}

/// 極端な値でも panic せず飽和する。
#[test]
fn extreme_values_saturate_without_panic() {
    let frame = BaseFrame::Window {
        rect: rect(i32::MAX - 1, i32::MAX - 1, i32::MAX, i32::MAX),
        basepos: pt(i32::MAX, i32::MAX),
    };
    let d = directive(
        AxisSpec::Px(i32::MAX),
        AxisSpec::Px(i32::MIN),
        RefPoint::BASE_BASE,
        RefPoint::LEFT_TOP,
    );
    let got = compute_move_target(
        &frame,
        rect(i32::MIN, 0, i32::MIN + 1, 1),
        pt(0, 0),
        &d,
        k2(),
    );
    assert_eq!(got.position.x, i32::MAX);
}
