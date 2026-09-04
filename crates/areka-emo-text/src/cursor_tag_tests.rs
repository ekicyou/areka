//! 解決層 `cursor_tag` の決定論テスト（`areka-P0-cursor-tag-canon`）。
//!
//! design.md「Testing Strategy ／ Unit Tests（`cursor_tag_tests.rs`・純関数）」の住処である。
//! タスク 3.1 が置くのは**解決表の各行を 1 本ずつ通す最小限**であり、全網羅（両軸 × 全書式 ×
//! 境界値 × 縮退 × ログ件数）はタスク 3.3 が本ファイルへ追加する。
//!
//! 共通前提は design.md「Unit Tests」の逐語:
//! `font_height = 10`・`line_pitch = 13`・`image_size = (400, 224)`・
//! `origin`＝宣言例 `(50, 20)`・`current = (200, 30)`。
//!
//! **期待値は正典（design.md 解決表）の式から書く**——実装が返した値を書き写さない。基点 3 種
//! （`origin`・`current`・画像原寸の半分）と係数 4 種（1・`font_height`・`line_pitch`・
//! `font_height / 100`）はいずれも互いに異なる値になるよう選んであるので、基点や係数を
//! 取り違えた実装はどれか 1 本で必ず赤になる。

use super::{CursorAxis, CursorBasis, CursorDegrade, resolve_cursor_axis, unit_coefficient};
use crate::state::{CursorCoord, CursorUnit};

/// design.md「Unit Tests」共通前提の文字高さ（正典 `1em`＝タグ時点の文字高さ）。
const FONT_HEIGHT: f32 = 10.0;
/// 同・行送り（正典 `1lh`＝1em＋行間。`ceil(10 × 1.25) = 13`）。
const LINE_PITCH: f32 = 13.0;
/// 同・バルーン画像原寸（`centerx`／`centery` の基準）。
const IMAGE_SIZE: (f32, f32) = (400.0, 224.0);
/// 同・宣言された文字描画開始点（絶対座標の基点）。
const ORIGIN: (f32, f32) = (50.0, 20.0);
/// 同・現在の文字描画位置（`@` 相対の基点）。
const CURRENT: (f32, f32) = (200.0, 30.0);

fn basis() -> CursorBasis {
    CursorBasis {
        origin: ORIGIN,
        current: CURRENT,
        image_size: IMAGE_SIZE,
        font_height: FONT_HEIGHT,
        line_pitch: LINE_PITCH,
    }
}

/// 解決表「`""`（省略）→ `Ok(None)`＝動かさない・無音」（R1.6/5.5）。両軸とも同じ。
#[test]
fn omitted_axis_does_not_move() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Omitted, CursorAxis::X, &basis()),
        Ok(None)
    );
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Omitted, CursorAxis::Y, &basis()),
        Ok(None)
    );
}

/// 解決表「`N`（数値・負値・小数）→ `origin[axis] + N × coef`」（R2.1/1.3/2.3）。
///
/// 基点が `origin` であること（`current` でも 0 でもない）と、負値・小数がそのまま
/// 通る（内側へ寄せない）ことを、単位 Px／Em の 2 系統で檻化する。
#[test]
fn absolute_is_measured_from_the_origin() {
    // 正の小数 Px: origin.x + 12.5 × 1
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 12.5,
                unit: CursorUnit::Px,
            },
            CursorAxis::X,
            &basis()
        ),
        Ok(Some(ORIGIN.0 + 12.5))
    );
    // 負値 Em（クランプせず素通し）: origin.y + (−3) × font_height
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: -3.0,
                unit: CursorUnit::Em,
            },
            CursorAxis::Y,
            &basis()
        ),
        Ok(Some(ORIGIN.1 + -3.0 * FONT_HEIGHT))
    );
}

/// 解決表「`@N`（単位付き可）→ `current[axis] + N × coef`」（R3.1/3.2/3.3）。
///
/// 基点が `current` であること（`origin` ではない）と、`%` の係数が
/// `font_height / 100` であることを檻化する。
#[test]
fn relative_is_measured_from_the_current_position() {
    // 正典の記述例 `@-1lh`（「1 列ぶん左の列の先頭へ」）: current.x + (−1) × line_pitch。
    // 値を束縛で置くのは「基点 + 値 × 係数」の式の形を崩さずに書くためである。
    let minus_one_lh = -1.0_f32;
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: minus_one_lh,
                unit: CursorUnit::Lh,
            },
            CursorAxis::X,
            &basis()
        ),
        Ok(Some(CURRENT.0 + minus_one_lh * LINE_PITCH))
    );
    // design.md「Unit Tests」の例 `@-1650%`: current.y + (−1650) × font_height / 100
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: -1650.0,
                unit: CursorUnit::Percent,
            },
            CursorAxis::Y,
            &basis()
        ),
        Ok(Some(CURRENT.1 + -1650.0 * (FONT_HEIGHT / 100.0)))
    );
}

