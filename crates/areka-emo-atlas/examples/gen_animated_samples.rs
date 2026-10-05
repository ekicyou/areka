//! 動く絵の検体を作る（spec: areka-P0-animated-image-decode 要件 8.1）。
//!
//! `cargo run -p areka-emo-atlas --example gen_animated_samples` で
//! `src/testdata/animated/` に 12 個を書く。使うのは `image` と std だけ。
//! 同じ版の `image`・`image-webp` なら何度走らせても同じバイト列になる。
//! 中身の説明は同じフォルダの `README.md`。
//!
//! - APNG: `image` の PNG の書き手で 1 コマずつ静止画を作り、`IHDR` と `IDAT` の中身を
//!   取り出して `acTL`・`fcTL`・`IDAT`・`fdAT` に包み直す（CRC は下の数行）。
//! - WebP: `image` の可逆の書き手で 1 コマずつ作り、`VP8L` のチャンクを
//!   `VP8X`・`ANIM`・`ANMF` に包む。
//! - GIF: 手書きのバイト列。

use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageEncoder};
use std::path::Path;

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// w×h の RGBA を、座標から色を決める関数で塗る。
fn paint(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| f(x, y))
        .collect()
}

/// RGBA から α を落として RGB にする。
fn rgb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect()
}

/// 8 ビットの RGBA を 16 ビット（ネイティブの並び）へ広げる（`image` の書き手の約束）。
fn widen16(rgba: &[u8]) -> Vec<u8> {
    rgba.iter()
        .flat_map(|&c| (u16::from(c) * 257).to_ne_bytes())
        .collect()
}

// ---- PNG / APNG ----

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// `image` の PNG の書き手で静止画を作り、`IHDR` の中身と `IDAT` の中身（連結）を返す。
fn encode_png(pixels: &[u8], w: u32, h: u32, color: ExtendedColorType) -> (Vec<u8>, Vec<u8>) {
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(pixels, w, h, color)
        .expect("PNG を書けない");
    let (mut ihdr, mut idat) = (Vec::new(), Vec::new());
    let mut pos = 8;
    while pos + 8 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
        let data = &png[pos + 8..pos + 8 + len];
        match &png[pos + 4..pos + 8] {
            b"IHDR" => ihdr = data.to_vec(),
            b"IDAT" => idat.extend_from_slice(data),
            _ => {}
        }
        pos += 12 + len;
    }
    (ihdr, idat)
}

const APNG_DISPOSE_NONE: u8 = 0;
const APNG_DISPOSE_BACKGROUND: u8 = 1;
const APNG_DISPOSE_PREVIOUS: u8 = 2;
const APNG_BLEND_SOURCE: u8 = 0;
const APNG_BLEND_OVER: u8 = 1;

/// APNG の 1 コマ。`pixels` は `color` の形で w×h。
struct ApngFrame {
    pixels: Vec<u8>,
    w: u32,
    h: u32,
    x: u32,
    y: u32,
    delay: (u16, u16),
    dispose: u8,
    blend: u8,
}

/// 全面・残す・置き換えのコマ。
fn full(pixels: Vec<u8>, delay: (u16, u16)) -> ApngFrame {
    ApngFrame {
        pixels,
        w: 8,
        h: 8,
        x: 0,
        y: 0,
        delay,
        dispose: APNG_DISPOSE_NONE,
        blend: APNG_BLEND_SOURCE,
    }
}

struct Apng<'a> {
    color: ExtendedColorType,
    num_plays: u32,
    /// 動きに入らない既定の絵（`fcTL` を前に置かない `IDAT`）。
    default_image: Option<Vec<u8>>,
    /// `tRNS` の中身（`IHDR` の後ろ・`IDAT` の前に置く）。
    trns: Option<&'a [u8]>,
    frames: Vec<ApngFrame>,
}

