//! `sniff` の兄弟テスト（spec: areka-P0-animated-image-decode 要件 1.1〜1.5・6.3・8.3）。
//!
//! 検体 12 個の答えと、メモリ上のバイト列で作る境目を判定する。

use std::io::{Cursor, Read, Seek, SeekFrom};

use super::sniff;
use crate::decode::AnimationInfo;

fn sniff_bytes(bytes: &[u8]) -> Option<AnimationInfo> {
    sniff(&mut Cursor::new(bytes))
}

fn info(width: u32, height: u32, frame_count: u32) -> Option<AnimationInfo> {
    Some(AnimationInfo {
        width,
        height,
        frame_count,
    })
}

// ---- 検体 12 個 ----

macro_rules! sample {
    ($name:literal) => {
        include_bytes!(concat!("../testdata/animated/", $name)).as_slice()
    };
}

#[test]
fn samples_answer_by_content() {
    let cases: [(&str, &[u8], Option<AnimationInfo>); 12] = [
        ("basic.apng", sample!("basic.apng"), info(8, 8, 4)),
        // 途中で切れていても見出し（IDAT より前）は揃っている
        ("truncated.apng", sample!("truncated.apng"), info(8, 8, 4)),
        // 既定の絵は数えない（acTL の枚数どおり）
        (
            "default_image.apng",
            sample!("default_image.apng"),
            info(8, 8, 2),
        ),
        ("rgb.apng", sample!("rgb.apng"), info(8, 8, 2)),
        ("trns.apng", sample!("trns.apng"), info(8, 8, 2)),
        // 16 ビットでも見出しは普通の動く APNG と同じ
        ("deep16.apng", sample!("deep16.apng"), info(8, 8, 2)),
        ("single.apng", sample!("single.apng"), None),
        ("alpha.webp", sample!("alpha.webp"), info(8, 8, 3)),
        ("rgb.webp", sample!("rgb.webp"), info(8, 8, 2)),
        ("single.webp", sample!("single.webp"), None),
        // 拡張子は .png でも中身は動く WebP
        ("webp_named.png", sample!("webp_named.png"), info(8, 8, 3)),
        // 動く GIF は対象外
        ("two_frames.gif", sample!("two_frames.gif"), None),
    ];
    for (name, bytes, expected) in cases {
        assert_eq!(sniff_bytes(bytes), expected, "{name}");
    }
}

// ---- バイト列の組み立て ----

const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&[0; 4]); // CRC（sniff は見ない）
}

fn ihdr(width: u32, height: u32) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&width.to_be_bytes());
    d.extend_from_slice(&height.to_be_bytes());
    d.extend_from_slice(&[8, 6, 0, 0, 0]);
    d
}

fn actl(frames: u32) -> Vec<u8> {
    let mut d = frames.to_be_bytes().to_vec();
    d.extend_from_slice(&0u32.to_be_bytes());
    d
}

/// 署名＋IHDR（寸法 w×h）。続くチャンクは呼び手が足す。
fn png_head(width: u32, height: u32) -> Vec<u8> {
    let mut out = PNG_SIG.to_vec();
    png_chunk(&mut out, b"IHDR", &ihdr(width, height));
    out
}

fn webp_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(kind);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0); // RIFF の詰め
    }
}

fn vp8x(flags: u8, width: u32, height: u32) -> Vec<u8> {
    let mut d = vec![flags, 0, 0, 0];
    d.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    d.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    d
}

/// RIFF の見出しを付ける（長さの欄は中身から計算）。
fn riff(body: &[u8]) -> Vec<u8> {
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(body);
    out
}

const ANIM_FLAG: u8 = 0x02;

/// 読んだバイト数を数える読み手。
struct Counting<'a> {
    inner: Cursor<&'a [u8]>,
    read: usize,
}

impl Read for Counting<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n;
        Ok(n)
    }
}

impl Seek for Counting<'_> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

// ---- 署名 ----

#[test]
fn shorter_than_12_bytes_is_none() {
    assert_eq!(sniff_bytes(&[]), None);
    assert_eq!(sniff_bytes(&PNG_SIG), None);
    let mut short_png = PNG_SIG.to_vec();
    short_png.extend_from_slice(&[0, 0, 0]);
    assert_eq!(short_png.len(), 11);
    assert_eq!(sniff_bytes(&short_png), None);
    assert_eq!(sniff_bytes(b"RIFF\0\0\0\0WEB"), None);
}

#[test]
fn static_png_and_static_webp_are_none() {
    let mut png = png_head(8, 8);
    png_chunk(&mut png, b"IDAT", &[0; 16]);
    png_chunk(&mut png, b"IEND", &[]);
    assert_eq!(sniff_bytes(&png), None);

    // 旗なしの VP8X（ANMF を 2 つ持っていても動きの旗が無ければ静止画）
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8X", &vp8x(0x10, 8, 8));
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    assert_eq!(sniff_bytes(&riff(&body)), None);

    // VP8X の無い単純な WebP
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8L", &[0; 16]);
    assert_eq!(sniff_bytes(&riff(&body)), None);
}

// ---- PNG の境目 ----

#[test]
fn png_dimensions_come_from_ihdr() {
    let mut png = png_head(300, 70_000);
    png_chunk(&mut png, b"acTL", &actl(5));
    png_chunk(&mut png, b"IDAT", &[0; 4]);
    assert_eq!(sniff_bytes(&png), info(300, 70_000, 5));
}

