//! `Child` を持たないスレッドから helper プロセスを終わらせる取っ手（要件 2.4・3.5）。
//!
//! `Child::kill` は `&mut Child` を要し、`Child` は shiori のスレッドの中に閉じている。
//! 本型はプロセスの取っ手を複製して持ち、どのスレッドからでも `TerminateProcess` を呼べる。
//! windows 依存は [`crate::job`] と同じ理由でこのファイルに隔離する（ワークスペースの
//! `windows` 依存に既にある機能だけを使う）。

use std::os::windows::io::{AsHandle, AsRawHandle, OwnedHandle};
use std::process::Child;
use std::sync::Arc;

use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, HANDLE};
use windows::Win32::System::Threading::TerminateProcess;

/// helper のプロセスの取っ手の複製。`Clone + Send + Sync`。最後の複製の `Drop` で取っ手を閉じる。
#[derive(Debug, Clone)]
pub struct HelperTerminator(Arc<OwnedHandle>);

impl HelperTerminator {
    /// `child` のプロセスの取っ手（`PROCESS_ALL_ACCESS`）を複製して取っ手を作る。
    pub(crate) fn from_child(child: &Child) -> std::io::Result<Self> {
        Ok(Self(Arc::new(child.as_handle().try_clone_to_owned()?)))
    }

    /// helper を終了コード 1 で終わらせる。
    ///
    /// 既に終わった・終わりかけのプロセス（`TerminateProcess` が `ERROR_ACCESS_DENIED` を返す）は
    /// `Ok` に畳む（[`crate::HelperHandle::terminate`] と同じ冪等の約束）。
    /// 失敗は呼び手へ返し、本型は記録しない（記録は呼び手が `error!` で行う）。
    ///
    /// # Errors
    /// `TerminateProcess` の失敗（既に終わった場合を除く）を [`std::io::Error`] として返す。
    pub fn terminate(&self) -> std::io::Result<()> {
        let process = HANDLE(self.0.as_raw_handle());
        // SAFETY: `process` は本型が所有する有効なプロセスの取っ手（`Arc<OwnedHandle>` が生きている間
        // 閉じられない）。`TerminateProcess` は取っ手を借りるだけ。
        match unsafe { TerminateProcess(process, 1) } {
            Ok(()) => Ok(()),
            // 取っ手は `PROCESS_ALL_ACCESS` の複製なので、`ERROR_ACCESS_DENIED` は「既に終わった／
            // 終わりかけ」だけを意味する（終わりかけではまだシグナル状態でないので待ちは照会しない）。
            Err(e) if e.code() == ERROR_ACCESS_DENIED.to_hresult() => Ok(()),
            Err(e) => {
                // 記録が読みやすいよう、Win32 由来の HRESULT（0x8007xxxx）は Win32 のコードに戻す。
                let hr = e.code().0 as u32;
                Err(if hr & 0xFFFF_0000 == 0x8007_0000 {
                    std::io::Error::from_raw_os_error((hr & 0xFFFF) as i32)
                } else {
                    e.into()
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "terminator_tests.rs"]
mod terminator_tests;
