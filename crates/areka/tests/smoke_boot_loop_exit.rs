//! 骨格 boot→loop→exit の統合 smoke テスト（task 4.2・R4.1/R2.4）。
//!
//! env ゲート（`AREKA_APP_SMOKE_EXIT_MS`・task 2.3）を立てた areka バイナリの子プロセスを
//! 起動し、検証用ダミー窓を開いて `app.run()` ループを回した後に自動 despawn → `WindowRegistry`
//! 空遷移 → `run()` 復帰 → **exit 0** で正常終了する経路を実プロセスで踏破・証明する。
//!
//! 本テストが駆動する経路（design "Testing Strategy / Integration Tests / 骨格 smoke"）:
//! boot（`WinApp::new`）→ `open_startup_window`（ダミー窓 spawn・env ゲート自動 close タスク投入）
//! → `app.run()`（main 所有ループ）→ 指定 ms 後 despawn → 空遷移 → exit 0。
//!
//! 実装規律:
//! - 子プロセスは `cargo run` ではなく Cargo が用意する `CARGO_BIN_EXE_areka`（統合テスト用に
//!   Cargo が bin を先にビルドして渡すパス）を直接起動する。再コンパイルによるノイズを避ける。
//! - タイムアウト番犬は純 std（`std::process` + `std::thread`/`std::time`・新規依存なし・R6.1）。
//!   `try_wait()` を短周期でポーリングし、寛大な締切内に終了しなければ `kill()` してテスト失敗。
//! - **合否は exit 0 のみで判定する**。良性の teardown warn
//!   （`WARN ... Could not despawn entity ... generation 1`・smoke timer と window-registry close の
//!   二重 despawn 競合）は warn レベルで exit 0 に影響しないため assert 対象にしない
//!   （tasks.md Implementation Notes・実測済み）。

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// 子プロセスへ渡す自動 close 遅延（ms）。ダミー窓を開き run ループを一巡させるだけの
/// 十分小さな値。0 でも受理されるが（即時発火）、窓生成と run 立ち上げの前後関係を
/// 現実的にするため小さな正値を与える。
const SMOKE_EXIT_MS: &str = "500";

/// タイムアウト番犬の締切。初回起動は遅い（窓生成＋COM/DPI 初期化）ため寛大に取る。
/// これを超えて終了しなければハング＝テスト失敗（TIMEOUT）と判定する。
const WATCHDOG_DEADLINE: Duration = Duration::from_secs(60);

/// `try_wait()` のポーリング周期。
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// env ゲートを立てた areka バイナリが境界時間内に exit 0 で正常終了することを実プロセスで証明する。
///
/// - `CARGO_BIN_EXE_areka`（Cargo 提供の実ビルド済みバイナリパス）を直接起動する。
/// - `AREKA_APP_SMOKE_EXIT_MS` を立て、ダミー窓の自動 close → 空遷移 → exit 0 を誘発する。
/// - タイムアウト番犬内で終了を待ち、締切超過なら kill してテスト失敗（ハングを隠さない）。
/// - 終了コードが 0（`status.success()`）であることのみを assert する（warn の有無は見ない）。
#[test]
fn skeleton_boots_loops_and_exits_zero_within_watchdog() {
    let bin = env!("CARGO_BIN_EXE_areka");

    // 子プロセス起動: env ゲートを立て、診断用に stdout/stderr を捕捉する。
    // stdout/stderr を piped にすることで、失敗時に子の tracing 出力を assert メッセージへ回せる。
    let mut child = Command::new(bin)
        .env("AREKA_APP_SMOKE_EXIT_MS", SMOKE_EXIT_MS)
        // RUST_LOG は明示しない（骨格既定 info でよい）。診断は捕捉出力から得る。
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("areka smoke バイナリの起動に失敗しました（{bin}）: {e}"));

    // タイムアウト番犬: try_wait() を短周期でポーリングし、締切内の終了を待つ。
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= WATCHDOG_DEADLINE {
                    // ハング: 子を kill してから明示的に失敗する（マスクしない）。
                    // kill 後に wait_with_output() で子を回収しつつ捕捉出力を診断へ回す。
                    let _ = child.kill();
                    let (out, err) = child
                        .wait_with_output()
                        .map(|o| {
                            (
                                String::from_utf8_lossy(&o.stdout).into_owned(),
                                String::from_utf8_lossy(&o.stderr).into_owned(),
                            )
                        })
                        .unwrap_or_default();
                    panic!(
                        "smoke プロセスが番犬締切（{:?}）内に終了しませんでした（ハング）。\
                         boot→loop→exit の欠陥の疑い。\n--- child stdout ---\n{out}\n--- child stderr ---\n{err}",
                        WATCHDOG_DEADLINE
                    );
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => panic!("子プロセスの状態取得（try_wait）に失敗しました: {e}"),
        }
    };

    // 終了済み: 診断のため出力を回収する。
    let output = child.wait_with_output();
    let (out, err) = output
        .map(|o| {
            (
                String::from_utf8_lossy(&o.stdout).into_owned(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        })
        .unwrap_or_default();

    // 合否判定は exit 0 のみ（良性 teardown warn は見ない・tasks.md Implementation Notes）。
    assert!(
        status.success(),
        "smoke プロセスは exit 0 で終了すべきですが status={status:?} でした。\
         \n--- child stdout ---\n{out}\n--- child stderr ---\n{err}"
    );
}
