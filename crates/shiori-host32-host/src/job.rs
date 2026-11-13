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
//! 2. **標準エラーの管の書き口を握り続ける**ため、host の標準エラーの読み手が管の終わりを得られず、
//!    その読み手を待つ側が止まる（helper の標準入力・標準出力は NUL、標準エラーは host の管。
//!    helper は始めに自分の標準の取っ手の受け継ぎの印を落とすので、書き口を持つのは helper だけ）。
//!
//! 本モジュールは Windows の **Job Object** による OS 公式の道連れ機構でこれを塞ぐ:
//! helper を spawn 直後に `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 付き job へ割り当て、job
//! ハンドルを host（＝親プロセス内）が [`OwnedHandle`] として保持する。親プロセスが
//! 終了（正常・異常いずれも）すると OS が全ハンドルを閉じ、job クローズで helper が確実に
//! kill される。**helper 側の協力にも UNLOAD 到達にも依存しない安全網**であり、下流の
//! 正規 UNLOAD 終了経路と両立する（先に UNLOAD で正常終了していれば job には対象が残らず無害）。
//!
//! Job Object の呼び出しはこのモジュールにまとめる（[`process_host`](crate::process_host) も
//! 起こし方・待ち方・標準エラーの読みで `windows` を使う）。

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::Child;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject,
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
                tracing::error!(
                    target: crate::process_host::LOG_TARGET,
                    error = %e,
                    "helper 孤児化防止: CreateJobObject 失敗（縮退）"
                );
                return None;
            }
        };

        // 2) KILL_ON_JOB_CLOSE（job の最後のハンドルが閉じたら所属プロセスを全 kill）と
        //    SILENT_BREAKAWAY_OK（helper が起こしたプロセス＝SHIORI の子は job に入らず、ゴーストを
        //    閉じても残る＝SSP と同じ向き・要件 10.7）を設定する。helper 自身は job に残る。
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
        if let Err(e) = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            core::ptr::addr_of!(info) as *const core::ffi::c_void,
            core::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) {
            tracing::error!(
                target: crate::process_host::LOG_TARGET,
                error = %e,
                "helper 孤児化防止: SetInformationJobObject 失敗（縮退）"
            );
            let _ = CloseHandle(job);
            return None;
        }

        // 3) helper を job へ割り当て（child の OS プロセスハンドル）。
        let child_handle = HANDLE(child.as_raw_handle());
        if let Err(e) = AssignProcessToJobObject(job, child_handle) {
            tracing::error!(
                target: crate::process_host::LOG_TARGET,
                error = %e,
                "helper 孤児化防止: AssignProcessToJobObject 失敗（縮退）"
            );
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
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    use windows::Win32::System::JobObjects::{
        JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK, QueryInformationJobObject,
    };

    /// 約 60 秒生きる子（`ping` をじかに起こす。`cmd /c` を挟むと孫が残る＝`terminator_tests.rs` と同じ）。
    fn long_lived() -> std::process::Child {
        Command::new("ping.exe")
            .args(["-n", "60", "127.0.0.1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("ping stand-in を spawn できる")
    }

    /// job の旗を読み戻すと、道連れ（KILL_ON_JOB_CLOSE）と孫の抜け出し（SILENT_BREAKAWAY_OK）の
    /// 2 つが立っている（要件 10.7・指摘 13）。
    #[test]
    fn job_limits_let_grandchildren_break_away() {
        let mut child = long_lived();
        let job = attach_kill_on_close_job(&child).expect("job 割り当てが成功する");
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        // SAFETY: `job` は生きている job の取っ手。`info` は渡す長さの書き込み先。
        unsafe {
            QueryInformationJobObject(
                Some(HANDLE(job.as_raw_handle())),
                JobObjectExtendedLimitInformation,
                core::ptr::addr_of_mut!(info).cast(),
                core::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                None,
            )
        }
        .expect("job の旗を読み戻せる");
        let flags = info.BasicLimitInformation.LimitFlags;
        let _ = child.kill();
        let _ = child.wait();
        assert!(
            flags.contains(JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE),
            "{flags:?}"
        );
        assert!(
            flags.contains(JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK),
            "{flags:?}"
        );
    }

    /// KILL_ON_JOB_CLOSE の実効性: 長時間走る子を job へ割り当て、job ハンドル（`OwnedHandle`）を
    /// drop すると子が OS により**速やかに終了させられる**（＝孤児化しない）ことを実プロセスで実証する。
    ///
    /// 判定は exit code ではなく**終了時刻**で行う（job kill 時の exit code は環境依存で 0 にも
    /// なり得るため）。割り当てが破れていれば子は ~60s の ping を完走するまで生き続け、`try_wait`
    /// が締切内に `Some` を返さない＝assert 失敗として孤児化バグを実回帰検出できる。
    #[test]
    fn dropping_job_handle_terminates_child_promptly() {
        // ~60s 走る stand-in（ping）。job kill されなければ締切（10s）を大きく超えて生存する。
        let mut child = long_lived();

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
