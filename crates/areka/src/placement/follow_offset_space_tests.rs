//! 単位空間契約の純関数のテスト
//! （要件 2.2/2.4/2.5/3.1/3.3/3.6・7.5〜7.9・design「Unit Tests」1〜5）。
//!
//! 前半は task 1.2 が設計を駆動するために置いた最小限の主張、後半（「判断分岐の全網羅」節
//! 以降）は task 1.3 が足した網羅——表示 DPI 行列（96 の倍数でない値を分子・分母の両側へ）・
//! 負値・`i32::MIN` 近傍・飽和・作者基準 DPI の食い違い・[`OffsetRescale`] の 4 腕と
//! 値を変えない縮退 3 経路の全到達・および design D8 の残差上限の実測である。
//!
//! 期待値はすべて逐語（`assert_eq!`）で置く。近似比較はしない。
//!
//! design D8 の「揃えの残差の許容量」の実測（要件 4.4・research §12 の宿題）だけは、
//! 1,000 行制限（要件 9.6）に収めるため兄弟ファイル `follow_offset_residual_tests.rs`
//! へ分けてある。

use areka_emo_compose::ScaleRatio;
use wintf::ecs::DPI;

use super::offset_space::{
    OffsetBase, OffsetRescale, ScaledAxis, UnresolvedScale, rescale_follow_offset,
    scale_author_offset,
};
use crate::placement::resolver::PointPx;

/// 表示 DPI（両軸同値）。
fn dpi(v: u16) -> DPI {
    DPI::from_dpi(v, v)
}

/// 基準対（係留済み）。
fn base_at(x: i32, y: i32, d: u16) -> OffsetBase {
    OffsetBase {
        offset: PointPx { x, y },
        dpi: Some(dpi(d)),
    }
}

// -------------------------------------------------------------------------
// 遷移の変換規則（`rescale_follow_offset`）
// -------------------------------------------------------------------------

/// 未係留（永続値の腕・5.2）は**値を変えずに**現在の表示 DPI へ係留する。
#[test]
fn unpinned_base_anchors_without_changing_value() {
    let base = OffsetBase {
        offset: PointPx { x: 10, y: -20 },
        dpi: None,
    };
    assert_eq!(
        rescale_follow_offset(base, dpi(192)),
        OffsetRescale::Anchored { base_dpi: dpi(192) }
    );
}

/// 基準 DPI と現在 DPI が同一なら値も基準も 1 bit も動かない（恒等・2.2）。
#[test]
fn identical_dpi_is_unchanged() {
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 96), dpi(96)),
        OffsetRescale::Unchanged
    );
}

/// 表示 DPI 比だけで追随する（96→192＝2 倍・3.1）。
#[test]
fn rescales_by_display_dpi_ratio() {
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 96), dpi(192)),
        OffsetRescale::Rescaled {
            offset: PointPx { x: 20, y: -40 },
            saturated: false,
        }
    );
}

/// 往復しても基準から引き直すため bit 同一で戻る（3.3）。
#[test]
fn roundtrip_returns_bit_identical_value() {
    let base = base_at(10, -20, 96);
    let first = rescale_follow_offset(base, dpi(144));
    assert_eq!(
        rescale_follow_offset(base, dpi(96)),
        OffsetRescale::Unchanged
    );
    assert_eq!(rescale_follow_offset(base, dpi(144)), first);
    assert_eq!(
        first,
        OffsetRescale::Rescaled {
            offset: PointPx { x: 15, y: -30 },
            saturated: false,
        }
    );
}

/// 拡大率を解決できない腕は値も基準も変えず、理由を値として返す（3.6・9.4）。
#[test]
fn zero_dpi_is_unresolved_with_reason() {
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 0), dpi(192)),
        OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroBaseDpi
        }
    );
    assert_eq!(
        rescale_follow_offset(base_at(10, -20, 96), dpi(0)),
        OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroCurrentDpi
        }
    );
}

// -------------------------------------------------------------------------
// 供給時の換算（`scale_author_offset`）
// -------------------------------------------------------------------------

/// 恒等比は生値を素通しする（2.2）。
#[test]
fn identity_ratio_passes_raw_through() {
    let (x, y) = scale_author_offset((7, -13), ScaleRatio::ONE);
    assert_eq!((x.value, y.value), (7, -13));
    assert!(!x.saturated && !y.saturated);
}

