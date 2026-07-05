//! 周期運転（連打）＋正規 clean shutdown の end-to-end テスト
//! （task 5.1・design.md Testing Strategy #8「lifecycle_cyclic_e2e.rs::cyclic_run_and_clean_shutdown」）。
//!
//! **実 i686 helper プロセス**を spawn し、**実 i686 testdll（`shiori.dll`）**を
//! `LoadLibraryW` させたうえで、ハンドシェイク → LOAD 先行 → **固定応答の往復を 200 回連続
//! （実時間 sleep なしの back-to-back）** で行い、各往復の成功と fixture 固定応答を確認する。
//! 反復後も helper が生存継続していることを確認したうえで、**正規の正常終了経路**
//! （UNLOAD → ループ正常終了 → `exit(0)`）を通じて **正常終了種別（`ExitKind::Clean`）**が
//! 観測されることを、1 つの `#[test]` 関数で実証する。
//!
//! 検証フロー（design.md System Flows「正規の正常終了経路」・Testing Strategy #8）:
//! - ① 親窓 `create()` → helper spawn → HELLO pump でハンドシェイク完了を観測。
//! - ② **LOAD 先行**（helper 内に proxy を確立＝REQUEST の構造的前提）→ ack `[1]` を assert。
//! - ③ **周期運転**（REPETITIONS=200・back-to-back・sleep なし）: 各反復で GET が fixture 固定
//!   Value を返す（R3.2/3.3）／NOTIFY が Ok（R3.2）／`status()==Running`（helper 生存・R3.4）を assert。
//! - ④ ループ後も `status()==Running`（200 反復を生き延びた・R3.4）を assert。
//! - ⑤ **正規 clean shutdown**: `request_clean_shutdown` が `ExitKind::Clean` を返す
//!   （実 helper の UNLOAD arm 経由で exit code 0＝正規の正常終了経路の成立証拠・R5.1/R5.3）。
//!
//! R3.5 の最小十分 assert 集合（design 決定）: 200×2 往復の全成功 ＋ ループ後 `status()==Running`
//! ＋ clean shutdown が `Clean` を返すこと。OS ハンドル数の計数は非決定的ゆえ **assert しない**。
//! 実時間 sleep も **入れない**（R7.5・有限復帰は凍結 SMTO timeout ＋ request timeout に乗る）。
//!
//! fixture 契約値は testdll 側 `TEST_GET_ID`／`TEST_NOTIFY_ID`／固定 Value と一致するが、host
//! テストクレートは testdll へ依存しないため、**文字列 ID と期待 Value をハードコード**する
//! （crates/shiori-host32-testdll/src/lib.rs の契約に忠実追随）。
//!
//! ## testdll / helper の所在解決（無言スキップで緑を偽装しない・R7.4）
//! env override 最優先 → `CARGO_MANIFEST_DIR` から `target/i686-pc-windows-msvc/{debug,release}/`
//! を探索 → 不在は**明確な panic**（i686 未ビルドの指摘）。silent-skip（緑の偽装）を禁ずる。
//!
//! ## 二窓 discipline（R6.3・design Testing Strategy #10）
//! 本ファイルには **windowed テストが 2 本**存在する:
//! - `cyclic_run_and_clean_shutdown`（task 5.1・決定的・常に窓を作る）
//! - `cyclic_real_pasta_optional`（task 5.2・env-gate・`HOST32_PASTA_DLL` **設定時のみ**窓を作る）
//!
//! 親 message-only 窓は同一プロセスで同時 2 組生成不可（既知の 1 窓＝2 組制約）。ゆえに
//! **`HOST32_PASTA_DLL` を設定して実行する場合は必ず `--test-threads=1` で直列化**すること
//! （下記 PowerShell 手順に明記）。CI（env 未設定）では `cyclic_real_pasta_optional` が
//! **窓生成前に早期 return** するため、通常の並行 `cargo test` でも 2 窓が同時生存せず衝突しない。
//!
//! ## 実行前提（PowerShell・Git Bash 不可・2 段ビルド→x64 test）
//! ```powershell
//! cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc
//! cargo build -p shiori-host32-helper  --target i686-pc-windows-msvc
//! cargo test  -p shiori-host32-host    --test lifecycle_cyclic_e2e
//!
//! # env-gate 実 pasta 追験（任意・R6）を含める場合は必ず --test-threads=1 で直列化（2 窓制約）:
//! $env:HOST32_PASTA_DLL="C:\path\to\pasta.dll"
//! cargo test -p shiori-host32-host --test lifecycle_cyclic_e2e -- --test-threads=1
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{
    ExitKind, HelperLifecycle, HelperStatus, ParentMessageWindow, Shiori3Client, spawn,
};
use shiori_host32_ipc::MsgTag;

