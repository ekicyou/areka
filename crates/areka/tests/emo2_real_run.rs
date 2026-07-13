//! 実 pasta 実走テスト（tasks.md task 7.2・R9.1/R9.2/R9.3）＋人間サインオフ手順。
//!
//! 「emo2 が起動して喋る」最終統合（M-boot）を、**実 pasta.dll・実表示・実 DPI（≠96）**で
//! 任意に追験するための env-gate opt-in ハーネス。観測の二段（決定論 spine ＋ 本 env-gate 実走）の
//! うち後者に相当し、決定論 spine（`crates/areka/src/emo2_boot/spine.rs`）が headless GPU で
//! 押さえ切れない「実表示・実 helper・実 DPI」の最終確認を人間判断に委ねる。
//!
//! ## 既定 OFF（R9.2・DoD 非前提）
//! 環境変数 [`REAL_RUN_ENV`]（`AREKA_EMO2_REAL_RUN`）が **未設定／空／空白のみ／非 UTF-8** のときは
//! 即座に `return`（skip）する。ゆえに常設スイート（`cargo test --workspace`・外部 CI なし・
//! ローカル DoD ゲート）では**実 helper／実表示に一切依存せず自明に緑**を保つ。この skip 経路こそが
//! 常設スイートでの観測可能な完了条件であり、実走そのものは DoD の前提にしない（人間 opt-in の追験）。
//!
//! ## 実走（env セット時・R9.1）
//! `CARGO_BIN_EXE_areka`（Cargo が統合テスト用に先ビルドして渡す**実バイナリ**のパス）を、
//! emo2 fixture を位置引数に子プロセス起動する（smoke ドナー `tests/smoke_boot_loop_exit.rs` と同型・
//! 純 std `std::process::Command`＋番犬・**新規依存を追加しない**・R10.5）:
//! - argv[1] = ghost_root（`crates/pilot/examples/shiori-host-32/fixtures/emo2`）
//! - argv[2] = balloon_root（同 fixture 下 `emo2/emo2-kakukaku`）
//! - env `AREKA_APP_SMOKE_EXIT_MS` = [`SMOKE_EXIT_MS`]（自動 close で exit 0 を無人観測可能にする）
//! - env `RUST_LOG` = `info`（wire 成立／attach 完了マーカーは `info!` ゆえ確実に捕捉する）
//!
//! 捕捉した stdout＋stderr に対し **(1) exit 0（自動 close→正常終了）・(2) wire 成立マーカー・
//! (3) attach 完了マーカー** を assert する（design.md「E2E / Smoke」の R9.1 自動部）。
//!
//! ## 実走の前提（人間が opt-in 時に用意する・既定 OFF 経路は非依存）
//! - **実 pasta helper**: `default_helper_exe_path()`（`crates/areka/src/main.rs`）は
//!   `current_exe()` 隣の `shiori-host32-helper.exe` を解決する。実バイナリは
//!   `target/<profile>/areka.exe` ゆえ、**`target/<profile>/shiori-host32-helper.exe` に実 helper を
//!   配置**しておく必要がある（i686 host-32 成果物を
//!   `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` で先にビルドし、実バイナリ
//!   隣へコピーする）。boot は actor spawn で完了し SHIORI 接続の成否を待たない（実測でも helper LOAD
//!   失敗より前に wire 成立へ到達する）ため、helper 不在でも wire 成立マーカー自体は出るが、実発話・
//!   実 typewriter の**目視サインオフ**には実 helper が要る。
//! - **実表示・実 DPI（≠96）**: attach 完了マーカーは `GhostWindows`＋GPU 資源ゲート成立フレームで
//!   初めて発火する。モニタ 0 台の headless 環境ではゴースト窓がダミー窓へフォールバックしゲートが
//!   成立せず attach 完了マーカーが出ない（＝本テストは headless では成立しない）。これは env-gate
//!   opt-in の設計意図どおりであり、実表示のある実機でのみ緑になる。

use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// 実走を有効化する opt-in 環境変数（未設定／空なら即 skip・R9.2・DoD 非前提）。
const REAL_RUN_ENV: &str = "AREKA_EMO2_REAL_RUN";

/// 子プロセスへ渡す自動 close 遅延（ms）。実表示・実 GPU の初期化と初回 attach フレーム到達、
/// および OnBoot トークの流れ始めを無人でも観測できる程度に、smoke ドナー（500ms）より寛大に取る。
const SMOKE_EXIT_MS: &str = "1500";

