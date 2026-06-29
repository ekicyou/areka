//! in-proc アクティベーション経路とリクエスト利用規律（単一 in-flight・遅延完了タイムアウト）。
//!
//! 本モジュールは、同一プロセス内の `IShiori` 実装（脳）へ到達し、`Load` で task 4.1 の
//! [`ShioriHostSink`](crate::shiori_host::ShioriHostSink) を sink として受け渡す最小経路
//! （アクティベーション）と、areka 側のリクエスト利用規律を所有する
//! （design.md §System Flows → ライフサイクル, §Components and Interfaces → IShiori → State Management,
//! §Boundary Commitments → This Spec Owns・議題3, requirements.md 1.5/2.4/5.1/5.2）。
//!
//! ## アクティベーション最小経路（design.md §System Flows → ライフサイクル, 要件 5.1/5.4）
//! [`ShioriSession::activate`] は in-proc の `&IShiori`（脳）を受け取り、areka 実装の
//! [`ShioriHostSink`] を生成して [`ShioriExt::load`](shiori_abi::ergonomic::ShioriExt::load)
//! で脳へ sink を受け渡す。脳は `Load`〜`Unload` 間 host を AddRef 保持し、`Unload` で
//! Release する（議題2・保持参照モデル）。areka は脳解放前に必ず [`ShioriSession::unload`]
//! を呼ぶ（teardown 順序）。実装種別差（native / 過去互換）はこのアクティベーション経路に
//! のみ局所化し、確立済みの `IShiori` 利用面へ波及させない（要件 1.5）。本セッションは
//! native in-proc 経路の最小受け皿である。
//!
//! ## 利用規律: 単一 in-flight（議題3・design.md §State Management, 要件 2.4）
//! areka は同時に高々 1 リクエストのみ発行する。[`ShioriSession::request`] が
//! [`RequestOutcome::Deferred`](shiori_abi::outcome::RequestOutcome::Deferred) を返した間は
//! **保留状態**であり、host の突合枠へトークンをセットする
//! （[`ShioriHostSink::set_pending_token`]）。保留中に次の `request` を呼ぼうとすると
//! [`SessionError::RequestInFlight`] で拒否する。保留は次のいずれかで解消し次 request を許可する:
//! - **`Complete` 受領**: host メールボックスへ `Completed` が届くと
//!   [`ShioriSession::poll_completions`] が突合枠と照合して保留を解除する（要件 6.4 経路）。
//! - **`Unload`**: [`ShioriSession::unload`] が保留を取消する。
//! - **遅延完了タイムアウト**: [`ShioriSession::expire_if_elapsed`] が設定可能な
//!   タイムアウトを超過した保留枠を放棄し次 request を許可する。タイムアウト超過後に遅れて
//!   来た `Complete` は host が [`SHIORI_E_UNKNOWN_TOKEN`](shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN)
//!   で弾く（突合枠が既にクリアされているため）。
//!
//! ## タイムアウトの決定性（議題3 e1・テスト容易性）
//! タイムアウト判定は実時間 `sleep` に依存しない。設定値 [`ShioriSession::timeout`] は
//! [`Duration`] で保持し、保留開始時刻からの経過時間 [`Duration`] を**注入可能な**
//! [`expire_if_elapsed`](ShioriSession::expire_if_elapsed) へ渡して判定する。これにより
//! 結合テストが実時間を待たずタイムアウト経路を決定的に検証できる。
//!
//! ## 非ブロッキング前提（議題3 e2・design.md §State Management・要件 5.2）
//! `Request`/`Load`/`Unload` は in-proc 直 vtable で areka の呼び出しスレッド上で実行され、
//! 脳が即時に `S_OK` / [`SHIORI_S_PENDING`](shiori_abi::error::SHIORI_S_PENDING) / error HRESULT を
//! 返す契約である（非ブロッキング）。重い処理は脳側で `SHIORI_S_PENDING` ＋後続 `Complete` へ
//! 後送りされ、本セッションのいずれのメソッドも同期ハングしない。in-proc 直 vtable のため
//! OOP マーシャリングは介在しない（要件 5.2）。

use std::time::Duration;

use shiori_abi::ergonomic::ShioriExt;
use shiori_abi::interface::{IShiori, IShioriHost};
use shiori_abi::outcome::{CorrelationToken, RequestOutcome};
use windows_core::HSTRING;

