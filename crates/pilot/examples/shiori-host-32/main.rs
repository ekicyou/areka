//! host-32 先進坑: x64 親の入口。
//!
//! `cargo run -p pilot --example shiori-host-32`（x64）で起こす。i686 helper を起こし、
//! WM_COPYDATA で OnBoot を 1 往復させて Value を受け（go 基準(1)）、同じ helper を N 秒
//! 生かしてから UNLOAD で clean unload させる（go 基準(2)）。`--selftest-errors` では
//! 異常系 err(1)・err(2) を観測する。
//! 葉ノード隔離（命綱）: コードは本フォルダ配下のみ。production クレートへの
//! inbound 依存を作らない。

#[path = "ipc.rs"]
mod ipc;

#[path = "shiori3.rs"]
mod shiori3;

#[path = "process_host.rs"]
mod process_host;

#[path = "parent_window.rs"]
mod parent_window;

#[path = "sample_support.rs"]
mod sample_support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ipc::MsgTag;
use parent_window::ParentMessageWindow;
use process_host::{ExitKind, HelperHandle, ProcessHost};
use std::pin::Pin;

/// HWND ハンドシェイク待ちの上限（ハーネスを塞がないよう短く）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// OnBoot 1 往復（REQUEST→RESPONSE 再入受領）の上限。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// UNLOAD 送出のタイムアウト（clean unload 指示）。
const UNLOAD_TIMEOUT: Duration = Duration::from_secs(3);

/// go 基準(2) で helper のメッセージループが生き続けるのを親が見る長さ N。
/// helper 側の上限（helper.rs が `run_helper` へ渡す 30s）より十分短くする。
const SURVIVAL_WINDOW: Duration = Duration::from_secs(4);
/// 生存窓の間、親が helper 生死を `poll_exit_kind` で見る間隔（この刻みで N 秒を消化する）。
const SURVIVAL_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// helper の exe（環境変数 `HELPER_EXE`、無ければ `--` で始まらない最初の引数）。
fn helper_exe_from_process() -> Option<PathBuf> {
    process_host::resolve_helper_exe(
        std::env::var(process_host::HELPER_EXE_ENV).ok(),
        std::env::args().skip(1),
    )
}

fn main() {
    // 絶対パスが要る。相対だと pasta.dll の LOAD が 0x8007007E で落ちる。
    let ghostdir = sample_support::emo2_ghost_master();
    let ghostdir = ghostdir.as_path();

    // --- 異常系セルフテスト: IPC の時間切れと helper の強制終了の検出。どちらもハングしないことが本質 ---
    if std::env::args().any(|a| a == "--selftest-errors") {
        match drive_selftest_errors(ghostdir) {
            Ok(()) => {
                println!(
                    "=== --selftest-errors: 両異常系 PASS（IPC Timeout ＋ helper 異常終了検出・いずれもハングせず） ==="
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("=== --selftest-errors FAIL: {e} ===");
                std::process::exit(1);
            }
        }
    }

    let helper_exe = match helper_exe_from_process() {
        Some(p) => p,
        None => {
            eprintln!(
                "[go(1)] HELPER_EXE 未設定かつ `--` で始まらない引数なし。\
                 $env:HELPER_EXE に i686 helper exe を設定して再実行してください。"
            );
            std::process::exit(2);
        }
    };

    match drive(&helper_exe, ghostdir) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("=== FAIL: {e} ===");
            std::process::exit(1);
        }
    }
}

