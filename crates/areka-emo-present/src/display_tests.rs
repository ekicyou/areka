//! 記録レシピ（T-N1）と丸めの一致（T-N3）を固定する GPU 非依存の檻。
//!
//! T-N1 が固定する性質は 4 点のみ：bitmap 寸＝合成結果の原寸・pitch＝合成結果の実 stride
//! （w*4 から再算出しない）・宛先矩形＝`(0, 0, w, h)`・補間＝`D2D1_INTERPOLATION_MODE_LINEAR`。
//! 拡大率 k を引数に取らないことは `DisplayRecipe::for_surface` の署名そのものが保証する
//! （コンパイル時の性質であり、実行時テストでは検証しない）。
//!
//! T-N3 は「利用者から見える物理寸＝[`ScaleRatio::scaled_extent`]」と「wintf の描画面寸＝
//! `GlobalArrangement.bounds` の `ceil`」の関係を表で固定する（設計 §丸めの一致）。

use super::*;

use areka_emo_compose::ScaleRatio;
use wintf::ecs::{GlobalArrangement, Rect, calculate_surface_size_from_global_arrangement};

use crate::mount::logical_arrangement;

/// 複数寸で「bitmap 寸＝native・pitch＝実 stride・dest＝(0,0,w,h)・補間＝LINEAR」を固定する。
#[test]
fn recipe_uses_native_size_stride_full_dest_and_linear() {
    for (w, h) in [(1u32, 1u32), (8, 6), (382, 547)] {
        let surface = ComposedSurface::new(w, h);
        let recipe = DisplayRecipe::for_surface(&surface);

        assert_eq!(
            recipe.bitmap_size,
            (w, h),
            "bitmap 寸は native 原寸と一致するべき"
        );
        assert_eq!(
            recipe.pitch,
            surface.stride(),
            "pitch は合成結果の実 stride と一致するべき（w*4 の再算出ではない）"
        );
        assert_eq!(
            recipe.dest,
            (0.0, 0.0, w as f32, h as f32),
            "宛先矩形は (0, 0, w, h) の論理 px であるべき"
        );
        assert_eq!(
            recipe.interpolation, D2D1_INTERPOLATION_MODE_LINEAR,
            "補間は LINEAR 固定であるべき"
        );
    }
}

/// **T-N3**: 本番と同じ式で導いた境界幅が原寸×拡大率と一致し、上流（wintf）の面寸算出と
/// 丸め権威（[`ScaleRatio::scaled_extent`]）の差が 0 か +1 に収まり、整数の拡大率では 0 になること。
///
/// 式は本番どおり「窓の `GlobalArrangement`（scale 1.0・原点は窓のスクリーン位置）×
/// [`logical_arrangement`]」（`impl Mul<Arrangement> for GlobalArrangement`）。そこから
/// `calculate_surface_size_from_global_arrangement`（`ceil`）が wintf の面寸を、
/// `scaled_extent`（round half away from zero）が利用者から見える物理寸を出す。
///
/// `ceil ≥ round` ゆえ差は負にならず、小数部が (0, 0.5) のときだけ wintf が 1 px 大きい。
/// その 1 px は窓 client（`scaled_extent` 寸）の外にあり見えない（要件 2.1・4.3 の許容）。
///
/// 表の 4 行目までは設計 §丸めの一致の表そのもの。5 行目以降は端数 0.5 ちょうど（両者とも
/// 切り上げ＝差 0）と、小数部 (0, 0.5) の追加例（差 +1）を足した較正である。
#[test]
fn surface_size_matches_scaled_extent_within_one_pixel() {
    // (原寸 w, 原寸 h, k の分子, k の分母, 期待する差 (w, h))
    const TABLE: &[(u32, u32, u32, u32, (u32, u32))] = &[
        (382, 547, 2, 1, (0, 0)),
        (382, 547, 5, 4, (0, 0)),
        (5, 5, 5, 4, (1, 1)),
        (31, 31, 7, 6, (1, 1)),
        (382, 547, 1, 1, (0, 0)),
        (7, 7, 3, 2, (0, 0)),
        (10, 10, 4, 3, (1, 1)),
    ];
    // 窓のスクリーン原点。bounds は screen 座標ゆえ、面寸は「大きな値どうしの引き算」で出る
    // （f32 の桁落ちを含めて本番と同じ経路を通す）。原点 0 の行では厳密一致も見る。
    const ORIGINS: &[(f32, f32)] = &[(0.0, 0.0), (1024.0, 768.0)];

    for &(w, h, num, den, expected_diff) in TABLE {
        let k = ScaleRatio::new(num, den).expect("k は非ゼロ");
        let scaled = k.scaled_extent(w, h);

        for &(ox, oy) in ORIGINS {
            let window_global = GlobalArrangement {
                bounds: Rect {
                    left: ox,
                    top: oy,
                    right: ox,
                    bottom: oy,
                },
                ..GlobalArrangement::default()
            };
            let surface_global = window_global * logical_arrangement((w, h), k);
            let (bw, bh) = surface_global.size();

            // 境界幅＝原寸 × 拡大率（変換行列の係数）。
            let (want_w, want_h) = (w as f32 * k.as_f32(), h as f32 * k.as_f32());
            if ox == 0.0 && oy == 0.0 {
                assert_eq!(
                    (bw, bh),
                    (want_w, want_h),
                    "原点 0 の窓では境界幅は原寸×拡大率そのもの: {w}x{h} k={num}/{den}"
                );
            } else {
                assert!(
                    (bw - want_w).abs() < 1e-3 && (bh - want_h).abs() < 1e-3,
                    "窓を平行移動しても境界幅は原寸×拡大率のまま: {w}x{h} k={num}/{den} \
                     bounds=({bw},{bh}) want=({want_w},{want_h})"
                );
            }

            let wintf_size = calculate_surface_size_from_global_arrangement(&surface_global)
                .expect("非ゼロ寸なら面寸は決まる");

            // wintf（ceil）は scaled_extent（round）以上。
            assert!(
                wintf_size.0 >= scaled.0 && wintf_size.1 >= scaled.1,
                "ceil >= round ゆえ wintf の面寸が小さくなることはない: {w}x{h} k={num}/{den} \
                 wintf={wintf_size:?} scaled_extent={scaled:?}"
            );
            let diff = (wintf_size.0 - scaled.0, wintf_size.1 - scaled.1);
            assert!(
                diff.0 <= 1 && diff.1 <= 1,
                "差は 1 px 以内: {w}x{h} k={num}/{den} diff={diff:?}"
            );
            assert_eq!(
                diff, expected_diff,
                "丸めの一致表と一致すべき: {w}x{h} k={num}/{den} \
                 wintf={wintf_size:?} scaled_extent={scaled:?}"
            );
            if den == 1 {
                assert_eq!(diff, (0, 0), "整数の拡大率では差 0: {w}x{h} k={num}/{den}");
            }
        }
    }
}
