//! host-32 の i686 helper 実行バイナリ（HelperMessageWindow）。
//!
//! `wintf-winmsg-executor` 0.0.5 の message-only 窓 ＋ `MessageLoop::run` を i686 で回す
//! transport の helper 側。責務は 3 点に限る:
//!
//! 1. **起動時 HELLO**: 窓生成後、親へ自 HWND を u32 LE で最初の WM_COPYDATA として送る。
//! 2. **REQUEST → proxy 駆動 → 即 return**: `Request` の request バイト列を確立済み proxy の
//!    [`ShioriByteProxy::request`] へ流し、SHIORI/3.0 応答を `Response` として親へ **1 通だけ**
//!    返して即 return する（それ以上の跨プロセス `SendMessage` を発行しない）。
//!    proxy 未確立なら `SHIORI/3.0 500` を返し crash させない。
//!    不正フレーム／未知タグは crash させず観測カウンタに数え、`[helper]` の行に残すのみ（上位へ渡さない）。
//! 3. **`main`**: 親 HWND を arg/env の u32 ワイヤ値で取得 → 窓生成 → HELLO 送出 →
//!    `MessageLoop::run`。WndProc は LOAD・REQUEST・UNLOAD を受け、
//!    UNLOAD で proxy を閉じてループを抜け、exit 0 で終わる。
//!
//! ## proxy 駆動の差し替え点
//! 凍結する seam は WM_COPYDATA の REQUEST/RESPONSE ワイヤ形式であって proxy 実装ではないので、
//! trait 抽象は設けない。
//!
//! ## デッドロック回避
//! 往復は single-in-flight で厳密にネストし、WndProc は RESPONSE を 1 通返すだけで即 return するので、
//! 循環待ちは構造的に起きない。
//!
//! ## 依存方向
//! `shiori-host32-ipc`（proto）へ一方向に依存し、`shiori-host32-host` へは依存しない
//! （host↔helper はプロセス境界の WM_COPYDATA だけでつながる）。

// unsafe な FFI はここに一点集約する。
mod shiori_proxy;

#[cfg(test)]
mod proxy_test_support;

use shiori_proxy::ShioriByteProxy;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use shiori_host32_ipc::{
    FramingError, IpcError, MsgTag, copydata_payload, encode_hwnd_le, hwnd_from_u32, read_copydata,
    send_copydata, send_copydata_response,
};
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_COPYDATA};
use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

/// 応答送出 `SendMessageTimeoutW` の上限時間。
///
/// ハングした peer でも上限時間で戻るための上限。RESPONSE 返送は同期送出で即完了するので短めで足りる。
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// `[helper]` の行の出口。標準エラーへ 1 行書く（host の読み手が `warn` で記録へ回す）。
/// テストのときだけ同じスレッドの記録の列にも積む。
fn helper_log(line: &str) {
    use std::io::Write;
    // 書けない＝host の読み手が終わった。もう記録の行き先は無いので捨てる（eprintln! は窓の手続きの中で panic→abort）。
    let _ = writeln!(std::io::stderr().lock(), "{line}");
    #[cfg(test)]
    HELPER_LOG.with(|log| log.borrow_mut().push(line.to_string()));
}

