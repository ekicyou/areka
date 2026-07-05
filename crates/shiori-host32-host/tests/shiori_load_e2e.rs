//! 実プロセス LOAD E2E（task 7.1・design.md「#### LoadE2E」／System Flows WS-A LOAD）。
//!
//! **実 i686 helper プロセス**を spawn し、**実 i686 testdll（`shiori.dll`）**を
//! `LoadLibraryW` させて **LOAD の成否 ack を跨プロセスで観測**する。3 ケースを 1 つの
//! `#[test]` 関数に集約し、各ケースで親 message-only 窓を新規 `create()`→使用→明示 `drop`
//! することで、**同時生存する親窓を高々 1 つ**に保つ（wintf の message-only 窓は同一プロセスで
//! 2 組独立生成すると 2 組目が失敗する既知制約・shiori_request_e2e/parent_window 参照）。
//!
//! 検証系列（design.md「検証系列」①〜④）:
//! - ① 成功 ack `[1]`（testdll を `load_dir\shiori.dll` へコピー・cwd=load_dir 慣習）→ R5.3/7.3
//! - ② env `HOST32_TESTDLL_LOAD_FAIL=1` 強制失敗 ack `[0]`（testdll の `load`→false）→ R6.3/7.3
//! - ③ DLL 不在（空 load_dir）ack `[0]`（`LoadLibraryW` 失敗）→ R6.1
//! - ④ 各失敗後の helper 生存（`poll_exit_kind`→`None`）→ R6.4
//!
//! LOAD ack 待機は推奨既定 timeout `LOAD_ACK_TIMEOUT`（親 per-call・凍結 timeout 機構は不変・R5.4）。
//!
//! ## 実 pasta 追験（⑤・任意）
//! env `HOST32_PASTA_DLL`（DLL フルパス）が設定されている場合のみ、実 pasta を追加検証する
//! （load_dir=親 dir・shiori_name=ファイル名・**指定 DLL 不在なら明示 fail**・R7.4/7.5）。
//! 未設定なら silent に skip（CI 必須ゲートにしない・R7.6）。testdll/helper とは扱いが異なる
//! （testdll/helper 不在は panic、pasta env 未設定は任意ゆえ skip）。
//!
//! ## testdll / helper の所在解決（無言スキップで緑を偽装しない・R13.2）
//! env override 最優先 → `CARGO_MANIFEST_DIR` から `target/i686-pc-windows-msvc/{debug,release}/`
//! を探索 → 不在は**明確な panic**（i686 未ビルドの指摘）。
//!
//! ## 実行前提（PowerShell・Git Bash 不可・2 段ビルド→x64 test）
//! ```powershell
//! cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc
//! cargo build -p shiori-host32-helper  --target i686-pc-windows-msvc
//! cargo test  -p shiori-host32-host    --test shiori_load_e2e
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{ParentMessageWindow, poll_exit_kind, spawn};
use shiori_host32_ipc::MsgTag;

/// ハンドシェイク（HELLO 受領）の上限時間。i686 helper の起動＋窓生成＋HELLO 送出に十分な余裕。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

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

/// helper exe（事前ビルドした i686 バイナリ）のパスを堅牢に解決する（shiori_request_e2e 同型）。
///
/// 優先順位:
/// 1. env `HOST32_HELPER_EXE`（明示指定）。
/// 2. `CARGO_MANIFEST_DIR`（= `crates/shiori-host32-host`）→ ワークスペースルート →
///    `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`。
///
/// いずれも見つからなければ**明確な panic で fail**（無言スキップで緑を偽装しない）。
fn resolve_helper_exe() -> PathBuf {
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

    let target_base = workspace_i686_target_base();
    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join("shiori-host32-helper.exe");
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

/// testdll（事前ビルドした i686 `shiori.dll` fixture）のパスを堅牢に解決する（task 5.2 と同型）。
///
/// 優先順位:
/// 1. env `HOST32_TESTDLL_DLL`（明示指定）。
/// 2. `CARGO_MANIFEST_DIR` → ワークスペースルート →
///    `target/i686-pc-windows-msvc/{debug,release}/shiori.dll`。
///
/// いずれも見つからなければ**明確な panic で fail**（i686 testdll 未ビルドの指摘・無言スキップ禁止）。
fn resolve_testdll() -> PathBuf {
    if let Ok(explicit) = std::env::var("HOST32_TESTDLL_DLL") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return p;
        }
        panic!(
            "HOST32_TESTDLL_DLL={explicit:?} が指すファイルが存在しません。\
             i686 testdll を先にビルドし正しいパスを指してください:\n  \
             cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc"
        );
    }

    let target_base = workspace_i686_target_base();
    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join("shiori.dll");
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "i686 testdll（shiori.dll）が見つかりません（探索先: {}\\{{debug,release}}\\shiori.dll）。\
         PowerShell で先に i686 testdll をビルドしてください（Git Bash 不可）:\n  \
         cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc\n\
         あるいは env HOST32_TESTDLL_DLL で DLL パスを明示してください。",
        target_base.display()
    );
}

