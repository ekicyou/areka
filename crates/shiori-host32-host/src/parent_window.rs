//! `ParentMessageWindow`（要件 3.2 / 3.4・design.md §383-435）: x64/arm64 親側の
//! message-only 窓で HELLO ハンドシェイクを観測する。
//!
//! 本タスク（4.2）の責務は 2 点に限定される（design.md §390-398）:
//!
//! 1. **HELLO 記録**（要件 3.2）: WndProc で inbound WM_COPYDATA を framing 検証し、
//!    `Hello` なら payload（helper 窓 HWND の u32 LE）を `decode_hwnd_le` で復号して
//!    [`ParentShared::helper_hwnd`] に記録＝ハンドシェイク完了を観測する。不正フレーム／
//!    未知タグ／想定外タグは crash させず観測カウンタに記録するのみ（要件 2.5・上位へ渡さない）。
//! 2. **`pump_until_hello_or`**（要件 3.4）: HELLO 受領（helper HWND 確定）まで、または
//!    `timeout` 経過まで `MessageLoop` を **bounded** に回す。無入力でも期限で必ず抜けられる
//!    よう別スレッドから自窓へ定期 `PostMessageW(WM_NULL)` を撃って `GetMessage` を起こす
//!    （heartbeat・pump フェーズ専用）。受領なら `Some(helper_hwnd)`、期限内未受領なら `None`
//!    （呼び出し側は `None` を [`crate::HandshakeError::Timeout`] として扱える）。
//!
//! ## 本タスクのスコープ外（task 4.3 が追加する）
//! 送信パス `send_request`・RESPONSE の `ResponseSlot` 再入受領・ハンドシェイクゲート
//! （`Incomplete` で送信拒否）は task 4.3 の領分ゆえ本ファイルには入れない。よって
//! `ParentShared` は `helper_hwnd` と観測カウンタのみを持ち、`ResponseSlot` フィールドは
//! 4.3 が追加する（未使用フィールドで dead_code/clippy 警告を出さないため）。
//!
//! ## 窓状態の共有パターン（design.md §426-429）
//! `wintf-winmsg-executor` の `Window<S>` は `state: S` を窓と同居させ WndProc へ `Pin<&S>`
//! で渡す（`Rc` 不要・GWLP_USERDATA 手詰め不要）。single-in-flight・単一 UI スレッドゆえ
//! 内部可変は [`Cell`] で足りる。

use std::cell::Cell;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use shiori_host32_ipc::{
    FramingError, MsgTag, copydata_payload, decode_hwnd_le, encode_hwnd_le, hwnd_from_u32,
};
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{FilterResult, MessageLoop};
#[cfg(test)]
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_COPYDATA};

/// heartbeat（別スレッドからの `PostMessageW(WM_NULL)`）の送信間隔。
///
/// pump フェーズ専用の起こし用。無入力でも `GetMessage` をブロックさせ続けず、
/// `pump_until_hello_or` のループが deadline を再評価できるようにする（design.md §398/§429）。
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(25);

/// inbound WM_COPYDATA の framing 検証結果に応じた親 WndProc の取るべき動作（窓なしで単体検証可）。
///
/// 窓・FFI から切り離した純ロジックとして [`classify_inbound`] が算出し、WndProc はこの結果を
/// 見て副作用（状態記録・カウンタ更新）を行う。これにより HELLO 記録・不正フレーム分類を
/// 実窓なしで決定的に単体検証できる（helper の `classify_inbound` 方式を踏襲）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum InboundAction {
    /// `Hello` を受領。同梱の helper HWND（u32 ワイヤ値）を記録すべき＝ハンドシェイク完了。
    RecordHello(u32),
    /// framing 上は正当だが親が能動処理しないタグ（`Load`/`Request`/`Response`/`Unload`）。
    /// crash させず記録のみ（本タスクでは `Response` の再入受領は 4.3 が担う）。
    IgnoreKnown(MsgTag),
    /// 未知タグ・長さ不整合など不正フレーム。crash させず記録のみ・上位へ渡さない（要件 2.5）。
    IgnoreBad(FramingError),
    /// `Hello` だが payload 長が 4 でない（HWND u32 LE でない）。記録せず無視する。
    IgnoreMalformedHello,
}

