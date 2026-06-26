//! areka 本体側 `IShioriHost` 実装（単一 sink・突合枠・メールボックス投函）。
//!
//! 脳（`IShiori` 実装）が [`shiori_abi::interface::IShiori::Load`] で受け取る単一 sink を
//! areka 側で `#[implement(IShioriHost)]` により実装する（design.md §Components and Interfaces →
//! `in-proc activation + IShioriHost impl`, requirements.md 3.3/6.1〜6.5）。
//!
//! ## 役割（design.md §IShioriHost Postconditions・§Boundary Commitments → This Spec Owns）
//! - **単一 sink** が能動通知（[`Raise`](shiori_abi::interface::IShioriHost::Raise)）と
//!   遅延応答（[`Complete`](shiori_abi::interface::IShioriHost::Complete)）の双方を受ける（要件 6.1）。
//! - **突合枠 `Option<CorrelationToken>`**（単一 in-flight・議題3）を thread-safe に所有する。
//!   areka は遅延 request 発行時に [`ShioriHostSink::set_pending_token`] でトークンをセットし、
//!   `Complete` は保持中トークンと突き合わせて応答をメールボックスへ投函する（design.md §State）。
//! - `Raise`/`Complete` は脳の任意スレッドから来うる前提（議題3）で **thread-safe にメールボックスへ
//!   投函して即返す**。突合不能/stale/未知トークンは [`shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN`] を返す。
//! - `[in]` HSTRING は借用（呼び出し中のみ有効）。保持/投函する内容は host 側で clone する（議題2/要件 6.5）。
//! - **非循環所有**: host struct は脳（`IShiori`）へ強参照を持たない（脳→host の一方向）。
//!
//! ## メールボックスの範囲（design.md §Boundary Commitments）
//! メールボックスは「受け皿」までを本仕様が所有する（thread-safe queue の最小実装）。
//! ECS/bevy への実際の配送は本仕様スコープ外であり、ここでは取り出して検証できる最小形に留める。

#![allow(dead_code)] // メールボックス取り出し API 等は task 4.1 範囲では結合テストからのみ利用する。

use std::collections::VecDeque;
use std::sync::Mutex;

use shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN;
use shiori_abi::outcome::CorrelationToken;
use windows_core::{HRESULT, HSTRING, implement};

/// メールボックスへ投函される host 受信メッセージ（能動通知 / 遅延完了）。
///
/// `Raise`/`Complete` が受領内容を clone してこの enum で投函する。ECS/上位への配送は
/// 本仕様外のため、テスト/上位が取り出して解釈する最小表現に留める（design.md §Boundary）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostMessage {
    /// 能動通知（`Raise`）。さくらスクリプト相当の不透明 HSTRING を内包する（要件 6.3）。
    Raised(HSTRING),
    /// 遅延完了応答（`Complete`）。突合に成功したトークンと応答 HSTRING を内包する（要件 6.4）。
    Completed {
        token: CorrelationToken,
        response: HSTRING,
    },
}

/// areka 本体側の `IShioriHost` 実装（単一 sink）。
///
/// 突合枠とメールボックスを thread-safe に所有する（`Mutex` による最小実装）。
/// 脳へ強参照を持たない（非循環所有・design.md §IShioriHost）。
#[implement(shiori_abi::interface::IShioriHost)]
pub struct ShioriHostSink {
    /// 唯一の保留枠（単一 in-flight・議題3）。areka が遅延 request 発行時にセットし、
    /// `Complete` が突き合わせる。突合成功時にクリアして stale/重複 `Complete` を弾く。
    pending: Mutex<Option<CorrelationToken>>,
    /// 能動通知・遅延完了の受け皿（thread-safe queue の最小実装）。
    mailbox: Mutex<VecDeque<HostMessage>>,
}

impl ShioriHostSink {
    /// 空の突合枠・空メールボックスで sink を生成する。
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            mailbox: Mutex::new(VecDeque::new()),
        }
    }

    /// 遅延 request 発行時に areka が突合枠へ相関トークンをセットする（単一 in-flight）。
    ///
    /// 既存の保留トークンは上書きされる（単一 in-flight 前提では発行前に解消済みのはず）。
    pub fn set_pending_token(&self, token: CorrelationToken) {
        *self.pending.lock().expect("pending mutex poisoned") = Some(token);
    }

    /// 現在の突合枠の内容を返す（観測用）。
    pub fn pending_token(&self) -> Option<CorrelationToken> {
        *self.pending.lock().expect("pending mutex poisoned")
    }

    /// 突合枠を空にする（保留取消・タイムアウト放棄時に areka が呼ぶ・task 4.2）。
    ///
    /// 突合枠が空になるため、以降に遅れて来る `Complete`（タイムアウト後・`Unload` 後の stale）は
    /// トークン不一致で [`SHIORI_E_UNKNOWN_TOKEN`] により弾かれる（議題3）。
    pub fn clear_pending_token(&self) {
        *self.pending.lock().expect("pending mutex poisoned") = None;
    }

    /// メールボックスから先頭メッセージを取り出す（FIFO・受け皿からの取り出し）。
    pub fn try_recv(&self) -> Option<HostMessage> {
        self.mailbox
            .lock()
            .expect("mailbox mutex poisoned")
            .pop_front()
    }

    /// メールボックスに滞留しているメッセージ数（観測用）。
    pub fn mailbox_len(&self) -> usize {
        self.mailbox.lock().expect("mailbox mutex poisoned").len()
    }

    /// メールボックスへ thread-safe に投函する内部ヘルパ。
    fn enqueue(&self, msg: HostMessage) {
        self.mailbox
            .lock()
            .expect("mailbox mutex poisoned")
            .push_back(msg);
    }
}