use crate::shiori_host::{HostMessage, ShioriHostSink};

/// in-proc セッション/アクティベーションの利用規律エラー（areka 側の状態に由来）。
///
/// `IShiori` の HRESULT 由来エラー（[`shiori_abi::error::ShioriError`]）とは別に、areka 側の
/// **利用規律**（単一 in-flight 等）違反を型化する（design.md §State Management・議題3）。
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    /// 保留 request が in-flight の間に次 request を発行しようとした（単一 in-flight・議題3）。
    #[error("a deferred request is still in flight (single in-flight discipline)")]
    RequestInFlight,
    /// `IShiori` 操作が失敗した（HRESULT 由来）。元の [`shiori_abi::error::ShioriError`] を内包する。
    #[error(transparent)]
    Shiori(#[from] shiori_abi::error::ShioriError),
}

/// `request` の利用規律つき結果（areka 側）。
///
/// 即時応答はそのまま返し、遅延時は保留枠へトークンをセットしたうえで `Deferred` を返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRequest {
    /// 即時応答。脳が `S_OK`＋応答 HSTRING を返した（保留枠は使わない）。
    Immediate(HSTRING),
    /// 遅延。保留枠へトークンをセットした。次 request は保留解除まで拒否される（単一 in-flight）。
    Deferred(CorrelationToken),
}

/// in-proc アクティベーション経路と利用規律を所有するセッション（design.md §This Spec Owns・議題3）。
///
/// 脳（`IShiori`）への in-proc 参照と areka 実装の sink（`IShioriHost`）を保持し、単一 in-flight・
/// 遅延完了タイムアウト・`Unload` 保留取消の利用規律を担う。脳の AddRef 保持は脳側が
/// `Load`〜`Unload` 間に行う（議題2）。本セッションは脳へ COM 強参照（[`IShiori`]）を保持するが、
/// host 実装（[`ShioriHostSink`]）は脳へ強参照を持たない（非循環・所有方向 areka→脳→host）。
pub struct ShioriSession {
    /// in-proc 脳への参照（COM ポインタ）。`activate` で受け取り `Load`/`Request`/`Unload` に用いる。
    brain: IShiori,
    /// areka 実装の sink（単一 in-flight 突合枠＋メールボックスを所有）。脳が `Load`〜`Unload` 間保持する。
    host: IShioriHost,
    /// 設定可能な遅延完了タイムアウト（議題3 e1）。経過時間との比較は注入された経過時間で行う。
    // デモ駆動経路は遅延を Complete で即解消するためタイムアウトを参照しない。本フィールドは
    // タイムアウト経路を検証する結合テスト（task 5.x の `expire_if_elapsed`/`with_timeout`）からのみ読まれる。
    #[allow(dead_code)]
    timeout: Duration,
    /// 現在保留中の相関トークン（単一 in-flight）。`Deferred` でセットし、解消（`Complete`/`Unload`/
    /// タイムアウト）でクリアする。host 側突合枠（[`ShioriHostSink`]）と同期して扱う。
    pending: Option<CorrelationToken>,
}

impl ShioriSession {
    /// 既定タイムアウト（議題3 e1: SSP 同様の無応答ガード相当の保守的既定値）。
    ///
    /// 設定可能であり、[`ShioriSession::with_timeout`] で上書きできる。
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

    /// in-proc の脳へアクティベーションし、`Load` で areka 実装の sink を受け渡す（要件 5.1/5.4/6.2）。
    ///
    /// 既定タイムアウト（[`DEFAULT_TIMEOUT`](Self::DEFAULT_TIMEOUT)）でセッションを確立する。
    /// 脳は `Load` で受け取った host を AddRef して `Unload` まで保持してよい（議題2）。
    ///
    /// # Errors
    /// `Load` が失敗 HRESULT を返した場合 [`SessionError::Shiori`]（`LoadFailed`）を返す（要件 2.3）。
    ///
    /// # 非ブロッキング
    /// `Load` は即時に戻る契約（議題3 e2・in-proc 直 vtable）。
    pub fn activate(brain: IShiori) -> Result<Self, SessionError> {
        Self::activate_with_timeout(brain, Self::DEFAULT_TIMEOUT)
    }

