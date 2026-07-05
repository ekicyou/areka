//! 強制 kill 注入 → 異常検出 → 有限復帰 → 統一報告分類 → 冪等二重 kill の end-to-end テスト
//! （task 5.3・design.md Testing Strategy #9「lifecycle_kill_e2e.rs::kill_injection_detection_and_reporting」）。
//!
//! **実 i686 helper プロセス**を spawn し、**実 i686 testdll（`shiori.dll`）**を
//! `LoadLibraryW` させたうえで、ベースラインの GET 往復成功（healthy round-trip）を確認した後、
//! **強制終了（`TerminateProcess` 相当）を注入**し、以下を **1 つの `#[test]` 関数**で実証する:
//! - ① kill 後、bounded ポーリングで **異常な終了種別**（helper プロセス由来の非正常終了・R4.1）が検出される。
//! - ② その後の request が **無限待ちにならず**、有限時間内に **観測可能なエラー**として返る（R4.2）。
//! - ③ そのエラーから **統一報告が死活起因（`HelperDown`）として分類**される（R2.1/2.5・死優先）。
//! - ④ **二重の強制終了要求が冪等に成功**として扱われる（R5.2）。
//! - ⑤（design #9 bonus）異常後の後始末の決定性: 既終了への `request_clean_shutdown` は
//!   UNLOAD を送出せず観測済み非 Clean kind へ短絡する（R5.3）。
//!
//! 検証フロー（design.md System Flows「強制 kill 注入と統一報告」）:
//! ```text
//! status()==Running → terminate()（kill 注入）→ bounded poll status()→Exited(Abnormal|Terminated)
//! → get(dummy)→Err(Ipc|Timeout)（SMTO 有限復帰）→ report_failure(err)→HelperDown(kind)
//! → 二重 terminate()→Ok（冪等）→ request_clean_shutdown→Ok(非 Clean kind)（短絡）
//! ```
//!
//! **1 窓制約対処**: 親 message-only 窓は同一プロセスで 2 組独立生成すると 2 組目が失敗する既知制約ゆえ、
//! 周期連打テスト（`lifecycle_cyclic_e2e.rs`）とは**別のテストバイナリ**（＝別プロセス＝単一窓）とする。
//!
//! **有限復帰の worst-case を短く決定化する env**: SMTO が dead HWND 上で即時 fast-fail せず timeout まで
//! 待つ場合に備え、テスト先頭で `AREKA_SHIORI_REQUEST_TIMEOUT_MS=3000` を設定し worst-case を 3s に縛る
//! （`Shiori3Client` の実効 timeout がこの env を読む）。本テストは単一テストバイナリ（別プロセス）ゆえ、
//! この process-global な `set_var` は他テストバイナリへ漏れない（`cargo test --test <name>` は各統合
//! テストバイナリを別プロセスで実行する）。
//!
//! fixture 契約値は testdll 側 `TEST_GET_ID`／固定 Value と一致するが、host テストクレートは testdll へ
//! 依存しないため、**文字列 ID と期待 Value をハードコード**する（testdll の契約に忠実追随）。
//!
//! ## testdll / helper の所在解決（無言スキップで緑を偽装しない・R7.4）
//! env override 最優先 → `CARGO_MANIFEST_DIR` から `target/i686-pc-windows-msvc/{debug,release}/`
//! を探索 → 不在は**明確な panic**（i686 未ビルドの指摘）。silent-skip（緑の偽装）を禁ずる。
//!
//! ## 実行前提（PowerShell・Git Bash 不可・2 段ビルド→x64 test）
//! ```powershell
//! cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc
//! cargo build -p shiori-host32-helper  --target i686-pc-windows-msvc
//! cargo test  -p shiori-host32-host    --test lifecycle_kill_e2e
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{
    ExitKind, FailureClass, HelperLifecycle, HelperStatus, ParentMessageWindow, RequestError,
    Shiori3Client, spawn,
};
use shiori_host32_ipc::MsgTag;

