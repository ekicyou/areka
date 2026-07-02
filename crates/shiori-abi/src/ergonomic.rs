//! snake_case インヘレント安全面（SafeSurface・design.md §SafeSurface）。
//!
//! 旧 `ShioriExt` 拡張トレイト（raw `unsafe fn -> HRESULT` を `Result` へ変換する層）は Task 8.1 で
//! 撤去された。本モジュールは Task 8.3 で「vtable 面（PascalCase）＝[`crate::interface`]／安全面
//! （snake_case）＝ここ」という **2 面インヘレント impl** として安全面を再構築する。
//!
//! # 2 面インヘレント（design.md §SafeSurface・R12.5）
//! [`crate::interface`] の `#[interface]` は各 COM 境界を PascalCase の pub vtable メソッド
//! （`CreateInstance`/`Get`/`Notify`/`Raise`/`Complete`/`GetProperty`/`SetProperty`）として生成する。
//! 本モジュールは同一クレート内の **第 2 インヘレント impl** を snake_case で追加し、`OutRef` 受け皿・
//! HRESULT 判別・error 写像を内包した `Result` 直返し API を提供する。PascalCase と snake_case は
//! 命名衝突しないため `use` 儀式（拡張トレイトの import）は不要で、consumer から raw ポインタ・
//! HRESULT・`unsafe` は一切見えない（R11.1）。`ShioriExt` トレイトは**復活させない**（R12.5）。
//!
//! # 提供する安全面（design.md §SafeSurface Service Interface）
//! - [`IShioriFactory::create`]: `OutRef` 受け皿を隠蔽し load 完了済み [`IShiori`] を直返し（R8.2）。
//!   失敗は [`ShioriError::CreateFailed`]（半構築非露出・R8.6）。
//! - [`IShiori::get`]: HRESULT 3 分岐を [`GetOutcome`] へ復元（R9.4）。
//! - [`IShiori::notify`]: 片道通知（失敗は [`ShioriError::NotifyFailed`]）。
//! - [`IShioriHost::raise`]/[`complete`](IShioriHost::complete)/[`get_property`](IShioriHost::get_property)/
//!   [`set_property`](IShioriHost::set_property): host sink の安全面。
//!
//! # HRESULT マッピング規約（design.md §SafeSurface Invariants・R9.4）
//! [`get`](IShiori::get) は raw `Get` の HRESULT を次の**固定順序**で振り分ける（旧 `ergonomic.rs` の
//! マッピングロジックを流用）:
//! 1. [`SHIORI_S_PENDING`] → [`GetOutcome::Deferred`]（`out_token` を [`CorrelationToken`] へ）。
//! 2. `is_ok()` → [`GetOutcome::Immediate`]（`out_response` を move-out して所有）。
//! 3. その他失敗 → [`ShioriError::GetFailed`]（HRESULT 内包・R9.4）。
//!
//! **必ず [`SHIORI_S_PENDING`] 判別を `is_ok()` より先に行う**。[`SHIORI_S_PENDING`] も成功コード
//! （severity=0）ゆえ、順序を誤ると遅延を即時へ誤分類する。

use windows_core::{HSTRING, Ref};

use crate::error::{SHIORI_S_PENDING, ShioriError};
use crate::interface::{IShiori, IShioriFactory, IShioriHost};
use crate::outcome::{CorrelationToken, GetOutcome};

