//! com/wic.rs — WIC ラッパーのヘッドレステスト
//!
//! WIC は CPU のみで動作するため GPU デバイス不要。CoInitializeEx の
//! 既存パターン（tests/widget/bitmap_source_integration_test.rs）に従う。
//! 既存の widget テストは上位 API（load_bitmap_source）経由のため、
//! 本テストは com::wic の各 Ext トレイトを直接検証する。

use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Imaging::D2D::IWICImagingFactory2;
use windows::Win32::Graphics::Imaging::*;
use windows::core::*;
use wintf::com::wic::*;

use super::common::{test_asset_path, with_com_initialized};

fn decode_first_frame(filename: &str) -> (IWICImagingFactory2, IWICBitmapFrameDecode) {
    let factory = wic_factory().expect("wic_factory");
    let path = test_asset_path(filename);
    let path_h = HSTRING::from(path.as_os_str());
    let decoder = factory
        .create_decoder_from_filename(&path_h, None, GENERIC_READ, WICDecodeMetadataCacheOnDemand)
        .expect("create_decoder_from_filename");
    let frame = decoder.frame(0).expect("frame(0)");
    (factory, frame)
}

// ============================================================
// ファクトリ / デコーダ / フレーム
// ============================================================

#[test]
fn decode_png_frame_and_get_size() {
    with_com_initialized(|| {
        let (_factory, frame) = decode_first_frame("test_8x8_rgba.png");
        let source: IWICBitmapSource = frame.cast().expect("frame → IWICBitmapSource");
        let (width, height) = source.get_size().expect("get_size");
        assert_eq!((width, height), (8, 8));
    });
}

#[test]
fn create_decoder_from_nonexistent_file_fails() {
    with_com_initialized(|| {
        let factory = wic_factory().expect("wic_factory");
        let path_h = HSTRING::from(test_asset_path("does_not_exist.png").as_os_str());
        let result = factory.create_decoder_from_filename(
            &path_h,
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        );
        assert!(result.is_err(), "missing file should fail");
    });
}

#[test]
fn create_decoder_from_invalid_content_fails() {
    with_com_initialized(|| {
        let factory = wic_factory().expect("wic_factory");
        let path_h = HSTRING::from(test_asset_path("invalid.bin").as_os_str());
        let result = factory.create_decoder_from_filename(
            &path_h,
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        );
        assert!(result.is_err(), "non-image content should fail");
    });
}

// ============================================================
// フォーマットコンバータ
// ============================================================

