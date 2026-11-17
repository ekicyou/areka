//! host-32 先進坑: helper（i686）の message-only 窓とメッセージのループ（helper だけが使う）。
//!
//! `wintf-winmsg-executor` で message-only 窓を立て、起動時に親へ HELLO（自分の窓の HWND の
//! 下 32 ビット・LE）を WM_COPYDATA で送る。窓の手続きは REQUEST を受けると pasta を駆動し、
//! RESPONSE を起動時に受けた親の HWND へ 1 通だけ返してすぐ戻る（それ以上の跨プロセスの
//! `SendMessage` を出さないので、REQUEST 待ち ⊃ RESPONSE 受け、の入れ子だけ＝循環待ちが無い）。
//! UNLOAD を受けるか上限に達するとループを抜け、抜け方を終了コードで返す（`exit_code`）。
//!
//! pasta.dll は窓を立てる前に読み込む（REQUEST を窓の手続きの中ですぐ処理するため）。

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::pin::Pin;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LRESULT};
use windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA;
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};

use crate::ipc::{self, MsgTag};
use crate::shiori_proxy::{ShioriByteProxy, ShioriEntries};

/// HELLO と RESPONSE の送りの上限。
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

/// 窓の手続きと外側（ループ・selftest）が共有する helper の窓の状態（UI スレッド固定ゆえ `Cell`／`RefCell`）。
pub struct HelperShared {
    /// 親のメッセージ窓の HWND（下 32 ビット）。HELLO と RESPONSE の送り先。
    parent_hwnd: u32,
    /// 読み込んだ pasta のエントリ。捨てるときに `FreeLibrary` する。
    entries: RefCell<Option<ShioriEntries>>,
    /// UNLOAD を受けたら真。ループがこれを見て抜ける。
    want_quit: Cell<bool>,
}

/// 窓の手続き: WM_COPYDATA を判別子で分ける。応答は 1 通だけ返してすぐ戻る。
///
/// RESPONSE は起動時に受けた `parent_hwnd` へ返す（`wParam` の送り主は見ない）。
fn handle_message(s: &HelperShared, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // 対象外は DefWindowProc へ。
    }

    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS の約束）。
    let (tag_raw, payload) = match unsafe { ipc::copydata_payload(msg.lparam) } {
        Some(v) => v,
        None => return Some(LRESULT(0)),
    };

    match MsgTag::try_from_u32(tag_raw) {
        Ok(MsgTag::Request) => {
            let response = {
                let guard = s.entries.borrow();
                match guard.as_ref() {
                    Some(e) => ShioriByteProxy::shiori_request(e, &payload).ok(),
                    None => None,
                }
            };
            if let Some(bytes) = response {
                let target = ipc::hwnd_from_u32(s.parent_hwnd);
                if let Err(e) =
                    ipc::send_copydata(target, msg.hwnd, MsgTag::Response, &bytes, SEND_TIMEOUT)
                {
                    eprintln!("[helper] RESPONSE 送出失敗: {e:?}");
                }
            } else {
                eprintln!("[helper] REQUEST 処理失敗（応答なし）");
            }
            Some(LRESULT(0))
        }
        Ok(MsgTag::Unload) => {
            // ループを抜ける印。pasta の後片付けは外側（`clean_unload`）でする。
            s.want_quit.set(true);
            Some(LRESULT(0))
        }
        Ok(other) => {
            // HELLO／RESPONSE は親から helper へは来ない。記録だけ。
            eprintln!("[helper] 想定外タグ受領（無視）: {other:?}");
            Some(LRESULT(0))
        }
        Err(raw) => {
            eprintln!("[helper] 未知タグ受領（無視）: {raw:#x}");
            Some(LRESULT(0))
        }
    }
}

/// helper の message-only 窓。捨てると窓も壊れる。
pub struct HelperMessageWindow {
    window: Window<HelperShared>,
}

