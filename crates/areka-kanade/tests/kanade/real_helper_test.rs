//! env-gate 実 helper 追験・単一 `#[test]`。
//!
//! 実 32bit helper＋実 pasta.dll 越しに、real shiori アクター（`src/shiori/real.rs`）を通して
//! 起動→毎秒 Tick 数回→close の運行完走を追験する。応答内容は実会話エンジン依存ゆえ、
//! スクリプトの中身ではなく「Value か 204 のいずれかで運行が完走する」ことを検証する。
//!
//! # env-gate
//! `HOST32_PASTA_DLL`（pasta DLL のフルパス）が未設定なら silent skip（`return`・CI 必須ゲート
//! にしない）。この env 名・skip の語法は `crates/shiori-host32-host/tests/shiori_request_e2e.rs`
//! （`request_e2e_real_pasta_optional`）／`shiori_load_e2e.rs`（`load_e2e_real_pasta_optional`）と
//! 同じ。env 設定済みだが DLL 不在なら明示 fail（silent skip を認めない）。
//!
//! # 接続手順は connect クロージャが所有する
//! spawn→HELLO pump→LOAD（`send_request(MsgTag::Load, ..)`）の接続確立手順は kanade の責務外で、
//! テスト側の connect クロージャが自前結線する。手順は上記の既存 E2E
//! （`ParentMessageWindow::create` → `spawn` → `pump_until_hello_or` → `send_request(Load)`）と同じ。
//! 成功すれば `Box<dyn ShioriBackend>`（実体は `ShioriConnection`）を返し、失敗は `Err(String)`
//! （→ `ShioriDown`）で返す。

use std::path::PathBuf;
use std::time::Duration;

use areka_kanade::{
    CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriBackend, ShioriConnection,
    spawn_kanade, spawn_shiori_actor,
};
use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{Charset, CharsetNegotiator, HelperLifecycle, ParentMessageWindow, spawn};
use shiori_host32_ipc::MsgTag;

use super::common::{DEFAULT_TIMEOUT, QuitPolicy, join_bounded, spawn_mock_sakura};

/// HELLO 受領の上限時間（既存 E2E の `HANDSHAKE_TIMEOUT` と同値）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// helper exe（事前ビルドした i686 バイナリ）のパスを解決する。env `HOST32_HELPER_EXE` 最優先 →
/// ワークスペース `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`。
/// いずれも不在なら `Err(String)`（connect 失敗として ShioriDown へ写る）。
///
/// 同じ探し方の写しが他のクレートにある（`crates/areka-ghost/tests/ghost/common.rs`・
/// `crates/shiori-host32-host/tests/` の 5 本・`crates/pilot/examples/shiori-host-32/main.rs`）。
fn resolve_helper_exe() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("HOST32_HELPER_EXE") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "HOST32_HELPER_EXE={explicit:?} が指すファイルが存在しません（i686 helper を先にビルド）"
        ));
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // ワークスペースルート
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

/// 実 helper への接続手順（spawn→HELLO pump→LOAD）を実行し接続済み [`ShioriConnection`] を返す。
///
/// 手順は既存 host32 E2E（`shiori_request_e2e::request_e2e_real_pasta_optional`）と同じ:
/// 1. `ParentMessageWindow::create()` で親 message-only 窓を生成。
/// 2. `spawn(helper_exe, load_dir, shiori_name, parent_hwnd)` で i686 helper を起動。
/// 3. `pump_until_hello_or(HANDSHAKE_TIMEOUT)` で HELLO 受領を観測。
/// 4. `send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)` で LOAD を先行発行し ack `[1]` を確認。
///
/// いずれかの段で失敗すれば `Err(String)`（ShioriDown へ写る）。成功時は `Box<dyn ShioriBackend>`
/// （実体は `ShioriConnection`）を返す。`HelperLifecycle` の drop で子プロセスが片付く。
///
/// # 引数
/// - `helper_exe`: i686 helper 実行ファイル。
/// - `load_dir`: helper の cwd＝DLL 探索起点（pasta DLL の親 dir）。
/// - `shiori_name`: 解決する DLL 名（pasta DLL のファイル名）。
fn connect_real_helper(
    helper_exe: PathBuf,
    load_dir: PathBuf,
    shiori_name: String,
) -> Result<Box<dyn ShioriBackend>, String> {
    // 親 message-only 窓は !Send で、アクタースレッド上で生成される。
    let window =
        ParentMessageWindow::create().map_err(|e| format!("親 message-only 窓生成に失敗: {e}"))?;
    let parent_hwnd = window.hwnd_u32();

    let helper = spawn(&helper_exe, &load_dir, &shiori_name, parent_hwnd)
        .map_err(|e| format!("i686 helper の spawn に失敗: {e}"))?;

    let helper_hwnd = window.pump_until_hello_or(HANDSHAKE_TIMEOUT);
    if helper_hwnd.is_none() {
        return Err(
            "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完）".to_string(),
        );
    }

    // LOAD は helper 内に proxy を確立する REQUEST の前提で、kanade の責務外ゆえ接続手順で明示発行する。
    let load_ack = window
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .map_err(|e| format!("LOAD の send_request が失敗（ack 未達）: {e}"))?;
    if load_ack != vec![1u8] {
        return Err(format!(
            "実 pasta の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: {load_ack:?}"
        ));
    }

    Ok(Box::new(ShioriConnection {
        window,
        helper: HelperLifecycle::new(helper),
        // 採取フィクスチャ側も本番の接続手続き（`areka-ghost` の `shiori_wiring`）と同じ初期値を
        // 置く（UTF-8 始まり・強制なし）。以後の文字コードは相手の応答の `Charset` ヘッダ次第で、
        // ヘッダが無いか `UTF-8` を名乗る限り UTF-8 のまま変わらない。
        negotiator: CharsetNegotiator::new(Charset::UTF_8, false),
        version: shiori_host32_host::ShioriVersion::Shiori3,
    }))
}

