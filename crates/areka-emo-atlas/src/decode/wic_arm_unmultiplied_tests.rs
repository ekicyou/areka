//! `WicDecoderArm::decode_unmultiplied` の兄弟テスト。
//!
//! 絵はテストの中でメモリに組み、ワークツリーの `target` の下へ書いてから読む（WIC はファイル名で開く）。
//! ガンマと色の情報のチャンク（gAMA・cHRM・sRGB・iCCP）を IHDR の直後へ差し込み、R・G・B が動かないことを見る。

use std::path::{Path, PathBuf};

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

use crate::decode::{DecodeError, ElementDecoder};
use crate::test_support::{pixel, with_arm};

/// 書いた RGBA（左から: 半透明・不透明・透明だが色あり・ほぼ透明）。
const RGBA: [[u8; 4]; 4] = [
    [200, 100, 50, 128],
    [10, 20, 30, 255],
    [77, 88, 99, 0],
    [255, 1, 128, 1],
];

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let body = [&kind[..], data].concat();
    [
        &(data.len() as u32).to_be_bytes()[..],
        &body,
        &crc32(&body).to_be_bytes(),
    ]
    .concat()
}

/// 無圧縮の deflate の 1 塊で包んだ zlib の流れ（iCCP の中身に使う）。
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let len = data.len() as u16;
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    [
        &[0x78, 0x01, 0x01][..],
        &len.to_le_bytes(),
        &(!len).to_le_bytes(),
        data,
        &((b << 16) | a).to_be_bytes(),
    ]
    .concat()
}

/// ガンマ 1.0 の灰色の ICC の形（見出し 128 バイト＋タグ 0 個）。中身を WIC が当てれば色が動く。
fn icc_profile() -> Vec<u8> {
    let mut p = vec![0u8; 132];
    p[0..4].copy_from_slice(&132u32.to_be_bytes());
    p[8] = 2;
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(b"RGB ");
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p
}

/// 差し込むチャンク（名前・中身）。gAMA はガンマ 1.0、cHRM は sRGB から外した原色。
fn color_chunks() -> Vec<(&'static str, Vec<u8>)> {
    let chrm: Vec<u8> = [
        31_270u32, 32_900, 70_000, 25_000, 20_000, 75_000, 10_000, 1_000,
    ]
    .iter()
    .flat_map(|v| v.to_be_bytes())
    .collect();
    let iccp = [&b"p\0\0"[..], &zlib_stored(&icc_profile())].concat();
    vec![
        ("gAMA", 100_000u32.to_be_bytes().to_vec()),
        ("cHRM", chrm),
        ("sRGB", vec![0]),
        ("iCCP", iccp),
    ]
}

/// 絵を PNG にし、`extra` のチャンクを IHDR の直後へ差し込んで `target` の下へ書く。
fn write_png(
    name: &str,
    raw: &[u8],
    color: ExtendedColorType,
    extra: &[(&str, Vec<u8>)],
) -> PathBuf {
    let width = raw.len() as u32
        / if color == ExtendedColorType::Rgba8 {
            4
        } else {
            3
        };
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(raw, width, 1, color)
        .expect("encode png");
    // 署名 8 バイト＋IHDR 25 バイト
    let mut bytes = png[..33].to_vec();
    for (kind, data) in extra {
        bytes.extend(chunk(kind.as_bytes().try_into().unwrap(), data));
    }
    bytes.extend_from_slice(&png[33..]);
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/emo_atlas_unmultiplied_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.png"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn rgba_png(name: &str, extra: &[(&str, Vec<u8>)]) -> PathBuf {
    write_png(name, RGBA.as_flattened(), ExtendedColorType::Rgba8, extra)
}

/// 読んだ BGRA を RGBA の並びへ戻す。
fn read_rgba(path: &Path) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    with_arm(|arm| {
        let img = arm.decode_unmultiplied(path).expect("unmultiplied read");
        assert_eq!((img.width, img.height, img.stride), (4, 1, 16));
        assert_eq!(img.bgra.len(), 16);
        out = (0..img.width)
            .map(|x| {
                let [b, g, r, a] = pixel(&img.bgra, img.stride, x, 0);
                [r, g, b, a]
            })
            .collect();
    });
    out
}

