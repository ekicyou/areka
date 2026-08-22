//! com/d2d/mod.rs — D2D Ext トレイト群のテスト
//!
//! デバイス生成は既存テストの前例（GraphicsCore::new() を用いる
//! tests/graphics 配下のテストがヘッドレスで合格）に従う。
//! 描画コマンドは ID2D1CommandList に記録し、ID2D1CommandList::Stream で
//! RecCommandSink へ再生することで、COM オブジェクト引数を要する
//! command_sink.rs のコールバック（DrawGlyphRun のスライス複製等）も
//! 実データで検証する。

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
// D2D1FactoryExt
// ============================================================

#[test]
fn factory_creates_path_and_rounded_rectangle_geometry() {
    let core = make_core();
    let factory = core.d2d_factory().expect("factory");

    factory
        .create_path_geometry()
        .expect("create_path_geometry");

    let rounded = D2D1_ROUNDED_RECT {
        rect: unit_rect(),
        radiusX: 4.0,
        radiusY: 4.0,
    };
    factory
        .create_rounded_rectangle_geometry(&rounded)
        .expect("create_rounded_rectangle_geometry");
}

// ============================================================
// D2D1DeviceExt
// ============================================================

#[test]
fn device_creates_device_context_and_command_list() {
    let core = make_core();
    let device = core.d2d_device().expect("device");

    device
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("create_device_context");
    device.create_command_list().expect("create_command_list");
}

// ============================================================
// D2D1CommandListExt
// ============================================================

/// open() は常に E_NOTIMPL を返す（純粋ロジックの特性化:
/// ID2D1CommandList は直接 Open できないという設計判断の固定）。
#[test]
fn command_list_open_always_returns_e_notimpl() {
    let core = make_core();
    let device = core.d2d_device().expect("device");
    let cl = device.create_command_list().expect("command list");

    let result = cl.open();
    assert_eq!(
        result.expect_err("open should fail").code(),
        windows::Win32::Foundation::E_NOTIMPL
    );
}

#[test]
fn command_list_close_succeeds_once_then_fails() {
    let core = make_core();
    let device = core.d2d_device().expect("device");
    let cl = device.create_command_list().expect("command list");

    cl.close().expect("first close");
    assert!(
        cl.close().is_err(),
        "second close should fail (wrong state)"
    );
}

// ============================================================
// D2D1DeviceContextExt — 描画ラッパー一式を CommandList へ記録し、
// RecCommandSink へ Stream 再生する統合テスト
// ============================================================

#[test]
fn device_context_wrappers_record_and_stream_into_rec_command_sink() {
    with_com_initialized(|| {
        let core = make_core();
        let device = core.d2d_device().expect("device");
        let dwrite = core.dwrite_factory().expect("dwrite factory");

        let dc = device
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("dc");
        let cl = device.create_command_list().expect("command list");
        unsafe { dc.SetTarget(&cl) };

        // DC と同一ファクトリからジオメトリを生成（factory 不一致を回避）
        let dc_factory: ID2D1Factory = unsafe { dc.GetFactory() }.expect("GetFactory");
        let rounded = D2D1_ROUNDED_RECT {
            rect: unit_rect(),
            radiusX: 4.0,
            radiusY: 4.0,
        };
        let geometry = dc_factory
            .create_rounded_rectangle_geometry(&rounded)
            .expect("geometry");

        // WIC 画像から D2D ビットマップを生成（create_bitmap_from_wic_bitmap）
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
        dc.fill_geometry(&geometry, &brush);
        dc.draw_bitmap(
            &bitmap,
            Some(&unit_rect()),
            1.0,
            D2D1_INTERPOLATION_MODE_LINEAR,
            None,
            None,
        );
        dc.draw_text(
            &HSTRING::from("Hi"),
            &format,
            &unit_rect(),
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );
        dc.draw_text_layout(
            Vector2 { X: 0.0, Y: 0.0 },
            &layout,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );

        // draw_image: 閉じた空のコマンドリストは有効な ID2D1Image
        let empty_cl = device.create_command_list().expect("empty cl");
        empty_cl.close().expect("close empty cl");
        dc.draw_image(&empty_cl);

        unsafe { dc.EndDraw(None, None) }.expect("EndDraw");
        cl.close().expect("close");

        // --- RecCommandSink へ再生: DrawGlyphRun（テキスト）/ DrawBitmap /
        // FillGeometry 等の COM 引数コールバックを実データで通す ---
        let sink: ID2D1CommandSink5 = RecCommandSink::new().into();
        unsafe { cl.Stream(&sink) }.expect("Stream into RecCommandSink");
    });
}

