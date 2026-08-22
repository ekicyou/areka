//! env-gate 実 helper 追験・単一 `#[test]`（Req 7.4・DD-8）。
//!
//! 実 32bit helper＋実 pasta.dll 越しに、real shiori アクター（`src/shiori/real.rs`）を通して
//! **起動→毎秒 Tick 数回→close の運行完走**を追験する。応答内容は実会話エンジン依存ゆえ、
//! スクリプトの中身ではなく「Value か 204 のいずれかで運行が完走する」ことを検証する。
//!
//! # env-gate（既存 host32 E2E 慣行の踏襲）
//! `HOST32_PASTA_DLL`（pasta DLL のフルパス）が未設定なら silent skip（`return`・CI 必須ゲート
//! にしない）。この env 名・skip 語法は既存の
//! `crates/shiori-host32-host/tests/shiori_request_e2e.rs`
//! （`request_e2e_real_pasta_optional`）／`shiori_load_e2e.rs`（`load_e2e_real_pasta_optional`）
//! の慣行をそのまま写している。env 設定済みだが DLL 不在なら明示 fail（silent skip を認めない）。
//!
//! # 接続手順は connect クロージャが所有する（DD-8）
//! spawn→HELLO pump→LOAD（`send_request(MsgTag::Load, ..)`）の接続確立手順は kanade の責務外で、
//! テスト側の connect クロージャが自前結線する。これも上記既存 E2E の手順
//! （`ParentMessageWindow::create` → `spawn` → `pump_until_hello_or` → `send_request(Load)`）を
//! 忠実に写している。connect が成功すれば `Box<dyn ShioriBackend>`（実体は
//! `ShioriConnection { window, helper: HelperLifecycle::new(handle) }`）を返し、失敗は
//! `Err(String)`（→ `ShioriDown`）で返す。
//!
//! # 親窓 1 枚制約
//! 親 message-only 窓は同一プロセスで 2 組独立生成すると 2 組目が失敗する既知制約ゆえ、本追験は
//! 単一の `#[test]` に集約する（既存 host32 E2E と同じ前提）。

use std::path::PathBuf;
use std::time::Duration;

use areka_kanade::{
    CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriBackend, ShioriConnection,
    spawn_kanade, spawn_shiori_actor,
};
use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{HelperLifecycle, ParentMessageWindow, spawn};
use shiori_host32_ipc::MsgTag;

use super::common::{DEFAULT_TIMEOUT, QuitPolicy, join_bounded, spawn_mock_sakura};

/// ハンドシェイク（HELLO 受領）の上限時間（既存 E2E の `HANDSHAKE_TIMEOUT` と同値）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// helper exe（事前ビルドした i686 バイナリ）のパスを解決する（既存 E2E `resolve_helper_exe`
/// と同型）。env `HOST32_HELPER_EXE` 最優先 → ワークスペース
/// `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`。いずれも不在なら
/// `Err(String)`（connect 失敗として ShioriDown へ写る・helper 成果物不備を明示）。
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

    // CARGO_MANIFEST_DIR = crates/areka-kanade → ワークスペースルート。
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
/// 既存 host32 E2E（`shiori_request_e2e::request_e2e_real_pasta_optional`）の connect 手順を
/// 忠実に写す:
/// 1. `ParentMessageWindow::create()` で親 message-only 窓を生成。
/// 2. `spawn(helper_exe, load_dir, shiori_name, parent_hwnd)` で i686 helper を起動。
/// 3. `pump_until_hello_or(HANDSHAKE_TIMEOUT)` で HELLO 受領（ハンドシェイク完了）を観測。
/// 4. `send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)` で LOAD を先行発行し ack `[1]` を確認。
///
/// いずれかの段で失敗すれば `Err(String)`（connect 失敗＝ShioriDown へ写る）。成功時は
/// `Box<dyn ShioriBackend>`（実体は `ShioriConnection`）を返す（`helper`＝`HelperLifecycle` の
/// drop で子プロセス資材が RAII teardown される）。
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
    // 1. 親 message-only 窓（!Send・アクタースレッド上で生成される）。
    let window =
        ParentMessageWindow::create().map_err(|e| format!("親 message-only 窓生成に失敗: {e}"))?;
    let parent_hwnd = window.hwnd_u32();

    // 2. i686 helper を spawn（cwd=load_dir）。
    let helper = spawn(&helper_exe, &load_dir, &shiori_name, parent_hwnd)
        .map_err(|e| format!("i686 helper の spawn に失敗: {e}"))?;

    // 3. HELLO 受領＝ハンドシェイク完了を観測（上限時間内に来なければ失敗）。
    let helper_hwnd = window.pump_until_hello_or(HANDSHAKE_TIMEOUT);
    if helper_hwnd.is_none() {
        return Err(
            "上限時間内に helper から HELLO を受領できなかった（ハンドシェイク未完）".to_string(),
        );
    }

    // 4. LOAD 先行（helper 内に proxy を確立＝REQUEST の構造的前提）→ ack[1] を確認。
    //    LOAD は kanade の責務外ゆえ、接続手順（テスト側）で明示発行する（DD-8・既存 E2E 慣行）。
    let load_ack = window
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .map_err(|e| format!("LOAD の send_request が失敗（ack 未達）: {e}"))?;
    if load_ack != vec![1u8] {
        return Err(format!(
            "実 pasta の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: {load_ack:?}"
        ));
    }

    // 生 HelperHandle を HelperLifecycle へ包む（正規 clean shutdown／死活監視の器・要件 3.2/6.2）。
    Ok(Box::new(ShioriConnection {
        window,
        helper: HelperLifecycle::new(helper),
    }))
}