/// 大きさは既存の丸め権威へ委譲する（5/4 倍・2.4）。
#[test]
fn scales_author_offset_by_ratio() {
    let k = ScaleRatio::new(120, 96).expect("120/96");
    let (x, y) = scale_author_offset((10, -10), k);
    assert_eq!((x.value, y.value), (13, -13));
}

/// 域を超える換算は回り込まず飽和し、その事実を値として返す（2.5）。
#[test]
fn saturating_axis_is_reported() {
    let k = ScaleRatio::new(192, 96).expect("192/96");
    let (x, y) = scale_author_offset((i32::MAX, 0), k);
    assert_eq!(x.value, i32::MAX);
    assert!(x.saturated);
    assert_eq!(y.value, 0);
    assert!(!y.saturated);
}

// =========================================================================
// 判断分岐の全網羅（task 1.3・要件 7.5〜7.9／design「Unit Tests」1〜5）
// =========================================================================

/// 判定が値を持ち去らなかったこと——`Rescaled` 以外の 3 腕は offset を運ばない。
///
/// 縮退・恒等・係留の各腕で「値が 1 bit も動かない」ことは、**新しい値がどこにも
/// 現れない**ことで成立する——実効的な檻はこの match の側だけである。
///
/// 続く基準対の照合は檻ではない。`OffsetBase` は `Copy` で
/// `rescale_follow_offset` は値渡しゆえ、実装が何をしても呼び手の複製は動かない。
/// 不変を守っているのは型システムであり、この行は読み手向けの再確認に留まる。
fn assert_value_untouched(verdict: OffsetRescale, base: OffsetBase, before: OffsetBase) {
    assert_eq!(base, before, "基準対の不変（型で保証済み・再確認）");
    match verdict {
        OffsetRescale::Rescaled { offset, .. } => {
            panic!("値を変えない腕のはずが Rescaled({offset:?}) を返した")
        }
        OffsetRescale::Anchored { .. }
        | OffsetRescale::Unchanged
        | OffsetRescale::Unresolved { .. } => {}
    }
}

/// 換算結果の 2 軸を値だけの組へ落とす（飽和は別途主張する）。
fn axis_values(pair: (ScaledAxis, ScaledAxis)) -> (i32, i32) {
    (pair.0.value, pair.1.value)
}

// -------------------------------------------------------------------------
// 1. 往復の bit 同一（要件 3.3／7.8・design D4）
// -------------------------------------------------------------------------

/// `96 → 120 → 192 → 120 → 96` を巡っても、同じ表示 DPI へ戻るたびに初回と bit 同一。
///
/// 基準対から毎回引き直すため出力が入力へ戻らず、誤差が連鎖しない（D4）。
/// 基準そのものが呼出をまたいで不変であることも同時に主張する。
#[test]
fn roundtrip_over_dpi_chain_is_bit_identical() {
    let base = base_at(10, -20, 96);
    let before = base;
    let mut seen: Vec<(u16, OffsetRescale)> = Vec::new();
    for step in [96u16, 120, 192, 120, 96] {
        let verdict = rescale_follow_offset(base, dpi(step));
        if let Some((_, first)) = seen.iter().find(|(d, _)| *d == step) {
            assert_eq!(&verdict, first, "同じ表示 DPI {step} へ戻ったのに値が違う");
        } else {
            seen.push((step, verdict));
        }
    }
    // 基準対の不変は `Copy` 渡しで型が保証する（再確認であって檻ではない）。
    assert_eq!(base, before, "基準対の不変（型で保証済み・再確認）");
    // 逐語（近似ではない）。96 は恒等ゆえ Unchanged、他は基準からの引き直し。
    assert_eq!(
        seen,
        vec![
            (96u16, OffsetRescale::Unchanged),
            (
                120u16,
                OffsetRescale::Rescaled {
                    offset: PointPx { x: 13, y: -25 },
                    saturated: false,
                }
            ),
            (
                192u16,
                OffsetRescale::Rescaled {
                    offset: PointPx { x: 20, y: -40 },
                    saturated: false,
                }
            ),
        ]
    );
}