/// go 基準(1) の 1 往復の後、同じ helper で go 基準(2) の生存→clean unload を観測する。
fn drive(helper_exe: &Path, ghostdir: &Path) -> Result<(), String> {
    // 親の窓は go(1)/go(2) を通じて生かす（HELLO と RESPONSE の受け皿）。
    let parent = ParentMessageWindow::create()?;

    // --- setup + go 基準(1): 往復して Value を受領（UNLOAD はまだ送らない）---
    let mut session = drive_go_criterion_1(&parent, helper_exe, ghostdir)?;

    println!("=== go 基準(1) OBSERVABLE: emo2 OnBoot Value ===");
    println!("Value: {}", session.value);
    println!("=== go 基準(1) PASS: x64 親が i686 helper 越しに OnBoot Value を受領 ===");

    let request_roundtrip = session.request_roundtrip;
    let response_bytes = session.response_bytes;
    let value_bytes = session.value_bytes;

    // --- go 基準(2): N 秒メッセージループ生存 → UNLOAD → clean unload → 終了コード 0 観測 ---
    let go2 = drive_go_criterion_2(&mut session)?;

    // === 一次記録のまとめ。go 判定そのものは開発者が人の判断で決める ===
    println!("=== GO 検証記録 (実 pasta.dll / emo2) ===");
    println!(
        "go(1): Value={} bytes 受領 [PASS] / RESPONSE={} bytes / REQUEST→RESPONSE 同期往復={} ms (block-on-reply)",
        value_bytes,
        response_bytes,
        request_roundtrip.as_millis()
    );
    println!(
        "go(2): survived {:.2}s (poll={} 回 alive / probe: 生存後 REQUEST {}) → clean unload kind={:?} [PASS]",
        go2.survived.as_secs_f64(),
        go2.polls,
        if go2.post_survival_response_bytes.is_some() {
            "応答あり"
        } else {
            "応答なし"
        },
        go2.exit_kind,
    );
    println!(
        "load→spawn_actor: design.md §495 が load 時 actor スレッドを仮説化したが利用可能ソースでは未確認\
         （vendors/pasta に該当シンボル無し・pasta_shiori/src/shiori.rs は single-threaded と明記／\
         かつ実ロードは prebuilt emo2 pasta.dll でその内部は vendored source から検証不能）\
         ⇒ 挙動証拠のみに格下げ: helper が go(1) 要求＋go(2) 生存後要求（実 2 応答）を処理し clean unload\
         ＝リクエスト処理ループが生存窓を跨いで稼働（代理証拠・x64 親は helper 内スレッド数を直接観測できない）"
    );
    println!("（go 判定は README・人間判断・要件 6.5）===");

    Ok(())
}

/// go 基準(2) の観測記録（一次記録サマリ用・数値のみ）。
struct Go2Record {
    /// N 秒生存窓で実際に生存した実時間。
    survived: Duration,
    /// 生存窓中の poll_exit_kind 回数（すべて alive=None）。
    polls: u64,
    /// 生存後の追加 REQUEST に helper が返した応答バイト数（None=応答なし）。
    /// Some ＝ helper のリクエスト処理（メッセージ）ループが生存窓の後も動いていることの代理証拠（pasta の actor スレッドは未確認）。
    post_survival_response_bytes: Option<usize>,
    /// 終わり方の分類（Clean＝終了コード 0 を期待）。
    exit_kind: ExitKind,
}

/// go 基準(1) の 1 往復で立てた生きた helper セッション。UNLOAD 未送出のまま go(2) へ渡す。
struct HelperSession<'p> {
    /// 親 message-only 窓（RESPONSE 受け皿・go(2) 中も生存）。
    parent: &'p ParentMessageWindow,
    /// helper 子プロセスハンドル。`wait_kind` が所有権を消費するので `Option`。
    handle: Option<HelperHandle>,
    /// 親窓 HWND（UNLOAD の from）。
    parent_hwnd: windows::Win32::Foundation::HWND,
    /// helper メッセージ窓 HWND（UNLOAD / 追加 REQUEST の宛先）。
    helper_hwnd: windows::Win32::Foundation::HWND,
    /// go(1) で受領した OnBoot の Value（親が確認済み）。
    value: String,
    /// go(1) の OnBoot REQUEST→RESPONSE の同期往復に要した実時間。
    request_roundtrip: Duration,
    /// go(1) で受けた RESPONSE のバイト数（run ごとに変わりうる）。
    response_bytes: usize,
    /// go(1) で受けた Value のバイト数（run ごとに変わりうる）。
    value_bytes: usize,
}

