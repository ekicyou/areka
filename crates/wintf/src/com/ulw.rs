//! D2D1 → HBITMAP ピクセル転送 & UpdateLayeredWindow ユーティリティ
//!
//! - `transfer_to_hbitmap`: D2D1 ステージングビットマップ (`ID2D1Bitmap1`) から
//!   GDI DIBSection HBITMAP へピクセルデータを高速転送する。
//! - `present_layered_window`: UpdateLayeredWindow を呼び出し、合成済みビットマップを
//!   WS_EX_LAYERED ウィンドウに表示する。
//!
//! ECS 非依存の純粋ユーティリティ関数として実装されている。

use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Gdi::{AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, HDC};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, ULW_ALPHA, UpdateLayeredWindow};

/// ステージング ID2D1Bitmap1 のピクセルデータを DIBSection HBITMAP にコピーする。
///
/// # Safety
/// - `dib_bits` は `width * height * 4` バイト以上のメモリを指すこと。
/// - staging bitmap は `D2D1_BITMAP_OPTIONS_CPU_READ` フラグ付きで作成されていること。
/// - 呼び出し元で `CopyFromBitmap` が完了済みであること。
pub unsafe fn transfer_to_hbitmap(
    staging: &ID2D1Bitmap1,
    dib_bits: *mut u8,
    width: u32,
    height: u32,
) -> windows::core::Result<()> {
    // Map でステージングビットマップのピクセルデータにアクセス
    let mapped = unsafe { staging.Map(D2D1_MAP_OPTIONS_READ)? };

    let pitch = mapped.pitch as usize;
    let stride = (width as usize) * 4;
    let src = mapped.bits;

    if pitch == stride {
        // pitch と stride が一致: 一括コピー
        unsafe {
            std::ptr::copy_nonoverlapping(src, dib_bits, stride * (height as usize));
        }
    } else {
        // pitch != stride: 行単位コピー（GPU パディングアライメント対応）
        for y in 0..height as usize {
            unsafe {
                let src_row = src.add(y * pitch);
                let dst_row = dib_bits.add(y * stride);
                std::ptr::copy_nonoverlapping(src_row, dst_row, stride);
            }
        }
    }

    // Unmap でマッピングを解除
    unsafe { staging.Unmap()? };

    Ok(())
}

/// UpdateLayeredWindow を呼び出し、合成済みビットマップを WS_EX_LAYERED ウィンドウに表示する。
///
/// - `pptDst`: ウィンドウ位置を `GetWindowRect` で取得して明示的に指定（pptDst=None ではリセットされる問題を回避）
/// - `ptSrc` = `POINT { x: 0, y: 0 }`: ソースビットマップの原点
/// - `BLENDFUNCTION`: `AC_SRC_OVER`, `SourceConstantAlpha=255`, `AC_SRC_ALPHA`
/// - `ULW_ALPHA` モードで alpha 透過合成
///
/// # Arguments
/// - `hwnd` - 表示対象の WS_EX_LAYERED ウィンドウ
/// - `hdc_src` - 合成済みビットマップが SelectObject 済みの MemoryDC
/// - `size` - ウィンドウサイズ（幅, 高さ）
pub fn present_layered_window(hwnd: HWND, hdc_src: HDC, size: &SIZE) -> windows::core::Result<()> {
    // ウィンドウ位置を取得（pptDst=None だと位置がリセットされる問題への対策）
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect)? };
    let pt_dst = POINT {
        x: rect.left,
        y: rect.top,
    };

    let pt_src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,          // hdcDst (screen DC, None = default)
            Some(&pt_dst), // pptDst (現在位置を明示的に指定)
            Some(size),    // psize
            Some(hdc_src), // hdcSrc
            Some(&pt_src), // pptSrc
            COLORREF(0),   // crKey (ULW_ALPHA モードでは不使用)
            Some(&blend),  // pblend
            ULW_ALPHA,     // dwFlags
        )?;
    }
    Ok(())
}
