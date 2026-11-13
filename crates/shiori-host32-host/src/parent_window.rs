//! `ParentMessageWindow`（要件 3.2 / 3.4・design.md §383-435）: x64/arm64 親側の
//! message-only 窓で HELLO ハンドシェイクを観測する。
//!
//! 責務（design.md §390-398）:
//!
//! 1. **HELLO 記録**（要件 3.2）: WndProc で inbound WM_COPYDATA を framing 検証し、
//!    `Hello` なら payload（helper 窓 HWND の u32 LE）を `decode_hwnd_le` で復号して
//!    [`ParentShared::helper_hwnd`] に記録＝ハンドシェイク完了を観測する（1 度だけ・送り手の窓と
//!    中身の窓が同じときだけ・指摘 38。RESPONSE も helper の窓からだけ受ける）。不正フレーム／
//!    未知タグ／想定外タグは crash させず WARN の記録を残すのみ（要件 2.5・上位へ渡さない）。
//! 2. **`pump_until_hello_or`**（要件 3.4）: HELLO 受領（helper HWND 確定）まで、または
//!    `timeout` 経過まで `MessageLoop` を **bounded** に回す。無入力でも期限で必ず抜けられる
//!    よう別スレッドから自窓へ定期 `PostMessageW(WM_NULL)` を撃って `GetMessage` を起こす
//!    （heartbeat・握手フェーズの起こし専用。定常時は heartbeat を用いない＝責務 5）。受領なら
//!    `Some(helper_hwnd)`、期限内未受領なら `None`（呼び出し側は `None` を
//!    [`crate::HandshakeError::Timeout`] として扱える）。
//! 3. **RESPONSE 再入受領**（task 4.3・要件 4.2）: WndProc で `Response` を受けたら payload を
//!    [`ParentShared::response_slot`] へ store して**即 return**（跨プロセス SendMessage を
//!    発行しない＝デッドロック回避の核）。
//! 4. **送信パス `send_request`**（task 4.3・要件 3.3/4.1/4.4/5.x）: ハンドシェイクゲート下で
//!    `slot.clear → SendMessageTimeout(REQUEST, SMTO_ABORTIFHUNG) → slot.take` の 1 往復。
//!    未ハンドシェイクは [`SendError::Handshake`]（`Incomplete`）で拒否し、上限内未応答は
//!    [`SendError::Ipc`]（[`IpcError::Timeout`]）で復帰する。
//! 5. **定常時の保守 `pump_pending_messages`**: 握手の後、往復の外で呼び出し側が周期的に呼び、
//!    自スレッドのキューを空になるまで取り出して配る（窓が応答なしと判定されないため）。
//!    heartbeat スレッドは要さない。往復の内側からは呼ばない。
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
    FramingError, IpcError, MsgTag, ResponseSlot, copydata_payload, decode_hwnd_le, encode_hwnd_le,
    hwnd_from_u32, send_request as ipc_send_request,
};

use crate::error::HandshakeError;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetWindowThreadProcessId, MSG, PM_REMOVE, PeekMessageW, PostMessageW,
    TranslateMessage, WM_COPYDATA,
};
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

/// heartbeat（別スレッドからの `PostMessageW(WM_NULL)`）の送信間隔。
///
/// 握手フェーズ（`pump_until_hello_or`）専用の起こし用。無入力でも `GetMessage` をブロックさせ続けず、
/// `pump_until_hello_or` のループが deadline を再評価できるようにする（design.md §398/§429）。
/// 定常時の保守（`pump_pending_messages`）は呼び出し側の周期で回るため、これを用いない。
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(25);

