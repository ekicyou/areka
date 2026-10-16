//! 置き場にするフォルダを選ぶ画面（design「RootsPick」・要件 6.2・6.9・11.5）。
//!
//! OS 標準のフォルダ選択（`IFileOpenDialog`＋`FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM |
//! FOS_PATHMUSTEXIST`）を持ち主の窓なしで出し、選ばれたフォルダのパスを返す。呼んだスレッドを
//! 塞ぐので、呼び手は短命のスレッド `roots-pick`（`desk::start_pick` が起こす）に限る。
//! COM は `install/pick.rs` と同じく呼んだスレッドで単一スレッドの形に初期化し、戻ったら解放する。
//! COM の口は [`show_dialog`] に閉じ、解放の前に手放す。この module の `unsafe` はこのファイルに閉じる。

use std::path::PathBuf;

use windows::Win32::Foundation::ERROR_CANCELLED;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog,
    SIGDN_FILESYSPATH,
};
use windows::core::{HSTRING, PCWSTR};

use crate::install::pick::PickError;

/// 画面の題。
const TITLE: &str = "置き場にするフォルダを選んでください";

/// フォルダを選ぶ画面を出して、選ばれたフォルダを返す（取り消しは None）。呼んだスレッドを塞ぐ。
pub(crate) fn pick_folder() -> Result<Option<PathBuf>, PickError> {
    // SAFETY: 引数は None と定数。初期化に成功したときだけ、同じスレッドで対にして解放する。
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_err() {
        return Err(PickError::Com(hr));
    }
    // COM の口は show_dialog の中で作って手放す（解放の前に落ちる）。
    let result = show_dialog();
    // SAFETY: 上の初期化の成功と対になる解放（同じスレッド・COM の口はもう残っていない）。
    unsafe { CoUninitialize() };
    result
}

/// 画面を作って出し、選ばれたフォルダを読む。COM の口はこの関数の中だけで生きる。
fn show_dialog() -> Result<Option<PathBuf>, PickError> {
    // SAFETY: このスレッドの COM は呼び手が初期化済み。クラスと口の組は OS の定義どおり。
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| PickError::Com(e.code()))?;
    let title = HSTRING::from(TITLE);
    // SAFETY: `dialog` は生きている口。`title` は呼び出しの間ずっと生存する。
    unsafe {
        let options = dialog.GetOptions().map_err(|e| PickError::Com(e.code()))?;
        dialog
            .SetOptions(options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)
            .map_err(|e| PickError::Com(e.code()))?;
        dialog
            .SetTitle(PCWSTR(title.as_ptr()))
            .map_err(|e| PickError::Com(e.code()))?;
    }
    // SAFETY: 持ち主の窓は渡さない（None）。戻るまでこのスレッドを塞ぐ。
    if let Err(e) = unsafe { dialog.Show(None) } {
        if e.code() == ERROR_CANCELLED.to_hresult() {
            return Ok(None);
        }
        return Err(PickError::Com(e.code()));
    }
    // SAFETY: `Show` が成功した後の結果の項目を読む。返された文字列は読み終えてから CoTaskMemFree で返す。
    unsafe {
        let item = dialog.GetResult().map_err(|e| PickError::Com(e.code()))?;
        let name = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|e| PickError::Com(e.code()))?;
        let path = name.to_hstring().to_os_string();
        CoTaskMemFree(Some(name.0 as *const _));
        Ok(Some(PathBuf::from(path)))
    }
}