/// ハンドシェイク（HELLO 受領）の上限時間。i686 helper の起動＋窓生成＋HELLO 送出に十分な余裕。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// 周期運転の反復回数（連打・back-to-back・sleep なし・R3.1/R7.5）。
const REPETITIONS: usize = 200;

/// 実 pasta 追験（`cyclic_real_pasta_optional`）の NOTIFY 連打回数（応答内容非依存・R6.1/design #10）。
const N_PASTA: usize = 300;

// NOTE: GET／NOTIFY 往復の timeout は `Shiori3Client` が env シーム（`AREKA_SHIORI_REQUEST_TIMEOUT_MS`・
// 既定 60s）から内部解決するため、本テストで per-call timeout 定数を持たない（client の効果 timeout に委ねる）。
// 有限復帰は `send_request` の SMTO_ABORTIFHUNG が保証するため、テスト全体もハングしない。

/// GET(`OnTestValue`) への fixture 固定 Value（testdll の `RESP_GET_200` の `Value:` 行と一致）。
///
/// さくらスクリプトリテラル（バックスラッシュは通常 ASCII バイト）。raw 文字列で literal backslash を保つ。
/// host テストクレートは testdll へ依存しないため契約値をここへハードコードする。
const EXPECTED_GET_VALUE: &str = r"\0\s[0]host32 request roundtrip ok\e";

/// GET request の `ID`（testdll `TEST_GET_ID` と一致・ハードコード・イベント意味論を持たないダミー ID）。
const TEST_GET_ID: &str = "OnTestValue";