/// draw_text の `hstring_borrow.as_ref().unwrap()` は空 HSTRING
/// （内部表現 null の HSTRING::new() / HSTRING::from("")）でも panic しない
/// ことの特性化（W2-S からの申し送り）。
///
/// 根拠: windows-core 0.62.2 では HSTRING は CloneType であり
/// `Type<T, CloneType>::is_null()` が常に false を返すため、
/// `Ref<HSTRING>::as_ref()` は None になり得ない。空 HSTRING は
/// Deref で空スライス &[] として DrawText へ渡り、記録・Close も成功する。
#[test]
fn draw_text_with_empty_hstring_does_not_panic() {
    let core = make_core();
    let device = core.d2d_device().expect("device");
    let dwrite = core.dwrite_factory().expect("dwrite factory");

    let dc = device
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("dc");
    let cl = device.create_command_list().expect("command list");
    unsafe { dc.SetTarget(&cl) };

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

    unsafe { dc.BeginDraw() };
    let brush = dc
        .create_solid_color_brush(&red(), None)
        .expect("create_solid_color_brush");

    // HSTRING::new() と HSTRING::from("") はいずれも内部表現 null（空文字列）
    dc.draw_text(
        &HSTRING::new(),
        &format,
        &unit_rect(),
        &brush,
        D2D1_DRAW_TEXT_OPTIONS_NONE,
        DWRITE_MEASURING_MODE_NATURAL,
    );
    dc.draw_text(
        &HSTRING::from(""),
        &format,
        &unit_rect(),
        &brush,
        D2D1_DRAW_TEXT_OPTIONS_NONE,
        DWRITE_MEASURING_MODE_NATURAL,
    );

    unsafe { dc.EndDraw(None, None) }.expect("EndDraw");
    cl.close().expect("close");
}

/// create_bitmap_from_wic_bitmap: WIC ソース（PBGRA32 変換済み）から
/// D2D ビットマップを生成する。
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

    dc.create_bitmap_from_wic_bitmap(&converter.cast::<IWICBitmapSource>().expect("cast"))
        .expect("create_bitmap_from_wic_bitmap")
}

#[test]
fn create_bitmap_from_wic_bitmap_preserves_size() {
    with_com_initialized(|| {
        let core = make_core();
        let dc = core.device_context().expect("dc");
        let bitmap = load_test_bitmap(dc);
        let size = unsafe { bitmap.GetPixelSize() };
        assert_eq!((size.width, size.height), (8, 8));
    });
}

// ============================================================
// command_types::dup_com の特性化（実 COM オブジェクト使用）
// ============================================================

/// dup_com は参照カウントを増やさず ManuallyDrop で包むため、
/// コマンド構造体の drop で元の COM オブジェクトは解放されない
/// （drop 後も元のブラシが使用可能であることを検証）。
#[test]
fn command_struct_drop_does_not_release_original_com_object() {
    let core = make_core();
    let dc = core.device_context().expect("dc");
    let brush = dc.create_solid_color_brush(&red(), None).expect("brush");
    let brush_base: ID2D1Brush = brush.cast().expect("cast to ID2D1Brush");

    let cmd = FillRectangle::new(unit_rect(), &brush_base);
    drop(cmd);

    // コマンド drop 後も元ブラシは生存している（解放されていれば UB/クラッシュ）
    unsafe {
        brush.SetOpacity(0.5);
        assert_eq!(brush.GetOpacity(), 0.5);
    }
}

/// Option<&T> を取るコンストラクタは None をそのまま None として保持する。
#[test]
fn draw_line_with_none_stroke_style_stores_none() {
    let core = make_core();
    let dc = core.device_context().expect("dc");
    let brush = dc.create_solid_color_brush(&red(), None).expect("brush");
    let brush_base: ID2D1Brush = brush.cast().expect("cast");

    let cmd = DrawLine::new(
        Vector2 { X: 0.0, Y: 0.0 },
        Vector2 { X: 10.0, Y: 10.0 },
        &brush_base,
        2.0,
        None,
    );
    assert!(cmd.strokestyle.is_none());
    assert_eq!(cmd.strokewidth, 2.0);
    assert_eq!(cmd.point1.X, 10.0);
}