#[cfg(test)]
thread_local! {
    static HELPER_LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// このスレッドで `helper_log` が書いた行を取り出す（取り出した行は列から消える）。
#[cfg(test)]
fn take_helper_log() -> Vec<String> {
    HELPER_LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

/// load-ack の 1 byte: 成功（DLL ロード＋3 解決＋`load`→true）。`MsgTag::Response` で厳密 1 byte 返す。
const LOAD_ACK_OK: u8 = 1;
/// load-ack の 1 byte: 失敗（DLL 不在・エクスポート欠落・`load`→false などあらゆる `ProxyError`）。
/// helper は生き続ける。
const LOAD_ACK_FAIL: u8 = 0;

/// REQUEST を駆動できなかったとき RESPONSE で返す、識別可能なエラーバイト列。
///
/// echo でも空でもなく error status の SHIORI/3.0 応答にして、host の codec が SHIORI エラーと
/// 読めるようにする。新しい `MsgTag` は足さない。
const REQUEST_ERROR_RESPONSE: &[u8] = b"SHIORI/3.0 500 Internal Server Error\r\n\r\n";

/// 応答（`MsgTag::Response`）の送出の失敗を、生の last error と一緒に行へ残す。
///
/// `SendMessageTimeoutW` の戻り 0 は `IpcError::SendFailed` に潰れて由来が見分けられない
/// （期限切れ＝last error 1460 は `IpcError::Timeout` で別に届く）。応答方向は `SMTO_ABORTIFHUNG` を
/// 付けないので、last error 0 の失敗はその旗以外の原因を疑う手がかりになる。
fn log_response_send_failure(what: &str, e: &IpcError) {
    // SAFETY: Win32 境界。直前の送出呼び出しが残したスレッド局所のエラー値を読むだけ。
    let last_error = unsafe { GetLastError() }.0;
    helper_log(&format!(
        "[helper] {what} 送出失敗（観測）: {e:?} / last_error={last_error}"
    ));
}

/// 自窓へ `PostMessageW(WM_NULL)` を置き、メッセージループを起こす。
/// 置けなければ `[helper]` の行で書いて偽を返す。
fn wake_self(hwnd: HWND) -> bool {
    // SAFETY: WM_NULL(0) は無害な起こし用メッセージ。失敗は Err で返る。
    match unsafe { PostMessageW(Some(hwnd), 0, WPARAM(0), LPARAM(0)) } {
        Ok(()) => true,
        Err(e) => {
            helper_log(&format!(
                "[helper] 自窓を起こせない（PostMessageW 失敗）: {e}"
            ));
            false
        }
    }
}

/// inbound WM_COPYDATA の framing 検証の結果に応じて WndProc が取る動作。
///
/// 実窓なしで単体検証できるよう、窓・FFI から切り離した [`classify_inbound`] が算出し、
/// 副作用（送出・カウンタ更新）は WndProc が行う。
#[derive(Debug, Clone, PartialEq, Eq)]
enum InboundAction {
    /// `Request` を受領。request バイト列を運ぶだけで、proxy の駆動は `handle_message` の `Reply` の腕が行う。
    Reply(Vec<u8>),
    /// `Load` を受領＝SHIORI ロードのトリガ。wire でパスを運ばない（パスは arg/env の `load_dir`）ので、
    /// ペイロードの有無を問わずこの動作になる。
    TriggerLoad,
    /// `Unload` を受領＝正規の正常終了経路のトリガ。ペイロードの有無を問わずこの動作になる。
    /// DLL の呼び出しの最中に来たときは ack `[1]` を先に返し、`close` 以降は呼び出しが戻ってから行う。
    TriggerUnload,
    /// framing 上は正当だが helper が応答しないタグ（`Hello`/`Response`）。crash させず記録のみ。
    IgnoreKnown(MsgTag),
    /// 未知タグ・長さ不整合などの不正フレーム。crash させず記録のみで、上位へ渡さない。
    IgnoreBad(FramingError),
}

/// inbound WM_COPYDATA の生値（tag 生値・宣言長・実データ）を [`InboundAction`] へ写像する純関数。
/// framing 検証は `shiori-host32-ipc::copydata_payload` に任せる。
fn classify_inbound(dw_data: usize, declared_len: usize, data: Vec<u8>) -> InboundAction {
    // `copydata_payload` は渡したバイト列をそのまま返すので、受け取った Vec を写さずに渡す。
    match copydata_payload(dw_data, declared_len, &data).map(|(tag, _)| tag) {
        Ok(MsgTag::Request) => InboundAction::Reply(data),
        Ok(MsgTag::Load) => InboundAction::TriggerLoad,
        Ok(MsgTag::Unload) => InboundAction::TriggerUnload,
        Ok(tag) => InboundAction::IgnoreKnown(tag),
        Err(err) => InboundAction::IgnoreBad(err),
    }
}

/// `ShioriByteProxy::load` の結果を load-ack 1 byte へ写す純関数。
/// 既確立のときの冪等 ack `[1]` は呼び手の WndProc が別に決める。
fn load_result_to_ack<T>(result: &Result<T, shiori_proxy::ProxyError>) -> u8 {
    match result {
        Ok(_) => LOAD_ACK_OK,
        Err(_) => LOAD_ACK_FAIL,
    }
}

/// WndProc と外側で共有する helper 窓の状態。
///
/// `Window<S>` が `state: S` を窓と同居させ WndProc へ `Pin<&S>` で渡すので、`Rc` も GWLP_USERDATA も
/// 要らない。single-in-flight・単一 UI スレッドなので内部可変は [`Cell`] で足りる。
struct HelperShared {
    /// 親のメッセージ窓 HWND（u32 ワイヤ値）。HELLO と RESPONSE の宛先。
    parent_hwnd: u32,
    /// ロード対象のリソース根（arg2/`HOST32_LOAD_DIR`）。DLL の場所と flat-C `load` の引数に使う。
    load_dir: PathBuf,
    /// SHIORI 名（arg3/`HOST32_SHIORI_NAME`）。descript.txt を解釈せずそのまま使う。
    shiori_name: String,
    /// 常設の proxy。確立後の再 LOAD は `load` を呼び直さず ack `[1]` を返す。
    /// 再入規律: この borrow を送出関数（`send_copydata` / `send_copydata_response`。ブロッキング SMTO で
    /// WndProc が再入しうる）越しに保持しない（`BorrowMutError` の panic を避ける）。
    proxy: RefCell<Option<ShioriByteProxy>>,
    /// 観測カウンタ: 送出した HELLO 数。
    hellos_sent: Cell<u64>,
    /// 観測カウンタ: 受領した REQUEST 数。
    requests_handled: Cell<u64>,
    /// 観測カウンタ: 返送した RESPONSE 数。
    responses_sent: Cell<u64>,
    /// 観測カウンタ: 不正フレーム／未知タグ受領数。
    bad_frames: Cell<u64>,
    /// LOAD 観測カウンタ: `TriggerLoad` 受領数（冪等の再受領も含む）。
    loads_attempted: Cell<u64>,
    /// LOAD 観測カウンタ: ack `[1]`（成功／冪等）送出数。
    load_acks_ok: Cell<u64>,
    /// LOAD 観測カウンタ: ack `[0]`（あらゆる `ProxyError` 失敗）送出数。
    load_acks_fail: Cell<u64>,
    /// 終了要求の印。UNLOAD の腕が立て、main のフィルタが読んで `msg_loop.quit()` する。
    quit_requested: Cell<bool>,
    /// UNLOAD 観測カウンタ: `TriggerUnload` 受領数。
    unloads_handled: Cell<u64>,
    /// DLL を呼んでいる最中か。立っている間の REQUEST は 500、UNLOAD は後回し
    /// （DLL を入れ子に呼ばず、`proxy` の借りも衝突させない）。
    in_dll_call: Cell<bool>,
    /// DLL の呼び出しの最中に UNLOAD が来た印。呼び出しが戻ったら遅れた応答を送らずに後始末する。
    unload_pending: Cell<bool>,
}

/// `in_dll_call` を立て、戻り方によらず下ろす見張りの値。
struct InDllCall<'a>(&'a Cell<bool>);

impl<'a> InDllCall<'a> {
    fn enter(flag: &'a Cell<bool>) -> Self {
        flag.set(true);
        Self(flag)
    }
}

impl Drop for InDllCall<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl HelperShared {
    fn new(parent_hwnd: u32, load_dir: PathBuf, shiori_name: String) -> Self {
        Self {
            parent_hwnd,
            load_dir,
            shiori_name,
            proxy: RefCell::new(None),
            hellos_sent: Cell::new(0),
            requests_handled: Cell::new(0),
            responses_sent: Cell::new(0),
            bad_frames: Cell::new(0),
            loads_attempted: Cell::new(0),
            load_acks_ok: Cell::new(0),
            load_acks_fail: Cell::new(0),
            quit_requested: Cell::new(false),
            unloads_handled: Cell::new(0),
            in_dll_call: Cell::new(false),
            unload_pending: Cell::new(false),
        }
    }
}

/// UNLOAD の後始末: proxy を `close`（unload → FreeLibrary）してから終了要求を立てる。`close` の失敗は
/// 行で書き、返事は [1] のまま。
/// `quit_requested` は後始末が済んでから立てる（DLL のモーダルな窓のループの中で回しを止めない）。
fn close_and_request_quit(s: &HelperShared) {
    let taken = s.proxy.borrow_mut().take();
    if let Some(Err(e)) = taken.map(ShioriByteProxy::close) {
        helper_log(&format!(
            "[helper] UNLOAD の後始末に失敗（観測・ack[1] のまま）: {e:?}"
        ));
    }
    s.quit_requested.set(true);
}

/// DLL の呼び出しが戻ったとき、最中に来た UNLOAD があれば後始末して真を返す（呼び手は遅れた応答を
/// 送らない・host はもう待っていない）。ack [1] は UNLOAD の受領のときに返し済み。
fn finish_deferred_unload(s: &HelperShared, self_hwnd: HWND, late: &str) -> bool {
    if !s.unload_pending.get() {
        return false;
    }
    helper_log(&format!(
        "[helper] DLL の呼び出しの最中に来た UNLOAD を後始末する（遅れた {late} は送らない）"
    ));
    close_and_request_quit(s);
    wake_self(self_hwnd);
    true
}

/// WndProc 本体。RESPONSE の宛先は起動時に決まった `parent_hwnd`、送信元は `self_hwnd`。
/// 1 通返したらそれ以上の跨プロセス `SendMessage` は発行しない（循環待ちを作らない）。
fn handle_message(s: &HelperShared, self_hwnd: HWND, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // None を返すと lib が DefWindowProc へ回す。
    }

    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS 契約）。
    let Some((dw_data, declared_len, payload)) = (unsafe { read_copydata(msg.lparam) }) else {
        s.bad_frames.set(s.bad_frames.get() + 1);
        helper_log("[helper] lparam が空の WM_COPYDATA を受領（無視）");
        return Some(LRESULT(0));
    };

    match classify_inbound(dw_data, declared_len, payload) {
        InboundAction::Reply(request_bytes) => {
            // 再入規律: `s.proxy` の borrow は同期の FFI `proxy.request` の間は持ってよいが、
            // RESPONSE 返送（ブロッキング SMTO で WndProc が再入しうる）へは持ち越さない。
            s.requests_handled.set(s.requests_handled.get() + 1);

            // DLL の呼び出しの最中（DLL が helper の窓へ送り返した）なら DLL を入れ子に呼ばず 500。
            let response_bytes: Vec<u8> = if s.in_dll_call.get() {
                helper_log(
                    "[helper] DLL の呼び出しの最中に REQUEST を受領（DLL を入れ子に呼ばず 500 返送）",
                );
                REQUEST_ERROR_RESPONSE.to_vec()
            } else {
                let guard = s.proxy.borrow();
                match guard.as_ref() {
                    Some(proxy) => {
                        let result = {
                            let _call = InDllCall::enter(&s.in_dll_call);
                            proxy.request(&request_bytes)
                        };
                        match result {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                helper_log(&format!(
                                    "[helper] REQUEST 駆動失敗（観測・500 返送）: {e:?}"
                                ));
                                REQUEST_ERROR_RESPONSE.to_vec()
                            }
                        }
                    }
                    None => {
                        // Load-before-Request は構造的な不変でここへは来ないが、来ても crash させない。
                        helper_log(
                            "[helper] proxy 未確立で REQUEST 受領（観測・500 返送）: Load-before-Request 不変違反",
                        );
                        REQUEST_ERROR_RESPONSE.to_vec()
                    }
                }
            }; // borrow はここで落ちる。以降の送出は borrow を持たない。
            if finish_deferred_unload(s, self_hwnd, "RESPONSE") {
                return Some(LRESULT(0));
            }

            // 応答方向なので `send_copydata_response`（ハング打ち切りの旗を付けず上限時間のみ）。
            let target = hwnd_from_u32(s.parent_hwnd);
            match send_copydata_response(
                target,
                self_hwnd,
                MsgTag::Response,
                &response_bytes,
                REPLY_TIMEOUT,
            ) {
                Ok(()) => s.responses_sent.set(s.responses_sent.get() + 1),
                Err(e) => log_response_send_failure("RESPONSE", &e),
            }
        }
        InboundAction::TriggerLoad => {
            // 再入規律: `s.proxy` の borrow は短命スコープに閉じ、ack 送出（ブロッキング SMTO で WndProc が
            // 再入しうる）の時には生かしておかない（再入 LOAD の `BorrowMutError` を避ける）。
            s.loads_attempted.set(s.loads_attempted.get() + 1);

            let already = s.proxy.borrow().is_some();

            let ack: u8 = if already {
                LOAD_ACK_OK
            } else {
                // 絶対 DLL パス（DLL 検索パスの曖昧さを避ける）。
                let dll_path = s.load_dir.join(&s.shiori_name);
                let result = {
                    let _call = InDllCall::enter(&s.in_dll_call);
                    ShioriByteProxy::load(&dll_path, &s.load_dir)
                };
                let ack = load_result_to_ack(&result);
                match result {
                    Ok(proxy) => {
                        *s.proxy.borrow_mut() = Some(proxy);
                    }
                    Err(e) => {
                        helper_log(&format!("[helper] LOAD 失敗（観測・ack[0]）: {e:?}"));
                    }
                }
                ack
            };
            // 最中に UNLOAD が来ていれば、読めた proxy をそのまま閉じ、遅れた ack は送らない。
            if finish_deferred_unload(s, self_hwnd, "load-ack") {
                return Some(LRESULT(0));
            }

            if ack == LOAD_ACK_OK {
                s.load_acks_ok.set(s.load_acks_ok.get() + 1);
            } else {
                s.load_acks_fail.set(s.load_acks_fail.get() + 1);
            }
            let target = hwnd_from_u32(s.parent_hwnd);
            // 応答方向なのでハング打ち切りの旗なし。送出失敗は行に残すだけ（親は timeout で検出する）。
            if let Err(e) =
                send_copydata_response(target, self_hwnd, MsgTag::Response, &[ack], REPLY_TIMEOUT)
            {
                log_response_send_failure("load-ack", &e);
            }
        }
        InboundAction::TriggerUnload => {
            s.unloads_handled.set(s.unloads_handled.get() + 1);
            // DLL の呼び出しの最中なら proxy に触れず（借りの衝突の元を無くす）印だけ立て、後始末は
            // 呼び出しが戻ってから。返事 [1] は先に返す（host は [1] の後に終わりを待つ）。
            let deferred = s.in_dll_call.get();
            if deferred {
                s.unload_pending.set(true);
                helper_log(
                    "[helper] DLL の呼び出しの最中に UNLOAD を受領（ack[1] を返し、後始末は呼び出しが戻ってから）",
                );
            } else {
                close_and_request_quit(s);
            }
            // ack[1]＝unload 完了・終了系列に入った。送出失敗は行に残すだけ（親は ack timeout で検出する）。
            let target = hwnd_from_u32(s.parent_hwnd);
            if let Err(e) = send_copydata_response(
                target,
                self_hwnd,
                MsgTag::Response,
                &[LOAD_ACK_OK],
                REPLY_TIMEOUT,
            ) {
                log_response_send_failure("unload-ack", &e);
            }
            // posted メッセージで MessageLoop を起こす（sent-message はフィルタに現れない）。
            // 後回しのときは終了要求がまだ無いので、後始末の側で起こす。
            if !deferred {
                wake_self(self_hwnd);
            }
        }
        InboundAction::IgnoreKnown(tag) => {
            helper_log(&format!("[helper] 応答対象外タグ受領（無視）: {tag:?}"));
        }
        InboundAction::IgnoreBad(err) => {
            s.bad_frames.set(s.bad_frames.get() + 1);
            helper_log(&format!("[helper] 不正フレーム受領（無視）: {err}"));
        }
    }
    Some(LRESULT(0))
}

