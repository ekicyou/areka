//! `balloon_limit` クランプ核の決定論檻（要件 7.1・design「Testing Strategy / Unit Tests 1」）。
//!
//! 行列: はみ出し方向 4 辺の**単独**と**複合**（4 隅）× k=1 相当（dpi=96）および
//! k≠1 相当（dpi=120/144/192）の寸法組 × 作業領域より大きいバルーン（逆転区間）×
//! 非はみ出し（無補正）× 冪等性 × キャラ窓の既存クランプ（`resolver.rs` の
//! `clamp_axis`）との意味論同値。
//!
//! limit の有効／無効（limit={0,1}）は**呼び出し側（関門）の責務**であり本層は
//! 「常にクランプする式」しか持たない（design C5「limit=0 は呼び出し側で skip」）。
//! ゆえに本檻は limit の分岐を持たず、式そのものの判断分岐だけを全網羅する。

use super::*;

// ---------------------------------------------------------------------------
// 檻の道具（本体とは独立にここへ置く・resolver 檻 `resolver_test_support.rs` と同流儀）
// ---------------------------------------------------------------------------

/// DPI パラメタ水準。96＝k=1 相当、120/144/192＝k≠1 相当（3.4・U5・要件 2.10）。
const DPIS: [i32; 4] = [96, 120, 144, 192];

/// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制）。
///
/// 本体は物理 px しか見ない（要件 2.10）。同じ論理形状を各 DPI の物理値で与えたとき
/// 期待値も同じ物理式から出ることを固定する（隠れた `/96` 変換や丸めがあれば
/// 96 以外の水準で崩れる）。
fn px(logical: i32, dpi: i32) -> i32 {
    assert_eq!(
        (logical * dpi) % 96,
        0,
        "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
    );
    logical * dpi / 96
}

/// 左上が原点**でない**作業領域（クランプ境界の left/top 依存を暴く）。
fn work_area(dpi: i32) -> RectPx {
    RectPx {
        left: px(64, dpi),
        top: px(40, dpi),
        right: px(64 + 1920, dpi),
        bottom: px(40 + 1080, dpi),
    }
}

/// 検証用バルーン寸（論理 400x300）。
fn balloon_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(400, dpi),
        h: px(300, dpi),
    }
}

fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// `resolver.rs` の private な `clamp_axis` を**本体の import とは独立に**逐語再掲した参照実装。
///
/// キャラ窓の既存クランプと同一の意味論であることを檻側で突合するための基準
/// （逆転区間 `hi < lo` では `lo`＝left/top 側が勝つ・`i32::clamp` は panic するため使わない）。
fn clamp_axis_reference(v: i32, lo: i32, hi: i32) -> i32 {
    v.min(hi).max(lo)
}

/// 矩形 (pos, size) が area へ 4 辺内包されているか（right/bottom は排他側）。
///
/// 右下辺の算出は `saturating_add`——本檻は極値入力（`i32::MAX` 位置）も与えるため、
/// **判定道具の側が先に溢れて**本体の欠陥を隠すことがあってはならない。
fn contained(pos: PointPx, size: SizePx, area: RectPx) -> bool {
    pos.x >= area.left
        && pos.y >= area.top
        && pos.x.saturating_add(size.w) <= area.right
        && pos.y.saturating_add(size.h) <= area.bottom
}

/// 位置グリッド（十分外側〜内側〜十分外側）。等値・冪等・事後条件の全面走査に使う。
fn position_grid(area: RectPx, dpi: i32) -> Vec<PointPx> {
    let step = px(160, dpi);
    let mut out = Vec::new();
    let mut x = area.left - step * 3;
    while x <= area.right + step * 3 {
        let mut y = area.top - step * 3;
        while y <= area.bottom + step * 3 {
            out.push(point(x, y));
            y += step;
        }
        x += step;
    }
    out
}

// ---------------------------------------------------------------------------
// T-BL1: 非はみ出し＝無補正（要件 2.2 の裏・「補正なし」が観測できること）
// ---------------------------------------------------------------------------