impl Default for ShioriHostSink {
    fn default() -> Self {
        Self::new()
    }
}

// windows-core 0.62: `#[implement]` 生成の `*_Impl` 型に対し raw vtable メソッドを実装する。
// 引数の `[in]` HSTRING（`*const HSTRING`）は借用——呼び出し中のみ参照可で解放せず、
// 投函する内容は clone して所有する（議題2/要件 6.5・interface.rs §HSTRING 所有権規約）。
impl shiori_abi::interface::IShioriHost_Impl for ShioriHostSink_Impl {
    unsafe fn Raise(&self, script: *const HSTRING) -> HRESULT {
        // `[in]` 借用: 保持するため clone して所有する（要件 6.5）。
        let script = unsafe { (*script).clone() };
        self.enqueue(HostMessage::Raised(script));
        HRESULT(0) // S_OK
    }

    unsafe fn Complete(&self, token: u64, response: *const HSTRING) -> HRESULT {
        let token = CorrelationToken(token);
        // 突合枠と照合（単一 in-flight・議題3）。一致時のみ受理してクリアし、
        // stale/重複 `Complete` を以降トークン不一致で弾けるようにする。
        {
            let mut pending = self.pending.lock().expect("pending mutex poisoned");
            match *pending {
                Some(expected) if expected == token => {
                    *pending = None;
                }
                // 突合枠が空 or トークン不一致（未知/stale）= 突合不能（議題3）。
                _ => return SHIORI_E_UNKNOWN_TOKEN,
            }
        }
        // `[in]` 借用: 保持するため clone して所有する（要件 6.5）。
        let response = unsafe { (*response).clone() };
        self.enqueue(HostMessage::Completed { token, response });
        HRESULT(0) // S_OK
    }
}

#[cfg(test)]
mod tests {
    //! areka 側 `IShioriHost` 実装の結合テスト（要件 3.3/6.1〜6.5・design.md §System Flows）。
    //!
    //! host を立て、**COM ポインタ（`IShioriHost`）経由の vtable 直呼び**で `Raise`/`Complete` を
    //! 駆動する。windows-core 0.62 の `#[interface]` 生成 raw メソッドは ABI 定義モジュール private の
    //! ため、別クレートからは `Interface::vtable(self).<slot>(self.as_raw(), ..)` で呼ぶ
    //! （tasks.md §Implementation Notes）。
    //!
    //! 観測:
    //! - (a) トークンをセット→`Complete(token, resp)` でメールボックスに `Completed` が入り `S_OK`。
    //! - (b) 未知トークンの `Complete` は `SHIORI_E_UNKNOWN_TOKEN` を返し投函しない。
    //! - (c) `Raise(script)` でメールボックスに `Raised` が入り `S_OK`。

    use super::*;
    use shiori_abi::interface::IShioriHost;
    use windows_core::{AsImpl, Interface};

    /// COM ポインタ経由で raw `Raise` を呼ぶヘルパ（vtable 直呼び・Implementation Notes）。
    ///
    /// # Safety
    /// `host` は有効な `IShioriHost` COM ポインタ、`script` は呼び出し中有効な `*const HSTRING`。
    unsafe fn call_raise(host: &IShioriHost, script: &HSTRING) -> HRESULT {
        unsafe { (Interface::vtable(host).Raise)(host.as_raw(), script as *const HSTRING) }
    }

    /// COM ポインタ経由で raw `Complete` を呼ぶヘルパ（vtable 直呼び・Implementation Notes）。
    ///
    /// # Safety
    /// `host` は有効な `IShioriHost` COM ポインタ、`response` は呼び出し中有効な `*const HSTRING`。
    unsafe fn call_complete(host: &IShioriHost, token: u64, response: &HSTRING) -> HRESULT {
        unsafe {
            (Interface::vtable(host).Complete)(host.as_raw(), token, response as *const HSTRING)
        }
    }