/// env-gate 実 helper 追験。
///
/// `HOST32_PASTA_DLL` 未設定なら silent skip（`shiori_request_e2e::request_e2e_real_pasta_optional`
/// と同じ任意 gate）。設定時は実 32bit helper＋実 pasta 越しに real shiori アクター経由で
/// 起動→毎秒 Tick 数回→close の運行完走を追験する。
///
/// # 検証対象
/// 応答内容は実会話エンジン依存ゆえ、スクリプトの内容自体は検証しない。「Value か 204 のいずれ
/// かで運行が完走する」——すなわち kanade アクターが close 系列を経て終了に到達し
/// （`join_bounded` が成功する）ことのみを検証する。
///
/// # bounded
/// connect 手順の pump は `HANDSHAKE_TIMEOUT`・LOAD ack は `LOAD_ACK_TIMEOUT`・request 往復は
/// `Shiori3Client` 内部の env timeout（`AREKA_SHIORI_REQUEST_TIMEOUT_MS`・既定 60s）＋
/// `SMTO_ABORTIFHUNG` で必ず有限復帰する。kanade の join は `join_bounded`／`DEFAULT_TIMEOUT` で
/// 期限付き（ハングしない）。
#[test]
fn real_helper_boot_pump_close_completes() {
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 helper 追験を skip（任意 gate・Req 7.4）。\
             実 pasta 越しの運行完走を追験するには env に pasta DLL のフルパスを設定してください。"
        );
        return;
    };

    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail）"
    );

    // helper 不在は connect 失敗（ShioriDown）へ倒さず、追験の前提として明示 fail させる。
    let helper_exe = resolve_helper_exe().expect("i686 helper exe の解決に失敗");

    let load_dir = dll_path
        .parent()
        .expect("HOST32_PASTA_DLL に親 dir がない")
        .to_path_buf();
    let shiori_name = dll_path
        .file_name()
        .expect("HOST32_PASTA_DLL にファイル名がない")
        .to_string_lossy()
        .into_owned();

    // connect はアクタースレッド上で一度だけ実行される。on_down は接続確立失敗時の ShioriDown に加え、
    // 接続成功後も受信ループの生存期間中保持され死活監視の届け先を兼ねる。ここでは接続失敗・死活検出の
    // 観測チャンネルとして張る。
    let (down_tx, down_rx) = std::sync::mpsc::channel::<KanadeMsg>();
    let (shiori_tx, shiori_handle, _shiori_probe) = spawn_shiori_actor(
        move || connect_real_helper(helper_exe, load_dir, shiori_name),
        down_tx,
    );

    // close talk が発生した場合に確実に終了へ倒すため、quit ポリシーは Fixed(true)。
    // shell_name（OnBoot Ref0）は運行完走のみを問う本追験では厳密でなくてよく、既存 E2E に倣い "master"。
    let config = KanadeConfig::new("master", "1.0.0");
    let (talk_tx, talk_rx) = std::sync::mpsc::channel();
    // boot prefetch（username 照会）は実 shiori 経路で駆動されるが、本追験は照会結果を消費しないので
    // no-op sink を注入する。
    let (kanade_inbox_tx, kanade_handle) =
        spawn_kanade(config, shiori_tx, talk_tx, Box::new(|_, _| {}));

    let sakura = spawn_mock_sakura(talk_rx, kanade_inbox_tx.clone(), QuitPolicy::Fixed(true));

    kanade_inbox_tx.send(KanadeMsg::Boot).expect("send Boot");
    for i in 1..=3u64 {
        kanade_inbox_tx
            .send(KanadeMsg::Tick {
                now: MonotonicMs(i * 1_000),
            })
            .expect("send Tick");
    }
    kanade_inbox_tx
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("send CloseRequest");

    // close talk が出れば mock sakura が quit:true の TalkDone を返して終了系列を駆動し、出ない
    // （OnClose 204）場合も終了系列へ倒れる。いずれでも kanade は終了する。
    join_bounded("real-helper kanade join", DEFAULT_TIMEOUT, kanade_handle)
        .expect("実 helper 越しに boot→pump→close の運行が完走し kanade が終了する");

    drop(kanade_inbox_tx);
    let _ = sakura.join_bounded_then_commands("real-helper mock-sakura join", DEFAULT_TIMEOUT);

    // real shiori アクターは kanade 停止（StopSelf→ShioriMsg::Close 送出）で受信ループを抜け、
    // ShioriConnection（親窓＋helper）を RAII teardown する。
    join_bounded("real-helper shiori join", DEFAULT_TIMEOUT, shiori_handle)
        .expect("real shiori アクターが Close 後に接続資材を teardown して終了する");

    // 正常終了なら on_down は drop 済み（Disconnected）。ShioriDown が届いていれば接続確立失敗か
    // 実行中の死活検出なので、原因を明示するため最後に確認する。
    if let Ok(KanadeMsg::ShioriDown { reason, .. }) = down_rx.try_recv() {
        panic!("実 helper への接続確立に失敗した（ShioriDown）: {reason}");
    }
}
