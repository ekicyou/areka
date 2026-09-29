//! ファイルを選ぶ画面（design「areka / install / pick」・要件 1.3・1.4・設計で決めたこと 11）。
//!
//! `GetOpenFileNameW` を持ち主の窓なしで出し、選ばれたファイル 1 つのパスを返す。呼んだスレッドを
//! 塞ぐので、呼び手は短命のスレッド `install-pick`（`desk::pick_and_submit` が起こす）に限る。
//! COM は呼んだスレッドで単一スレッドの形に初期化し、戻ったら解放する。作業フォルダは変えない
//! （`OFN_NOCHANGEDIR`）。この module の `unsafe` は [`pick_archive`] に閉じる。

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY,
    OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::core::{HRESULT, PCWSTR, PWSTR};

/// 選ぶ画面のフィルタ（表示名とパターンの組を並んだ順に）。書庫と「すべてのファイル」。
const FILTERS: [(&str, &str); 2] = [
    ("書庫 (*.nar;*.zip)", "*.nar;*.zip"),
    ("すべてのファイル (*.*)", "*.*"),
];

/// 選んだパスを受ける領域の長さ（UTF-16 の単位・長いパスも受ける）。
const PATH_CAPACITY: usize = 32 * 1024;

/// 選ぶ画面を出せなかった理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickError {
    /// COM を初期化できなかった。
    Com(HRESULT),
    /// 画面が失敗した（`CommDlgExtendedError` の値）。
    Dialog(u32),
}

/// ファイルを選ぶ画面を出して、選ばれたパスを返す（取り消しは None）。呼んだスレッドを塞ぐ。
pub(crate) fn pick_archive() -> Result<Option<PathBuf>, PickError> {
    // SAFETY: 引数は None と定数。初期化に成功したときだけ、同じスレッドで対にして解放する。
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_err() {
        return Err(PickError::Com(hr));
    }
    let filter = filter_wide();
    let mut buffer = vec![0u16; PATH_CAPACITY];
    let mut ofn = OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        Flags: OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_HIDEREADONLY
            | OFN_NOCHANGEDIR
            | OFN_EXPLORER,
        ..Default::default()
    };
    // SAFETY: `ofn` の指す領域（フィルタ・パスの受け）は呼び出しの間ずっと生存し、長さは実物と一致する。
    // 持ち主の窓は渡さない（null）。
    let chosen = unsafe { GetOpenFileNameW(&mut ofn) }.as_bool();
    // SAFETY: 引数なし。同じスレッドの直前の画面の失敗の値を読むだけ。
    let code = if chosen {
        0
    } else {
        unsafe { CommDlgExtendedError() }.0
    };
    // SAFETY: 上の初期化の成功と対になる解放（同じスレッド）。
    unsafe { CoUninitialize() };
    match (chosen, code) {
        (true, _) => Ok(Some(path_from_buffer(&buffer))),
        // 失敗の値が 0 なら利用者の取り消し。
        (false, 0) => Ok(None),
        (false, code) => Err(PickError::Dialog(code)),
    }
}

/// フィルタを画面へ渡す形（各語を NUL で区切り、末尾を NUL 2 つで閉じる）に組む（純粋）。
fn filter_wide() -> Vec<u16> {
    let mut wide: Vec<u16> = FILTERS
        .iter()
        .flat_map(|(label, pattern)| [*label, *pattern])
        .flat_map(|s| s.encode_utf16().chain([0]))
        .collect();
    wide.push(0);
    wide
}

/// 受けの領域から最初の NUL までをパスとして読む（純粋）。
fn path_from_buffer(buffer: &[u16]) -> PathBuf {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    PathBuf::from(OsString::from_wide(&buffer[..end]))
}

#[cfg(test)]
#[path = "pick_tests.rs"]
mod tests;