/// go 基準(1) の全体駆動。成功で生きた helper セッションを返す。
///
/// UNLOAD はここでは送らず、go 基準(2) の生存窓の後に送る。
fn drive_go_criterion_1<'p>(
    parent: &'p ParentMessageWindow,
    helper_exe: &Path,
    ghostdir: &Path,
) -> Result<HelperSession<'p>, String> {
    let parent_hwnd_u32 = parent.hwnd_u32();
    let parent_hwnd = ipc::hwnd_from_u32(parent_hwnd_u32);
    println!("[go(1)] 親 message-only 窓 生成: hwnd(u32)={parent_hwnd_u32:#010x}");
    println!("[go(1)] helper_exe = {}", helper_exe.display());
    println!("[go(1)] ghostdir   = {}", ghostdir.display());

    let mut handle = ProcessHost::spawn(helper_exe, ghostdir, parent_hwnd_u32)
        .map_err(|e| format!("helper 起動失敗: {e}"))?;
    println!("[go(1)] helper 起動 OK（HELLO を待機）");

    if let Some(kind) = ProcessHost::poll_exit_kind(&mut handle) {
        return Err(format!("helper が HELLO 前に終了した: {kind:?}"));
    }

    let helper_hwnd_u32 = match parent.pump_until_hello_or(HANDSHAKE_TIMEOUT) {
        Some(h) => h,
        None => {
            let alive = ProcessHost::poll_exit_kind(&mut handle);
            return Err(format!(
                "HELLO ハンドシェイクが {HANDSHAKE_TIMEOUT:?} 内に成立しなかった（helper={alive:?}）"
            ));
        }
    };
    let helper_hwnd = ipc::hwnd_from_u32(helper_hwnd_u32);
    println!("[go(1)] HWND ハンドシェイク完了: helper hwnd(u32)={helper_hwnd_u32:#010x}");

    let onboot = shiori3::build_onboot();
    println!("[go(1)] OnBoot 組立 OK（{} バイト）", onboot.len());

    // 親は SendMessageTimeout でブロックし、その間に helper の RESPONSE を親の WndProc が
    // 再入で受け皿セルへ入れる（デッドロックしない）。この区間を測り、同じ同期呼び出しの中で
    // Value が返る＝親が pasta の応答を待った（block-on-reply）ことを記録する。
    let req_started = Instant::now();
    let response = match parent.request(helper_hwnd, &onboot, REQUEST_TIMEOUT) {
        Ok(Some(r)) => r,
        Err(ipc::IpcError::Timeout) => {
            return Err(format!(
                "OnBoot の送りが {REQUEST_TIMEOUT:?} で時間切れ: helper が時間内に REQUEST を捌かなかった\
                 （helper の窓が詰まっている）"
            ));
        }
        Err(ipc::IpcError::SendFailed(code)) => {
            return Err(format!(
                "OnBoot を送れなかった（Win32 エラー {code}）: HELLO の HWND が違うか helper の窓が無い"
            ));
        }
        Ok(None) => {
            return Err(
                "OnBoot は送れたが RESPONSE が受け皿に届かなかった: helper の標準エラーを見る\
                 （pasta の読み込みや request の失敗）。それでなければ再入受領が i686↔x64 で不成立\
                 （design.md §210 Revalidation Trigger＝named pipe 後退を人間判断）"
                    .to_string(),
            );
        }
    };
    let request_roundtrip = req_started.elapsed();
    println!("[go(1)] RESPONSE 再入受領 OK（{} バイト）", response.len());
    println!(
        "[go(1)] REQUEST→RESPONSE 同期往復 {} ms（block-on-reply: 親は pasta 応答まで同期ブロック・design.md §492）",
        request_roundtrip.as_millis()
    );

    let value = shiori3::parse_value(&response)
        .ok_or_else(|| "RESPONSE に Value: が無い（emo2 OnBoot 応答の parse 失敗）".to_string())?;
    if value.is_empty() {
        return Err("Value: が空（起動挨拶さくらスクリプトが空）".to_string());
    }

    let value_bytes = value.len();
    Ok(HelperSession {
        parent,
        handle: Some(handle),
        parent_hwnd,
        helper_hwnd,
        value,
        request_roundtrip,
        response_bytes: response.len(),
        value_bytes,
    })
}

