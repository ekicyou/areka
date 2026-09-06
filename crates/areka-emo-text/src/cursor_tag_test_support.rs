//! 解決層 `cursor_tag` のテストが共有する前提（design.md「Testing Strategy ／ Unit Tests」の
//! 共通前提の**唯一の住処**）。
//!
//! テーマ別に分かれた 2 つのテストモジュール（`cursor_tag_tests.rs`＝記録の 2 口／
//! `cursor_tag_resolve_tests.rs`＝解決表の全網羅）が同じ値を見るために集約している。
//! 複製すると「共通前提」を名乗る値が黙って食い違う。
//!
//! 共通前提は design.md の逐語:
//! `font_height = 10`・`line_pitch = 12`・`image_size = (400, 224)`・
//! `origin`＝宣言例 `(50, 20)`・`current = (200, 30)`。
//!
//! 行送りだけは `areka-P0-emo-text-line-height-canon` の裁定で 13 から 12 へ改まった
//! （`1lh` は 1em＋行間で、行間の既定が 2 になった）。カーソルタグ側の座標語彙・原点・
//! 書字方向ごとの解決規則は変わらず、`1lh` が指す値だけが追随する。

use super::CursorBasis;
use crate::region::TextRegion;
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

/// design.md「Unit Tests」共通前提の文字高さ（正典 `1em`＝タグ時点の文字高さ）。
pub(super) const FONT_HEIGHT: f32 = 10.0;
/// 同・行送り（正典 `1lh`＝1em＋行間。`10 + 2 = 12`）。
pub(super) const LINE_PITCH: f32 = 12.0;
/// 同・バルーン画像原寸（`centerx`／`centery` の基準）。
pub(super) const IMAGE_SIZE: (f32, f32) = (400.0, 224.0);
/// 同・宣言された文字描画開始点（絶対座標の基点）。
pub(super) const ORIGIN: (f32, f32) = (50.0, 20.0);
/// 同・現在の文字描画位置（`@` 相対の基点）。
pub(super) const CURRENT: (f32, f32) = (200.0, 30.0);

/// design.md「Unit Tests」共通前提そのままの基点束。
pub(super) fn basis() -> CursorBasis {
    CursorBasis {
        origin: ORIGIN,
        current: CURRENT,
        image_size: IMAGE_SIZE,
        font_height: FONT_HEIGHT,
        line_pitch: LINE_PITCH,
    }
}

/// 3 つの基点が**両軸とも相異なる**基点束（弁別用）。
///
/// 共通前提の画像幅 400 は半分が 200 で `current.0`（＝200）と同値になり、X 軸では
/// 「画像中央」と「現在の文字描画位置」を取り違えた実装が素通りしてしまう。画像原寸だけを
/// `(360, 180)` に差し替えると 3 基点は X で `50 / 200 / 180`、Y で `20 / 30 / 90` となり、
/// どれを取り違えても値が変わる。
///
/// 係数 4 種（`1` / `font_height = 10` / `line_pitch = 13` / `font_height / 100 = 0.1`）も
/// 互いに異なるので、基点・係数のどちらを取り違えても檻のどれかが赤になる。
pub(super) const DISCRIMINATING_IMAGE_SIZE: (f32, f32) = (360.0, 180.0);

/// [`DISCRIMINATING_IMAGE_SIZE`] を使う基点束（他の成分は [`basis`] と同一）。
pub(super) fn discriminating_basis() -> CursorBasis {
    CursorBasis {
        image_size: DISCRIMINATING_IMAGE_SIZE,
        ..basis()
    }
}

/// 範囲外記録の檻で使う文字描画範囲（validrect）の 4 辺。
///
/// 画像全域（`0..400 × 0..224`）の**部分矩形**にしてある——全域にすると「validrect の辺」と
/// 「バルーン画像の辺」を取り違えた実装が素通りしてしまう（`image_size` は `(400, 224)`）。
pub(super) const VALID_LEFT: f32 = 40.0;
/// 同上（validrect の上辺）。
pub(super) const VALID_TOP: f32 = 20.0;
/// 同上（validrect の右辺）。
pub(super) const VALID_RIGHT: f32 = 360.0;
/// 同上（validrect の下辺）。
pub(super) const VALID_BOTTOM: f32 = 200.0;

/// 上の 4 辺を持つ `TextRegion`。
///
/// 書字方向は `note_out_of_range` の判定に**関与しない**——当該関数が読むのは validrect の
/// 4 辺だけで、書字方向が効くのは `start()`／`wrap_threshold()` の側である。ここで
/// `HorizontalTb` を渡すのは `TextRegion::resolve` の引数を埋めるためにすぎない。
///
/// **捕捉窓の外で組むこと**——`TextRegion::resolve` は未宣言の `origin` 成分について
/// `debug!` を出すので、窓の中で組むと範囲外記録の件数に混ざる。
pub(super) fn out_of_range_region() -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(
            Some(VALID_TOP as i32),
            Some(VALID_BOTTOM as i32),
            Some(VALID_LEFT as i32),
            Some(VALID_RIGHT as i32),
        ),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    let region = TextRegion::resolve(
        &model,
        (IMAGE_SIZE.0 as u32, IMAGE_SIZE.1 as u32),
        WritingMode::HorizontalTb,
    );
    // 檻の前提を檻にする（fixture が意図した矩形になっていることの確認）。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (VALID_LEFT, VALID_TOP, VALID_RIGHT, VALID_BOTTOM)
    );
    region
}
