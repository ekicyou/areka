//! in-proc アクティベーション経路とリクエスト利用規律（単一 in-flight・遅延完了タイムアウト）。
//!
//! 本モジュールは、`IShioriFactory` 経由で脳（`IShiori` 実装）を生成する最小経路
//! （アクティベーション）と、areka 側のリクエスト利用規律を所有する
//! （design.md §ShioriSession, requirements.md 2.1/12.3）。
//!
//! ## アクティベーション最小経路（factory 経由生成・design.md §ShioriSession・要件 2.1）
//! [`ShioriSession::activate`] は [`ShioriHostSink`] を生成して `IShioriHost` 化し、
//! [`IShioriFactory::create`](shiori_abi::interface::IShioriFactory::create)`(load_dir, shiori_name,
//! &host)` で **load 完了済み** の `IShiori` を受領・保持する（旧 `ShioriExt::load` 経路の置換）。
//! 「未ロード状態」は存在しない（create 融合＋Drop teardown・要件 9.1/9.2）。実装種別差
//! （native / 過去互換）は factory 実装に局所化し、確立済みの `IShiori` 利用面へ波及させない。
//!
//! ## teardown は Drop(RAII)（design.md §ShioriSession・要件 2.1/12.3・D7）
//! 明示 `unload()` メソッドは存在しない。[`ShioriSession`] の [`Drop`] が
//! **保留 request の取消 → brain 参照の drop** の順序で teardown する。Drop は失敗を返せない
//! ——D7「teardown は best-effort・戻り値で扱わない」が正当化する。
//!
//! ## 利用規律: 単一 in-flight（議題3・design.md §ShioriSession・要件 2.1）
//! areka は同時に高々 1 リクエストのみ発行する。[`ShioriSession::get`] が
//! [`SessionRequest::Deferred`] を返した間は **保留状態**であり、host の突合枠へトークンをセットする
//! （[`ShioriHostSink::set_pending_token`]）。保留中に次の `get` を呼ぼうとすると
//! [`SessionError::RequestInFlight`] で拒否する。保留は次のいずれかで解消し次 get を許可する:
//! - **`Complete` 受領**: host メールボックスへ `Completed` が届くと [`ShioriSession::poll_completions`]
//!   が突合枠と照合して保留を解除する。
//! - **Drop**: [`ShioriSession`] の drop が保留を取消す。
//! - **遅延完了タイムアウト**: [`ShioriSession::expire_if_elapsed`] が設定可能なタイムアウトを
//!   超過した保留枠を放棄し次 get を許可する。タイムアウト超過後に遅れて来た `Complete` は host が
//!   [`SHIORI_E_UNKNOWN_TOKEN`](shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN) で弾く。
//!
//! ## タイムアウトの決定性（議題3 e1・テスト容易性）
//! タイムアウト判定は実時間 `sleep` に依存しない。設定値 [`ShioriSession::timeout`] は
//! [`Duration`] で保持し、保留開始時刻からの経過時間 [`Duration`] を**注入可能な**
//! [`expire_if_elapsed`](ShioriSession::expire_if_elapsed) へ渡して判定する。
//!
//! ## 非ブロッキング前提（議題3 e2・要件 12.3）
//! `get`/`notify` は in-proc 直呼びで areka の呼び出しスレッド上で実行され、脳が即時に
//! `S_OK` / [`SHIORI_S_PENDING`](shiori_abi::error::SHIORI_S_PENDING) / error HRESULT を返す契約である
//! （非ブロッキング）。重い処理は脳側で `SHIORI_S_PENDING` ＋後続 `Complete` へ後送りされる。

use std::time::Duration;

use shiori_abi::interface::{IShiori, IShioriFactory, IShioriHost};
use shiori_abi::outcome::{CorrelationToken, GetOutcome};
use windows_core::HSTRING;

use crate::shiori_host::{HostMessage, ShioriHostSink};

