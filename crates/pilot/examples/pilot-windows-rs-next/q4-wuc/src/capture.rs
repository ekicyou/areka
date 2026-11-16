//! 窓の矩形を画面の DC から `BitBlt` で撮り、PNG に書く（7.1・新しく書いた物）。
//!
//! 画面の DC から撮るので、DWM が合成した後の画素（WUC の中身を含む見込み）が写る。

use bind_next::Windows::Win32::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, GetWindowRect, HBITMAP, HDC, HWND,
    RECT, ReleaseDC, SRCCOPY, SelectObject,
};
use windows_core::{Error, HRESULT, Result};

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);

/// GDI は最後のエラーを必ずは立てないので、立っていなければ E_FAIL と呼び出しの名前で返す。
fn fail(what: &str) -> Error {
    let e = Error::from_thread();
    if e.code().is_ok() {
        Error::new(E_FAIL, what)
    } else {
        e
    }
}

/// `hwnd` の窓の矩形（画面の座標）を撮り、`path` へ RGBA の PNG で書く。
// この rev の HWND は裸のポインタの別名なので clippy が咎める。取っ手は OS へ渡すだけで参照しない。
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn capture_window_png(hwnd: HWND, path: &std::path::Path) -> Result<()> {
    let mut r = RECT::default();
    if !unsafe { GetWindowRect(hwnd, &mut r) }.as_bool() {
        return Err(fail("GetWindowRect"));
    }
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w <= 0 || h <= 0 {
        return Err(Error::new(E_FAIL, "window rect is empty"));
    }
    let screen = unsafe { GetDC(None) };
    if screen.is_null() {
        return Err(fail("GetDC"));
    }
    let mem = unsafe { CreateCompatibleDC(Some(screen)) };
    let bmp = if mem.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { CreateCompatibleBitmap(screen, w, h) }
    };
    let pixels = if bmp.is_null() {
        Err(fail("CreateCompatibleDC/CreateCompatibleBitmap"))
    } else {
        read_pixels(screen, mem, bmp, r.left, r.top, w, h)
    };
    unsafe {
        if !bmp.is_null() {
            let _ = DeleteObject(bmp as _);
        }
        if !mem.is_null() {
            let _ = DeleteDC(mem);
        }
        ReleaseDC(None, screen);
    }
    let mut bgra = pixels?;
    // GDI の 32 bpp は BGRA の並び。PNG は RGBA。画面からの BitBlt は α を決めないので不透明にする。
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
        px[3] = 255;
    }
    write_png(path, w as u32, h as u32, &bgra)
}

fn read_pixels(
    screen: HDC,
    mem: HDC,
    bmp: HBITMAP,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Result<Vec<u8>> {
    let old = unsafe { SelectObject(mem, bmp as _) };
    let blt = unsafe { BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY) }.as_bool();
    unsafe { SelectObject(mem, old) };
    if !blt {
        return Err(fail("BitBlt"));
    }
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // 上から下への並び
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let lines = unsafe {
        GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS as u32,
        )
    };
    if lines != h {
        return Err(fail("GetDIBits"));
    }
    Ok(buf)
}

fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> Result<()> {
    let io = |e: &dyn std::fmt::Display| Error::new(E_FAIL, format!("png: {e}"));
    let file = std::fs::File::create(path).map_err(|e| io(&e))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| io(&e))?;
    writer.write_image_data(rgba).map_err(|e| io(&e))?;
    writer.finish().map_err(|e| io(&e))
}