impl IShioriFactory {
    /// 脳を生成し load 完了済みの [`IShiori`] を直返しする（生成＋load 融合・R8.2/8.3）。
    ///
    /// `OutRef` 受け皿の組み立てと取り出しを内包し、consumer から `OutRef`/HRESULT/`unsafe` を隠蔽する。
    /// 失敗時は out から何も取り出さず [`ShioriError::CreateFailed`] を返す（半構築を露出しない・R8.6）。
    ///
    /// - `load_dir`: ロードディレクトリ（借用・callee 保持は clone）。
    /// - `shiori_name`: SHIORI 名（借用・callee 保持は clone）。
    /// - `host`: areka 本体が実装する sink（借用・callee clone/AddRef で共同所有）。
    pub fn create(
        &self,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
        host: &IShioriHost,
    ) -> Result<IShiori, ShioriError> {
        // `OutRef` 受け皿を組み立てる（`Option<IShiori>` の可変借用を OutRef 化）。
        let mut out: Option<IShiori> = None;
        // Safety: pub vtable メソッド `CreateInstance` の呼出。
        // - `self` は有効な `IShioriFactory` COM ポインタ（借用で生存）。
        // - `load_dir`/`shiori_name` は `[in]` 借用で呼び出しを覆って生存する。
        // - `host` は `Ref` 借用（`(&host).into()`）で AddRef されず、呼び出し中有効。
        // - `out` は本層がスタックに確保した書き込み可能領域（`(&mut out).into()` で OutRef 化）。
        let result = unsafe {
            self.CreateInstance(load_dir, shiori_name, Ref::from(Some(host)), (&mut out).into())
        };
        match result {
            // 成功時のみ out から load 完了済み IShiori を取り出す。
            Ok(()) => out.ok_or(ShioriError::CreateFailed(windows_core::HRESULT(0))),
            // 失敗は CreateFailed（out からは何も取り出さない＝半構築非露出・R8.6）。
            Err(e) => Err(ShioriError::CreateFailed(e.code())),
        }
    }
}

impl IShiori {
    /// 同期 request を [`GetOutcome`] へ復元する（R9.4・design.md §SafeSurface Invariants）。
    ///
    /// raw `Get` の HRESULT を 3 分岐する（**[`SHIORI_S_PENDING`] 判別が `is_ok()` より先**）:
    /// - [`SHIORI_S_PENDING`] → [`GetOutcome::Deferred`]（`out_token` を [`CorrelationToken`] へ）。
    /// - `is_ok()`（`S_OK`）→ [`GetOutcome::Immediate`]（`out_response` を move-out して所有）。
    /// - その他失敗 → [`ShioriError::GetFailed`]（HRESULT 内包）。
    ///
    /// - `input`: 正準 content（借用・callee は読み取りのみで解放しない）。
    pub fn get(&self, input: &HSTRING) -> Result<GetOutcome, ShioriError> {
        // [out] 受け皿。`S_OK` 時に callee が `out_response` へ move-out（本層が所有・解放）。
        let mut out_response = HSTRING::new();
        // 遅延時に callee が相関トークンを書き込む受け皿（`SHIORI_S_PENDING` 時のみ有効）。
        let mut out_token: u64 = 0;
        // Safety: pub vtable メソッド `Get` の呼出。
        // - `self` は有効な `IShiori` COM ポインタ（借用で生存）。
        // - `input` は `[in]` 借用で呼び出しを覆って生存する。
        // - `out_response`/`out_token` は本層がスタックに確保した書き込み可能領域。
        let hr = unsafe { self.Get(input, &mut out_response, &mut out_token) };

        // 固定順序: SHIORI_S_PENDING → is_ok() → else（成功コード判別を誤らないため順序必須）。
        if hr == SHIORI_S_PENDING {
            // 遅延: `out_token` を相関トークンへ（`out_response` は空＝未書き込み）。
            Ok(GetOutcome::Deferred(CorrelationToken(out_token)))
        } else if hr.is_ok() {
            // 即時（`S_OK`）: `out_response` を move-out して所有（Drop は本層呼び出し側）。
            Ok(GetOutcome::Immediate(out_response))
        } else {
            // その他失敗（R9.4）: HRESULT を内包する文脈変種へ。
            Err(ShioriError::GetFailed(hr))
        }
    }