fn apng(a: &Apng) -> Vec<u8> {
    let encoded: Vec<_> = a
        .frames
        .iter()
        .map(|f| encode_png(&f.pixels, f.w, f.h, a.color))
        .collect();
    let default = a
        .default_image
        .as_ref()
        .map(|p| encode_png(p, 8, 8, a.color));
    let ihdr = &default.as_ref().unwrap_or(&encoded[0]).0;

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    png_chunk(&mut out, b"IHDR", ihdr);
    let mut actl = (a.frames.len() as u32).to_be_bytes().to_vec();
    actl.extend_from_slice(&a.num_plays.to_be_bytes());
    png_chunk(&mut out, b"acTL", &actl);
    if let Some(trns) = a.trns {
        png_chunk(&mut out, b"tRNS", trns);
    }
    if let Some((_, idat)) = &default {
        png_chunk(&mut out, b"IDAT", idat);
    }
    let mut seq = 0u32;
    for (i, (f, (_, idat))) in a.frames.iter().zip(&encoded).enumerate() {
        let mut fctl = seq.to_be_bytes().to_vec();
        for v in [f.w, f.h, f.x, f.y] {
            fctl.extend_from_slice(&v.to_be_bytes());
        }
        fctl.extend_from_slice(&f.delay.0.to_be_bytes());
        fctl.extend_from_slice(&f.delay.1.to_be_bytes());
        fctl.extend_from_slice(&[f.dispose, f.blend]);
        png_chunk(&mut out, b"fcTL", &fctl);
        seq += 1;
        if i == 0 && default.is_none() {
            png_chunk(&mut out, b"IDAT", idat);
        } else {
            let mut fdat = seq.to_be_bytes().to_vec();
            fdat.extend_from_slice(idat);
            png_chunk(&mut out, b"fdAT", &fdat);
            seq += 1;
        }
    }
    png_chunk(&mut out, b"IEND", &[]);
    out
}

/// `basic.apng` を、2 コマ目（1 番）の `fdAT` の中身の途中で切る。
fn truncate_in_second_frame(basic: &[u8]) -> Vec<u8> {
    let mut pos = 8;
    let mut fdat_seen = 0;
    loop {
        let len = u32::from_be_bytes(basic[pos..pos + 4].try_into().unwrap()) as usize;
        if &basic[pos + 4..pos + 8] == b"fdAT" {
            fdat_seen += 1;
            if fdat_seen == 1 {
                // 見出し 8 バイト＋番号 4 バイト＋圧縮した中身の半分まで。
                return basic[..pos + 12 + (len - 4) / 2].to_vec();
            }
        }
        pos += 12 + len;
    }
}

// ---- WebP ----

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
fn encode_vp8l(pixels: &[u8], w: u32, h: u32, color: ExtendedColorType) -> Vec<u8> {
    let mut webp = Vec::new();
    WebPEncoder::new_lossless(&mut webp)
        .encode(pixels, w, h, color)
        .expect("WebP を書けない");
    // 飾りの無い可逆の WebP は RIFF(12 バイト)＋VP8L の 1 チャンクだけ。
    assert_eq!(&webp[12..16], b"VP8L", "書き手の出力が VP8L 1 つでない");
    webp[12..].to_vec()
}

const WEBP_DISPOSE_BACKGROUND: u8 = 0x01;
const WEBP_NO_BLEND: u8 = 0x02;
const WEBP_FLAG_ALPHA: u8 = 0x10;
const WEBP_FLAG_ANIMATION: u8 = 0x02;

/// WebP の 1 コマ。x・y は偶数（ファイルには半分の値で入る）。
struct WebpFrame {
    pixels: Vec<u8>,
    w: u32,
    h: u32,
    x: u32,
    y: u32,
    duration_ms: u32,
    flags: u8,
}

fn webp_full(pixels: Vec<u8>, duration_ms: u32, flags: u8) -> WebpFrame {
    WebpFrame {
        pixels,
        w: 8,
        h: 8,
        x: 0,
        y: 0,
        duration_ms,
        flags,
    }
}