    /// 遅延完了タイムアウトを指定して in-proc アクティベーションする（議題3 e1・設定可能）。
    ///
    /// 新規 [`ShioriHostSink`] を生成し、[`ShioriExt::load`] で脳へ sink を受け渡す。
    pub fn activate_with_timeout(brain: IShiori, timeout: Duration) -> Result<Self, SessionError> {
        // areka 実装の sink を生成（単一 in-flight 突合枠＋メールボックスを所有・task 4.1）。
        let host: IShioriHost = ShioriHostSink::new().into();
        // アクティベーション: in-proc 脳へ `Load` で sink を受け渡す（ShioriExt 越し・要件 5.1/6.2）。
        // 脳は host を AddRef 保持してよい（議題2）。失敗は ShioriError へ写る。
        brain.load(&host)?;
        Ok(Self {
            brain,
            host,
            timeout,
            pending: None,
        })
    }

    /// 遅延完了タイムアウトを上書きする（議題3 e1・設定可能）。
    // デモ駆動経路では使わない。タイムアウト経路を検証する結合テスト（task 5.x）からのみ利用する。
    #[allow(dead_code)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 現在の遅延完了タイムアウト設定（議題3 e1）。
    // デモ駆動経路では使わない。タイムアウト設定を観測する結合テスト（task 5.x）からのみ利用する。
    #[allow(dead_code)]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// 保留 request が in-flight か（単一 in-flight・観測用）。
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// 現在保留中の相関トークン（観測用）。
    // デモ駆動経路は保留有無（`is_pending`）のみ観測する。保留トークンを照合する結合テスト（task 5.x）からのみ利用する。
    #[allow(dead_code)]
    pub fn pending_token(&self) -> Option<CorrelationToken> {
        self.pending
    }

    /// areka 実装の sink（`IShioriHost`）への参照（脳→host 通知の観測・テスト用）。
    // デモ駆動経路は `poll_completions` 越しに観測する。sink を直接観測する結合テスト（task 5.x）からのみ利用する。
    #[allow(dead_code)]
    pub fn host(&self) -> &IShioriHost {
        &self.host
    }

    /// 同期 request を発行する。単一 in-flight 規律を適用する（議題3・要件 2.4）。
    ///
    /// 保留中（[`is_pending`](Self::is_pending)）に呼ぶと [`SessionError::RequestInFlight`] で拒否する。
    /// 脳が [`RequestOutcome::Deferred`] を返した場合は host の突合枠へトークンをセットし
    /// （[`ShioriHostSink::set_pending_token`]）、セッションの保留状態を立てる。
    ///
    /// # Errors
    /// - 保留中の再発行 → [`SessionError::RequestInFlight`]。
    /// - `IShiori::Request` 失敗（未ロード含む）→ [`SessionError::Shiori`]。
    ///
    /// # 非ブロッキング
    /// `Request` は即時に戻る契約（議題3 e2）。重い処理は遅延応答（`Deferred`）へ回る。
    pub fn request(&mut self, content: &HSTRING) -> Result<SessionRequest, SessionError> {
        if self.pending.is_some() {
            // 単一 in-flight: 保留解除（Complete/Unload/タイムアウト）まで次 request を発行しない（議題3）。
            return Err(SessionError::RequestInFlight);
        }
        match self.brain.request(content)? {
            RequestOutcome::Immediate(response) => Ok(SessionRequest::Immediate(response)),
            RequestOutcome::Deferred(token) => {
                // 突合枠（host 側）へトークンをセットし、後続 Complete の突合を可能にする（task 4.1）。
                self.host_sink().set_pending_token(token);
                self.pending = Some(token);
                Ok(SessionRequest::Deferred(token))
            }
        }
    }

    /// host メールボックスから完了/通知を取り出し、保留 request の `Complete` を解消する（要件 6.4 経路）。
    ///
    /// 突合枠と一致する `Completed` を受領したら保留を解除し次 request を許可する。`Raised`（能動通知）は
    /// そのまま返す（保留状態には影響しない）。取り出したメッセージを順に返す（受け皿からの取り出し）。
    ///
    /// 戻り値は取り出した [`HostMessage`] の列（FIFO）。`Completed` が保留トークンと一致した場合のみ
    /// 保留状態を解除する。
    pub fn poll_completions(&mut self) -> Vec<HostMessage> {
        let mut drained = Vec::new();
        while let Some(msg) = self.host_sink().try_recv() {
            if let HostMessage::Completed { token, .. } = &msg
                && self.pending == Some(*token)
            {
                // 対応する Complete を受領: 保留解除→次 request 許可（議題3）。
                self.pending = None;
            }
            drained.push(msg);
        }
        drained
    }