    /// 片道通知（応答なし・R9.4）。失敗は [`ShioriError::NotifyFailed`]（HRESULT 内包）。
    ///
    /// - `input`: 通知 content（借用・callee 保持は clone）。
    pub fn notify(&self, input: &HSTRING) -> Result<(), ShioriError> {
        // Safety: pub vtable メソッド `Notify` の呼出。`self` は有効な `IShiori` COM ポインタ、
        // `input` は `[in]` 借用で呼び出しを覆って生存する。
        match unsafe { self.Notify(input) } {
            Ok(()) => Ok(()),
            Err(e) => Err(ShioriError::NotifyFailed(e.code())),
        }
    }
}

impl IShioriHost {
    /// 能動通知（wakeup）。失敗は [`ShioriError::Com`]（HRESULT 内包）。
    ///
    /// - `script`: 通知内容（借用・host 保持は clone）。
    pub fn raise(&self, script: &HSTRING) -> Result<(), ShioriError> {
        // Safety: pub vtable メソッド `Raise` の呼出。`self` は有効な `IShioriHost` COM ポインタ、
        // `script` は `[in]` 借用で呼び出しを覆って生存する。
        unsafe { self.Raise(script) }.map_err(ShioriError::from)
    }

    /// 遅延リクエストの完了応答を配送する（R10.1）。
    ///
    /// 未知/stale トークンは [`ShioriError::UnknownToken`]（[`crate::error::SHIORI_E_UNKNOWN_TOKEN`]
    /// の型付き写像）、他は [`ShioriError::Com`]。
    ///
    /// - `token`: 相関トークン（[`get`](IShiori::get) が遅延時に返した値）。
    /// - `response`: 応答内容（借用・host 保持は clone）。
    pub fn complete(
        &self,
        token: CorrelationToken,
        response: &HSTRING,
    ) -> Result<(), ShioriError> {
        // Safety: pub vtable メソッド `Complete` の呼出。`self` は有効な `IShioriHost` COM ポインタ、
        // `response` は `[in]` 借用で呼び出しを覆って生存する。`token.0` は u64 値渡し。
        match unsafe { self.Complete(token.0, response) } {
            Ok(()) => Ok(()),
            Err(e) if e.code() == crate::error::SHIORI_E_UNKNOWN_TOKEN => {
                Err(ShioriError::UnknownToken)
            }
            Err(e) => Err(ShioriError::Com(e)),
        }
    }

    /// プロパティストアから値を同期即答する（R10.1/10.3）。
    ///
    /// 欠落 key は [`ShioriError::PropertyNotFound`]（[`crate::error::SHIORI_E_PROPERTY_NOT_FOUND`]
    /// の型付き写像）、他は [`ShioriError::Com`]。
    ///
    /// - `key`: プロパティキー（借用）。
    pub fn get_property(&self, key: &HSTRING) -> Result<HSTRING, ShioriError> {
        // [out] 受け皿。成功時に callee が `out_value` へ move-out（本層が所有・解放）。
        let mut out_value = HSTRING::new();
        // Safety: pub vtable メソッド `GetProperty` の呼出。`self` は有効な `IShioriHost` COM
        // ポインタ、`key` は `[in]` 借用で呼び出しを覆って生存、`out_value` は本層確保の書込領域。
        match unsafe { self.GetProperty(key, &mut out_value) } {
            Ok(()) => Ok(out_value),
            Err(e) if e.code() == crate::error::SHIORI_E_PROPERTY_NOT_FOUND => {
                Err(ShioriError::PropertyNotFound)
            }
            Err(e) => Err(ShioriError::Com(e)),
        }
    }

    /// プロパティストアへ値を即書きする（R10.1）。失敗は [`ShioriError::Com`]。
    ///
    /// - `key`: プロパティキー（借用）。
    /// - `value`: 設定値（借用・host 保持は clone）。
    pub fn set_property(&self, key: &HSTRING, value: &HSTRING) -> Result<(), ShioriError> {
        // Safety: pub vtable メソッド `SetProperty` の呼出。`self` は有効な `IShioriHost` COM
        // ポインタ、`key`/`value` は `[in]` 借用で呼び出しを覆って生存する。
        unsafe { self.SetProperty(key, value) }.map_err(ShioriError::from)
    }
}