/// 96 の倍数でない DPI を挟んだ長い列でも往復は bit 同一で戻る（7.6／7.8）。
#[test]
fn roundtrip_over_non_multiple_dpi_chain_is_bit_identical() {
    let base = base_at(37, -83, 96);
    let first_192 = rescale_follow_offset(base, dpi(192));
    let walked: Vec<OffsetRescale> = [96u16, 120, 192, 168, 144, 96]
        .into_iter()
        .map(|step| rescale_follow_offset(base, dpi(step)))
        .collect();
    assert_eq!(
        walked.first(),
        walked.last(),
        "起点 DPI へ戻ったのに値が違う"
    );
    assert_eq!(
        walked.last(),
        Some(&OffsetRescale::Unchanged),
        "起点 DPI へ戻れば恒等"
    );
    assert_eq!(rescale_follow_offset(base, dpi(192)), first_192);
    assert_eq!(
        first_192,
        OffsetRescale::Rescaled {
            offset: PointPx { x: 74, y: -166 },
            saturated: false,
        }
    );
    assert_eq!(
        rescale_follow_offset(base, dpi(168)),
        OffsetRescale::Rescaled {
            offset: PointPx { x: 65, y: -145 },
            saturated: false,
        }
    );
    assert_eq!(
        rescale_follow_offset(base, dpi(144)),
        OffsetRescale::Rescaled {
            offset: PointPx { x: 56, y: -125 },
            saturated: false,
        }
    );
}

// -------------------------------------------------------------------------
// 2. 恒等の素通し（要件 2.2／7.5）
// -------------------------------------------------------------------------

/// 基準 DPI と現在 DPI が同一なら、どんな値でも 1 bit も動かない。
///
/// `i32::MIN`（窓寸の未確定センチネルと同じビット列）まで含めて素通しであることを
/// 主張する——遷移経路は恒等を**比を組む前に**返すため、飽和の腕へ落ちない。
#[test]
fn identity_dpi_never_moves_any_value() {
    for v in [0, 1, -1, i32::MAX, i32::MIN, 12_345, -12_345] {
        for d in [96u16, 120, 144, 168, 192, 300] {
            let base = OffsetBase {
                offset: PointPx {
                    x: v,
                    y: v.wrapping_neg(),
                },
                dpi: Some(dpi(d)),
            };
            let before = base;
            let verdict = rescale_follow_offset(base, dpi(d));
            assert_eq!(verdict, OffsetRescale::Unchanged, "v={v} dpi={d}");
            assert_value_untouched(verdict, base, before);
        }
    }
}

/// 軸ごとに DPI が違っても、両軸とも一致していれば恒等（`DPI` は 2 成分の同値比較）。
#[test]
fn identity_holds_for_anisotropic_dpi() {
    let base = OffsetBase {
        offset: PointPx { x: 11, y: -22 },
        dpi: Some(DPI::from_dpi(120, 144)),
    };
    let before = base;
    let verdict = rescale_follow_offset(base, DPI::from_dpi(120, 144));
    assert_eq!(verdict, OffsetRescale::Unchanged);
    assert_value_untouched(verdict, base, before);
}

/// 供給側の恒等比は正準形で判定される——`ScaleRatio::new(120, 120)` も `ONE` と同じ素通し。
#[test]
fn supply_identity_ratio_passes_raw_through_in_canonical_form() {
    let k = ScaleRatio::new(120, 120).expect("120/120");
    assert!(
        k.is_identity(),
        "既約化で 1/1 にならない比は恒等の主張が崩れる"
    );
    for v in [0, 1, -1, 7, -7, i32::MAX, -i32::MAX, 65_535, -65_535] {
        let (x, y) = scale_author_offset((v, -v), k);
        assert_eq!((x.value, y.value), (v, -v), "v={v}");
        assert!(!x.saturated && !y.saturated, "v={v}");
    }
}

/// 供給側の恒等の**唯一の例外**＝`i32::MIN`（正の側に対応する値が i32 に無い）。
///
/// 大きさ `2_147_483_648` は `i32` へ収まらないため `±i32::MAX` へ飽和し、飽和した事実を
/// 値として返す（回り込まない・要件 2.5）。`i32::MIN` は窓寸の未確定センチネル
/// （`CW_USEDEFAULT`）と同じビット列であり、正当なオフセットとしては供給されない。
/// ここで固定するのは「恒等でも黙って回り込まない」ことである。
#[test]
fn supply_identity_saturates_only_at_i32_min() {
    let (x, y) = scale_author_offset((i32::MIN, 0), ScaleRatio::ONE);
    assert_eq!(x.value, -i32::MAX);
    assert!(x.saturated);
    assert_eq!(y.value, 0);
    assert!(!y.saturated);
}

// -------------------------------------------------------------------------
// 3. 96 の倍数でない表示 DPI の行列（要件 7.6・design「Unit Tests」3）
// -------------------------------------------------------------------------