/// inbound WM_COPYDATA の生値（tag 生値・宣言長・実データ）を [`InboundAction`] へ写像する純関数。
///
/// framing 検証は `shiori-host32-ipc::copydata_payload` に委譲（重複実装しない）。`Hello` のみ
/// payload を `decode_hwnd_le` で復号して `RecordHello` とし、その他の既知タグは `IgnoreKnown`、
/// 不正フレームは `IgnoreBad`、payload 長不正の HELLO は `IgnoreMalformedHello` とする。
/// 副作用（状態記録・カウンタ）は持たず、WndProc 側が結果を見て実行する。
fn classify_inbound(dw_data: usize, declared_len: usize, data: &[u8]) -> InboundAction {
    match copydata_payload(dw_data, declared_len, data) {
        Ok((MsgTag::Hello, payload)) => {
            if payload.len() == 4 {
                let bytes = [payload[0], payload[1], payload[2], payload[3]];
                InboundAction::RecordHello(decode_hwnd_le(bytes))
            } else {
                InboundAction::IgnoreMalformedHello
            }
        }
        Ok((tag, _)) => InboundAction::IgnoreKnown(tag),
        Err(err) => InboundAction::IgnoreBad(err),
    }
}

/// WndProc と外側（呼び出し側）で共有する親窓の状態（design.md §426-429）。
///
/// single-in-flight・単一 UI スレッドゆえ内部可変は [`Cell`] で足りる。本タスク（4.2）では
/// ハンドシェイク観測に必要な `helper_hwnd` と観測カウンタのみを持つ。送信パス（`ResponseSlot`）は
/// task 4.3 が追加する。
struct ParentShared {
    /// HELLO で受領した helper のメッセージ窓 HWND（u32 ワイヤ値）。確定でハンドシェイク完了。
    helper_hwnd: Cell<Option<u32>>,
    /// 観測カウンタ: 受領した HELLO 数。
    hellos: Cell<u64>,
    /// 観測カウンタ: 応答対象外の既知タグ受領数（記録のみ・crash なし）。
    ignored_known: Cell<u64>,
    /// 観測カウンタ: 不正フレーム／未知タグ／不正 HELLO 受領数（crash させず記録・要件 2.5）。
    bad_frames: Cell<u64>,
}