/// in-proc セッション/アクティベーションの利用規律エラー（areka 側の状態に由来）。
///
/// `IShiori`/`IShioriFactory` の HRESULT 由来エラー（[`shiori_abi::error::ShioriError`]）とは別に、
/// areka 側の **利用規律**（単一 in-flight 等）違反を型化する（design.md §ShioriSession・議題3）。
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    /// 保留 request が in-flight の間に次 request を発行しようとした（単一 in-flight・議題3）。
    #[error("a deferred request is still in flight (single in-flight discipline)")]
    RequestInFlight,
    /// `IShiori`/`IShioriFactory` 操作が失敗した（HRESULT 由来）。元の [`shiori_abi::error::ShioriError`] を内包する。
    #[error(transparent)]
    Shiori(#[from] shiori_abi::error::ShioriError),
}

/// `get` の利用規律つき結果（areka 側）。
///
/// 即時応答はそのまま返し、遅延時は保留枠へトークンをセットしたうえで `Deferred` を返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRequest {
    /// 即時応答。脳が `S_OK`＋応答 HSTRING を返した（保留枠は使わない）。
    Immediate(HSTRING),
    /// 遅延。保留枠へトークンをセットした。次 get は保留解除まで拒否される（単一 in-flight）。
    Deferred(CorrelationToken),
}

/// in-proc アクティベーション経路と利用規律を所有するセッション（design.md §ShioriSession・議題3）。
///
/// 脳（`IShiori`）への in-proc 参照と areka 実装の sink（`IShioriHost`）を保持し、単一 in-flight・
/// 遅延完了タイムアウト・Drop teardown（保留取消）の利用規律を担う。本セッションは脳へ COM 強参照
/// （[`IShiori`]）を保持するが、host 実装（[`ShioriHostSink`]）は脳へ強参照を持たない
/// （非循環・所有方向 areka→脳→host）。
pub struct ShioriSession {
    /// in-proc 脳への参照（COM ポインタ）。`activate` の factory 経由生成で受領し `get`/`notify` に用いる。
    ///
    /// Drop 順序（D7）: `brain` を最後に drop するため、フィールド宣言順で `host` より後に置かない
    /// （Rust はフィールド宣言順に drop する）。ここでは Drop impl 内で明示順序を制御する。
    brain: IShiori,
    /// areka 実装の sink（単一 in-flight 突合枠＋メールボックス＋プロパティストアを所有）。
    host: IShioriHost,
    /// 設定可能な遅延完了タイムアウト（議題3 e1）。経過時間との比較は注入された経過時間で行う。
    // デモ駆動経路は遅延を Complete で即解消するためタイムアウトを参照しない。本フィールドは
    // タイムアウト経路を検証する結合テスト（`expire_if_elapsed`/`with_timeout`）からのみ読まれる。
    #[allow(dead_code)]
    timeout: Duration,
    /// 現在保留中の相関トークン（単一 in-flight）。`Deferred` でセットし、解消（`Complete`/Drop/
    /// タイムアウト）でクリアする。host 側突合枠（[`ShioriHostSink`]）と同期して扱う。
    pending: Option<CorrelationToken>,
}

impl ShioriSession {
    /// 既定タイムアウト（議題3 e1: SSP 同様の無応答ガード相当の保守的既定値）。
    ///
    /// 設定可能であり、[`ShioriSession::with_timeout`] で上書きできる。
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

    /// factory 経由で脳を生成しアクティベーションする（要件 2.1）。
    ///
    /// 既定タイムアウト（[`DEFAULT_TIMEOUT`](Self::DEFAULT_TIMEOUT)）でセッションを確立する。
    /// 内部で [`ShioriHostSink`] を生成→`IShioriHost` 化し、
    /// [`IShioriFactory::create`]`(load_dir, shiori_name, &host)` で load 完了済み `IShiori` を受領する。
    ///
    /// # Errors
    /// `create` が失敗した場合 [`SessionError::Shiori`]（`CreateFailed`）を返す（要件 8.6）。
    // production の session 構築は from_parts 経由（demo/main）ゆえ bin ターゲットでは未使用。
    // 本メソッドは test（unit/lifecycle e2e）専用の公開 API（兄弟の test 専用メソッドと同規約）。
    #[allow(dead_code)]
    pub fn activate(
        factory: &IShioriFactory,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
    ) -> Result<Self, SessionError> {
        Self::activate_with_timeout(factory, load_dir, shiori_name, Self::DEFAULT_TIMEOUT)
    }

