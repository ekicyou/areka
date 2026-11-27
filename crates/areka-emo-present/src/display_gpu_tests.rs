//! 閉じたコマンドリストを実 GPU で再生して読み戻すテストの支え。

use std::mem::ManuallyDrop;

use windows::Win32::Graphics::Direct2D::Common::{
    D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_COMPOSITE_MODE_SOURCE_OVER,
    D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ,
    D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_INTERPOLATION_MODE_LINEAR,
    D2D1_MAP_OPTIONS_READ, ID2D1Bitmap1, ID2D1ColorContext, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows_numerics::Matrix3x2;
use wintf::com::d2d::D2D1DeviceContextExt;
use wintf::ecs::GraphicsCommandList;

/// `w×h`・premultiplied BGRA の bitmap を作る（`options` で TARGET／staging を切り替える）。
fn make_bitmap(
    dc: &ID2D1DeviceContext,
    w: u32,
    h: u32,
    options: D2D1_BITMAP_OPTIONS,
) -> ID2D1Bitmap1 {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: options,
        colorContext: ManuallyDrop::new(None::<ID2D1ColorContext>),
    };
    // SAFETY: Windows API 境界。初期バイト列を渡さない空 bitmap の生成。
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: w,
                height: h,
            },
            None,
            0,
            &props,
        )
    }
    .expect("オフスクリーン bitmap の生成に失敗")
}

/// 閉じたコマンドリストを恒等変換で `w×h` のオフスクリーンへ再生し、行を詰めて読み戻す。
///
/// 再生の引数は本番 `render_surface`（wintf `ecs/graphics/systems/render.rs`）と同一
/// （透明クリア → `DrawImage(list, None, None, LINEAR, SOURCE_OVER)`）。
pub(crate) fn replay_and_read_back(
    dc: &ID2D1DeviceContext,
    list: &GraphicsCommandList,
    w: u32,
    h: u32,
) -> Vec<u8> {
    let target = make_bitmap(dc, w, h, D2D1_BITMAP_OPTIONS_TARGET);
    // SAFETY: Windows API 境界。ターゲット差し替えと描画（本スレッド直列）。
    unsafe { dc.SetTarget(&target) };
    dc.set_transform(&Matrix3x2::identity());
    // SAFETY: Windows API 境界。`BeginDraw`／`EndDraw` は本関数内で対になる。
    unsafe { dc.BeginDraw() };
    dc.clear(Some(&D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }));
    // SAFETY: Windows API 境界。`render_surface` と同一引数のコマンドリスト再生。
    unsafe {
        dc.DrawImage(
            list.command_list().expect("閉じたリストを持つ"),
            None,
            None,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
        );
        dc.EndDraw(None, None).expect("EndDraw（再生）失敗");
    }

    let staging = make_bitmap(
        dc,
        w,
        h,
        D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
    );
    let row = (w as usize) * 4;
    let mut out = vec![0u8; row * (h as usize)];
    // SAFETY: Windows API 境界。`Map(READ)` が返す `bits` は少なくとも `pitch * h` バイトの
    // 読み取り可能領域を指す。行ごとに `pitch` を跨いで詰め直す。
    unsafe {
        staging
            .CopyFromBitmap(None, &target, None)
            .expect("CopyFromBitmap（TARGET → staging）失敗");
        let mapped = staging.Map(D2D1_MAP_OPTIONS_READ).expect("Map(READ) 失敗");
        for y in 0..(h as usize) {
            std::ptr::copy_nonoverlapping(
                mapped.bits.add(y * mapped.pitch as usize),
                out.as_mut_ptr().add(y * row),
                row,
            );
        }
        staging.Unmap().expect("Unmap 失敗");
        // 共有 DC に自前ターゲットを残さない。
        dc.SetTarget(None);
    }
    out
}
