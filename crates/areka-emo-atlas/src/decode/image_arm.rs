//! `image` クレートで動く絵（APNG・WebP）の全コマ／動きの 1 枚目を読み、乗算済み BGRA へ直す
//! （spec: areka-P0-animated-image-decode 要件 2.1〜2.5・2.7・2.8・3.2・3.4・6.4・7.5）。
//! 本番のソースで `image` の型を綴る唯一のファイル。公開面には `image` の型を出さない
//! （答えは `decode.rs` の型と `String` の失敗だけ）。
//!
//! - 読み手は先頭の署名で選ぶ（拡張子は見ない）。`set_background_color` は呼ばない。
//! - 透明度を持つかは、読む前に色の形式で 1 回だけ聞き、全コマ同じ値にする。
//! - 状態を持たない関数なので、同じファイルからは同じ答えになる。

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, Frame, Frames, ImageDecoder, RgbaImage};

use super::{AnimatedImage, AnimationFrame, AnimationInfo, DecodedImage};
use crate::table::LoopCount;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// 開いた動く絵（重ね済みのコマの並び・透明度の有無・繰り返し回数）。
struct Opened {
    frames: Frames<'static>,
    has_alpha: bool,
    loop_count: LoopCount,
}

/// 動く絵の全コマを読む。`info` は `sniff` が返した見出し。枚数・寸法が `info` と違えば `Err`。
/// 最初に読めなかったコマで止める（読めた分だけを返さない）。
// 本番の呼び手（`WicDecoderArm::decode_frames`）はタスク 3.4 でつなぐ。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn read_frames(path: &Path, info: AnimationInfo) -> Result<AnimatedImage, String> {
    let opened = open(path)?;
    let mut frames = Vec::new();
    for (n, frame) in opened.frames.enumerate() {
        if n as u64 >= u64::from(info.frame_count) {
            return Err(format!(
                "more frames than the header's {}",
                info.frame_count
            ));
        }
        let frame = frame.map_err(|e| format!("frame {n}: {e}"))?;
        let delay_ms = {
            let (numer, denom) = frame.delay().numer_denom_ms();
            delay_ms(numer, denom)
        };
        let image =
            to_bgra(frame, info, opened.has_alpha).map_err(|e| format!("frame {n}: {e}"))?;
        frames.push(AnimationFrame { image, delay_ms });
    }
    if frames.len() as u64 != u64::from(info.frame_count) {
        return Err(format!(
            "{} frames, but the header says {}",
            frames.len(),
            info.frame_count
        ));
    }
    Ok(AnimatedImage {
        frames,
        loop_count: opened.loop_count,
    })
}

/// 動きの 1 枚目だけを読む。コマの並びから最初の 1 つを取って止める（2 枚目以降は解かない）。
/// `read_frames` と同じ内部の関数を通るので、`read_frames` の 0 番のコマと同じ絵になる。
// 本番の呼び手（`WicDecoderArm::decode_first_frame`）はタスク 3.4 でつなぐ。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn read_first_frame(path: &Path, info: AnimationInfo) -> Result<DecodedImage, String> {
    let mut opened = open(path)?;
    let frame = opened
        .frames
        .next()
        .ok_or("no animation frame")?
        .map_err(|e| format!("frame 0: {e}"))?;
    to_bgra(frame, info, opened.has_alpha).map_err(|e| format!("frame 0: {e}"))
}

/// 先頭の署名で APNG・WebP の読み手を選んで開く。
fn open(path: &Path) -> Result<Opened, String> {
    let mut reader = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
    let mut head = [0u8; 12];
    reader
        .read_exact(&mut head)
        .and_then(|_| reader.seek(SeekFrom::Start(0)))
        .map_err(|e| e.to_string())?;
    if head[..8] == PNG_SIGNATURE {
        let png = PngDecoder::new(reader).map_err(|e| e.to_string())?;
        let has_alpha = png.color_type().has_alpha();
        let apng = png.apng().map_err(|e| e.to_string())?;
        Ok(opened(apng, has_alpha))
    } else if &head[..4] == b"RIFF" && &head[8..] == b"WEBP" {
        let webp = WebPDecoder::new(reader).map_err(|e| e.to_string())?;
        let has_alpha = webp.color_type().has_alpha();
        Ok(opened(webp, has_alpha))
    } else {
        Err("not an APNG or WebP file".into())
    }
}

fn opened<D: AnimationDecoder<'static>>(decoder: D, has_alpha: bool) -> Opened {
    let loop_count = match decoder.loop_count() {
        image::metadata::LoopCount::Infinite => LoopCount::Infinite,
        image::metadata::LoopCount::Finite(n) => LoopCount::Finite(n),
    };
    Opened {
        frames: decoder.into_frames(),
        has_alpha,
        loop_count,
    }
}

/// 重ね済みのコマ（乗算していない RGBA・絵の全体）を乗算済み BGRA へ直す。寸法が違えば `Err`。
fn to_bgra(frame: Frame, info: AnimationInfo, has_alpha: bool) -> Result<DecodedImage, String> {
    let rgba: RgbaImage = frame.into_buffer();
    let (width, height) = rgba.dimensions();
    if (width, height) != (info.width, info.height) {
        return Err(format!(
            "frame is {width}x{height}, but the header says {}x{}",
            info.width, info.height
        ));
    }
    let mut bgra = rgba.into_raw();
    for px in bgra.as_chunks_mut::<4>().0 {
        let [r, g, b, a] = *px;
        *px = [premultiply(b, a), premultiply(g, a), premultiply(r, a), a];
    }
    Ok(DecodedImage {
        width,
        height,
        stride: width * 4,
        bgra,
        has_alpha,
    })
}

/// 色を乗算する（`(c × a + 127) / 255`・WIC と全 65,536 通りで一致する式）。
fn premultiply(c: u8, a: u8) -> u8 {
    ((u32::from(c) * u32::from(a) + 127) / 255) as u8
}

/// 待ち時間の分数（ミリ秒）を四捨五入した整数のミリ秒にする。0 は 0。分母 0 は `image` が
/// 作らない（作れば panic する）が、ここでは 0 として扱い panic しない。`u32` の上限で止める。
fn delay_ms(numer: u32, denom: u32) -> u32 {
    if denom == 0 {
        return 0;
    }
    let (n, d) = (u64::from(numer), u64::from(denom));
    u32::try_from((n * 2 + d) / (d * 2)).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "image_arm_tests.rs"]
mod tests;
