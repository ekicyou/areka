//! host-32 **i686 helper 実行バイナリ**（design.md §437-484・HelperMessageWindow）。
//!
//! `wintf-winmsg-executor` 0.0.5 の **message-only 窓** ＋ `MessageLoop::run` を i686 で回す
//! transport の helper 側。責務は 3 点に限定される（design.md §446-452・要件 3.1 / 4.2 / 6.1 / 7.1）:
//!
//! 1. **起動時 HELLO**（要件 3.1）: 窓生成後、親へ自 HWND を u32 LE で 1st WM_COPYDATA 送出。
//! 2. **REQUEST → proxy 駆動 → 即 return**（要件 3.1 / 4.7 / 4.8）: WndProc で inbound WM_COPYDATA を
//!    framing 検証し、`Request` なら受領した **request バイト列**を確立済み proxy の
//!    [`ShioriByteProxy::request`] へ流し、その結果（SHIORI/3.0 応答）を `Response` として親へ
//!    **1 通だけ** 返送して即 return する（それ以上の跨プロセス `SendMessage` を発行しない）。
//!    proxy 未確立なら明示エラーバイト列（`SHIORI/3.0 500`）を返送し crash させない（R3.1）。
//!    不正フレーム／未知タグは crash させず観測カウンタに記録するのみ（要件 2.5・上位へ渡さない）。
//! 3. **`main`**（要件 7.1）: 親 HWND を arg/env の u32 ワイヤ値で取得 → 窓生成 → HELLO 送出 →
//!    `MessageLoop::run`。lifecycle は request 往復実証に必要な最小に留める（常駐 lifecycle は
//!    Out of Boundary・下流 `host32-lifecycle`）。
//!
//! ## proxy 駆動の差し替え点（design.md §436 / §465-466）
//! REQUEST は `classify_inbound`（純関数・proxy 非依存）で **request バイト列**として `Reply` に
//! 載せられ、`handle_message` の `Reply` アーム（proxy へ到達できる唯一の点）が確立済み proxy の
//! [`ShioriByteProxy::request`] を実呼出して SHIORI/3.0 応答を得る。凍結する seam は WM_COPYDATA の
//! REQUEST/RESPONSE ワイヤ形式であって proxy 実装ではない（**trait 抽象は設けない**・YAGNI・design.md §482）。
//!
//! ## デッドロック回避（design.md §200 / §473）
//! WndProc は REQUEST に対し RESPONSE を 1 通返すだけで即 return する。往復は single-in-flight
//! で厳密にネストし、循環待ちが構造的に発生しない（要件 4.4）。
//!
//! ## 依存方向（design.md §119-127）
//! 本クレートは `shiori-host32-ipc`（proto）を一方向依存し、`shiori-host32-host` へは依存しない
//! （host↔helper のコード依存は無く、プロセス境界で WM_COPYDATA のみ）。

// ShioriByteProxy（unsafe FFI 一点集約・design §339）。Task 6 が WndProc の LOAD 結線から
// consume するまで未使用ゆえ、モジュール側で dead_code を抑止している。宣言のみ（Task 5.1 の領分）。
mod shiori_proxy;

use shiori_proxy::ShioriByteProxy;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
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
/// 無限待機の構造的排除（要件 5.3）。RESPONSE 返送は同期送出で即完了するため短めで足りるが、
/// ハング peer でも上限時間で復帰することを保証する。
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// load-ack の 1 byte 契約（design.md §330・要件 5.1）: 成功。
///
/// `MsgTag::Response` として親へ返す**厳密 1 byte**。`[0x01]`=DLL ロード＋3 解決＋`load`→true の成功。
const LOAD_ACK_OK: u8 = 1;
/// load-ack の 1 byte 契約（design.md §330・要件 5.1）: 失敗。
///
/// `[0x00]`=あらゆる `ProxyError`（DLL 不在・エクスポート欠落・`load`→false 等）。helper は生存継続（R6.4）。
const LOAD_ACK_FAIL: u8 = 0;

/// REQUEST 駆動失敗時に RESPONSE 返送する**識別可能なエラーバイト列**（design.md §436・R3.1）。
///
/// proxy 未確立（構造上起きないが crash 回避の最小防御）／`proxy.request` の `Err` 時に、echo でも
/// 空でもなく **error status の SHIORI/3.0 応答**を返す。host 側 codec がこれを識別可能な SHIORI
/// エラーへ解釈できる（research §7.4「空 or エラー status バイト列」許容）。新 `MsgTag` は導入しない。
const REQUEST_ERROR_RESPONSE: &[u8] = b"SHIORI/3.0 500 Internal Server Error\r\n\r\n";