impl ParentShared {
    fn new() -> Self {
        Self {
            helper_hwnd: Cell::new(None),
            hellos: Cell::new(0),
            ignored_known: Cell::new(0),
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

/// 親 WndProc 本体: WM_COPYDATA を [`classify_inbound`] で分類し、`RecordHello` なら helper HWND を
/// 記録＝ハンドシェイク完了を観測して即 return する（design.md §393-395・要件 3.2）。
///
/// **非ブロッキング**（記録して即 return・跨プロセス SendMessage を発行しない）。不正フレーム／
/// 未知タグ／想定外タグは crash させず観測カウンタに記録するのみ（要件 2.5・上位へ渡さない）。
fn handle_message(s: &ParentShared, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // 非対象は DefWindowProc へ委譲（closure が None を返すと lib が委譲）。
    }

    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS 契約）。
    let Some((dw_data, declared_len, payload)) = (unsafe { read_copydata(msg.lparam) }) else {
        return Some(LRESULT(0));
    };

    match classify_inbound(dw_data, declared_len, &payload) {
        InboundAction::RecordHello(helper_hwnd) => {
            // ハンドシェイク完了観測（要件 3.2）: helper HWND を確定させ即 return。
            s.helper_hwnd.set(Some(helper_hwnd));
            s.hellos.set(s.hellos.get() + 1);
        }
        InboundAction::IgnoreKnown(_tag) => {
            // Load/Request/Response/Unload は本タスクでは能動処理しない。記録のみ
            // （Response の再入受領は task 4.3 が RESPONSE アームを追加する・design.md §394）。
            s.ignored_known.set(s.ignored_known.get() + 1);
        }
        InboundAction::IgnoreBad(_err) => {
            // 未知タグ・長さ不整合は crash させず記録のみ・上位へ渡さない（要件 2.5）。
            s.bad_frames.set(s.bad_frames.get() + 1);
        }
        InboundAction::IgnoreMalformedHello => {
            // HELLO だが payload 長が 4 でない。記録せず不正として計上（要件 2.5）。
            s.bad_frames.set(s.bad_frames.get() + 1);
        }
    }
    Some(LRESULT(0))
}

/// ParentMessageWindow（design.md §383-435・要件 3.2 / 3.4）。
///
/// message-only 親窓を保持し、Drop で `DestroyWindow`（RAII）。helper を spawn する前に
/// [`create`](Self::create) で立て、HELLO の受け皿にする。
pub struct ParentMessageWindow {
    window: Window<ParentShared>,
}

impl ParentMessageWindow {
    /// message-only 親窓を生成する（design.md §413・要件 3.2）。
    ///
    /// helper 起動前に立て、HELLO 受領の受け皿にする。
    ///
    /// # Errors
    /// 窓生成失敗時に [`WindowCreationError`] を返す。design.md §413 は戻り値を
    /// `Result<Self, HandshakeError>` と記すが、**窓生成失敗はハンドシェイクの失敗ではない**
    /// （HELLO のやり取り以前の段階の失敗）ため、ハンドシェイク意味論を持つ
    /// [`crate::HandshakeError`] へ畳み込まず、窓生成固有の型で報告する（CONCERNS 参照）。
    pub fn create() -> Result<Self, WindowCreationError> {
        let shared = ParentShared::new();
        let window = Window::new(
            WindowType::MessageOnly,
            shared,
            move |state: Pin<&ParentShared>, msg: WindowMessage| -> Option<LRESULT> {
                handle_message(state.get_ref(), &msg)
            },
        )
        .map_err(|e| WindowCreationError(format!("{e:?}")))?;
        Ok(Self { window })
    }

    /// 親窓 HWND を u32 ワイヤ値で返す（`ProcessHost::spawn` へ渡す親 HWND・design.md §414）。
    #[must_use]
    pub fn hwnd_u32(&self) -> u32 {
        u32::from_le_bytes(encode_hwnd_le(self.window.hwnd()))
    }

    /// HELLO 受領（helper HWND 確定）まで、または `timeout` 経過までメッセージループを
    /// **bounded** に回す（要件 3.2 / 3.4・design.md §398/§415）。
    ///
    /// 受領なら `Some(helper_hwnd)`（u32 ワイヤ値）、期限内未受領なら `None`。呼び出し側は
    /// `None` を [`crate::HandshakeError::Timeout`] として扱える。
    ///
    /// 無入力でも期限で必ず抜けられるよう、別スレッドから自窓へ定期 `PostMessageW(WM_NULL)` を
    /// 撃って `GetMessage` を起こす（heartbeat・pump フェーズ専用）。ループの各回で deadline と
    /// helper HWND 確定を再評価し、いずれかで `msg_loop.quit()` する。
    #[must_use]
    pub fn pump_until_hello_or(&self, timeout: Duration) -> Option<u32> {
        // 既に受領済みなら即返す（helper が先に HELLO を送っている場合）。
        if let Some(h) = self.window.state().helper_hwnd.get() {
            return Some(h);
        }

        let deadline = Instant::now() + timeout;
        let state_ptr = self.window.state();

        // heartbeat: 別スレッドから自窓へ WM_NULL を撃ち、無入力でも GetMessage を起こす。
        let hb_hwnd_u32 = self.hwnd_u32();
        let stop = Arc::new(AtomicBool::new(false));
        let heartbeat = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let hwnd = hwnd_from_u32(hb_hwnd_u32);
                while !stop.load(Ordering::Relaxed) {
                    // SAFETY: WM_NULL(0) は無害な起こし用メッセージ。hwnd は自窓（生存中）。
                    unsafe {
                        let _ = PostMessageW(Some(hwnd), 0, WPARAM(0), LPARAM(0));
                    }
                    std::thread::sleep(HEARTBEAT_INTERVAL);
                }
            })
        };

