//! `--help`: 配布物の説明書を OS で開いて終わる（要件 9）。

use std::path::{Path, PathBuf};

use crate::readme::os_port::{OsCall, OsPort, Verb};

/// 説明書を開けなかったわけ（要件 9.3）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HelpError {
    /// exe の場所が分からない。
    NoExeDir,
    /// 説明書のファイルが無い。
    Missing(PathBuf),
    /// OS が開くのを断った（`code` は Win32 の符号）。
    OpenFailed { path: PathBuf, code: u32 },
}

/// 配布物の説明書の名前（exe の隣）。
const README: &str = "README.txt";

/// `exe_dir\README.txt` が在れば `os` で 1 回だけ開く（`Verb::Open`・要件 9.1・9.3）。
///
/// OS が受け付けたかまでを見て、表示されたかは見ない。
pub(crate) fn open_readme(
    exe_dir: Option<&Path>,
    os: &mut dyn OsPort,
) -> Result<PathBuf, HelpError> {
    let path = exe_dir.ok_or(HelpError::NoExeDir)?.join(README);
    if !path.is_file() {
        return Err(HelpError::Missing(path));
    }
    let call = OsCall {
        verb: Verb::Open,
        file: path.clone().into_os_string(),
        params: None,
        dir: None,
    };
    match os.shell_execute(&call) {
        Ok(()) => Ok(path),
        Err(code) => Err(HelpError::OpenFailed { path, code }),
    }
}

/// 本物の口: COM を整えてから、exe の本当の場所の隣の説明書を開く。
///
/// 開けたら info（`help_opened`）で `Ok(())`、だめなら error（`help_open_failed`）で
/// `Err(E_FAIL)`（0 でない終了・要件 9.3）。
pub(crate) fn run() -> windows::core::Result<()> {
    crate::readme::os_port::init_com_for_shell();
    let exe_dir = crate::boot_config::exe_location().and_then(Path::parent);
    match open_readme(exe_dir, &mut crate::readme::os_port::WindowsShell) {
        Ok(path) => {
            tracing::info!(
                event = "help_opened",
                path = %path.display(),
                "[cli] --help: 説明書を開いた"
            );
            Ok(())
        }
        Err(error) => {
            tracing::error!(
                event = "help_open_failed",
                reason = ?error,
                "[cli] --help: 説明書を開けなかった"
            );
            Err(windows::core::Error::from_hresult(
                windows::Win32::Foundation::E_FAIL,
            ))
        }
    }
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod help_tests;
