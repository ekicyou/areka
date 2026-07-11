//! 骨格 boot→loop→exit の統合 smoke テスト（task 4.2・R4.1/R2.4、task 6.2 で両方向へ拡張）。
//!
//! env ゲート（`AREKA_APP_SMOKE_EXIT_MS`・task 2.3）を立てた areka バイナリの子プロセスを
//! 起動し、起動窓を開いて `app.run()` ループを回した後に自動 despawn → `WindowRegistry`
//! 空遷移 → `run()` 復帰 → **exit 0** で正常終了する経路を実プロセスで踏破・証明する。
//!
//! window-placement task 6.2（`open_startup_window` 差し替え・要件 1.4）以降は **両方向**を張る:
//! - **フォールバック方向**（引数なし）: 既定プレースホルダ root は不在 → `warn!` の上で
//!   検証用ダミー窓へフォールバックして完走する（DD14）
//! - **本物方向**（emo2 fixture パスを引数で供給）: `prepare_ghost_windows` 成功 →
//!   本物のゴースト窓構成（2 スコープ×キャラ窓＋バルーン窓）で完走する
//!
//! 実装規律:
//! - 子プロセスは `cargo run` ではなく Cargo が用意する `CARGO_BIN_EXE_areka`（統合テスト用に
//!   Cargo が bin を先にビルドして渡すパス）を直接起動する。再コンパイルによるノイズを避ける。
//! - タイムアウト番犬は純 std（`std::process` + `std::thread`/`std::time`・新規依存なし・R6.1）。
//!   `try_wait()` を短周期でポーリングし、寛大な締切内に終了しなければ `kill()` してテスト失敗。
//! - **合否は exit 0＋経路マーカー（tracing の info/warn メッセージ本文）で判定する**。
//!   良性の teardown warn（`WARN ... Could not despawn entity ... generation 1`・smoke timer と
//!   window-registry close の二重 despawn 競合）は warn レベルで exit 0 に影響しないため
//!   assert 対象にしない（tasks.md Implementation Notes・実測済み）。

use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
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

/// emo2 fixture ルート（placement 単体テストと同一アンカー規約・task 4.1）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// env ゲートを立てた areka バイナリを与えた引数で起動し、番犬締切内の終了を待って
/// `(status, stdout, stderr)` を返す共通ドライバ（両方向テストで共有）。
fn run_smoke(args: &[&str]) -> (ExitStatus, String, String) {
    let bin = env!("CARGO_BIN_EXE_areka");

    // 子プロセス起動: env ゲートを立て、診断用に stdout/stderr を捕捉する。
    // stdout/stderr を piped にすることで、失敗時に子の tracing 出力を assert メッセージへ回せる。
    let mut child = Command::new(bin)
        .args(args)
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

    // 終了済み: 診断・経路マーカー assert のため出力を回収する。
    let output = child.wait_with_output();
    let (out, err) = output
        .map(|o| {
            (
                String::from_utf8_lossy(&o.stdout).into_owned(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        })
        .unwrap_or_default();

    (status, out, err)
}

/// フォールバック方向（task 6.2・DD14）: 引数なし＝既定プレースホルダ root は不在のため、
/// `warn!`（`StartPointMissing` は良性分類）の上で検証用ダミー窓へフォールバックし、
/// 自動 close → 空遷移 → **exit 0** で完走することを実プロセスで証明する。
#[test]
fn skeleton_boots_loops_and_exits_zero_within_watchdog() {
    let (status, out, err) = run_smoke(&[]);

    assert!(
        status.success(),
        "smoke プロセスは exit 0 で終了すべきですが status={status:?} でした。\
         \n--- child stdout ---\n{out}\n--- child stderr ---\n{err}"
    );

    // 経路マーカー（task 6.2 拡張）: warn! フォールバック → ダミー窓（本物窓は開かない）。
    // tracing のメッセージ本文はフィールド着色（ANSI）に分断されない（決定論）。
    let all = format!("{out}\n{err}");
    assert!(
        all.contains("窓配置の準備起点が見つかりません"),
        "フォールバック方向は StartPointMissing の warn! を出すべき。\
         \n--- child output ---\n{all}"
    );
    assert!(
        all.contains("検証用ダミー窓を開きました（placement フォールバック）"),
        "フォールバック方向はダミー窓を開くべき。\n--- child output ---\n{all}"
    );
    assert!(
        !all.contains("本物のゴースト窓を開きました"),
        "フォールバック方向で本物のゴースト窓が開くのは契約外。\n--- child output ---\n{all}"
    );
}

/// 本物方向（task 6.2・要件 1.4 の観測可能な完了状態）: emo2 fixture のパスを位置引数で
/// 供給し、`prepare_ghost_windows` 成功 → 本物のゴースト窓構成（2 スコープ）で
/// 自動 close → 空遷移 → **exit 0** で完走することを実プロセスで証明する。
///
/// 環境寛容（placement mod.rs `prepare_ghost_windows_uses_primary_monitor` と同流儀）:
/// モニタ 0 台の headless 環境では `PlacementError::Monitor` → `error!` フォールバックが
/// 契約どおりの挙動のため、その場合のみフォールバック完走を受理する（fixture は in-repo
/// ゆえ `StartPointMissing` は起き得ない＝それ以外のフォールバックは失敗として扱う）。
#[test]
fn skeleton_boots_with_real_ghost_windows_and_exits_zero() {
    let ghost_root = emo2_root();
    let balloon_root = ghost_root.join("emo2-kakukaku");
    assert!(
        ghost_root.join("ghost/master/descript.txt").exists(),
        "emo2 fixture が見つかりません（in-repo 前提）: {}",
        ghost_root.display()
    );

    let (status, out, err) = run_smoke(&[
        ghost_root.to_str().expect("fixture パスは UTF-8"),
        balloon_root.to_str().expect("fixture パスは UTF-8"),
    ]);

    assert!(
        status.success(),
        "smoke プロセスは exit 0 で終了すべきですが status={status:?} でした。\
         \n--- child stdout ---\n{out}\n--- child stderr ---\n{err}"
    );

    let all = format!("{out}\n{err}");
    if all.contains("窓配置の準備に失敗しました") && all.contains("モニタ") {
        // モニタ 0 台環境（headless CI）: Monitor エラー→ error! フォールバックが契約どおり。
        eprintln!("note: モニタ 0 台環境のため本物方向はフォールバック完走で受理（Monitor エラー）");
        assert!(
            all.contains("検証用ダミー窓を開きました（placement フォールバック）"),
            "Monitor エラー時はダミー窓フォールバックで完走すべき。\n--- child output ---\n{all}"
        );
        return;
    }

    // 通常環境: 本物のゴースト窓構成（scopes=[0, 1]）で完走し、フォールバックしない。
    assert!(
        all.contains("本物のゴースト窓を開きました"),
        "fixture あり環境では本物のゴースト窓を開くべき。\n--- child output ---\n{all}"
    );
    assert!(
        !all.contains("フォールバックします"),
        "fixture あり環境でフォールバックが発火するのは契約外。\n--- child output ---\n{all}"
    );
}
