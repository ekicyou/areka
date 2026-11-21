//! 問い 7:範囲の色・下線の位置・`TextLayout::new` の panic。
//!
//! 範囲は areka の文字の位置（拡張書記素クラスタの序数）で受け、`choices` の換算で
//! `windows-text` の UTF-8 のバイト位置（`set_underline`・`range_bounds`）と DirectWrite の
//! UTF-16 の位置（借りた `IDWriteTextLayout::SetDrawingEffect`）へ直す。クラスタの境目は
//! 必ず `char` の境目なので、`windows-text` の「境目でないと panic」には当たらない。

use bind_next::Windows::Win32::*;
use std::ops::Range;
use windows_text::{DecorationKind, Rect, TextFormat, TextLayout};

use crate::choices::{cluster_to_byte, cluster_to_utf16};
use crate::diff::Shot;
use crate::new_side::{self, Features};

const RED: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// areka の文字の位置の範囲を UTF-8 のバイト範囲へ（`windows-text` の範囲の口へ渡す形）。
pub fn byte_range(text: &str, clusters: &Range<usize>) -> Range<usize> {
    cluster_to_byte(text, clusters.start)..cluster_to_byte(text, clusters.end)
}

/// areka の文字の位置の範囲を DirectWrite の範囲（UTF-16）へ。
pub fn dwrite_range(text: &str, clusters: &Range<usize>) -> DWRITE_TEXT_RANGE {
    let start = cluster_to_utf16(text, clusters.start) as u32;
    let end = cluster_to_utf16(text, clusters.end) as u32;
    DWRITE_TEXT_RANGE {
        startPosition: start,
        length: end - start,
    }
}

/// `clusters` の範囲だけを赤の単色の筆で描く（`windows-text` に範囲の色の口が無いので、
/// 借りた DirectWrite の型の `SetDrawingEffect` を直に呼ぶ）。筆は描く文脈と同じ装置で作る。
pub fn draw_red(text: &str, features: Features, clusters: Range<usize>) -> Result<Shot, String> {
    let layout = new_side::layout(text, features)?;
    let range = dwrite_range(text, &clusters);
    new_side::render_with(&layout, |dc, raw| unsafe {
        let brush = dc.CreateSolidColorBrush(&RED, None)?;
        raw.SetDrawingEffect(&brush, range).ok()
    })
}

/// 赤の画素（premultiplied で B=G=0・R>0）の位置。
pub fn red_pixels(shot: &Shot) -> Vec<(u32, u32)> {
    pixels(shot, |p| p[0] == 0 && p[1] == 0 && p[2] > 0)
}

/// 黒の墨（B=G=R=0・α>0）の位置。
pub fn black_pixels(shot: &Shot) -> Vec<(u32, u32)> {
    pixels(shot, |p| p[3] > 0 && p[0] == 0 && p[1] == 0 && p[2] == 0)
}

fn pixels(shot: &Shot, keep: impl Fn(&[u8; 4]) -> bool) -> Vec<(u32, u32)> {
    let px = shot.bgra.as_chunks::<4>().0;
    (0..shot.height)
        .flat_map(|y| (0..shot.width).map(move |x| (x, y)))
        .filter(|&(x, y)| keep(&px[(y * shot.width + x) as usize]))
        .collect()
}

/// 下線を引いた配置から取った位置（どれも配置の座標・面の座標は x に `new_side::ORIGIN_X` を足す）。
pub struct Underline {
    /// `decorations()` の下線（DirectWrite が置く所）。
    pub lines: Vec<Rect>,
    /// 範囲の字の矩形（`range_bounds`）。
    pub glyphs: Vec<Rect>,
    /// 下線ごとの、areka の決まり（列の右）へ寄せた位置。
    pub areka: Vec<Rect>,
    /// DirectWrite が置いたまま描いた絵（下線が実際に出る所の確かめ）。
    pub shot: Shot,
}

/// `clusters` の範囲に `windows-text` の下線を引き、`decorations()` の位置を取る。
pub fn underline(
    text: &str,
    features: Features,
    clusters: Range<usize>,
) -> Result<Underline, String> {
    let mut layout = new_side::layout(text, features)?;
    let bytes = byte_range(text, &clusters);
    layout.set_underline(bytes.clone(), true);
    let lines: Vec<Rect> = layout
        .decorations()
        .into_iter()
        .filter(|d| d.kind == DecorationKind::Underline)
        .map(|d| d.bounds)
        .collect();
    let glyphs = layout.range_bounds(bytes);
    let column = column_of(&layout);
    let areka = lines.iter().map(|l| to_column_right(*l, column)).collect();
    let shot = new_side::render(&layout)?;
    Ok(Underline {
        lines,
        glyphs,
        areka,
        shot,
    })
}

/// 列の箱の左右（配置の座標）。1 列の配置なので、組んだ文の幅＝列の幅（行間の UNIFORM の高さ）。
fn column_of(layout: &TextLayout) -> (f32, f32) {
    let m = layout.metrics();
    (m.left, m.left + m.width)
}

/// 手当て: DirectWrite は縦書きの下線を列の左（文字の下＝反時計回りに 90 度の側）に置き、列の箱の左端を
/// またぐ（測った値: 配置の x=-0.62..0.40）。areka の決まり（選択肢の下線は列の右）には、列の箱の左右の
/// 中で鏡に映し（左端からの距離を右端からの距離にする）、鏡で右端を越えた分は内へ寄せる（列が面の右端に
/// 接すると越えた分は切れて細る）。太さと y の範囲はそのまま。
pub fn to_column_right(line: Rect, (left, right): (f32, f32)) -> Rect {
    let width = line.right - line.left;
    let end = (left + right - line.left).min(right);
    Rect {
        left: end - width,
        right: end,
        ..line
    }
}

/// 本体での扱いの見立て（候補）: `TextLayout::new` は DirectWrite の失敗を `unwrap` で panic にする
/// （書式も初めて使う所で同じ）。本体は入る前に値を確かめて `Err` にし、境（`TextLayout::new` を呼ぶ 1 か所）
/// の外へ panic を出さない。確かめるのは呼ぶ側が持つ値: 字の大きさ・箱の大きさ・文の UTF-16 の長さ。
pub fn checked_layout(
    text: &str,
    format: &TextFormat,
    font_size: f32,
    max_width: f32,
    max_height: f32,
) -> Result<TextLayout, String> {
    if !(font_size.is_finite() && font_size > 0.0) {
        return Err(format!("字の大きさが正の有限でない: {font_size}"));
    }
    for (name, v) in [("幅", max_width), ("高さ", max_height)] {
        if v.is_nan() || v < 0.0 {
            return Err(format!("箱の{name}が NaN か負: {v}"));
        }
    }
    if text.encode_utf16().count() > u32::MAX as usize {
        return Err("文が UTF-16 で u32 を越える".to_owned());
    }
    Ok(TextLayout::new(text, format, max_width, max_height))
}

#[cfg(test)]
#[path = "ranges_tests.rs"]
mod tests;