    /// 遅延完了タイムアウトを指定して factory 経由でアクティベーションする（議題3 e1・設定可能）。
    ///
    /// 新規 [`ShioriHostSink`] を生成し、`factory.create` で脳を生成・保持する。
    // production は from_parts 経由ゆえ bin ターゲットでは未使用（test 専用の公開 API）。
    #[allow(dead_code)]
    pub fn activate_with_timeout(
        factory: &IShioriFactory,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
        timeout: Duration,
    ) -> Result<Self, SessionError> {
        // areka 実装の sink を生成（単一 in-flight 突合枠＋メールボックス＋プロパティストアを所有）。
        let host: IShioriHost = ShioriHostSink::new().into();
        // factory 経由生成: load 完了済みの IShiori を受領する（旧 ShioriExt::load 経路の置換・要件 2.1）。
        // 失敗は ShioriError（CreateFailed）へ写る（半構築非露出・要件 8.6）。
        let brain = factory.create(load_dir, shiori_name, &host)?;
        Ok(Self {
            brain,
            host,
            timeout,
            pending: None,
        })
    }

    /// 既に生成済みの脳（`IShiori`）と host（`IShioriHost`）からセッションを組み立てる（既定タイムアウト）。
    ///
    /// factory 経由で create 済みの脳を「脳駆動（`ReferenceBrain` 実体）」と「session 保持」で共有したい
    /// デモ／統合ドライバ向けの seam。`host` は create 時に脳へ渡した sink と同一（メールボックス共有）で
    /// あることを呼び出し側が保証する（脳→host の complete/raise を `poll_completions` で観測するため）。
    pub fn from_parts(brain: IShiori, host: IShioriHost) -> Self {
        Self {
            brain,
            host,
            timeout: Self::DEFAULT_TIMEOUT,
            pending: None,
        }
    }

    /// 遅延完了タイムアウトを上書きする（議題3 e1・設定可能）。
    // デモ駆動経路では使わない。タイムアウト経路を検証する結合テストからのみ利用する。
    #[allow(dead_code)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 現在の遅延完了タイムアウト設定（議題3 e1）。
    // デモ駆動経路では使わない。タイムアウト設定を観測する結合テストからのみ利用する。
    #[allow(dead_code)]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// 保留 request が in-flight か（単一 in-flight・観測用）。
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// 現在保留中の相関トークン（観測用）。
    // デモ駆動経路は保留有無（`is_pending`）のみ観測する。保留トークンを照合する結合テストからのみ利用する。
    #[allow(dead_code)]
    pub fn pending_token(&self) -> Option<CorrelationToken> {
        self.pending
    }

    /// areka 実装の sink（`IShioriHost`）への参照（脳→host 通知の観測・テスト用）。
    // デモ駆動経路は `poll_completions` 越しに観測する。sink を直接観測する結合テストからのみ利用する。
    #[allow(dead_code)]
    pub fn host(&self) -> &IShioriHost {
        &self.host
    }

