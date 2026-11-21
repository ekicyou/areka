//! snake_case インヘレント安全面。
//!
//! # 2 面インヘレント
//! [`crate::interface`] の `#[interface]` は各 COM 境界を PascalCase の pub vtable メソッド
//! （`CreateInstance`/`Get`/`Notify`/`Raise`/`Complete`/`GetProperty`/`SetProperty`）として生成する。
//! 本モジュールは同じ型に **第 2 インヘレント impl** を snake_case で足し、`OutRef` 受け皿・
//! HRESULT 判別・error 写像を内包した `Result` 直返し API にする。PascalCase と snake_case は
//! 命名衝突しないため拡張トレイトの import は不要で、consumer から raw ポインタ・HRESULT・
//! `unsafe` は見えない。拡張トレイト（旧 `ShioriExt`）には戻さない。
//!
//! # 提供する安全面
//! - [`IShioriFactory::create`]: `OutRef` 受け皿を隠蔽し load 完了済み [`IShiori`] を直返し。
//!   失敗は [`ShioriError::CreateFailed`]（半構築非露出）。
//! - [`IShiori::get`]: HRESULT 3 分岐を [`GetOutcome`] へ復元。
//! - [`IShiori::notify`]: 片道通知（失敗は [`ShioriError::NotifyFailed`]）。
//! - [`IShioriHost::raise`]/[`complete`](IShioriHost::complete)/[`get_property`](IShioriHost::get_property)/
//!   [`set_property`](IShioriHost::set_property): host sink の安全面。
//!
//! # HRESULT マッピング規約
//! [`get`](IShiori::get) は raw `Get` の HRESULT を次の**固定順序**で振り分ける:
//! 1. [`SHIORI_S_PENDING`] → [`GetOutcome::Deferred`]（`out_token` を [`CorrelationToken`] へ）。
//! 2. `is_ok()` → [`GetOutcome::Immediate`]（`out_response` を move-out して所有）。
//! 3. その他失敗 → [`ShioriError::GetFailed`]（HRESULT 内包）。
//!
//! **必ず [`SHIORI_S_PENDING`] 判別を `is_ok()` より先に行う**。[`SHIORI_S_PENDING`] も成功コード
//! （severity=0）ゆえ、順序を誤ると遅延を即時へ誤分類する。

use windows_core::{HSTRING, Ref};

use crate::error::{SHIORI_S_PENDING, ShioriError};
use crate::interface::{IShiori, IShioriFactory, IShioriHost};
use crate::outcome::{CorrelationToken, GetOutcome};

impl IShioriFactory {
    /// 脳を生成し load 完了済みの [`IShiori`] を直返しする（生成と load を 1 回で行う）。
    ///
    /// 失敗時は out から何も取り出さず [`ShioriError::CreateFailed`] を返す（半構築を露出しない）。
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
        let mut out: Option<IShiori> = None;
        // SAFETY:
        // - `self` は有効な `IShioriFactory` COM ポインタ（借用で生存）。
        // - `load_dir`/`shiori_name` は `[in]` 借用で呼び出しを覆って生存する。
        // - `host` は `Ref` 借用で AddRef されず、呼び出し中有効。
        // - `out` は本層がスタックに確保した書き込み可能領域（`(&mut out).into()` で OutRef 化）。
        let result = unsafe {
            self.CreateInstance(
                load_dir,
                shiori_name,
                Ref::from(Some(host)),
                (&mut out).into(),
            )
        };
        match result {
            Ok(()) => out.ok_or(ShioriError::CreateFailed(windows_core::HRESULT(0))),
            Err(e) => Err(ShioriError::CreateFailed(e.code())),
        }
    }
}

impl IShiori {
    /// 同期 request を [`GetOutcome`] へ復元する。
    ///
    /// raw `Get` の HRESULT を 3 分岐する（**[`SHIORI_S_PENDING`] 判別が `is_ok()` より先**）:
    /// - [`SHIORI_S_PENDING`] → [`GetOutcome::Deferred`]（`out_token` を [`CorrelationToken`] へ）。
    /// - `is_ok()`（`S_OK`）→ [`GetOutcome::Immediate`]（`out_response` を move-out して所有）。
    /// - その他失敗 → [`ShioriError::GetFailed`]（HRESULT 内包）。
    ///
    /// - `input`: 正準 content（借用・callee は読み取りのみで解放しない）。
    pub fn get(&self, input: &HSTRING) -> Result<GetOutcome, ShioriError> {
        // `S_OK` のとき callee が move-out する（解放は本層）。
        let mut out_response = HSTRING::new();
        // `SHIORI_S_PENDING` のときだけ有効。
        let mut out_token: u64 = 0;
        // SAFETY:
        // - `self` は有効な `IShiori` COM ポインタ（借用で生存）。
        // - `input` は `[in]` 借用で呼び出しを覆って生存する。
        // - `out_response`/`out_token` は本層がスタックに確保した書き込み可能領域。
        let hr = unsafe { self.Get(input, &mut out_response, &mut out_token) };

        // `SHIORI_S_PENDING` も成功コードなので `is_ok()` より先に見る。
        if hr == SHIORI_S_PENDING {
            Ok(GetOutcome::Deferred(CorrelationToken(out_token)))
        } else if hr.is_ok() {
            Ok(GetOutcome::Immediate(out_response))
        } else {
            Err(ShioriError::GetFailed(hr))
        }
    }