/// タイムアウト番犬の締切。実 pasta helper の LOAD・実表示初期化・OnClose 握手（再生完了待ち）を
/// 含むため smoke ドナー（60s）より寛大に取る。これを超えて終了しなければハング＝テスト失敗と判定する。
const WATCHDOG_DEADLINE: Duration = Duration::from_secs(120);

/// `try_wait()` のポーリング周期（smoke ドナーと同一）。
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// emo2 fixture ルート（smoke ドナー `emo2_root` と同一アンカー規約・`CARGO_MANIFEST_DIR` 相対）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

// ===========================================================================
// 実 DPI 人間サインオフ手順（R9.3）
// ===========================================================================
//
// **マイルストーン（M-boot）完了は、この人間サインオフを経てのみ宣言される。AI 単独では宣言しない**
// （R9.3）。本ファイルの自動 assert（exit 0・wire 成立・attach 完了マーカー）は R9.1 の「自動部」を
// 押さえるに過ぎず、以下の目視確認（R9.1 の実 DPI・実発話・実ドラッグ）は人間が実機で行う。
//
// ## 前提
// - 実表示のある実機で、**画面 DPI を 96 以外（例 125%/150%）**に設定する（実 DPI≠96・R9.1）。
// - 実 pasta helper（`shiori-host32-helper.exe`）を実バイナリ隣へ配置する（本ファイル冒頭 doc 参照）。
//
// ## 目視チェックリスト（4 点・全て満たして初めてサインオフ）
//   1. **実サーフェス表示位置**: 起動だけで emo2 の実サーフェス（立ち絵）が既定位置に表示される
//      （不可視のまま・座標破綻・画面外沈み込みが無い・R1.1）。
//   2. **typewriter 進行**: OnBoot 応答トークがバルーンへ typewriter 進行で流れる（一括表示でなく
//      1 文字ずつ・R2.1/R2.2）。
//   3. **ドラッグ追従**: マスコットをドラッグするとサーフェスがポインタへ追従する（window-placement
//      のドラッグ機構・R9.1）。
//   4. **close→静かな終了**: 窓 close で OnClose 応答を経てから全エンジンが静かに正常終了する
//      （会話が途中で切れず・後片付けが行われる・R6.1/R6.2）。
//
// ## サインオフ実行の実際
// - 上記 1〜4 のうち **2（typewriter）・3（ドラッグ）・4（OnClose 経由の終了）は、窓が開いたまま
//   人間が観察・操作する必要がある**。自動 assert 用の `AREKA_APP_SMOKE_EXIT_MS`（自動 close）は
//   **付けずに**実バイナリを直接起動して目視する:
//
//       target\<profile>\areka.exe <emo2 ghost_root> <emo2 balloon_root>
//
//   （`<emo2 ghost_root>` = `crates\pilot\examples\shiori-host-32\fixtures\emo2`、
//     `<emo2 balloon_root>` = 同下 `emo2-kakukaku`）。観察後に窓を閉じて 4 を確認する。
// - 本ファイルの `#[test]`（`AREKA_EMO2_REAL_RUN=1` で起動）は、これと独立に**自動 close→exit 0＋
//   マーカー**の機械可読部を押さえる。両者（自動部＋人間目視部）が揃ってサインオフ成立とする。