/// message-only 窓を生成し、起動時に親へ HELLO を送る helper の窓。
struct HelperMessageWindow {
    /// Drop で `DestroyWindow`。
    window: Window<HelperShared>,
}

impl HelperMessageWindow {
    /// message-only 窓を生成し、親へ HELLO（自 HWND を u32 LE）を送る。
    fn create(parent_hwnd: u32, load_dir: PathBuf, shiori_name: String) -> Result<Self, String> {
        let shared = HelperShared::new(parent_hwnd, load_dir, shiori_name);
        // Fn（非 FnMut）なので `new` を使う。
        let window = Window::new(
            WindowType::MessageOnly,
            shared,
            move |state: Pin<&HelperShared>, msg: WindowMessage| -> Option<LRESULT> {
                let self_hwnd = msg.hwnd;
                handle_message(state.get_ref(), self_hwnd, &msg)
            },
        )
        .map_err(|e| format!("message-only 窓の生成に失敗: {e:?}"))?;

        let self_hwnd: HWND = window.hwnd();
        let hello_payload = encode_hwnd_le(self_hwnd);
        let target = hwnd_from_u32(parent_hwnd);
        send_copydata(
            target,
            self_hwnd,
            MsgTag::Hello,
            &hello_payload,
            REPLY_TIMEOUT,
        )
        .map_err(|e| format!("HELLO 送出に失敗: {e:?}"))?;
        window
            .state()
            .hellos_sent
            .set(window.state().hellos_sent.get() + 1);

        Ok(Self { window })
    }

