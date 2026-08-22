//! host-32 先進坑: **ParentMessageWindow**（design.md §176–212・x64 親専用）。
//!
//! `wintf-winmsg-executor` 0.0.5 で **message-only 窓**を生成し（helper 側 helper_window.rs と
//! 同一 API・donor のネイティブターゲットは x64）、その WndProc で helper からの
//! inbound WM_COPYDATA を `MsgTag` で捌く（design.md §183/§196–198）:
//!   - `Hello`(1st): helper のメッセージ窓 HWND（u32 LE）を復号して記録＝**HWND ハンドシェイク**。
//!     非ブロッキング（design.md §208）。
//!   - `Response`(2nd): 生 payload を**受け皿セル**（`ipc::ResponseSlot`）へ格納して**即 return**。
//!     それ以上の跨プロセス `SendMessage` を発行しない（**デッドロック回避の核**・design.md §210）。
//!
//! 本ファイルは **ParentDriver 境界**の x64 側窓（main.rs からのみ `#[path]` 取り込み）。
//! helper（i686）からは参照しない（プロセス分離）。窓/ループ/受け皿セル共有パターンは
//! helper_window.rs（HelperMessageWindow）の鏡像であり、同一の `wintf-winmsg-executor`
//! 窓・`MessageLoop::run` フィルタ・PostMessage ハートビートを用いる。

#![allow(dead_code)] // 一部アクセサは main.rs（ParentDriver）からのみ使用。

use std::cell::Cell;
use std::pin::Pin;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_COPYDATA};
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

use crate::ipc::{self, MsgTag, ResponseSlot};

/// WndProc と外側（ParentDriver）で共有する親窓の状態。
///
/// `wintf-winmsg-executor` の `Window<S>` は `state: S` を窓と同居させ、WndProc へ
/// `Pin<&S>` で渡す（GWLP_USERDATA 手詰め不要・helper_window.rs と同様）。single-in-flight
/// ゆえ内部可変は `Cell`/`ResponseSlot`(RefCell) で足りる（UI スレッド固定・design.md §209）。
pub struct ParentShared {
    /// HELLO で受領した helper のメッセージ窓 HWND（u32 ワイヤ値）。ハンドシェイク完了後に確定。
    helper_hwnd: Cell<Option<u32>>,
    /// **受け皿セル**（design.md §209）: `send_request` と共有し、RESPONSE payload を格納する。
    response_slot: ResponseSlot,
    /// 観測カウンタ: 受領した HELLO 数。
    hellos: Cell<u64>,
    /// 観測カウンタ: 受領した RESPONSE 数。
    responses: Cell<u64>,
    /// 観測カウンタ: 未知/想定外タグ受領数（crash させず記録）。
    unexpected: Cell<u64>,
}

impl ParentShared {
    fn new() -> Self {
        Self {
            helper_hwnd: Cell::new(None),
            response_slot: ResponseSlot::new(),
            hellos: Cell::new(0),
            responses: Cell::new(0),
            unexpected: Cell::new(0),
        }
    }

    /// ハンドシェイク完了で確定した helper HWND（u32 ワイヤ値）。未受領なら `None`。
    pub fn helper_hwnd(&self) -> Option<u32> {
        self.helper_hwnd.get()
    }

    /// `send_request` に渡す受け皿セルへの参照（WndProc が store する先と同一）。
    pub fn response_slot(&self) -> &ResponseSlot {
        &self.response_slot
    }
}

/// 受領した `WM_COPYDATA` の生バイト payload を安全に取り出す（`cbData` で境界画定・要件 2.1）。
///
/// # Safety
/// `lparam` は WM_COPYDATA の LPARAM で `*const COPYDATASTRUCT` を指すこと（OS が配送時に保証）。
unsafe fn copydata_payload(lparam: LPARAM) -> Option<(u32, Vec<u8>)> {
    let cds = lparam.0 as *const COPYDATASTRUCT;
    if cds.is_null() {
        return None;
    }
    // SAFETY: WM_COPYDATA 契約により lparam は有効な COPYDATASTRUCT を指す。
    let cds = unsafe { &*cds };
    let tag_raw = (cds.dwData & 0xFFFF_FFFF) as u32; // dwData は低 32bit のみ有意（ipc.rs §1）。
    let len = cds.cbData as usize;
    let payload = if len == 0 || cds.lpData.is_null() {
        Vec::new()
    } else {
        // SAFETY: lpData は cbData バイト有効（OS が marshal 済みの受信側コピー）。
        unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, len).to_vec() }
    };
    Some((tag_raw, payload))
}