#[cfg(test)]
mod tests {
    //! snake_case 安全面の 4 経路マッピング検証（design.md §SafeSurface Validation）。
    //!
    //! `#[implement]` の最小 mock（factory/brain/host）を立て、安全面インヘレントメソッドが
    //! HRESULT を Rust 風 `Result`/`GetOutcome` へ正しく写像することを実行で実証する:
    //! - ①即時: [`IShiori::get`] → [`GetOutcome::Immediate`]（応答内容一致）。
    //! - ②遅延: [`IShiori::get`] → [`GetOutcome::Deferred`]（トークン一致）。
    //! - ③create 失敗: [`IShioriFactory::create`] → [`ShioriError::CreateFailed`]（半構築非露出）。
    //! - ④notify 失敗: [`IShiori::notify`] → [`ShioriError::NotifyFailed`]。
    //! 加えて raise/complete(UnknownToken)/get_property(PropertyNotFound)/set_property の成功/失敗。

    use super::*;
    use crate::error::{SHIORI_E_PROPERTY_NOT_FOUND, SHIORI_E_UNKNOWN_TOKEN};
    use crate::interface::{
        IShiori_Impl, IShioriFactory_Impl, IShioriHost_Impl,
    };
    use windows_core::{HRESULT, OutRef, Result as ComResult, implement};

    /// 即時応答時に move-out する固定文字列。
    const IMMEDIATE_RESPONSE: &str = "pong";
    /// 遅延応答時に発行する固定相関トークン。
    const PENDING_TOKEN: u64 = 0xDEAD_BEEF;
    /// create 失敗を示すテスト用失敗 HRESULT（E_FAIL 相当）。
    const CREATE_FAIL_HR: HRESULT = HRESULT(0x8000_4005u32 as i32);
    /// notify 失敗を示すテスト用失敗 HRESULT（E_FAIL 相当）。
    const NOTIFY_FAIL_HR: HRESULT = HRESULT(0x8000_4005u32 as i32);

    // --- IShiori（脳）の最小モック ---

    /// `get` の 3 分岐と `notify` の成功/失敗を切り替える最小モック脳。
    #[implement(IShiori)]
    struct MockBrain {
        /// true なら `Get` は遅延（`SHIORI_S_PENDING`＋token）、false なら即時（`S_OK`＋応答）。
        defer: bool,
        /// true なら `Notify` は失敗（`NOTIFY_FAIL_HR`）を返す。
        notify_fails: bool,
    }

    impl IShiori_Impl for MockBrain_Impl {
        unsafe fn Get(
            &self,
            _input: &HSTRING,
            out_response: &mut HSTRING,
            out_token: &mut u64,
        ) -> HRESULT {
            if self.defer {
                *out_token = PENDING_TOKEN;
                SHIORI_S_PENDING
            } else {
                *out_response = HSTRING::from(IMMEDIATE_RESPONSE);
                HRESULT(0) // S_OK
            }
        }

        unsafe fn Notify(&self, _input: &HSTRING) -> ComResult<()> {
            if self.notify_fails {
                Err(NOTIFY_FAIL_HR.into())
            } else {
                Ok(())
            }
        }
    }

    // --- IShioriFactory（生成面）の最小モック ---

    /// `create` の成功/失敗を切り替える最小モック factory。
    #[implement(IShioriFactory)]
    struct MockFactory {
        /// true なら `CreateInstance` は失敗（`CREATE_FAIL_HR`）を返し out へ書かない。
        create_fails: bool,
        /// 生成する brain の `defer` フラグ。
        defer: bool,
    }

    impl IShioriFactory_Impl for MockFactory_Impl {
        unsafe fn CreateInstance(
            &self,
            _load_dir: &HSTRING,
            _shiori_name: &HSTRING,
            host: Ref<'_, IShioriHost>,
            out: OutRef<'_, IShiori>,
        ) -> ComResult<()> {
            if self.create_fails {
                // 失敗時は out へ書かない（半構築非露出）。
                return Err(CREATE_FAIL_HR.into());
            }
            let _host_ref: Option<&IShioriHost> = host.as_ref();
            let brain: IShiori = MockBrain {
                defer: self.defer,
                notify_fails: false,
            }
            .into();
            out.write(Some(brain))?;
            Ok(())
        }
    }