/// 完全に内側の矩形はクランプで動かず、`limit_correction` は `None`（無補正）を返す。
#[test]
fn t_bl1_inside_is_not_corrected() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let sz = balloon_size(dpi);
        let pos = point(wa.left + px(100, dpi), wa.top + px(80, dpi));

        assert_eq!(
            clamp_rect_to_work_area(pos, sz, wa),
            pos,
            "dpi={dpi}: 内側の矩形は動かない"
        );
        assert_eq!(
            limit_correction(pos, sz, wa),
            None,
            "dpi={dpi}: 補正不要なら None（無補正の観測可能化）"
        );
    }
}

/// 4 辺いずれかにぴったり接している境界ケースも「はみ出していない」＝無補正。
#[test]
fn t_bl1b_flush_edges_are_not_corrected() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let sz = balloon_size(dpi);
        let flush = [
            point(wa.left, wa.top),                   // 左上ぴったり
            point(wa.right - sz.w, wa.top),           // 右辺ぴったり
            point(wa.left, wa.bottom - sz.h),         // 下辺ぴったり
            point(wa.right - sz.w, wa.bottom - sz.h), // 右下ぴったり
        ];
        for pos in flush {
            assert_eq!(
                limit_correction(pos, sz, wa),
                None,
                "dpi={dpi}: 辺に接するだけの位置 {pos:?} は補正不要"
            );
            assert!(contained(pos, sz, wa), "dpi={dpi}: 前提（接触は内包）");
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL2: はみ出し方向 4 辺の**単独**（要件 2.3）
// ---------------------------------------------------------------------------

/// 左辺・右辺・上辺・下辺それぞれ単独のはみ出しを、該当軸だけ引き戻す。
/// 反対側の軸は 1px も動かない（辺の取り違えを暴く）。
#[test]
fn t_bl2_single_edge_overflow_each_side() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let sz = balloon_size(dpi);
        let over = px(200, dpi);
        let inx = wa.left + px(100, dpi);
        let iny = wa.top + px(80, dpi);

        let cases: [(&str, PointPx, PointPx); 4] = [
            ("left", point(wa.left - over, iny), point(wa.left, iny)),
            (
                "right",
                point(wa.right - sz.w + over, iny),
                point(wa.right - sz.w, iny),
            ),
            ("top", point(inx, wa.top - over), point(inx, wa.top)),
            (
                "bottom",
                point(inx, wa.bottom - sz.h + over),
                point(inx, wa.bottom - sz.h),
            ),
        ];

        for (side, pos, expected) in cases {
            assert_eq!(
                clamp_rect_to_work_area(pos, sz, wa),
                expected,
                "dpi={dpi} side={side}: 単独はみ出しは該当軸だけ引き戻す"
            );
            assert_eq!(
                limit_correction(pos, sz, wa),
                Some(expected),
                "dpi={dpi} side={side}: 補正要のとき Some(補正後位置)"
            );
            assert!(
                contained(expected, sz, wa),
                "dpi={dpi} side={side}: 事後条件＝4 辺内包"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL3: はみ出しの**複合**（4 隅・要件 2.3「辺の組み合わせによらず同一規則」）
// ---------------------------------------------------------------------------

/// 2 辺同時のはみ出し（左上・右上・左下・右下）を 1 回で両軸とも収める。
#[test]
fn t_bl3_composite_corner_overflow() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let sz = balloon_size(dpi);
        let over = px(240, dpi);
        let (lo_x, hi_x) = (wa.left, wa.right - sz.w);
        let (lo_y, hi_y) = (wa.top, wa.bottom - sz.h);

        let cases: [(&str, PointPx, PointPx); 4] = [
            (
                "left-top",
                point(lo_x - over, lo_y - over),
                point(lo_x, lo_y),
            ),
            (
                "right-top",
                point(hi_x + over, lo_y - over),
                point(hi_x, lo_y),
            ),
            (
                "left-bottom",
                point(lo_x - over, hi_y + over),
                point(lo_x, hi_y),
            ),
            (
                "right-bottom",
                point(hi_x + over, hi_y + over),
                point(hi_x, hi_y),
            ),
        ];

        for (corner, pos, expected) in cases {
            assert_eq!(
                clamp_rect_to_work_area(pos, sz, wa),
                expected,
                "dpi={dpi} corner={corner}: 複合はみ出しも同一規則で 1 回で収める"
            );
            assert_eq!(limit_correction(pos, sz, wa), Some(expected));
            assert!(contained(expected, sz, wa), "dpi={dpi} corner={corner}");
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL4: 作業領域より大きいバルーン＝逆転区間の左/上優先（要件 2.4）
// ---------------------------------------------------------------------------

/// バルーンが作業領域より大きい軸では left／top 辺を一致させる（両端は同時に収まらない）。
/// 幅だけ超過・高さだけ超過・両方超過の 3 通り、および右側から寄せた入力でも同じ結論。
#[test]
fn t_bl4_oversized_balloon_prefers_left_top() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let wide = SizePx {
            w: (wa.right - wa.left) + px(400, dpi),
            h: px(300, dpi),
        };
        let tall = SizePx {
            w: px(400, dpi),
            h: (wa.bottom - wa.top) + px(400, dpi),
        };
        let huge = SizePx {
            w: wide.w,
            h: tall.h,
        };

        // 左上から遠く外れた入力と、右下から遠く外れた入力の双方で同じ左/上優先へ落ちる
        let far_left_top = point(wa.left - px(800, dpi), wa.top - px(800, dpi));
        let far_right_bottom = point(wa.right + px(800, dpi), wa.bottom + px(800, dpi));

        for (label, sz, expect_left, expect_top) in [
            ("wide", wide, true, false),
            ("tall", tall, false, true),
            ("huge", huge, true, true),
        ] {
            for pos in [far_left_top, far_right_bottom] {
                let got = clamp_rect_to_work_area(pos, sz, wa);
                if expect_left {
                    assert_eq!(
                        got.x, wa.left,
                        "dpi={dpi} {label}: 幅超過軸は left 辺一致（{pos:?}）"
                    );
                } else {
                    assert_eq!(
                        got.x,
                        clamp_axis_reference(pos.x, wa.left, wa.right - sz.w),
                        "dpi={dpi} {label}: 非超過軸は通常クランプ"
                    );
                }
                if expect_top {
                    assert_eq!(
                        got.y, wa.top,
                        "dpi={dpi} {label}: 高さ超過軸は top 辺一致（{pos:?}）"
                    );
                } else {
                    assert_eq!(
                        got.y,
                        clamp_axis_reference(pos.y, wa.top, wa.bottom - sz.h),
                        "dpi={dpi} {label}: 非超過軸は通常クランプ"
                    );
                }
            }
        }
    }
}

/// 縮退した作業領域（right < left・bottom < top＝解決不能寸）でも panic せず left/top 優先。
#[test]
fn t_bl4b_degenerate_area_prefers_left_top() {
    for dpi in DPIS {
        let degenerate = RectPx {
            left: px(400, dpi),
            top: px(300, dpi),
            right: px(200, dpi),
            bottom: px(100, dpi),
        };
        let sz = balloon_size(dpi);
        for pos in [
            point(0, 0),
            point(px(4000, dpi), px(4000, dpi)),
            point(px(400, dpi), px(300, dpi)),
        ] {
            let got = clamp_rect_to_work_area(pos, sz, degenerate);
            assert_eq!(
                got,
                point(degenerate.left, degenerate.top),
                "dpi={dpi}: 逆転区間は left/top 側が勝つ（panic しない）"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL5: 冪等性（要件 2.1 の常時不変量が再適用で壊れない）
// ---------------------------------------------------------------------------

/// クランプ結果を再度クランプしても不変（グリッド全走査・通常寸と超過寸の双方）。
#[test]
fn t_bl5_clamp_is_idempotent() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let oversized = SizePx {
            w: (wa.right - wa.left) + px(200, dpi),
            h: (wa.bottom - wa.top) + px(200, dpi),
        };
        for sz in [balloon_size(dpi), oversized] {
            for pos in position_grid(wa, dpi) {
                let once = clamp_rect_to_work_area(pos, sz, wa);
                let twice = clamp_rect_to_work_area(once, sz, wa);
                assert_eq!(once, twice, "dpi={dpi} sz={sz:?} pos={pos:?}: 冪等");
                assert_eq!(
                    limit_correction(once, sz, wa),
                    None,
                    "dpi={dpi} sz={sz:?} pos={pos:?}: 補正後は無補正へ収束"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL6: キャラ窓の既存クランプ（resolver `clamp_axis`）との意味論同値
// ---------------------------------------------------------------------------

/// 2 軸独立に `clamp_axis` を適用したものと厳密に一致する（グリッド全走査・超過寸込み）。
#[test]
fn t_bl6_matches_clamp_axis_semantics() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let oversized = SizePx {
            w: (wa.right - wa.left) + px(200, dpi),
            h: (wa.bottom - wa.top) + px(200, dpi),
        };
        for sz in [balloon_size(dpi), oversized] {
            for pos in position_grid(wa, dpi) {
                let expected = point(
                    clamp_axis_reference(pos.x, wa.left, wa.right - sz.w),
                    clamp_axis_reference(pos.y, wa.top, wa.bottom - sz.h),
                );
                assert_eq!(
                    clamp_rect_to_work_area(pos, sz, wa),
                    expected,
                    "dpi={dpi} sz={sz:?} pos={pos:?}: clamp_axis と同一意味論"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL7: 2 関数の整合と事後条件（グリッド全走査）
// ---------------------------------------------------------------------------

/// `limit_correction` は「クランプ結果が入力と異なるときだけ Some」であり、
/// `Some` の中身は `clamp_rect_to_work_area` と一致する。
/// 収まる寸法なら結果は必ず 4 辺内包（事後条件）。
#[test]
fn t_bl7_correction_agrees_with_clamp_and_contains() {
    for dpi in DPIS {
        let wa = work_area(dpi);
        let sz = balloon_size(dpi);
        for pos in position_grid(wa, dpi) {
            let clamped = clamp_rect_to_work_area(pos, sz, wa);
            let expected = if clamped == pos { None } else { Some(clamped) };
            assert_eq!(
                limit_correction(pos, sz, wa),
                expected,
                "dpi={dpi} pos={pos:?}: 補正要否と補正値の整合"
            );
            assert!(
                contained(clamped, sz, wa),
                "dpi={dpi} pos={pos:?}: 事後条件＝4 辺内包"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// T-BL8: 極値でパニックしない（純関数の防波堤・resolver の飽和流儀を継承）
// ---------------------------------------------------------------------------

/// 極端な座標・作業領域でもオーバーフロー panic せず、内包側へ落ちる。
#[test]
fn t_bl8_extreme_values_do_not_panic() {
    let wa = RectPx {
        left: -1_000,
        top: -1_000,
        right: 1_000,
        bottom: 1_000,
    };
    let sz = SizePx { w: 400, h: 300 };
    for pos in [
        point(i32::MAX, i32::MAX),
        point(i32::MIN, i32::MIN),
        point(i32::MAX, i32::MIN),
        point(i32::MIN, i32::MAX),
    ] {
        let got = clamp_rect_to_work_area(pos, sz, wa);
        assert!(contained(got, sz, wa), "pos={pos:?}: 極値でも内包へ落ちる");
    }

    // 作業領域そのものが極値でも panic しない（`right - w` が i32 下限を割らない）
    let extreme_area = RectPx {
        left: i32::MIN,
        top: i32::MIN,
        right: i32::MAX,
        bottom: i32::MAX,
    };
    let got = clamp_rect_to_work_area(point(0, 0), sz, extreme_area);
    assert_eq!(got, point(0, 0), "極値作業領域では原点は動かない");
}

// ---------------------------------------------------------------------------
// T-BL9: 観測タグ定数（以降の全関門が共有する語彙・design C10）
// ---------------------------------------------------------------------------

/// タグ文字列は実機サインオフの grep 判定語（手順書と 1:1）。
/// **本体の定数とは独立に literal で置き**、勝手な変更を檻が止める。
#[test]
fn t_bl9_observation_tags_are_stable_literals() {
    assert_eq!(BALLOON_LIMIT_CLAMP_TAG, "[balloon-limit] Clamp");
    assert_eq!(BALLOON_LIMIT_UNRESOLVED_TAG, "[balloon-limit] Unresolved");
}