impl HelperMessageWindow {
    /// message-only 窓を立て、親へ HELLO（自分の HWND の下 32 ビット・LE）を送る。
    ///
    /// `entries` が `None` でも窓は立つ（REQUEST に応答なしとして記録する）。
    pub fn create(parent_hwnd: u32, entries: Option<ShioriEntries>) -> Result<Self, String> {
        let shared = HelperShared {
            parent_hwnd,
            entries: RefCell::new(entries),
            want_quit: Cell::new(false),
        };
        let window = Window::new_checked(
            WindowType::MessageOnly,
            shared,
            move |state: Pin<&HelperShared>, msg: WindowMessage| -> Option<LRESULT> {
                handle_message(state.get_ref(), &msg)
            },
        )
        .map_err(|e| format!("message-only 窓の生成に失敗: {e:?}"))?;

        let self_hwnd: HWND = window.hwnd();
        let hello_payload = ipc::hwnd_to_u32(self_hwnd).to_le_bytes();
        let target = ipc::hwnd_from_u32(parent_hwnd);
        if let Err(e) = ipc::send_copydata(
            target,
            self_hwnd,
            MsgTag::Hello,
            &hello_payload,
            SEND_TIMEOUT,
        ) {
            // 親が helper の HWND を得られないので起動の失敗。
            return Err(format!("HELLO 送出に失敗: {e:?}"));
        }
        Ok(Self { window })
    }

    pub fn hwnd(&self) -> HWND {
        self.window.hwnd()
    }

    pub fn shared(&self) -> Pin<&HelperShared> {
        self.window.state()
    }

    /// UNLOAD を受けるか `max_run` が経つまでメッセージのループを回す（`ipc::pump_until`）。
    /// UNLOAD を受けて抜けたら真、`max_run` で抜けたら偽。戻っても窓は生きている。
    pub fn run_until_unload_or(&self, max_run: Duration) -> bool {
        let s = self.window.state();
        ipc::pump_until(self.window.hwnd(), Instant::now() + max_run, || {
            s.want_quit.get()
        })
    }

    /// pasta を `shiori_unload` し、エントリを捨てる（捨てるときに `FreeLibrary`）。窓は `self` を捨てると壊れる。
    pub fn clean_unload(&self) {
        let s = self.window.state();
        if let Some(e) = s.entries.borrow().as_ref() {
            let _ = ShioriByteProxy::shiori_unload(e);
        }
        *s.entries.borrow_mut() = None;
    }
}

/// helper の抜け方から終了コードを決める。
///
/// 0＝UNLOAD で抜け pasta を読めていた・4＝UNLOAD が来ずに上限で抜けた（`loaded` に関係なく）・
/// 5＝UNLOAD で抜けたが pasta を読めていなかった。
pub fn exit_code(unloaded: bool, loaded: bool) -> i32 {
    match (unloaded, loaded) {
        (false, _) => 4,
        (true, false) => 5,
        (true, true) => 0,
    }
}

/// 本番の helper（親が起こす）。pasta を読み、窓を立てて HELLO を送り、UNLOAD か `max_run` まで
/// ループを回して後片付けし、終了コード（`exit_code`。窓／HELLO の失敗は 2）を返す。
///
/// `parent_hwnd` は親から受けた HWND の下 32 ビット。`ghostdir` は pasta.dll の在処。
pub fn run_helper(parent_hwnd: u32, ghostdir: &Path, max_run: Duration) -> i32 {
    let dll = ghostdir.join("pasta.dll");
    let (entries, loaded) = match ShioriByteProxy::load_dll(&dll) {
        Ok(e) => match ShioriByteProxy::shiori_load(&e, ghostdir) {
            Ok(()) => (Some(e), true),
            Err(err) => {
                eprintln!("[helper] shiori_load 失敗: {err:?}");
                // 窓は立てて HELLO は送る（親が見張れる形を変えない）。REQUEST は失敗を記録する。
                (Some(e), false)
            }
        },
        Err(err) => {
            eprintln!("[helper] pasta.dll ロード失敗: {err:?}");
            (None, false)
        }
    };

    let win = match HelperMessageWindow::create(parent_hwnd, entries) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[helper] 窓生成/HELLO 失敗: {e}");
            return 2;
        }
    };
    let unloaded = win.run_until_unload_or(max_run);
    win.clean_unload();
    let code = exit_code(unloaded, loaded);
    match code {
        4 => eprintln!("[helper] UNLOAD が来ないまま上限 {max_run:?} で抜けた（終了コード 4）"),
        5 => eprintln!("[helper] pasta を読めないまま UNLOAD で抜けた（終了コード 5）"),
        _ => {}
    }
    code
}