    // --- IShioriHost（sink）の最小モック ---

    /// raise/complete/get_property/set_property の成功/失敗を検証する最小モック host。
    #[implement(IShioriHost)]
    struct MockHost {
        raised: std::cell::RefCell<Vec<HSTRING>>,
        completed: std::cell::RefCell<Vec<(u64, HSTRING)>>,
        properties: std::cell::RefCell<std::collections::HashMap<String, HSTRING>>,
        valid_token: u64,
    }

    impl IShioriHost_Impl for MockHost_Impl {
        unsafe fn Raise(&self, script: &HSTRING) -> ComResult<()> {
            self.raised.borrow_mut().push(script.clone());
            Ok(())
        }

        unsafe fn Complete(&self, token: u64, response: &HSTRING) -> ComResult<()> {
            if token != self.valid_token {
                return Err(SHIORI_E_UNKNOWN_TOKEN.into());
            }
            self.completed.borrow_mut().push((token, response.clone()));
            Ok(())
        }

        unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> ComResult<()> {
            match self.properties.borrow().get(&key.to_string()) {
                Some(v) => {
                    *out_value = v.clone();
                    Ok(())
                }
                None => Err(SHIORI_E_PROPERTY_NOT_FOUND.into()),
            }
        }

        unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> ComResult<()> {
            self.properties
                .borrow_mut()
                .insert(key.to_string(), value.clone());
            Ok(())
        }
    }

    fn mock_host(valid_token: u64) -> IShioriHost {
        MockHost {
            raised: std::cell::RefCell::new(Vec::new()),
            completed: std::cell::RefCell::new(Vec::new()),
            properties: std::cell::RefCell::new(std::collections::HashMap::new()),
            valid_token,
        }
        .into()
    }

    // === 4 経路マッピングテスト（安全面インヘレント） ===

    /// ①即時: `create`→`get` が [`GetOutcome::Immediate`]（応答内容一致）を返すこと。
    #[test]
    fn get_immediate_maps_to_immediate_outcome() {
        let factory: IShioriFactory = MockFactory {
            create_fails: false,
            defer: false,
        }
        .into();
        let host = mock_host(42);

        // 安全面 `create`: OutRef 受け皿は隠蔽され IShiori 直返し。
        let brain = factory
            .create(
                &HSTRING::from("C:/ghost/master"),
                &HSTRING::from("reference"),
                &host,
            )
            .expect("create は Ok で IShiori を直返しすること");

        // 安全面 `get`: S_OK → Immediate（応答内容一致）。
        let outcome = brain
            .get(&HSTRING::from("GET SHIORI/3.0..."))
            .expect("即時 get は Ok であること");
        assert_eq!(
            outcome,
            GetOutcome::Immediate(HSTRING::from(IMMEDIATE_RESPONSE)),
            "即時応答は move-out された内容付き Immediate へ写像されること"
        );
    }

    /// ②遅延: `get` が [`GetOutcome::Deferred`]（トークン一致）を返すこと。
    /// **`SHIORI_S_PENDING`（成功コード）判別が `is_ok()` より先**である証左。
    #[test]
    fn get_deferred_maps_to_deferred_outcome_with_token() {
        let factory: IShioriFactory = MockFactory {
            create_fails: false,
            defer: true,
        }
        .into();
        let host = mock_host(42);
        let brain = factory
            .create(&HSTRING::from("dir"), &HSTRING::from("name"), &host)
            .expect("create は Ok であること");

        let outcome = brain
            .get(&HSTRING::from("GET SHIORI/3.0..."))
            .expect("遅延 get は Ok（Deferred）であること");
        assert_eq!(
            outcome,
            GetOutcome::Deferred(CorrelationToken(PENDING_TOKEN)),
            "SHIORI_S_PENDING は相関トークン付き Deferred へ写像されること（is_ok より先に判別）"
        );
    }