/// inbound WM_COPYDATA の framing 検証結果に応じた WndProc の取るべき動作（窓なしで単体検証可）。
///
/// 窓・FFI から切り離した純ロジックとして [`classify_inbound`] が算出し、WndProc はこの結果を
/// 見て副作用（送出・カウンタ更新）を行う。これにより REQUEST 分岐・不正フレーム分類を
/// 実窓なしで決定的に単体検証できる。
#[derive(Debug, Clone, PartialEq, Eq)]
enum InboundAction {
    /// `Request` を受領。同梱の **request バイト列**（echo ではない）を `handle_message` の `Reply`
    /// アームが確立済み proxy の [`ShioriByteProxy::request`] へ流し、その SHIORI/3.0 応答を `Response`
    /// として親へ返送すべき。純関数 [`classify_inbound`] は proxy へ到達しないため、ここでは駆動対象の
    /// request バイトを運ぶだけに留める（proxy 駆動は proxy へ到達できる `Reply` アームの領分）。
    Reply(Vec<u8>),
    /// `Load` を受領＝SHIORI ロード実行のトリガ（要件 4.1）。従来の「既知だが無視」を置換する。
    /// **ペイロードを持たない**（wire でパスを運ばない・パスは arg/env の `load_dir` から得る）ため、
    /// 受領フレームのペイロード有無を問わずこの動作へ写像される。実際のロード結線（proxy 確立・
    /// `load` 呼出・ack 返送）は下流タスク（Task 6・WndProc の LOAD 結線）が担う。
    TriggerLoad,
    /// framing 上は正当だが本 helper が応答しないタグ（`Hello`/`Response`/`Unload`）。
    /// crash させず無視する（記録のみ）。`Load` は `TriggerLoad` へ分離済みゆえ含まない。
    IgnoreKnown(MsgTag),
    /// 未知タグ・長さ不整合など不正フレーム。crash させず記録のみ・上位へ渡さない（要件 2.5）。
    IgnoreBad(FramingError),
}

/// inbound WM_COPYDATA の生値（tag 生値・宣言長・実データ）を [`InboundAction`] へ写像する純関数。
///
/// framing 検証は `shiori-host32-ipc::copydata_payload` に委譲（重複実装しない）。`Request` は受領した
/// **request バイト列**をそのまま `Reply` に載せる（echo でなく proxy 駆動対象・実駆動は `handle_message`
/// の `Reply` アーム）。その他の既知タグは `IgnoreKnown`、不正フレームは `IgnoreBad` とする。副作用
/// （送出・カウンタ・proxy 到達）は持たず、WndProc 側が結果を見て実行する（純粋・単体テスト可能性を保つ）。
fn classify_inbound(dw_data: usize, declared_len: usize, data: &[u8]) -> InboundAction {
    match copydata_payload(dw_data, declared_len, data) {
        Ok((MsgTag::Request, payload)) => InboundAction::Reply(payload.to_vec()),
        // Load はロード実行トリガ（要件 4.1）。ペイロードは無視（有無を問わず TriggerLoad）。
        // IgnoreKnown の一般アームより先に分離して受ける。
        Ok((MsgTag::Load, _)) => InboundAction::TriggerLoad,
        Ok((tag, _)) => InboundAction::IgnoreKnown(tag),
        Err(err) => InboundAction::IgnoreBad(err),
    }
}

/// `ShioriByteProxy::load` の `Result` を load-ack 1 byte へ写像する純関数（副作用なし・単体検証可）。
///
/// `Ok(_)` → [`LOAD_ACK_OK`]（`[0x01]`）／`Err(ProxyError)` → [`LOAD_ACK_FAIL`]（`[0x00]`）。
/// 既確立の冪等 ack `[1]`（`load` 再呼出なし）は呼出側 WndProc が別経路で決めるため、ここは
/// 「新規確立試行の結果 → ack」の写像に徹する（design.md §321・要件 5.1/6.4）。
fn load_result_to_ack<T>(result: &Result<T, shiori_proxy::ProxyError>) -> u8 {
    match result {
        Ok(_) => LOAD_ACK_OK,
        Err(_) => LOAD_ACK_FAIL,
    }
}