/// `(基準 DPI, 現在 DPI, 基準値, 期待値)` の逐語表。
///
/// 120（5/4）・144（3/2）・168（7/4）を**分子・分母の両側**へ置き、正負の対で並べる。
/// 期待値は `ScaleRatio::scale_len` の規約（round half away from zero・非ゼロ長は最小 1px）
/// から手計算した値であって、実装の出力を書き写したものではない。
const RESCALE_MATRIX: &[(u16, u16, i32, i32)] = &[
    // 96 → 120（5/4）
    (96, 120, 0, 0),
    (96, 120, 1, 1),
    (96, 120, -1, -1),
    (96, 120, 3, 4),
    (96, 120, -3, -4),
    (96, 120, 7, 9),
    (96, 120, -7, -9),
    (96, 120, 10, 13),
    (96, 120, -10, -13),
    (96, 120, 100, 125),
    (96, 120, -100, -125),
    // 120 → 96（4/5）
    (120, 96, 0, 0),
    (120, 96, 1, 1),
    (120, 96, -1, -1),
    (120, 96, 3, 2),
    (120, 96, -3, -2),
    (120, 96, 7, 6),
    (120, 96, -7, -6),
    (120, 96, 10, 8),
    (120, 96, -10, -8),
    (120, 96, 100, 80),
    (120, 96, -100, -80),
    // 96 → 144（3/2）
    (96, 144, 0, 0),
    (96, 144, 1, 2),
    (96, 144, -1, -2),
    (96, 144, 3, 5),
    (96, 144, -3, -5),
    (96, 144, 7, 11),
    (96, 144, -7, -11),
    (96, 144, 10, 15),
    (96, 144, -10, -15),
    (96, 144, 100, 150),
    (96, 144, -100, -150),
    // 144 → 96（2/3）
    (144, 96, 0, 0),
    (144, 96, 1, 1),
    (144, 96, -1, -1),
    (144, 96, 3, 2),
    (144, 96, -3, -2),
    (144, 96, 7, 5),
    (144, 96, -7, -5),
    (144, 96, 10, 7),
    (144, 96, -10, -7),
    (144, 96, 100, 67),
    (144, 96, -100, -67),
    // 120 → 144（6/5）
    (120, 144, 0, 0),
    (120, 144, 1, 1),
    (120, 144, -1, -1),
    (120, 144, 3, 4),
    (120, 144, -3, -4),
    (120, 144, 7, 8),
    (120, 144, -7, -8),
    (120, 144, 10, 12),
    (120, 144, -10, -12),
    (120, 144, 100, 120),
    (120, 144, -100, -120),
    // 144 → 120（5/6）
    (144, 120, 0, 0),
    (144, 120, 1, 1),
    (144, 120, -1, -1),
    (144, 120, 3, 3),
    (144, 120, -3, -3),
    (144, 120, 7, 6),
    (144, 120, -7, -6),
    (144, 120, 10, 8),
    (144, 120, -10, -8),
    (144, 120, 100, 83),
    (144, 120, -100, -83),
    // 120 → 168（7/5）
    (120, 168, 0, 0),
    (120, 168, 1, 1),
    (120, 168, -1, -1),
    (120, 168, 3, 4),
    (120, 168, -3, -4),
    (120, 168, 7, 10),
    (120, 168, -7, -10),
    (120, 168, 10, 14),
    (120, 168, -10, -14),
    (120, 168, 100, 140),
    (120, 168, -100, -140),
    // 168 → 120（5/7）
    (168, 120, 0, 0),
    (168, 120, 1, 1),
    (168, 120, -1, -1),
    (168, 120, 3, 2),
    (168, 120, -3, -2),
    (168, 120, 7, 5),
    (168, 120, -7, -5),
    (168, 120, 10, 7),
    (168, 120, -10, -7),
    (168, 120, 100, 71),
    (168, 120, -100, -71),
    // 144 → 168（7/6）
    (144, 168, 0, 0),
    (144, 168, 1, 1),
    (144, 168, -1, -1),
    (144, 168, 3, 4),
    (144, 168, -3, -4),
    (144, 168, 7, 8),
    (144, 168, -7, -8),
    (144, 168, 10, 12),
    (144, 168, -10, -12),
    (144, 168, 100, 117),
    (144, 168, -100, -117),
    // 168 → 144（6/7）
    (168, 144, 0, 0),
    (168, 144, 1, 1),
    (168, 144, -1, -1),
    (168, 144, 3, 3),
    (168, 144, -3, -3),
    (168, 144, 7, 6),
    (168, 144, -7, -6),
    (168, 144, 10, 9),
    (168, 144, -10, -9),
    (168, 144, 100, 86),
    (168, 144, -100, -86),
    // 96 → 192（2/1）
    (96, 192, 0, 0),
    (96, 192, 1, 2),
    (96, 192, -1, -2),
    (96, 192, 3, 6),
    (96, 192, -3, -6),
    (96, 192, 7, 14),
    (96, 192, -7, -14),
    (96, 192, 10, 20),
    (96, 192, -10, -20),
    (96, 192, 100, 200),
    (96, 192, -100, -200),
    // 192 → 96（1/2）
    (192, 96, 0, 0),
    (192, 96, 1, 1),
    (192, 96, -1, -1),
    (192, 96, 3, 2),
    (192, 96, -3, -2),
    (192, 96, 7, 4),
    (192, 96, -7, -4),
    (192, 96, 10, 5),
    (192, 96, -10, -5),
    (192, 96, 100, 50),
    (192, 96, -100, -50),
];

