//! # inline_marker — 行の中の印（`\![*]`・`%*`）の目印の字と形の計算（純粋層）
//!
//! 印の合図は、文字の層の項目の列に 1 字分の目印の字 [`MARKER_GLYPH`]（U+FFFC）として積む。
//! 測るときと描くときは全角の空白 [`MARKER_STAND_IN`] に読み替えるので、印の後ろの字の位置は
//! DirectWrite が描く字と同じ物差しで決まる（要件 5.5）。字を描いた後の段で、1 字分の枠
//! [`CellRect`] に既定の丸（[`dot_in_cell`]）か印の絵（[`image_in_cell`]）を塗る（要件 5.1・5.4）。
//!
//! **層規律**: 純粋層。COM も wintf の型も使わない（`PURE_SOURCES` に載せて走査する）。
//! D2D の型への写しは描き手（`viewbox_draw_render.rs`）の側で行う。

use std::sync::Arc;

use areka_emo_present::balloon_marker::MarkerImage;
use areka_parsers::sakura::MARKER_COMMAND;
use areka_sakura::contract::CueCommand;

use crate::layout::PositionedGlyph;

/// 項目の列に積む目印の字（1 つのクラスタ・UTF-16 で 1 単位）。
pub const MARKER_GLYPH: &str = "\u{FFFC}";
/// 測る・描くときの読み替え先（全角の空白・UTF-16 で 1 単位）。
pub const MARKER_STAND_IN: &str = "\u{3000}";
/// 既定の丸の直径（字の大きさに対する比・字の大きさ 20 で 10 画素＝SSP の測り）。
pub const DOT_DIAMETER_RATIO: f32 = 0.5;
/// 既定の丸の塗りの不透明度（その字の文字色に掛ける）。
pub const DOT_OPACITY: f32 = 0.64;

/// 印の見た目（アクターごと・既定は丸）。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum InlineMarkerLook {
    /// 既定の丸（バルーンに印の絵が無いとき）。
    #[default]
    Dot,
    /// バルーンの `marker.filename` などで選んだ印の絵。
    Image(Arc<MarkerImage>),
}

/// 1 字分の枠（バルーンの画像の画素・左上と右下）。COM の型を使わない純粋な数の組。
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CellRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl CellRect {
    fn width(&self) -> f32 {
        self.right - self.left
    }
    fn height(&self) -> f32 {
        self.bottom - self.top
    }
    fn center(&self) -> (f32, f32) {
        (
            (self.left + self.right) / 2.0,
            (self.top + self.bottom) / 2.0,
        )
    }
}

/// 印の合図（名前が [`MARKER_COMMAND`] の `\!` の汎用の合図）か。
pub fn is_marker_carrier(command: &CueCommand) -> bool {
    matches!(command.as_command_carrier(), Some((name, _)) if name == MARKER_COMMAND)
}

/// 測る・描く字（目印の字だけを全角の空白に替える）。
pub fn stand_in(text: &str) -> &str {
    if text == MARKER_GLYPH {
        MARKER_STAND_IN
    } else {
        text
    }
}

/// 行の文字列（DirectWrite で測る・描く字）。グリフの字を [`stand_in`] で読み替えて繋ぐ。
/// 行の文字列を作る所（描き手・照合の写し）は全部この 1 つを通す。
pub fn line_text(glyphs: &[PositionedGlyph]) -> String {
    glyphs.iter().map(|g| stand_in(&g.text)).collect()
}

/// 1 字分の枠から、丸の中心 `(x, y)` と半径（直径＝`em × DOT_DIAMETER_RATIO`）。
pub fn dot_in_cell(cell: CellRect, em: f32) -> ((f32, f32), f32) {
    (cell.center(), em * DOT_DIAMETER_RATIO / 2.0)
}

/// 1 字分の枠に、絵を原寸で真ん中に置く矩形（はみ出すときは縦横の比を保って枠に収める）。
pub fn image_in_cell(cell: CellRect, width: u32, height: u32) -> CellRect {
    let (w, h) = (width as f32, height as f32);
    let scale = if w <= cell.width() && h <= cell.height() {
        1.0
    } else {
        (cell.width() / w).min(cell.height() / h)
    };
    let (w, h) = (w * scale, h * scale);
    let (cx, cy) = cell.center();
    CellRect {
        left: cx - w / 2.0,
        top: cy - h / 2.0,
        right: cx + w / 2.0,
        bottom: cy + h / 2.0,
    }
}

#[cfg(test)]
#[path = "inline_marker_tests.rs"]
mod tests;
