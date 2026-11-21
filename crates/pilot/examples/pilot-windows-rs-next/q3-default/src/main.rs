// 問い 3（既定の形）の確かめの bin: 写した dwrite.rs・d2d/mod.rs の口で文字を描いて墨を数え、
// COM の型の `==` が同一性（IUnknown を通った別の参照で真・別の物で偽）を保つかを assert する。
// 写しの create_text_format は元のまま 8 引数（-D warnings で止まるので根で許す）
#![allow(clippy::too_many_arguments)]

mod bindings {
    #![allow(warnings)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
mod d2d;
mod dwrite;

use bindings::*;
use d2d::*;
use dwrite::*;
use windows_core::*;
use windows_numerics::Vector2;

const W: u32 = 256;
const H: u32 = 64;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[q3-default] error={e:?}");
            std::process::ExitCode::from(2)
        }
    }
}

fn run() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED as u32).ok()? };

    // ハードウェアの D3D11（D3D_DRIVER_TYPE_HARDWARE）→ D2D の装置と文脈
    let mut d3d = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT as u32,
            None,
            0,
            D3D11_SDK_VERSION as u32,
            Some(&mut d3d),
            None,
            None,
        )
        .ok()?
    };
    let d3d = d3d.ok_or_else(|| Error::new(E_POINTER, "D3D11CreateDevice returned no device"))?;
    let dxgi: IDXGIDevice = d3d.cast()?;
    let mut raw = core::ptr::null_mut();
    let factory = unsafe {
        D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &ID2D1Factory1::IID,
            None,
            &mut raw,
        )
        .ok()?;
        ID2D1Factory1::from_raw(raw)
    };
    let device = unsafe { factory.CreateDevice(&dxgi)? };
    let ctx = device.create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

    // 画面の外の描く先と CPU で読む絵
    let props = |options| D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: options,
        colorContext: core::mem::ManuallyDrop::new(None),
    };
    let size = D2D_SIZE_U {
        width: W,
        height: H,
    };
    let target = unsafe { ctx.CreateBitmap(size, None, 0, &props(D2D1_BITMAP_OPTIONS_TARGET))? };
    let readback = unsafe {
        ctx.CreateBitmap(
            size,
            None,
            0,
            &props(D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW),
        )?
    };

    // DirectWrite の工場・書式・配置（写した口で）
    let dw = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)?;
    let format = dw.create_text_format(
        w!("Segoe UI"),
        None,
        DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL,
        20.0,
        w!("ja-jp"),
    )?;
    let layout = dw.create_text_layout(w!("q3 default 墨"), &format, W as f32, H as f32)?;
    let clusters = layout.get_cluster_metrics()?.len();
    let overhang = layout.get_overhang_metrics()?;

    unsafe { ctx.SetTarget(&target) };
    let black = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let brush = ctx.create_solid_color_brush(&black, None)?;
    unsafe { ctx.BeginDraw() };
    ctx.set_transform(&windows_numerics::Matrix3x2::identity());
    ctx.clear(Some(&D2D1_COLOR_F::default()));
    ctx.draw_text_layout(
        Vector2 { x: 4.0, y: 4.0 },
        &layout,
        &brush,
        D2D1_DRAW_TEXT_OPTIONS_NONE,
    );
    unsafe { ctx.EndDraw(None, None).ok()? };

    // 読み戻して墨（α>0）の画素を数える
    unsafe { readback.CopyFromBitmap(None, &target, None).ok()? };
    let mut mapped = D2D1_MAPPED_RECT::default();
    unsafe { readback.Map(D2D1_MAP_OPTIONS_READ, &mut mapped).ok()? };
    let mut ink = 0usize;
    for y in 0..H as usize {
        let row = unsafe {
            core::slice::from_raw_parts(mapped.bits.add(y * mapped.pitch as usize), W as usize * 4)
        };
        ink += row.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count();
    }
    unsafe { readback.Unmap().ok()? };
    println!(
        "[q3-default] clusters={clusters} overhang_right={} ink={ink}",
        overhang.right
    );
    if ink == 0 {
        return Err(Error::new(E_FAIL, "no ink pixels"));
    }

    // `==` の確かめ（ID2D1CommandList で）。残りの写した口（塗り・絵）は命令の列へ記録して通す
    let list = unsafe { ctx.CreateCommandList()? };
    unsafe { ctx.SetTarget(&list) };
    unsafe { ctx.BeginDraw() };
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 8.0,
        bottom: 8.0,
    };
    ctx.fill_rectangle(&rect, &brush);
    let ellipse = D2D1_ELLIPSE {
        point: Vector2 { x: 4.0, y: 4.0 },
        radiusX: 2.0,
        radiusY: 2.0,
    };
    ctx.fill_ellipse(&ellipse, &brush);
    ctx.draw_bitmap(
        &target,
        Some(&rect),
        1.0,
        D2D1_INTERPOLATION_MODE_LINEAR,
        None,
        None,
    );
    unsafe { ctx.EndDraw(None, None).ok()? };
    list.close()?;
    let other = unsafe { ctx.CreateCommandList()? };
    let via_unknown: IUnknown = list.cast()?;
    let back: ID2D1CommandList = via_unknown.cast()?;
    let upcast: IUnknown = list.clone().into(); // QueryInterface を通らない参照
    let same_via_unknown = list == back;
    let same_unknown = via_unknown == upcast;
    let differ = list != other;
    println!(
        "[q3-default] eq same_via_unknown={same_via_unknown} same_unknown={same_unknown} \
         different_object_ne={differ} raw_list={:?} raw_back={:?} raw_unknown={:?} raw_upcast={:?}",
        list.as_raw(),
        back.as_raw(),
        via_unknown.as_raw(),
        upcast.as_raw()
    );
    assert!(
        same_via_unknown,
        "== must be true for the same object via IUnknown"
    );
    assert!(
        same_unknown,
        "== must be true for IUnknown of the same object"
    );
    assert!(differ, "== must be false for a different object");

    // 番地が違う参照でも同じ物なら真か（D3D11 の装置は ID3D11Device と IDXGIDevice で番地が違う）
    let d3d_unknown: IUnknown = d3d.clone().into();
    let dxgi_unknown: IUnknown = dxgi.clone().into();
    let same_object_other_address = d3d_unknown == dxgi_unknown;
    println!(
        "[q3-default] eq same_object_other_address={same_object_other_address} raw_d3d={:?} raw_dxgi={:?}",
        d3d_unknown.as_raw(),
        dxgi_unknown.as_raw()
    );
    assert!(
        same_object_other_address,
        "== must follow COM identity, not the address"
    );
    Ok(())
}