    /// 片道通知（応答なし）。失敗は [`ShioriError::NotifyFailed`]（HRESULT 内包）。
    ///
    /// - `input`: 通知 content（借用・callee 保持は clone）。
    pub fn notify(&self, input: &HSTRING) -> Result<(), ShioriError> {
        // SAFETY: `self` は有効な `IShiori` COM ポインタ、`input` は `[in]` 借用で呼び出しを覆って生存する。
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
        // SAFETY: `self` は有効な `IShioriHost` COM ポインタ、`script` は `[in]` 借用で呼び出しを覆って生存する。
        unsafe { self.Raise(script) }.map_err(ShioriError::from)
    }

    /// 遅延リクエストの完了応答を配送する。
    ///
    /// 未知/stale トークンは [`ShioriError::UnknownToken`]（[`crate::error::SHIORI_E_UNKNOWN_TOKEN`]
    /// の型付き写像）、他は [`ShioriError::Com`]。
    ///
    /// - `token`: 相関トークン（[`get`](IShiori::get) が遅延時に返した値）。
    /// - `response`: 応答内容（借用・host 保持は clone）。
    pub fn complete(&self, token: CorrelationToken, response: &HSTRING) -> Result<(), ShioriError> {
        // SAFETY: `self` は有効な `IShioriHost` COM ポインタ、`response` は `[in]` 借用で呼び出しを覆って生存する。`token.0` は u64 値渡し。
        match unsafe { self.Complete(token.0, response) } {
            Ok(()) => Ok(()),
            Err(e) if e.code() == crate::error::SHIORI_E_UNKNOWN_TOKEN => {
                Err(ShioriError::UnknownToken)
            }
            Err(e) => Err(ShioriError::Com(e)),
        }
    }

    /// プロパティストアから値を同期即答する。
    ///
    /// 欠落 key は [`ShioriError::PropertyNotFound`]（[`crate::error::SHIORI_E_PROPERTY_NOT_FOUND`]
    /// の型付き写像）、他は [`ShioriError::Com`]。
    ///
    /// - `key`: プロパティキー（借用）。
    pub fn get_property(&self, key: &HSTRING) -> Result<HSTRING, ShioriError> {
        // 成功時に callee が move-out する（解放は本層）。
        let mut out_value = HSTRING::new();
        // SAFETY: `self` は有効な `IShioriHost` COM ポインタ、`key` は `[in]` 借用で呼び出しを覆って生存、`out_value` は本層確保の書込領域。
        match unsafe { self.GetProperty(key, &mut out_value) } {
            Ok(()) => Ok(out_value),
            Err(e) if e.code() == crate::error::SHIORI_E_PROPERTY_NOT_FOUND => {
                Err(ShioriError::PropertyNotFound)
            }
            Err(e) => Err(ShioriError::Com(e)),
        }
    }

    /// プロパティストアへ値を即書きする。失敗は [`ShioriError::Com`]。
    ///
    /// - `key`: プロパティキー（借用）。
    /// - `value`: 設定値（借用・host 保持は clone）。
    pub fn set_property(&self, key: &HSTRING, value: &HSTRING) -> Result<(), ShioriError> {
        // SAFETY: `self` は有効な `IShioriHost` COM ポインタ、`key`/`value` は `[in]` 借用で呼び出しを覆って生存する。
        unsafe { self.SetProperty(key, value) }.map_err(ShioriError::from)
    }
}

#[cfg(test)]
mod tests {
    //! `#[implement]` の最小 mock（factory/brain/host）で、安全面のメソッドが HRESULT を
    //! `Result`/`GetOutcome` へ写像することを実行で確かめる。

    use super::*;
    use crate::error::{SHIORI_E_PROPERTY_NOT_FOUND, SHIORI_E_UNKNOWN_TOKEN};
    use crate::interface::{IShiori_Impl, IShioriFactory_Impl, IShioriHost_Impl};
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

    #[test]
    fn get_immediate_maps_to_immediate_outcome() {
        let factory: IShioriFactory = MockFactory {
            create_fails: false,
            defer: false,
        }
        .into();
        let host = mock_host(42);

        let brain = factory
            .create(
                &HSTRING::from("C:/ghost/master"),
                &HSTRING::from("reference"),
                &host,
            )
            .expect("create は Ok で IShiori を直返しすること");

        let outcome = brain
            .get(&HSTRING::from("GET SHIORI/3.0..."))
            .expect("即時 get は Ok であること");
        assert_eq!(
            outcome,
            GetOutcome::Immediate(HSTRING::from(IMMEDIATE_RESPONSE)),
            "即時応答は move-out された内容付き Immediate へ写像されること"
        );
    }

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

    #[test]
    fn raise_success_maps_to_ok() {
        let host = mock_host(42);
        host.raise(&HSTRING::from("\\h\\s[0]hello"))
            .expect("raise は Ok であること");
    }

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

    #[test]
    fn get_property_maps_missing_key_to_typed_error() {
        let host = mock_host(42);
        let key = HSTRING::from("path.to.key");
        let value = HSTRING::from("some-value");

        host.set_property(&key, &value)
            .expect("set_property は Ok であること");
        let got = host
            .get_property(&key)
            .expect("get_property は Ok であること");
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