/// go 基準(2) の駆動。
///
/// 前提: `session` は go(1) で立てた稼働中の helper（UNLOAD 未送出）。
///   1. helper を `SURVIVAL_WINDOW` 生かし、刻みごとに `ProcessHost::poll_exit_kind` で生存を確かめる。
///      途中で終われば失敗。
///   2. 生存窓の後に UNLOAD を送る。
///   3. `ProcessHost::wait_kind` で `ExitKind::Clean`（終了コード 0）を親が観測する。それ以外は失敗。
///
/// どの段も上限がある。`wait_kind` は helper が UNLOAD ですぐ終わるので即返り、最悪でも
/// helper 側の上限（helper.rs が `run_helper` へ渡す 30s）で終わる。
fn drive_go_criterion_2(session: &mut HelperSession) -> Result<Go2Record, String> {
    println!("=== go 基準(2): メッセージループ {SURVIVAL_WINDOW:?} 生存 → clean unload を観測 ===");

    // --- 1. 生存窓 ---
    let started = Instant::now();
    let mut polls = 0u64;
    let handle = session
        .handle
        .as_mut()
        .ok_or_else(|| "go 基準(2) 内部エラー: helper handle が既に消費済み".to_string())?;
    while started.elapsed() < SURVIVAL_WINDOW {
        if let Some(kind) = ProcessHost::poll_exit_kind(handle) {
            let elapsed = started.elapsed();
            return Err(format!(
                "go 基準(2) 不成立: helper が生存窓 {SURVIVAL_WINDOW:?} を待たず {elapsed:?} で終了した \
                 （メッセージループが破綻・{kind:?}・UNLOAD 未送出）"
            ));
        }
        polls += 1;
        std::thread::sleep(SURVIVAL_POLL_INTERVAL);
    }
    let survived = started.elapsed();
    println!(
        "[go(2)] helper survived {:.2}s（poll_exit_kind={} 回 すべて alive=None・メッセージループ生存・要件 5.2）",
        survived.as_secs_f64(),
        polls
    );

    // 応答が返れば、helper のリクエスト処理ループが生存窓を跨いで動いていることの代理証拠になる
    // （x64 親は helper 内のスレッド数を直接見られない）。失敗しても致命ではない。
    let post_survival_response_bytes = mid_window_probe(session).ok();

    // --- 2. 生存窓の後に UNLOAD 送出 ---
    match ipc::send_copydata(
        session.helper_hwnd,
        session.parent_hwnd,
        MsgTag::Unload,
        &[],
        UNLOAD_TIMEOUT,
    ) {
        Ok(()) => println!("[go(2)] UNLOAD 送出 OK（N 秒生存後・clean unload 指示）"),
        Err(e) => {
            eprintln!(
                "[go(2)] UNLOAD 送出失敗: {e:?}（helper は上限で抜けて終了コード 4 になり、go 基準(2) は不成立になる）"
            );
        }
    }

    // --- 3. clean unload を待って終了コード 0 を親が観測 ---
    let handle = session
        .handle
        .take()
        .ok_or_else(|| "go 基準(2) 内部エラー: helper handle が既に消費済み".to_string())?;
    match ProcessHost::wait_kind(handle) {
        Ok(kind) => {
            println!("[go(2)] helper exited kind={kind:?}（親が終わり方を観測）");
            if kind.is_clean() {
                println!(
                    "=== go 基準(2) PASS: helper survived {:.2}s → clean unload → exit code=0 kind=Clean を親が観測（要件 1.3/5.2/5.4・design.md §230） ===",
                    survived.as_secs_f64()
                );
                Ok(Go2Record {
                    survived,
                    polls,
                    post_survival_response_bytes,
                    exit_kind: kind,
                })
            } else {
                Err(format!(
                    "go 基準(2) 不成立: helper が clean(0) で終了しなかった（{kind:?}）"
                ))
            }
        }
        Err(e) => Err(format!("go 基準(2) 不成立: clean unload 待ちに失敗（{e}）")),
    }
}

// ============================================================================
// 異常系セルフテスト
//
// 2 つの異常系を x64 親側で観測する。どちらもハングしないこと:
//   err(1) IPC タイムアウト
//   err(2) helper 強制終了検出
// ============================================================================

/// err(1) の Timeout 観測に使う短いタイムアウト（wedge の 3s より十分短い）。
const ERR_IPC_TIMEOUT: Duration = Duration::from_millis(500);
/// err(1) の wedged 窓が REQUEST 受領時にブロックする時間（Timeout より十分長い）。
const ERR_WEDGE_SLEEP: Duration = Duration::from_secs(3);
/// err(1) の Timeout 判定の上限。SendMessageTimeout(500ms) が返るまでを寛容に見積もり、
/// wedge(3s) より十分短ければ合格。
const ERR_TIMEOUT_UPPER_BOUND: Duration = Duration::from_millis(1500);
/// err(1) の Timeout 判定の下限（`ERR_IPC_TIMEOUT` − 100 ms）。これより早い Timeout は上限を守った証拠にならない。
const ERR_TIMEOUT_LOWER_BOUND: Duration = Duration::from_millis(400);
/// err(2) で helper の HELLO を待つ上限（起動確認・bounded）。
const ERR_HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// 異常系 2 種を続けて観測する。両方 PASS で `Ok(())`。
fn drive_selftest_errors(ghostdir: &Path) -> Result<(), String> {
    drive_err_ipc_timeout()?;
    drive_err_helper_abnormal_exit(ghostdir)?;
    Ok(())
}