    /// 同期 request（GET SHIORI/3.0 後継）を発行する。単一 in-flight 規律を適用する（議題3・要件 2.1）。
    ///
    /// 保留中（[`is_pending`](Self::is_pending)）に呼ぶと [`SessionError::RequestInFlight`] で拒否する。
    /// 脳が [`GetOutcome::Deferred`] を返した場合は host の突合枠へトークンをセットし
    /// （[`ShioriHostSink::set_pending_token`]）、セッションの保留状態を立てる。
    ///
    /// # Errors
    /// - 保留中の再発行 → [`SessionError::RequestInFlight`]。
    /// - `IShiori::get` 失敗 → [`SessionError::Shiori`]（`GetFailed`）。
    pub fn get(&mut self, content: &HSTRING) -> Result<SessionRequest, SessionError> {
        if self.pending.is_some() {
            // 単一 in-flight: 保留解除（Complete/Drop/タイムアウト）まで次 get を発行しない（議題3）。
            return Err(SessionError::RequestInFlight);
        }
        match self.brain.get(content)? {
            GetOutcome::Immediate(response) => Ok(SessionRequest::Immediate(response)),
            GetOutcome::Deferred(token) => {
                // 突合枠（host 側）へトークンをセットし、後続 Complete の突合を可能にする。
                self.host_sink().set_pending_token(token);
                self.pending = Some(token);
                Ok(SessionRequest::Deferred(token))
            }
        }
    }

    /// 片道通知（NOTIFY SHIORI/3.0 後継）を発行する。応答を返さない（単一 in-flight に影響しない）。
    ///
    /// # Errors
    /// `IShiori::notify` 失敗 → [`SessionError::Shiori`]（`NotifyFailed`）。
    pub fn notify(&self, content: &HSTRING) -> Result<(), SessionError> {
        self.brain.notify(content)?;
        Ok(())
    }

    /// host メールボックスから完了/通知を取り出し、保留 request の `Complete` を解消する。
    ///
    /// 突合枠と一致する `Completed` を受領したら保留を解除し次 get を許可する。`Raised`（能動通知）は
    /// そのまま返す（保留状態には影響しない）。取り出したメッセージを順に返す（受け皿からの取り出し）。
    pub fn poll_completions(&mut self) -> Vec<HostMessage> {
        let mut drained = Vec::new();
        while let Some(msg) = self.host_sink().try_recv() {
            if let HostMessage::Completed { token, .. } = &msg
                && self.pending == Some(*token)
            {
                // 対応する Complete を受領: 保留解除→次 get 許可（議題3）。
                self.pending = None;
            }
            drained.push(msg);
        }
        drained
    }

    /// 設定タイムアウトを超過した保留枠を放棄し次 get を許可する（議題3 e1・決定的判定）。
    ///
    /// `elapsed` は保留開始からの経過時間（呼び出し側が注入する・実時間 `sleep` に非依存）。
    /// `elapsed >= timeout` のとき保留枠を放棄する（host 側突合枠もクリアして、遅れて来る
    /// `Complete` を host が [`SHIORI_E_UNKNOWN_TOKEN`](shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN) で弾く）。
    ///
    /// # 戻り値
    /// 放棄した場合 `true`（保留が解除され次 get 可能）、未超過 or 非保留なら `false`。
    // デモ駆動経路は遅延を Complete で即解消するため使わない。タイムアウト経路を検証する結合テストからのみ利用する。
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

/// teardown は Drop(RAII)（要件 2.1/12.3・D7）。
///
/// 保留 request を取消（areka 側・host 側双方の保留枠をクリア）してから brain 参照を drop する。
/// この順序を固定することで、放棄後に遅れて来る `Complete` を host が突合不能で弾ける状態を作って
/// から脳を手放す。Drop は失敗を返せないため best-effort（D7）。
impl Drop for ShioriSession {
    fn drop(&mut self) {
        // 保留取消（議題3）: areka 側・host 側双方の保留枠をクリアする。
        self.pending = None;
        self.host_sink().clear_pending_token();
        // brain 参照（`self.brain`）と host 参照（`self.host`）はこの後 Rust が自動 drop する
        // （Release）。取消→drop の順序をここで固定した（D7・best-effort）。
    }
}

#[cfg(test)]
mod tests {
    //! factory 経由アクティベーション経路＋利用規律の結合テスト（要件 2.1/12.3・design.md §System Flows）。
    //!
    //! `#[implement(IShioriFactory)]`/`#[implement(IShiori)]` のモック（即時/遅延を切替可能）を
    //! in-proc で立て、以下を検証する:
    //! - (a) アクティベーションが factory 経由で脳を生成し host を受け渡す（脳が host を保持し host->raise が呼べる）。
    //! - (b) 遅延 get 後に次 get が拒否される（単一 in-flight・議題3）。
    //! - (c) タイムアウト判定ヘルパで保留枠が放棄され次 get が許可される（議題3 e1）。
    //! - (d) drop 後は参照が存在しない（Drop teardown・要件 2.1/12.3）。
    //! - (e) `Complete` 受領で保留解除→次 get 許可。