/// 実 pasta 実走の自動観測（R9.1 自動部）。既定 OFF（R9.2）で、env セット時のみ実バイナリを
/// 子プロセス起動し **exit 0・wire 成立マーカー・attach 完了マーカー** を assert する。
///
/// env 未設定／空のときは即 skip（`return`）するため、常設 `cargo test --workspace` では実 helper／
/// 実表示に依存せず高速に緑を保つ（この skip が常設スイートでの観測可能な完了条件）。実 DPI・実発話・
/// 実ドラッグの目視サインオフ（R9.3）は本ファイル上部の「実 DPI 人間サインオフ手順」に従い人間が行う。
#[test]
fn emo2_real_run_boots_talks_and_exits_zero() {
    // 既定 OFF ゲート（R9.2・DoD 非前提）: 未設定／空／空白のみ／非 UTF-8 は全て skip 扱いで即 return。
    // これにより実 helper／実表示のない常設スイートでも本テストは自明に緑（＝観測可能な完了条件）。
    match std::env::var(REAL_RUN_ENV) {
        Ok(v) if !v.trim().is_empty() => { /* env セット済み → 実走へ進む */ }
        _ => {
            eprintln!(
                "emo2_real_run: skipped (opt-in; set {REAL_RUN_ENV}=1 to run the real-pasta / real-DPI verification)"
            );
            return;
        }
    }

    // --- 実走（R9.1）: emo2 fixture を位置引数に実バイナリを起動する ---
    let ghost_root = emo2_root();
    let balloon_root = ghost_root.join("emo2-kakukaku");
    assert!(
        ghost_root.join("ghost/master/descript.txt").exists(),
        "emo2 fixture が見つかりません（in-repo 前提）: {}",
        ghost_root.display()
    );

    let bin = env!("CARGO_BIN_EXE_areka");
    let mut child = Command::new(bin)
        .arg(ghost_root.to_str().expect("fixture パスは UTF-8"))
        .arg(balloon_root.to_str().expect("fixture パスは UTF-8"))
        // 自動 close（無人で exit 0 を観測可能にする・R6.4/R9.1）。
        .env("AREKA_APP_SMOKE_EXIT_MS", SMOKE_EXIT_MS)
        // wire 成立／attach 完了マーカーは info! ゆえ RUST_LOG=info を明示し確実に捕捉する
        // （人間の shell の RUST_LOG が warn 等でも取りこぼさない）。
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("areka 実バイナリの起動に失敗しました（{bin}）: {e}"));

    // タイムアウト番犬: try_wait() を短周期でポーリングし、締切内の終了を待つ（純 std・新規依存なし）。
    let start = Instant::now();
    let status: ExitStatus = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= WATCHDOG_DEADLINE {
                    // ハング: 子を kill して回収しつつ捕捉出力を診断へ回し、明示的に失敗する（マスクしない）。
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
                        "実走プロセスが番犬締切（{WATCHDOG_DEADLINE:?}）内に終了しませんでした（ハング）。\
                         boot→talk→close の欠陥、または実 helper／実表示の欠落の疑い。\
                         \n--- child stdout ---\n{out}\n--- child stderr ---\n{err}"
                    );
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => panic!("子プロセスの状態取得（try_wait）に失敗しました: {e}"),
        }
    };

    // 終了済み: 診断・マーカー assert のため出力を回収する。
    let (out, err) = child
        .wait_with_output()
        .map(|o| {
            (
                String::from_utf8_lossy(&o.stdout).into_owned(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            )
        })
        .unwrap_or_default();
    let all = format!("{out}\n{err}");

    // (1) exit 0: 自動 close→空遷移→run() 復帰→終了握手（shutdown＋seriko join）まで正常終了（R6.3/R6.4/R9.1）。
    assert!(
        status.success(),
        "実走プロセスは exit 0 で終了すべきですが status={status:?} でした。\
         \n--- child stdout ---\n{out}\n--- child stderr ---\n{err}"
    );

    // (2) wire 成立マーカー（`emo2_boot::wire_emo2_boot`＝mod.rs・実 sink 結線成立時に info! 発火・R7.3
    //     観測境界）。実 fixture 経路では構築入力組立→spawn_seriko→areka_ghost::boot が成立し wired=true。
    assert!(
        all.contains("emo2-boot: 実 sink 結線が成立しました（wire 成立）"),
        "実走は emo2-boot の wire 成立マーカー（実 sink 結線成立）を残すべき。\
         \n--- child output ---\n{all}"
    );

    // (3) attach 完了マーカー（`emo2_boot::frame::run_attach_phase`＝frame.rs・GPU 資源＋GhostWindows
    //     ゲート成立フレームで装着計画を実行し info! 発火・R1.2/DD-12）。実表示への装着が end-to-end で
    //     少なくとも 1 回踏まれた証跡（headless/モニタ 0 台ではダミー窓フォールバックゆえ出ない＝
    //     実表示のある実機でのみ緑＝env-gate opt-in の設計意図どおり）。
    assert!(
        all.contains("emo2 attach: 装着計画を実行"),
        "実走は emo2 attach 完了マーカー（実表示への装着計画実行）を残すべき。\
         \n--- child output ---\n{all}"
    );
}