    /// (a) 突合枠にトークンをセット → 一致 `Complete` でメールボックスに応答が入り `S_OK`（要件 3.3/6.4）。
    #[test]
    fn complete_with_matching_token_enqueues_response_and_returns_s_ok() {
        let sink = ShioriHostSink::new();
        sink.set_pending_token(CorrelationToken(7));
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("deferred-response-body");
        let hr = unsafe { call_complete(&host, 7, &response) };
        assert!(hr.is_ok(), "一致トークンの Complete は S_OK: 0x{:08X}", hr.0);

        // COM ポインタが包む実体を取り出して観測する。
        let observed = host_inner(&host);
        assert_eq!(
            observed.try_recv(),
            Some(HostMessage::Completed {
                token: CorrelationToken(7),
                response: HSTRING::from("deferred-response-body"),
            }),
            "メールボックスに完了応答が投函されていること"
        );
        assert_eq!(
            observed.pending_token(),
            None,
            "突合成功で突合枠がクリアされること（stale/重複防御）"
        );
    }

    /// (b) 未知/stale トークンの `Complete` は `SHIORI_E_UNKNOWN_TOKEN` を返し投函しないこと（議題3）。
    #[test]
    fn complete_with_unknown_token_returns_unknown_token_error() {
        let sink = ShioriHostSink::new();
        sink.set_pending_token(CorrelationToken(7));
        let host: IShioriHost = sink.into();

        // 突合枠(7)と不一致のトークン(999)。
        let response = HSTRING::from("stale");
        let hr = unsafe { call_complete(&host, 999, &response) };
        assert_eq!(
            hr, SHIORI_E_UNKNOWN_TOKEN,
            "未知トークンの Complete は SHIORI_E_UNKNOWN_TOKEN を返すこと"
        );

        let observed = host_inner(&host);
        assert_eq!(
            observed.mailbox_len(),
            0,
            "未知トークンでは投函しないこと"
        );
        assert_eq!(
            observed.pending_token(),
            Some(CorrelationToken(7)),
            "突合枠は不一致では消費されないこと"
        );
    }

    /// 突合枠が空のときの `Complete` も `SHIORI_E_UNKNOWN_TOKEN`（議題3・空枠）。
    #[test]
    fn complete_with_empty_slot_returns_unknown_token_error() {
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("orphan");
        let hr = unsafe { call_complete(&host, 1, &response) };
        assert_eq!(
            hr, SHIORI_E_UNKNOWN_TOKEN,
            "空の突合枠での Complete は SHIORI_E_UNKNOWN_TOKEN を返すこと"
        );
        assert_eq!(host_inner(&host).mailbox_len(), 0);
    }

    /// (c) `Raise(script)` でメールボックスに通知が入り `S_OK`（要件 6.3/6.5）。
    #[test]
    fn raise_enqueues_script_and_returns_s_ok() {
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();
        let script = HSTRING::from("\\h\\s[0]hello");
        let hr = unsafe { call_raise(&host, &script) };
        assert!(hr.is_ok(), "Raise は S_OK: 0x{:08X}", hr.0);

        let observed = host_inner(&host);
        assert_eq!(
            observed.try_recv(),
            Some(HostMessage::Raised(HSTRING::from("\\h\\s[0]hello"))),
            "メールボックスに能動通知が投函されていること"
        );
    }

    /// `Complete` の重複呼び出し: 1 回目で枠をクリアするため 2 回目は弾かれること（stale 防御・議題3）。
    #[test]
    fn duplicate_complete_is_rejected_after_first_consumes_slot() {
        let sink = ShioriHostSink::new();
        sink.set_pending_token(CorrelationToken(42));
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("once");

        let hr1 = unsafe { call_complete(&host, 42, &response) };
        assert!(hr1.is_ok(), "初回 Complete は S_OK");
        let hr2 = unsafe { call_complete(&host, 42, &response) };
        assert_eq!(
            hr2, SHIORI_E_UNKNOWN_TOKEN,
            "枠消費後の重複 Complete は SHIORI_E_UNKNOWN_TOKEN を返すこと"
        );
        assert_eq!(host_inner(&host).mailbox_len(), 1, "投函は初回の 1 件のみ");
    }

    /// COM ポインタ `IShioriHost` が包む `ShioriHostSink` 実体への参照を取り出す。
    ///
    /// windows-core 0.62 の `#[implement]` は `AsImpl<ShioriHostSink>` を生成するため、
    /// `host.as_impl()` で実体参照を借りられる（テスト専用の観測）。
    fn host_inner(host: &IShioriHost) -> &ShioriHostSink {
        // Safety: `host` は本テストで `ShioriHostSink` から構築した COM ポインタであり、
        // 実装実体が `ShioriHostSink` であることが保証される。
        unsafe { AsImpl::<ShioriHostSink>::as_impl(host) }
    }
}
