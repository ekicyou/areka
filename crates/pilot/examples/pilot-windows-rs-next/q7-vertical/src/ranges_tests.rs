//! 8.4 の試験。描くのは実 GPU の画面の外だけ（机は要らない・窓は出さない）。

use super::*;
use crate::new_side::ORIGIN_X;
use crate::old_side::TEXT;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// 「待って」: クラスタ 6..9（「 ち ょ っ と 、 待 っ て 。 🍣 」の 7〜9 字目）。絵文字を含めない。
const RANGE: Range<usize> = 6..9;

fn y_span(rects: &[Rect]) -> (f32, f32) {
    (
        rects.iter().map(|r| r.top).fold(f32::INFINITY, f32::min),
        rects
            .iter()
            .map(|r| r.bottom)
            .fold(f32::NEG_INFINITY, f32::max),
    )
}

#[test]
fn positions_convert_through_clusters() {
    // 「 は 3 バイト・UTF-16 で 1。クラスタ 6 は 6 字目の後＝18 バイト・UTF-16 の 6。
    assert_eq!(byte_range(TEXT, &RANGE), 18..27);
    let r = dwrite_range(TEXT, &RANGE);
    assert_eq!((r.startPosition, r.length), (6, 3));
    // 絵文字（クラスタ 10）は UTF-8 で 4 バイト・UTF-16 で 2。
    assert_eq!(byte_range(TEXT, &(10..11)), 30..34);
    let e = dwrite_range(TEXT, &(10..11));
    assert_eq!((e.startPosition, e.length), (10, 2));
}

#[test]
fn set_drawing_effect_paints_the_range_red() {
    let layout = new_side::layout(TEXT, Features::Areka).expect("配置");
    let glyphs = layout.range_bounds(byte_range(TEXT, &RANGE));
    let (top, bottom) = y_span(&glyphs);
    let plain = new_side::draw(TEXT, Features::Areka).expect("色なし");
    let red = draw_red(TEXT, Features::Areka, RANGE).expect("赤");
    let base = red_pixels(&plain).len();
    let painted = red_pixels(&red);
    let inside = |&(_, y): &(u32, u32)| (y as f32) >= top && (y as f32) < bottom;
    let black_inside = black_pixels(&red).iter().filter(|p| inside(p)).count();
    eprintln!(
        "[q7] red range bytes={:?} y={top}..{bottom} red={} base={base} black_inside={black_inside} black_total={}/{}",
        byte_range(TEXT, &RANGE),
        painted.len(),
        black_pixels(&red).len(),
        black_pixels(&plain).len()
    );
    assert!(painted.len() > base, "赤が付いていない");
    assert!(
        painted.iter().filter(|p| !inside(p)).count() <= base,
        "範囲の外に赤が出た"
    );
    assert_eq!(black_inside, 0, "範囲の字に黒が残った");
    // 範囲の外の字は黒のまま: 減った黒の数がちょうど赤になった数。
    assert_eq!(
        black_pixels(&plain).len() - black_pixels(&red).len(),
        painted.len() - base,
        "黒から赤へ移った数が合わない"
    );
}

#[test]
fn decorations_put_the_vertical_underline_left_of_the_column() {
    let u = underline(TEXT, Features::Areka, RANGE).expect("下線");
    eprintln!("[q7] underline lines={:?}", u.lines);
    eprintln!("[q7] underline glyphs={:?}", u.glyphs);
    eprintln!("[q7] underline areka={:?}", u.areka);
    assert!(!u.lines.is_empty(), "decorations() に下線が無い");
    let glyph_mid = (u.glyphs[0].left + u.glyphs[0].right) / 2.0;
    for (line, moved) in u.lines.iter().zip(&u.areka) {
        assert!(line.right <= glyph_mid, "下線が列の左に無い: {line:?}");
        assert!(
            moved.left >= glyph_mid,
            "寄せた下線が列の右に無い: {moved:?}"
        );
        assert!(
            moved.right <= u.glyphs[0].right,
            "寄せた下線が列の箱の右端を越えた: {moved:?}"
        );
        let thickness = (moved.right - moved.left) - (line.right - line.left);
        assert!(thickness.abs() < 1e-4, "太さが変わった");
        assert_eq!(
            (moved.top, moved.bottom),
            (line.top, line.bottom),
            "y の範囲"
        );
    }
    // 描いた絵でも同じ所に出る: 下線の矩形（面の座標）の中に墨がある。
    let ink = pixels(&u.shot, |p| p[3] > 0);
    let line = u.lines[0];
    let hits = ink
        .iter()
        .filter(|&&(x, y)| {
            let (x, y) = (x as f32 - ORIGIN_X as f32 + 0.5, y as f32 + 0.5);
            x >= line.left - 0.5 && x <= line.right + 0.5 && y >= line.top && y <= line.bottom
        })
        .count();
    eprintln!("[q7] underline ink inside the line rect={hits}");
    assert!(hits > 0, "下線が描いた絵の同じ所に出ていない");
}

fn panics(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

#[test]
fn text_layout_new_panics_on_bad_sizes_and_checked_layout_refuses_them() {
    let format = TextFormat::new("Yu Gothic", 20.0);
    for (w, h) in [
        (f32::NAN, 100.0),
        (-1.0, 100.0),
        (f32::INFINITY, 100.0),
        (20.0, f32::NAN),
    ] {
        let p = panics(|| {
            TextLayout::new("あ", &format, w, h);
        });
        eprintln!("[q7] TextLayout::new(w={w}, h={h}) panics={p}");
    }
    let bad_font = TextFormat::new("Yu Gothic", f32::NAN);
    let p = panics(|| {
        TextLayout::new("あ", &bad_font, 20.0, 100.0);
    });
    eprintln!("[q7] TextLayout::new(font NaN) panics={p}");
    assert!(panics(|| {
        TextLayout::new("あ", &format, f32::NAN, 100.0);
    }));
    assert!(checked_layout("あ", &format, 20.0, f32::NAN, 100.0).is_err());
    assert!(checked_layout("あ", &format, 20.0, -1.0, 100.0).is_err());
    assert!(checked_layout("あ", &bad_font, f32::NAN, 20.0, 100.0).is_err());
    assert!(checked_layout("あ", &format, 20.0, 20.0, 100.0).is_ok());
}