/// 行列の全行を逐語で固定する（丸めが変われば必ず赤くなる）。
#[test]
fn rescale_matrix_pins_rounding_verbatim() {
    for &(base_dpi, current, v, expected) in RESCALE_MATRIX {
        let base = base_at(v, -v, base_dpi);
        assert_eq!(
            rescale_follow_offset(base, dpi(current)),
            OffsetRescale::Rescaled {
                offset: PointPx {
                    x: expected,
                    y: -expected,
                },
                saturated: false,
            },
            "{base_dpi} → {current}, v={v}"
        );
    }
}

/// 表の中に、切り捨てと**食い違う**行が実在すること（丸めの主張が空回りしない）。
///
/// round half away from zero は 0 から遠い側へしか倒れないため、正値では
/// 「切り捨てより大きい」行だけが現れる。その行数を逐語で固定する。
#[test]
fn rescale_matrix_actually_exercises_rounding_away_from_zero() {
    let mut away = 0usize;
    let mut toward = 0usize;
    for &(base_dpi, current, v, expected) in RESCALE_MATRIX {
        if v <= 0 {
            continue;
        }
        let truncated = (v as i64 * current as i64) / base_dpi as i64;
        match (expected as i64).cmp(&truncated) {
            std::cmp::Ordering::Greater => away += 1,
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Less => toward += 1,
        }
    }
    assert_eq!(toward, 0, "0 方向へ倒れた行がある＝丸め規約が変わった");
    assert_eq!(
        away, 28,
        "0 から遠い側へ倒れた行数（行列が痩せると主張も痩せる）"
    );
}

/// 軸ごとに異なる表示 DPI 遷移が、軸ごとに独立して換算されること。
#[test]
fn anisotropic_dpi_scales_each_axis_independently() {
    let base = OffsetBase {
        offset: PointPx { x: 100, y: -100 },
        dpi: Some(DPI::from_dpi(120, 144)),
    };
    assert_eq!(
        rescale_follow_offset(base, DPI::from_dpi(168, 96)),
        OffsetRescale::Rescaled {
            // x: 100 × 168/120 = 140（厳密）／y: −100 × 96/144 = −66.67 → −67
            offset: PointPx { x: 140, y: -67 },
            saturated: false,
        }
    );
}

/// 縮小が 0.5px 未満になっても値は消えない——非ゼロ長は最小 1px（`scale_len` の規約）。
///
/// 300 → 96（8/25）では `1 × 0.32 = 0.32`・`2 × 0.32 = 0.64` がいずれも 1 へ持ち上がる。
/// 符号は保存される。
#[test]
fn shrinking_below_half_pixel_keeps_minimum_one_px() {
    for (v, expected) in [(1, 1), (-1, -1), (2, 1), (-2, -1), (5, 2), (-5, -2)] {
        assert_eq!(
            rescale_follow_offset(base_at(v, -v, 300), dpi(96)),
            OffsetRescale::Rescaled {
                offset: PointPx {
                    x: expected,
                    y: -expected,
                },
                saturated: false,
            },
            "v={v}"
        );
    }
}

/// 下限近傍と飽和の腕——**負の側**も含めて回り込まず `±i32::MAX` で止まる（要件 2.5）。
#[test]
fn saturates_on_both_signs_without_wrapping() {
    for (base_dpi, current) in [(96u16, 192u16), (96, 120), (96, 144)] {
        for v in [i32::MAX, -i32::MAX, i32::MIN, i32::MAX - 1] {
            let verdict = rescale_follow_offset(base_at(v, 0, base_dpi), dpi(current));
            let OffsetRescale::Rescaled { offset, saturated } = verdict else {
                panic!("{base_dpi} → {current}, v={v}: 追随の腕に入らなかった: {verdict:?}");
            };
            assert!(saturated, "{base_dpi} → {current}, v={v}");
            assert_eq!(
                offset.x,
                if v < 0 { -i32::MAX } else { i32::MAX },
                "{base_dpi} → {current}, v={v}"
            );
            assert_eq!(offset.y, 0, "他軸は巻き込まれない");
        }
    }
}