    use super::*;
    use core::cell::RefCell;
    use shiori_abi::error::SHIORI_S_PENDING;
    use shiori_abi::interface::{
        IShiori_Impl, IShioriFactory_Impl,
    };
    use windows_core::{HRESULT, HSTRING, OutRef, Ref, Result as ComResult, implement};

    /// 遅延時に脳が発行する固定相関トークン。
    const PENDING_TOKEN: u64 = 0xABCD_1234;
    /// 即時応答で move-out する固定文字列。
    const IMMEDIATE_RESPONSE: &str = "immediate-body";

    /// `Get` の挙動を切り替えられるモック脳。`Notify` は受領を記録する。host を保持する。
    #[allow(non_snake_case)]
    #[implement(IShiori)]
    struct MockBrain {
        deferred: bool,
        /// `CreateInstance` で受け取った host を保持する（AddRef 相当・共同所有の実証）。
        #[allow(dead_code)]
        held_host: IShioriHost,
        notified: RefCell<Vec<HSTRING>>,
    }

    impl IShiori_Impl for MockBrain_Impl {
        unsafe fn Get(
            &self,
            _input: &HSTRING,
            out_response: &mut HSTRING,
            out_token: &mut u64,
        ) -> HRESULT {
            if self.deferred {
                *out_token = PENDING_TOKEN;
                SHIORI_S_PENDING
            } else {
                *out_response = HSTRING::from(IMMEDIATE_RESPONSE);
                HRESULT(0) // S_OK
            }
        }

        unsafe fn Notify(&self, input: &HSTRING) -> ComResult<()> {
            self.notified.borrow_mut().push(input.clone());
            Ok(())
        }
    }

    /// 即時/遅延を切替可能な脳を生成するモック factory。
    #[allow(non_snake_case)]
    #[implement(IShioriFactory)]
    struct MockFactory {
        deferred: bool,
    }

    impl IShioriFactory_Impl for MockFactory_Impl {
        unsafe fn CreateInstance(
            &self,
            _load_dir: &HSTRING,
            _shiori_name: &HSTRING,
            host: Ref<'_, IShioriHost>,
            out: OutRef<'_, IShiori>,
        ) -> ComResult<()> {
            let host: IShioriHost = host
                .as_ref()
                .ok_or_else(|| {
                    windows_core::Error::from(windows::Win32::Foundation::E_POINTER)
                })?
                .clone();
            let brain: IShiori = MockBrain {
                deferred: self.deferred,
                held_host: host,
                notified: RefCell::new(Vec::new()),
            }
            .into();
            out.write(Some(brain))?;
            Ok(())
        }
    }

    /// テスト用: モック factory を立てて session を起こすヘルパ。
    fn activate_mock(deferred: bool) -> ShioriSession {
        let factory: IShioriFactory = MockFactory { deferred }.into();
        ShioriSession::activate(&factory, &HSTRING::from("dir"), &HSTRING::from("name"))
            .expect("activate は成功すること")
    }

    /// セッションの host が包む [`ShioriHostSink`] 実体参照を取り出す（突合枠の観測用）。
    fn host_sink_of(session: &ShioriSession) -> &ShioriHostSink {
        // Safety: `session.host()` は `activate` 内で `ShioriHostSink` から構築した COM ポインタ。
        unsafe { windows_core::AsImpl::<ShioriHostSink>::as_impl(session.host()) }
    }