    /// 設定タイムアウトを超過した保留枠を放棄し次 request を許可する（議題3 e1・決定的判定）。
    ///
    /// `elapsed` は保留開始からの経過時間（呼び出し側が注入する・実時間 `sleep` に非依存）。
    /// `elapsed >= timeout` のとき保留枠を放棄する（host 側突合枠もクリアして、遅れて来る
    /// `Complete` を host が [`SHIORI_E_UNKNOWN_TOKEN`](shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN) で弾く）。
    ///
    /// # 戻り値
    /// 放棄した場合 `true`（保留が解除され次 request 可能）、未超過 or 非保留なら `false`。
    // デモ駆動経路は遅延を Complete で即解消するため使わない。タイムアウト経路を検証する結合テスト（task 5.x）からのみ利用する。
    #[allow(dead_code)]
    pub fn expire_if_elapsed(&mut self, elapsed: Duration) -> bool {
        if self.pending.is_some() && elapsed >= self.timeout {
            // 保留枠を放棄: areka 側保留解除＋host 側突合枠クリア（stale Complete を弾く）。
            self.pending = None;
            self.host_sink().clear_pending_token();
            true
        } else {
            false
        }
    }

    /// 脳をアンロードし、保留 request を取消する（議題3・要件 2.2）。
    ///
    /// `Unload` 前に areka 側・host 側の保留枠をクリアする（保留取消）。脳は `Unload` で host を
    /// Release する（議題2）。areka は脳解放前に必ず本メソッドを呼ぶ（teardown 順序・非循環所有）。
    ///
    /// # Errors
    /// `Unload` が失敗 HRESULT を返した場合 [`SessionError::Shiori`] を返す。
    ///
    /// # 非ブロッキング
    /// `Unload` は即時に戻る契約（議題3 e2）。
    pub fn unload(&mut self) -> Result<(), SessionError> {
        // 保留取消（議題3）: areka 側・host 側双方の保留枠をクリアしてから Unload する。
        self.pending = None;
        self.host_sink().clear_pending_token();
        self.brain.unload()?;
        Ok(())
    }

    /// host（`IShioriHost`）が包む [`ShioriHostSink`] 実体への参照を取り出す内部ヘルパ。
    ///
    /// 本セッションが `activate` で `ShioriHostSink` から構築した COM ポインタであるため、
    /// 実装実体が `ShioriHostSink` であることが保証される。
    fn host_sink(&self) -> &ShioriHostSink {
        // Safety: `self.host` は `activate` 内で `ShioriHostSink` から構築した COM ポインタであり、
        // 実装実体が `ShioriHostSink` であることが本セッションの不変条件として保証される。
        unsafe { windows_core::AsImpl::<ShioriHostSink>::as_impl(&self.host) }
    }
}

#[cfg(test)]
mod tests {
    //! in-proc アクティベーション経路＋利用規律の結合テスト（要件 1.5/2.4/5.1/5.2・design.md §System Flows）。
    //!
    //! `#[implement(IShiori)]` のモック脳（即時/遅延を切替可能）を in-proc で立て、以下を検証する:
    //! - (a) アクティベーションが脳へ到達し `Load` で sink を渡す（脳が host を保持し host->Raise が呼べる）。
    //! - (b) 遅延 request 後に次 request が拒否される（単一 in-flight・議題3）。
    //! - (c) タイムアウト判定ヘルパで保留枠が放棄され次 request が許可される（議題3 e1）。
    //! - (d) `Unload` で保留取消（議題3）。
    //! - (e) `Complete` 受領で保留解除→次 request 許可（要件 6.4 経路）。

    use super::*;
    use shiori_abi::error::SHIORI_S_PENDING;
    use shiori_abi::interface::IShiori_Impl;
    use std::cell::{Cell, RefCell};
    use windows_core::{HRESULT, Interface, implement};