/// `i32::MIN` の縮小は飽和せずに収まる——`|i32::MIN|` は i32 に無いが、半分にすれば入る。
#[test]
fn i32_min_shrinks_into_range_without_saturating() {
    assert_eq!(
        rescale_follow_offset(base_at(i32::MIN, i32::MAX, 192), dpi(96)),
        OffsetRescale::Rescaled {
            // |i32::MIN| = 2_147_483_648 → 半分 1_073_741_824（i32 域内）
            offset: PointPx {
                x: -1_073_741_824,
                y: 1_073_741_824,
            },
            saturated: false,
        }
    );
}

/// 飽和は 1 軸で起きても判定結果には**まとめて**現れる（要件 2.5 と同型）。
#[test]
fn saturation_flag_is_the_or_of_both_axes() {
    let verdict = rescale_follow_offset(base_at(i32::MAX, 10, 96), dpi(192));
    assert_eq!(
        verdict,
        OffsetRescale::Rescaled {
            offset: PointPx { x: i32::MAX, y: 20 },
            saturated: true,
        }
    );
}

/// 供給側も負の側で飽和する（task 1.2 の檻は正の側だけを踏んでいた）。
#[test]
fn supply_saturates_on_the_negative_side_too() {
    let k = ScaleRatio::new(192, 96).expect("192/96");
    let (x, y) = scale_author_offset((-i32::MAX, i32::MIN), k);
    assert_eq!((x.value, y.value), (-i32::MAX, -i32::MAX));
    assert!(x.saturated && y.saturated);
}

/// 供給側も 96 の倍数でない比で丸めどおりに換算する（7.6）。
#[test]
fn supply_scales_by_non_multiple_ratios_verbatim() {
    for (num, den, raw, expected) in [
        (120u32, 96u32, 10i32, 13i32),
        (96, 120, 10, 8),
        (168, 144, 100, 117),
        (144, 168, 100, 86),
        (168, 120, 7, 10),
        (120, 168, 7, 5),
    ] {
        let k = ScaleRatio::new(num, den).expect("非ゼロ比");
        assert_eq!(
            axis_values(scale_author_offset((raw, -raw), k)),
            (expected, -expected),
            "{num}/{den}, raw={raw}"
        );
    }
}

// -------------------------------------------------------------------------
// 4. 作者基準 DPI の食い違い（要件 7.7／4.4・design D5）
// -------------------------------------------------------------------------

/// シェルの作者基準 DPI（テスト内の模型）。
const SHELL_AUTHOR_DPI: u32 = 96;
/// バルーンの作者基準 DPI（同上・シェルとわざと違える）。
const BALLOON_AUTHOR_DPI: u32 = 120;

/// 表示 DPI `d` における軸ごとの拡大率（`k_axis(d) = d ÷ author_dpi_axis`・app_scale=1）。
fn k_axis(display_dpi: u32, author_dpi: u32) -> ScaleRatio {
    ScaleRatio::new(display_dpi, author_dpi).expect("非ゼロ DPI")
}