/// err(1) IPC タイムアウト。
///
/// 別スレッドの無応答（wedged）窓が REQUEST で `ERR_WEDGE_SLEEP` ブロックする（無応答 helper の模擬）。
/// そこへ `ERR_IPC_TIMEOUT` で `ipc::send_copydata` を撃ち、wedge を待たずに
/// `Err(IpcError::Timeout)` が返ることを観測する。
///
/// 同じスレッド宛の SendMessageTimeout は窓の手続きを直に呼び上限が効かないので、wedged 窓は別スレッドに置く。
fn drive_err_ipc_timeout() -> Result<(), String> {
    use std::sync::mpsc;

    println!("--- err(1) IPC タイムアウト観測（要件 2.3・design.md §340/§525）---");

    // 窓とループは wedge スレッドに閉じる（窓アフィニティ）。
    let (hwnd_tx, hwnd_rx) = mpsc::channel::<u32>();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_thread = stop.clone();

    let wedge = std::thread::spawn(move || {
        let window = match wintf_winmsg_executor::util::Window::new_checked(
            wintf_winmsg_executor::util::WindowType::MessageOnly,
            (),
            move |_state: Pin<&()>,
                  msg: wintf_winmsg_executor::util::WindowMessage|
                  -> Option<windows::Win32::Foundation::LRESULT> {
                if msg.msg == windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA {
                    std::thread::sleep(ERR_WEDGE_SLEEP);
                    return Some(windows::Win32::Foundation::LRESULT(0));
                }
                None
            },
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[err(1)] wedged 窓生成に失敗: {e:?}");
                return;
            }
        };
        let hwnd_u32 = ipc::hwnd_to_u32(window.hwnd());
        let _ = hwnd_tx.send(hwnd_u32);

        // main が stop を立ててから投函する WM_NULL で起き、stop を見て抜ける。
        wintf_winmsg_executor::MessageLoop::run(|msg_loop, _msg| {
            if stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
                msg_loop.quit();
            }
            wintf_winmsg_executor::FilterResult::Forward
        });
        drop(window);
    });

    let wedge_hwnd_u32 = hwnd_rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "wedged 窓の HWND を受領できなかった（窓生成失敗の可能性）".to_string())?;
    let wedge_hwnd = ipc::hwnd_from_u32(wedge_hwnd_u32);
    println!(
        "[err(1)] wedged 窓（別スレッド・REQUEST で {ERR_WEDGE_SLEEP:?} ブロック）: hwnd(u32)={wedge_hwnd_u32:#010x}"
    );

    // wedged 窓のループが回り始めるまで軽く待つ。
    std::thread::sleep(Duration::from_millis(100));

    // self_hwnd は WPARAM に載るだけの形式上の送信元。応答は来ないので wedge_hwnd を流用する。
    let self_hwnd = wedge_hwnd;

    let started = Instant::now();
    let result = ipc::send_copydata(
        wedge_hwnd,
        self_hwnd,
        MsgTag::Request,
        b"GET SHIORI/3.0\r\nID: OnBoot\r\n\r\n",
        ERR_IPC_TIMEOUT,
    );
    let elapsed = started.elapsed();

    // MessageLoop はメッセージで起きるので、stop を立てた後に wedged 窓へ 1 回 PostMessage で起こす。
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(wedge_hwnd),
            0, // WM_NULL
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
    let _ = wedge.join();

    match result {
        Err(ipc::IpcError::Timeout) => {
            if elapsed < ERR_TIMEOUT_LOWER_BOUND {
                return Err(format!(
                    "err(1) 不成立: Timeout は返ったが elapsed={elapsed:?} が下限 {ERR_TIMEOUT_LOWER_BOUND:?} 未満（上限まで待たずに返った）"
                ));
            }
            if elapsed > ERR_TIMEOUT_UPPER_BOUND {
                return Err(format!(
                    "err(1) 不成立: Timeout は返ったが elapsed={elapsed:?} が上限 {ERR_TIMEOUT_UPPER_BOUND:?} を超過（bounded でない）"
                ));
            }
            println!(
                "[err(1)] IPC timeout observed in {}ms (timeout={:?}, wedge={:?}, bounded, no hang) PASS",
                elapsed.as_millis(),
                ERR_IPC_TIMEOUT,
                ERR_WEDGE_SLEEP
            );
            Ok(())
        }
        Err(other) => Err(format!(
            "err(1) 不成立: 期待は IpcError::Timeout だが {other:?} が返った（elapsed={elapsed:?}）"
        )),
        Ok(()) => Err(format!(
            "err(1) 不成立: 無応答 wedge への送りが通った（elapsed={elapsed:?}・Timeout であるべき）"
        )),
    }
}

