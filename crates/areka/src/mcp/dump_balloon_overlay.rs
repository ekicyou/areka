//! バルーンの文字の面（物理寸）を原寸へ縮めて、背景（原寸）の上へ重ねる
//! （spec: areka-P0-mcp-dump-images・要件 3.1・3.3）。窓も GPU も要らない純粋な関数だけを置く。

/// `canvas`（背景・乗算済み BGRA・原寸）へ、文字の面を重ねて書き換える。
///
/// 原寸の画素 `(x, y)` が画面で占める範囲（横 `[x×pw÷nw, (x+1)×pw÷nw)`・縦も同じ）から
/// `text_offset` を引き、そこに掛かる文字の面の画素を掛かった面積で重み付けして平均する
/// （文字の面の外は透明として数える）。平均を乗算済みのまま「上に重ねる」:
/// `出力 = 文字 + 背景×(255−文字のアルファ)÷255`。範囲の端は f64 のまま画素へ寄せず、
/// 平均と重ねる計算も f64 で行い、最後に 1 回だけ四捨五入する。拡大率 1 の専用の分岐は持たない。
///
/// `canvas.len() == 原寸の幅×高さ×4`・`text.len() == 文字の面の幅×高さ×4` は呼ぶ側が確かめる。
pub(super) fn overlay_text(
    canvas: &mut [u8],
    native: (u32, u32),
    physical: (u32, u32),
    text: &[u8],
    text_size: (u32, u32),
    text_offset: (f32, f32),
) {
    let (nw, nh) = native;
    let (tw, th) = text_size;
    let sx = f64::from(physical.0) / f64::from(nw);
    let sy = f64::from(physical.1) / f64::from(nh);
    let (ox, oy) = (f64::from(text_offset.0), f64::from(text_offset.1));
    for y in 0..nh {
        // 文字の面の座標での、この行の範囲。
        let y0 = f64::from(y) * sy - oy;
        let y1 = f64::from(y + 1) * sy - oy;
        let rows = covered(y0, y1, th);
        for x in 0..nw {
            let x0 = f64::from(x) * sx - ox;
            let x1 = f64::from(x + 1) * sx - ox;
            let area = (x1 - x0) * (y1 - y0);
            if area <= 0.0 {
                continue;
            }
            let mut sum = [0.0f64; 4];
            for ty in rows.clone() {
                let wy = overlap(y0, y1, ty);
                for tx in covered(x0, x1, tw) {
                    let w = overlap(x0, x1, tx) * wy;
                    let i = ((ty * tw + tx) * 4) as usize;
                    for (s, &c) in sum.iter_mut().zip(&text[i..i + 4]) {
                        *s += w * f64::from(c);
                    }
                }
            }
            let src = sum.map(|s| s / area);
            let i = ((y * nw + x) * 4) as usize;
            let keep = (255.0 - src[3]) / 255.0;
            for (dst, s) in canvas[i..i + 4].iter_mut().zip(src) {
                *dst = (s + f64::from(*dst) * keep).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// 範囲 `[lo, hi)` に掛かる、`0..len` の中の画素の番号。
fn covered(lo: f64, hi: f64, len: u32) -> std::ops::Range<u32> {
    let first = lo.floor().max(0.0).min(f64::from(len)) as u32;
    let end = hi.ceil().max(0.0).min(f64::from(len)) as u32;
    first..end
}

/// 範囲 `[lo, hi)` と画素 `[p, p+1)` の重なりの長さ。
fn overlap(lo: f64, hi: f64, p: u32) -> f64 {
    let p = f64::from(p);
    (hi.min(p + 1.0) - lo.max(p)).max(0.0)
}

#[cfg(test)]
#[path = "dump_balloon_overlay_tests.rs"]
mod tests;