        MessageLoop::run(|msg_loop, _msg| {
            if state_ptr.helper_hwnd.get().is_some() || Instant::now() >= deadline {
                msg_loop.quit();
            }
            FilterResult::Forward
        });

        stop.store(true, Ordering::Relaxed);
        let _ = heartbeat.join();

        self.window.state().helper_hwnd.get()
    }

    /// 自窓 HWND（loopback セルフテストからの観測用）。
    #[cfg(test)]
    fn hwnd(&self) -> HWND {
        self.window.hwnd()
    }

    /// 共有状態（観測カウンタ）への参照（loopback セルフテストからの観測用）。
    #[cfg(test)]
    fn shared(&self) -> Pin<&ParentShared> {
        self.window.state()
    }
}

/// 親 message-only 窓の生成失敗（`create` 固有・ハンドシェイク以前の失敗）。
///
/// 窓生成は HELLO のやり取り以前の段階であり、ハンドシェイク意味論を持つ
/// [`crate::HandshakeError`] とは別の失敗クラスである（design.md §413 は
/// `HandshakeError` を記すが、意味論整合を優先して固有型を用いる・CONCERNS 参照）。
#[derive(thiserror::Error, Debug)]
#[error("failed to create parent message-only window: {0}")]
pub struct WindowCreationError(String);

#[cfg(test)]
mod classify_tests {
    use super::*;

    // 要件 3.2: HELLO（helper HWND u32 LE 4 バイト）は RecordHello へ分類される。
    #[test]
    fn hello_classifies_to_record_with_decoded_hwnd() {
        for value in [0x1u32, 0x1234_5678, 0xDEAD_BEEF, u32::MAX] {
            let payload = value.to_le_bytes();
            let raw = MsgTag::Hello.as_u32() as usize;
            let action = classify_inbound(raw, payload.len(), &payload);
            assert_eq!(
                action,
                InboundAction::RecordHello(value),
                "HELLO payload {value:#x} を decode_hwnd_le で復号して記録する"
            );
        }
    }

    // 要件 3.2: HELLO 復号は copydata_payload→decode_hwnd_le の等価性を満たす。
    #[test]
    fn hello_decode_matches_ipc_primitives() {
        let value = 0xCAFE_F00Du32;
        let payload = encode_hwnd_le(hwnd_from_u32(value));
        let raw = MsgTag::Hello.as_u32() as usize;
        assert_eq!(
            classify_inbound(raw, payload.len(), &payload),
            InboundAction::RecordHello(value)
        );
    }

    // 要件 2.5: HELLO だが payload 長が 4 でない → 記録せず不正扱い。
    #[test]
    fn malformed_hello_is_ignored() {
        let raw = MsgTag::Hello.as_u32() as usize;
        // 3 バイト（短い）
        assert_eq!(
            classify_inbound(raw, 3, b"abc"),
            InboundAction::IgnoreMalformedHello
        );
        // 5 バイト（長い）
        assert_eq!(
            classify_inbound(raw, 5, b"abcde"),
            InboundAction::IgnoreMalformedHello
        );
        // 空
        assert_eq!(
            classify_inbound(raw, 0, b""),
            InboundAction::IgnoreMalformedHello
        );
    }

