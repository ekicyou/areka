//! 決定的 request E2E（task 6.1・design.md「#### shiori_request_e2e」／System Flows GET 往復）。
//!
//! **実 i686 helper プロセス**を spawn し、**実 i686 testdll（`shiori.dll`）**を
//! `LoadLibraryExW` させたうえで、**helper 越しに GET／NOTIFY request を跨プロセスで往復**させ、
//! [`Shiori3Client`] の出口 API が正しい Value を取り出す／204 を破棄することを観測する。
//! GET と NOTIFY を **1 つの `#[test]` 関数**に集約し、単一の親窓＋helper を再利用する
//! （R6.9「両 request line を 1 回の決定的 run で行使」）。親 message-only 窓は同一プロセスで
//! 2 組独立生成すると 2 組目が失敗する既知制約ゆえ、**同時生存する親窓を高々 1 つ**に保つ
//! （使用後に明示 `drop`・shiori_load_e2e.rs / parent_window 参照）。
//!
//! 検証フロー（design.md GET 往復・Load-before-Request 構造不変条件）:
//! - ① 親窓 `create()` → helper spawn → HELLO pump でハンドシェイク完了を観測。
//! - ② **LOAD 先行**（helper 内に proxy を確立＝REQUEST の構造的前提）→ ack `[1]` を assert。
//!   （`Shiori3Client` は LOAD しない＝LOAD は host32-shiori-load の領分ゆえ、ここで明示発行する。）
//! - ③ **GET** `client.get("OnTestValue", &[])` → `Ok(Some(固定 Value))` を assert。これが証明するのは
//!   (a) request が正しく組めた（GET＋`ID: OnTestValue` が fixture へ到達・誤組立なら 400→`Err`）、
//!   (b) Value 抽出が働く、(c) HGLOBAL 所有権往復が二重解放なく完了した（さもなくばテストプロセス abort）。
//! - ④ **NOTIFY** `client.notify("OnTestNotify", &[])` → `Ok(())` を assert（fixture は 204・client は破棄・R4.8/R6.9）。
//! - ⑤ 往復後の helper 生存（`poll_exit_kind`→`None`）を assert（no-crash・R6.4）。
//! - ⑥ handle／parent を明示 drop（同時生存親窓を高々 1 に保つ）。
//!
//! fixture 契約値は testdll 側 `TEST_GET_ID`／`TEST_NOTIFY_ID`／固定 Value と一致するが、host
//! テストクレートは testdll へ依存しないため、**文字列 ID と期待 Value をハードコード**する
//! （crates/shiori-host32-testdll/src/lib.rs の契約に忠実追随）。
//!
//! ## testdll / helper の所在解決（無言スキップで緑を偽装しない・R7.3）
//! env override 最優先 → `CARGO_MANIFEST_DIR` から `target/i686-pc-windows-msvc/{debug,release}/`
//! を探索 → 不在は**明確な panic**（i686 未ビルドの指摘）。silent-skip（緑の偽装）を禁ずる。
//!
//! ## 実行前提（PowerShell・Git Bash 不可・2 段ビルド→x64 test）
//! ```powershell
//! cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc
//! cargo build -p shiori-host32-helper  --target i686-pc-windows-msvc
//! cargo test  -p shiori-host32-host    --test shiori_request_e2e
//! ```

use std::path::PathBuf;

use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{
    Charset, CharsetNegotiator, ParentMessageWindow, Shiori3Client, poll_exit_kind, spawn,
};
use shiori_host32_ipc::MsgTag;

mod common;
use common::{TestRoot, handshake, resolve_helper_exe, resolve_testdll};

// NOTE: GET／NOTIFY 往復の timeout は `Shiori3Client` が env シーム（`AREKA_SHIORI_REQUEST_TIMEOUT_MS`・
// 既定 60s）から内部解決するため、本テストで per-call timeout 定数を持たない（client の効果 timeout に委ねる）。
// 有限復帰は `send_request` の SMTO_ABORTIFHUNG が保証するため、テスト全体もハングしない。

/// GET(`OnTestValue`) への fixture 固定 Value（testdll の `RESP_GET_200` の `Value:` 行と一致）。
///
/// さくらスクリプトリテラル（バックスラッシュは通常 ASCII バイト）。raw 文字列で literal backslash を保つ。
/// host テストクレートは testdll へ依存しないため契約値をここへハードコードする。
const EXPECTED_GET_VALUE: &str = r"\0\s[0]host32 request roundtrip ok\e";

/// GET request の `ID`（testdll `TEST_GET_ID` と一致・ハードコード）。
const TEST_GET_ID: &str = "OnTestValue";

/// NOTIFY request の `ID`（testdll `TEST_NOTIFY_ID` と一致・ハードコード）。
const TEST_NOTIFY_ID: &str = "OnTestNotify";