/// inbound WM_COPYDATA の framing 検証結果に応じた親 WndProc の取るべき動作（窓なしで単体検証可）。
///
/// 窓・FFI から切り離した純ロジックとして [`classify_inbound`] が算出し、WndProc はこの結果を
/// 見て副作用（状態記録・記録の行）を行う。これにより HELLO 記録・不正フレーム分類を
/// 実窓なしで決定的に単体検証できる（helper の `classify_inbound` 方式を踏襲）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum InboundAction {
    /// `Hello` を受領。同梱の helper HWND（u32 ワイヤ値）を記録すべき＝ハンドシェイク完了。
    RecordHello(u32),
    /// `Response` を受領。同梱 payload を [`ParentShared::response_slot`] へ store すべき
    /// ＝再入受領（要件 4.2・store 後即 return）。
    StoreResponse(Vec<u8>),
    /// framing 上は正当だが親が能動処理しないタグ（`Load`/`Request`/`Unload`）。
    /// crash させず記録のみ。
    IgnoreKnown(MsgTag),
    /// 未知タグ・長さ不整合など不正フレーム。crash させず記録のみ・上位へ渡さない（要件 2.5）。
    IgnoreBad(FramingError),
    /// `Hello` だが payload 長が 4 でない（HWND u32 LE でない）。記録せず無視する。
    IgnoreMalformedHello,
}

/// inbound WM_COPYDATA の生値（tag 生値・宣言長・実データ）を [`InboundAction`] へ写像する純関数。
///
/// framing 検証は `shiori-host32-ipc::copydata_payload` に委譲（重複実装しない）。`Hello` は
/// payload を `decode_hwnd_le` で復号して `RecordHello`、`Response` は payload を `StoreResponse`、
/// その他の既知タグは `IgnoreKnown`、不正フレームは `IgnoreBad`、payload 長不正の HELLO は
/// `IgnoreMalformedHello` とする。
/// 副作用（状態記録・記録の行）は持たず、WndProc 側が結果を見て実行する。
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
        Ok((MsgTag::Response, payload)) => InboundAction::StoreResponse(payload.to_vec()),
        Ok((tag, _)) => InboundAction::IgnoreKnown(tag),
        Err(err) => InboundAction::IgnoreBad(err),
    }
}

/// ハンドシェイクゲート（要件 3.3・design.md §396）: helper HWND が確定しているかを判定し、
/// 送信先 HWND を返す純関数（窓・FFI から切り離して単体検証可能）。
///
/// `helper_hwnd_wire` が `None`（HELLO 未受領＝ハンドシェイク未完）なら
/// [`SendError::Handshake`]（[`HandshakeError::Incomplete`]）で拒否し、往復を開始させない
/// （要件 3.3・「ハンドシェイクが未完了である間は往復を開始しない」）。確定済なら
/// [`hwnd_from_u32`] で当該プロセスの `HWND` へ復元して返す。
fn resolve_send_target(helper_hwnd_wire: Option<u32>) -> Result<HWND, SendError> {
    match helper_hwnd_wire {
        Some(wire) => Ok(hwnd_from_u32(wire)),
        None => Err(SendError::Handshake(HandshakeError::Incomplete)),
    }
}

/// WndProc と外側（呼び出し側）で共有する親窓の状態（design.md §426-429）。
///
/// single-in-flight・単一 UI スレッドゆえ内部可変は [`Cell`] で足りる。本タスク（4.2）では
/// ハンドシェイク観測に必要な `helper_hwnd` のみを持つ。送信パス（`ResponseSlot`）は
/// task 4.3 が追加する。
struct ParentShared {
    /// HELLO で受領した helper のメッセージ窓 HWND（u32 ワイヤ値）。確定でハンドシェイク完了。
    helper_hwnd: Cell<Option<u32>>,
    /// single-in-flight の応答受け皿（要件 4.1/4.2/4.3・design.md §426-429）。
    /// `send_request` が `clear→…→take` で消費し、RESPONSE の WndProc アームが再入で `store` する。
    /// 両者は同一 `&ResponseSlot` を参照する（`send_request` は `self.window.state()` 経由で取得）。
    response_slot: ResponseSlot,
    /// HELLO を受けてよい子のプロセス番号（[`ParentMessageWindow::expect_helper_pid`]・指摘 38）。
    expected_pid: Cell<Option<u32>>,
}

impl ParentShared {
    fn new() -> Self {
        Self {
            helper_hwnd: Cell::new(None),
            response_slot: ResponseSlot::new(),
            expected_pid: Cell::new(None),
        }
    }
}

