//! 下線の幾何の檻（決定論・GPU なし・要件 3.3・3.4・3.10・3.11・4.3）。

use super::{ReadingAxis, StrokeRect, UnderlineRaw, pull_inside, stroke_overhang, stroke_rect};
use crate::viewbox::LineOverhang;
use crate::writing::WritingMode;

fn raw(reading: ReadingAxis, baseline_origin: (f32, f32), offset: f32) -> UnderlineRaw {
    UnderlineRaw {
        baseline_origin,
        width: 80.0,
        thickness: 2.0,
        offset,
        reading,
    }
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> StrokeRect {
    StrokeRect {
        left,
        top,
        right,
        bottom,
    }
}

// --- stroke_rect ---------------------------------------------------------------------------

/// 横書き（左から右）: 上端＝ベースライン（offset は使わない）・原点から右へ幅・太さのまま。
#[test]
fn horizontal_ltr_top_sits_on_the_baseline_and_runs_right() {
    let r = stroke_rect(
        &raw(ReadingAxis::LeftToRight, (5.0, 24.0), 3.5),
        (10.0, 100.0),
    );
    assert_eq!(r, rect(15.0, 124.0, 95.0, 126.0));
}

/// 横書き（右から左）: 上端＝ベースライン・原点から左へ幅。
#[test]
fn horizontal_rtl_top_sits_on_the_baseline_and_runs_left() {
    let r = stroke_rect(
        &raw(ReadingAxis::RightToLeft, (90.0, 24.0), 3.5),
        (10.0, 100.0),
    );
    assert_eq!(r, rect(20.0, 124.0, 100.0, 126.0));
}

/// 横書きは offset の値に依らず同じ位置（行ごとに決め方を変えない・要件 3.10）。
#[test]
fn horizontal_ignores_offset_for_every_line() {
    for offset in [-4.0, 0.0, 2.25, 7.0] {
        let r = stroke_rect(
            &raw(ReadingAxis::LeftToRight, (0.0, 30.0), offset),
            (0.0, 0.0),
        );
        assert_eq!(r.top, 30.0, "offset {offset}");
        assert_eq!(r.bottom, 32.0, "offset {offset}");
    }
}

/// 縦書き（上から下）: D2D の既定の描き方と同じく時計回りに 90 度回す。
/// 横書きの [offset, offset+太さ] の「下」が縦書きでは [x−offset−太さ, x−offset] の「左」へ、
/// 幅は原点から下へ伸びる。offset は使う（位置は DirectWrite のまま・要件 3.4）。
#[test]
fn vertical_rotates_dwrite_geometry_clockwise() {
    let r = stroke_rect(
        &raw(ReadingAxis::TopToBottom, (14.0, 3.0), 3.5),
        (100.0, 10.0),
    );
    assert_eq!(r, rect(108.5, 13.0, 110.5, 93.0));
    // offset が負（字の右側）でも同じ写し方。
    let r = stroke_rect(
        &raw(ReadingAxis::TopToBottom, (14.0, 3.0), -6.0),
        (100.0, 10.0),
    );
    assert_eq!(r, rect(118.0, 13.0, 120.0, 93.0));
}

// --- pull_inside ---------------------------------------------------------------------------

/// 収まっている線はそのまま。
#[test]
fn pull_inside_keeps_a_stroke_that_fits() {
    let r = rect(10.0, 118.0, 90.0, 120.0);
    assert_eq!(pull_inside(r, WritingMode::HorizontalTb, (200.0, 122.0)), r);
    let v = rect(0.0, 10.0, 2.0, 90.0);
    assert_eq!(pull_inside(v, WritingMode::VerticalRl, (200.0, 122.0)), v);
}

/// 横書き: 面の下端からはみ出す線は太さを保って `bound − 太さ` へ寄る。行内の軸（x）は動かない。
#[test]
fn pull_inside_horizontal_moves_overflowing_stroke_up_keeping_thickness() {
    let r = rect(10.0, 121.0, 90.0, 123.0);
    assert_eq!(
        pull_inside(r, WritingMode::HorizontalTb, (200.0, 122.0)),
        rect(10.0, 120.0, 90.0, 122.0)
    );
    // 上端からはみ出す線は 0 へ寄る。
    let r = rect(10.0, -1.5, 90.0, 0.5);
    assert_eq!(
        pull_inside(r, WritingMode::HorizontalTb, (200.0, 122.0)),
        rect(10.0, 0.0, 90.0, 2.0)
    );
}

/// 縦書き: 行送りの軸は x。左右どちらの端も内側へ寄り、行内の軸（y）は動かない（端の列・要件 3.4）。
#[test]
fn pull_inside_vertical_moves_along_x_only() {
    let left = rect(-1.0, 5.0, 1.0, 85.0);
    let right = rect(199.0, 5.0, 201.0, 85.0);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        assert_eq!(
            pull_inside(left, mode, (200.0, 50.0)),
            rect(0.0, 5.0, 2.0, 85.0),
            "{mode:?}"
        );
        assert_eq!(
            pull_inside(right, mode, (200.0, 50.0)),
            rect(198.0, 5.0, 200.0, 85.0),
            "{mode:?}"
        );
    }
}

