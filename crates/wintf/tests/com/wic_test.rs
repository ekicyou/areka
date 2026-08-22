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
fn wic_factory_can_be_created() {
    with_com_initialized(|| {
        wic_factory().expect("wic_factory should succeed under COM apartment");
    });
}

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

#[test]
fn copy_pixels_whole_image_fills_buffer() {
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

        // 8x8 PBGRA32: stride = 8*4 = 32, total = 256 bytes
        let mut buffer = vec![0u8; 256];
        converted
            .copy_pixels(None, 32, &mut buffer)
            .expect("copy_pixels whole image");
    });
}

#[test]
fn copy_pixels_with_subrect_succeeds() {
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

        let rect = WICRect {
            X: 0,
            Y: 0,
            Width: 2,
            Height: 2,
        };
        let mut buffer = vec![0u8; 16]; // 2x2 * 4 bytes
        converted
            .copy_pixels(Some(&rect), 8, &mut buffer)
            .expect("copy_pixels subrect");
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
