//! 応答方向の送出が「応答なし」判定で打ち切られないことの統合の檻（要件 16.1・16.3⑵・design.md D16）。
//!
//! # 何を固定するか
//!
//! ホスト役の窓を持つスレッドは、要求の往復以外ではメッセージを取り出さない（本番では shiori
//! アクターが `recv()` で待つ）。helper はその窓へ応答を**再入で**送り返す。応答方向に
//! `SMTO_ABORTIFHUNG` が付いていると、OS がホストのスレッドを「応答なし」と見なした後は、この
//! 応答が**待たずに即座に失敗**する（戻り 0・last error 0）。ホストは受け皿が空のまま復帰して
//! `Timeout` と読む——これが 2026-09-07 の実機一周走行で解放（Unload）が落ちた機序である。
//!
//! この檻は同一プロセス内でその状況をそのまま組み、**往復②の応答が届くこと**を主張する。
//!
//! # なぜ遅い檻なのか（5.5 秒／10 秒では檻にならない・2026-09-07 較正）
//!
//! 打ち切りの発現には条件が 2 つある。**⑴ プロセスが起きてから概ね 20〜30 秒を過ぎていること**
//! （起動直後には猶予期間があり、その間は何秒待たせても打ち切られない）と、**⑵ 宛先の窓を持つ
//! スレッドが 14 秒以上メッセージを取り出していないこと**。`IsHungAppWindow` が真になる 5 秒とは
//! 別の判定である。最初の実装は「起動直後に 5.5〜20 秒待つ」形だったため、条件⑴ を満たさず
//! **直す前でも緑**になり、檻として無効だった。
//!
//! そこで待ちを 2 段にする——20 秒待つ → 往復①（このときプロセスは約 20 秒＝猶予期間の内ゆえ
//! 届く）→ さらに 20 秒待つ → 往復②（プロセスは約 40 秒＝猶予期間の外・スレッドは 20 秒
//! 取り出していない）。**直す前は往復②の応答が届かず赤**（`second_delivered=false`）、応答方向
//! からハング打ち切りの旗を外すと両方届く。
//!
//! 所要は約 42 秒（20＋20＋往復 2 回）。上限は 90 秒で、待ちも送出上限も有限ゆえ構造的に
//! 打ち切られる（無限待機なし）。速くする道は無い——条件⑴⑵ が実時間そのものだからである。
//!
//! # 組み立て
//!
//! - **ホスト役**＝テストのスレッド。message-only 窓を持つが `GetMessage` を 1 度も呼ばない
//!   （本番の shiori アクターと同じ「待機中に pump しない」姿）。往復は本番の
//!   [`shiori_host32_ipc::send_request`]（`slot.clear()` → 要求を同期送出 → 復帰後 `slot.take()`）。
//! - **helper 役**＝別スレッド。message-only 窓を持ち `GetMessage` を回し続ける。要求を受けた
//!   WndProc の中から本番の [`shiori_host32_ipc::send_copydata_response`] で応答を再入送出する
//!   （helper 本体の 3 か所と同じ関数・同じ旗）。
//!
//! 実 DLL は要らない（proxy を駆動しない）ため x64 でも走る。`main_loopback_tests.rs` の i686 専用
//! 檻とは別物で、あちらは無改変のままである。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use super::{HWND, LPARAM, LRESULT, WPARAM, read_copydata};
use shiori_host32_ipc::{
    IpcError, MsgTag, ResponseSlot, copydata_payload, encode_hwnd_le, hwnd_from_u32,
    send_copydata_response, send_request,
};
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MSG, PostMessageW, PostQuitMessage, RegisterClassExW, TranslateMessage, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_COPYDATA, WNDCLASSEXW,
};
use windows::core::w;

use std::time::Duration;

/// 各段の待ち（メッセージを取り出さない時間）。
///
/// 14 秒（スレッドが「応答なし」と見なされる境）を確実に超える最小の切りの良い値。
const IDLE: Duration = Duration::from_secs(20);
/// 往復 1 回の上限（要求方向・応答方向とも）。helper 本体の `REPLY_TIMEOUT` と同値。
const ROUND_TRIP_TIMEOUT: Duration = Duration::from_secs(5);
/// 檻全体の上限。実測の所要は約 42 秒で、構造上の最悪でも 20＋20＋5＋5＋起動待ち 10＝60 秒。
const CAGE_BOUND: Duration = Duration::from_secs(90);
/// helper 役スレッドが窓を作って HWND を返すまでの待ちの上限。
const HELPER_READY_TIMEOUT: Duration = Duration::from_secs(10);