/// 較正: 差し込んだチャンクを WIC が読めている（壊れたチャンクで黙って素通りしていない）。
fn wic_sees_chunk(arm: &super::WicDecoderArm, path: &Path, kind: &str) -> bool {
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Graphics::Imaging::WICDecodeMetadataCacheOnDemand;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::core::{HSTRING, PWSTR};
    let mut names = Vec::new();
    // SAFETY: COM は with_arm が初期化済み。Next が返した文字列は 1 度だけ読んで CoTaskMemFree で返す。
    unsafe {
        let frame = arm
            .factory
            .CreateDecoderFromFilename(
                &HSTRING::from(path),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .and_then(|d| d.GetFrame(0))
            .expect("frame 0");
        let items = frame
            .GetMetadataQueryReader()
            .and_then(|r| r.GetEnumerator())
            .expect("metadata names");
        loop {
            let mut name = [PWSTR::null()];
            let mut fetched = 0u32;
            if items.Next(&mut name, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            names.push(name[0].to_string().unwrap_or_default());
            CoTaskMemFree(Some(name[0].0 as *const _));
        }
    }
    names.iter().any(|n| n == &format!("/{kind}"))
}

#[test]
fn semi_transparent_rgb_is_returned_as_written() {
    let path = rgba_png("plain", &[]);
    with_arm(|arm| assert!(!wic_sees_chunk(arm, &path, "gAMA")));
    assert_eq!(read_rgba(&path), RGBA);
    with_arm(|arm| assert!(arm.decode_unmultiplied(&path).unwrap().has_alpha));
}

#[test]
fn color_chunks_do_not_move_rgb() {
    for (kind, data) in color_chunks() {
        let path = rgba_png(&format!("chunk_{kind}"), &[(kind, data)]);
        with_arm(|arm| assert!(wic_sees_chunk(arm, &path, kind), "WIC reads {kind}"));
        assert_eq!(read_rgba(&path), RGBA, "with {kind}");
    }
    let all: Vec<_> = color_chunks()
        .into_iter()
        .filter(|(k, _)| *k != "iCCP")
        .collect();
    assert_eq!(read_rgba(&rgba_png("chunk_gama_chrm_srgb", &all)), RGBA);
}

#[test]
fn image_without_alpha_reads_as_opaque() {
    let rgb: Vec<u8> = RGBA.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
    let path = write_png("rgb", &rgb, ExtendedColorType::Rgb8, &color_chunks()[..1]);
    let expected: Vec<[u8; 4]> = RGBA.iter().map(|p| [p[0], p[1], p[2], 255]).collect();
    assert_eq!(read_rgba(&path), expected);
    with_arm(|arm| assert!(!arm.decode_unmultiplied(&path).unwrap().has_alpha));
}

#[test]
fn unreadable_and_missing_fail_with_reason() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/emo_atlas_unmultiplied_test");
    std::fs::create_dir_all(&dir).unwrap();
    let broken = dir.join("broken.png");
    std::fs::write(&broken, b"not a png").unwrap();
    let missing = dir.join("does_not_exist.png");
    with_arm(|arm| {
        match arm.decode_unmultiplied(&broken) {
            Err(DecodeError::Decode { path, source }) => {
                assert_eq!(path, broken);
                assert!(!source.is_empty());
            }
            other => panic!("expected Decode, got {other:?}"),
        }
        match arm.decode_unmultiplied(&missing) {
            Err(DecodeError::NotFound { path }) => assert_eq!(path, missing),
            other => panic!("expected NotFound, got {other:?}"),
        }
    });
}

#[test]
fn side_over_max_is_refused() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/emo_atlas_unmultiplied_test/16385x1.png");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbaImage::new(16_385, 1).save(&path).unwrap();
    with_arm(|arm| match arm.decode_unmultiplied(&path) {
        Err(DecodeError::Decode { source, .. }) => {
            assert!(
                source.contains("16385") && source.contains("16384"),
                "{source}"
            );
        }
        other => panic!("expected Decode, got {:?}", other.map(|i| i.width)),
    });
}
