//! areka 本体側 `IShioriHost` 実装（単一 sink・突合枠・メールボックス投函＋プロパティストア）。
//!
//! 脳（`IShiori` 実装）が [`IShioriFactory::create`](shiori_abi::interface::IShioriFactory) 時に
//! 受け取る単一 sink を areka 側で `#[implement(IShioriHost)]` により実装する
//! （design.md §ShioriHostSink, requirements.md 10.2/10.3/10.4/12.4）。
//!
//! ## 役割（design.md §ShioriHostSink・§Boundary Commitments → This Spec Owns）
//! - **単一 sink** が能動通知（[`Raise`](shiori_abi::interface::IShioriHost::Raise)）・
//!   遅延応答（[`Complete`](shiori_abi::interface::IShioriHost::Complete)）・
//!   プロパティアクセス（[`GetProperty`](shiori_abi::interface::IShioriHost::GetProperty)/
//!   [`SetProperty`](shiori_abi::interface::IShioriHost::SetProperty)）の全てを受ける（要件 10.1）。
//! - **突合枠 `Option<CorrelationToken>`**（単一 in-flight・議題3）を thread-safe に所有する。
//!   areka は遅延 request 発行時に [`ShioriHostSink::set_pending_token`] でトークンをセットし、
//!   `Complete` は保持中トークンと突き合わせて応答をメールボックスへ投函する（design.md §State）。
//! - `Raise`/`Complete` は脳の任意スレッドから来うる前提（議題3）で **thread-safe にメールボックスへ
//!   投函して即返す**。突合不能/stale/未知トークンは [`shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN`] を返す。
//! - **プロパティストア** `Mutex<HashMap<String, HSTRING>>` を内蔵し、`GetProperty` はストアから
//!   **同期即答**する（mailbox 投函で代替しない・要件 10.3）。欠落 key は
//!   [`shiori_abi::error::SHIORI_E_PROPERTY_NOT_FOUND`] を返す（暗黙の空値で続行しない）。`SetProperty`
//!   は即書き。任意スレッドから呼出可能（`Mutex` が担保）。
//! - `[in]` HSTRING は借用（呼び出し中のみ有効）。保持/投函する内容は host 側で clone する（要件 10.4/12.4）。
//! - **非循環所有**: host struct は脳（`IShiori`）へ強参照を持たない（脳→host の一方向）。
//!
//! ## 再入規約（契約として doc 固定・design.md §確定設計判断 (b)・要件 10.3）
//! areka は [`IShiori::Get`](shiori_abi::interface::IShiori::Get)/
//! [`IShiori::Notify`](shiori_abi::interface::IShiori::Notify) 呼出中にプロパティストアのロックを
//! 保持しない。→ 脳が `Get` 処理中に同一スレッドで `get_property` を呼び戻してもデッドロックしない。
//! これは各メソッドがロックを最小区間でのみ取得し、他 sink 呼出（`Raise`/`Complete`）を跨いで
//! 保持しないことで構造的に担保される。
//!
//! ## メールボックスの範囲（design.md §Boundary Commitments）
//! メールボックスは「受け皿」までを本仕様が所有する（thread-safe queue の最小実装）。
//! ECS/bevy への実際の配送は本仕様スコープ外であり、ここでは取り出して検証できる最小形に留める。

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use shiori_abi::error::{SHIORI_E_PROPERTY_NOT_FOUND, SHIORI_E_UNKNOWN_TOKEN};
use shiori_abi::outcome::CorrelationToken;
use windows_core::{HSTRING, Result, implement};

/// メールボックスへ投函される host 受信メッセージ（能動通知 / 遅延完了）。
///
/// `Raise`/`Complete` が受領内容を clone してこの enum で投函する。ECS/上位への配送は
/// 本仕様外のため、テスト/上位が取り出して解釈する最小表現に留める（design.md §Boundary）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostMessage {
    /// 能動通知（`Raise`）。さくらスクリプト相当の不透明 HSTRING を内包する（要件 10.1）。
    Raised(HSTRING),
    /// 遅延完了応答（`Complete`）。突合に成功したトークンと応答 HSTRING を内包する（要件 10.1）。
    Completed {
        token: CorrelationToken,
        response: HSTRING,
    },
}

