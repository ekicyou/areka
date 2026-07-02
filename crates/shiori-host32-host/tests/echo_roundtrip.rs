//! 往復 echo 統合テスト（task 5.1・Requirement 6 の M1 ゲート指標）。
//!
//! **実 i686 helper プロセスを spawn** し、**実 WM_COPYDATA 往復**を検証する
//! （単体テストの `classify_inbound` / loopback は同一プロセス内・本テストは
//! 跨プロセスの本物往復）。design.md の System Flows シーケンス
//! （create→spawn→HELLO→pump_until_hello_or→send_request→RESPONSE→poll_exit_kind）
//! をそのまま辿る:
//!
//! 1. [`ParentMessageWindow::create`] で親 message-only 窓を先に立てる。
//! 2. [`shiori_host32_host::spawn`] で i686 helper を spawn（親 HWND を u32 で渡す）。
//! 3. [`ParentMessageWindow::pump_until_hello_or`] で HELLO を受領し helper HWND を確定
//!    （ハンドシェイク完了・要件 3.x）。
//! 4. 非自明なバイナリ含みの request bytes を [`ParentMessageWindow::send_request`] で送出し、
//!    RESPONSE として同一 bytes を受領・照合する（要件 6.1 / 6.2）。空 payload・複数回往復も確認。
//! 5. [`shiori_host32_host::poll_exit_kind`] が `None`（helper 稼働中＝無クラッシュ）を返すことを確認
//!    （要件 6.3・両プロセス生存）。
//! 6. cleanup: [`HelperHandle::terminate`] で helper を確実に終了（プロセスリーク防止）。
//!
//! すべての pump / send_request は上限時間で bounded に復帰する（`SMTO_ABORTIFHUNG` ＋
//! 上限時間）ため、テスト全体もハングしない（要件 4.4・無デッドロック）。
//!
//! ## helper exe の所在解決（無言スキップで緑を偽装しない）
//! env `HOST32_HELPER_EXE` を最優先で見て、無ければ `CARGO_MANIFEST_DIR` から
//! ワークスペース `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`
//! を探す。見つからない場合は「先に i686 helper をビルドせよ」という明確な panic で fail する。
//!
//! ## 実行前提（PowerShell・Git Bash 不可）
//! ```powershell
//! cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
//! cargo test  -p shiori-host32-host   --test echo_roundtrip
//! ```

use std::path::PathBuf;
use std::time::{Duration, Instant};

use shiori_host32_host::{ParentMessageWindow, poll_exit_kind, spawn};
use shiori_host32_ipc::MsgTag;

/// ハンドシェイク（HELLO 受領）の上限時間。i686 helper の起動＋窓生成＋HELLO 送出に十分な余裕。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// 1 往復（REQUEST→RESPONSE）の上限時間。echo は即応ゆえ短めで足りるが、余裕を持たせる。
const ROUNDTRIP_TIMEOUT: Duration = Duration::from_secs(5);

/// spawn した helper を Drop 時に必ず terminate する scope guard（panic 時もプロセスリークを防ぐ）。
///
/// [`HelperHandle::terminate`] は冪等（終了済みでも `Ok`）ゆえ、正常終了・panic のどちらでも安全に呼べる。
struct HelperGuard {
    handle: shiori_host32_host::HelperHandle,
}

impl Drop for HelperGuard {
    fn drop(&mut self) {
        // 冪等: 既に終了していても Ok。panic unwinding 中でも helper を確実に片付ける。
        let _ = self.handle.terminate();
    }
}

/// helper exe（事前ビルドした i686 バイナリ）のパスを堅牢に解決する。
///
/// 優先順位:
/// 1. env `HOST32_HELPER_EXE`（明示指定・CI やカスタム target-dir 向け）。
/// 2. `CARGO_MANIFEST_DIR`（= `crates/shiori-host32-host`）からワークスペースルートへ上り、
///    `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe` を探索。
///
/// いずれも見つからなければ、i686 helper 未ビルドと判断して**明確な panic で fail**する
/// （無言スキップで緑を偽装しない）。
fn resolve_helper_exe() -> PathBuf {
    // (1) env override 最優先。
    if let Ok(explicit) = std::env::var("HOST32_HELPER_EXE") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return p;
        }
        panic!(
            "HOST32_HELPER_EXE={explicit:?} が指すファイルが存在しません。\
             i686 helper を先にビルドし正しいパスを指してください:\n  \
             cargo build -p shiori-host32-helper --target i686-pc-windows-msvc"
        );
    }

    // (2) CARGO_MANIFEST_DIR（crates/shiori-host32-host）→ ワークスペースルート（親の親）。
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // ワークスペースルート
        .unwrap_or(&manifest_dir);
    let target_base = workspace_root.join("target").join("i686-pc-windows-msvc");

    for profile in ["debug", "release"] {
        let candidate = target_base
            .join(profile)
            .join("shiori-host32-helper.exe");
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "i686 helper exe が見つかりません（探索先: {}\\{{debug,release}}\\shiori-host32-helper.exe）。\
         PowerShell で先に i686 helper をビルドしてください（Git Bash 不可）:\n  \
         cargo build -p shiori-host32-helper --target i686-pc-windows-msvc\n\
         あるいは env HOST32_HELPER_EXE で exe パスを明示してください。",
        target_base.display()
    );
}