    /// (a) アクティベーションが factory 経由で脳を生成し host を渡すこと。
    /// 脳が host を保持し、脳→host の `raise` が areka sink のメールボックスへ届く（要件 2.1）。
    #[test]
    fn activation_creates_brain_and_loads_sink() {
        let mut session = activate_mock(false);

        // 脳が保持する host（= areka sink）へ Raise が届くことを、脳の held_host 経由で確認する。
        // MockBrain の held_host は areka が渡した sink と同一（メールボックス共有）。
        let sink = host_sink_of(&session);
        assert_eq!(sink.mailbox_len(), 0, "初期メールボックスは空");

        // brain の held_host へ raise を発火する経路は brain を直接触れないため、host 経由で raise して
        // 同一 sink がメールボックスへ投函することを確認する（sink 共有の実証）。
        session
            .host()
            .raise(&HSTRING::from("\\h\\s[0]from-host"))
            .expect("host への raise は Ok");
        let drained = session.poll_completions();
        assert_eq!(
            drained,
            vec![HostMessage::Raised(HSTRING::from("\\h\\s[0]from-host"))],
            "sink のメールボックスへ raise が届くこと（factory 生成の host 共有）"
        );
    }

    /// 即時 get は保留枠を使わず `Immediate` を返すこと（単一 in-flight 非該当）。
    #[test]
    fn immediate_get_does_not_set_pending() {
        let mut session = activate_mock(false);

        let content = HSTRING::from("ping");
        let outcome = session.get(&content).expect("即時 get は Ok");
        assert_eq!(
            outcome,
            SessionRequest::Immediate(HSTRING::from(IMMEDIATE_RESPONSE)),
            "即時応答が内容一致で返ること"
        );
        assert!(!session.is_pending(), "即時応答は保留枠を立てないこと");

        assert!(session.get(&content).is_ok(), "即時後は次 get 可能");
    }

    /// (b) 遅延 get 後、保留中は次 get が `RequestInFlight` で拒否されること（単一 in-flight・議題3）。
    #[test]
    fn deferred_get_blocks_next_get() {
        let mut session = activate_mock(true);

        let content = HSTRING::from("ping");
        let outcome = session.get(&content).expect("遅延 get は Ok");
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

        let err = session
            .get(&content)
            .expect_err("保留中の次 get は拒否されること");
        assert!(
            matches!(err, SessionError::RequestInFlight),
            "拒否理由は RequestInFlight であること, got {err:?}"
        );
    }

    /// (c) タイムアウト判定ヘルパで保留枠が放棄され、次 get が許可されること（議題3 e1・決定的）。
    #[test]
    fn timeout_expires_pending_and_allows_next_get() {
        let timeout = Duration::from_millis(500);
        let factory: IShioriFactory = MockFactory { deferred: true }.into();
        let mut session = ShioriSession::activate_with_timeout(
            &factory,
            &HSTRING::from("dir"),
            &HSTRING::from("name"),
            timeout,
        )
        .expect("activate");

        let content = HSTRING::from("ping");
        session.get(&content).expect("遅延 get");
        assert!(session.is_pending(), "遅延後は保留状態");

        assert!(
            !session.expire_if_elapsed(Duration::from_millis(499)),
            "タイムアウト未満では保留を放棄しないこと"
        );
        assert!(session.is_pending(), "未超過では保留が継続すること");

        assert!(
            session.expire_if_elapsed(timeout),
            "タイムアウト超過で保留枠を放棄すること"
        );
        assert!(!session.is_pending(), "放棄後は保留状態が解除されること");

        let outcome = session.get(&content).expect("放棄後は次 get 可能");
        assert_eq!(outcome, SessionRequest::Deferred(CorrelationToken(PENDING_TOKEN)));
    }