/// NOTIFY request の `ID`（testdll `TEST_NOTIFY_ID` と一致・ハードコード）。
const TEST_NOTIFY_ID: &str = "OnTestNotify";

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
    let dir = std::env::temp_dir().join(format!("host32-cyclic-e2e-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("一時 dir の作成に失敗");
    dir
}

/// 周期運転（200 回連打）＋正規 clean shutdown の E2E（task 5.1・design.md Testing Strategy #8）。
///
/// PRIMARY 構造: 単一の親窓＋helper を 200 回の GET/NOTIFY 往復で再利用し（back-to-back・sleep なし）、
/// 反復後の helper 生存を確認したうえで正規の正常終了経路（UNLOAD → 正常終了 → exit0）を発行し、
/// `ExitKind::Clean` を観測する。`HelperLifecycle` は handle を単独所有し、panic 経路では Drop で
/// 冪等 terminate するため、別途 HelperGuard は不要（親窓は使用後に明示 drop）。
///
/// bounded: pump は HANDSHAKE_TIMEOUT、LOAD/UNLOAD ack は各 ACK_TIMEOUT、request 往復は client の
/// env timeout ＋SMTO_ABORTIFHUNG、終了観測は EXIT_OBSERVE_TIMEOUT で必ず有限復帰するため、
/// テスト全体もハングしない（数秒で完了する）。
#[test]
fn cyclic_run_and_clean_shutdown() {
    let helper_exe = resolve_helper_exe();
    let testdll = resolve_testdll();

    // --- load_dir に testdll を shiori.dll としてコピー（helper の cwd＝DLL 探索起点）---
    let load_dir = make_unique_temp_dir("cyclic");
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

    // --- request 出口 API を構築（ハンドシェイク＋LOAD 済みの親窓を借用）---
    let client = Shiori3Client::new(&parent);

    // --- ③ 周期運転: REPETITIONS 回の GET/NOTIFY 往復（back-to-back・実時間 sleep なし・R3.1/R7.5）---
    for i in 0..REPETITIONS {
        // GET: 固定 Value を取り出す（request 正組立＋Value 抽出＋所有権往復の証明・R3.2/3.3）。
        let value = client
            .get(TEST_GET_ID, &[])
            .unwrap_or_else(|e| panic!("反復 {i}: GET(OnTestValue) が Err（{e:?}）"));
        assert_eq!(
            value,
            Some(EXPECTED_GET_VALUE.to_string()),
            "反復 {i}: GET は fixture 固定 Value を返す（R3.2/3.3）"
        );

        // NOTIFY: 片道イベント（fixture は 204・client は応答を破棄して Ok(())・R3.2）。
        client
            .notify(TEST_NOTIFY_ID, &[])
            .unwrap_or_else(|e| panic!("反復 {i}: NOTIFY(OnTestNotify) が Err（{e:?}）"));

        // helper 生存継続（sticky・非ブロッキング status()・R3.4）。
        assert!(
            matches!(lifecycle.status(), HelperStatus::Running),
            "反復 {i}: 往復中も helper は稼働継続している（R3.4）"
        );
    }

    // --- ④ 200 反復を生き延びた（R3.4）---
    assert!(
        matches!(lifecycle.status(), HelperStatus::Running),
        "全 {REPETITIONS} 反復後も helper は稼働継続している（連打を生き延びた・R3.4）"
    );

    // --- client（`&parent` 借用）を解放してから shutdown へ進む（clean・借用終端の明示）---
    drop(client);

    // --- ⑤ 正規の正常終了経路: UNLOAD → ループ正常終了 → exit(0) → ExitKind::Clean を観測（R5.1/R5.3）---
    let kind = lifecycle
        .request_clean_shutdown(&parent)
        .expect("clean shutdown should succeed");
    assert_eq!(
        kind,
        ExitKind::Clean,
        "正規の正常終了経路が exit code 0（Clean）を生む（実 helper の UNLOAD arm 経由・R5.1/R5.3）"
    );

    // --- 親窓を明示 drop（同時生存親窓を高々 1 に保つ）→ best-effort cleanup ---
    drop(parent);
    let _ = std::fs::remove_dir_all(&load_dir);
}

/// 実 pasta.dll への周期連打 confidence テスト（env-gate・任意・R6.1〜6.3・design Testing Strategy #10）。
///
/// 上の決定的テスト（testdll・固定 response）とは扱いが異なる:
/// - env `HOST32_PASTA_DLL`（DLL フルパス）**未設定**なら明示 skip（eprintln・CI 必須ゲートにしない・R6.3）。
///   この skip は本テスト（任意 env-gate）に限る例外であり、決定的テストの「無言スキップ禁止」規律とは
///   別物である（helper 不在は `resolve_helper_exe` が引き続き panic する）。
/// - env 設定済みだが**指定 DLL が不在**なら明示 fail（silent skip を認めない・R6.2）。
/// - env 設定済みで**存在**する場合は `N_PASTA = 300` 回の `notify("OnSecondChange", ..)` 連打を行い
///   （応答内容非依存＝notify は破棄契約ゆえ実 DLL の応答内容に依らず **transport 健全性のみ**を観測。
///   イベント運行の意味論検証はしない＝kanade 領分・R6.1）、helper 生存を確認したうえで
///   **正規の clean shutdown**（UNLOAD → 正常終了 → exit0）で `ExitKind::Clean` を観測する（confidence・R6.1）。
///
/// 構造は決定的テストと同型（HELLO pump → LOAD 先行 ack[1] → `Shiori3Client::new` → 連打 → clean shutdown）。
/// load_dir=DLL の親 dir・shiori_name=ファイル名（実 pasta は自身の dir に住むため。request 系
/// `request_e2e_real_pasta_optional` と同じ流儀）＝load_dir は削除しない（本テストが作った temp ではない）。
///
/// **2 窓 discipline**: 本テストは cyclic（`cyclic_run_and_clean_shutdown`）と同一バイナリの windowed
/// テスト 2 本目にあたる。env `HOST32_PASTA_DLL` 設定時の実行は必ず `--test-threads=1` で直列化すること
/// （1 窓＝2 組制約対処・ファイル冒頭 doc の PowerShell 手順に明記）。env 未設定（CI）では下記の早期
/// return が窓生成の**前**に発火するため、通常の並行 `cargo test` でも 2 窓が同時生存せず衝突しない。
///
/// bounded: pump=HANDSHAKE_TIMEOUT・LOAD ack=LOAD_ACK_TIMEOUT・notify 往復=client の env timeout
/// ＋SMTO_ABORTIFHUNG・終了観測=`request_clean_shutdown` 内 timeout で必ず有限復帰する（ハングしない）。
#[test]
fn cyclic_real_pasta_optional() {
    // --- env `HOST32_PASTA_DLL` 未設定＝任意 gate ゆえ明示 skip（R6.3・CI 必須ゲートにしない）---
    //     この早期 return は「窓生成の前」に起きる。ゆえに CI（env 未設定）では本テストは窓を作らず、
    //     並行実行中の `cyclic_run_and_clean_shutdown` と 2 窓が同時生存することはない（衝突しない）。
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 pasta 追験を skip（任意 gate・R6.3）。\
             実 pasta への周期連打＋正規 clean shutdown を検証するには env に pasta DLL の\
             フルパスを設定し、`--test-threads=1` で直列実行してください（2 窓制約）。"
        );
        return;
    };

    // --- env 設定済みだが DLL が不在なら明示 fail（silent skip を認めない・R6.2）---
    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（明示 fail・R6.2）"
    );

    // helper 不在は引き続き panic（silent-skip 禁止・helper 成果物は任意ではない）。
    let helper_exe = resolve_helper_exe();

    // load_dir=DLL の親 dir・shiori_name=ファイル名（実 pasta は自身の dir に住む・削除しない）。
    let load_dir = dll_path
        .parent()
        .expect("HOST32_PASTA_DLL に親 dir がない")
        .to_path_buf();
    let shiori_name = dll_path
        .file_name()
        .expect("HOST32_PASTA_DLL にファイル名がない")
        .to_string_lossy()
        .into_owned();

    // --- ① 親 message-only 窓（同時 1 窓厳守・使用後に明示 drop）---
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    let handle = spawn(&helper_exe, &load_dir, &shiori_name, parent_hwnd)
        .expect("i686 helper の spawn に失敗（helper exe を確認）");
    // HelperLifecycle は handle を単独所有（by value）。Drop で冪等 terminate するため
    // 別途 HelperGuard 不要（panic 経路でもプロセスリークしない）。
    let mut lifecycle = HelperLifecycle::new(handle);

    // --- HELLO 受領でハンドシェイク完了（helper 不在は resolve_helper_exe が既に panic 済み）---
    let helper_hwnd = parent.pump_until_hello_or(HANDSHAKE_TIMEOUT);
    assert!(
        helper_hwnd.is_some(),
        "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完・helper 起動を確認）"
    );

    // --- ② LOAD 先行（実 pasta の proxy を確立＝REQUEST の構造的前提）→ ack[1] を assert ---
    let load_ack = parent
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .expect("LOAD の send_request が失敗（ack 未達・proxy 未確立では REQUEST 不能）");
    assert_eq!(
        load_ack,
        vec![1u8],
        "実 pasta の LOAD 成功 ack [1]（proxy 確立）を先に得る（REQUEST の前提・R6.1）"
    );

    // --- request 出口 API を構築（ハンドシェイク＋LOAD 済みの親窓を借用）---
    let client = Shiori3Client::new(&parent);

    // --- ③ 周期運転: N_PASTA 回の NOTIFY 連打（応答内容非依存＝transport 健全性のみ観測・R6.1）---
    //     NOTIFY は応答を破棄する契約ゆえ実 pasta の応答内容に依らない。イベント運行の意味論検証は
    //     しない（kanade 領分）。連打が Err なく通ることと helper 生存継続を観測する。
    for i in 0..N_PASTA {
        // notify transport が成功すること（応答 status は破棄・内容に assert しない・R6.1）。
        client
            .notify("OnSecondChange", &[])
            .unwrap_or_else(|e| panic!("反復 {i}: notify(OnSecondChange) の transport が Err（{e:?}）"));

        // helper 生存継続（sticky・非ブロッキング status()）。
        assert!(
            matches!(lifecycle.status(), HelperStatus::Running),
            "反復 {i}: 連打中も実 pasta helper は稼働継続している（R6.1）"
        );
    }

    // --- ④ N_PASTA 連打を生き延びた ---
    assert!(
        matches!(lifecycle.status(), HelperStatus::Running),
        "全 {N_PASTA} 連打後も実 pasta helper は稼働継続している（R6.1）"
    );

    // --- client（`&parent` 借用）を解放してから shutdown へ進む（clean・借用終端の明示）---
    drop(client);

    // --- ⑤ 正規の clean shutdown: UNLOAD → 正常終了 → exit0 → ExitKind::Clean を観測（confidence・R6.1）---
    let kind = lifecycle
        .request_clean_shutdown(&parent)
        .expect("real pasta clean shutdown");
    assert_eq!(
        kind,
        ExitKind::Clean,
        "実 pasta も正規正常終了経路で Clean（confidence・R6.1）"
    );
    eprintln!(
        "実 pasta 周期連打（{N_PASTA} 回 NOTIFY）＋正規 clean shutdown 完走（Clean・R6.1）: {pasta_dll:?}"
    );

    // --- 親窓を明示 drop（同時生存親窓を高々 1 に保つ）---
    //     load_dir は実 pasta 自身の dir ゆえ削除しない（本テストが作った temp ではない）。
    drop(parent);
}
