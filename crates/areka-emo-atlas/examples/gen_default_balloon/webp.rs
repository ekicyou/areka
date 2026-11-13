//! 動く WebP の組み立て（spec: default-balloon-selfmade 要件 3.2・3.3・3.4）。
//!
//! `image` の可逆の書き手で 1 コマずつ `VP8L` を作り、`VP8X`・`ANIM`・`ANMF` に包む。

#[path = "../common/riff.rs"]
mod riff;

use super::draw::Picture;
use image::ExtendedColorType;
use riff::{
    WEBP_DISPOSE_BACKGROUND, WEBP_FLAG_ALPHA, WEBP_FLAG_ANIMATION, WEBP_NO_BLEND, encode_vp8l,
    riff_chunk, u24,
};

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
        anmf.extend_from_slice(&encode_vp8l(&f.rgba, w, h, ExtendedColorType::Rgba8));
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