/// `CARGO_MANIFEST_DIR`（= `crates/shiori-host32-host`）からワークスペースの
/// `target/i686-pc-windows-msvc` ベースを求める。
fn workspace_i686_target_base() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // ワークスペースルート
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.clone());
    workspace_root.join("target").join("i686-pc-windows-msvc")
}

/// 一意な一時ディレクトリを作る（並行テスト・再実行での衝突回避）。
///
/// `std::env::temp_dir()` 配下に pid＋単調カウンタで一意名を作り、best-effort で作成する。
fn make_unique_temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "host32-load-e2e-{tag}-{}-{n}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("一時 dir の作成に失敗");
    dir
}

/// 1 ケース分の LOAD 往復を実行し、observed ack バイト列を返す。
///
/// - `load_dir`: helper の cwd＝DLL 探索起点。呼び出し側が testdll コピー／空 dir を用意する。
/// - `shiori_name`: 解決する DLL 名（成功/env 失敗は `"shiori.dll"`・不在ケースも同名で不在）。
/// - `set_load_fail`: `true` なら spawn 前に env `HOST32_TESTDLL_LOAD_FAIL=1` を set し、
///   spawn 後に即 remove する（子は生成時 env スナップショットを継承・親 global env 汚染を最小化）。
///
/// 手順: `ParentMessageWindow::create` → `spawn` → HELLO pump → `send_request(Load, LOAD_ACK_TIMEOUT)`
/// → **失敗後の helper 生存を `poll_exit_kind`→`None` で観測** → guard/parent を明示 drop
/// （同時生存親窓を高々 1 に保つ）。ack バイト列を返す。
fn run_load_case(
    helper_exe: &Path,
    load_dir: &Path,
    shiori_name: &str,
    set_load_fail: bool,
) -> Vec<u8> {
    // --- 親 message-only 窓（同時 1 窓厳守・使用後に明示 drop）---
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    // env は spawn 直前に set・spawn 直後に remove（子は生成時スナップショットを保持）。
    if set_load_fail {
        // SAFETY: edition 2024 の set_var は unsafe。テストは単一スレッドで env を
        // 触っており、spawn 前 set→spawn 後 remove の窓を最小化している。
        unsafe {
            std::env::set_var("HOST32_TESTDLL_LOAD_FAIL", "1");
        }
    }

    let handle = spawn(helper_exe, load_dir, shiori_name, parent_hwnd)
        .expect("i686 helper の spawn に失敗（helper exe を確認）");
    let mut guard = HelperGuard { handle };

    if set_load_fail {
        // 子は既に env スナップショットを継承済み。親 global env を即クリーンアップ。
        // SAFETY: 上記 set と対（単一スレッド）。
        unsafe {
            std::env::remove_var("HOST32_TESTDLL_LOAD_FAIL");
        }
    }

    // --- HELLO 受領で helper HWND 確定＝ハンドシェイク完了 ---
    let helper_hwnd = parent.pump_until_hello_or(HANDSHAKE_TIMEOUT);
    assert!(
        helper_hwnd.is_some(),
        "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完・helper 起動を確認）"
    );

    // --- LOAD トリガ → ack（1 byte）を推奨既定 timeout で待機（R5.3/5.4）---
    let ack = parent
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .expect("LOAD の send_request が失敗（ack 未達・要件 5.2/5.3）");

    // --- LOAD 後（成功・失敗いずれも）helper は生存継続（R6.4・no-crash）---
    assert!(
        poll_exit_kind(&mut guard.handle).is_none(),
        "LOAD 後も helper は稼働中である（クラッシュしていない・要件 6.4）"
    );

    // 同時生存親窓を高々 1 に保つため、ここで明示 drop（helper も terminate）。
    drop(guard);
    drop(parent);

    ack
}