/// ghostdir（helper の作業ディレクトリ）に使う既存ディレクトリを返す。
///
/// helper は cwd を ghostdir に設定して起動されるだけで、本 echo 実証では内容を読まない。
/// 存在する一時ディレクトリで足りる。
fn ghostdir() -> PathBuf {
    std::env::temp_dir()
}

/// 実 helper を spawn して 1 組の親窓で往復 echo を検証する統合テスト
/// （要件 6.1 / 6.2 / 6.3 / 4.4・design.md System Flows / Integration Tests 1）。
///
/// wintf の message-only 窓は同一プロセスで 2 組独立生成すると 2 組目が失敗する既知制約ゆえ、
/// 本 x64 テストプロセスで親窓は 1 組のみ生成する（helper 窓は別プロセスゆえ無関係）。
/// テストは単一関数に集約する。
#[test]
fn echo_roundtrip_over_real_helper_process() {
    let helper_exe = resolve_helper_exe();
    let ghostdir = ghostdir();

    // --- (1) 親 message-only 窓を先に立てる（HELLO の受け皿）---
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    // --- (2) i686 helper を spawn（親 HWND を u32 で渡す・cross-task 契約）---
    let handle = spawn(&helper_exe, &ghostdir, parent_hwnd)
        .expect("i686 helper の spawn に失敗（helper exe を確認）");
    // spawn 直後から Drop guard で確実に片付ける（以降の panic でも terminate される）。
    let mut guard = HelperGuard { handle };

    // --- (3) HELLO 受領で helper HWND 確定＝ハンドシェイク完了（要件 3.x）---
    let helper_hwnd = parent.pump_until_hello_or(HANDSHAKE_TIMEOUT);
    assert!(
        helper_hwnd.is_some(),
        "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完・helper 起動を確認）"
    );

    // --- (4) 往復 echo: 非自明なバイナリ含み payload を送り、同一 bytes を RESPONSE として受領・照合 ---
    // 要件 6.1（request→対応 response）/ 6.2（照合可能な観測）。
    let payloads: [&[u8]; 3] = [
        b"echo-me-\x00\x01\xfe\xff-payload", // 非自明・NUL/高位バイト含むバイナリ
        b"",                                 // 空 payload（0 バイト往復）
        b"\x00\x00\x00\x00second-round\xaa\xbb", // 2 回目の往復（single-in-flight の連続往復）
    ];

    let overall_start = Instant::now();
    for (i, &payload) in payloads.iter().enumerate() {
        let before = Instant::now();
        let response = parent
            .send_request(MsgTag::Request, payload, ROUNDTRIP_TIMEOUT)
            .unwrap_or_else(|e| panic!("往復 {i} の send_request が失敗（要件 6.1/4.4）: {e:?}"));
        assert_eq!(
            response,
            payload.to_vec(),
            "往復 {i}: 受領 response bytes が送出 request bytes と一致しない（echo・要件 6.1/6.2）"
        );
        assert!(
            before.elapsed() < Duration::from_secs(10),
            "往復 {i}: send_request が上限時間で有限復帰しない（無デッドロック・要件 4.4）"
        );
    }
    assert!(
        overall_start.elapsed() < Duration::from_secs(30),
        "往復 echo 全体がハングせず bounded に完了する（要件 4.4）"
    );

    // --- (5) helper 稼働中＝無クラッシュ・両プロセス生存（要件 6.3）---
    let exit = poll_exit_kind(&mut guard.handle);
    assert!(
        exit.is_none(),
        "echo 往復後も helper は稼働中である（クラッシュしていない・要件 6.3）: got {exit:?}"
    );

    // --- (6) cleanup: guard の Drop で helper を terminate（明示 drop でリーク防止を確定）---
    drop(guard);
    drop(parent);
}