#[test]
fn format_converter_init_to_pbgra32_succeeds() {
    with_com_initialized(|| {
        let (factory, frame) = decode_first_frame("test_8x8_rgba.png");
        let source: IWICBitmapSource = frame.cast().expect("cast");

        let converter = factory.create_format_converter().expect("converter");
        converter
            .init(
                &source,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .expect("converter init");

        // 変換後もサイズは保存される
        let converted: IWICBitmapSource = converter.cast().expect("cast");
        assert_eq!(converted.get_size().expect("get_size"), (8, 8));
    });
}

// ============================================================
// copy_pixels
// ============================================================

/// 8x8 の検体を PBGRA32 に変えて全体を写す（stride = 8*4 = 32、計 256 バイト）。
fn copy_whole_pbgra(factory: &IWICImagingFactory2, frame: &IWICBitmapFrameDecode) -> Vec<u8> {
    let source: IWICBitmapSource = frame.cast().expect("cast");
    let converter = factory.create_format_converter().expect("converter");
    converter
        .init(
            &source,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .expect("init");
    let converted: IWICBitmapSource = converter.cast().expect("cast");
    let mut buffer = vec![0u8; 256];
    converted
        .copy_pixels(None, 32, &mut buffer)
        .expect("copy_pixels whole image");
    buffer
}

/// 入れ物は 0 で始めるので、写さない包みなら 0 のままになる。
#[test]
fn copy_pixels_whole_image_fills_buffer() {
    with_com_initialized(|| {
        let (factory, frame) = decode_first_frame("test_8x8_rgba.png");
        let buffer = copy_whole_pbgra(&factory, &frame);
        assert!(
            buffer.iter().any(|&b| b != 0),
            "copy_pixels should write the image into the buffer"
        );
    });
}

/// 部分の写しは、全体の写しの同じ画素と一致する。
#[test]
fn copy_pixels_with_subrect_matches_whole_copy() {
    with_com_initialized(|| {
        let (factory, frame) = decode_first_frame("test_8x8_rgba.png");
        let whole = copy_whole_pbgra(&factory, &frame);

        let source: IWICBitmapSource = frame.cast().expect("cast");
        let converter = factory.create_format_converter().expect("converter");
        converter
            .init(
                &source,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .expect("init");
        let converted: IWICBitmapSource = converter.cast().expect("cast");

        let (x, y, w, h) = (1usize, 1usize, 2usize, 2usize);
        let rect = WICRect {
            X: x as i32,
            Y: y as i32,
            Width: w as i32,
            Height: h as i32,
        };
        let mut buffer = vec![0u8; w * h * 4];
        converted
            .copy_pixels(Some(&rect), (w * 4) as u32, &mut buffer)
            .expect("copy_pixels subrect");

        let expected: Vec<u8> = (y..y + h)
            .flat_map(|row| {
                whole[(row * 8 + x) * 4..(row * 8 + x + w) * 4]
                    .iter()
                    .copied()
            })
            .collect();
        assert!(
            expected.iter().any(|&b| b != 0),
            "sample area must not be blank"
        );
        assert_eq!(buffer, expected);
    });
}

/// バッファ不足はラッパーがエラーを伝播する（境界検査は WIC 側）。
#[test]
fn copy_pixels_with_undersized_buffer_fails() {
    with_com_initialized(|| {
        let (factory, frame) = decode_first_frame("test_8x8_rgba.png");
        let source: IWICBitmapSource = frame.cast().expect("cast");
        let converter = factory.create_format_converter().expect("converter");
        converter
            .init(
                &source,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .expect("init");
        let converted: IWICBitmapSource = converter.cast().expect("cast");

        let mut buffer = vec![0u8; 16]; // 256 必要なところを 16 のみ
        let result = converted.copy_pixels(None, 32, &mut buffer);
        assert!(result.is_err(), "undersized buffer should fail");
    });
}

// ============================================================
// load_bitmap_source の α の有無（番号つきの形式）
// ============================================================

/// 検体は emo-atlas と共有する（作り手 `crates/areka-emo-atlas/examples/gen_palette_samples.rs`、
/// 表と作り直しの手順は `crates/areka-emo-atlas/src/testdata/palette/README.md`）。
/// crate の依存は足さず、ファイルを相対パスで読むだけ。
fn palette_has_alpha_of(filename: &str) -> bool {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../areka-emo-atlas/src/testdata/palette")
        .join(filename);
    with_com_initialized(|| {
        let factory = wic_factory().expect("wic_factory");
        let (_source, has_alpha) = load_bitmap_source(&factory, &path).expect("load_bitmap_source");
        has_alpha
    })
}

#[test]
#[cfg_attr(
    not(areka_palette_samples),
    ignore = "areka-emo-atlas のパレットの見本が無い（ワークスペースの外）"
)]
fn palette_idx8_mixed_has_alpha() {
    assert!(palette_has_alpha_of("idx8_mixed.png"));
}

#[test]
#[cfg_attr(
    not(areka_palette_samples),
    ignore = "areka-emo-atlas のパレットの見本が無い（ワークスペースの外）"
)]
fn palette_idx4_mixed_has_alpha() {
    assert!(palette_has_alpha_of("idx4_mixed.png"));
}

#[test]
#[cfg_attr(
    not(areka_palette_samples),
    ignore = "areka-emo-atlas のパレットの見本が無い（ワークスペースの外）"
)]
fn palette_gray8_trns_has_alpha() {
    assert!(palette_has_alpha_of("gray8_trns.png"));
}

#[test]
#[cfg_attr(
    not(areka_palette_samples),
    ignore = "areka-emo-atlas のパレットの見本が無い（ワークスペースの外）"
)]
fn palette_trans_gif_has_alpha() {
    assert!(palette_has_alpha_of("trans.gif"));
}

#[test]
#[cfg_attr(
    not(areka_palette_samples),
    ignore = "areka-emo-atlas のパレットの見本が無い（ワークスペースの外）"
)]
fn palette_idx8_plain_has_no_alpha() {
    assert!(!palette_has_alpha_of("idx8_plain.png"));
}

#[test]
#[cfg_attr(
    not(areka_palette_samples),
    ignore = "areka-emo-atlas のパレットの見本が無い（ワークスペースの外）"
)]
fn palette_idx8_opaque_trns_has_no_alpha() {
    assert!(!palette_has_alpha_of("idx8_opaque_trns.png"));
}