/// helper 役の窓へ「ループを畳め」と伝える posted メッセージ。
const WM_CAGE_QUIT: u32 = WM_APP + 1;

/// helper 役が受け取った要求の数（スレッドをまたいで読むため atomic）。
static HELPER_REQUESTS_SEEN: AtomicU32 = AtomicU32::new(0);
/// helper 役の応答送出が失敗した回数（打ち切りの直接の観測点）。
static HELPER_RESPONSE_SEND_FAILURES: AtomicU32 = AtomicU32::new(0);
/// ホスト役の窓 HWND のワイヤ表現（helper 役が応答の宛先として読む・本番の `parent_hwnd` と同型）。
static HOST_HWND_WIRE: AtomicU32 = AtomicU32::new(0);

thread_local! {
    /// ホスト役スレッドの応答受け皿。WndProc が `store`、[`send_request`] が `take` する。
    static HOST_SLOT: ResponseSlot = ResponseSlot::new();
}

/// ホスト役の WndProc: 応答を受け皿へ積むだけ（本番の親窓と同型）。
unsafe extern "system" fn host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_COPYDATA {
        // SAFETY: WM_COPYDATA 契約により lparam は有効な COPYDATASTRUCT を指す。
        if let Some((dw, len, payload)) = unsafe { read_copydata(lparam) }
            && let Ok((MsgTag::Response, p)) = copydata_payload(dw, len, &payload)
        {
            let bytes = p.to_vec();
            HOST_SLOT.with(|slot| slot.store(bytes));
        }
        return LRESULT(0);
    }
    // SAFETY: Win32 境界。既定のウィンドウ手続きへ委譲する。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// helper 役の WndProc: 要求を受けたその場から応答を再入送出する（本番の helper と同型）。
unsafe extern "system" fn helper_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CAGE_QUIT {
        // SAFETY: Win32 境界。自スレッドのループへ終了を積む。
        unsafe { PostQuitMessage(0) };
        return LRESULT(0);
    }
    if msg == WM_COPYDATA {
        // SAFETY: WM_COPYDATA 契約により lparam は有効な COPYDATASTRUCT を指す。
        if let Some((dw, len, payload)) = unsafe { read_copydata(lparam) }
            && let Ok((MsgTag::Request, p)) = copydata_payload(dw, len, &payload)
        {
            HELPER_REQUESTS_SEEN.fetch_add(1, Ordering::SeqCst);
            let target = hwnd_from_u32(HOST_HWND_WIRE.load(Ordering::SeqCst));
            let body = p.to_vec();
            // 本番の応答経路と同じ関数・同じ旗（要件 16.1）。
            if send_copydata_response(target, hwnd, MsgTag::Response, &body, ROUND_TRIP_TIMEOUT)
                .is_err()
            {
                HELPER_RESPONSE_SEND_FAILURES.fetch_add(1, Ordering::SeqCst);
            }
        }
        return LRESULT(0);
    }
    // SAFETY: Win32 境界。既定のウィンドウ手続きへ委譲する。
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 固有名のクラスを 1 度だけ登録して message-only 窓を作る。
fn create_message_window(
    class: windows::core::PCWSTR,
    proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) -> HWND {
    // SAFETY: Win32 境界。自プロセス固有名のクラス登録と message-only 窓の生成。
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(None).expect("GetModuleHandleW").into();
        let wc = WNDCLASSEXW {
            cbSize: core::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(proc),
            hInstance: hinstance,
            lpszClassName: class,
            ..Default::default()
        };
        // 同名クラスの二重登録は 0 を返すが、既に登録済みなら窓は作れる。生成の成否で判定する。
        let _atom = RegisterClassExW(&wc);
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("host32-hung-cage"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance),
            None,
        )
        .expect("message-only 窓の生成に失敗した")
    }
}

/// ホスト役の往復 1 回（本番と同じ [`send_request`]）。
fn round_trip(host: HWND, helper: HWND, body: &[u8]) -> Result<Vec<u8>, IpcError> {
    HOST_SLOT.with(|slot| {
        send_request(
            helper,
            host,
            MsgTag::Request,
            body,
            ROUND_TRIP_TIMEOUT,
            slot,
        )
    })
}

