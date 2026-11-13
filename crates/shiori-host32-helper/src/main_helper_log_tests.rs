//! `helper_log` の檻: 標準エラーの読み手（host）が居なくなっても panic しない（タスク 14・再監査 1 回目の指摘 1）。
//!
//! 標準エラーはプロセス全体で 1 つで、libtest は `eprintln!` を自分の捕まえ口へ回すため、子プロセスの
//! 中（`--nocapture`）で OS の取っ手を読み口の閉じた管へ差し替えて確かめる。

use super::helper_log;
use std::os::windows::io::AsRawHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, SetStdHandle};

const CHILD_ENV: &str = "SHIORI_HOST32_HELPER_LOG_CHILD";

#[test]
fn helper_log_does_not_panic_when_stderr_reader_is_gone() {
    if std::env::var_os(CHILD_ENV).is_some() {
        let (reader, writer) = std::io::pipe().expect("管を作れない");
        drop(reader); // 読み口を閉じる＝host の読み手が終わった姿
        // SAFETY: Win32 境界。自分の標準エラーの取っ手を取り、差し替え、最後に戻すだけ。
        let original = unsafe { GetStdHandle(STD_ERROR_HANDLE) }.expect("標準エラーの取っ手");
        unsafe { SetStdHandle(STD_ERROR_HANDLE, HANDLE(writer.as_raw_handle())) }
            .expect("標準エラーを差し替えられない");
        let result = std::panic::catch_unwind(|| helper_log("[helper] 読み手の居ない標準エラー"));
        unsafe { SetStdHandle(STD_ERROR_HANDLE, original) }.expect("標準エラーを戻せない");
        drop(writer);
        assert!(result.is_ok(), "helper_log が panic した");
        return;
    }
    // 子の名前は module_path!() から組む（手書きの名前は改名で 0 件走行になり親が緑のまま残る）。
    let entry = format!(
        "{}::helper_log_does_not_panic_when_stderr_reader_is_gone",
        module_path!()
            .split_once("::")
            .expect("crate 名の後の経路")
            .1
    );
    let output = std::process::Command::new(std::env::current_exe().expect("テストの exe"))
        .args([entry.as_str(), "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("子のテストを起こせない");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "子のテストが赤: {:?}\n{stdout}",
        output.status
    );
    assert!(
        stdout.contains("1 passed"),
        "子が 1 件走っていない: {stdout}"
    );
}