/// 8×8 の動く WebP。`alpha` が偽ならコマは RGB で書き、`VP8X` の α の旗を立てない。
fn animated_webp(alpha: bool, loop_count: u16, frames: &[WebpFrame]) -> Vec<u8> {
    let color = if alpha {
        ExtendedColorType::Rgba8
    } else {
        ExtendedColorType::Rgb8
    };
    let mut body = b"WEBP".to_vec();
    let mut vp8x = vec![
        WEBP_FLAG_ANIMATION | if alpha { WEBP_FLAG_ALPHA } else { 0 },
        0,
        0,
        0,
    ];
    vp8x.extend_from_slice(&u24(8 - 1));
    vp8x.extend_from_slice(&u24(8 - 1));
    riff_chunk(&mut body, b"VP8X", &vp8x);
    // 背景色は不透明な白（並びは B・G・R・A）。
    let mut anim = vec![255, 255, 255, 255];
    anim.extend_from_slice(&loop_count.to_le_bytes());
    riff_chunk(&mut body, b"ANIM", &anim);
    for f in frames {
        assert!(f.x % 2 == 0 && f.y % 2 == 0, "WebP のコマの位置は偶数");
        let pixels = if alpha {
            f.pixels.clone()
        } else {
            rgb(&f.pixels)
        };
        let mut anmf = Vec::new();
        for v in [f.x / 2, f.y / 2, f.w - 1, f.h - 1, f.duration_ms] {
            anmf.extend_from_slice(&u24(v));
        }
        anmf.push(f.flags);
        anmf.extend_from_slice(&encode_vp8l(&pixels, f.w, f.h, color));
        riff_chunk(&mut body, b"ANMF", &anmf);
    }
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

// ---- GIF ----

/// 2×2・2 コマ（赤・緑）・各 100 ms・繰り返し無限の GIF89a。手書き。
/// 画素の符号は「消去・色」を 4 回くり返して終わり（どれも 3 ビット）なので、表が育たない。
const TWO_FRAMES_GIF: &[u8] = &[
    b'G', b'I', b'F', b'8', b'9', b'a', //
    0x02, 0x00, 0x02, 0x00, // 幅 2・高さ 2
    0x91, 0x00, 0x00, // 全体の色表あり（4 色）・背景 0・縦横比 0
    // 色表: 赤・緑・黒・黒
    0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //
    0x21, 0xFF, 0x0B, b'N', b'E', b'T', b'S', b'C', b'A', b'P', b'E', b'2', b'.', b'0', //
    0x03, 0x01, 0x00, 0x00, 0x00, // 繰り返し 0（無限）
    // 1 コマ目: 100 ms・全部が色 0（赤）
    0x21, 0xF9, 0x04, 0x00, 0x0A, 0x00, 0x00, 0x00, //
    0x2C, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, //
    0x02, 0x04, 0x04, 0x41, 0x10, 0x05, 0x00, //
    // 2 コマ目: 100 ms・全部が色 1（緑）
    0x21, 0xF9, 0x04, 0x00, 0x0A, 0x00, 0x00, 0x00, //
    0x2C, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, //
    0x02, 0x04, 0x0C, 0xC3, 0x30, 0x05, 0x00, //
    0x3B,
];

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/testdata/animated");
    std::fs::create_dir_all(&dir).expect("検体のフォルダを作れない");
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(dir.join(name), bytes).expect("検体を書けない");
        println!("{name}: {} バイト", bytes.len());
    };

    let left_half =
        |left: [u8; 4], right: [u8; 4]| paint(8, 8, move |x, _| if x < 4 { left } else { right });
    let solid = |c: [u8; 4]| paint(8, 8, move |_, _| c);
    let rgba = ExtendedColorType::Rgba8;

    let basic = apng(&Apng {
        color: rgba,
        num_plays: 2,
        default_image: None,
        trns: None,
        frames: vec![
            full(left_half(RED, CLEAR), (1, 3)),
            ApngFrame {
                pixels: paint(4, 4, |_, _| GREEN),
                w: 4,
                h: 4,
                x: 4,
                y: 0,
                delay: (0, 100),
                dispose: APNG_DISPOSE_BACKGROUND,
                blend: APNG_BLEND_OVER,
            },
            ApngFrame {
                pixels: paint(2, 2, |_, _| BLUE),
                w: 2,
                h: 2,
                x: 2,
                y: 2,
                delay: (7, 0),
                dispose: APNG_DISPOSE_PREVIOUS,
                blend: APNG_BLEND_OVER,
            },
            full(solid(CLEAR), (1, 1000)),
        ],
    });
    write("basic.apng", &basic);
    write("truncated.apng", &truncate_in_second_frame(&basic));

    write(
        "default_image.apng",
        &apng(&Apng {
            color: rgba,
            num_plays: 0,
            default_image: Some(solid(BLACK)),
            trns: None,
            frames: vec![full(solid(RED), (1, 10)), full(solid(GREEN), (1, 10))],
        }),
    );

    let rgb_frames = || {
        vec![
            full(rgb(&left_half(WHITE, RED)), (1, 10)),
            full(rgb(&left_half(WHITE, BLUE)), (1, 10)),
        ]
    };
    write(
        "rgb.apng",
        &apng(&Apng {
            color: ExtendedColorType::Rgb8,
            num_plays: 0,
            default_image: None,
            trns: None,
            frames: rgb_frames(),
        }),
    );
    // tRNS の RGB は 16 ビットずつ: 白（255,255,255）を透明にする。
    write(
        "trns.apng",
        &apng(&Apng {
            color: ExtendedColorType::Rgb8,
            num_plays: 0,
            default_image: None,
            trns: Some(&[0, 255, 0, 255, 0, 255]),
            frames: rgb_frames(),
        }),
    );
    write(
        "single.apng",
        &apng(&Apng {
            color: rgba,
            num_plays: 0,
            default_image: None,
            trns: None,
            frames: vec![full(solid(RED), (1, 10))],
        }),
    );
    write(
        "deep16.apng",
        &apng(&Apng {
            color: ExtendedColorType::Rgba16,
            num_plays: 0,
            default_image: None,
            trns: None,
            frames: vec![
                full(widen16(&solid(RED)), (1, 10)),
                full(widen16(&solid(GREEN)), (1, 10)),
            ],
        }),
    );

    let alpha = animated_webp(
        true,
        3,
        &[
            webp_full(
                left_half(RED, CLEAR),
                100,
                WEBP_DISPOSE_BACKGROUND | WEBP_NO_BLEND,
            ),
            WebpFrame {
                pixels: paint(4, 8, |_, y| if y == 0 { CLEAR } else { GREEN }),
                w: 4,
                h: 8,
                x: 4,
                y: 0,
                duration_ms: 0,
                flags: 0,
            },
            WebpFrame {
                pixels: paint(2, 2, |_, _| BLUE),
                w: 2,
                h: 2,
                x: 2,
                y: 2,
                duration_ms: 70,
                flags: 0,
            },
        ],
    );
    write("alpha.webp", &alpha);
    write("webp_named.png", &alpha);
    write(
        "rgb.webp",
        &animated_webp(
            false,
            0,
            &[
                webp_full(left_half(WHITE, RED), 100, WEBP_NO_BLEND),
                webp_full(left_half(WHITE, BLUE), 100, WEBP_NO_BLEND),
            ],
        ),
    );
    write(
        "single.webp",
        &animated_webp(true, 0, &[webp_full(solid(RED), 100, WEBP_NO_BLEND)]),
    );

    write("two_frames.gif", TWO_FRAMES_GIF);
}