/// 決定的 request E2E: helper 越し fixture へ GET/NOTIFY を往復し、Value 抽出／204 破棄を観測する
/// （task 6.1・design.md「#### shiori_request_e2e」・System Flows GET 往復・R6.3/6.4/6.9/7.2/7.3）。
///
/// PRIMARY 構造: 単一の親窓＋helper を GET／NOTIFY 双方で再利用し、両 request line を 1 回の
/// 決定的 run で行使する（R6.9）。親窓は create→使用→最後に明示 drop で同時生存を高々 1 に保つ。
///
/// bounded: pump は HANDSHAKE_TIMEOUT、LOAD ack は LOAD_ACK_TIMEOUT、request 往復は REQUEST_TIMEOUT
/// ＋SMTO_ABORTIFHUNG で必ず有限復帰するため、テスト全体もハングしない。
#[test]
fn request_e2e_get_value_and_notify_discard() {
    let helper_exe = resolve_helper_exe();
    let testdll = resolve_testdll();

    // --- load_dir に testdll を shiori.dll としてコピー（helper の cwd＝DLL 探索起点）---
    let root = TestRoot::new("request-e2e-req");
    let load_dir = root.path();
    let dll_path = load_dir.join("shiori.dll");
    std::fs::copy(&testdll, &dll_path).expect("testdll を load_dir へコピーできない");

    // --- ① 親 message-only 窓（同時 1 窓厳守・使用後に明示 drop）---
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    let mut handle = spawn(&helper_exe, load_dir, "shiori.dll", parent_hwnd)
        .expect("i686 helper の spawn に失敗（helper exe を確認）");

    // --- HELLO 受領で helper HWND 確定＝ハンドシェイク完了（silent-skip 禁止: helper 不在は
    //     resolve_helper_exe が既に panic 済み）---
    let helper_hwnd = handshake(&parent, handle.pid());
    assert!(
        helper_hwnd.is_some(),
        "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完・helper 起動を確認）"
    );

    // --- ② LOAD 先行（helper 内に proxy を確立＝REQUEST の構造的前提・Load-before-Request 不変条件）---
    //     Shiori3Client は LOAD しない（LOAD は host32-shiori-load の領分）ため、ここで明示発行する。
    let load_ack = parent
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .expect("LOAD の send_request が失敗（ack 未達・proxy 未確立では REQUEST 不能）");
    assert_eq!(
        load_ack,
        vec![1u8],
        "LOAD 成功 ack [1]（proxy 確立）を先に得る（REQUEST の前提・R7.2）"
    );

    // --- request 出口 API を構築（ハンドシェイク＋LOAD 済みの親窓を借用）---
    // 交渉状態は接続の持ち物（本番は `ShioriConnection`）。本テストは接続の値を組まないので
    // 適用前と同じ初期値（UTF-8・強制なし）をローカルに置く。testdll は `Charset` を見ない。
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let mut client = Shiori3Client::new(&parent, &mut negotiator);

    // --- ③ GET: 固定 Value を取り出す（request 正組立＋Value 抽出＋HGLOBAL 所有権往復の証明・R6.4）---
    //     誤組立なら fixture は 400 を返し get は Err(RequestError::Shiori) へ写る。
    let value = client
        .get(TEST_GET_ID, &[], None)
        .expect("GET(OnTestValue) が Err（request 誤組立なら fixture 400→Err・R6.4）");
    assert_eq!(
        value,
        Some(EXPECTED_GET_VALUE.to_string()),
        "GET は fixture 固定 Value を返す（request line=GET＋ID=OnTestValue が正しく届いた証拠・R6.3/6.9）"
    );

    // --- ④ NOTIFY: 片道イベント（fixture は 204・client は応答を破棄して Ok(())・R4.8/R6.9）---
    //     誤組立なら fixture は 400 を返すが notify は応答 status を破棄する契約ゆえ、
    //     primary assertion は Ok(()) とする（204 決定的経路に依拠）。
    client
        .notify(TEST_NOTIFY_ID, &[], None)
        .expect("NOTIFY(OnTestNotify) は Ok(())（fixture 204 を client が破棄・R4.8/6.9）");

    // --- ⑤ 往復後も helper は生存継続（no-crash・所有権規約無違反の傍証・R6.4）---
    assert!(
        poll_exit_kind(&mut handle).is_none(),
        "GET/NOTIFY 往復後も helper は稼働中である（クラッシュしていない・R6.4）"
    );

    // --- ⑥ 同時生存親窓を高々 1 に保つため、ここで明示 drop（helper も terminate）---
    //     client は `&parent` を借用するだけ（Drop 非実装）で、最後の使用（NOTIFY）以降 borrow は
    //     終了している（NLL）。ゆえに parent を安全に drop できる。handle→parent の順で明示 drop する。
    drop(handle);
    drop(parent);

    // best-effort cleanup（DLL ハンドルは helper terminate で解放済み）。
}

