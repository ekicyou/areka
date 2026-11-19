//! host-32 先進坑: 子のプロセスの見張り（x64 の親だけが取り込む）。
//!
//! i686 の helper を `std::process::Command` で起こし、生き死にと終わり方を
//! 送りの層とは別に見る（要件 1.1〜1.4・2.4）。helper（i686）からは取り込まない（要件 1.5）。
//!
//! 親の HWND は下 32 ビットの 10 進で引数に載せて渡す（`argv[1]=ghostdir`・
//! `argv[2]=parent_hwnd`）。環境変数では渡さない。
//! 持ち手（`HelperHandle`）が捨てられるとき、子がまだ動いていれば止めて待つ。
//! 親がどの `?` で抜けても helper が残らない。

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// 親が helper の exe を受ける環境変数の名前（空でない値なら引数より先に使う）。
pub const HELPER_EXE_ENV: &str = "HELPER_EXE";

/// helper の exe を選ぶ。`env` が空でない値ならそれ、無ければ `args`（exe 名を除いた引数）の
/// うち `--` で始まらない最初のもの。どちらも無ければ `None`。
pub fn resolve_helper_exe(
    env: Option<String>,
    args: impl IntoIterator<Item = String>,
) -> Option<PathBuf> {
    if let Some(p) = env.filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    args.into_iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
}

/// 起こした helper の持ち手。
pub struct HelperHandle {
    /// 子のプロセス（終わり方と生き死にを見る唯一の元）。
    pub child: Child,
}

impl Drop for HelperHandle {
    /// まだ動いていれば記録を出してから止めて待つ（待ち終えた後は何もしない）。
    fn drop(&mut self) {
        // 見えない（try_wait の失敗）ときも止めにいく。
        if let Ok(Some(_)) = self.child.try_wait() {
            return;
        }
        let pid = self.child.id();
        eprintln!("[process_host] helper (pid {pid}) がまだ動いているので止める");
        if let Err(e) = self.child.kill() {
            eprintln!("[process_host] helper (pid {pid}) を止められなかった: {e}");
        }
        if let Err(e) = self.child.wait() {
            eprintln!("[process_host] helper (pid {pid}) の終わりを待てなかった: {e}");
        }
    }
}

/// helper の終わり方（要件 1.4: clean と異常を見分けられる形に分ける）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// 終了コード 0 = clean shutdown（要件 1.3）。
    Clean,
    /// 0 でない終了コード = 異常終了（要件 1.4）。
    Abnormal(i32),
    /// 終了コードが取れない終わり方（強制終了など）。
    Terminated,
}

impl ExitKind {
    /// 終了コード（`None` = コードが無い）を分類へ写す。`None` は `Terminated`（異常）。
    pub fn classify(code: Option<i32>) -> ExitKind {
        match code {
            Some(0) => ExitKind::Clean,
            Some(c) => ExitKind::Abnormal(c),
            None => ExitKind::Terminated,
        }
    }

    /// clean shutdown か（要件 1.3 の観測点）。
    pub fn is_clean(self) -> bool {
        matches!(self, ExitKind::Clean)
    }
}

/// helper のプロセスの起こしと見張り。
pub struct ProcessHost;

impl ProcessHost {
    /// i686 の helper を起こす。`argv[1] = ghostdir`・`argv[2] = parent_hwnd`（10 進）。
    pub fn spawn(
        helper_exe: &Path,
        ghostdir: &Path,
        parent_hwnd: u32,
    ) -> std::io::Result<HelperHandle> {
        let child = Command::new(helper_exe)
            .arg(ghostdir)
            .arg(parent_hwnd.to_string())
            .spawn()?;
        Ok(HelperHandle { child })
    }

    /// 生き死にを待たずに見る（要件 1.2・2.4）。`None` = 動いている。
    /// `try_wait` 自体の失敗は見張れないので `Terminated` とする。
    pub fn poll_exit_kind(handle: &mut HelperHandle) -> Option<ExitKind> {
        match handle.child.try_wait() {
            Ok(Some(status)) => Some(ExitKind::classify(status.code())),
            Ok(None) => None,
            Err(_) => Some(ExitKind::Terminated),
        }
    }

    /// 終わるまで待って終わり方を返す（要件 1.3・1.4）。
    pub fn wait_kind(mut handle: HelperHandle) -> std::io::Result<ExitKind> {
        let status = handle.child.wait()?;
        Ok(ExitKind::classify(status.code()))
    }

    /// helper を強制終了する（err(2) の異常終了の観測用）。
    /// Windows では `TerminateProcess` で、終了コードは 0 でない（clean unload と見分けられる）。
    pub fn terminate(handle: &mut HelperHandle) -> std::io::Result<()> {
        handle.child.kill()
    }
}