/// 親 WndProc: helper からの inbound WM_COPYDATA を `MsgTag` で捌く（design.md §183/§196）。
/// **非ブロッキング**（HELLO/RESPONSE いずれも記録して即 return・跨プロセス SendMessage なし・§210）。
fn handle_message(s: &ParentShared, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // 非対象は DefWindowProc へ（closure が None を返すと lib が委譲）。
    }
    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS 契約）。
    let (tag_raw, payload) = match unsafe { copydata_payload(msg.lparam) } {
        Some(v) => v,
        None => return Some(LRESULT(0)),
    };

    match MsgTag::try_from_u32(tag_raw) {
        Ok(MsgTag::Hello) => {
            // HWND ハンドシェイク: payload = helper 窓 HWND の u32 LE（design.md §208）。
            if payload.len() == 4 {
                let bytes = [payload[0], payload[1], payload[2], payload[3]];
                let helper_hwnd_u32 = ipc::decode_hwnd_le(bytes);
                s.helper_hwnd.set(Some(helper_hwnd_u32));
                s.hellos.set(s.hellos.get() + 1);
            } else {
                s.unexpected.set(s.unexpected.get() + 1);
                eprintln!(
                    "[parent] HELLO payload 長が 4 でない（{} バイト・無視）",
                    payload.len()
                );
            }
            Some(LRESULT(0))
        }
        Ok(MsgTag::Response) => {
            // ★ 再入受領（design.md §209/§210）: payload を受け皿セルへ格納して即 return。
            //   ここで跨プロセス SendMessage を発行しない（循環待ちなし＝デッドロック回避の核）。
            s.responses.set(s.responses.get() + 1);
            s.response_slot.store(payload);
            Some(LRESULT(0))
        }
        Ok(other) => {
            // LOAD/REQUEST/UNLOAD は親 → helper 方向であり helper → 親では来ない。記録のみ。
            s.unexpected.set(s.unexpected.get() + 1);
            eprintln!("[parent] 想定外タグ受領（無視）: {other:?}");
            Some(LRESULT(0))
        }
        Err(raw) => {
            s.unexpected.set(s.unexpected.get() + 1);
            eprintln!("[parent] 未知タグ受領（無視）: {raw:#x}");
            Some(LRESULT(0))
        }
    }
}

/// ParentMessageWindow のハンドル。窓（`Window<ParentShared>`）を保持し、Drop で窓破棄。
pub struct ParentMessageWindow {
    window: Window<ParentShared>,
}

impl ParentMessageWindow {
    /// message-only 親窓を生成する（helper 起動前に立て、HELLO 受け皿にする）。
    pub fn create() -> Result<Self, String> {
        let shared = ParentShared::new();
        let window = Window::new_checked(
            WindowType::MessageOnly,
            shared,
            move |state: Pin<&ParentShared>, msg: WindowMessage| -> Option<LRESULT> {
                handle_message(state.get_ref(), &msg)
            },
        )
        .map_err(|e| format!("親 message-only 窓の生成に失敗: {e:?}"))?;
        Ok(Self { window })
    }

    pub fn hwnd(&self) -> HWND {
        self.window.hwnd()
    }

    /// 親窓 HWND を u32 ワイヤ値で（ProcessHost::spawn / HELLO seed 用）。
    pub fn hwnd_u32(&self) -> u32 {
        u32::from_le_bytes(ipc::encode_hwnd_le(self.window.hwnd()))
    }

    pub fn shared(&self) -> Pin<&ParentShared> {
        self.window.state()
    }

    /// HELLO 受領（`helper_hwnd` 確定）まで、または `timeout` 経過までメッセージループを回す
    /// （HWND ハンドシェイク・design.md §184）。返り値 = 受領した helper HWND（u32）。
    /// 期限内に HELLO が来なければ `None`（観測可能な失敗・ハングしない）。
    ///
    /// helper_window.rs::run_until_unload_or と同じく、無入力でも抜けられるよう別スレッドから
    /// 自窓へ定期 WM_NULL を撃って GetMessage を起こす（heartbeat）。
    pub fn pump_until_hello_or(&self, timeout: Duration) -> Option<u32> {
        // 既に受領済みなら即返す（helper が先に HELLO を送っている場合）。
        if let Some(h) = self.window.state().helper_hwnd.get() {
            return Some(h);
        }
        let deadline = Instant::now() + timeout;
        let state_ptr = self.window.state();

        let hb_hwnd_u32 = ipc::encode_hwnd_le(self.window.hwnd());
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let heartbeat = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let hwnd = ipc::hwnd_from_u32(u32::from_le_bytes(hb_hwnd_u32));
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    // WM_NULL(0) は無害な起こし用。GetMessage をブロックさせ続けない。
                    unsafe {
                        let _ = PostMessageW(Some(hwnd), 0, WPARAM(0), LPARAM(0));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            })
        };

        MessageLoop::run(|msg_loop, _msg| {
            if state_ptr.helper_hwnd.get().is_some() || Instant::now() >= deadline {
                msg_loop.quit();
            }
            FilterResult::Forward
        });

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = heartbeat.join();

        self.window.state().helper_hwnd.get()
    }
}
