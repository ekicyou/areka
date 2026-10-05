//! OS の境界（areka-P0-open-external-tags task 1.2・要件 2.3・7.1・7.7・10.1）。
//!
//! OS へ渡す呼び出し（`ShellExecuteExW`）と環境変数の読み取りを 1 つの trait に閉じる。
//! 本物は [`WindowsShell`]、テストの偽物は `opener_test_support.rs` の `FakeOs`。
//! `crates/areka/src` で `ShellExecute` を綴るのは、このファイルだけにする（要件 7.1）。

use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx,
};
use windows::Win32::UI::Shell::{SEE_MASK_FLAG_NO_UI, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

/// OS の既定の動詞（areka 独自の設定値は持たない・要件 7.7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    /// 「開く」（既定の関連付け）。
    Open,
    /// 「編集」の関連付け。
    Edit,
}

/// OS へ渡す 1 回分の呼び出し（偽物はこれを記録する）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OsCall {
    pub verb: Verb,
    /// SHELLEXECUTEINFOW の lpFile（URL・パス・名前だけの実行ファイル・explorer.exe）。
    pub file: OsString,
    /// lpParameters（explorer.exe の /select だけが使う）。
    pub params: Option<OsString>,
    /// lpDirectory（作業フォルダ）。
    pub dir: Option<PathBuf>,
}

/// OS の境界。
pub(crate) trait OsPort {
    /// OS へ渡す。失敗は GetLastError の Win32 の符号（例: 見つからない 2・関連付けが無い 1155）。
    fn shell_execute(&mut self, call: &OsCall) -> Result<(), u32>;
    /// 環境変数（Windows の規則で大文字小文字を区別しない）。
    fn env_var(&self, name: &str) -> Option<String>;
}

/// 本物（`ShellExecuteExW`（`SEE_MASK_FLAG_NO_UI`）・`std::env::var`）。
pub(crate) struct WindowsShell;

/// 終端 0 付きの UTF-16 へ写す。
fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

impl OsPort for WindowsShell {
    fn shell_execute(&mut self, call: &OsCall) -> Result<(), u32> {
        let file = wide(&call.file);
        let params = call.params.as_deref().map(wide);
        let dir = call.dir.as_deref().map(|d| wide(d.as_os_str()));
        let ptr = |v: &Option<Vec<u16>>| v.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            // OS のエラーの窓も「開く方法を選ぶ」窓も出さない（設計ディスカッション議題 1）。
            fMask: SEE_MASK_FLAG_NO_UI,
            lpVerb: match call.verb {
                Verb::Open => w!("open"),
                Verb::Edit => w!("edit"),
            },
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: ptr(&params),
            lpDirectory: ptr(&dir),
            nShow: SW_SHOWNORMAL.0,
            // 親の窓は無し（hwnd は既定の null）。
            ..Default::default()
        };
        // SAFETY: 文字列はすべて終端 0 付きで、呼び出しの間ずっと生存する。
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| {
            // windows の Error は GetLastError を HRESULT_FROM_WIN32 で包んでいるので、下位 16 ビットへ戻す。
            let h = e.code().0 as u32;
            if h & 0xFFFF_0000 == 0x8007_0000 {
                h & 0xFFFF
            } else {
                h
            }
        })
    }

    fn env_var(&self, name: &str) -> Option<String> {
        // Windows の環境変数の読み取り（GetEnvironmentVariableW）は大文字小文字を区別しない。
        std::env::var(name).ok()
    }
}

/// 開く専用のスレッドの最初に 1 度だけ呼ぶ（`COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE`）。
///
/// 失敗は `warn!` を残して続ける（`ShellExecuteExW` は COM 無しでも多くの場合は動く）。
pub(crate) fn init_com_for_shell() {
    // SAFETY: 予約の引数は None。対の CoUninitialize はスレッドの終わりまで呼ばない。
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if let Err(err) = hr.ok() {
        tracing::warn!(
            event = "open_external_com_init_failed",
            code = format_args!("{:#010x}", err.code().0),
            "[readme] CoInitializeEx failed: opening continues without it"
        );
    }
}