/// 解決表「`centerx` on X → `image_size.0 / 2`」（R4.1/4.3）。
///
/// 共通前提の画像幅 400 は半分が 200 で `current.x` と同値になり基点の取り違えを弁別
/// できないため、`current`・`origin` のどちらとも異なる半分を持つ画像原寸を 1 件足す。
#[test]
fn centerx_on_x_resolves_to_half_the_image_width() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterX, CursorAxis::X, &basis()),
        Ok(Some(IMAGE_SIZE.0 / 2.0))
    );
    let discriminating = CursorBasis {
        image_size: (360.0, 180.0),
        ..basis()
    };
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterX, CursorAxis::X, &discriminating),
        Ok(Some(360.0 / 2.0))
    );
}

/// 解決表「`centery` on Y → `image_size.1 / 2`」（R4.2/4.3）。
#[test]
fn centery_on_y_resolves_to_half_the_image_height() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterY, CursorAxis::Y, &basis()),
        Ok(Some(IMAGE_SIZE.1 / 2.0))
    );
    let discriminating = CursorBasis {
        image_size: (360.0, 180.0),
        ..basis()
    };
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterY, CursorAxis::Y, &discriminating),
        Ok(Some(180.0 / 2.0))
    );
}

/// 解決表「`centerx` on Y・`centery` on X → `Err(CenterAxisMismatch)`」（R1.5・縮退表）。
#[test]
fn center_written_on_the_wrong_axis_degrades_to_axis_mismatch() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterX, CursorAxis::Y, &basis()),
        Err(CursorDegrade::CenterAxisMismatch)
    );
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterY, CursorAxis::X, &basis()),
        Err(CursorDegrade::CenterAxisMismatch)
    );
}

/// 解決表「解釈不能・非有限 → `Err(Unparsable)`」（R1.5/5.1/5.2）。
#[test]
fn invalid_degrades_to_unparsable() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Invalid, CursorAxis::X, &basis()),
        Err(CursorDegrade::Unparsable)
    );
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Invalid, CursorAxis::Y, &basis()),
        Err(CursorDegrade::Unparsable)
    );
}

/// 単位の係数は正典どおりのスカラーで、**軸に依らない**（R1.3/1.4）。
///
/// 係数そのもの（Px=1・Em=font_height・Lh=line_pitch・%=font_height/100）を檻化したうえで、
/// 同じ `1lh` を X と Y に与えたときの**基点からの移動量**が等しいことを見る。基点は
/// 軸ごとに異なる（`origin = (50, 20)`）ので、移動量で比較しないと軸非依存性は測れない。
#[test]
fn unit_coefficient_is_a_scalar_that_does_not_depend_on_the_axis() {
    assert_eq!(
        unit_coefficient(CursorUnit::Px, FONT_HEIGHT, LINE_PITCH),
        1.0
    );
    assert_eq!(
        unit_coefficient(CursorUnit::Em, FONT_HEIGHT, LINE_PITCH),
        FONT_HEIGHT
    );
    assert_eq!(
        unit_coefficient(CursorUnit::Lh, FONT_HEIGHT, LINE_PITCH),
        LINE_PITCH
    );
    assert_eq!(
        unit_coefficient(CursorUnit::Percent, FONT_HEIGHT, LINE_PITCH),
        FONT_HEIGHT / 100.0
    );

    let coord = CursorCoord::Absolute {
        value: 2.0,
        unit: CursorUnit::Lh,
    };
    let x = resolve_cursor_axis(coord, CursorAxis::X, &basis())
        .expect("実導出（縮退しない）")
        .expect("移動が成立する");
    let y = resolve_cursor_axis(coord, CursorAxis::Y, &basis())
        .expect("実導出（縮退しない）")
        .expect("移動が成立する");
    assert_eq!(x - ORIGIN.0, 2.0 * LINE_PITCH);
    assert_eq!(y - ORIGIN.1, 2.0 * LINE_PITCH);
    assert_eq!(x - ORIGIN.0, y - ORIGIN.1);
}