    /// ③create 失敗: `create` が [`ShioriError::CreateFailed`]（半構築非露出）を返すこと。
    #[test]
    fn create_failure_maps_to_create_failed_without_exposing_half_construct() {
        let factory: IShioriFactory = MockFactory {
            create_fails: true,
            defer: false,
        }
        .into();
        let host = mock_host(42);

        let err = factory
            .create(&HSTRING::from("dir"), &HSTRING::from("name"), &host)
            .expect_err("create 失敗は Err であること（半構築 IShiori を返さない）");
        match err {
            ShioriError::CreateFailed(hr) => {
                assert_eq!(hr, CREATE_FAIL_HR, "失敗 HRESULT を内包すること")
            }
            other => panic!("expected CreateFailed, got {other:?}"),
        }
    }

    /// ④notify 失敗: `notify` が [`ShioriError::NotifyFailed`] を返すこと。
    #[test]
    fn notify_failure_maps_to_notify_failed() {
        // notify_fails=true の brain を直接立てる（factory 経由では notify_fails を制御しないため）。
        let brain: IShiori = MockBrain {
            defer: false,
            notify_fails: true,
        }
        .into();

        let err = brain
            .notify(&HSTRING::from("NOTIFY SHIORI/3.0..."))
            .expect_err("notify 失敗は Err であること");
        match err {
            ShioriError::NotifyFailed(hr) => {
                assert_eq!(hr, NOTIFY_FAIL_HR, "失敗 HRESULT を内包すること")
            }
            other => panic!("expected NotifyFailed, got {other:?}"),
        }
    }

    /// notify 成功経路（片道・Ok）。
    #[test]
    fn notify_success_maps_to_ok() {
        let brain: IShiori = MockBrain {
            defer: false,
            notify_fails: false,
        }
        .into();
        brain
            .notify(&HSTRING::from("NOTIFY SHIORI/3.0..."))
            .expect("notify 成功は Ok であること");
    }

    // === host sink 安全面（raise/complete/get_property/set_property） ===

    /// `raise` 成功が Ok を返すこと。
    #[test]
    fn raise_success_maps_to_ok() {
        let host = mock_host(42);
        host.raise(&HSTRING::from("\\h\\s[0]hello"))
            .expect("raise は Ok であること");
    }

    /// `complete`: 有効トークン→Ok・未知トークン→[`ShioriError::UnknownToken`]。
    #[test]
    fn complete_maps_unknown_token_to_typed_error() {
        let host = mock_host(42);
        let response = HSTRING::from("response-body");

        host.complete(CorrelationToken(42), &response)
            .expect("有効トークンの complete は Ok であること");

        let err = host
            .complete(CorrelationToken(999), &response)
            .expect_err("未知トークンの complete は Err であること");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "SHIORI_E_UNKNOWN_TOKEN は ShioriError::UnknownToken へ写像されること, got {err:?}"
        );
    }

    /// `set_property`→`get_property`: 同期即答・欠落 key→[`ShioriError::PropertyNotFound`]。
    #[test]
    fn get_property_maps_missing_key_to_typed_error() {
        let host = mock_host(42);
        let key = HSTRING::from("path.to.key");
        let value = HSTRING::from("some-value");

        host.set_property(&key, &value)
            .expect("set_property は Ok であること");
        let got = host.get_property(&key).expect("get_property は Ok であること");
        assert_eq!(got, value, "設定した値が move-out されること");

        let err = host
            .get_property(&HSTRING::from("no.such.key"))
            .expect_err("欠落 key の get_property は Err であること");
        assert!(
            matches!(err, ShioriError::PropertyNotFound),
            "SHIORI_E_PROPERTY_NOT_FOUND は ShioriError::PropertyNotFound へ写像されること, got {err:?}"
        );
    }
}
