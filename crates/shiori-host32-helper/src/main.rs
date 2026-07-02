//! host-32 **i686 helper 実行バイナリ**（design.md §437-484・HelperMessageWindow）。
//!
//! `wintf-winmsg-executor` 0.0.5 の **message-only 窓** ＋ `MessageLoop::run` を i686 で回す
//! transport の helper 側。責務は 3 点に限定される（design.md §446-452・要件 3.1 / 4.2 / 6.1 / 7.1）:
//!
//! 1. **起動時 HELLO**（要件 3.1）: 窓生成後、親へ自 HWND を u32 LE で 1st WM_COPYDATA 送出。
//! 2. **REQUEST → echo → 即 return**（要件 4.2 / 6.1）: WndProc で inbound WM_COPYDATA を
//!    framing 検証し、`Request` なら [`respond`]（echo）した bytes を `Response` として親へ
//!    **1 通だけ** 返送して即 return する（それ以上の跨プロセス `SendMessage` を発行しない）。
//!    不正フレーム／未知タグは crash させず観測カウンタに記録するのみ（要件 2.5・上位へ渡さない）。
//! 3. **`main`**（要件 7.1）: 親 HWND を arg/env の u32 ワイヤ値で取得 → 窓生成 → HELLO 送出 →
//!    `MessageLoop::run`。lifecycle は echo 実証に必要な最小に留める（常駐 lifecycle は
//!    Out of Boundary・下流 `host32-lifecycle`）。
//!
//! ## 下流の差し替え点（design.md §451 / §465-466）
//! [`respond`] は **plain fn の echo**（`fn respond(req: &[u8]) -> Vec<u8> { req.to_vec() }`）で、
//! pasta 非依存。これが Requirement 6 の「意味を持たない生バイト往復」を成立させる。下流
//! `shiori-host32-shiori-load` はこの 1 関数を pasta 駆動へ置換する。**trait 抽象は設けない**
//! （YAGNI・凍結する seam は WM_COPYDATA の REQUEST/RESPONSE ワイヤ形式であって respond 実装
//! ではない・design.md §482）。
//!
//! ## デッドロック回避（design.md §200 / §473）
//! WndProc は REQUEST に対し RESPONSE を 1 通返すだけで即 return する。往復は single-in-flight
//! で厳密にネストし、循環待ちが構造的に発生しない（要件 4.4）。
//!
//! ## 依存方向（design.md §119-127）
//! 本クレートは `shiori-host32-ipc`（proto）を一方向依存し、`shiori-host32-host` へは依存しない
//! （host↔helper のコード依存は無く、プロセス境界で WM_COPYDATA のみ）。

use std::cell::Cell;
use std::pin::Pin;
use std::time::Duration;

use shiori_host32_ipc::{
    FramingError, MsgTag, copydata_payload, encode_hwnd_le, hwnd_from_u32, send_copydata,
};
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{FilterResult, MessageLoop};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA;

/// 応答送出 `SendMessageTimeoutW` の上限時間。
///
/// 無限待機の構造的排除（要件 5.3）。echo 応答は同期送出で即完了するため短めで足りるが、
/// ハング peer でも上限時間で復帰することを保証する。
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// **下流の差し替え点**（design.md §465-466・要件 6.1）: request bytes をそのまま返す echo。
///
/// pasta 非依存の単純 echo で、これが Requirement 6 の「意味を持たない生バイト往復」を
/// 成立させる。下流 `shiori-host32-shiori-load` がこの中身を pasta 駆動へ置換する。
/// **trait 抽象は設けない**（plain fn・YAGNI）。
fn respond(req: &[u8]) -> Vec<u8> {
    req.to_vec()
}

/// inbound WM_COPYDATA の framing 検証結果に応じた WndProc の取るべき動作（窓なしで単体検証可）。
///
/// 窓・FFI から切り離した純ロジックとして [`classify_inbound`] が算出し、WndProc はこの結果を
/// 見て副作用（送出・カウンタ更新）を行う。これにより REQUEST 分岐・不正フレーム分類を
/// 実窓なしで決定的に単体検証できる。
#[derive(Debug, Clone, PartialEq, Eq)]
enum InboundAction {
    /// `Request` を受領。同梱の echo bytes を `Response` として親へ返送すべき。
    Reply(Vec<u8>),
    /// framing 上は正当だが本 helper が応答しないタグ（`Hello`/`Load`/`Response`/`Unload`）。
    /// crash させず無視する（記録のみ）。
    IgnoreKnown(MsgTag),
    /// 未知タグ・長さ不整合など不正フレーム。crash させず記録のみ・上位へ渡さない（要件 2.5）。
    IgnoreBad(FramingError),
}

