//! com/d2d/mod.rs — D2D Ext トレイト群のテスト
//!
//! デバイス生成は既存テストの前例（GraphicsCore::new() を用いる
//! tests/graphics 配下のテストがヘッドレスで合格）に従う。
//! 描画コマンドは ID2D1CommandList に記録して閉じる。

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Imaging::*;
use windows::core::*;
use windows_numerics::Vector2;
use wintf::com::d2d::*;
use wintf::com::dwrite::*;
use wintf::com::wic::*;
use wintf::ecs::GraphicsCore;

use super::common::{test_asset_path, with_com_initialized};

fn make_core() -> GraphicsCore {
    GraphicsCore::new().expect("GraphicsCore::new")
}

fn red() -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }
}

fn unit_rect() -> D2D_RECT_F {
    D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 32.0,
        bottom: 32.0,
    }
}

// ============================================================
// D2D1CommandListExt
// ============================================================

#[test]
fn command_list_close_succeeds_once_then_fails() {
    let core = make_core();
    let dc = core.device_context().expect("device context");
    let cl = unsafe { dc.CreateCommandList() }.expect("command list");

    cl.close().expect("first close");
    assert!(
        cl.close().is_err(),
        "second close should fail (wrong state)"
    );
}

// ============================================================
// D2D1DeviceContextExt — 描画ラッパー一式を CommandList へ記録して閉じる統合テスト
// ============================================================

#[test]
fn device_context_wrappers_record_into_command_list() {
    with_com_initialized(|| {
        let core = make_core();
        let device = core.d2d_device().expect("device");
        let dwrite = core.dwrite_factory().expect("dwrite factory");

        let dc = device
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("dc");
        let cl = unsafe { dc.CreateCommandList() }.expect("command list");
        unsafe { dc.SetTarget(&cl) };

        // DC と同一ファクトリからジオメトリを生成（factory 不一致を回避）
        let dc_factory: ID2D1Factory = unsafe { dc.GetFactory() }.expect("GetFactory");
        let rounded = D2D1_ROUNDED_RECT {
            rect: unit_rect(),
            radiusX: 4.0,
            radiusY: 4.0,
        };
        let geometry =
            unsafe { dc_factory.CreateRoundedRectangleGeometry(&rounded) }.expect("geometry");

        // WIC 画像から D2D ビットマップを生成
        let bitmap = load_test_bitmap(&dc);

        // DWrite フォーマット/レイアウト
        let format = dwrite
            .create_text_format(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                16.0,
                w!("en-us"),
            )
            .expect("text format");
        let layout = dwrite
            .create_text_layout(w!("Layout"), &format, 200.0, 50.0)
            .expect("text layout");

        unsafe { dc.BeginDraw() };

        // --- 各ラッパーで記録 ---
        let identity = windows_numerics::Matrix3x2::identity();
        dc.set_transform(&identity);
        dc.clear(Some(&red()));
        dc.clear(None);

        let brush = dc
            .create_solid_color_brush(&red(), None)
            .expect("create_solid_color_brush");

        dc.fill_rectangle(&unit_rect(), &brush);
        dc.fill_ellipse(
            &D2D1_ELLIPSE {
                point: Vector2 { X: 16.0, Y: 16.0 },
                radiusX: 8.0,
                radiusY: 8.0,
            },
            &brush,
        );
        unsafe { dc.FillGeometry(&geometry, &brush, None) };
        dc.draw_bitmap(
            &bitmap,
            Some(&unit_rect()),
            1.0,
            D2D1_INTERPOLATION_MODE_LINEAR,
            None,
            None,
        );
        unsafe {
            dc.DrawText(
                &HSTRING::from("Hi"),
                &format,
                &unit_rect(),
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            )
        };
        dc.draw_text_layout(
            Vector2 { X: 0.0, Y: 0.0 },
            &layout,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );

        // DrawImage: 閉じた空のコマンドリストは有効な ID2D1Image
        let empty_cl = unsafe { dc.CreateCommandList() }.expect("empty cl");
        empty_cl.close().expect("close empty cl");
        unsafe {
            dc.DrawImage(
                &empty_cl,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            )
        };

        unsafe { dc.EndDraw(None, None) }.expect("EndDraw");
        cl.close().expect("close");
    });
}

/// WIC ソース（PBGRA32 変換済み）から D2D ビットマップを生成する。
fn load_test_bitmap(dc: &ID2D1DeviceContext) -> ID2D1Bitmap1 {
    let factory = wic_factory().expect("wic_factory");
    let path_h = HSTRING::from(test_asset_path("test_8x8_rgba.png").as_os_str());
    let decoder = factory
        .create_decoder_from_filename(
            &path_h,
            None,
            windows::Win32::Foundation::GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )
        .expect("decoder");
    let frame = decoder.frame(0).expect("frame");
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

    unsafe { dc.CreateBitmapFromWicBitmap(&converter, None) }.expect("CreateBitmapFromWicBitmap")
}
