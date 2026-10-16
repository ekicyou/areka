//! 下線の幾何（純粋層・areka-P0-balloon-text-bugs 件 3）。
//!
//! DirectWrite が `IDWriteTextRenderer::DrawUnderline` で教える下線の値（COM 層
//! `viewbox_draw_underline.rs` が数値に写して渡す）から、areka が塗る矩形を決める。
//!
//! - 横書き（左から右・右から左）は線の上端をベースラインに置く（要件 3.10。DirectWrite の
//!   `offset` は使わない＝どの行も同じ規則）。太さと幅は DirectWrite の値のまま（要件 3.2）。
//! - 縦書き（上から下）は DirectWrite の `offset` と太さのまま、D2D の既定の描き方と同じ向き
//!   （ベースラインの原点の周りに時計回りに 90 度）へ回す（要件 3.4「側と位置は今のまま」）。
//! - 面に収まらない線だけを行送りの軸で面の内側へ寄せる（要件 3.11・3.4）。太さは保ち、行内の
//!   軸は動かさない。面は広げない（下線の有無でレイアウトを変えない＝要件 3.7）。
//!
//! 本モジュールは `windows` に依存しない（層の檻 `PURE_SOURCES` が走査する）。

use crate::viewbox::LineOverhang;
use crate::writing::WritingMode;

/// 下線の行内の向き（DirectWrite の `DWRITE_UNDERLINE::readingDirection` の写し）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadingAxis {
    /// 左から右（線はベースラインの原点から右へ伸びる）。
    LeftToRight,
    /// 右から左（線はベースラインの原点から左へ伸びる）。
    RightToLeft,
    /// 上から下（縦書き。線はベースラインの原点から下へ伸びる）。
    TopToBottom,
}

/// DirectWrite が下線について教える値の写し（行の原点基準・画像の画素）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UnderlineRaw {
    /// ベースラインの原点（`DrawUnderline` の `baselineOriginX/Y`）。
    pub baseline_origin: (f32, f32),
    /// 行内の軸の長さ。
    pub width: f32,
    /// 太さ。
    pub thickness: f32,
    /// ベースラインからのずれ（正＝字から離れる側）。横書きでは使わない。
    pub offset: f32,
    /// 行内の向き。
    pub reading: ReadingAxis,
}

/// 塗る矩形（画像の画素・面の左上が原点）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StrokeRect {
    /// 左辺。
    pub left: f32,
    /// 上辺。
    pub top: f32,
    /// 右辺。
    pub right: f32,
    /// 下辺。
    pub bottom: f32,
}

/// 塗る矩形を決める（`origin`＝行の原点の面の上の位置）。
///
/// 横書き（左から右・右から左）は上端をベースラインに置く（offset を使わない）。
/// 縦書き（上から下）は DirectWrite の offset と太さのまま、D2D の既定の描き方と同じく
/// ベースラインの原点の周りに時計回りに 90 度回す（横書きの「下」が縦書きの「左」になる）。
pub(crate) fn stroke_rect(raw: &UnderlineRaw, origin: (f32, f32)) -> StrokeRect {
    let x = origin.0 + raw.baseline_origin.0;
    let y = origin.1 + raw.baseline_origin.1;
    let t = raw.thickness;
    match raw.reading {
        ReadingAxis::LeftToRight => StrokeRect {
            left: x,
            top: y,
            right: x + raw.width,
            bottom: y + t,
        },
        ReadingAxis::RightToLeft => StrokeRect {
            left: x - raw.width,
            top: y,
            right: x,
            bottom: y + t,
        },
        // 横書きの矩形 [0, width] × [offset, offset + t] を (u, v) → (−v, u) で回す。
        ReadingAxis::TopToBottom => StrokeRect {
            left: x - raw.offset - t,
            top: y,
            right: x - raw.offset,
            bottom: y + raw.width,
        },
    }
}

/// 行送りの軸（横書き＝縦・縦書き＝横）で、面の範囲 `[0, bound]` に収まらなければ内側へ寄せる。
///
/// 太さは保つ（太さが範囲より大きければ 0 の側に揃える）。行内の軸は動かさない。
/// 収まっていれば同じ矩形を返す。`bound`＝面の物理寸 ÷ k（画像の画素・`(幅, 高さ)`）。
pub(crate) fn pull_inside(rect: StrokeRect, mode: WritingMode, bound: (f32, f32)) -> StrokeRect {
    // 1 本の軸の [lo, hi] を [0, limit] の内側へ寄せる（長さは保つ）。
    let pull = |lo: f32, hi: f32, limit: f32| {
        let len = hi - lo;
        if len > limit || lo < 0.0 {
            (0.0, len)
        } else if hi > limit {
            (limit - len, limit)
        } else {
            (lo, hi)
        }
    };
    match mode {
        WritingMode::HorizontalTb => {
            let (top, bottom) = pull(rect.top, rect.bottom, bound.1);
            StrokeRect {
                top,
                bottom,
                ..rect
            }
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let (left, right) = pull(rect.left, rect.right, bound.0);
            StrokeRect {
                left,
                right,
                ..rect
            }
        }
    }
}

/// 行の箱（行送りの軸 `[0, extent]`・行の原点基準）から線がはみ出す量を、はみ出した側の欄に
/// 入れて返す（はみ出さない側と行内の軸の欄は 0）。`strokes` は `origin = (0, 0)` で決めた矩形。
pub(crate) fn stroke_overhang(
    strokes: &[StrokeRect],
    mode: WritingMode,
    extent: f32,
) -> LineOverhang {
    // 行送りの軸の両端で、行の箱 [0, extent] から出た量の最大（出なければ 0）。
    let (before, after) = strokes.iter().fold((0.0f32, 0.0f32), |(b, a), s| {
        let (lo, hi) = match mode {
            WritingMode::HorizontalTb => (s.top, s.bottom),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (s.left, s.right),
        };
        (b.max(-lo), a.max(hi - extent))
    });
    match mode {
        WritingMode::HorizontalTb => LineOverhang {
            top: before,
            bottom: after,
            ..Default::default()
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => LineOverhang {
            left: before,
            right: after,
            ..Default::default()
        },
    }
}

#[cfg(test)]
#[path = "underline_tests.rs"]
mod tests;