/// WndProc と外側で共有する helper 窓の状態（design.md §475-478）。
///
/// `wintf-winmsg-executor` の `Window<S>` は `state: S` を窓と同居させ、WndProc へ `Pin<&S>` で
/// 渡す（`Rc` 不要・GWLP_USERDATA 手詰め不要）。single-in-flight・単一 UI スレッドゆえ内部可変は
/// [`Cell`] で足りる。lifecycle 状態機械は request 往復実証に必要な最小（観測カウンタのみ）に留める。
struct HelperShared {
    /// 親のメッセージ窓 HWND（u32 ワイヤ値）。HELLO 送出先＆RESPONSE 返送先。
    parent_hwnd: u32,
    /// ロード対象のリソース根（arg2/`HOST32_LOAD_DIR` 由来・R3.4）。LOAD トリガ結線で
    /// `load_dir.join(shiori_name)` の DLL 特定と flat-C `load` 引数（リソース根）に用いる。
    /// `PathBuf` で保持し join を型安全に行う（design.md §320）。
    load_dir: PathBuf,
    /// 受領した SHIORI 名（arg3/`HOST32_SHIORI_NAME` 由来）。そのまま使用し descript.txt を解釈しない
    /// （R3.6）。`load_dir.join(&shiori_name)` で絶対 DLL パスを組む。
    shiori_name: String,
    /// **常設プロキシ保持スロット**（design.md §320・WndProc の LOAD 結線）。確立成功時に
    /// `Some(proxy)` を常設保持し、以後の再 LOAD は `load` 再呼出なしで ack `[1]` 冪等返送する（R2.4）。
    /// 非 `Copy` ゆえ `Cell` 不可・single UI thread 前提で `RefCell` で足りる。**再入規律**（validation
    /// issue #1）: この `RefCell` の borrow を `send_copydata`（ブロッキング SMTO・再入可）越しに保持
    /// しない（`BorrowMutError` panic の構造的排除・R6.4）。
    proxy: RefCell<Option<ShioriByteProxy>>,
    /// 観測カウンタ: 送出した HELLO 数。
    hellos_sent: Cell<u64>,
    /// 観測カウンタ: 受領した REQUEST 数。
    requests_handled: Cell<u64>,
    /// 観測カウンタ: 返送した RESPONSE 数。
    responses_sent: Cell<u64>,
    /// 観測カウンタ: 不正フレーム／未知タグ受領数（crash させず記録・要件 2.5）。
    bad_frames: Cell<u64>,
    /// LOAD 観測カウンタ: `TriggerLoad` 受領数（冪等再受領も含む・design.md §320）。
    loads_attempted: Cell<u64>,
    /// LOAD 観測カウンタ: ack `[1]`（成功／冪等）送出数。
    load_acks_ok: Cell<u64>,
    /// LOAD 観測カウンタ: ack `[0]`（あらゆる `ProxyError` 失敗）送出数。
    load_acks_fail: Cell<u64>,
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

/// WndProc 本体: WM_COPYDATA を [`classify_inbound`] で分類し、`Reply` なら request バイト列を
/// 確立済み proxy の [`ShioriByteProxy::request`] へ流し、その SHIORI/3.0 応答を RESPONSE として
/// 親へ 1 通返送して即 return する（design.md §436・要件 3.1 / 4.7 / 4.8）。
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
        InboundAction::Reply(request_bytes) => {
            // REQUEST 駆動（design.md §436・要件 3.1/4.7/4.8）。`classify_inbound` は純関数ゆえ proxy へ
            // 到達しない。proxy に到達できるのはこの `Reply` アームのみ＝ここで確立済み proxy を実駆動する。
            //
            // **RefCell 再入規律（validation issue #1・LOAD アームと同型）**: `s.proxy` の borrow を
            // FFI `proxy.request` 呼出中に保持することは可（FFI は同期・跨プロセス SendMessage を発しない）
            // だが、その後の RESPONSE 返送（`send_copydata`・ブロッキング SMTO・WndProc 再入を許す）へ
            // borrow を持ち越さない。応答バイトを scoped borrow 内で確定し borrow を drop してから送出する。
            // 再入 REQUEST が borrow を掴んでも `BorrowError` panic を起こさない（R6.4）。
            s.requests_handled.set(s.requests_handled.get() + 1);

            // 1) scoped borrow で proxy を実駆動し response_bytes を確定（borrow は本ブロックで即終了）。
            //    proxy 未確立（構造上起きないが crash 回避の最小防御・R3.1）／`request` の Err は
            //    識別可能なエラーバイト列（500）へ写像し、観測ログを残す（log-first・silent swallow 禁止）。
            let response_bytes: Vec<u8> = {
                let guard = s.proxy.borrow();
                match guard.as_ref() {
                    Some(proxy) => match proxy.request(&request_bytes) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            eprintln!("[helper] REQUEST 駆動失敗（観測・500 返送）: {e:?}");
                            REQUEST_ERROR_RESPONSE.to_vec()
                        }
                    },
                    None => {
                        // Load-before-Request は構造的不変（設計上ここへは来ない）。来ても crash させず
                        // 識別可能なエラーバイト列を返し helper は生存継続（R3.1/R6.4）。
                        eprintln!(
                            "[helper] proxy 未確立で REQUEST 受領（観測・500 返送）: Load-before-Request 不変違反"
                        );
                        REQUEST_ERROR_RESPONSE.to_vec()
                    }
                }
            }; // ← ここで borrow が drop され、以後 RESPONSE 送出は borrow 非保持で行う。

            // 2) borrow を一切保持しない状態で RESPONSE を親へ 1 通だけ返す（ブロッキング SMTO・再入可・§200）→
            //    即 return（それ以上の跨プロセス SendMessage 不可）。
            let target = hwnd_from_u32(s.parent_hwnd);
            match send_copydata(target, self_hwnd, MsgTag::Response, &response_bytes, REPLY_TIMEOUT) {
                Ok(()) => s.responses_sent.set(s.responses_sent.get() + 1),
                Err(e) => eprintln!("[helper] RESPONSE 送出失敗（観測）: {e:?}"),
            }
        }
        InboundAction::TriggerLoad => {
            // LOAD 実結線（design.md §321・要件 4.1/4.3/5.1/6.4）。
            //
            // **RefCell 再入規律（validation issue #1・最重要）**: `s.proxy` の borrow/borrow_mut を
            // `send_copydata`（ブロッキング SMTO・WndProc 再入を許す）越しに保持しない。
            // 「既確立判定 → 未確立なら確立して即 borrow_mut を閉じる → borrow 非保持で ack 送出」の
            // 順序を固定し、再入 LOAD の borrow_mut 衝突（BorrowMutError panic＝R6.4 違反）を構造的に
            // 排除する。以下、borrow は全て短命スコープに閉じ、ack 送出時には生きていない。
            s.loads_attempted.set(s.loads_attempted.get() + 1);

            // 1) 既確立判定（scoped borrow・即 drop）。
            let already = s.proxy.borrow().is_some();

            // 2) 未確立なら proxy 確立（この match 自体は RefCell に触れない）。
            let ack: u8 = if already {
                // 冪等: load 再呼出なし・ack[1]（R2.4）。
                LOAD_ACK_OK
            } else {
                // 絶対 DLL パス = load_dir\<shiori_name>（DLL 検索パス曖昧性の排除・design.md §321）。
                let dll_path = s.load_dir.join(&s.shiori_name);
                let result = ShioriByteProxy::load(&dll_path, &s.load_dir);
                let ack = load_result_to_ack(&result);
                match result {
                    Ok(proxy) => {
                        // 3) 成功: scoped borrow_mut で常設保持（borrow は本ブロックで即終了）。
                        *s.proxy.borrow_mut() = Some(proxy);
                    }
                    Err(e) => {
                        // 失敗は観測ログのみ・helper は生存継続（exit/panic しない・R6.4）。
                        eprintln!("[helper] LOAD 失敗（観測・ack[0]）: {e:?}");
                    }
                }
                ack
            };

            // 4) borrow を一切保持しない状態で ack 送出（ブロッキング SMTO・再入可・R5.2）。
            if ack == LOAD_ACK_OK {
                s.load_acks_ok.set(s.load_acks_ok.get() + 1);
            } else {
                s.load_acks_fail.set(s.load_acks_fail.get() + 1);
            }
            let target = hwnd_from_u32(s.parent_hwnd);
            // ack は既存 Response 再入経路で 1 通返送。送出失敗は観測ログのみ（親は timeout で検出）。
            if let Err(e) =
                send_copydata(target, self_hwnd, MsgTag::Response, &[ack], REPLY_TIMEOUT)
            {
                eprintln!("[helper] load-ack 送出失敗（観測）: {e:?}");
            }
        }
        InboundAction::IgnoreKnown(tag) => {
            // Hello/Response/Unload は helper が能動応答しない。記録のみ（無応答）。
            // Load は TriggerLoad へ分離済みゆえここには来ない。
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
    /// `load_dir`（`PathBuf`）／`shiori_name` は arg/env 由来のロード対象パラメーター（R3.4）で
    /// `HelperShared` へ保持し、WndProc の LOAD 結線が `load_dir.join(&shiori_name)` で消費する。
    fn create(parent_hwnd: u32, load_dir: PathBuf, shiori_name: String) -> Result<Self, String> {
        let shared = HelperShared::new(parent_hwnd, load_dir, shiori_name);
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

/// arg-n 優先・env fallback の**決定的な純関数**（design.md §322・R3.4/3.5）。
///
/// arg 値・env 値を引数として受け取り（`std::env` を直接読まない＝単体テスト可能）、それぞれ
/// `trim` した上で**空でない最初の値**を採用する。arg が空文字／空白のみなら env へ委ね、両者とも
/// 空／不在なら `None` を返す（呼出側が必須パラメーター欠落として起動失敗＝`exit(2)` に用いる）。
/// 採用値は `trim` 済み。`parent_hwnd`／`load_dir`／`shiori_name` の 3 適用で共用する。
/// **値は arg/env から取得し cwd からは推測しない**（R3.4）。
fn resolve_param(arg: Option<String>, env: Option<String>) -> Option<String> {
    let pick = |v: Option<String>| -> Option<String> {
        v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };
    pick(arg).or_else(|| pick(env))
}

/// 実 args/env を読む薄いラッパ（`resolve_param` へ委譲）。
///
/// 親 HWND の u32 ワイヤ値を arg（第 1 引数・index 1）優先・env（`HOST32_PARENT_HWND`）fallback で
/// 取得し、10 進 u32 へ parse する。値の取得規約は [`resolve_param`] と共通（arg 優先・空 trim）。
/// parse 不能なら `None`（呼出側が起動失敗として扱う）。
fn parent_hwnd_arg_env() -> Option<u32> {
    resolve_param(std::env::args().nth(1), std::env::var("HOST32_PARENT_HWND").ok())
        .and_then(|s| s.parse::<u32>().ok())
}

/// 実 args/env を読む薄いラッパ（`resolve_param` へ委譲）。
///
/// `load_dir` を arg（第 2 引数・index 2）優先・env（`HOST32_LOAD_DIR`）fallback で取得する。
/// env キー名は host 側（Task 2）が確定した `HOST32_LOAD_DIR` と厳密一致させる。
/// **値は arg/env から取得し cwd から推測しない**（R3.4）。
fn load_dir_arg_env() -> Option<String> {
    resolve_param(std::env::args().nth(2), std::env::var("HOST32_LOAD_DIR").ok())
}

/// 実 args/env を読む薄いラッパ（`resolve_param` へ委譲）。
///
/// SHIORI 名を arg（第 3 引数・index 3）優先・env（`HOST32_SHIORI_NAME`）fallback で取得する。
/// env キー名は host 側（Task 2）が確定した `HOST32_SHIORI_NAME` と厳密一致させる。
/// helper は受領した SHIORI 名をそのまま使用し、descript.txt を自ら解釈しない（R3.6）。
fn shiori_name_arg_env() -> Option<String> {
    resolve_param(std::env::args().nth(3), std::env::var("HOST32_SHIORI_NAME").ok())
}

fn main() {
    let Some(parent_hwnd) = parent_hwnd_arg_env() else {
        eprintln!(
            "[helper] 親 HWND（u32 ワイヤ値）が未指定です。arg1 または env HOST32_PARENT_HWND で渡してください。"
        );
        std::process::exit(2);
    };

    // load_dir・SHIORI 名は arg/env から取得（cwd 推測禁止・R3.4）。欠落は決定的な起動失敗＝
    // exit(2)（R3.5・parent_hwnd 前例と同型）。親は HELLO 不達＋プロセス終了で決定的に観測する。
    let Some(load_dir) = load_dir_arg_env() else {
        eprintln!(
            "[helper] load_dir が未指定です。arg2 または env HOST32_LOAD_DIR で渡してください。"
        );
        std::process::exit(2);
    };
    let Some(shiori_name) = shiori_name_arg_env() else {
        eprintln!(
            "[helper] SHIORI 名が未指定です。arg3 または env HOST32_SHIORI_NAME で渡してください。"
        );
        std::process::exit(2);
    };

    // load_dir は arg/env の String を PathBuf へ整合（LOAD 結線の join を型安全化・design.md §320）。
    let load_dir = PathBuf::from(load_dir);
    let win = match HelperMessageWindow::create(parent_hwnd, load_dir, shiori_name) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[helper] 窓生成/HELLO 失敗: {e}");
            std::process::exit(2);
        }
    };

    // REQUEST を受領して proxy 駆動 RESPONSE を返すため、メッセージループを回す（要件 3.1 / 4.7 / 4.8）。
    // 常駐 lifecycle（UNLOAD 停止等）は Out of Boundary（下流 host32-lifecycle）ゆえ、
    // 本ユニットは request 往復実証に必要な最小ループとし、終了条件は下流が結線する。
    // `win` は本スコープで生存し続け、Drop（窓破棄）は main 終了時。
    let _keep_alive = &win;
    MessageLoop::run(|_msg_loop, _msg| FilterResult::Forward);
}

