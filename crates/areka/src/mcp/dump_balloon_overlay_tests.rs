//! 文字の面を原寸へ縮めて背景へ重ねる処理の決定論テスト（要件 3.1・3.3）。

use super::*;

/// 乗算済み BGRA の 1 画素。
fn bgra(b: u8, g: u8, r: u8, a: u8) -> [u8; 4] {
    [b, g, r, a]
}

/// 同じ画素で埋めた幅×高さの絵。
fn filled(px: [u8; 4], w: u32, h: u32) -> Vec<u8> {
    px.repeat((w * h) as usize)
}

/// 絵の `(x, y)` の画素。
fn at(img: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    img[i..i + 4].try_into().unwrap()
}

fn set(img: &mut [u8], w: u32, x: u32, y: u32, px: [u8; 4]) {
    let i = ((y * w + x) * 4) as usize;
    img[i..i + 4].copy_from_slice(&px);
}

const BG: [u8; 4] = [10, 20, 30, 255];

#[test]
fn scale_one_with_integer_offset_puts_text_pixels_as_they_are() {
    // 背景 4×3・文字の面 2×2 を (1, 1) へ。不透明の画素はそのまま、透明の画素は背景が残る。
    let mut canvas = filled(BG, 4, 3);
    let mut text = vec![0u8; 2 * 2 * 4];
    set(&mut text, 2, 0, 0, bgra(200, 100, 50, 255));
    set(&mut text, 2, 1, 0, bgra(1, 2, 3, 255));
    set(&mut text, 2, 0, 1, bgra(0, 0, 0, 0));
    set(&mut text, 2, 1, 1, bgra(255, 255, 255, 255));

    overlay_text(&mut canvas, (4, 3), (4, 3), &text, (2, 2), (1.0, 1.0));

    assert_eq!(at(&canvas, 4, 1, 1), bgra(200, 100, 50, 255));
    assert_eq!(at(&canvas, 4, 2, 1), bgra(1, 2, 3, 255));
    assert_eq!(at(&canvas, 4, 1, 2), BG);
    assert_eq!(at(&canvas, 4, 2, 2), bgra(255, 255, 255, 255));
}

#[test]
fn twice_the_physical_size_folds_a_2x2_block_back_into_one_pixel() {
    // 原寸 3×2・物理寸 6×4。文字の面 4×2 を物理 (2, 2) へ＝原寸の (1, 1)・(2, 1) に当たる。
    let mut canvas = filled(BG, 3, 2);
    let mut text = vec![0u8; 4 * 2 * 4];
    for (x, px) in [(0, bgra(40, 80, 120, 255)), (2, bgra(9, 8, 7, 255))] {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            set(&mut text, 4, x + dx, dy, px);
        }
    }

    overlay_text(&mut canvas, (3, 2), (6, 4), &text, (4, 2), (2.0, 2.0));

    assert_eq!(at(&canvas, 3, 1, 1), bgra(40, 80, 120, 255));
    assert_eq!(at(&canvas, 3, 2, 1), bgra(9, 8, 7, 255));
    assert_eq!(at(&canvas, 3, 0, 0), BG);
    assert_eq!(at(&canvas, 3, 0, 1), BG);
}

#[test]
fn half_transparent_text_blends_with_the_background() {
    // 乗算済みの半透明の白（アルファ 128）を不透明の背景へ: 出力 = 文字 + 背景×(255−128)÷255。
    let mut canvas = filled(bgra(200, 100, 0, 255), 1, 1);
    let text = bgra(128, 128, 128, 128).to_vec();

    overlay_text(&mut canvas, (1, 1), (1, 1), &text, (1, 1), (0.0, 0.0));

    // 128 + 200×127÷255 = 227.6 → 228、128 + 100×127÷255 = 177.8 → 178、128 + 0 = 128、
    // 128 + 255×127÷255 = 255。
    assert_eq!(at(&canvas, 1, 0, 0), bgra(228, 178, 128, 255));
}

#[test]
fn outside_the_text_surface_stays_background() {
    // 背景 5×5 の真ん中 1 画素だけに文字の面。その周り（面の外）は背景のまま。
    let mut canvas = filled(BG, 5, 5);
    let text = bgra(255, 0, 0, 255).to_vec();

    overlay_text(&mut canvas, (5, 5), (5, 5), &text, (1, 1), (2.0, 2.0));

    for y in 0..5 {
        for x in 0..5 {
            let expected = if (x, y) == (2, 2) {
                bgra(255, 0, 0, 255)
            } else {
                BG
            };
            assert_eq!(at(&canvas, 5, x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn scale_below_one_does_not_panic_and_keeps_the_background_size() {
    // 原寸 4×4・物理寸 2×2（拡大率 0.5）。不透明の文字の面 2×2 が原寸の全画素を覆う。
    let mut canvas = filled(BG, 4, 4);
    let text = filled(bgra(0, 0, 255, 255), 2, 2);

    overlay_text(&mut canvas, (4, 4), (2, 2), &text, (2, 2), (0.0, 0.0));

    assert_eq!(canvas.len(), 4 * 4 * 4);
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(at(&canvas, 4, x, y), bgra(0, 0, 255, 255), "({x}, {y})");
        }
    }
}

#[test]
fn non_integer_ratio_weights_edge_pixels_by_overlap_and_rounds_once() {
    // 原寸 2×1・物理寸 3×1（比 1.5）・offset (0, 0)・背景は透明。
    // 原寸の画素 0 は物理 [0, 1.5)＝文字の画素 0 を重み 1・画素 1 を重み 0.5、
    // 画素 1 は物理 [1.5, 3)＝文字の画素 1 を重み 0.5・画素 2 を重み 1。面積は 1.5。
    let mut canvas = vec![0u8; 2 * 4];
    let mut text = Vec::new();
    for px in [
        bgra(255, 0, 0, 255),
        bgra(1, 255, 2, 255),
        bgra(0, 0, 1, 255),
    ] {
        text.extend_from_slice(&px);
    }

    overlay_text(&mut canvas, (2, 1), (3, 1), &text, (3, 1), (0.0, 0.0));

    // 画素 0: B=(255+0.5×1)÷1.5=170.33→170・G=(0.5×255)÷1.5=85・R=(0.5×2)÷1.5=0.67→1・A=255。
    // 標本ごとに丸めると B=(255+1)÷1.5=170.67→171、重みを 1 にすると B=256÷1.5→171 になる。
    assert_eq!(at(&canvas, 2, 0, 0), bgra(170, 85, 1, 255));
    // 画素 1: B=(0.5×1)÷1.5=0.33→0・G=85・R=(0.5×2+1)÷1.5=1.33→1・A=255。
    // 標本ごとに面積で割って丸めると R=round(0.67)+round(0.67)=2 になる。
    assert_eq!(at(&canvas, 2, 1, 0), bgra(0, 85, 1, 255));
}
