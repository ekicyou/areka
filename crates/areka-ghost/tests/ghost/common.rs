//! common.rs — ghost 統合テスト共有の小道具（テスト支援）。
//!
//! `tests/ghost.rs` の束ねと `tests/real_pasta_online.rs` の 2 つの実行ファイルがパス指定で取り込む。
//!
//! - [`test_system_vars`]: 退役した `areka_ghost::default_system_vars()`（W1 暫定 provider）の代役。
//!   既存 e2e テストの既定 username 前提を [`areka_ghost::SystemVarWiring::Custom`] の直接注入で保つ。
//! - [`resolve_helper_exe`]: 実 pasta のテストが使う i686 helper の探し。
//! - [`run_bounded`]／[`run_bounded_shutdown`]: 宙吊りを括る有界の実行と正規の終了。
//! - [`real_pasta_turn`]: 実 pasta を起こすテストの順番（127.0.0.1:9276 の取り合いを避ける）。

use std::path::PathBuf;
use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use areka_ghost::{GhostRuntime, SystemVarSource};
use areka_kanade::CloseReason;
use areka_sakura::contract::SystemVarSnapshot;
use areka_sakura::sysvar::DEFAULT_USERNAME;

/// 退役した `default_system_vars()` と同一挙動: `{"username": DEFAULT_USERNAME}` のみを
/// 充填した凍結スナップショットを毎回新規構築して返すクロージャ。`GhostBootOptions.system_vars`
/// へは `SystemVarWiring::Custom(test_system_vars())` として渡す。
pub fn test_system_vars() -> SystemVarSource {
    Box::new(|| {
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", DEFAULT_USERNAME);
        snapshot
    })
}

/// i686 helper 実行ファイルのパスを解決する（env `HOST32_HELPER_EXE` 最優先 → ワークスペース
/// `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`）。
pub fn resolve_helper_exe() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("HOST32_HELPER_EXE") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "HOST32_HELPER_EXE={explicit:?} が指すファイルが存在しません（i686 helper を先にビルド）"
        ));
    }
    // CARGO_MANIFEST_DIR = crates/areka-ghost → 2 つ上がワークスペースルート。
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.clone());
    let target_base = workspace_root.join("target").join("i686-pc-windows-msvc");
    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join("shiori-host32-helper.exe");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "i686 helper exe が見つかりません（探索先: {}\\{{debug,release}}\\shiori-host32-helper.exe）。\
         PowerShell で先に i686 helper をビルドしてください: \
         cargo build -p shiori-host32-helper --target i686-pc-windows-msvc",
        target_base.display()
    ))
}

/// `f` を別スレッドで走らせ、`timeout` 以内に終わればその値を返す。閉包が panic したら
/// 「panic した」、時間内に終わらなければ「止まった」と分けて panic する（原因を取り違えない）。
pub fn run_bounded<T: Send + 'static>(
    what: &str,
    timeout: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (done_tx, done_rx) = sync_channel::<T>(1);
    std::thread::spawn(move || {
        let _ = done_tx.send(f());
    });
    match done_rx.recv_timeout(timeout) {
        Ok(value) => value,
        // 送り手が値を送らずに落ちた＝閉包が panic した。
        Err(RecvTimeoutError::Disconnected) => panic!("'{what}' panicked (not a hang)"),
        Err(RecvTimeoutError::Timeout) => {
            panic!("'{what}' did not complete within {timeout:?} (possible hang)")
        }
    }
}

/// 正規の終了（`CloseReason::System`）を `timeout` の上限つきで待ち、`Ok(())` を確かめる。
pub fn run_bounded_shutdown(what: &str, timeout: Duration, runtime: GhostRuntime) {
    let result = run_bounded(what, timeout, move || {
        runtime
            .shutdown(CloseReason::System)
            .map_err(|err| format!("{err:?}"))
    });
    assert!(
        result.is_ok(),
        "'{what}': shutdown は Ok(()) を返すはず: {result:?}"
    );
}

/// 実 pasta を起こすテストの順番。`PASTA_DEBUG` が立っていると emo2 は 127.0.0.1:9276 を握るので、
/// 同じ実行ファイルの中で並べて走らせると取り合う。テストの本体の間ずっと持つ。前のテストの
/// panic で毒になっていても取り直す（順番だけの錠で、守る値は無い）。
pub fn real_pasta_turn() -> MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "'probe' panicked (not a hang)")]
    fn run_bounded_reports_a_panic_as_a_panic() {
        run_bounded::<()>("probe", Duration::from_secs(60), || {
            panic!("the closure fails on purpose")
        });
    }
}