/// env-gate 実 helper 追験（Req 7.4・DD-8）。
///
/// `HOST32_PASTA_DLL` 未設定なら silent skip（既存 host32 E2E の任意 gate 慣行と同一・
/// `shiori_request_e2e::request_e2e_real_pasta_optional` を写す）。設定時は実 32bit helper＋
/// 実 pasta 越しに real shiori アクター経由で **起動→毎秒 Tick 数回→close の運行完走**を追験する。
///
/// # 検証対象（Req 7.4）
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
    // --- env `HOST32_PASTA_DLL` 未設定＝任意 gate ゆえ silent skip（CI 必須ゲートにしない）---
    //     既存 host32 E2E（request_e2e_real_pasta_optional / load_e2e_real_pasta_optional）と同じ語法。
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 helper 追験を skip（任意 gate・Req 7.4）。\
             実 pasta 越しの運行完走を追験するには env に pasta DLL のフルパスを設定してください。"
        );
        return;
    };

    // --- env 設定済みだが DLL 不在なら明示 fail（silent skip を認めない）---
    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail）"
    );

    // helper 不在も connect 失敗（ShioriDown）へ倒すのではなく、追験の前提として明示 fail させる。
    let helper_exe = resolve_helper_exe().expect("i686 helper exe の解決に失敗");

    // load_dir=DLL の親 dir・shiori_name=ファイル名（既存 E2E と同じ流儀）。
    let load_dir = dll_path
        .parent()
        .expect("HOST32_PASTA_DLL に親 dir がない")
        .to_path_buf();
    let shiori_name = dll_path
        .file_name()
        .expect("HOST32_PASTA_DLL にファイル名がない")
        .to_string_lossy()
        .into_owned();

    // --- real shiori アクターを起動（connect はアクタースレッド上で一度だけ実行される）---
    //     connect クロージャが spawn→HELLO→LOAD を自前結線し Box<dyn ShioriBackend>
    //     （実体 ShioriConnection）を返す（DD-8）。on_down は接続確立失敗時の ShioriDown に加え、
    //     接続成功後も受信ループの生存期間中保持され死活監視の届け先を兼ねる（Req 3.4）。
    //     ここでは接続失敗・死活検出のいずれも可視化する観測チャンネルとして張り、shiori アクター
    //     終了（Close 経路）まで生存する。
    let (down_tx, down_rx) = std::sync::mpsc::channel::<KanadeMsg>();
    let (shiori_tx, shiori_handle) = spawn_shiori_actor(
        move || connect_real_helper(helper_exe, load_dir, shiori_name),
        down_tx,
    );

    // --- kanade を起動（real shiori 送信端＋mock sakura sink を結線）---
    //     kanade→sakura の StartTalk を mock sink がドレインし、即時 TalkDone を返す（common 再利用）。
    //     close talk が発生した場合に確実に終了へ倒すため、quit ポリシーは Fixed(true)。
    //     shell_name（OnBoot Ref0）は運行完走のみを問う本追験では厳密でなくてよく、既存 E2E
    //     （request_e2e_real_pasta_optional）に倣い "master" を用いる（妥当な Reference0）。
    let config = KanadeConfig::new("master", "1.0.0");
    let (talk_tx, talk_rx) = std::sync::mpsc::channel();
    // boot prefetch（username 照会・R4.1）は実 shiori 経路で駆動されるが、本追験は運行完走のみを
    // 問い照会結果を消費しないため no-op sink を注入する（Implementation Notes）。
    let (kanade_inbox_tx, kanade_handle) =
        spawn_kanade(config, shiori_tx, talk_tx, Box::new(|_, _| {}));

    let sakura = spawn_mock_sakura(talk_rx, kanade_inbox_tx.clone(), QuitPolicy::Fixed(true));

    // --- 運行を注入で駆動する（起動→毎秒 Tick 数回→close）---
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
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    // --- 運行完走の観測（Value か 204 のいずれでも close 系列で終了へ到達する）---
    //     close talk が出た場合は mock sakura が quit:true の TalkDone を返し終了系列を駆動する。
    //     close talk が出ない（OnClose 204）場合も終了系列へ倒れる。いずれでも kanade は終了する。
    join_bounded("real-helper kanade join", DEFAULT_TIMEOUT, kanade_handle)
        .expect("実 helper 越しに boot→pump→close の運行が完走し kanade が終了する");

    // --- teardown: kanade 停止後に inbox 送信端を drop → mock sakura sink も自然終了 ---
    drop(kanade_inbox_tx);
    sakura.join_bounded("real-helper mock-sakura join", DEFAULT_TIMEOUT);

    // real shiori アクターは kanade 停止（StopSelf→ShioriMsg::Close 送出）で受信ループを抜け、
    // ShioriConnection（親窓＋helper）を RAII teardown する。期限付き join で完走を確認する。
    join_bounded("real-helper shiori join", DEFAULT_TIMEOUT, shiori_handle)
        .expect("real shiori アクターが Close 後に接続資材を teardown して終了する");

    // 接続失敗／死活検出の可視化: shiori アクターは上で join 済み（Close 経路で終了）ゆえ on_down
    // は自然に drop されている（Disconnected）。万一 ShioriDown が届いていた場合は接続確立失敗
    // または実行中の死活検出であり、上の kanade join が既に終了系列（Unloading{Fault}）を
    // 通っているはずだが、原因を明示するため down_rx を最後に確認する。
    if let Ok(KanadeMsg::ShioriDown { reason }) = down_rx.try_recv() {
        panic!("実 helper への接続確立に失敗した（ShioriDown）: {reason}");
    }
}