    /// 遅延時に脳が発行する固定相関トークン。
    const PENDING_TOKEN: u64 = 0xABCD_1234;
    /// 即時応答で move-out する固定文字列。
    const IMMEDIATE_RESPONSE: &str = "immediate-body";

    /// `Request` の挙動を切り替えられるモック脳。`Load` で受け取った host を AddRef 保持する。
    ///
    /// - `behavior`: 即時 or 遅延。
    /// - 保持 host: `Load` で渡された `IShioriHost` を AddRef して保持し、`Unload` で Release する
    ///   （議題2・保持参照モデル）。脳→host の `Raise` 呼び出しに用いる。
    #[implement(IShiori)]
    struct MockBrain {
        deferred: bool,
        /// `Load` で受け取った host を保持する（AddRef 相当・`IShioriHost` は Rc/Arc 同様の COM 参照）。
        held_host: RefCell<Option<IShioriHost>>,
        loaded: Cell<bool>,
        unloaded: Cell<bool>,
    }

    impl MockBrain {
        fn new(deferred: bool) -> Self {
            Self {
                deferred,
                held_host: RefCell::new(None),
                loaded: Cell::new(false),
                unloaded: Cell::new(false),
            }
        }
    }

    impl IShiori_Impl for MockBrain_Impl {
        unsafe fn Load(&self, host: *mut core::ffi::c_void) -> HRESULT {
            // 受け取った raw host ポインタを型付き COM 参照として AddRef 保持する（議題2）。
            // Safety: host は areka が所有する有効な IShioriHost raw ポインタ（呼び出し中有効）。
            // from_raw_borrowed は借用ビューを返すため、保持するには clone で AddRef する。
            let borrowed: Option<&IShioriHost> =
                unsafe { IShioriHost::from_raw_borrowed(&host) };
            *self.held_host.borrow_mut() = borrowed.cloned();
            self.loaded.set(true);
            HRESULT(0) // S_OK
        }

        unsafe fn Unload(&self) -> HRESULT {
            // 保持 host を Release（drop）する（議題2・Unload 後は host を呼ばない）。
            *self.held_host.borrow_mut() = None;
            self.unloaded.set(true);
            HRESULT(0) // S_OK
        }

        unsafe fn Request(
            &self,
            _input: *const HSTRING,
            out_response: *mut HSTRING,
            out_token: *mut u64,
        ) -> HRESULT {
            if self.deferred {
                // 遅延: 相関トークンを out_token へ。out_response は空。
                unsafe { core::ptr::write(out_token, PENDING_TOKEN) };
                SHIORI_S_PENDING
            } else {
                // 即時: callee 確保の HSTRING を move-out（所有権規約）。
                unsafe { core::ptr::write(out_response, HSTRING::from(IMMEDIATE_RESPONSE)) };
                HRESULT(0) // S_OK
            }
        }
    }

    /// COM ポインタ経由で脳の保持 host->Raise を駆動するヘルパ（脳→host・vtable 直呼び）。
    ///
    /// # Safety
    /// `host` は有効な `IShioriHost` COM ポインタ、`script` は呼び出し中有効な `*const HSTRING`。
    unsafe fn call_raise(host: &IShioriHost, script: &HSTRING) -> HRESULT {
        unsafe { (Interface::vtable(host).Raise)(host.as_raw(), script as *const HSTRING) }
    }

    /// COM ポインタ経由で脳の保持 host->Complete を駆動するヘルパ（脳→host・vtable 直呼び）。
    ///
    /// # Safety
    /// `host` は有効な `IShioriHost` COM ポインタ、`response` は呼び出し中有効な `*const HSTRING`。
    unsafe fn call_complete(host: &IShioriHost, token: u64, response: &HSTRING) -> HRESULT {
        unsafe {
            (Interface::vtable(host).Complete)(host.as_raw(), token, response as *const HSTRING)
        }
    }

    /// モック脳の実体参照を取り出す（保持 host の観測用）。
    fn brain_inner(brain: &IShiori) -> &MockBrain {
        // Safety: `brain` は本テストで `MockBrain` から構築した COM ポインタ。
        unsafe { windows_core::AsImpl::<MockBrain>::as_impl(brain) }
    }