/// areka 本体側の `IShioriHost` 実装（単一 sink）。
///
/// 突合枠・メールボックス・プロパティストアを thread-safe に所有する（`Mutex` による最小実装）。
/// 脳へ強参照を持たない（非循環所有・design.md §ShioriHostSink）。
#[implement(shiori_abi::interface::IShioriHost)]
pub struct ShioriHostSink {
    /// 唯一の保留枠（単一 in-flight・議題3）。areka が遅延 request 発行時にセットし、
    /// `Complete` が突き合わせる。突合成功時にクリアして stale/重複 `Complete` を弾く。
    pending: Mutex<Option<CorrelationToken>>,
    /// 能動通知・遅延完了の受け皿（thread-safe queue の最小実装）。
    mailbox: Mutex<VecDeque<HostMessage>>,
    /// プロパティストア（同期即答・要件 10.3・design.md §確定設計判断 (b)）。
    ///
    /// `GetProperty` はここから同期即答し、`SetProperty` は即書きする。任意スレッドから
    /// 呼出可能（`Mutex` が担保）。key は SSP プロパティシステムの dotted パス（要件 10.2）。
    /// M1 最小 key 集合と値の充填は実装フェーズ／利用側の領分（要件 10.5）。
    properties: Mutex<HashMap<String, HSTRING>>,
}

impl ShioriHostSink {
    /// 空の突合枠・空メールボックス・空プロパティストアで sink を生成する。
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            mailbox: Mutex::new(VecDeque::new()),
            properties: Mutex::new(HashMap::new()),
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

    /// 突合枠を空にする（保留取消・タイムアウト放棄時に areka が呼ぶ）。
    ///
    /// 突合枠が空になるため、以降に遅れて来る `Complete`（タイムアウト後・teardown 後の stale）は
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

    /// areka 側からプロパティストアへ値を充填する（`AsImpl` 経由・要件 10.5）。
    ///
    /// M1 最小 key 集合の値生成源は利用側の領分であり、本 API はその充填口を提供する。
    /// key は SSP プロパティシステムの dotted パス（要件 10.2）。任意スレッドから呼出可能。
    pub fn set_property_value(&self, key: &str, value: HSTRING) {
        self.properties
            .lock()
            .expect("properties mutex poisoned")
            .insert(key.to_string(), value);
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

// windows-core 0.62: `#[implement]` 生成の `*_Impl` 型に対し pub vtable メソッドを実装する。
// 型付き引数（`&HSTRING` 借用／`&mut HSTRING` move-out）により本体から raw ポインタ操作を排する。
// `[in]` HSTRING（`&HSTRING`）は借用——呼び出し中のみ参照可で解放せず、投函/保持する内容は
// clone して所有する（要件 10.4/12.4・interface.rs §HSTRING 所有権規約）。
//
// 再入規約（要件 10.3）: 各メソッドはロックを最小区間でのみ取得し、他 sink 呼出を跨いで保持しない。
// 特に `GetProperty`/`SetProperty` は `properties` ロックのみを短く取り、`Get`/`Notify` 呼出中の
// 同一スレッド呼び戻しでもデッドロックしない。
impl shiori_abi::interface::IShioriHost_Impl for ShioriHostSink_Impl {
    unsafe fn Raise(&self, script: &HSTRING) -> Result<()> {
        // `[in]` 借用: 保持するため clone して所有する（要件 10.4）。
        self.enqueue(HostMessage::Raised(script.clone()));
        Ok(())
    }

    unsafe fn Complete(&self, token: u64, response: &HSTRING) -> Result<()> {
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
                _ => return Err(SHIORI_E_UNKNOWN_TOKEN.into()),
            }
        }
        // `[in]` 借用: 保持するため clone して所有する（要件 10.4）。
        self.enqueue(HostMessage::Completed {
            token,
            response: response.clone(),
        });
        Ok(())
    }

    unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> Result<()> {
        // プロパティストアから同期即答する（mailbox 投函で代替しない・要件 10.3）。
        // ロックは最小区間で取得し、`Get`/`Notify` 呼出中の同一スレッド呼び戻しでも
        // デッドロックしない（再入規約）。
        let store = self.properties.lock().expect("properties mutex poisoned");
        match store.get(&key.to_string()) {
            Some(v) => {
                // 存在時: 値を out へ move-out（callee 確保・caller 解放）。
                *out_value = v.clone();
                Ok(())
            }
            // 欠落 key は PROPERTY_NOT_FOUND（out_value 未書込・暗黙の空値で続行しない）。
            None => Err(SHIORI_E_PROPERTY_NOT_FOUND.into()),
        }
    }

    unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> Result<()> {
        // 即書き（要件 10.1）。`[in]` 借用ゆえ key/value ともに clone して所有する。
        self.properties
            .lock()
            .expect("properties mutex poisoned")
            .insert(key.to_string(), value.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! areka 側 `IShioriHost` 実装の結合テスト（要件 10.2/10.3/10.4/12.4・design.md §System Flows）。
    //!
    //! host を立て、**COM ポインタ（`IShioriHost`）経由の安全面メソッド**（`raise`/`complete`/
    //! `get_property`/`set_property`）で駆動する。pub メソッド化により vtable 直呼びヘルパは不要。
    //!
    //! 観測:
    //! - (a) トークンをセット→`complete(token, resp)` でメールボックスに `Completed` が入り Ok。
    //! - (b) 未知トークンの `complete` は `UnknownToken` を返し投函しない。
    //! - (c) `raise(script)` でメールボックスに `Raised` が入り Ok。
    //! - (d) `set_property`→`get_property` の同期即答・欠落 key→`PropertyNotFound`・別スレッド set。
    //! - (e) `Get` 実装内からの `get_property` 再入同期応答（デッドロックしない・要件 10.3）。

    use super::*;
    use shiori_abi::error::ShioriError;
    use shiori_abi::interface::IShioriHost;
    use windows_core::AsImpl;

    /// (a) 突合枠にトークンをセット → 一致 `complete` でメールボックスに応答が入り Ok（要件 10.1）。
    #[test]
    fn complete_with_matching_token_enqueues_response_and_returns_ok() {
        let sink = ShioriHostSink::new();
        sink.set_pending_token(CorrelationToken(7));
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("deferred-response-body");
        host.complete(CorrelationToken(7), &response)
            .expect("一致トークンの complete は Ok");

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

    /// (b) 未知/stale トークンの `complete` は `UnknownToken` を返し投函しないこと（議題3）。
    #[test]
    fn complete_with_unknown_token_returns_unknown_token_error() {
        let sink = ShioriHostSink::new();
        sink.set_pending_token(CorrelationToken(7));
        let host: IShioriHost = sink.into();

        // 突合枠(7)と不一致のトークン(999)。
        let response = HSTRING::from("stale");
        let err = host
            .complete(CorrelationToken(999), &response)
            .expect_err("未知トークンの complete は Err");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "未知トークンの complete は UnknownToken を返すこと, got {err:?}"
        );

        let observed = host_inner(&host);
        assert_eq!(observed.mailbox_len(), 0, "未知トークンでは投函しないこと");
        assert_eq!(
            observed.pending_token(),
            Some(CorrelationToken(7)),
            "突合枠は不一致では消費されないこと"
        );
    }

    /// 突合枠が空のときの `complete` も `UnknownToken`（議題3・空枠）。
    #[test]
    fn complete_with_empty_slot_returns_unknown_token_error() {
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("orphan");
        let err = host
            .complete(CorrelationToken(1), &response)
            .expect_err("空枠の complete は Err");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "空の突合枠での complete は UnknownToken を返すこと, got {err:?}"
        );
        assert_eq!(host_inner(&host).mailbox_len(), 0);
    }

    /// (c) `raise(script)` でメールボックスに通知が入り Ok（要件 10.1/10.4）。
    #[test]
    fn raise_enqueues_script_and_returns_ok() {
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();
        let script = HSTRING::from("\\h\\s[0]hello");
        host.raise(&script).expect("raise は Ok");

        let observed = host_inner(&host);
        assert_eq!(
            observed.try_recv(),
            Some(HostMessage::Raised(HSTRING::from("\\h\\s[0]hello"))),
            "メールボックスに能動通知が投函されていること"
        );
    }

    /// `complete` の重複呼び出し: 1 回目で枠をクリアするため 2 回目は弾かれること（stale 防御・議題3）。
    #[test]
    fn duplicate_complete_is_rejected_after_first_consumes_slot() {
        let sink = ShioriHostSink::new();
        sink.set_pending_token(CorrelationToken(42));
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("once");

        host.complete(CorrelationToken(42), &response)
            .expect("初回 complete は Ok");
        let err = host
            .complete(CorrelationToken(42), &response)
            .expect_err("枠消費後の重複 complete は Err");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "枠消費後の重複 complete は UnknownToken を返すこと, got {err:?}"
        );
        assert_eq!(host_inner(&host).mailbox_len(), 1, "投函は初回の 1 件のみ");
    }

    /// (d) `set_property`→`get_property` の同期即答・欠落 key→`PropertyNotFound`（要件 10.2/10.3）。
    #[test]
    fn set_then_get_property_is_synchronous_and_missing_key_errors() {
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();

        let key = HSTRING::from("path.to.key");
        let value = HSTRING::from("some-value");
        host.set_property(&key, &value).expect("set_property は Ok");

        // 同期即答（mailbox 投函で代替しない）: set した値がそのまま move-out される。
        let got = host.get_property(&key).expect("get_property は Ok");
        assert_eq!(got, value, "設定した値が同期即答で move-out されること");

        // 欠落 key は PropertyNotFound（暗黙の空値で続行しない）。
        let err = host
            .get_property(&HSTRING::from("no.such.key"))
            .expect_err("欠落 key の get_property は Err");
        assert!(
            matches!(err, ShioriError::PropertyNotFound),
            "欠落 key は PropertyNotFound を返すこと, got {err:?}"
        );
    }