/// HELLO を受けてよいかを確かめる（指摘 38）。受けないときは記録の `reason` を返す。
///
/// 1 度だけ・送り手の窓（`wParam`）が中身の窓と同じときだけ受ける。子のプロセス番号が
/// 伝えられていれば、送り手の窓のプロセス番号も確かめる。
fn hello_rejection(
    s: &ParentShared,
    sender: HWND,
    sender_wire: u32,
    hello: u32,
) -> Option<&'static str> {
    if s.helper_hwnd.get().is_some() {
        return Some("second_hello");
    }
    if sender_wire != hello {
        return Some("hello_sender_mismatch");
    }
    if let Some(pid) = s.expected_pid.get() {
        let mut sender_pid = 0u32;
        // SAFETY: sender は任意の値でよい（無効な窓なら 0 を返し sender_pid も 0 のまま）。
        unsafe { GetWindowThreadProcessId(sender, Some(&mut sender_pid)) };
        if sender_pid != pid {
            return Some("hello_from_other_process");
        }
    }
    None
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
/// 記録＝ハンドシェイク完了、`StoreResponse` なら payload を `response_slot` へ store（再入受領・
/// 要件 4.2）して即 return する（design.md §393-395）。
///
/// **非ブロッキング**（記録／store して即 return・跨プロセス SendMessage を発行しない）。不正フレーム／
/// 未知タグ／想定外タグは crash させず WARN の記録を残すのみ（要件 2.5・上位へ渡さない）。
fn handle_message(s: &ParentShared, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // 非対象は DefWindowProc へ委譲（closure が None を返すと lib が委譲）。
    }

    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS 契約）。
    let Some((dw_data, declared_len, payload)) = (unsafe { read_copydata(msg.lparam) }) else {
        return Some(LRESULT(0));
    };

    // 送り手の窓（WM_COPYDATA の wParam・ipc の送り手は自分の窓を書く）。
    let sender = HWND(msg.wparam.0 as *mut core::ffi::c_void);
    let sender_wire = u32::from_le_bytes(encode_hwnd_le(sender));

    match classify_inbound(dw_data, declared_len, &payload) {
        InboundAction::RecordHello(helper_hwnd) => {
            match hello_rejection(s, sender, sender_wire, helper_hwnd) {
                // ハンドシェイク完了観測（要件 3.2）: helper HWND を確定させ即 return。
                None => s.helper_hwnd.set(Some(helper_hwnd)),
                Some(reason) => tracing::warn!(
                    target: "shiori-host32",
                    event = "parent_frame_ignored",
                    reason,
                    sender = sender_wire,
                    hello = helper_hwnd,
                    "受けてよい送り手からでない HELLO を無視した"
                ),
            }
        }
        // shortcut: wParam は送り手の自己申告なので、helper の窓を名乗る同じ利用者のプロセスの RESPONSE は防げない（同じ利用者のプロセスは DLL の差し込みなど、もっと強いことができる）。防ぐ要が出たら、起こすたびの合言葉を枠に載せる形へ上げる
        InboundAction::StoreResponse(_) if s.helper_hwnd.get() != Some(sender_wire) => {
            tracing::warn!(
                target: "shiori-host32",
                event = "parent_frame_ignored",
                reason = "response_not_from_helper",
                sender = sender_wire,
                "helper の窓でない送り手からの RESPONSE を無視した"
            );
        }
        InboundAction::StoreResponse(payload) => {
            // 再入受領の核（要件 4.2・design.md §394）: RESPONSE payload を受け皿へ store して
            // 即 return する。ここで跨プロセス SendMessage を一切発行しない（循環待ちなし）。
            // 親は `send_request` の `SendMessageTimeout` でブロック中であり、helper の
            // RESPONSE（同期 SendMessage）が OS によりこの WndProc へ再入配送される。
            s.response_slot.store(payload);
        }
        // 無視したものは crash させず WARN を 1 行残すだけ・上位へ渡さない（要件 2.5・指摘 11）。
        InboundAction::IgnoreKnown(tag) => tracing::warn!(
            target: "shiori-host32",
            event = "parent_frame_ignored",
            reason = "not_for_parent",
            tag = ?tag,
            "親の窓が受けない種類のメッセージを無視した"
        ),
        InboundAction::IgnoreBad(FramingError::UnknownTag(raw)) => tracing::warn!(
            target: "shiori-host32",
            event = "parent_frame_ignored",
            reason = "unknown_tag",
            raw,
            "知らない種類のメッセージを無視した"
        ),
        InboundAction::IgnoreBad(FramingError::LengthMismatch { declared, actual }) => {
            tracing::warn!(
                target: "shiori-host32",
                event = "parent_frame_ignored",
                reason = "length_mismatch",
                declared,
                actual,
                "宣言の長さと中身の長さが合わないメッセージを無視した"
            );
        }
        InboundAction::IgnoreMalformedHello => tracing::warn!(
            target: "shiori-host32",
            event = "parent_frame_ignored",
            reason = "malformed_hello",
            len = payload.len(),
            "中身が 4 バイトでない HELLO を無視した"
        ),
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

    /// HELLO を受けてよい子のプロセス番号を伝える（spawn の直後・握手を回す前に呼ぶ・指摘 38）。
    ///
    /// 呼んだ後は、送り手の窓がこのプロセスのものでない HELLO を記録して捨てる。呼ばなければ
    /// 「1 度だけ・送り手と中身の窓が同じ」の決まりだけで受ける。
    pub fn expect_helper_pid(&self, pid: u32) {
        self.window.state().expected_pid.set(Some(pid));
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
    /// 撃って `GetMessage` を起こす（heartbeat・握手フェーズの起こし専用）。ループの各回で deadline と
    /// helper HWND 確定を再評価し、いずれかで `msg_loop.quit()` する。握手の後の定常時の保守は
    /// [`pump_pending_messages`](Self::pump_pending_messages) が往復の外で担い、heartbeat を要さない。
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

    /// ハンドシェイクゲート下で 1 往復（REQUEST → RESPONSE）を送る（要件 3.3/4.1/4.4/5.1/5.2/5.3・
    /// design.md §397/§417）。
    ///
    /// 手順:
    /// 1. **ハンドシェイクゲート（要件 3.3）**: helper HWND が未確定なら送信せず
    ///    [`SendError::Handshake`]（[`HandshakeError::Incomplete`]）で拒否する（往復を開始しない）。
    /// 2. **送信本体（要件 4.1/4.4/5.x）**: proto の [`ipc_send_request`] へ委譲する。内部で
    ///    `slot.clear → SendMessageTimeout(REQUEST, SMTO_ABORTIFHUNG, timeout) → slot.take`。
    ///    親はブロック中、待機の最中に helper の RESPONSE が親 WndProc へ**再入配送**され、
    ///    `StoreResponse` アームが `response_slot` へ store する。上限時間内に未受領なら
    ///    [`IpcError::Timeout`]（[`SendError::Ipc`] で包む）。single-in-flight。
    ///
    /// **heartbeat・定常時の保守と不干渉（design.md §429）**: 本関数は pump ループも heartbeat
    /// スレッドも起動せず、[`pump_pending_messages`](Self::pump_pending_messages) も呼ばない。
    /// in-flight 中は `SendMessageTimeout` がブロックし、キューの WM_NULL（`PostMessage`）は配送
    /// されないため、`clear→store→take` 不変条件が保たれる。heartbeat は `pump_until_hello_or` の
    /// 握手フェーズ専用であり、定常時の保守は往復の外でしか走らないため、in-flight 中の
    /// 不変条件は保たれる。
    ///
    /// `slot` は WndProc の RESPONSE アームが参照するものと同一 `&ResponseSlot`（`self.window.state()`
    /// 経由で取得）。
    ///
    /// # Errors
    /// - 未ハンドシェイク: [`SendError::Handshake`]（`Incomplete`）。
    /// - 送出失敗 / 上限内未応答 / ハング peer 中断: [`SendError::Ipc`]（`Timeout` / `SendFailed`）。
    pub fn send_request(
        &self,
        tag: MsgTag,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, SendError> {
        let state = self.window.state();
        // ① ハンドシェイクゲート（要件 3.3）: 未確定なら SendMessage を発行せず拒否。
        let target = resolve_send_target(state.helper_hwnd.get())?;
        let self_hwnd = self.window.hwnd();
        // ② 再入受領前提の 1 往復（要件 4.1/4.2/4.4/5.x）を proto へ委譲。RESPONSE は
        //    ブロック中に親 WndProc の StoreResponse アームが同一 slot へ store する。
        ipc_send_request(
            target,
            self_hwnd,
            tag,
            payload,
            timeout,
            &state.response_slot,
        )
        .map_err(SendError::Ipc)
    }

    /// 自窓を所有するスレッドのキューを空になるまで取り出して配る（定常時の保守）。
    ///
    /// 背景: Windows は「プロセス開始から 20〜30 秒を過ぎ、かつ窓を所有するスレッドが
    /// 一定時間（`IsHungAppWindow` は 5 秒・`SMTO_ABORTIFHUNG` の実測は 14 秒）メッセージを
    /// 取り出していない」窓を応答なしと判定する。往復の外で周期的に本メソッドを呼ぶ限り
    /// この判定に落ちない。呼び出し側の周期は `areka-kanade` の `IDLE_INTERVAL`。
    ///
    /// 往復（[`send_request`](Self::send_request)）の内側からは呼ばないこと
    /// （`clear→store→take` の不変条件が崩れる）。
    ///
    /// `ParentMessageWindow` は `!Send`／`!Sync`（`HWND` が生ポインタ・状態が [`Cell`]）ゆえ
    /// `&self` を持てるのは窓を作ったスレッドだけであり、`PeekMessageW` が呼び出しスレッドの
    /// キューを見る前提が型で保証される。資材を生成せず、失敗経路もログも持たない。
    /// `WM_QUIT` は特別扱いしない（このスレッドで `PostQuitMessage` を呼ぶ者はいない）。
    pub fn pump_pending_messages(&self) {
        let mut msg = MSG::default();
        // SAFETY: msg は本スレッドのスタック上で有効。取り出した msg をそのまま配るだけ。
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// 自窓 HWND（loopback セルフテストからの観測用）。
    #[cfg(test)]
    fn hwnd(&self) -> HWND {
        self.window.hwnd()
    }

    /// 共有状態への参照（loopback セルフテストからの観測用）。
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

/// 送信パス [`ParentMessageWindow::send_request`] の失敗（要件 3.3 / 5.2 / 5.3）。
///
/// design.md §417 は `send_request` の戻りを `Result<Vec<u8>, IpcError>` と記すが、
/// ハンドシェイクゲート（要件 3.3）が拒否する失敗は `IpcError`（transport 層）ではなく
/// [`HandshakeError::Incomplete`]（handshake 層）であり、型が食い違う。両層の失敗を
/// 呼び出し側が `?` で一様に扱えるよう、送信パス固有の統合エラーとして両者を保持する
/// （`IpcError` に handshake 意味論を混入させず、`HandshakeError` を transport へ拡張もしない・
/// 各型の単一責務を維持する）。CONCERNS 参照。
#[derive(thiserror::Error, Debug)]
pub enum SendError {
    /// ハンドシェイク未完（helper HWND 未確定）のまま送信が試みられた（要件 3.3・ゲート拒否）。
    #[error("send rejected by handshake gate: {0}")]
    Handshake(#[from] HandshakeError),
    /// transport 層の失敗（上限内未応答＝`Timeout` / 送出失敗＝`SendFailed`・要件 5.2/5.3）。
    #[error("ipc transport error: {0}")]
    Ipc(#[from] IpcError),
}

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

    // 要件 2.5: 親が能動処理しない既知タグ（Load/Request/Unload）は IgnoreKnown（記録のみ・crash なし）。
    #[test]
    fn known_nonhello_tags_are_ignored_known() {
        for tag in [MsgTag::Load, MsgTag::Request, MsgTag::Unload] {
            let raw = tag.as_u32() as usize;
            assert_eq!(
                classify_inbound(raw, 0, b""),
                InboundAction::IgnoreKnown(tag)
            );
        }
    }

    // 要件 4.2: RESPONSE は payload を StoreResponse へ分類する（再入受領で slot へ store）。
    #[test]
    fn response_classifies_to_store_with_payload() {
        let raw = MsgTag::Response.as_u32() as usize;
        let payload: &[u8] = b"echo-response";
        assert_eq!(
            classify_inbound(raw, payload.len(), payload),
            InboundAction::StoreResponse(payload.to_vec()),
            "RESPONSE payload を store 用に持ち上げる（要件 4.2）"
        );
        // 空 payload の RESPONSE も store 対象（0 バイト応答）。
        assert_eq!(
            classify_inbound(raw, 0, b""),
            InboundAction::StoreResponse(Vec::new())
        );
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
mod gate_tests {
    use super::*;

    // 要件 3.3: helper HWND 未確定なら送信ゲートが Incomplete で弾く（窓なしの純ロジック）。
    #[test]
    fn gate_rejects_when_helper_hwnd_unset() {
        let err = resolve_send_target(None).expect_err("未確定は Err で弾く");
        assert!(
            matches!(err, SendError::Handshake(HandshakeError::Incomplete)),
            "未ハンドシェイク送信は Incomplete（要件 3.3）"
        );
    }

    // 要件 3.3: helper HWND 確定なら target HWND を返し送信を許可する。
    #[test]
    fn gate_allows_when_helper_hwnd_set() {
        let wire: u32 = 0x1234_5678;
        let target = resolve_send_target(Some(wire)).expect("確定なら Ok");
        assert_eq!(
            target,
            hwnd_from_u32(wire),
            "確定 helper HWND を target とする"
        );
    }

    // SendError の Display/Debug（要件 3.3 / 5.2 の一様報告）。
    #[test]
    fn send_error_variants_render() {
        let hs = SendError::Handshake(HandshakeError::Incomplete);
        let ipc = SendError::Ipc(shiori_host32_ipc::IpcError::Timeout);
        assert!(!format!("{hs}").is_empty());
        assert!(!format!("{ipc}").is_empty());
        assert!(!format!("{hs:?}").is_empty());
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
    /// - (b) 同じ親窓へ自分の窓を名乗る HELLO を loopback 送出 → `pump_until_hello_or` が `Some(helper_hwnd)` を返し helper_hwnd が確定する（要件 3.2）。
    /// - (c) 無視したメッセージの記録は [`ignored_frames_are_logged`] が確かめる。
    /// - (d) RESPONSE を loopback 送出 → WndProc の Response アームが `response_slot` へ store する（要件 4.2）。
    /// - (e) 無応答相手（親自身）へ `send_request` → 上限時間で `SendError::Ipc(Timeout)`・親はハングしない（要件 5.1/5.2/5.3）。
    ///
    /// bounded: いずれの pump / send_request も timeout / 受領 / quit で必ず抜ける（無限ループ禁止）。
    #[test]
    fn pump_times_out_then_hello_response_and_send_timeout() {
        // 窓生成テストの直列化（wintf-winmsg-executor 0.0.5 の「同一プロセスで 2 組の
        // message-only 窓を同時生成すると 2 組目が WindowCreationError」既知制約への対処）。
        // lifecycle.rs の窓生成テストと同一ロックを取り、並列窓生成の衝突を防ぐ。
        // poison は無視する（ロックは相互排他のみが目的）。
        let _guard = crate::lifecycle::tests::WINDOW_TEST_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");

        // --- (a) HELLO 未受領 → 短い timeout で None（要件 3.4）---
        let before = Instant::now();
        let none = parent.pump_until_hello_or(Duration::from_millis(120));
        assert_eq!(
            none, None,
            "HELLO 未受領なら pump は None を返す（要件 3.4）"
        );
        assert!(
            before.elapsed() < Duration::from_secs(5),
            "pump は timeout で bounded に抜ける（無限待機しない）"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);

        // --- (b) HELLO を loopback 送出 → pump が Some(helper_hwnd) を返す（要件 3.2）---
        // 親窓宛に HELLO（helper HWND u32 LE）を送る。送り手と中身の窓が同じでないと捨てられる
        // （指摘 38）ので、親自身を helper として名乗る。send_copydata は同期送出ゆえ、
        // WndProc が呼ばれ helper_hwnd が確定した状態で戻る。
        let helper_hwnd_wire: u32 = parent.hwnd_u32();
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

        // 確定後の pump は即 Some を返す（bounded・要件 3.2）。
        let some = parent.pump_until_hello_or(Duration::from_secs(5));
        assert_eq!(
            some,
            Some(helper_hwnd_wire),
            "HELLO 受領後 pump は Some(helper_hwnd) を返す（要件 3.2）"
        );

        // --- (d) RESPONSE を親窓へ loopback → ResponseSlot に payload が store される（要件 4.2）---
        // WndProc の Response アームが store することを観測する。send_copydata は同期ゆえ、
        // 復帰時点で slot に格納済み。
        {
            parent.shared().response_slot.clear();
            let resp_payload: &[u8] = b"echoed-bytes";
            shiori_host32_ipc::send_copydata(
                parent.hwnd(),
                parent.hwnd(),
                MsgTag::Response,
                resp_payload,
                Duration::from_secs(5),
            )
            .expect("RESPONSE loopback 送出に失敗");
            assert_eq!(
                parent.shared().response_slot.take(),
                Some(resp_payload.to_vec()),
                "RESPONSE の WndProc アームが payload を ResponseSlot へ store する（要件 4.2）"
            );
        }

        // --- (e) 無応答相手への send_request は上限時間で Timeout（要件 5.1/5.2/5.3）---
        // helper_hwnd は (b) で親自身。REQUEST を親 WndProc が受けても RESPONSE を
        // store しない（Request アームは IgnoreKnown）→ slot は空のまま → Timeout。
        // SMTO_ABORTIFHUNG ＋短 timeout で有限復帰し親はハングしない。
        {
            let before = Instant::now();
            let result = parent.send_request(MsgTag::Request, b"ping", Duration::from_millis(150));
            assert!(
                matches!(
                    result,
                    Err(SendError::Ipc(shiori_host32_ipc::IpcError::Timeout))
                ),
                "無応答時は Timeout で復帰する（要件 5.2）: got {result:?}"
            );
            assert!(
                before.elapsed() < Duration::from_secs(5),
                "send_request は SMTO_ABORTIFHUNG ＋上限時間で有限復帰する（要件 5.3）"
            );
        }

        drop(parent);
    }

    /// 生の WM_COPYDATA を親窓へ同期送出する（`dwData`・中身をそのまま渡す・送り手は親自身）。
    fn send_raw(parent: &ParentMessageWindow, dw_data: usize, payload: &[u8]) {
        send_raw_from(parent, parent.hwnd_u32(), dw_data, payload);
    }

    /// 送り手の窓（`wParam`）を `sender` と名乗って生の WM_COPYDATA を親窓へ同期送出する。
    fn send_raw_from(parent: &ParentMessageWindow, sender: u32, dw_data: usize, payload: &[u8]) {
        let cds = COPYDATASTRUCT {
            dwData: dw_data,
            cbData: payload.len() as u32,
            lpData: payload.as_ptr() as *mut core::ffi::c_void,
        };
        // SAFETY: parent.hwnd() は有効。&cds と payload は本呼び出し中生存。
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                parent.hwnd(),
                WM_COPYDATA,
                Some(WPARAM(sender as usize)),
                Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
            );
        }
    }

    /// 無視したメッセージ（未知のタグ・形の崩れた HELLO・親が受けない既知の種類）は crash させず、
    /// それぞれ `parent_frame_ignored` の WARN を 1 行ずつ残す（指摘 11・要件 3.1 / 4.1）。
    /// 無視したメッセージは helper HWND を書き換えない。
    #[test]
    fn ignored_frames_are_logged() {
        let _guard = crate::lifecycle::tests::WINDOW_TEST_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");

        let ((), events) = log_capture_kit::capture(|| {
            send_raw(&parent, 0xFF, b"");
            send_raw(&parent, MsgTag::Hello.as_u32() as usize, b"abc");
            send_raw(&parent, MsgTag::Load.as_u32() as usize, b"");
        });
        let reasons: Vec<_> = events
            .iter()
            .filter(|e| {
                e.target == "shiori-host32" && e.field_str("event") == Some("parent_frame_ignored")
            })
            .map(|e| {
                assert_eq!(e.level, tracing::Level::WARN, "{e:?}");
                e.field_str("reason").unwrap_or_default().to_owned()
            })
            .collect();
        assert_eq!(
            reasons,
            ["unknown_tag", "malformed_hello", "not_for_parent"],
            "{events:?}"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);
    }

    /// `parent_frame_ignored` の WARN の `reason` を順に取り出す。
    fn ignored_reasons(events: &[log_capture_kit::CapturedEvent]) -> Vec<String> {
        events
            .iter()
            .filter(|e| {
                e.target == "shiori-host32" && e.field_str("event") == Some("parent_frame_ignored")
            })
            .map(|e| {
                assert_eq!(e.level, tracing::Level::WARN, "{e:?}");
                e.field_str("reason").unwrap_or_default().to_owned()
            })
            .collect()
    }

    /// よその送り手の HELLO を捨てる（指摘 38）: 送り手の窓と中身の窓が違う HELLO と、
    /// `expect_helper_pid` の番号でないプロセスの窓からの HELLO は記録して捨てる。
    /// 伝えた番号のプロセス（ここでは自分）の窓から名乗った HELLO は受ける。
    #[test]
    fn hello_from_a_foreign_sender_is_dropped() {
        let _guard = crate::lifecycle::tests::WINDOW_TEST_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
        let own = parent.hwnd_u32();
        let hello = MsgTag::Hello.as_u32() as usize;

        let ((), events) = log_capture_kit::capture(|| {
            // 送り手は親の窓・中身は別の窓。
            send_raw_from(&parent, own, hello, &0x0BAD_F00Du32.to_le_bytes());
            // 送り手と中身は同じだが、伝えた番号のプロセスの窓でない。
            parent.expect_helper_pid(std::process::id().wrapping_add(1));
            send_raw_from(&parent, own, hello, &own.to_le_bytes());
        });
        assert_eq!(
            ignored_reasons(&events),
            ["hello_sender_mismatch", "hello_from_other_process"],
            "{events:?}"
        );
        assert_eq!(parent.shared().helper_hwnd.get(), None);

        parent.expect_helper_pid(std::process::id());
        send_raw_from(&parent, own, hello, &own.to_le_bytes());
        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));
    }

    /// 2 度目の HELLO を捨てる（指摘 38）: helper の窓が決まった後の HELLO は記録して捨て、
    /// 決まった窓は変わらない。
    #[test]
    fn a_second_hello_is_dropped() {
        let _guard = crate::lifecycle::tests::WINDOW_TEST_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
        let own = parent.hwnd_u32();
        let hello = MsgTag::Hello.as_u32() as usize;

        send_raw_from(&parent, own, hello, &own.to_le_bytes());
        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));

        let ((), events) =
            log_capture_kit::capture(|| send_raw_from(&parent, own, hello, &own.to_le_bytes()));
        assert_eq!(ignored_reasons(&events), ["second_hello"], "{events:?}");
        assert_eq!(parent.shared().helper_hwnd.get(), Some(own));
    }

    /// helper でない送り手の RESPONSE を捨てる（指摘 38）: 握手の前の RESPONSE と、
    /// 送り手の窓が helper の窓でない RESPONSE は受け皿に入れず記録して捨てる。
    #[test]
    fn a_response_from_a_non_helper_sender_is_dropped() {
        let _guard = crate::lifecycle::tests::WINDOW_TEST_SERIAL
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
        let own = parent.hwnd_u32();
        let response = MsgTag::Response.as_u32() as usize;

        let ((), events) = log_capture_kit::capture(|| {
            // 握手の前（helper の窓が決まっていない）。
            send_raw_from(&parent, own, response, b"before-hello");
            send_raw_from(
                &parent,
                own,
                MsgTag::Hello.as_u32() as usize,
                &own.to_le_bytes(),
            );
            // helper の窓（親自身）でない送り手。
            send_raw_from(&parent, 0x0BAD_F00D, response, b"foreign");
        });
        assert_eq!(
            ignored_reasons(&events),
            ["response_not_from_helper", "response_not_from_helper"],
            "{events:?}"
        );
        assert_eq!(parent.shared().response_slot.take(), None);

        send_raw_from(&parent, own, response, b"from-helper");
        assert_eq!(
            parent.shared().response_slot.take(),
            Some(b"from-helper".to_vec())
        );
    }
}