/// 実プロセス LOAD E2E: 成功 ack[1]／env 強制失敗 ack[0]／DLL 不在 ack[0]＋各失敗後の生存
/// （design.md「#### LoadE2E」検証系列①〜④・System Flows WS-A LOAD・確定設計判断 (d)）。
///
/// PRIMARY 構造: 3 ケースを 1 関数内で逐次実行し、各ケースで親窓を新規 create→使用→明示 drop
/// することで同時生存親窓を高々 1 に保つ（wintf 制約回避）。
///
/// bounded: pump は HANDSHAKE_TIMEOUT、send_request は LOAD_ACK_TIMEOUT＋SMTO_ABORTIFHUNG で
/// 必ず有限復帰するため、テスト全体もハングしない（要件 4.4）。
#[test]
fn load_e2e_success_fail_survival() {
    let helper_exe = resolve_helper_exe();
    let testdll = resolve_testdll();

    // --- ケース① 成功 ack[1]（testdll を load_dir\shiori.dll へコピー・cwd=load_dir 検証）---
    {
        let load_dir = make_unique_temp_dir("ok");
        let dll_path = load_dir.join("shiori.dll");
        std::fs::copy(&testdll, &dll_path).expect("testdll を load_dir へコピーできない");

        let ack = run_load_case(&helper_exe, &load_dir, "shiori.dll", false);
        assert_eq!(
            ack,
            vec![1u8],
            "ケース①: 成功 LOAD は ack [1] を返す（要件 5.3/7.3）"
        );

        let _ = std::fs::remove_dir_all(&load_dir); // best-effort cleanup
    }

    // --- ケース② env 強制失敗 ack[0]（testdll の load→false）---
    {
        let load_dir = make_unique_temp_dir("fail");
        let dll_path = load_dir.join("shiori.dll");
        std::fs::copy(&testdll, &dll_path).expect("testdll を load_dir へコピーできない");

        // set_load_fail=true: spawn 前に HOST32_TESTDLL_LOAD_FAIL=1 を set→spawn 後 remove。
        let ack = run_load_case(&helper_exe, &load_dir, "shiori.dll", true);
        assert_eq!(
            ack,
            vec![0u8],
            "ケース②: env 強制失敗（load→false）は ack [0] を返す（要件 6.3/7.3）"
        );

        let _ = std::fs::remove_dir_all(&load_dir);
    }

    // --- ケース③ DLL 不在 ack[0]（空 load_dir・LoadLibraryW 失敗）---
    {
        let load_dir = make_unique_temp_dir("absent");
        // shiori.dll を置かない＝LoadLibraryW が失敗する。

        let ack = run_load_case(&helper_exe, &load_dir, "shiori.dll", false);
        assert_eq!(
            ack,
            vec![0u8],
            "ケース③: DLL 不在（LoadLibraryW 失敗）は ack [0] を返す（要件 6.1）"
        );

        let _ = std::fs::remove_dir_all(&load_dir);
    }
}

/// ⑤ 実 pasta 追験（任意・env `HOST32_PASTA_DLL` 設定時のみ）。
///
/// env 値＝DLL フルパス・load_dir＝その親 dir・shiori_name＝ファイル名で実 pasta を LOAD し、
/// ack `[1]` を検証する。**指定 DLL が不在なら明示 fail**（R7.4/7.5）。env 未設定なら silent skip
/// （CI 必須ゲートにしない・R7.6）。testdll/helper 不在は panic だが、pasta は任意ゆえ skip で扱いが異なる。
#[test]
fn load_e2e_real_pasta_optional() {
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        // 未設定＝任意 gate ゆえ skip（silent・R7.6）。testdll/helper とは扱いが異なる。
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 pasta 追験を skip（任意 gate・R7.6）。\
             実 pasta を検証するには env に pasta DLL のフルパスを設定してください。"
        );
        return;
    };

    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail・R7.4/7.5）"
    );

    let helper_exe = resolve_helper_exe();
    let load_dir = dll_path
        .parent()
        .expect("HOST32_PASTA_DLL に親 dir がない")
        .to_path_buf();
    let shiori_name = dll_path
        .file_name()
        .expect("HOST32_PASTA_DLL にファイル名がない")
        .to_string_lossy()
        .into_owned();

    let ack = run_load_case(&helper_exe, &load_dir, &shiori_name, false);
    assert_eq!(
        ack,
        vec![1u8],
        "実 pasta LOAD は ack [1] を返す（load_dir=親 dir・shiori_name=ファイル名・R7.4/7.5）"
    );
}