/// 供給時は軸ごとに換算され、遷移の追随では作者基準 DPI が約分で消える（D5）。
///
/// 主張は 2 段。⑴ 同じ生値でもシェル軸とバルーン軸では換算結果が違う（軸の選択は
/// 供給時にだけ意味を持つ）。⑵ 96 で供給した値を 192 へ**追随**させた結果は、192 で
/// **供給し直した**値と一致する——シェル軸でもバルーン軸でも。ゆえに遷移の追随に
/// 軸の選択は生じない（要件 4.4 の「どちらを用いるか」への答え）。
#[test]
fn author_dpi_axes_split_on_supply_but_cancel_on_transition() {
    let raw = (40, -40);

    // ⑴ 供給: 軸で結果が割れる。
    let shell_at_96 = axis_values(scale_author_offset(raw, k_axis(96, SHELL_AUTHOR_DPI)));
    let balloon_at_96 = axis_values(scale_author_offset(raw, k_axis(96, BALLOON_AUTHOR_DPI)));
    assert_eq!(shell_at_96, (40, -40), "シェル軸 96/96 は恒等");
    assert_eq!(balloon_at_96, (32, -32), "バルーン軸 96/120 は 4/5");
    assert_ne!(
        shell_at_96, balloon_at_96,
        "軸が割れていない模型では 7.7 を踏めない"
    );

    let shell_at_192 = axis_values(scale_author_offset(raw, k_axis(192, SHELL_AUTHOR_DPI)));
    let balloon_at_192 = axis_values(scale_author_offset(raw, k_axis(192, BALLOON_AUTHOR_DPI)));
    assert_eq!(shell_at_192, (80, -80), "シェル軸 192/96 は 2/1");
    assert_eq!(balloon_at_192, (64, -64), "バルーン軸 192/120 は 8/5");

    // ⑵ 遷移: 96 で供給した値を 192 へ追随させると、192 で供給し直した値に一致する。
    for (label, supplied_at_96, supplied_at_192) in [
        ("shell", shell_at_96, shell_at_192),
        ("balloon", balloon_at_96, balloon_at_192),
    ] {
        assert_eq!(
            rescale_follow_offset(base_at(supplied_at_96.0, supplied_at_96.1, 96), dpi(192)),
            OffsetRescale::Rescaled {
                offset: PointPx {
                    x: supplied_at_192.0,
                    y: supplied_at_192.1,
                },
                saturated: false,
            },
            "{label}: 追随の結果が供給し直しと食い違う＝作者基準 DPI が約分で消えていない"
        );
    }
}

/// 遷移の判定は作者基準 DPI をそもそも入力に取らない——同じ基準対なら軸に依らず同一。
///
/// 上のテストが「一致する」ことを値で示すのに対し、こちらは 2 軸の拡大率が実際に
/// 異なっていること（模型が空回りしていないこと）を先に確かめたうえで、判定が
/// 1 つしか無いことを示す。
#[test]
fn transition_verdict_is_axis_independent() {
    let base = base_at(32, -32, 96);
    let shell_ratio = k_axis(192, SHELL_AUTHOR_DPI);
    let balloon_ratio = k_axis(192, BALLOON_AUTHOR_DPI);
    assert_ne!(
        shell_ratio, balloon_ratio,
        "2 軸が同じ比では主張が空回りする"
    );
    assert_eq!(
        rescale_follow_offset(base, dpi(192)),
        OffsetRescale::Rescaled {
            offset: PointPx { x: 64, y: -64 },
            saturated: false,
        }
    );
}

// -------------------------------------------------------------------------
// 5. 判定 4 腕と、値を変えない縮退 3 経路の全到達（要件 3.6／9.4）
// -------------------------------------------------------------------------

/// 縮退 3 経路（`ZeroBaseDpi`／`ZeroCurrentDpi`／未係留）はいずれも値を変えない。
#[test]
fn all_three_degenerate_paths_leave_the_value_untouched() {
    let offset = PointPx { x: 10, y: -20 };

    // ⑴ 未係留（永続値の腕・5.2）。
    let unpinned = OffsetBase { offset, dpi: None };
    let verdict = rescale_follow_offset(unpinned, dpi(192));
    assert_eq!(verdict, OffsetRescale::Anchored { base_dpi: dpi(192) });
    assert_value_untouched(verdict, unpinned, OffsetBase { offset, dpi: None });

    // ⑵ 基準 DPI が 0（軸ごとに独立して縮退する）。
    for zero_base in [
        DPI::from_dpi(0, 0),
        DPI::from_dpi(0, 96),
        DPI::from_dpi(96, 0),
    ] {
        let base = OffsetBase {
            offset,
            dpi: Some(zero_base),
        };
        let before = base;
        let verdict = rescale_follow_offset(base, dpi(192));
        assert_eq!(
            verdict,
            OffsetRescale::Unresolved {
                reason: UnresolvedScale::ZeroBaseDpi
            },
            "base_dpi={zero_base:?}"
        );
        assert_value_untouched(verdict, base, before);
    }

    // ⑶ 現在 DPI が 0。
    for zero_current in [
        DPI::from_dpi(0, 0),
        DPI::from_dpi(0, 96),
        DPI::from_dpi(96, 0),
    ] {
        let base = base_at(offset.x, offset.y, 192);
        let before = base;
        let verdict = rescale_follow_offset(base, zero_current);
        assert_eq!(
            verdict,
            OffsetRescale::Unresolved {
                reason: UnresolvedScale::ZeroCurrentDpi
            },
            "current={zero_current:?}"
        );
        assert_value_untouched(verdict, base, before);
    }
}