/// 太さが範囲より大きいときは 0 の側に揃える（太さは保つ）。
#[test]
fn pull_inside_thicker_than_bound_aligns_to_zero() {
    let r = rect(10.0, 2.0, 90.0, 7.0);
    assert_eq!(
        pull_inside(r, WritingMode::HorizontalTb, (200.0, 4.0)),
        rect(10.0, 0.0, 90.0, 5.0)
    );
    let v = rect(2.0, 10.0, 7.0, 90.0);
    assert_eq!(
        pull_inside(v, WritingMode::VerticalRl, (4.0, 200.0)),
        rect(0.0, 10.0, 5.0, 90.0)
    );
}

// --- stroke_overhang -----------------------------------------------------------------------

/// 横書き: 行の箱の下からはみ出す量は bottom の欄だけに入る。
#[test]
fn overhang_horizontal_goes_into_bottom_only() {
    let strokes = [rect(0.0, 24.0, 80.0, 26.0), rect(80.0, 24.0, 120.0, 27.5)];
    assert_eq!(
        stroke_overhang(&strokes, WritingMode::HorizontalTb, 25.0),
        LineOverhang {
            bottom: 2.5,
            ..Default::default()
        }
    );
    // 上へはみ出す線は top の欄だけ。
    assert_eq!(
        stroke_overhang(
            &[rect(0.0, -1.0, 80.0, 1.0)],
            WritingMode::HorizontalTb,
            25.0
        ),
        LineOverhang {
            top: 1.0,
            ..Default::default()
        }
    );
}

/// 縦書き: はみ出しは left／right の欄だけに入る（行内の軸のはみ出しは数えない）。
#[test]
fn overhang_vertical_goes_into_left_or_right_only() {
    assert_eq!(
        stroke_overhang(
            &[rect(-3.0, -9.0, -1.0, 99.0)],
            WritingMode::VerticalRl,
            28.0
        ),
        LineOverhang {
            left: 3.0,
            ..Default::default()
        }
    );
    assert_eq!(
        stroke_overhang(
            &[rect(27.0, 0.0, 29.0, 80.0)],
            WritingMode::VerticalLr,
            28.0
        ),
        LineOverhang {
            right: 1.0,
            ..Default::default()
        }
    );
}

/// 行の箱に収まる線・線の無い行は 0。
#[test]
fn overhang_is_zero_when_strokes_fit_or_are_absent() {
    assert_eq!(
        stroke_overhang(
            &[rect(0.0, 20.0, 80.0, 22.0)],
            WritingMode::HorizontalTb,
            25.0
        ),
        LineOverhang::default()
    );
    assert_eq!(
        stroke_overhang(&[], WritingMode::VerticalRl, 25.0),
        LineOverhang::default()
    );
}