#[test]
fn actl_after_idat_is_none_and_reading_stops_at_idat() {
    let mut png = png_head(8, 8);
    png_chunk(&mut png, b"IDAT", &[0; 4096]);
    png_chunk(&mut png, b"acTL", &actl(4));
    png_chunk(&mut png, b"IEND", &[]);

    let mut reader = Counting {
        inner: Cursor::new(png.as_slice()),
        read: 0,
    };
    assert_eq!(sniff(&mut reader), None);
    // 署名＋IHDR＋IDAT の見出しまで（IDAT の中身は読まない）
    assert!(reader.read < 64, "read {} bytes", reader.read);
}

#[test]
fn actl_frame_count_zero_and_one_are_none() {
    for frames in [0, 1] {
        let mut png = png_head(8, 8);
        png_chunk(&mut png, b"acTL", &actl(frames));
        png_chunk(&mut png, b"IDAT", &[0; 4]);
        assert_eq!(sniff_bytes(&png), None, "acTL frames = {frames}");
    }
    let mut png = png_head(8, 8);
    png_chunk(&mut png, b"acTL", &actl(2));
    png_chunk(&mut png, b"IDAT", &[0; 4]);
    assert_eq!(sniff_bytes(&png), info(8, 8, 2));
}

#[test]
fn png_chunk_longer_than_rest_stops_with_what_was_read() {
    // acTL の後ろで長すぎるチャンク → acTL までの答え
    let mut png = png_head(8, 8);
    png_chunk(&mut png, b"acTL", &actl(3));
    png.extend_from_slice(&u32::MAX.to_be_bytes());
    png.extend_from_slice(b"tEXt");
    png.extend_from_slice(&[0; 8]);
    let mut reader = Counting {
        inner: Cursor::new(png.as_slice()),
        read: 0,
    };
    assert_eq!(sniff(&mut reader), info(8, 8, 3));
    assert!(reader.read <= png.len());

    // acTL そのものが残りより長い → 読まない
    let mut png = png_head(8, 8);
    png.extend_from_slice(&64u32.to_be_bytes());
    png.extend_from_slice(b"acTL");
    png.extend_from_slice(&actl(3));
    assert_eq!(sniff_bytes(&png), None);

    // acTL より前で長すぎるチャンク → acTL に届かない
    let mut png = png_head(8, 8);
    png.extend_from_slice(&u32::MAX.to_be_bytes());
    png.extend_from_slice(b"tEXt");
    png_chunk(&mut png, b"acTL", &actl(3));
    assert_eq!(sniff_bytes(&png), None);
}

// ---- WebP の境目 ----

#[test]
fn webp_dimensions_are_24_bit_plus_one() {
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8X", &vp8x(ANIM_FLAG, 1 << 24, 300));
    webp_chunk(&mut body, b"ANIM", &[0; 6]);
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    assert_eq!(sniff_bytes(&riff(&body)), info(1 << 24, 300, 2));
}

#[test]
fn webp_odd_length_chunks_skip_the_pad_byte() {
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8X", &vp8x(ANIM_FLAG, 8, 8));
    webp_chunk(&mut body, b"ICCP", &[1, 2, 3]); // 奇数長＋詰め 1 バイト
    webp_chunk(&mut body, b"ANIM", &[0; 6]);
    webp_chunk(&mut body, b"ANMF", &[0; 17]); // 中身も奇数長
    webp_chunk(&mut body, b"ANMF", &[0; 17]);
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    assert_eq!(sniff_bytes(&riff(&body)), info(8, 8, 3));
}

#[test]
fn webp_anim_flag_without_anmf_is_none() {
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8X", &vp8x(ANIM_FLAG, 8, 8));
    webp_chunk(&mut body, b"ANIM", &[0; 6]);
    assert_eq!(sniff_bytes(&riff(&body)), None);
}

#[test]
fn webp_chunk_longer_than_rest_stops_with_what_was_counted() {
    let huge = |body: &mut Vec<u8>| {
        body.extend_from_slice(b"ANMF");
        body.extend_from_slice(&u32::MAX.to_le_bytes());
        body.extend_from_slice(&[0; 8]);
    };

    // 2 つ数えた後で長すぎる ANMF → 数えた 2 つで答える
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8X", &vp8x(ANIM_FLAG, 8, 8));
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    huge(&mut body);
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    let bytes = riff(&body);
    let mut reader = Counting {
        inner: Cursor::new(bytes.as_slice()),
        read: 0,
    };
    assert_eq!(sniff(&mut reader), info(8, 8, 2));
    assert!(reader.read <= bytes.len());

    // 1 つ数えた後で長すぎる ANMF → 2 に届かない
    let mut body = Vec::new();
    webp_chunk(&mut body, b"VP8X", &vp8x(ANIM_FLAG, 8, 8));
    webp_chunk(&mut body, b"ANMF", &[0; 16]);
    huge(&mut body);
    assert_eq!(sniff_bytes(&riff(&body)), None);

    // VP8X そのものが残りより長い
    let mut body = b"VP8X".to_vec();
    body.extend_from_slice(&64u32.to_le_bytes());
    body.extend_from_slice(&vp8x(ANIM_FLAG, 8, 8));
    assert_eq!(sniff_bytes(&riff(&body)), None);
}
