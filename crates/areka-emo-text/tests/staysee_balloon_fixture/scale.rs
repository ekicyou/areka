//! 表示スケールの扱いを固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.7**・設計 **C2** の I 行）。
//!
//! ## ここで固定するもの
//!
//! 検体の面 0（335×205）の描画範囲について、物理寸法への変換が、描画範囲の幅（遠辺 − 左辺）と
//! 左辺のそれぞれで k 倍になることを手計算の絶対値で固定する。
//!
//! ## k ごとの配置の比べはここで行わない
//!
//! テストが自分で枠の配置を作って画像の寸法を解決へ渡すと、k は入力に入らず、同じ純関数を
//! 同じ値で呼び直すだけになる（本番が物理寸法を渡す形に変わっても緑のまま）。本番が画像の
//! 寸法を渡す向きは `tests/attach_wiring_test.rs` が実の presenter で守り、画像の寸法から解いた
//! 領域の値は [`super::region`] が固定する。
//!
//! ## 決定論
//!
//! ファイル読み込みと純粋層の解決のみ。実 GPU・実窓・DirectWrite を要さず、
//! 同一入力に対して常に同一の結果を返す。

use areka_emo_text::region::{ImagePx, ScaleContract};

use super::test_support::{EXPECTED_LEFT, EXPECTED_RIGHT};

/// 描画範囲の幅（遠辺 309 − 左辺 22 ＝ 287・image px）。
///
/// [`super::region`] が固定している 4 辺から導く（値を書き写すと、片方だけ直したときに
/// 同一性が黙って壊れる）。
const EXPECTED_INLINE_EXTENT: f32 = EXPECTED_RIGHT - EXPECTED_LEFT;

// ── 物理寸法への変換（設計 C2 の I 行）──────────────────────────────────────────

/// 物理寸法への変換が、描画範囲の幅と左辺のそれぞれで k 倍になること。
///
/// 期待値は手計算の絶対値である。幅 287 の側は `ceil(287k)` なので k=1.25 で
/// 358.75 → **359**（切り上げ。同じ端数を切り捨てれば 358・四捨五入すれば 359）、左辺 22 の側は切り上げを
/// 挟まない `22k` なので k=1.25 で **27.5**——両者で丸めの有無が違うことまで込みで固定する。
#[test]
fn physical_conversion_scales_with_the_display_scale() {
    // (k, ceil(287k), 22k)。いずれも f32 で正確に表現できる値。
    let expected: [(f32, u32, f32); 3] = [
        (1.0, 287, 22.0),
        (1.25, 359, 27.5), // 287×1.25=358.75→ceil 359・22×1.25=27.5
        (2.0, 574, 44.0),
    ];

    for (k, want_extent, want_left) in expected {
        let contract = ScaleContract::new(k, None);

        let extent = contract.physical_extent(ImagePx(EXPECTED_INLINE_EXTENT));
        assert_eq!(
            extent, want_extent,
            "k={k}: 描画範囲の幅 {EXPECTED_INLINE_EXTENT} の物理寸法が {want_extent} ではなく \
             {extent}（`ceil(幅 × k)`）"
        );
        let left = contract.to_physical(ImagePx(EXPECTED_LEFT)).0;
        assert_eq!(
            left, want_left,
            "k={k}: 描画範囲の左辺 {EXPECTED_LEFT} の物理座標が {want_left} ではなく {left}\
             （`左辺 × k`）"
        );
    }
}
