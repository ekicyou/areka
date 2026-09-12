//! `DisplayRecipe::for_surface` の記録レシピを固定する GPU 非依存の檻（T-N1）。
//!
//! 固定する性質は 4 点のみ：bitmap 寸＝合成結果の原寸・pitch＝合成結果の実 stride（w*4 から
//! 再算出しない）・宛先矩形＝`(0, 0, w, h)`・補間＝`D2D1_INTERPOLATION_MODE_LINEAR`。
//! 拡大率 k を引数に取らないことは `DisplayRecipe::for_surface` の署名そのものが保証する
//! （コンパイル時の性質であり、実行時テストでは検証しない）。

use super::*;

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
