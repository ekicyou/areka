//! 乗算を戻す・PNG・base64 の決定論テスト（要件 7.1・5.1〜5.3・5.5）。
//! PNG は自分で読むだけでなく、別の実装（WIC の復号）でも読み戻して確かめる。

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Imaging::{
    GUID_WICPixelFormat32bppRGBA, IWICBitmapSource, WICBitmapDitherTypeNone,
    WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::{HSTRING, Interface};
use wintf::com::wic::{
    WICBitmapDecoderExt, WICBitmapSourceExt, WICFormatConverterExt, WICImagingFactoryExt,
    wic_factory,
};

use super::*;

/// 乗算済み BGRA の 1 画素。
fn bgra(b: u8, g: u8, r: u8, a: u8) -> [u8; 4] {
    [b, g, r, a]
}

fn assert_within_one(actual: &[u8], expected: &[u8], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: 長さ");
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            a.abs_diff(*e) <= 1,
            "{what}: {i} バイト目が {a}（期待 {e}・差は 1 段階まで）"
        );
    }
}

#[test]
fn unpremultiply_transparent_translucent_opaque() {
    let input: Vec<u8> = [
        bgra(0, 0, 0, 0),        // 完全に透明
        bgra(9, 8, 7, 0),        // アルファ 0 の色のごみ → 全部 0
        bgra(32, 64, 96, 128),   // 半透明
        bgra(1, 0, 127, 128),    // 半透明（四捨五入の境目を含む）
        bgra(10, 20, 30, 255),   // 不透明＝そのまま
        bgra(200, 100, 50, 100), // 色がアルファを超える壊れた値 → 255 で頭打ち
    ]
    .concat();
    let expected: Vec<u8> = [
        [0, 0, 0, 0],
        [0, 0, 0, 0],
        [191, 128, 64, 128], // R=96·255/128=191.25・G=127.5・B=63.75
        [253, 0, 2, 128],    // R=127·255/128=253.0・B=1·255/128=1.99
        [30, 20, 10, 255],
        [128, 255, 255, 100], // R=50·255/100=127.5・G=255・B=510→255
    ]
    .concat();
    let actual = unpremultiply(&input);
    assert_within_one(&actual, &expected, "乗算を戻した RGBA");
    // 完全に透明は「1 段階以内」でなく厳密に 0（要件 5.2）。
    assert_eq!(&actual[..8], &[0; 8], "アルファ 0 は全部 0");
    // 不透明は丸めが起きないので厳密に一致する。
    assert_eq!(&actual[16..20], &[30, 20, 10, 255], "不透明はそのまま");
}

/// PNG のチャンク 1 つ（種類・中身・書かれていた CRC）。
struct Chunk {
    kind: [u8; 4],
    data: Vec<u8>,
    crc: u32,
}

/// 署名の後ろをチャンクの並びへ割る。長さが合わなければ落とす。
fn chunks(png: &[u8]) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut rest = &png[8..];
    while !rest.is_empty() {
        let len = u32::from_be_bytes(rest[..4].try_into().unwrap()) as usize;
        let kind: [u8; 4] = rest[4..8].try_into().unwrap();
        let data = rest[8..8 + len].to_vec();
        let crc = u32::from_be_bytes(rest[8 + len..12 + len].try_into().unwrap());
        out.push(Chunk { kind, data, crc });
        rest = &rest[12 + len..];
    }
    out
}

#[test]
fn png_of_known_2x2_has_exact_container_and_raw_rows() {
    // 1 行目: 不透明の赤・完全に透明 ／ 2 行目: 半透明の緑・不透明の白（すでに乗算を戻した RGBA）。
    let rgba: Vec<u8> = [
        [255, 0, 0, 255],
        [0, 0, 0, 0],
        [0, 255, 0, 128],
        [255, 255, 255, 255],
    ]
    .concat();
    let png = png(&rgba, 2, 2);

    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "署名");
    let chunks = chunks(&png);
    let kinds: Vec<&[u8; 4]> = chunks.iter().map(|c| &c.kind).collect();
    assert_eq!(
        kinds,
        [b"IHDR", b"IDAT", b"IEND"],
        "チャンクは 3 つだけ・この順"
    );

    // 幅 2・高さ 2・深さ 8・カラータイプ 6（RGBA）・圧縮 0・フィルタ 0・インターレース 0。
    assert_eq!(
        chunks[0].data,
        [0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0],
        "ヘッダの 13 バイト"
    );
    for c in &chunks {
        let mut covered = c.kind.to_vec();
        covered.extend_from_slice(&c.data);
        assert_eq!(
            c.crc,
            areka_nar::crc32(&covered),
            "{} の CRC（種類＋中身）",
            String::from_utf8_lossy(&c.kind)
        );
    }
    assert!(chunks[2].data.is_empty(), "IEND は中身なし");
    assert_eq!(chunks[2].crc, 0xAE42_6082, "IEND の CRC は既知の定数");

    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&chunks[1].data)
        .expect("IDAT は zlib として伸長できる");
    let mut rows = vec![0u8];
    rows.extend_from_slice(&rgba[..8]);
    rows.push(0);
    rows.extend_from_slice(&rgba[8..]);
    assert_eq!(raw, rows, "伸長した IDAT＝各行「フィルタ 0＋RGBA」");
}

