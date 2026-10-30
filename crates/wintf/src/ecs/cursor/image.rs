//! カーソルの作成と破棄（Win32）。

use super::{CursorError, cur_file};
use std::path::Path;
use tracing::warn;
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DestroyCursor, GetCursor, HCURSOR, IDC_ARROW, IMAGE_CURSOR,
    LR_DEFAULTCOLOR, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadCursorW, LoadImageW, SM_CXCURSOR,
    SetCursor,
};
use windows::core::{HSTRING, PCWSTR};

/// 自分で作ったカーソルの持ち主。`Drop` で捨てる。共有のカーソル（標準の形）は持たない。
#[derive(Debug)]
pub struct OwnedCursor(HCURSOR);

// SAFETY: HCURSOR はプロセスのどのスレッドからでも使えるハンドルで、捨てるのは持ち主 1 つだけ。
unsafe impl Send for OwnedCursor {}

impl OwnedCursor {
    /// `.cur`・`.ani` のファイルから作る（共有にしない）。
    pub fn load_file(path: &Path) -> Result<Self, CursorError> {
        let wide = HSTRING::from(path.as_os_str());
        // SAFETY: Win32 境界。LR_SHARED を付けないので、返るカーソルはこの持ち主だけのもの。
        let handle = unsafe {
            LoadImageW(
                None,
                &wide,
                IMAGE_CURSOR,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
        }
        .map_err(|source| CursorError::Load {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self(HCURSOR(handle.0)))
    }

    /// `.cur` のバイト列から、`size` 以上で最も小さい項目（無ければ最も大きい項目）を作る。
    pub fn from_cur_bytes(bytes: &[u8], size: u32) -> Result<Self, CursorError> {
        let entries = cur_file::read_cur_directory(bytes).map_err(CursorError::CurFile)?;
        let entry = cur_file::pick_cur_entry(&entries, size)
            .ok_or(CursorError::CurFile(cur_file::CurFileError::Empty))?;
        // CreateIconFromResourceEx のカーソルは、絵の本体の前に当たりの点の 2 語を置く。
        let mut res = Vec::with_capacity(4 + entry.data.len());
        res.extend_from_slice(&entry.hotspot.0.to_le_bytes());
        res.extend_from_slice(&entry.hotspot.1.to_le_bytes());
        res.extend_from_slice(&bytes[entry.data.clone()]);
        let side = size as i32;
        // SAFETY: Win32 境界。res は呼び出しの間生きている。返るカーソルはこの持ち主だけのもの。
        let icon = unsafe {
            CreateIconFromResourceEx(&res, false, 0x0003_0000, side, side, LR_DEFAULTCOLOR)
        }
        .map_err(CursorError::Create)?;
        Ok(Self(HCURSOR(icon.0)))
    }

    pub fn handle(&self) -> HCURSOR {
        self.0
    }
}

impl Drop for OwnedCursor {
    fn drop(&mut self) {
        // SAFETY: Win32 境界。今出ているカーソルを捨てないよう、先に標準の矢印へ替える。
        unsafe {
            if GetCursor() == self.0 {
                SetCursor(LoadCursorW(None, IDC_ARROW).ok());
            }
            if let Err(e) = DestroyCursor(self.0) {
                warn!(error = %e, "カーソルを捨てられない");
            }
        }
    }
}

/// OS の標準の形（`IDC_*`）を読む。共有のカーソルなので捨てない。
pub fn stock_cursor(id: PCWSTR) -> Result<HCURSOR, CursorError> {
    // SAFETY: Win32 境界。共有のカーソルを読むだけで、所有権は移らない。
    unsafe { LoadCursorW(None, id) }.map_err(CursorError::Stock)
}

/// 窓の DPI でのカーソルの寸法（px）。
pub fn cursor_size_for_dpi(dpi: u32) -> u32 {
    // SAFETY: Win32 境界。値を読むだけ。
    unsafe { GetSystemMetricsForDpi(SM_CXCURSOR, dpi) }.max(0) as u32
}

#[cfg(test)]
#[path = "image_tests.rs"]
mod image_tests;
