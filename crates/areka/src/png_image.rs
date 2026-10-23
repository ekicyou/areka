//! 乗算済み BGRA の絵を、乗算を戻した 8 ビット RGBA の透過 PNG にする唯一の場所（spec:
//! areka-P0-mcp-dump-images・要件 5.2〜5.3／areka-P0-offline-surface-dump・要件 4.4・4.6）。
//! MCP の書き出しとオフラインの書き出しが共有する。窓も GPU も要らない純粋な関数だけを置く。

/// 乗算済み BGRA（1 行＝幅×4・長さ＝幅×高さ×4）を、乗算を戻した RGBA の PNG のバイト列にする。
/// 長さと寸法が合うこと・幅と高さが 1 以上であることは呼ぶ側が確かめる。
pub(crate) fn encode_png(premultiplied_bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
    png(&unpremultiply(premultiplied_bgra), width, height)
}

/// 乗算済み BGRA → 乗算していない RGBA。アルファ 0 は `(0,0,0,0)`。
/// それ以外は各色 `min(255, round(色×255÷アルファ))`（要件 5.2）。
fn unpremultiply(premultiplied_bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(premultiplied_bgra.len());
    for &[b, g, r, a] in premultiplied_bgra.as_chunks::<4>().0 {
        if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let a32 = u32::from(a);
        // 整数で四捨五入: (c·255 + a/2) / a。
        let un = |c: u8| ((u32::from(c) * 255 + a32 / 2) / a32).min(255) as u8;
        rgba.extend_from_slice(&[un(r), un(g), un(b), a]);
    }
    rgba
}

/// 8 ビット RGBA の PNG。署名 → `IHDR` → `IDAT` 1 つ（各行の先頭にフィルタ 0 を付けて zlib 圧縮）→ `IEND`。
fn png(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row = width as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for line in rgba.chunks_exact(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    // 深さ 8・カラータイプ 6（RGBA）・圧縮 0・フィルタ 0・インターレース 0。
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let idat = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [
        (b"IHDR", &ihdr[..]),
        (b"IDAT", &idat[..]),
        (b"IEND", &[][..]),
    ] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = areka_nar::crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    out
}

#[cfg(test)]
#[path = "png_image_tests.rs"]
mod tests;