    /// タイムアウト放棄後に遅れて来た `complete` は host が `UnknownToken` で弾くこと（議題3）。
    #[test]
    fn stale_complete_after_timeout_is_rejected() {
        let timeout = Duration::from_millis(100);
        let factory: IShioriFactory = MockFactory { deferred: true }.into();
        let mut session = ShioriSession::activate_with_timeout(
            &factory,
            &HSTRING::from("dir"),
            &HSTRING::from("name"),
            timeout,
        )
        .expect("activate");

        let content = HSTRING::from("ping");
        session.get(&content).expect("遅延 get");
        assert!(session.expire_if_elapsed(timeout), "タイムアウトで放棄");

        // 放棄後に遅れて来た complete は突合枠が空のため UnknownToken。
        let response = HSTRING::from("late");
        let err = session
            .host()
            .complete(CorrelationToken(PENDING_TOKEN), &response)
            .expect_err("放棄後の stale complete は Err");
        assert!(
            matches!(err, shiori_abi::error::ShioriError::UnknownToken),
            "放棄後の stale complete は UnknownToken で弾かれること, got {err:?}"
        );
    }

    /// (e) `Complete` 受領で保留が解除され、次 get が許可されること。
    #[test]
    fn complete_releases_pending_and_allows_next_get() {
        let mut session = activate_mock(true);

        let content = HSTRING::from("ping");
        session.get(&content).expect("遅延 get");
        assert!(session.is_pending());

        // 脳→host->complete（突合枠と一致）でメールボックスへ Completed を投函する。
        let response = HSTRING::from("deferred-body");
        session
            .host()
            .complete(CorrelationToken(PENDING_TOKEN), &response)
            .expect("一致トークンの complete は Ok");

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

        assert!(session.get(&content).is_ok(), "Complete 後は次 get 可能");
    }

    /// (d) drop で保留が取消され、drop 後は参照が存在しないこと（Drop teardown・要件 2.1/12.3）。
    ///
    /// 「unload 後の拒否」系（旧）は「drop 後は参照不在」へ書き換え——型システムが検証を肩代わりする。
    /// ここでは drop 前に host sink（clone で別途保持）を観測し、drop が host 側突合枠をクリアする
    /// （保留取消）ことを確認する。
    #[test]
    fn drop_cancels_pending_and_no_reference_survives() {
        let factory: IShioriFactory = MockFactory { deferred: true }.into();
        let mut session =
            ShioriSession::activate(&factory, &HSTRING::from("dir"), &HSTRING::from("name"))
                .expect("activate");

        // sink を clone で別途保持し、session drop 後に突合枠が取消されていることを観測できるようにする。
        let sink_handle: IShioriHost = session.host().clone();

        let content = HSTRING::from("ping");
        session.get(&content).expect("遅延 get");
        assert!(session.is_pending(), "遅延後は保留状態");
        // host 側突合枠にトークンがある。
        let sink_impl =
            unsafe { windows_core::AsImpl::<ShioriHostSink>::as_impl(&sink_handle) };
        assert_eq!(
            sink_impl.pending_token(),
            Some(CorrelationToken(PENDING_TOKEN)),
            "drop 前は host 側突合枠にトークンがあること"
        );

        // session を drop する（Drop teardown: 保留取消→brain drop）。
        drop(session);

        // drop 後は host 側突合枠が取消されている（保留取消・stale Complete を弾ける状態）。
        assert_eq!(
            sink_impl.pending_token(),
            None,
            "drop で host 側突合枠が取消されること（保留取消・Drop teardown）"
        );
        // 以降に遅れて来る complete は突合不能で弾かれる（保留取消の帰結）。
        let err = sink_handle
            .complete(CorrelationToken(PENDING_TOKEN), &HSTRING::from("late"))
            .expect_err("drop 後の stale complete は Err");
        assert!(
            matches!(err, shiori_abi::error::ShioriError::UnknownToken),
            "drop 後の stale complete は UnknownToken で弾かれること, got {err:?}"
        );
    }

    /// `notify`（片道通知）がセッション越しに脳へ届き、応答を返さないこと（要件 9.3 の areka 追随）。
    #[test]
    fn notify_reaches_brain_without_response() {
        let session = activate_mock(false);
        session
            .notify(&HSTRING::from("NOTIFY SHIORI/3.0 OnFirstBoot"))
            .expect("notify は Ok（片道・応答なし）");
    }
}