// 子のプロセスを使うテスト。helper の exe には頼らず、`cmd.exe`・`ping` を代わりに起こす。
#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// `cmd.exe /c exit <code>` を持ち手に包む。
    fn spawn_exit(code: i32) -> HelperHandle {
        let child = Command::new("cmd.exe")
            .args(["/c", &format!("exit {code}")])
            .spawn()
            .expect("cmd.exe must spawn on Windows");
        HelperHandle { child }
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// 指摘 4: exe の選び方の表。
    #[test]
    fn resolve_helper_exe_table() {
        assert_eq!(
            resolve_helper_exe(None, strings(&["--selftest-errors", "h.exe"])),
            Some(PathBuf::from("h.exe")),
            "--selftest-errors の後ろの exe を選ぶ"
        );
        assert_eq!(
            resolve_helper_exe(None, strings(&["--selftest-errors"])),
            None,
            "exe が無ければ None"
        );
        assert_eq!(
            resolve_helper_exe(Some(String::new()), strings(&["h.exe"])),
            Some(PathBuf::from("h.exe")),
            "空の環境変数は無いのと同じ"
        );
        assert_eq!(
            resolve_helper_exe(Some("a.exe".to_string()), strings(&["h.exe"])),
            Some(PathBuf::from("a.exe")),
            "環境変数が引数に勝つ"
        );
    }

    /// テストが落ちても子を残さないため、握ったハンドルで最後に止めて閉じる。
    struct ProcessGuard(windows::Win32::Foundation::HANDLE);

    impl Drop for ProcessGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = windows::Win32::System::Threading::TerminateProcess(self.0, 1);
                let _ = windows::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }

    /// 指摘 6: 持ち手を捨てると、まだ動いている子が止まる。
    #[test]
    fn dropping_handle_kills_running_child() {
        use windows::Win32::Foundation::WAIT_OBJECT_0;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, WaitForSingleObject,
        };

        let child = Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("ping must spawn on Windows");
        let pid = child.id();
        let guard = ProcessGuard(
            unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) }
                .expect("OpenProcess must succeed for our own child"),
        );
        let handle = HelperHandle { child };

        drop(handle);

        let wait = unsafe { WaitForSingleObject(guard.0, 0) };
        assert_eq!(
            wait, WAIT_OBJECT_0,
            "持ち手を捨てた後、子のプロセスは終わっていなければならない"
        );
    }

    /// 要件 1.3: 終了コード 0 を `wait_kind` で Clean と見る。
    #[test]
    fn wait_kind_classifies_zero_as_clean() {
        let kind = ProcessHost::wait_kind(spawn_exit(0)).expect("wait must succeed");
        assert_eq!(kind, ExitKind::Clean);
        assert!(kind.is_clean());
    }

    /// 要件 1.4: 0 でない終了コードを Abnormal と見る。
    #[test]
    fn wait_kind_classifies_nonzero_as_abnormal() {
        let kind = ProcessHost::wait_kind(spawn_exit(3)).expect("wait must succeed");
        assert_eq!(kind, ExitKind::Abnormal(3));
        assert!(!kind.is_clean());
    }

    /// 要件 1.2・2.4: 待たずに見る。動いている間は `None`、終わった後は `Some(Clean)`。
    #[test]
    fn poll_exit_kind_is_none_while_running_then_clean_after_exit() {
        // `ping -n 2` は約 1 秒待つ（cmd の timeout は対話端末で不安定なため ping を使う）。
        let child = Command::new("cmd.exe")
            .args(["/c", "ping -n 2 127.0.0.1 >NUL & exit 0"])
            .spawn()
            .expect("cmd.exe must spawn");
        let mut handle = HelperHandle { child };

        assert_eq!(
            ProcessHost::poll_exit_kind(&mut handle),
            None,
            "poll_exit_kind must be None while the child is still running"
        );

        let mut observed = None;
        for _ in 0..200 {
            if let Some(kind) = ProcessHost::poll_exit_kind(&mut handle) {
                observed = Some(kind);
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            observed,
            Some(ExitKind::Clean),
            "poll_exit_kind must become Some(Clean) after the child exits cleanly"
        );
        // 終わった後は何度見ても同じ。
        assert_eq!(
            ProcessHost::poll_exit_kind(&mut handle),
            Some(ExitKind::Clean)
        );
    }

    /// 分類の表（要件 1.3・1.4）: 0=Clean・0 でない=Abnormal・None=Terminated。
    #[test]
    fn exit_kind_classification_table() {
        assert_eq!(ExitKind::classify(Some(0)), ExitKind::Clean);
        assert_eq!(ExitKind::classify(Some(1)), ExitKind::Abnormal(1));
        assert_eq!(ExitKind::classify(Some(255)), ExitKind::Abnormal(255));
        assert_eq!(ExitKind::classify(None), ExitKind::Terminated);
        assert!(ExitKind::classify(Some(0)).is_clean());
        assert!(!ExitKind::classify(Some(2)).is_clean());
        assert!(!ExitKind::classify(None).is_clean());
    }
}
