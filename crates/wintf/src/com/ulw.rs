//! D2D1 → HBITMAP ピクセル転送ユーティリティ
//!
//! `transfer_to_hbitmap` は、D2D1 ステージングビットマップ (`ID2D1Bitmap1`) から
//! GDI DIBSection HBITMAP へピクセルデータを高速転送する。
//! ECS 非依存の純粋ユーティリティ関数として実装されている。

use windows::Win32::Graphics::Direct2D::*;

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