/// ハンドシェイク（HELLO 受領）の上限時間。i686 helper の起動＋窓生成＋HELLO 送出に十分な余裕。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// GET(`OnTestValue`) への fixture 固定 Value（testdll の `RESP_GET_200` の `Value:` 行と一致）。
///
/// さくらスクリプトリテラル（バックスラッシュは通常 ASCII バイト）。raw 文字列で literal backslash を保つ。
/// host テストクレートは testdll へ依存しないため契約値をここへハードコードする。
const EXPECTED_GET_VALUE: &str = r"\0\s[0]host32 request roundtrip ok\e";

/// GET request の `ID`（testdll `TEST_GET_ID` と一致・ハードコード・イベント意味論を持たないダミー ID）。
const TEST_GET_ID: &str = "OnTestValue";

/// kill 後の request が有限復帰する上限（R4.2・無限待ちなし）。
///
/// worst-case は先頭で設定する `AREKA_SHIORI_REQUEST_TIMEOUT_MS=3000`（3s）＋SMTO_ABORTIFHUNG で
/// 縛られるため、10s は安全なマージン付き上界。これを超えたら「有限復帰していない」＝fail。
const BOUNDED_LIMIT: Duration = Duration::from_secs(10);

/// kill 後の終了観測（bounded poll）の上限。helper プロセスの実際の終了までの余裕。
const EXIT_OBSERVE_LIMIT: Duration = Duration::from_secs(10);

/// bounded poll の刻み（実時間 sleep はこれのみ・R7.5）。
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// helper exe（事前ビルドした i686 バイナリ）のパスを堅牢に解決する（shiori_request_e2e 同型）。
///
/// 優先順位:
/// 1. env `HOST32_HELPER_EXE`（明示指定）。
/// 2. `CARGO_MANIFEST_DIR`（= `crates/shiori-host32-host`）→ ワークスペースルート →
///    `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`。
///
/// いずれも見つからなければ**明確な panic で fail**（無言スキップで緑を偽装しない・R7.4）。
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