#[cfg(test)]
mod resolve_param_tests {
    use super::*;

    // R3.4: arg 優先。arg=Some, env=Some → arg（parent_hwnd/load_dir/shiori_name 3 適用の共通観点）。
    #[test]
    fn arg_takes_priority_over_env() {
        let got = resolve_param(Some("C:\\ghost\\master".to_string()), Some("C:\\env".to_string()));
        assert_eq!(got.as_deref(), Some("C:\\ghost\\master"));
    }

    // R3.4: env fallback。arg=None → env（load_dir/shiori_name が env のみ供給された経路）。
    #[test]
    fn env_used_when_arg_absent() {
        let got = resolve_param(None, Some("shiori.dll".to_string()));
        assert_eq!(got.as_deref(), Some("shiori.dll"));
    }

    // R3.4: arg が空文字/空白のみ → env へ（trim 後に空でない最初の値を採る・parent_hwnd と同型）。
    #[test]
    fn blank_arg_falls_through_to_env() {
        assert_eq!(
            resolve_param(Some("   ".to_string()), Some("fallback".to_string())).as_deref(),
            Some("fallback")
        );
        assert_eq!(
            resolve_param(Some(String::new()), Some("fallback".to_string())).as_deref(),
            Some("fallback")
        );
    }

    // R3.4: arg も env も空白のみ → None（両欠落と等価に扱う）。
    #[test]
    fn blank_arg_and_blank_env_yield_none() {
        assert_eq!(resolve_param(Some("  ".to_string()), Some("\t".to_string())), None);
    }