/// 未係留は「現在 DPI をそのまま」係留する——軸ごとに違う DPI でも作り替えない。
#[test]
fn anchoring_adopts_the_observed_dpi_verbatim() {
    let base = OffsetBase {
        offset: PointPx { x: 10, y: -20 },
        dpi: None,
    };
    let current = DPI::from_dpi(120, 144);
    assert_eq!(
        rescale_follow_offset(base, current),
        OffsetRescale::Anchored { base_dpi: current }
    );
}

/// 未係留は現在 DPI が 0 でもそのまま係留する（係留は恒等・縮退より**先**に判定される）。
///
/// 永続値の腕は比を組まないので、比を解決できるかどうかを問う前に答えが出る。
/// 値は動かないため要件 5.2／3.6 の規範はいずれも満たされる。
#[test]
fn unpinned_base_anchors_before_any_zero_check() {
    let base = OffsetBase {
        offset: PointPx { x: 10, y: -20 },
        dpi: None,
    };
    let before = base;
    let verdict = rescale_follow_offset(base, DPI::from_dpi(0, 0));
    assert_eq!(
        verdict,
        OffsetRescale::Anchored {
            base_dpi: DPI::from_dpi(0, 0)
        }
    );
    assert_value_untouched(verdict, base, before);
}

/// **腕の順序の確定**: 基準 DPI と現在 DPI がともに 0 なら `Unchanged`（`Unresolved` ではない）。
///
/// 恒等の判定が 0 検査より先に置かれているためである。この順序は意図であって事故ではない。
///
/// ⑴ 値はどちらの腕でも 1 bit も動かない——要件 3.6 の規範（「追従オフセットを変更しない」）
///    は満たされる。腕によって変わるのは呼び手が残す判定語だけである。
/// ⑵ `base_dpi == current` は**そもそも遷移が起きていない**ことを意味する。表示 DPI が
///    0 のまま動いていないという事態は、比を組む側ではなく DPI の供給層が報告すべきもので
///    あり、「解決できない比」として遷移側に語を立てると出所を誤らせる。
/// ⑶ 表示 DPI が 0 になる経路は本番に無い（`WM_DPICHANGED` の wparam もモニタ問い合わせも
///    非ゼロを返す）。ゆえに判定語が `unchanged` になることの実害も無い。
///
/// 呼び手（追随相・task 6.x）はこの順序を前提に判定語を出すため、逆転させると記録の
/// 意味が変わる。ここで固定しておく。
#[test]
fn zero_on_both_sides_is_unchanged_not_unresolved() {
    let base = OffsetBase {
        offset: PointPx { x: 10, y: -20 },
        dpi: Some(DPI::from_dpi(0, 0)),
    };
    let before = base;
    let verdict = rescale_follow_offset(base, DPI::from_dpi(0, 0));
    assert_eq!(verdict, OffsetRescale::Unchanged);
    assert_value_untouched(verdict, base, before);
}

/// 基準側と現在側の 0 が**同時に**あるとき、理由は基準側が先に立つ。
#[test]
fn zero_base_dpi_wins_over_zero_current_dpi() {
    let base = OffsetBase {
        offset: PointPx { x: 10, y: -20 },
        dpi: Some(DPI::from_dpi(0, 96)),
    };
    assert_eq!(
        rescale_follow_offset(base, DPI::from_dpi(96, 0)),
        OffsetRescale::Unresolved {
            reason: UnresolvedScale::ZeroBaseDpi
        }
    );
}

/// [`OffsetRescale`] の 4 腕すべてに到達していること（腕の取りこぼしを構造で防ぐ）。
#[test]
fn every_verdict_arm_is_reachable() {
    let offset = PointPx { x: 10, y: -20 };
    let arms = [
        rescale_follow_offset(OffsetBase { offset, dpi: None }, dpi(96)),
        rescale_follow_offset(base_at(offset.x, offset.y, 96), dpi(96)),
        rescale_follow_offset(base_at(offset.x, offset.y, 96), dpi(192)),
        rescale_follow_offset(base_at(offset.x, offset.y, 96), dpi(0)),
    ];
    let mut seen = [false; 4];
    for arm in arms {
        let idx = match arm {
            OffsetRescale::Anchored { .. } => 0,
            OffsetRescale::Unchanged => 1,
            OffsetRescale::Rescaled { .. } => 2,
            OffsetRescale::Unresolved { .. } => 3,
        };
        seen[idx] = true;
    }
    assert_eq!(seen, [true; 4], "到達していない判定の腕がある");
}
