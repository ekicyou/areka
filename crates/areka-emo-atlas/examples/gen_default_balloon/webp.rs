//! 動く WebP の組み立て（spec: default-balloon-selfmade 要件 3.2・3.3・3.4）。
//!
//! `image` の可逆の書き手で 1 コマずつ `VP8L` を作り、`VP8X`・`ANIM`・`ANMF` に包む。
// 前例 `gen_animated_samples.rs` から写す（example 同士は共有できず、前例を直すと検体の testdata のバイト列が動くため）。

use super::draw::Picture;
use image::ExtendedColorType;
use image::codecs::webp::WebPEncoder;

const WEBP_DISPOSE_BACKGROUND: u8 = 0x01;
const WEBP_NO_BLEND: u8 = 0x02;
const WEBP_FLAG_ALPHA: u8 = 0x10;
const WEBP_FLAG_ANIMATION: u8 = 0x02;

/// RIFF のチャンク（奇数長なら 0 を 1 バイト詰める）。
fn riff_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(kind);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
}

fn u24(v: u32) -> [u8; 3] {
    let b = v.to_le_bytes();
    [b[0], b[1], b[2]]
}

/// `image` の可逆の書き手で静止画を作り、`VP8L` のチャンク（見出しと詰めを含む）を返す。
fn encode_vp8l(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut webp = Vec::new();
    WebPEncoder::new_lossless(&mut webp)
        .encode(rgba, w, h, ExtendedColorType::Rgba8)
        .expect("cannot encode WebP frame");
    // 飾りの無い可逆の WebP は RIFF(12 バイト)＋VP8L の 1 チャンクだけ。
    assert_eq!(
        &webp[12..16],
        b"VP8L",
        "encoder output is not a single VP8L"
    );
    webp[12..].to_vec()
}

/// 同じ大きさの α 付き RGBA のコマを、繰り返し無限・各コマ `duration_ms`・全面 (0,0)・
/// 重ねない・背景は透明で消す動く WebP にする。大きさは 1 コマ目から取る。
pub fn animated_webp(frames: &[Picture], duration_ms: u32) -> Vec<u8> {
    let (w, h) = (frames[0].width, frames[0].height);
    let mut body = b"WEBP".to_vec();
    let mut vp8x = vec![WEBP_FLAG_ANIMATION | WEBP_FLAG_ALPHA, 0, 0, 0];
    vp8x.extend_from_slice(&u24(w - 1));
    vp8x.extend_from_slice(&u24(h - 1));
    riff_chunk(&mut body, b"VP8X", &vp8x);
    // 背景色は透明（並びは B・G・R・A）・繰り返し 0（無限）。
    let mut anim = vec![0, 0, 0, 0];
    anim.extend_from_slice(&0u16.to_le_bytes());
    riff_chunk(&mut body, b"ANIM", &anim);
    for f in frames {
        assert!(
            f.width == w && f.height == h,
            "all frames must share one size"
        );
        let mut anmf = Vec::new();
        for v in [0, 0, w - 1, h - 1, duration_ms] {
            anmf.extend_from_slice(&u24(v));
        }
        anmf.push(WEBP_DISPOSE_BACKGROUND | WEBP_NO_BLEND);
        anmf.extend_from_slice(&encode_vp8l(&f.rgba, w, h));
        riff_chunk(&mut body, b"ANMF", &anmf);
    }
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::AnimationDecoder;
    use image::codecs::webp::WebPDecoder;

    /// 77×36・3 コマ・コマ k は列 k 番目から右を α 128 の色 k で塗る。
    fn frames() -> Vec<Picture> {
        (0..3u8)
            .map(|k| Picture {
                width: 77,
                height: 36,
                rgba: (0..36 * 77)
                    .flat_map(|i| {
                        if i % 77 >= u32::from(k) * 20 {
                            [k * 80, 255 - k * 80, 40, 128]
                        } else {
                            [0, 0, 0, 0]
                        }
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn chunks_and_round_trip() {
        let src = frames();
        let bytes = animated_webp(&src, 300);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
        assert_eq!(&bytes[12..16], b"VP8X");
        assert_eq!(bytes[20], WEBP_FLAG_ANIMATION | WEBP_FLAG_ALPHA);
        // ANIM の中身: 背景色 0・0・0・0（透明）と繰り返し 0（無限）。
        assert_eq!(&bytes[30..34], b"ANIM");
        assert_eq!(&bytes[38..44], &[0, 0, 0, 0, 0, 0]);
        let anmf = bytes.windows(4).filter(|c| c == b"ANMF").count();
        assert_eq!(anmf, 3);
        assert_eq!(bytes, animated_webp(&src, 300), "not deterministic");

        let dec = WebPDecoder::new(std::io::Cursor::new(&bytes)).unwrap();
        let out = dec.into_frames().collect_frames().unwrap();
        assert_eq!(out.len(), 3);
        for (f, s) in out.iter().zip(&src) {
            assert_eq!(f.delay().numer_denom_ms(), (300, 1));
            assert_eq!(f.buffer().dimensions(), (77, 36));
            assert_eq!(f.buffer().as_raw(), &s.rgba);
        }
    }
}
