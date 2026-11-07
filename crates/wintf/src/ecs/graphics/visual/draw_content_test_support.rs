//! テストで節の描く中身を作る道具（要件 6.7）。実の GPU も D3D の装置も使わず、
//! 工場から作る WIC のビットマップの的（CPU の描き手）を `ID2D1DeviceContext` として使う。
//! 画素を読み戻す WARP の足場（`draw/emit_test_support.rs`）とは別。

use super::VisualDrawContent;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;
use windows::core::Interface;

/// 単色の矩形 `rect` を 1 つ描いて閉じた命令の列を持つ描く中身を作る。
/// `local_aabb` は塗った `rect` そのもの。
pub(crate) fn solid_rect_content(rect: D2D_RECT_F) -> VisualDrawContent {
    // テストはスレッドごとに走るので呼ぶたびに初期化する（同じスレッドの 2 度目は S_FALSE）。
    // 既に MTA でも WIC はどちらでも動く。解かないのは返す COM の物がこの後も生きるため。
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    assert!(
        hr.is_ok() || hr == RPC_E_CHANGED_MODE,
        "CoInitializeEx: {hr:?}"
    );
    let wic = crate::com::wic::wic_factory().unwrap();
    let bitmap = unsafe {
        wic.CreateBitmap(
            16,
            16,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapCacheOnDemand,
        )
    }
    .unwrap();
    let factory: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }.unwrap();
    let props = D2D1_RENDER_TARGET_PROPERTIES {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        ..Default::default()
    };
    let dc: ID2D1DeviceContext = unsafe { factory.CreateWicBitmapRenderTarget(&bitmap, &props) }
        .unwrap()
        .cast()
        .unwrap();
    let list = unsafe { dc.CreateCommandList() }.unwrap();
    unsafe {
        dc.SetTarget(&list);
        dc.BeginDraw();
        let brush = dc
            .CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                None,
            )
            .unwrap();
        dc.FillRectangle(&rect, &brush);
        dc.EndDraw(None, None).unwrap();
        dc.SetTarget(None);
        list.Close().unwrap();
    }
    VisualDrawContent::new(rect.into(), list)
}