/// env-gated 実 pasta.dll OnBoot 追験（confidence 検証・任意・R6.5/6.6/6.7）。
///
/// 上の決定的テスト（testdll・固定 response）とは扱いが異なる:
/// - env `HOST32_PASTA_DLL`（DLL フルパス）**設定時のみ** 実 pasta へ OnBoot request を送出し、
///   `Value`（起動あいさつのさくらスクリプト本体）の受領を検証する（R6.5）。
/// - env 設定済みだが**指定 DLL が不在**なら明示 fail（silent skip を認めない・R6.6）。
/// - env **未設定**なら silent skip（CI 必須ゲートにしない・R6.7）。この skip は本テスト（任意 env-gate）
///   に限る例外であり、決定的テストの「無言スキップ禁止」規律とは別物である（testdll/helper 不在は
///   `resolve_helper_exe` が引き続き panic する）。
///
/// 構造は決定的テストと同型（HELLO pump → LOAD 先行 ack[1] → `Shiori3Client::new` → OnBoot GET）。
/// load_dir=DLL の親 dir・shiori_name=ファイル名（`load_e2e_real_pasta_optional` と同じ流儀）。
/// OnBoot の Reference0=シェル名（ukadoc）ゆえ emo2 のシェル dir 名 "master" を渡す（妥当な Reference0）。
///
/// bounded: pump=HANDSHAKE_TIMEOUT・LOAD ack=LOAD_ACK_TIMEOUT・GET 往復=client の env timeout
/// ＋SMTO_ABORTIFHUNG で必ず有限復帰する（ハングしない）。
#[test]
fn request_e2e_real_pasta_optional() {
    // --- env `HOST32_PASTA_DLL` 未設定＝任意 gate ゆえ silent skip（R6.7・CI 必須ゲートにしない）---
    //     testdll/helper 不在（panic）とは扱いが異なる（実 pasta は confidence 目的の任意追験）。
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 pasta 追験を skip（任意 gate・R6.7）。\
             実 pasta の OnBoot Value 受領を検証するには env に pasta DLL のフルパスを設定してください。"
        );
        return;
    };

    // --- env 設定済みだが DLL が不在なら明示 fail（silent skip を認めない・R6.6）---
    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail・R6.6）"
    );

    // helper 不在は引き続き panic（silent-skip 禁止・helper 成果物は任意ではない）。
    let helper_exe = resolve_helper_exe();

    // load_dir=DLL の親 dir・shiori_name=ファイル名（load_e2e_real_pasta_optional 同流儀）。
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
    //     決定的テストは実行終了時に自窓を drop 済み・本テストは既定 skip ゆえ、通常 run では
    //     同時に窓生成経路を走る #[test] は 1 つに留まる（shiori_load_e2e と同じ env-gate 前提）。
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    let mut handle = spawn(&helper_exe, &load_dir, &shiori_name, parent_hwnd)
        .expect("i686 helper の spawn に失敗（helper exe を確認）");

    // --- HELLO 受領でハンドシェイク完了（helper 不在は resolve_helper_exe が既に panic 済み）---
    let helper_hwnd = handshake(&parent, handle.pid());
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
        "実 pasta の LOAD 成功 ack [1]（proxy 確立）を先に得る（REQUEST の前提・R6.5）"
    );

    // --- ③ OnBoot GET: 起動あいさつの Value 受領を検証（R6.5）---
    //     OnBoot Reference0=シェル名（ukadoc）ゆえ emo2 のシェル dir 名 "master" を渡す。
    // 交渉状態は接続の持ち物（本番は `ShioriConnection`）。本テストは接続の値を組まないので
    // 適用前と同じ初期値（UTF-8・強制なし）をローカルに置く。testdll は `Charset` を見ない。
    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let mut client = Shiori3Client::new(&parent, &mut negotiator);
    let value = client
        .get("OnBoot", &["master".to_string()], None)
        .expect("OnBoot GET が Err（実 pasta との request 往復が失敗・R6.5）");

    // OnBoot は起動あいさつのさくらスクリプト Value を返す＝Some かつ非空を assert（R6.5）。
    let value =
        value.expect("OnBoot は Value（起動あいさつ）を返すべき（Ok(None) ではない・R6.5）");
    assert!(
        !value.is_empty(),
        "OnBoot の Value は非空である（起動あいさつのさくらスクリプト本体・R6.5）"
    );
    eprintln!("実 pasta OnBoot Value 受領（R6.5）: {value:?}");

    // --- ④ 往復後も helper は生存継続（no-crash の傍証）---
    assert!(
        poll_exit_kind(&mut handle).is_none(),
        "OnBoot 往復後も helper は稼働中である（クラッシュしていない）"
    );

    // --- ⑤ 同時生存親窓を高々 1 に保つため明示 drop（handle→parent の順）---
    drop(handle);
    drop(parent);
}