    // R3.5: arg=None, env=None → None（必須パラメーター欠落＝呼出側が exit(2) の判定に用いる）。
    #[test]
    fn both_absent_yield_none() {
        assert_eq!(resolve_param(None, None), None);
    }

    // 採用値は trim される（cwd 推測でなく arg/env 由来の値を正規化して返す）。
    #[test]
    fn adopted_value_is_trimmed() {
        assert_eq!(
            resolve_param(Some("  C:\\ghost\\master  ".to_string()), None).as_deref(),
            Some("C:\\ghost\\master")
        );
    }
}

#[cfg(test)]
mod classify_tests {
    use super::*;

    // 要件 3.1 / 4.7: REQUEST は受領 request バイト列（echo ではなく proxy 駆動対象）を伴う Reply
    // へ分類される（純関数ゆえ proxy へ到達しない・実駆動は handle_message の Reply アーム）。
    // バイト内容は受領 payload と同一だが、意味は「駆動すべき request」であって「応答（echo）」ではない。
    #[test]
    fn request_classifies_to_reply_with_request_bytes() {
        let payload = b"round-trip";
        let raw = MsgTag::Request.as_u32() as usize;
        let action = classify_inbound(raw, payload.len(), payload);
        assert_eq!(action, InboundAction::Reply(payload.to_vec()));
    }