/// inbound WM_COPYDATA の生値（tag 生値・宣言長・実データ）を [`InboundAction`] へ写像する純関数。
///
/// framing 検証は `shiori-host32-ipc::copydata_payload` に委譲（重複実装しない）。`Request` のみ
/// [`respond`] で echo bytes を作り `Reply` とし、その他の既知タグは `IgnoreKnown`、不正フレームは
/// `IgnoreBad` とする。副作用（送出・カウンタ）は持たず、WndProc 側が結果を見て実行する。
fn classify_inbound(dw_data: usize, declared_len: usize, data: &[u8]) -> InboundAction {
    match copydata_payload(dw_data, declared_len, data) {
        Ok((MsgTag::Request, payload)) => InboundAction::Reply(respond(payload)),
        Ok((tag, _)) => InboundAction::IgnoreKnown(tag),
        Err(err) => InboundAction::IgnoreBad(err),
    }
}

/// WndProc と外側で共有する helper 窓の状態（design.md §475-478）。
///
/// `wintf-winmsg-executor` の `Window<S>` は `state: S` を窓と同居させ、WndProc へ `Pin<&S>` で
/// 渡す（`Rc` 不要・GWLP_USERDATA 手詰め不要）。single-in-flight・単一 UI スレッドゆえ内部可変は
/// [`Cell`] で足りる。lifecycle 状態機械は echo 実証に必要な最小（観測カウンタのみ）に留める。
struct HelperShared {
    /// 親のメッセージ窓 HWND（u32 ワイヤ値）。HELLO 送出先＆RESPONSE 返送先。
    parent_hwnd: u32,
    /// 観測カウンタ: 送出した HELLO 数。
    hellos_sent: Cell<u64>,
    /// 観測カウンタ: 受領した REQUEST 数。
    requests_handled: Cell<u64>,
    /// 観測カウンタ: 返送した RESPONSE 数。
    responses_sent: Cell<u64>,
    /// 観測カウンタ: 不正フレーム／未知タグ受領数（crash させず記録・要件 2.5）。
    bad_frames: Cell<u64>,
}

impl HelperShared {
    fn new(parent_hwnd: u32) -> Self {
        Self {
            parent_hwnd,
            hellos_sent: Cell::new(0),
            requests_handled: Cell::new(0),
            responses_sent: Cell::new(0),
            bad_frames: Cell::new(0),
        }
    }
}

/// 受領した WM_COPYDATA の `(dwData 生値, cbData 宣言長, payload コピー)` を取り出す。
///
/// # Safety
/// `lparam` は WM_COPYDATA の LPARAM であり `*const COPYDATASTRUCT` を指すこと（OS が
/// WM_COPYDATA 配送時に保証）。`lpData` は `cbData` バイト分有効であること。
unsafe fn read_copydata(lparam: LPARAM) -> Option<(usize, usize, Vec<u8>)> {
    let cds = lparam.0 as *const COPYDATASTRUCT;
    if cds.is_null() {
        return None;
    }
    // SAFETY: WM_COPYDATA 契約により lparam は有効な COPYDATASTRUCT を指す。
    let cds = unsafe { &*cds };
    let dw_data = cds.dwData;
    let len = cds.cbData as usize;
    let payload = if len == 0 || cds.lpData.is_null() {
        Vec::new()
    } else {
        // SAFETY: lpData は cbData バイト有効（OS が marshal 済みの受信側コピー）。
        unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, len).to_vec() }
    };
    Some((dw_data, len, payload))
}