#[test]
fn png_of_flat_256x256_is_compressed() {
    let rgba = [12u8, 34, 56, 200].repeat(256 * 256);
    let png = png(&rgba, 256, 256);
    assert!(
        png.len() < rgba.len(),
        "同じ色が続く絵の PNG（{} バイト）は元の画素（{} バイト）より小さい",
        png.len(),
        rgba.len()
    );
}

/// ワークツリーの `target\` の下に置く一時ディレクトリ（OS の一時フォルダは使わない）。破棄で中身ごと消える。
struct TargetTemp(PathBuf);

impl TargetTemp {
    fn new(label: &str) -> Self {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-tmp")
            .join(format!("areka-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("target の下に一時ディレクトリを作れるはず");
        TargetTemp(dir)
    }
}

impl Drop for TargetTemp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// WIC で復号し、乗算していない 32 ビット RGBA へ変換した（幅, 高さ, 画素）を返す。
fn wic_decode_rgba(path: &Path) -> (u32, u32, Vec<u8>) {
    let factory = wic_factory().expect("WIC の工場");
    let decoder = factory
        .create_decoder_from_filename(
            &HSTRING::from(path.as_os_str()),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )
        .expect("WIC が PNG を開ける");
    let frame = decoder.frame(0).expect("先頭のフレーム");
    let converter = factory.create_format_converter().expect("変換器");
    converter
        .init(
            &frame.cast::<IWICBitmapSource>().expect("フレームは絵の源"),
            &GUID_WICPixelFormat32bppRGBA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .expect("乗算していない RGBA へ変換できる");
    let source: IWICBitmapSource = converter.cast().expect("変換器は絵の源");
    let (w, h) = source.get_size().expect("大きさ");
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    source
        .copy_pixels(None, w * 4, &mut pixels)
        .expect("画素を写せる");
    (w, h, pixels)
}

#[test]
fn png_reads_back_through_wic_with_same_size_and_pixels() {
    // SAFETY: このテストのスレッドで WIC に要る COM を MTA で初期化する（既初期化の S_FALSE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    // 幅 3・高さ 2（縦横の取り違えが分かるよう正方形にしない）。透明・半透明・不透明を混ぜる。
    let premultiplied: Vec<u8> = [
        bgra(0, 0, 255, 255),
        bgra(0, 0, 0, 0),
        bgra(32, 64, 96, 128),
        bgra(255, 255, 255, 255),
        bgra(0, 40, 0, 51),
        bgra(10, 20, 30, 255),
    ]
    .concat();
    let rgba = unpremultiply(&premultiplied);
    let dir = TargetTemp::new("dump-surface-image-wic");
    let file = dir.0.join("readback.png");
    std::fs::write(&file, png(&rgba, 3, 2)).expect("PNG を書ける");

    let (w, h, pixels) = wic_decode_rgba(&file);
    assert_eq!((w, h), (3, 2), "幅と高さ（要件 5.3）");
    assert_eq!(pixels, rgba, "WIC で読み戻した画素が書いた RGBA と一致");

    let kept = dir.0.clone();
    drop(dir);
    assert!(!kept.exists(), "一時ファイルはテストの後に消える");
}

#[test]
fn base64_known_vectors() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    // 標準の文字集合の 62・63 番＝`+`・`/`（URL 用の `-`・`_` ではない）。
    assert_eq!(base64(&[0xFB, 0xEF, 0xBE]), "++++");
    assert_eq!(base64(&[0xFF, 0xFF, 0xFF]), "////");
    assert_eq!(base64(&[0xFB, 0xFF]), "+/8=");
}

#[test]
fn png_base64_has_no_line_breaks_for_large_image() {
    let s = png_base64(&[1u8, 2, 3, 4].repeat(64 * 64), 64, 64);
    assert!(!s.contains('\n') && !s.contains('\r'), "改行なし");
    assert_eq!(s.len() % 4, 0, "詰めありで 4 の倍数");
    assert!(s.starts_with("iVBORw0KGgo"), "PNG の署名の base64 で始まる");
}
