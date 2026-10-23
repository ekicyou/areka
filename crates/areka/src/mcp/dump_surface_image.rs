//! 乗算済み BGRA の絵を、乗算を戻した 8 ビット RGBA の透過 PNG にして base64 で返す
//! （spec: areka-P0-mcp-dump-images・要件 5.1〜5.3・5.5）。窓も GPU も要らない純粋な関数だけを置く。

use crate::png_image;

/// 乗算済み BGRA（1 行＝幅×4・長さ＝幅×高さ×4）を、乗算を戻した RGBA の PNG にして base64 で返す。
/// 長さと寸法が合うこと・幅と高さが 1 以上であることは呼ぶ側が確かめる。大きさでは断らない（要件 5.5）。
pub(in crate::mcp) fn png_base64(premultiplied_bgra: &[u8], width: u32, height: u32) -> String {
    base64(&png_image::encode_png(premultiplied_bgra, width, height))
}

/// 標準の文字集合・`=` の詰めあり・改行なしの base64（要件 5.1）。
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let n = group
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= group.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "dump_surface_image_tests.rs"]
mod tests;