/// 要件 16.1・16.3⑵ / design.md D16 の統合の檻。
///
/// ホスト役のスレッドが 20 秒メッセージを取り出さずに待つ → 往復①（猶予期間の内・届く）→
/// さらに 20 秒待つ → 往復②（猶予期間の外）。**往復②の応答が届く**ことを主張する。
///
/// 直す前（応答方向にも `SMTO_ABORTIFHUNG` が付いている状態）は往復②が
/// `second_delivered=false`（`slot.take()` が空＝`Timeout`）で赤になる。
#[test]
fn the_response_reaches_a_host_thread_that_has_not_pumped_for_twenty_seconds() {
    let started = Instant::now();
    HELPER_REQUESTS_SEEN.store(0, Ordering::SeqCst);
    HELPER_RESPONSE_SEND_FAILURES.store(0, Ordering::SeqCst);

    // --- helper 役スレッド（常時メッセージループ）を起こし、その窓 HWND をワイヤ値で受け取る ---
    let (tx, rx) = mpsc::channel::<u32>();
    let helper_thread = std::thread::spawn(move || {
        let hwnd = create_message_window(w!("areka-host32-hung-cage-helper"), helper_wnd_proc);
        tx.send(u32::from_le_bytes(encode_hwnd_le(hwnd)))
            .expect("helper 役の HWND 送出に失敗した");
        // SAFETY: Win32 境界。WM_QUIT が来るまでメッセージを取り出して配る。
        unsafe {
            let mut msg = MSG::default();
            loop {
                let got = GetMessageW(&mut msg, None, 0, 0);
                if got.0 <= 0 {
                    break; // 0 = WM_QUIT / -1 = エラー。どちらもループを畳む。
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = DestroyWindow(hwnd);
        }
    });
    let helper_hwnd = hwnd_from_u32(
        rx.recv_timeout(HELPER_READY_TIMEOUT)
            .expect("helper 役の窓が上限内に立ち上がらなかった"),
    );

    // --- ホスト役＝このスレッド。窓を作るが GetMessage は 1 度も呼ばない（本番のアクターと同型）---
    let host_hwnd = create_message_window(w!("areka-host32-hung-cage-host"), host_wnd_proc);
    HOST_HWND_WIRE.store(
        u32::from_le_bytes(encode_hwnd_le(host_hwnd)),
        Ordering::SeqCst,
    );

    // ① 20 秒 pump せず待つ（プロセスは約 20 秒＝猶予期間の内）→ 往復。
    std::thread::sleep(IDLE);
    let uptime_at_first = started.elapsed();
    let first = round_trip(host_hwnd, helper_hwnd, b"first");

    // ② さらに 20 秒 pump せず待つ（プロセスは約 40 秒＝猶予期間の外）→ 往復。
    std::thread::sleep(IDLE);
    let uptime_at_second = started.elapsed();
    let second = round_trip(host_hwnd, helper_hwnd, b"second");

    // --- 後始末（主張の前に必ず畳む。panic しても helper スレッドが残らないようにする）---
    // SAFETY: Win32 境界。自プロセスの窓へ posted メッセージを送り、自スレッドの窓を破棄する。
    unsafe {
        let _ = PostMessageW(Some(helper_hwnd), WM_CAGE_QUIT, WPARAM(0), LPARAM(0));
        let _ = DestroyWindow(host_hwnd);
    }
    let _ = helper_thread.join();

    let requests_seen = HELPER_REQUESTS_SEEN.load(Ordering::SeqCst);
    let send_failures = HELPER_RESPONSE_SEND_FAILURES.load(Ordering::SeqCst);
    let diag = format!(
        "first_delivered={} second_delivered={} \
         requests_seen={requests_seen} response_send_failures={send_failures} \
         idle={IDLE:?} uptime_at_first={uptime_at_first:?} uptime_at_second={uptime_at_second:?} \
         first={first:?} second={second:?}",
        first.is_ok(),
        second.is_ok(),
    );

    assert_eq!(
        requests_seen, 2,
        "helper 役が要求を 2 回受け取る（要求方向は両方とも届く）: {diag}"
    );
    assert!(
        first.is_ok(),
        "往復①の応答が届く（猶予期間の内・要件 16.1）: {diag}"
    );
    assert!(
        second.is_ok(),
        "往復②の応答が届く＝待機中のホストへの応答を「応答なし」判定で打ち切らない（要件 16.1・16.3⑵）: {diag}"
    );
    assert_eq!(
        send_failures, 0,
        "応答の送出が 1 度も失敗しない（打ち切りが起きていない）: {diag}"
    );
    assert_eq!(
        second.as_deref().ok(),
        Some(&b"second"[..]),
        "往復②で受け取るのは往復②の応答である（受け皿の取り違えでない）: {diag}"
    );

    let elapsed = started.elapsed();
    assert!(
        elapsed < CAGE_BOUND,
        "檻は上限 {CAGE_BOUND:?} の内で終わる（実測 {elapsed:?}）: {diag}"
    );
}