    /// セッションの host が包む [`ShioriHostSink`] 実体参照を取り出す（突合枠の観測用）。
    fn host_sink_of(session: &ShioriSession) -> &ShioriHostSink {
        // Safety: `session.host()` は `activate` 内で `ShioriHostSink` から構築した COM ポインタ。
        unsafe { windows_core::AsImpl::<ShioriHostSink>::as_impl(session.host()) }
    }

    /// (a) アクティベーションが in-proc 脳へ到達し `Load` で sink を渡すこと。
    /// 脳が host を保持し、脳→host の `Raise` が areka sink のメールボックスへ届く（要件 5.1/6.2）。
    #[test]
    fn activation_reaches_brain_and_loads_sink() {
        let brain: IShiori = MockBrain::new(false).into();
        let session = ShioriSession::activate(brain.clone()).expect("activate は成功すること");

        // 脳の `Load` が呼ばれ host を保持していること（議題2・AddRef 保持）。
        let inner = brain_inner(&brain);
        assert!(inner.loaded.get(), "脳の Load が呼ばれていること");
        assert!(
            inner.held_host.borrow().is_some(),
            "脳が host を AddRef 保持していること（Load で sink を受け渡した）"
        );

        // 脳が保持する host へ Raise を発行し、areka sink のメールボックスへ届くことを確認（要件 6.2/6.3）。
        let held = inner.held_host.borrow();
        let host = held.as_ref().expect("held host");
        let script = HSTRING::from("\\h\\s[0]from-brain");
        let hr = unsafe { call_raise(host, &script) };
        assert!(hr.is_ok(), "保持 host への Raise は S_OK");

        // session.host() == 脳が保持する host == 同一 areka sink（メールボックスを共有）。
        let drained = {
            // poll_completions は Raised も取り出す。
            let mut session = session;
            session.poll_completions()
        };
        assert_eq!(
            drained,
            vec![HostMessage::Raised(HSTRING::from("\\h\\s[0]from-brain"))],
            "脳→host->Raise が areka sink のメールボックスへ in-proc 到達していること"
        );
    }

    /// 即時 request は保留枠を使わず `Immediate` を返すこと（単一 in-flight 非該当）。
    #[test]
    fn immediate_request_does_not_set_pending() {
        let brain: IShiori = MockBrain::new(false).into();
        let mut session = ShioriSession::activate(brain).expect("activate");

        let content = HSTRING::from("ping");
        let outcome = session.request(&content).expect("即時 request は Ok");
        assert_eq!(
            outcome,
            SessionRequest::Immediate(HSTRING::from(IMMEDIATE_RESPONSE)),
            "即時応答が内容一致で返ること"
        );
        assert!(!session.is_pending(), "即時応答は保留枠を立てないこと");

        // 続けて次 request を出せること（単一 in-flight に抵触しない）。
        assert!(session.request(&content).is_ok(), "即時後は次 request 可能");
    }

    /// (b) 遅延 request 後、保留中は次 request が `RequestInFlight` で拒否されること（単一 in-flight・議題3）。
    #[test]
    fn deferred_request_blocks_next_request() {
        let brain: IShiori = MockBrain::new(true).into();
        let mut session = ShioriSession::activate(brain).expect("activate");

        let content = HSTRING::from("ping");
        let outcome = session.request(&content).expect("遅延 request は Ok");
        assert_eq!(
            outcome,
            SessionRequest::Deferred(CorrelationToken(PENDING_TOKEN)),
            "遅延はトークン付き Deferred を返すこと"
        );
        assert!(session.is_pending(), "遅延後は保留状態であること");
        assert_eq!(
            host_sink_of(&session).pending_token(),
            Some(CorrelationToken(PENDING_TOKEN)),
            "host 側突合枠にもトークンがセットされていること"
        );

        // 単一 in-flight: 保留中の次 request は拒否される（議題3）。
        let err = session
            .request(&content)
            .expect_err("保留中の次 request は拒否されること");
        assert!(
            matches!(err, SessionError::RequestInFlight),
            "拒否理由は RequestInFlight であること, got {err:?}"
        );
    }