    /// 終了要求の印（UNLOAD 受領で立つ）。main のループのフィルタが読む。
    fn quit_requested(&self) -> bool {
        self.window.state().quit_requested.get()
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

/// arg 優先・env fallback の純関数（`std::env` を直接読まないので単体テストできる）。
///
/// それぞれ `trim` して空でない最初の値を採り、両方空／不在なら `None`（呼び手が `exit(2)` にする）。
/// 値は arg/env からだけ取り、cwd からは推測しない。
fn resolve_param(arg: Option<String>, env: Option<String>) -> Option<String> {
    let pick = |v: Option<String>| -> Option<String> {
        v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };
    pick(arg).or_else(|| pick(env))
}

/// 親 HWND の u32 ワイヤ値を arg1 優先・env `HOST32_PARENT_HWND` fallback で取り、10 進で parse する
/// （parse 不能は `None`）。
fn parent_hwnd_arg_env() -> Option<u32> {
    resolve_param(
        std::env::args().nth(1),
        std::env::var("HOST32_PARENT_HWND").ok(),
    )
    .and_then(|s| s.parse::<u32>().ok())
}

/// `load_dir` を arg2 優先・env `HOST32_LOAD_DIR` fallback で取る。env キー名は host 側と厳密に一致させる。
fn load_dir_arg_env() -> Option<String> {
    resolve_param(
        std::env::args().nth(2),
        std::env::var("HOST32_LOAD_DIR").ok(),
    )
}

/// SHIORI 名を arg3 優先・env `HOST32_SHIORI_NAME` fallback で取る。env キー名は host 側と厳密に一致させる。
fn shiori_name_arg_env() -> Option<String> {
    resolve_param(
        std::env::args().nth(3),
        std::env::var("HOST32_SHIORI_NAME").ok(),
    )
}

/// 自分の標準入力・標準出力・標準エラーの取っ手から受け継ぎの印を落とす。
///
/// 親は helper の標準エラーを管にして読み手のスレッドで記録へ回す。SHIORI が取っ手を受け継ぐ形で
/// 起こしたプロセスが管の書き口を持ったまま残ると、ゴーストを閉じるたびに親の Drop が読み手を待って
/// 止まるため、その道をここで断つ。取っ手が無い・失敗は `[helper]` の行で書いて続ける。
fn disinherit_std_handles() {
    use windows::Win32::Foundation::{HANDLE_FLAG_INHERIT, HANDLE_FLAGS, SetHandleInformation};
    use windows::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    for (name, which) in [
        ("標準入力", STD_INPUT_HANDLE),
        ("標準出力", STD_OUTPUT_HANDLE),
        ("標準エラー", STD_ERROR_HANDLE),
    ] {
        // SAFETY: Win32 境界。自分の標準の取っ手を取るだけ。
        match unsafe { GetStdHandle(which) } {
            Ok(h) if !h.is_invalid() && !h.0.is_null() => {
                // SAFETY: Win32 境界。自分の取っ手の受け継ぎの印を落とすだけ。
                if let Err(e) =
                    unsafe { SetHandleInformation(h, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }
                {
                    helper_log(&format!(
                        "[helper] {name}の取っ手の受け継ぎの印を落とせない: {e}"
                    ));
                }
            }
            Ok(_) => helper_log(&format!(
                "[helper] {name}の取っ手が無い（受け継ぎの印は落とさない）"
            )),
            Err(e) => helper_log(&format!("[helper] {name}の取っ手を取れない: {e}")),
        }
    }
}

fn main() {
    disinherit_std_handles();
    let Some(parent_hwnd) = parent_hwnd_arg_env() else {
        helper_log(
            "[helper] 親 HWND（u32 ワイヤ値）が未指定です。arg1 または env HOST32_PARENT_HWND で渡してください。",
        );
        std::process::exit(2);
    };

    // 欠落は exit(2)。親は HELLO 不達＋プロセス終了で観測する。
    let Some(load_dir) = load_dir_arg_env() else {
        helper_log(
            "[helper] load_dir が未指定です。arg2 または env HOST32_LOAD_DIR で渡してください。",
        );
        std::process::exit(2);
    };
    let Some(shiori_name) = shiori_name_arg_env() else {
        helper_log(
            "[helper] SHIORI 名が未指定です。arg3 または env HOST32_SHIORI_NAME で渡してください。",
        );
        std::process::exit(2);
    };

    let load_dir = PathBuf::from(load_dir);
    let win = match HelperMessageWindow::create(parent_hwnd, load_dir, shiori_name) {
        Ok(w) => w,
        Err(e) => {
            helper_log(&format!("[helper] 窓生成/HELLO 失敗: {e}"));
            std::process::exit(2);
        }
    };

    // 終了は exit(0) で打ち切らず、UNLOAD の腕が quit_requested を立てて自窓を起こし、このフィルタが
    // `msg_loop.quit()` → ループが正常終了 → main が return、の道で行う。
    // proxy は UNLOAD の道で close 済み（ループを抜けるのはその後だけ）で、`win` の Drop が後始末するのは
    // UNLOAD を経ない道だけ。
    MessageLoop::run(|msg_loop, _msg| {
        if win.quit_requested() {
            msg_loop.quit();
        }
        FilterResult::Forward
    });
}

#[cfg(test)]
#[path = "main_resolve_param_tests.rs"]
mod resolve_param_tests;

#[cfg(test)]
#[path = "main_classify_tests.rs"]
mod classify_tests;

#[cfg(test)]
#[path = "main_load_ack_tests.rs"]
mod load_ack_tests;

#[cfg(test)]
#[path = "main_loopback_tests.rs"]
mod loopback_tests;

#[cfg(test)]
#[path = "main_helper_log_tests.rs"]
mod helper_log_tests;

// 約 42 秒の檻。判定は OS の振る舞いで CPU の種類に依らないため、helper を配る i686 の段でだけ走らせる
// （x64 のワークスペース全テストで同じ 42 秒を重ねて払わない）。
#[cfg(all(test, target_arch = "x86"))]
#[path = "main_response_flavor_hung_cage_tests.rs"]
mod response_flavor_hung_cage_tests;