/// err(2) helper 強制終了検出。
///
/// 実 i686 helper を起こし、HELLO で稼働を確かめてから `ProcessHost::terminate` で強制終了する。
/// `wait_kind` の終わり方が `ExitKind::Clean` でない（clean unload の exit 0 と区別できる）ことを確かめる。
fn drive_err_helper_abnormal_exit(ghostdir: &Path) -> Result<(), String> {
    println!("--- err(2) helper 強制終了検出（要件 1.4/2.4・design.md §528）---");

    let helper_exe = match helper_exe_from_process() {
        Some(p) => p,
        None => {
            return Err(
                "HELPER_EXE 未設定かつ argv に helper exe なし。err(2) には実 i686 helper が必要（$env:HELPER_EXE を設定）".to_string(),
            );
        }
    };
    println!("[err(2)] helper_exe = {}", helper_exe.display());

    let parent = ParentMessageWindow::create()?;
    let parent_hwnd_u32 = parent.hwnd_u32();
    let mut handle = ProcessHost::spawn(&helper_exe, ghostdir, parent_hwnd_u32)
        .map_err(|e| format!("helper 起動失敗: {e}"))?;
    println!("[err(2)] helper 起動 OK（HELLO を待機・稼働確認）");

    // kill は HELLO が無くてもできるが、稼働状態からの強制終了を観測するため HELLO を待つ。
    match parent.pump_until_hello_or(ERR_HELLO_TIMEOUT) {
        Some(h) => {
            println!("[err(2)] helper 稼働確認 OK（HELLO 受領・helper hwnd(u32)={h:#010x}）")
        }
        None => {
            eprintln!(
                "[err(2)] HELLO を {ERR_HELLO_TIMEOUT:?} 内に受領できず（稼働確認省略・kill は続行）"
            );
        }
    }

    // 既に落ちていたら kill の意味がない。
    if let Some(kind) = ProcessHost::poll_exit_kind(&mut handle) {
        return Err(format!(
            "err(2) 準備失敗: kill 前に helper が既に終了していた（{kind:?}）"
        ));
    }

    // 予期せぬ終了を模擬する。
    let killed_at = Instant::now();
    ProcessHost::terminate(&mut handle).map_err(|e| format!("helper の強制終了に失敗: {e}"))?;
    println!("[err(2)] helper を強制終了（Child::kill）");

    // kill 済みなので wait はすぐ返る。
    let kind = ProcessHost::wait_kind(handle).map_err(|e| format!("終了待ちに失敗: {e}"))?;
    let detect_elapsed = killed_at.elapsed();

    if kind.is_clean() {
        return Err(format!(
            "err(2) 不成立: 強制終了したのに ExitKind::Clean と観測された（異常検出できず）"
        ));
    }
    let code_str = match kind {
        ExitKind::Abnormal(c) => format!("{c}"),
        ExitKind::Terminated => "none(Terminated)".to_string(),
        ExitKind::Clean => unreachable!(),
    };
    println!(
        "[err(2)] force-killed helper detected: kind={kind:?} code={code_str} (detected in {}ms, bounded, no hang) PASS",
        detect_elapsed.as_millis()
    );
    drop(parent);
    Ok(())
}

/// 生存窓の後、helper のメッセージループがまだ REQUEST を捌けるかを見る任意のプローブ。
/// 失敗しても go(2) は失敗にしない（生存＋clean unload が本質）。
fn mid_window_probe(session: &HelperSession) -> Result<usize, ()> {
    let onboot = shiori3::build_onboot();
    match session
        .parent
        .request(session.helper_hwnd, &onboot, REQUEST_TIMEOUT)
    {
        Ok(Some(resp)) => {
            println!(
                "[go(2)] 生存後の追加 REQUEST に helper が応答（{} バイト）＝ループは生存後も稼働中",
                resp.len()
            );
            Ok(resp.len())
        }
        Ok(None) => {
            eprintln!("[go(2)] 追加 REQUEST は送れたが応答なし（補足観測のみ・致命ではない）");
            Err(())
        }
        Err(e) => {
            eprintln!("[go(2)] 追加 REQUEST は失敗（補足観測のみ・致命ではない）: {e:?}");
            Err(())
        }
    }
}
