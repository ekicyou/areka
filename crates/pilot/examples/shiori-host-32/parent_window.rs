//! host-32 先進坑: 親（x64）の message-only 窓と 1 往復の送り（親だけが使う・helper からは取り込まない）。
//!
//! `wintf-winmsg-executor` の message-only 窓の手続きで、helper から来る WM_COPYDATA を捌く:
//!   - `Hello`: 中身＝helper の窓の HWND の下 32 ビット（`to_le_bytes`）。記録してすぐ戻る。
//!   - `Response`: 中身を受け皿セル（`ResponseSlot`）へ入れてすぐ戻る。ここから跨プロセスの
//!     `SendMessage` を出さない（REQUEST 待ち ⊃ RESPONSE 受け、の入れ子だけ＝循環待ちが無い）。
//!
//! 1 往復の送り（`ParentMessageWindow::request`）は、受け皿を空にして REQUEST を送り、送りが
//! 戻った後に受け皿を取る。送りで待つ間に helper の RESPONSE が窓の手続きへ入れ子で届く。

use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LRESULT};
use windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA;
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};

use crate::ipc::{self, IpcError, MsgTag};

/// 窓の手続きと `request` が共有する、同時に 1 つだけの応答の置き場（UI スレッド固定ゆえ `RefCell`）。
#[derive(Default)]
struct ResponseSlot {
    inner: RefCell<Option<Vec<u8>>>,
}

impl ResponseSlot {
    /// 送る直前に空にする（前の往復の残りを消す）。
    fn clear(&self) {
        *self.inner.borrow_mut() = None;
    }

    /// 窓の手続きが RESPONSE の中身を入れる。
    fn store(&self, bytes: Vec<u8>) {
        *self.inner.borrow_mut() = Some(bytes);
    }

    /// 送りが戻った後に取り出す（1 回で消える）。
    fn take(&self) -> Option<Vec<u8>> {
        self.inner.borrow_mut().take()
    }
}

/// 窓の手続きと外側で共有する親の窓の状態（`Window<S>` が窓と同居させ `Pin<&S>` で渡す）。
#[derive(Default)]
struct ParentShared {
    /// HELLO で受けた helper の窓の HWND（下 32 ビット）。
    helper_hwnd: Cell<Option<u32>>,
    response_slot: ResponseSlot,
}

/// 親の窓の手続き。HELLO と RESPONSE を記録してすぐ戻る。
fn handle_message(s: &ParentShared, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // DefWindowProc へ。
    }
    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS の約束）。
    let Some((tag_raw, payload)) = (unsafe { ipc::copydata_payload(msg.lparam) }) else {
        return Some(LRESULT(0));
    };

    match MsgTag::try_from_u32(tag_raw) {
        Ok(MsgTag::Hello) => match <[u8; 4]>::try_from(payload.as_slice()) {
            Ok(bytes) => s.helper_hwnd.set(Some(u32::from_le_bytes(bytes))),
            Err(_) => eprintln!(
                "[parent] HELLO の中身が 4 バイトでない（{} バイト・無視）",
                payload.len()
            ),
        },
        Ok(MsgTag::Response) => s.response_slot.store(payload),
        // REQUEST/UNLOAD は親 → helper の向きで、helper からは来ない。
        Ok(other) => eprintln!("[parent] 想定外のタグ（無視）: {other:?}"),
        Err(raw) => eprintln!("[parent] 未知のタグ（無視）: {raw:#x}"),
    }
    Some(LRESULT(0))
}

/// 親の message-only 窓。Drop で窓を壊す。
pub struct ParentMessageWindow {
    window: Window<ParentShared>,
}

impl ParentMessageWindow {
    /// message-only 窓を作る（helper を起こす前に立て、HELLO の受け手にする）。
    pub fn create() -> Result<Self, String> {
        let window = Window::new_checked(
            WindowType::MessageOnly,
            ParentShared::default(),
            move |state: Pin<&ParentShared>, msg: WindowMessage| -> Option<LRESULT> {
                handle_message(state.get_ref(), &msg)
            },
        )
        .map_err(|e| format!("親 message-only 窓の生成に失敗: {e:?}"))?;
        Ok(Self { window })
    }

    /// 窓の HWND の下 32 ビット（helper へ引数で渡す値）。
    pub fn hwnd_u32(&self) -> u32 {
        ipc::hwnd_to_u32(self.window.hwnd())
    }

    /// HELLO を受けるか `timeout` までメッセージのループを回す。戻り値は helper の HWND（下 32 ビット）・
    /// 期限までに来なければ `None`。
    pub fn pump_until_hello_or(&self, timeout: Duration) -> Option<u32> {
        let s = self.window.state();
        ipc::pump_until(self.window.hwnd(), Instant::now() + timeout, || {
            s.helper_hwnd.get().is_some()
        });
        s.helper_hwnd.get()
    }

    /// 受け皿を空にして REQUEST を送り、送りが通ったら受け皿を取る。
    /// `Ok(None)`＝送れたが RESPONSE が来なかった（時間切れとは呼ばない）。
    pub fn request(
        &self,
        helper: HWND,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Option<Vec<u8>>, IpcError> {
        let slot = &self.window.state().get_ref().response_slot;
        slot.clear();
        ipc::send_copydata(
            helper,
            self.window.hwnd(),
            MsgTag::Request,
            payload,
            timeout,
        )?;
        Ok(slot.take())
    }
}

#[cfg(test)]
#[path = "parent_window_tests.rs"]
mod tests;