    /// areka 側充填 API `set_property_value`（`AsImpl` 経由）で充填した値が同期即答されること（要件 10.5）。
    #[test]
    fn set_property_value_fills_store_observable_via_get_property() {
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();
        // areka 側の充填口（AsImpl 経由）で値を埋める。
        host_inner(&host).set_property_value("ghost.name", HSTRING::from("reference"));

        let got = host
            .get_property(&HSTRING::from("ghost.name"))
            .expect("充填済み key の get_property は Ok");
        assert_eq!(got, HSTRING::from("reference"), "充填値が同期即答されること");
    }

    /// (e) 別スレッドからのプロパティストア充填が同期反映されること（プロパティストアのスレッド安全）。
    ///
    /// `IShioriHost`（COM ポインタ）はアパートメント制約で `Send` ではないため、スレッド間で
    /// COM ポインタを跨がせない。プロパティストアの thread-safe 性（`Mutex` 担保）は、`Send + Sync`
    /// な `ShioriHostSink` 実体（`Arc` 共有）へ別スレッドから `set_property_value` して検証する。
    #[test]
    fn property_store_is_thread_safe() {
        use std::sync::Arc;

        // ShioriHostSink は Mutex フィールドのみゆえ Send + Sync。Arc で別スレッドと共有する。
        let sink = Arc::new(ShioriHostSink::new());
        let sink_for_thread = Arc::clone(&sink);

        let handle = std::thread::spawn(move || {
            // 別スレッドで充填（任意スレッド可・Mutex が担保）。
            sink_for_thread.set_property_value("threaded.key", HSTRING::from("from-thread"));
        });
        handle.join().expect("スレッド join");

        // 同スレッドから COM 面 `get_property` で読めること（同期反映）。
        // Arc を COM 面 IShioriHost に変換するため一意所有を取り出す（Debug 不要の match で剥がす）。
        let sink = match Arc::try_unwrap(sink) {
            Ok(s) => s,
            Err(_) => panic!("Arc の一意所有を取り出せること（別スレッドは join 済み）"),
        };
        let host: IShioriHost = sink.into();
        let got = host
            .get_property(&HSTRING::from("threaded.key"))
            .expect("別スレッド set の値を get できること");
        assert_eq!(got, HSTRING::from("from-thread"), "別スレッド set が同期反映されること");
    }

    /// `Get` 実装内から `get_property` を同一スレッドで呼び戻してもデッドロックしないこと（再入規約・要件 10.3）。
    ///
    /// mock 脳の `Get` 実装が host の `get_property` を呼び戻す。areka はプロパティストアの
    /// ロックを `Get` 呼出を跨いで保持しないため、同一スレッド呼び戻しでデッドロックしない。
    #[test]
    fn get_property_reentrant_call_from_brain_get_does_not_deadlock() {
        use shiori_abi::interface::{IShiori, IShiori_Impl};
        use windows_core::{HRESULT, Result as ComResult, implement};

        /// `Get` 内で保持 host の `get_property` を呼び戻す再入 mock 脳。
        #[allow(non_snake_case)]
        #[implement(IShiori)]
        struct ReentrantBrain {
            host: IShioriHost,
        }

        impl IShiori_Impl for ReentrantBrain_Impl {
            unsafe fn Get(
                &self,
                _input: &HSTRING,
                out_response: &mut HSTRING,
                _out_token: &mut u64,
            ) -> HRESULT {
                // Get 処理中に同一スレッドで get_property を呼び戻す（再入）。
                let v = self
                    .host
                    .get_property(&HSTRING::from("reentrant.key"))
                    .expect("再入 get_property は Ok（デッドロックしない）");
                *out_response = v;
                HRESULT(0) // S_OK
            }

            unsafe fn Notify(&self, _input: &HSTRING) -> ComResult<()> {
                Ok(())
            }
        }

        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();
        host_inner(&host).set_property_value("reentrant.key", HSTRING::from("reentrant-value"));

        let brain: IShiori = ReentrantBrain { host: host.clone() }.into();

        // brain.get が内部で host.get_property を呼び戻す。デッドロックせず値を得られること。
        let outcome = brain
            .get(&HSTRING::from("GET SHIORI/3.0..."))
            .expect("再入する get は Ok");
        assert_eq!(
            outcome,
            shiori_abi::outcome::GetOutcome::Immediate(HSTRING::from("reentrant-value")),
            "Get 内から呼び戻した get_property の値が即時応答として返ること（再入・非デッドロック）"
        );
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
