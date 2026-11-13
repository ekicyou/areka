//! 動く WebP（RIFF）を組む部品（`gen_animated_samples` と `gen_default_balloon` の `webp.rs` が `#[path]` で読む）。
//!
//! このフォルダには `main.rs` が無いので、cargo は example として拾わない。

use image::ExtendedColorType;
use image::codecs::webp::WebPEncoder;

pub const WEBP_DISPOSE_BACKGROUND: u8 = 0x01;
pub const WEBP_NO_BLEND: u8 = 0x02;
pub const WEBP_FLAG_ALPHA: u8 = 0x10;
pub const WEBP_FLAG_ANIMATION: u8 = 0x02;

/// RIFF のチャンク（奇数長なら 0 を 1 バイト詰める）。
pub fn riff_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(kind);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

pub fn u24(v: u32) -> [u8; 3] {
    let b = v.to_le_bytes();
    [b[0], b[1], b[2]]
}

/// `image` の可逆の書き手で静止画を作り、`VP8L` のチャンク（見出しと詰めを含む）を返す。
pub fn encode_vp8l(pixels: &[u8], w: u32, h: u32, color: ExtendedColorType) -> Vec<u8> {
    let mut webp = Vec::new();
    WebPEncoder::new_lossless(&mut webp)
        .encode(pixels, w, h, color)
        .expect("cannot encode WebP frame");
    // 飾りの無い可逆の WebP は RIFF(12 バイト)＋VP8L の 1 チャンクだけ。
    assert_eq!(
        &webp[12..16],
        b"VP8L",
        "encoder output is not a single VP8L"
    );
    webp[12..].to_vec()
}