/// WndProc 本体: WM_COPYDATA を [`classify_inbound`] で分類し、`Reply` なら echo RESPONSE を
/// 親へ 1 通返送して即 return する（design.md §450・要件 4.2 / 6.1）。
///
/// `self_hwnd` は自窓 HWND（RESPONSE の送信元として載せる）。RESPONSE の宛先は起動時に確定した
/// `parent_hwnd`。それ以上の跨プロセス `SendMessage` は発行しない（要件 4.2・循環待ちなし）。
fn handle_message(s: &HelperShared, self_hwnd: HWND, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // 非対象は DefWindowProc へ委譲（closure が None を返すと lib が委譲）。
    }

    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS 契約）。
    let Some((dw_data, declared_len, payload)) = (unsafe { read_copydata(msg.lparam) }) else {
        return Some(LRESULT(0));
    };

    match classify_inbound(dw_data, declared_len, &payload) {
        InboundAction::Reply(bytes) => {
            s.requests_handled.set(s.requests_handled.get() + 1);
            // RESPONSE を親へ 1 通だけ返す（それ以上の跨プロセス SendMessage 不可・§200）→ 即 return。
            let target = hwnd_from_u32(s.parent_hwnd);
            match send_copydata(target, self_hwnd, MsgTag::Response, &bytes, REPLY_TIMEOUT) {
                Ok(()) => s.responses_sent.set(s.responses_sent.get() + 1),
                Err(e) => eprintln!("[helper] RESPONSE 送出失敗（観測）: {e:?}"),
            }
        }
        InboundAction::IgnoreKnown(tag) => {
            // Hello/Load/Response/Unload は helper が能動応答しない。記録のみ（無応答）。
            eprintln!("[helper] 応答対象外タグ受領（無視）: {tag:?}");
        }
        InboundAction::IgnoreBad(err) => {
            // 未知タグ・長さ不整合は crash させず記録のみ・上位へ渡さない（要件 2.5）。
            s.bad_frames.set(s.bad_frames.get() + 1);
            eprintln!("[helper] 不正フレーム受領（無視）: {err}");
        }
    }
    Some(LRESULT(0))
}

/// HelperMessageWindow: message-only 窓を生成し、起動時に親へ HELLO を送出する（design.md §437-484）。
struct HelperMessageWindow {
    /// 所有する窓ハンドル。Drop で `DestroyWindow`（RAII）ゆえ本体は保持のためだけに存在する
    /// （非 test ビルドでは read されないが、生存＝窓生存の Drop guard）。観測は test 経由。
    #[cfg_attr(not(test), allow(dead_code))]
    window: Window<HelperShared>,
}

impl HelperMessageWindow {
    /// message-only 窓を生成し、起動時に親へ HELLO（自 HWND を u32 LE・要件 3.1）を送出する。
    ///
    /// `parent_hwnd` は親から受けた u32 ワイヤ値（HELLO 送出先＆RESPONSE 返送先）。
    fn create(parent_hwnd: u32) -> Result<Self, String> {
        let shared = HelperShared::new(parent_hwnd);
        // Fn（非 FnMut）ゆえ new を用いる。self_hwnd は WndProc の msg.hwnd から得る。
        let window = Window::new(
            WindowType::MessageOnly,
            shared,
            move |state: Pin<&HelperShared>, msg: WindowMessage| -> Option<LRESULT> {
                let self_hwnd = msg.hwnd;
                handle_message(state.get_ref(), self_hwnd, &msg)
            },
        )
        .map_err(|e| format!("message-only 窓の生成に失敗: {e:?}"))?;

        // 起動時 HELLO（1st WM_COPYDATA・自 HWND を u32 LE・要件 3.1）。
        let self_hwnd: HWND = window.hwnd();
        let hello_payload = encode_hwnd_le(self_hwnd);
        let target = hwnd_from_u32(parent_hwnd);
        send_copydata(target, self_hwnd, MsgTag::Hello, &hello_payload, REPLY_TIMEOUT)
            .map_err(|e| format!("HELLO 送出に失敗: {e:?}"))?;
        window.state().hellos_sent.set(window.state().hellos_sent.get() + 1);

        Ok(Self { window })
    }

    /// 自窓 HWND（loopback セルフテストからの観測用）。
    #[cfg(test)]
    fn hwnd(&self) -> HWND {
        self.window.hwnd()
    }

    /// 共有状態（観測カウンタ）への参照（loopback セルフテストからの観測用）。
    #[cfg(test)]
    fn shared(&self) -> Pin<&HelperShared> {
        self.window.state()
    }
}

/// 親 HWND の u32 ワイヤ値を arg（第 1 引数）または env（`HOST32_PARENT_HWND`）から取得する。
///
/// arg 優先。いずれも無い／解釈不能なら `None`（呼び出し側が起動失敗として扱う）。
fn parent_hwnd_from_env() -> Option<u32> {
    if let Some(v) = std::env::args().nth(1).and_then(|a| a.trim().parse::<u32>().ok()) {
        return Some(v);
    }
    std::env::var("HOST32_PARENT_HWND")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

fn main() {
    let Some(parent_hwnd) = parent_hwnd_from_env() else {
        eprintln!(
            "[helper] 親 HWND（u32 ワイヤ値）が未指定です。arg1 または env HOST32_PARENT_HWND で渡してください。"
        );
        std::process::exit(2);
    };

    let win = match HelperMessageWindow::create(parent_hwnd) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[helper] 窓生成/HELLO 失敗: {e}");
            std::process::exit(2);
        }
    };

    // REQUEST を受領して echo RESPONSE を返すため、メッセージループを回す（要件 4.2 / 6.1）。
    // 常駐 lifecycle（UNLOAD 停止等）は Out of Boundary（下流 host32-lifecycle）ゆえ、
    // 本ユニットは echo 実証に必要な最小ループとし、終了条件は下流が結線する。
    // `win` は本スコープで生存し続け、Drop（窓破棄）は main 終了時。
    let _keep_alive = &win;
    MessageLoop::run(|_msg_loop, _msg| FilterResult::Forward);
}

