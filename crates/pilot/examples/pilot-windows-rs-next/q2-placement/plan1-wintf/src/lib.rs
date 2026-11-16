// 問い 2 の案 1 の wintf 役: 自分の束縛（wintf 役のフィルターだけ）で、ハードウェアの D3D11 → D2D の装置と文脈 → 描く先と CPU で読む絵を作り、読み戻す。
#[allow(warnings)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
// 自分の束縛を公開する（emo-text 役は Target.context の型をここから名指しする）
pub use bindings::*;
use windows_core::*;

/// 描く先を付けた文脈と、読み戻しに使う絵
pub struct Target {
    pub context: ID2D1DeviceContext,
    bitmap: ID2D1Bitmap1,
    readback: ID2D1Bitmap1,
    width: u32,
    height: u32,
}

pub fn create_target(width: u32, height: u32) -> Result<Target> {
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
    let context = unsafe { device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)? };
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
    let size = D2D_SIZE_U { width, height };
    let bitmap =
        unsafe { context.CreateBitmap(size, None, 0, &props(D2D1_BITMAP_OPTIONS_TARGET))? };
    let readback = unsafe {
        context.CreateBitmap(
            size,
            None,
            0,
            &props(D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW),
        )?
    };
    unsafe { context.SetTarget(&bitmap) };
    Ok(Target {
        context,
        bitmap,
        readback,
        width,
        height,
    })
}

/// 描く先を CPU で読む絵へ写して、premultiplied BGRA（行の詰め無し）で返す
pub fn read_back(target: &Target) -> Result<Vec<u8>> {
    let rb = &target.readback;
    unsafe { rb.CopyFromBitmap(None, &target.bitmap, None).ok()? };
    let mut mapped = D2D1_MAPPED_RECT::default();
    unsafe { rb.Map(D2D1_MAP_OPTIONS_READ, &mut mapped).ok()? };
    let row = target.width as usize * 4;
    let mut out = Vec::with_capacity(row * target.height as usize);
    for y in 0..target.height as usize {
        out.extend_from_slice(unsafe {
            core::slice::from_raw_parts(mapped.bits.add(y * mapped.pitch as usize), row)
        });
    }
    unsafe { rb.Unmap().ok()? };
    Ok(out)
}