    #[test]
    fn request_with_empty_payload_carries_empty_request_bytes() {
        let raw = MsgTag::Request.as_u32() as usize;
        let action = classify_inbound(raw, 0, b"");
        assert_eq!(action, InboundAction::Reply(Vec::new()));
    }

    // 要件 4.2: 応答対象外の既知タグは IgnoreKnown（無応答・crash なし）。
    // 注: Load は本タスク（3.2）で TriggerLoad へ分離したため、ここから除外する（R4.1）。
    #[test]
    fn known_nonrequest_tags_are_ignored() {
        for tag in [MsgTag::Hello, MsgTag::Response, MsgTag::Unload] {
            let raw = tag.as_u32() as usize;
            let action = classify_inbound(raw, 0, b"");
            assert_eq!(action, InboundAction::IgnoreKnown(tag));
        }
    }

    // 要件 4.1: Load 受領はロード実行トリガ（TriggerLoad）へ分類される。従来の
    // 「既知だが無視（IgnoreKnown）」を置換する。ペイロードにパスを期待しない
    // ゆえ、ペイロード有無を問わず TriggerLoad であること。
    #[test]
    fn load_classifies_to_trigger_load() {
        let raw = MsgTag::Load.as_u32() as usize;

        // ペイロード無し。
        let action = classify_inbound(raw, 0, b"");
        assert_eq!(action, InboundAction::TriggerLoad);

        // ペイロード有り（wire でパスを運ばないため、内容は無視され同じく TriggerLoad）。
        let payload = b"C:\\ghost\\master\\ignored-path";
        let action = classify_inbound(raw, payload.len(), payload);
        assert_eq!(action, InboundAction::TriggerLoad);
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
mod load_ack_tests {
    use super::*;
    use shiori_proxy::ProxyError;

    // 要件 5.1/6.4: 新規確立成功（Ok）→ ack[1]。
    #[test]
    fn ok_maps_to_ack_one() {
        let r: Result<(), ProxyError> = Ok(());
        assert_eq!(load_result_to_ack(&r), LOAD_ACK_OK);
        assert_eq!(LOAD_ACK_OK, 1);
    }

    // 要件 6.1/6.4: あらゆる ProxyError → ack[0]（DLL 不在・エクスポート欠落・load→false 等）。
    #[test]
    fn every_proxy_error_maps_to_ack_zero() {
        let errors: [ProxyError; 3] = [
            ProxyError::EntryNotFound("load"),
            ProxyError::EncodingFailed,
            ProxyError::LoadReturnedFalse,
        ];
        for e in errors {
            let r: Result<(), ProxyError> = Err(e);
            assert_eq!(load_result_to_ack(&r), LOAD_ACK_FAIL);
        }
        assert_eq!(LOAD_ACK_FAIL, 0);
    }
}

#[cfg(test)]
mod loopback_tests {
    use super::*;
    use std::path::PathBuf;
    use std::rc::Rc;
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

    /// testdll の `shiori.dll` を解決する（proxy 単体テストと同型・Task 5.2）。
    /// 優先順: env `HOST32_TESTDLL_DLL` → `CARGO_MANIFEST_DIR` から target 探索。
    /// **silent skip 禁止**: 見つからなければ明確に panic。
    fn resolve_testdll() -> PathBuf {
        if let Ok(p) = std::env::var("HOST32_TESTDLL_DLL") {
            let path = PathBuf::from(p);
            assert!(
                path.is_file(),
                "HOST32_TESTDLL_DLL={} が実ファイルでない",
                path.display()
            );
            return path;
        }
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_root = manifest.join("../../target/i686-pc-windows-msvc");
        for profile in ["debug", "release"] {
            let candidate = target_root.join(profile).join("shiori.dll");
            if candidate.is_file() {
                return candidate;
            }
        }
        panic!(
            "testdll shiori.dll not found. まず PowerShell で \
             `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc` を実行するか、\
             env HOST32_TESTDLL_DLL に絶対パスを設定すること（silent skip 禁止）。探索した base: {}",
            target_root.display()
        );
    }

    /// 単一 loopback テスト（HELLO 送出・未確立 REQUEST→500・LOAD・proxy 駆動 REQUEST(GET/NOTIFY)・
    /// 不正フレーム記録・bounded 生存を同一窓で網羅）。
    ///
    /// wintf の message-only 窓を i686 テストプロセスで 2 組独立生成すると 2 組目が
    /// WindowCreationError になる既知制約ゆえ、窓は 1 組（stand-in parent ＋ helper）に集約する。
    /// bounded: 有限メッセージを PostMessage で撒いた後に必ず抜ける（無限ループ禁止）。
    #[test]
    #[cfg_attr(
        not(target_arch = "x86"),
        ignore = "i686 専用: 32bit testdll(shiori.dll) を load するため x64 では BAD_EXE_FORMAT。`cargo test -p shiori-host32-helper --target i686-pc-windows-msvc` で実行"
    )]
    fn loopback_hello_request_proxy_driven_and_bounded_loop() {
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
        // LOAD 経路検証のため、testdll(shiori.dll) を入れた一時 load_dir で helper を構築する。
        // dll_path = load_dir\shiori.dll（絶対）。testdll 解決は silent skip 禁止（resolve_testdll が panic）。
        let src_dll = resolve_testdll();
        let unique = format!(
            "host32_helper_loopback_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let load_dir = std::env::temp_dir().join(&unique);
        std::fs::create_dir_all(&load_dir).expect("create temp load_dir");
        std::fs::copy(&src_dll, load_dir.join("shiori.dll")).expect("copy shiori.dll into load_dir");

        let helper = HelperMessageWindow::create(
            parent_hwnd_u32,
            load_dir.clone(),
            "shiori.dll".to_string(),
        )
        .expect("HelperMessageWindow 生成に失敗");
        let helper_hwnd_u32 = u32::from_le_bytes(encode_hwnd_le(helper.hwnd()));

        // HELLO 送出経路が動いた: 親が helper HWND を復号一致で受領（要件 3.1）。
        assert_eq!(
            hello_helper_hwnd.get(),
            Some(helper_hwnd_u32),
            "親が HELLO を復号一致で受領する（要件 3.1）"
        );
        assert_eq!(helper.shared().hellos_sent.get(), 1);

        // --- 未確立 REQUEST: proxy 未確立（LOAD 前）で REQUEST を送っても crash せず、識別可能な
        //     500 エラー RESPONSE を親が受領する（echo ではない・R3.1）---
        // Load-before-Request は構造的不変だが、helper は不変違反でも crash しない最小防御を持つ。
        let pre_load_req =
            b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
        send_copydata(
            helper.hwnd(),
            parent.hwnd(),
            MsgTag::Request,
            pre_load_req,
            REPLY_TIMEOUT,
        )
        .expect("未確立 REQUEST 送出に失敗");

        assert_eq!(
            helper.shared().requests_handled.get(),
            1,
            "WndProc が未確立 REQUEST も処理する（crash しない・R3.1）"
        );
        assert_eq!(
            helper.shared().responses_sent.get(),
            1,
            "未確立 REQUEST にも RESPONSE を 1 通返す（無応答でない・R3.1）"
        );
        assert_eq!(responses.get(), 1, "親が未確立 REQUEST の RESPONSE を受領する");
        {
            let text = String::from_utf8(last_response.borrow().clone())
                .expect("500 RESPONSE は UTF-8");
            assert!(
                text.contains("SHIORI/3.0 500 Internal Server Error"),
                "未確立 proxy では識別可能な 500 を返す（echo ではない・R3.1）: {text:?}"
            );
            assert_ne!(
                last_response.borrow().as_slice(),
                &pre_load_req[..],
                "RESPONSE は request の echo ではない（proxy 駆動結果／エラー）"
            );
        }

        // --- LOAD 経路: MsgTag::Load（空ペイロード）→ 未確立ゆえ proxy 確立 → ack[1]（要件 4.1/5.1）---
        // 親が受領した最新 RESPONSE を LOAD ack として観測する（load_dir\shiori.dll=testdll を確立）。
        send_copydata(
            helper.hwnd(),
            parent.hwnd(),
            MsgTag::Load,
            &[],
            REPLY_TIMEOUT,
        )
        .expect("LOAD 送出に失敗");

        assert_eq!(
            helper.shared().loads_attempted.get(),
            1,
            "WndProc が LOAD トリガを受領する（要件 4.1）"
        );
        assert_eq!(
            helper.shared().load_acks_ok.get(),
            1,
            "初回 LOAD は proxy 確立成功で ack[1] を送出する（要件 5.1/6.4）"
        );
        assert_eq!(
            helper.shared().load_acks_fail.get(),
            0,
            "成功 LOAD で ack[0] は送出しない"
        );
        assert!(
            helper.shared().proxy.borrow().is_some(),
            "確立成功した proxy が常設保持される（要件 4.3）"
        );
        assert_eq!(responses.get(), 2, "親が LOAD ack を 1 通受領する（未確立 REQUEST 500 と合わせ 2）");
        assert_eq!(
            &*last_response.borrow(),
            &[LOAD_ACK_OK],
            "LOAD ack は厳密 1 byte [1]（成功・要件 5.1）"
        );

        // --- proxy 駆動 REQUEST(GET): 確立済み proxy で REQUEST を送ると testdll が固定 200 応答を返し、
        //     それが echo でなく proxy 駆動結果として親へ RESPONSE 返送される（R4.7・Observable）---
        let get_req =
            b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
        send_copydata(
            helper.hwnd(),
            parent.hwnd(),
            MsgTag::Request,
            get_req,
            REPLY_TIMEOUT,
        )
        .expect("proxy 駆動 GET REQUEST 送出に失敗");

        assert_eq!(
            helper.shared().requests_handled.get(),
            2,
            "WndProc が proxy 駆動 GET REQUEST を処理する（要件 4.7）"
        );
        assert_eq!(responses.get(), 3, "親が proxy 駆動 GET の RESPONSE を受領する（計 3）");
        {
            let text = String::from_utf8(last_response.borrow().clone())
                .expect("GET RESPONSE は UTF-8");
            assert!(
                text.contains("SHIORI/3.0 200 OK"),
                "proxy 駆動 GET は testdll の固定 200 応答を返す（R4.7・Observable）: {text:?}"
            );
            assert!(
                text.contains("Value: \\0\\s[0]host32 request roundtrip ok\\e"),
                "proxy 駆動 GET の固定 Value 行（echo ではない・R4.7）: {text:?}"
            );
            assert_ne!(
                last_response.borrow().as_slice(),
                &get_req[..],
                "RESPONSE は request の echo ではない（proxy 駆動 SHIORI/3.0 応答）"
            );
        }

        // --- proxy 駆動 REQUEST(NOTIFY): 同経路で helper は proxy.request を駆動し DLL 戻り（204）を
        //     そのまま RESPONSE 返送する（GET と同一経路・host 側で破棄・R4.8 の helper 側）---
        let notify_req =
            b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestNotify\r\n\r\n";
        send_copydata(
            helper.hwnd(),
            parent.hwnd(),
            MsgTag::Request,
            notify_req,
            REPLY_TIMEOUT,
        )
        .expect("proxy 駆動 NOTIFY REQUEST 送出に失敗");

        assert_eq!(
            helper.shared().requests_handled.get(),
            3,
            "WndProc が proxy 駆動 NOTIFY REQUEST も同経路で処理する（R4.8）"
        );
        assert_eq!(responses.get(), 4, "親が proxy 駆動 NOTIFY の RESPONSE を受領する（計 4）");
        {
            let text = String::from_utf8(last_response.borrow().clone())
                .expect("NOTIFY RESPONSE は UTF-8");
            assert!(
                text.contains("SHIORI/3.0 204 No Content"),
                "proxy 駆動 NOTIFY は testdll の固定 204 応答を返す（helper は GET と同一駆動・R4.8）: {text:?}"
            );
        }

        // --- 冪等再 LOAD: もう一度 Load → load 再呼出なしで ack[1] 冪等返送（R2.4・無 panic）---
        send_copydata(
            helper.hwnd(),
            parent.hwnd(),
            MsgTag::Load,
            &[],
            REPLY_TIMEOUT,
        )
        .expect("再 LOAD 送出に失敗");

        assert_eq!(
            helper.shared().loads_attempted.get(),
            2,
            "再 LOAD も TriggerLoad として受領する"
        );
        assert_eq!(
            helper.shared().load_acks_ok.get(),
            2,
            "確立済み再 LOAD は load 再呼出なしで ack[1] を冪等返送する（R2.4）"
        );
        assert_eq!(
            helper.shared().load_acks_fail.get(),
            0,
            "冪等 LOAD でも ack[0] は送出しない"
        );
        assert_eq!(responses.get(), 5, "親が冪等 LOAD ack を追加 1 通受領する（計 5）");
        assert_eq!(
            &*last_response.borrow(),
            &[LOAD_ACK_OK],
            "冪等 LOAD ack も厳密 1 byte [1]（R2.4）"
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
        // 未確立 REQUEST 500(1)＋LOAD ack(2)＋GET(3)＋NOTIFY(4)＋冪等 LOAD ack(5) の計 5 のまま。
        assert_eq!(responses.get(), 5, "不正フレームに応答しない（無応答・計 5 のまま）");

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

        // 窓は Drop で破棄（helper Drop で proxy の courtesy unload → FreeLibrary が走る）。
        drop(helper);
        drop(parent);

        // 後始末（best-effort）: 一時 load_dir（testdll コピー先）を掃除。
        let _ = std::fs::remove_dir_all(&load_dir);
    }
}