/// helper の `main` から呼ぶ同じプロセスの中のセルフテスト（親を立てずに helper の窓を確かめる）:
///   1. 代わりの親（message-only 窓）を立て、HELLO を実の WM_COPYDATA で受けて記録する。
///   2. HelperMessageWindow を立てる（HELLO は `create` の中で送る）。
///   3. 代わりの親が受けた HWND が helper の HWND と一致することを確かめる。
///   4. ループを `secs` 秒回し、上限で抜けた（偽）ことを確かめる。
///   5. UNLOAD の印を立ててループに入り、UNLOAD で抜けた（真）ことを確かめて後片付けする。
///
/// 確かめを標準出力に出し、全部通れば `Ok(())`。
pub fn selftest_loop(secs: u64) -> Result<(), String> {
    // --- 1. 代わりの親（HELLO の受け皿）---
    let received_hello: std::rc::Rc<Cell<Option<u32>>> = std::rc::Rc::new(Cell::new(None));
    let parent = {
        let received = received_hello.clone();
        Window::new_checked(
            WindowType::MessageOnly,
            (),
            move |_state: Pin<&()>, msg: WindowMessage| -> Option<LRESULT> {
                if msg.msg == WM_COPYDATA {
                    // SAFETY: WM_COPYDATA の約束。
                    if let Some((tag_raw, payload)) = unsafe { ipc::copydata_payload(msg.lparam) } {
                        if MsgTag::try_from_u32(tag_raw) == Ok(MsgTag::Hello) && payload.len() == 4
                        {
                            let bytes = [payload[0], payload[1], payload[2], payload[3]];
                            received.set(Some(u32::from_le_bytes(bytes)));
                        }
                    }
                    return Some(LRESULT(0));
                }
                None
            },
        )
        .map_err(|e| format!("stand-in parent 窓生成に失敗: {e:?}"))?
    };
    let parent_hwnd_u32 = ipc::hwnd_to_u32(parent.hwnd());
    println!("[selftest] stand-in parent 窓 生成: hwnd(u32)={parent_hwnd_u32:#010x}");

    // --- 2. HelperMessageWindow（pasta は使わない）---
    let helper = HelperMessageWindow::create(parent_hwnd_u32, None)
        .map_err(|e| format!("HelperMessageWindow 生成に失敗: {e}"))?;
    let helper_hwnd_u32 = ipc::hwnd_to_u32(helper.hwnd());
    println!("[selftest] HelperMessageWindow 生成: hwnd(u32)={helper_hwnd_u32:#010x}");

    // --- 3. HELLO の HWND の一致 ---
    match received_hello.get() {
        Some(got) if got == helper_hwnd_u32 => {
            println!(
                "[selftest] HELLO 受領 OK: stand-in parent が helper HWND を復号一致 ({got:#010x})"
            );
        }
        Some(got) => {
            return Err(format!(
                "HELLO で復号した helper HWND 不一致: got={got:#010x} expected={helper_hwnd_u32:#010x}"
            ));
        }
        None => {
            return Err("stand-in parent が HELLO を受領しなかった".to_string());
        }
    }

    // --- 4. 上限で抜ける ---
    println!("[selftest] メッセージループを {secs} 秒運転...");
    let started = Instant::now();
    if helper.run_until_unload_or(Duration::from_secs(secs)) {
        return Err("UNLOAD を送っていないのにループが UNLOAD で抜けた".to_string());
    }
    let elapsed = started.elapsed();
    println!("[selftest] ループ生存 OK: {elapsed:?} 回り続けて上限で抜けた");

    // --- 5. UNLOAD で抜ける ---
    helper.shared().want_quit.set(true);
    if !helper.run_until_unload_or(Duration::from_secs(2)) {
        return Err("UNLOAD の印を立てたのにループが上限で抜けた".to_string());
    }
    helper.clean_unload();
    println!("[selftest] UNLOAD → ループ停止 → clean unload OK");

    drop(helper);
    drop(parent);
    println!("[selftest] 窓破棄・清掃終了 OK（exit 0）");
    Ok(())
}

#[cfg(test)]
#[path = "helper_window_tests.rs"]
mod tests;