#[cfg(test)]
mod respond_tests {
    use super::*;

    // 要件 6.1: request bytes → 同一 response bytes（echo 等価性）。
    #[test]
    fn respond_echoes_nonempty_payload() {
        assert_eq!(respond(b"hello-echo"), b"hello-echo".to_vec());
    }

    #[test]
    fn respond_echoes_empty_payload() {
        assert_eq!(respond(b""), Vec::<u8>::new());
    }

    #[test]
    fn respond_echoes_binary_payload() {
        let bytes = [0u8, 1, 2, 255, 128, 0, 42];
        assert_eq!(respond(&bytes), bytes.to_vec());
    }
}

#[cfg(test)]
mod classify_tests {
    use super::*;

    // 要件 6.1 / 4.2: REQUEST は echo bytes を伴う Reply へ分類される。
    #[test]
    fn request_classifies_to_reply_with_echo() {
        let payload = b"round-trip";
        let raw = MsgTag::Request.as_u32() as usize;
        let action = classify_inbound(raw, payload.len(), payload);
        assert_eq!(action, InboundAction::Reply(payload.to_vec()));
    }

    #[test]
    fn request_with_empty_payload_replies_empty() {
        let raw = MsgTag::Request.as_u32() as usize;
        let action = classify_inbound(raw, 0, b"");
        assert_eq!(action, InboundAction::Reply(Vec::new()));
    }

    // 要件 4.2: 応答対象外の既知タグは IgnoreKnown（無応答・crash なし）。
    #[test]
    fn known_nonrequest_tags_are_ignored() {
        for tag in [MsgTag::Hello, MsgTag::Load, MsgTag::Response, MsgTag::Unload] {
            let raw = tag.as_u32() as usize;
            let action = classify_inbound(raw, 0, b"");
            assert_eq!(action, InboundAction::IgnoreKnown(tag));
        }
    }

    // 要件 2.5: 未知タグは crash させず IgnoreBad（記録のみ）。
    #[test]
    fn unknown_tag_is_ignored_as_bad() {
        let action = classify_inbound(0xFFusize, 0, b"");
        assert!(matches!(action, InboundAction::IgnoreBad(_)));
    }

    // 要件 2.5: cbData と実長の不整合は crash させず IgnoreBad。
    #[test]
    fn length_mismatch_is_ignored_as_bad() {
        let raw = MsgTag::Request.as_u32() as usize;
        let action = classify_inbound(raw, 10, b"abc");
        assert!(matches!(action, InboundAction::IgnoreBad(_)));
    }
}

#[cfg(test)]
mod loopback_tests {
    use super::*;
    use std::rc::Rc;
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