    // 要件 2.5: helper → 親では来ない既知タグ／親発方向タグは IgnoreKnown（記録のみ・crash なし）。
    #[test]
    fn known_nonhello_tags_are_ignored_known() {
        for tag in [MsgTag::Load, MsgTag::Request, MsgTag::Response, MsgTag::Unload] {
            let raw = tag.as_u32() as usize;
            assert_eq!(
                classify_inbound(raw, 0, b""),
                InboundAction::IgnoreKnown(tag)
            );
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
        let raw = MsgTag::Hello.as_u32() as usize;
        let action = classify_inbound(raw, 10, b"abc");
        assert!(matches!(action, InboundAction::IgnoreBad(_)));
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    /// 単一 loopback テスト（窓 1 組制約を厳守・deterministic）。
    ///
    /// wintf の message-only 窓は同一プロセスで 2 組独立生成すると 2 組目が WindowCreationError に
    /// なる既知制約ゆえ、窓は 1 組（親窓のみ）に集約する。以下を順に検証する:
    ///
    /// - (a) HELLO 未受領で短い timeout の `pump_until_hello_or` が `None`（要件 3.4・Timeout 経路）。
    /// - (b) 同じ親窓へ HELLO を loopback 送出 → `pump_until_hello_or` が `Some(helper_hwnd)` を返し helper_hwnd が確定する（要件 3.2）。
    /// - (c) 不正フレーム（未知タグ）を親窓へ送っても crash せず観測カウンタ記録のみ（要件 2.5）。
    ///
    /// bounded: いずれの pump も timeout / 受領 / quit で必ず抜ける（無限ループ禁止）。
    #[test]
    fn pump_none_then_some_and_bad_frame_recorded() {
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");

        // --- (a) HELLO 未受領 → 短い timeout で None（要件 3.4）---
        let before = Instant::now();
        let none = parent.pump_until_hello_or(Duration::from_millis(120));
        assert_eq!(none, None, "HELLO 未受領なら pump は None を返す（要件 3.4）");
        assert!(
            before.elapsed() < Duration::from_secs(5),
            "pump は timeout で bounded に抜ける（無限待機しない）"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);

        // --- (b) HELLO を loopback 送出 → pump が Some(helper_hwnd) を返す（要件 3.2）---
        // 親窓宛に HELLO（helper HWND u32 LE）を送る。send_copydata は同期送出ゆえ、
        // WndProc が呼ばれ helper_hwnd が確定した状態で戻る。
        let helper_hwnd_wire: u32 = 0x0BAD_F00D;
        let hello_payload = helper_hwnd_wire.to_le_bytes();
        shiori_host32_ipc::send_copydata(
            parent.hwnd(),
            parent.hwnd(),
            MsgTag::Hello,
            &hello_payload,
            Duration::from_secs(5),
        )
        .expect("HELLO loopback 送出に失敗");

        // 送出直後に helper_hwnd は確定しているはず（send_copydata は同期）。
        assert_eq!(
            parent.shared().helper_hwnd.get(),
            Some(helper_hwnd_wire),
            "HELLO 受領で helper HWND が確定する（要件 3.2）"
        );
        assert_eq!(parent.shared().hellos.get(), 1);

        // 確定後の pump は即 Some を返す（bounded・要件 3.2）。
        let some = parent.pump_until_hello_or(Duration::from_secs(5));
        assert_eq!(
            some,
            Some(helper_hwnd_wire),
            "HELLO 受領後 pump は Some(helper_hwnd) を返す（要件 3.2）"
        );

        // --- (c) 不正フレーム（未知タグ）→ crash せず bad_frames 記録のみ（要件 2.5）---
        {
            let payload: &[u8] = b"";
            let cds = COPYDATASTRUCT {
                dwData: 0xFFusize,
                cbData: 0,
                lpData: payload.as_ptr() as *mut core::ffi::c_void,
            };
            // SAFETY: parent.hwnd() は有効。&cds は本呼び出し中生存。
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    parent.hwnd(),
                    WM_COPYDATA,
                    Some(WPARAM(parent.hwnd().0 as usize)),
                    Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
                );
            }
        }
        assert_eq!(
            parent.shared().bad_frames.get(),
            1,
            "不正フレームは crash させず記録のみ（要件 2.5）"
        );
        // 不正フレームは helper_hwnd を書き換えない。
        assert_eq!(parent.shared().helper_hwnd.get(), Some(helper_hwnd_wire));

        drop(parent);
    }
}