/// testdll（事前ビルドした i686 `shiori.dll` fixture）のパスを堅牢に解決する（shiori_request_e2e 同型）。
///
/// 優先順位:
/// 1. env `HOST32_TESTDLL_DLL`（明示指定）。
/// 2. `CARGO_MANIFEST_DIR` → ワークスペースルート →
///    `target/i686-pc-windows-msvc/{debug,release}/shiori.dll`。
///
/// いずれも見つからなければ**明確な panic で fail**（i686 testdll 未ビルドの指摘・無言スキップ禁止・R7.4）。
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
    let dir = std::env::temp_dir().join(format!("host32-kill-e2e-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("一時 dir の作成に失敗");
    dir
}

/// 強制 kill 注入 → 異常検出 → 有限復帰 → 統一報告分類 → 冪等二重 kill の E2E
/// （task 5.3・design.md Testing Strategy #9・System Flows「強制 kill 注入と統一報告」・
/// R4.1/4.2/4.3, R2.1/2.5, R5.2/5.3）。
///
/// PRIMARY 構造: 単一の親窓＋helper を用い、ベースライン往復成功を確認したうえで強制終了を注入し、
/// 異常検出・有限復帰・統一報告分類・冪等二重終了要求のすべてを **1 つの `#[test]`** で実証する。
/// `HelperLifecycle` は handle を単独所有し、panic 経路では Drop で冪等 terminate するため
/// 別途 HelperGuard は不要（親窓は使用後に明示 drop）。
///
/// bounded: pump=HANDSHAKE_TIMEOUT、LOAD ack=LOAD_ACK_TIMEOUT、kill 後 GET=env 3s timeout＋
/// SMTO_ABORTIFHUNG（BOUNDED_LIMIT=10s で assert）、終了観測=EXIT_OBSERVE_LIMIT で必ず有限復帰する
/// ため、テスト全体もハングしない（数秒で完了する）。
#[test]
fn kill_injection_detection_and_reporting() {
    // --- 有限復帰の worst-case を短く決定化（R4.2）: dead HWND 上で SMTO が fast-fail せず timeout
    //     まで待つ場合に備え、request timeout を 3s に縛る。client 構築の前に設定する必要がある
    //     （client は構築ごとに env から実効 timeout を解決する）。単一テストバイナリ（別プロセス）
    //     ゆえこの process-global set_var は他テストバイナリへ漏れない。---
    unsafe {
        std::env::set_var("AREKA_SHIORI_REQUEST_TIMEOUT_MS", "3000");
    }

    let helper_exe = resolve_helper_exe();
    let testdll = resolve_testdll();

    // --- load_dir に testdll を shiori.dll としてコピー（helper の cwd＝DLL 探索起点）---
    let load_dir = make_unique_temp_dir("kill");
    let dll_path = load_dir.join("shiori.dll");
    std::fs::copy(&testdll, &dll_path).expect("testdll を load_dir へコピーできない");

    // --- ① 親 message-only 窓（同時 1 窓厳守・独立テストバイナリなので本プロセスで唯一）---
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    let handle = spawn(&helper_exe, &load_dir, "shiori.dll", parent_hwnd)
        .expect("i686 helper の spawn に失敗（helper exe を確認）");
    // HelperLifecycle は handle を単独所有（by value）。Drop で冪等 terminate するため
    // 別途 HelperGuard 不要（panic 経路でもプロセスリークしない）。
    let mut lifecycle = HelperLifecycle::new(handle);

    // --- HELLO 受領で helper HWND 確定＝ハンドシェイク完了（helper 不在は resolve_helper_exe が既に panic 済み）---
    let helper_hwnd = parent.pump_until_hello_or(HANDSHAKE_TIMEOUT);
    assert!(
        helper_hwnd.is_some(),
        "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完・helper 起動を確認）"
    );

    // --- ② LOAD 先行（helper 内に proxy を確立＝REQUEST の構造的前提・Load-before-Request 不変条件）---
    let load_ack = parent
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .expect("LOAD の send_request が失敗（ack 未達・proxy 未確立では REQUEST 不能）");
    assert_eq!(
        load_ack,
        vec![1u8],
        "LOAD 成功 ack [1]（proxy 確立）を先に得る（REQUEST の前提）"
    );

    // --- request 出口 API を構築（ハンドシェイク＋LOAD 済みの親窓を借用・env timeout 3s を解決済み）---
    let client = Shiori3Client::new(&parent);

    // --- ③ baseline GET: kill 前の healthy round-trip を確認（fixture 固定 Value を返す）---
    let baseline = client
        .get(TEST_GET_ID, &[])
        .expect("baseline GET(OnTestValue) が Err（kill 前は健全に往復するべき）");
    assert_eq!(
        baseline,
        Some(EXPECTED_GET_VALUE.to_string()),
        "baseline GET は fixture 固定 Value を返す（kill 前の healthy round-trip の証明）"
    );
    // kill 前は稼働中である（R1.1）。
    assert!(
        matches!(lifecycle.status(), HelperStatus::Running),
        "kill 注入前は helper が稼働中である（Running）"
    );

    // --- ④ 強制 kill 注入（R5.2）: 稼働中の helper を強制終了する ---
    lifecycle
        .terminate()
        .expect("kill injection should succeed");

    // --- ⑤ 異常検出（R4.1）: bounded poll で非正常な終了種別を検出する ---
    let deadline = Instant::now() + EXIT_OBSERVE_LIMIT;
    let kind = loop {
        if let HelperStatus::Exited(kind) = lifecycle.status() {
            break kind;
        }
        assert!(
            Instant::now() < deadline,
            "kill 注入後、上限時間内に helper の終了を観測できなかった（R4.1）"
        );
        std::thread::sleep(EXIT_POLL_INTERVAL);
    };
    // Windows std の Child::kill は TerminateProcess(_, 1) → 通常 Abnormal(1)。
    // Terminated（code 取得不能）も強制終了経路として受容する。いずれにせよ Clean ではない（R4.1）。
    assert!(
        matches!(kind, ExitKind::Abnormal(_) | ExitKind::Terminated),
        "強制終了の種別は Abnormal または Terminated（非正常・R4.1）: got {kind:?}"
    );
    assert_ne!(
        kind,
        ExitKind::Clean,
        "強制終了は正常終了（Clean）ではない（R4.1）: got {kind:?}"
    );
    eprintln!("観測された強制終了種別（R4.1）: {kind:?}");

    // --- ⑥ 有限復帰（R4.2）: kill 後の GET が無限待ちにならず、有限時間内に観測可能なエラーで返る ---
    let started = Instant::now();
    let err = client
        .get(TEST_GET_ID, &[])
        .expect_err("post-kill GET must fail (helper dead)");
    let elapsed = started.elapsed();
    // 観測可能なエラー: transport 送出失敗（Ipc）または wire timeout（R4.2）。
    assert!(
        matches!(err, RequestError::Ipc(_) | RequestError::Timeout),
        "kill 後の GET は観測可能なエラー（Ipc|Timeout）で返る（R4.2）: got {err:?}"
    );
    // 有限復帰: worst-case（env 3s＋SMTO）内に返る（無限待ちなし・R4.2）。
    assert!(
        elapsed < BOUNDED_LIMIT,
        "kill 後の GET は有限時間内に返る（無限待ちなし・R4.2）: 経過 {elapsed:?} >= 上限 {BOUNDED_LIMIT:?}"
    );
    eprintln!("kill 後 GET の観測エラー（R4.2）: {err:?}（経過 {elapsed:?}）");

    // --- ⑦ 統一報告分類（R2.1/2.5）: 死活起因（HelperDown・非 Clean）として分類される ---
    //     表面の err が Ipc/Timeout でも、観測された終了が死優先で HelperDown へ写る（death precedence）。
    let report = lifecycle.report_failure(err);
    assert!(
        matches!(report.class, FailureClass::HelperDown(k) if k != ExitKind::Clean),
        "統一報告は死活起因（HelperDown・非 Clean）として分類される（R2.1/2.5・死優先）: got {:?}",
        report.class
    );
    // 原本 error は単一不透明へ潰さず報告に保持される（R2.4）。
    assert!(
        matches!(report.error, RequestError::Ipc(_) | RequestError::Timeout),
        "report は原本 RequestError（Ipc|Timeout）を保持する（R2.4）: got {:?}",
        report.error
    );

    // --- ⑧ 冪等二重 kill（R5.2）: 既終了プロセスへの二重の強制終了要求は両方 Ok ---
    assert!(
        lifecycle.terminate().is_ok(),
        "既終了 helper への 2 回目の terminate は Ok（冪等・R5.2）"
    );
    assert!(
        lifecycle.terminate().is_ok(),
        "既終了 helper への 3 回目の terminate も Ok（冪等・R5.2）"
    );

    // --- ⑨（design #9 bonus・R5.3）異常後後始末の決定性: 既終了への request_clean_shutdown は
    //     UNLOAD を送出せず観測済み非 Clean kind へ短絡する（既終了への終了要求を失敗させない）---
    drop(client); // `&parent` 借用を解放してから shutdown 経路へ進む（借用終端の明示）。
    let kind2 = lifecycle
        .request_clean_shutdown(&parent)
        .expect("short-circuit on already-exited");
    assert_ne!(
        kind2,
        ExitKind::Clean,
        "既終了（強制 kill）への request_clean_shutdown は観測済み非 Clean kind へ短絡する（UNLOAD 非送出・R5.3）: got {kind2:?}"
    );

    // --- 親窓を明示 drop（同時生存親窓を高々 1 に保つ）→ best-effort cleanup ---
    drop(parent);
    let _ = std::fs::remove_dir_all(&load_dir);
}