    /// 単一 loopback テスト（HELLO 送出・REQUEST echo・不正フレーム記録・bounded 生存を同一窓で網羅）。
    ///
    /// wintf の message-only 窓を i686 テストプロセスで 2 組独立生成すると 2 組目が
    /// WindowCreationError になる既知制約ゆえ、窓は 1 組（stand-in parent ＋ helper）に集約する。
    /// bounded: 有限メッセージを PostMessage で撒いた後に必ず抜ける（無限ループ禁止）。
    #[test]
    fn loopback_hello_request_echo_and_bounded_loop() {
        // --- stand-in parent（HELLO / RESPONSE 受け皿）。受領を Rc<Cell> に記録 ---
        let hello_helper_hwnd: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));
        let responses: Rc<Cell<u64>> = Rc::new(Cell::new(0));
        let last_response: Rc<std::cell::RefCell<Vec<u8>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));

        let parent = {
            let hello_helper_hwnd = hello_helper_hwnd.clone();
            let responses = responses.clone();
            let last_response = last_response.clone();
            Window::new(
                WindowType::MessageOnly,
                (),
                move |_state: Pin<&()>, msg: WindowMessage| -> Option<LRESULT> {
                    if msg.msg != WM_COPYDATA {
                        return None;
                    }
                    // SAFETY: WM_COPYDATA 契約。
                    if let Some((dw, len, payload)) = unsafe { read_copydata(msg.lparam) } {
                        match copydata_payload(dw, len, &payload) {
                            Ok((MsgTag::Hello, p)) if p.len() == 4 => {
                                let bytes = [p[0], p[1], p[2], p[3]];
                                hello_helper_hwnd
                                    .set(Some(shiori_host32_ipc::decode_hwnd_le(bytes)));
                            }
                            Ok((MsgTag::Response, p)) => {
                                responses.set(responses.get() + 1);
                                *last_response.borrow_mut() = p.to_vec();
                            }
                            _ => {}
                        }
                    }
                    Some(LRESULT(0))
                },
            )
            .expect("stand-in parent 窓生成に失敗")
        };
        let parent_hwnd_u32 =
            u32::from_le_bytes(encode_hwnd_le(parent.hwnd()));

        // --- helper 窓生成（HELLO は create 内で親へ送出）---
        let helper = HelperMessageWindow::create(parent_hwnd_u32)
            .expect("HelperMessageWindow 生成に失敗");
        let helper_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(helper.hwnd()));

        // HELLO 送出経路が動いた: 親が helper HWND を復号一致で受領（要件 3.1）。
        assert_eq!(
            hello_helper_hwnd.get(),
            Some(helper_hwnd_u32),
            "親が HELLO を復号一致で受領する（要件 3.1）"
        );
        assert_eq!(helper.shared().hellos_sent.get(), 1);

        // --- REQUEST → helper WndProc → echo RESPONSE を親が受領（要件 4.2 / 6.1）---
        // helper 自窓へ REQUEST を送り、WndProc が respond→RESPONSE を親へ返すのを観測する。
        let req = b"echo-me-42";
        send_copydata(
            helper.hwnd(),
            parent.hwnd(),
            MsgTag::Request,
            req,
            REPLY_TIMEOUT,
        )
        .expect("REQUEST 送出に失敗");

        assert_eq!(
            helper.shared().requests_handled.get(),
            1,
            "WndProc が REQUEST を処理する（要件 4.2）"
        );
        assert_eq!(
            helper.shared().responses_sent.get(),
            1,
            "WndProc が RESPONSE を 1 通返送する（要件 4.2）"
        );
        assert_eq!(responses.get(), 1, "親が echo RESPONSE を受領する（要件 6.1）");
        assert_eq!(
            &*last_response.borrow(),
            &req.to_vec(),
            "受領 response bytes が request bytes と一致（echo・要件 6.1）"
        );

        // --- 不正フレーム: 未知タグを helper へ送っても crash せず記録のみ（要件 2.5）---
        // 未知タグ生値 0xFF を dwData に載せて自窓へ送る（copydata_payload が UnknownTag で弾く）。
        {
            let payload: &[u8] = b"";
            let cds = COPYDATASTRUCT {
                dwData: 0xFFusize,
                cbData: 0,
                lpData: payload.as_ptr() as *mut core::ffi::c_void,
            };
            // SAFETY: helper.hwnd() は有効。&cds は本呼び出し中生存。
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    helper.hwnd(),
                    WM_COPYDATA,
                    Some(windows::Win32::Foundation::WPARAM(parent.hwnd().0 as usize)),
                    Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
                );
            }
        }
        assert_eq!(
            helper.shared().bad_frames.get(),
            1,
            "不正フレームは crash させず記録のみ（要件 2.5）"
        );
        // 不正フレームでは RESPONSE を送らない（無応答）。
        assert_eq!(responses.get(), 1, "不正フレームに応答しない（無応答）");

        // --- bounded ループ生存: 有限個の WM_NULL を撒いてから quit し、必ず抜ける（無クラッシュ）---
        let pumped = Rc::new(Cell::new(0u32));
        const PUMP_N: u32 = 8;
        // SAFETY: helper.hwnd() は有効。WM_NULL(0) は無害な起こし用メッセージ。
        for _ in 0..PUMP_N {
            unsafe {
                let _ = PostMessageW(
                    Some(helper.hwnd()),
                    0, // WM_NULL
                    windows::Win32::Foundation::WPARAM(0),
                    LPARAM(0),
                );
            }
        }
        {
            let pumped = pumped.clone();
            MessageLoop::run(move |msg_loop, _msg| {
                pumped.set(pumped.get() + 1);
                if pumped.get() >= PUMP_N {
                    msg_loop.quit();
                }
                FilterResult::Forward
            });
        }
        assert!(
            pumped.get() >= PUMP_N,
            "bounded ループが有限回で必ず抜ける（無限ループでない・要件 6.3）"
        );

        // 窓は Drop で破棄。
        drop(helper);
        drop(parent);
    }
}