    /// (c) タイムアウト判定ヘルパで保留枠が放棄され、次 request が許可されること（議題3 e1・決定的）。
    #[test]
    fn timeout_expires_pending_and_allows_next_request() {
        let timeout = Duration::from_millis(500);
        let brain: IShiori = MockBrain::new(true).into();
        let mut session =
            ShioriSession::activate_with_timeout(brain, timeout).expect("activate");

        let content = HSTRING::from("ping");
        session.request(&content).expect("遅延 request");
        assert!(session.is_pending(), "遅延後は保留状態");

        // タイムアウト未満の経過では放棄しない（決定的・実時間非依存）。
        assert!(
            !session.expire_if_elapsed(Duration::from_millis(499)),
            "タイムアウト未満では保留を放棄しないこと"
        );
        assert!(session.is_pending(), "未超過では保留が継続すること");

        // タイムアウト超過で保留枠を放棄し次 request を許可する（議題3 e1）。
        assert!(
            session.expire_if_elapsed(timeout),
            "タイムアウト超過で保留枠を放棄すること"
        );
        assert!(!session.is_pending(), "放棄後は保留状態が解除されること");

        // 次 request が可能になること（再び遅延を立てられる）。
        let outcome = session.request(&content).expect("放棄後は次 request 可能");
        assert_eq!(outcome, SessionRequest::Deferred(CorrelationToken(PENDING_TOKEN)));
    }

    /// タイムアウト放棄後に遅れて来た `Complete` は host が `SHIORI_E_UNKNOWN_TOKEN` で弾くこと（議題3）。
    #[test]
    fn stale_complete_after_timeout_is_rejected() {
        let timeout = Duration::from_millis(100);
        let brain: IShiori = MockBrain::new(true).into();
        let mut session =
            ShioriSession::activate_with_timeout(brain, timeout).expect("activate");

        let content = HSTRING::from("ping");
        session.request(&content).expect("遅延 request");
        assert!(session.expire_if_elapsed(timeout), "タイムアウトで放棄");

        // 放棄後に遅れて来た Complete は突合枠が空のため SHIORI_E_UNKNOWN_TOKEN。
        let response = HSTRING::from("late");
        let hr = unsafe { call_complete(session.host(), PENDING_TOKEN, &response) };
        assert_eq!(
            hr,
            shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN,
            "放棄後の stale Complete は SHIORI_E_UNKNOWN_TOKEN で弾かれること"
        );
    }

    /// (e) `Complete` 受領で保留が解除され、次 request が許可されること（要件 6.4 経路）。
    #[test]
    fn complete_releases_pending_and_allows_next_request() {
        let brain: IShiori = MockBrain::new(true).into();
        let mut session = ShioriSession::activate(brain).expect("activate");

        let content = HSTRING::from("ping");
        session.request(&content).expect("遅延 request");
        assert!(session.is_pending());

        // 脳→host->Complete（突合枠と一致）でメールボックスへ Completed を投函する。
        let response = HSTRING::from("deferred-body");
        let hr = unsafe { call_complete(session.host(), PENDING_TOKEN, &response) };
        assert!(hr.is_ok(), "一致トークンの Complete は S_OK");

        // poll で Completed を取り出し、保留が解除されること。
        let drained = session.poll_completions();
        assert_eq!(
            drained,
            vec![HostMessage::Completed {
                token: CorrelationToken(PENDING_TOKEN),
                response: HSTRING::from("deferred-body"),
            }],
            "Completed が突合トークン付きで取り出されること"
        );
        assert!(!session.is_pending(), "Complete 受領で保留が解除されること");

        // 次 request が可能になること。
        assert!(session.request(&content).is_ok(), "Complete 後は次 request 可能");
    }

    /// (d) `Unload` で保留が取消されること（議題3・要件 2.2）。
    #[test]
    fn unload_cancels_pending() {
        let brain: IShiori = MockBrain::new(true).into();
        let mut session = ShioriSession::activate(brain.clone()).expect("activate");

        let content = HSTRING::from("ping");
        session.request(&content).expect("遅延 request");
        assert!(session.is_pending(), "遅延後は保留状態");

        session.unload().expect("unload は成功すること");
        assert!(!session.is_pending(), "Unload で保留が取消されること");

        // 脳の Unload が呼ばれ host を Release していること（議題2・teardown）。
        let inner = brain_inner(&brain);
        assert!(inner.unloaded.get(), "脳の Unload が呼ばれていること");
        assert!(
            inner.held_host.borrow().is_none(),
            "脳が host を Release していること（Unload 後は host を呼ばない）"
        );
    }
}
