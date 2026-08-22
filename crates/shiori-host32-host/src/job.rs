//! host-32 helper の孤児化防止（Windows Job Object・KILL_ON_JOB_CLOSE）。
//!
//! helper（i686 別プロセス）の常駐 lifecycle——親からの UNLOAD による**正規の正常終了**——は
//! 下流 `host32-lifecycle` の領分であり、helper 側には親死亡検知が無い（`main` は
//! `quit_requested`＝UNLOAD 受領でしか終了しない）。そのため親（host を内包する areka プロセス）が
//! UNLOAD を送らずに終了する経路——プロセス crash・SHIORI **LOAD 失敗後の高速 teardown**・
//! smoke exit 等——では helper が message loop を回し続けて **孤児化**する。
//!
//! 孤児化の実害は 2 つ:
//! 1. **プロセスリーク**（ゴースト起動のたびに zombie helper が残り得る）。
//! 2. **継承した stdio パイプを握り続ける**ため、親を piped で起動した側の
//!    `wait_with_output()` が EOF を得られず **無限ブロック**する（統合 smoke テストのハング源）。
//!
//! 本モジュールは Windows の **Job Object** による OS 公式の道連れ機構でこれを塞ぐ:
//! helper を spawn 直後に `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 付き job へ割り当て、job
//! ハンドルを host（＝親プロセス内）が [`OwnedHandle`] として保持する。親プロセスが
//! 終了（正常・異常いずれも）すると OS が全ハンドルを閉じ、job クローズで helper が確実に
//! kill される。**helper 側の協力にも UNLOAD 到達にも依存しない安全網**であり、下流の
//! 正規 UNLOAD 終了経路と両立する（先に UNLOAD で正常終了していれば job には対象が残らず無害）。
//!
//! windows 依存はこのモジュールに隔離し、[`process_host`](crate::process_host) の
//! 純ロジック（`ExitKind::classify`・`resolve_param` 等）の std-only 単体テスト可能性を保つ。

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Child;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};

/// spawn 済み `child`（helper）を `KILL_ON_JOB_CLOSE` 付き Job Object へ割り当て、その
/// job ハンドルを RAII owner（[`OwnedHandle`]）として返す。
///
/// 返した `OwnedHandle` を保持する限り job は生存し、**保持プロセス（親）終了時＝全ハンドル
/// 閉鎖**で job クローズ→helper kill が発火する。`Drop` で `CloseHandle` される（明示 kill 不要）。
///
/// いずれかの Win32 呼び出しが失敗した場合は `None` を返して**縮退**する（`error!` 記録の上、
/// spawn 自体は継続＝孤児化防止だけを諦める。子が生成直後に即終了しており割り当て不能な
/// 場合も含む）。panic はしない（log-first・R6.4 と同格の非致命縮退）。
#[must_use]
pub(crate) fn attach_kill_on_close_job(child: &Child) -> Option<OwnedHandle> {
    // SAFETY: 各 Win32 呼び出しは下記契約で用い、失敗は Result で受けて panic しない。
    // 生成した job ハンドルは、成功時は OwnedHandle へ move して RAII 管理、失敗時は
    // 明示 CloseHandle で確実に解放する（二重クローズなし）。
    unsafe {
        // 1) 無名 job を生成（非継承ハンドル＝子は job ハンドルを継承しない）。
        let job: HANDLE = match CreateJobObjectW(None, None) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(error = %e, "helper 孤児化防止: CreateJobObject 失敗（縮退）");
                return None;
            }
        };

        // 2) KILL_ON_JOB_CLOSE を設定（job の最後のハンドルが閉じたら所属プロセスを全 kill）。
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(e) = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            core::ptr::addr_of!(info) as *const core::ffi::c_void,
            core::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) {
            tracing::error!(error = %e, "helper 孤児化防止: SetInformationJobObject 失敗（縮退）");
            let _ = CloseHandle(job);
            return None;
        }

        // 3) helper を job へ割り当て（child の OS プロセスハンドル）。
        let child_handle = HANDLE(child.as_raw_handle());
        if let Err(e) = AssignProcessToJobObject(job, child_handle) {
            tracing::error!(error = %e, "helper 孤児化防止: AssignProcessToJobObject 失敗（縮退）");
            let _ = CloseHandle(job);
            return None;
        }

        // 4) job ハンドルを RAII owner へ（drop で CloseHandle → KILL_ON_JOB_CLOSE 発火）。
        Some(OwnedHandle::from_raw_handle(job.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};

    /// KILL_ON_JOB_CLOSE の実効性: 長時間走る子を job へ割り当て、job ハンドル（`OwnedHandle`）を
    /// drop すると子が OS により**速やかに終了させられる**（＝孤児化しない）ことを実プロセスで実証する。
    ///
    /// 判定は exit code ではなく**終了時刻**で行う（job kill 時の exit code は環境依存で 0 にも
    /// なり得るため）。割り当てが破れていれば子は ~60s の ping を完走するまで生き続け、`try_wait`
    /// が締切内に `Some` を返さない＝assert 失敗として孤児化バグを実回帰検出できる。
    #[test]
    fn dropping_job_handle_terminates_child_promptly() {
        // ~60s 走る stand-in（ping）。job kill されなければ締切（10s）を大きく超えて生存する。
        let mut child = Command::new("cmd.exe")
            .args(["/c", "ping", "-n", "60", "127.0.0.1"])
            .spawn()
            .expect("ping stand-in を spawn できる");

        let guard = attach_kill_on_close_job(&child).expect("job 割り当てが成功する");

        // job ハンドルを閉じる → KILL_ON_JOB_CLOSE で所属プロセス（child とその子孫）を全 kill。
        drop(guard);

        // 締切内に子が終了する（＝kill された）ことを非ブロッキング poll で確認する。
        // 割り当てが効いていれば <1s で終了、破れていれば ~60s 生存し締切超過で FAIL。
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break, // 子が終了＝job kill が効いた。
                Ok(None) => {
                    assert!(
                        Instant::now() < deadline,
                        "job ハンドル drop 後も子が生存＝孤児化（KILL_ON_JOB_CLOSE 不発）"
                    );
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => panic!("child の try_wait に失敗: {e}"),
            }
        }
    }
}
