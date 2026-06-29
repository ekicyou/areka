//! `IShiori` エルゴノミック変換層（`ShioriExt`）。
//!
//! raw `unsafe fn -> HRESULT` の COM 呼び出しを Rust 風の `Result` へ変換し、
//! 呼び出し側（areka 本体）へ raw `unsafe`/HRESULT を一切露出させない 2 層構造の
//! 安全側を担う（design.md §Components and Interfaces → Ergonomic Layer, D4・要件 1.2）。
//!
//! ## 提供する安全 API（design.md §ShioriExt / RequestOutcome / ShioriError）
//! [`ShioriExt`] が [`crate::interface::IShiori`] への拡張トレイトとして 3 操作を提供する:
//! - [`load`](ShioriExt::load): 脳をロードし sink（[`crate::interface::IShioriHost`]）を渡す（要件 2.1/2.3）。
//! - [`unload`](ShioriExt::unload): 脳をアンロードする（要件 2.2）。
//! - [`request`](ShioriExt::request): 同期 request を即時/遅延/失敗へ型化する（要件 3.1〜3.6）。
//!
//! ## HRESULT マッピング規約（design.md §HRESULT マッピング・要件 3.2/3.3/3.5/3.6/7.2）
//! `request` は raw `Request` の HRESULT を次のように振り分ける:
//! - `S_OK` → [`RequestOutcome::Immediate`]（`out_response` を move-out して所有。Drop は呼び出し側）。
//! - [`SHIORI_S_PENDING`] → [`RequestOutcome::Deferred`]（`out_token` を [`CorrelationToken`] へ）。
//! - [`SHIORI_E_NOT_LOADED`] → [`ShioriError::NotLoaded`]（要件 2.4）。
//! - その他失敗 → [`ShioriError::RequestFailed`]（HRESULT 内包・要件 3.6）。
//!
//! ## HSTRING 所有権規約（議題1・interface.rs モジュール doc・要件 5.4）
//! - `[in]`（content）= **借用**。`*const HSTRING` で渡すのみで解放しない。content の
//!   生存期間が `Request` 呼び出しを覆う（借用が呼び出し中有効）。
//! - `[out]`（`out_response`）= **callee 確保・caller 解放**の move-out。`S_OK` 時に callee が
//!   書き込んだ HSTRING の所有権を受け取り、`RequestOutcome::Immediate` として保持する
//!   （Drop は本層の呼び出し側）。

use windows_core::{HRESULT, HSTRING, Interface};

use crate::error::{SHIORI_E_NOT_LOADED, SHIORI_S_PENDING, ShioriError};
use crate::interface::{IShiori, IShioriHost};
use crate::outcome::{CorrelationToken, RequestOutcome};

/// [`crate::interface::IShiori`] への安全な拡張トレイト（design.md §ShioriExt・D4）。
///
/// raw `unsafe` 呼び出しを `Result` へ変換し、呼び出し側へ raw/HRESULT を露出させない。
/// areka 本体はこのトレイト越しにのみ脳を操作する（`IShioriHost` を `#[implement]` で実装し
/// [`load`](ShioriExt::load) に渡す・design.md §Implementation Notes）。
pub trait ShioriExt {
    /// 脳をロードし sink（`host`）を渡す（要件 2.1/2.3）。
    ///
    /// `host` の raw COM ポインタを raw `Load` へ渡す。`S_OK`→`Ok(())`、失敗 HRESULT は
    /// [`ShioriError::LoadFailed`]（HRESULT 内包）へ変換する。
    fn load(&self, host: &IShioriHost) -> Result<(), ShioriError>;

    /// 脳をアンロードする（要件 2.2）。`S_OK`→`Ok(())`、失敗は [`ShioriError`] へ変換する。
    fn unload(&self) -> Result<(), ShioriError>;

    /// 同期 request を即時/遅延/失敗へ型化する（要件 3.1〜3.6）。
    ///
    /// `content` は `[in]` 借用（解放しない）。HRESULT マッピングはモジュール doc 参照。
    fn request(&self, content: &HSTRING) -> Result<RequestOutcome, ShioriError>;
}

impl ShioriExt for IShiori {
    fn load(&self, host: &IShioriHost) -> Result<(), ShioriError> {
        // `IShioriHost` の raw COM ポインタ（`*mut c_void`）を取り出して raw `Load` の
        // vtable スロットへ渡す。host の所有権は呼び出し側に残る（`as_raw` は借用ポインタを
        // 返すのみ＝AddRef しない）。raw COM 呼び出しは vtable 越し（`#[interface]` 生成の
        // 同名メソッドはモジュール private のため、vtable 関数ポインタを直に叩く）。
        let raw_host = host.as_raw();
        // Safety: `self.as_raw()` は有効な `IShiori` COM ポインタ（`self` が借用で生存）。
        // `raw_host` は呼び出し中有効な `IShioriHost` raw ポインタ（`host` が借用で生存）。
        let hr = unsafe { (Interface::vtable(self).Load)(self.as_raw(), raw_host) };
        if hr.is_ok() {
            Ok(())
        } else {
            // load 失敗は文脈変種 `LoadFailed`（HRESULT 内包・要件 2.3）。
            Err(ShioriError::LoadFailed(hr))
        }
    }

    fn unload(&self) -> Result<(), ShioriError> {
        // Safety: `self.as_raw()` は有効な `IShiori` COM ポインタ（`self` が借用で生存）。引数なし。
        let hr = unsafe { (Interface::vtable(self).Unload)(self.as_raw()) };
        if hr.is_ok() {
            Ok(())
        } else {
            Err(map_unload_failure(hr))
        }
    }

    fn request(&self, content: &HSTRING) -> Result<RequestOutcome, ShioriError> {
        // [out] 受け皿。`S_OK` 時に callee が `out_response` へ move-out（caller=本層が所有・解放）。
        let mut out_response = HSTRING::new();
        // 遅延時に callee が相関トークンを書き込む受け皿（`SHIORI_S_PENDING` 時のみ有効）。
        let mut out_token: u64 = 0;
        // Safety:
        // - `self.as_raw()` は有効な `IShiori` COM ポインタ（`self` が借用で生存）。
        // - `content` は `[in]` 借用。`*const HSTRING` で渡すのみで解放しない。`content` は
        //   引数借用で呼び出しを覆って生存するため、ポインタは呼び出し中有効。
        // - `out_response`/`out_token` は本層がスタックに確保した書き込み可能領域。
        let hr = unsafe {
            (Interface::vtable(self).Request)(
                self.as_raw(),
                content as *const HSTRING,
                &mut out_response,
                &mut out_token,
            )
        };

        // HRESULT マッピング規約（モジュール doc・design.md §HRESULT マッピング）。
        if hr == SHIORI_S_PENDING {
            // 遅延: `out_token` を相関トークンへ（`out_response` は空＝未書き込み）。
            Ok(RequestOutcome::Deferred(CorrelationToken(out_token)))
        } else if hr.is_ok() {
            // 即時（`S_OK`）: `out_response` を move-out して所有（Drop は本層呼び出し側）。
            Ok(RequestOutcome::Immediate(out_response))
        } else if hr == SHIORI_E_NOT_LOADED {
            // 未ロード拒否（要件 2.4）。
            Err(ShioriError::NotLoaded)
        } else {
            // その他失敗（要件 3.6）: HRESULT を内包する文脈変種へ。
            Err(ShioriError::RequestFailed(hr))
        }
    }
}

/// `unload` 失敗 HRESULT を [`ShioriError`] へ写す（未ロードは `NotLoaded`、他は `Com`）。
fn map_unload_failure(hr: HRESULT) -> ShioriError {
    if hr == SHIORI_E_NOT_LOADED {
        ShioriError::NotLoaded
    } else {
        ShioriError::Com(windows_core::Error::from(hr))
    }
}

#[cfg(test)]
mod tests {
    //! モック `IShiori`（`#[implement(IShiori)]`）経由で `ShioriExt` の安全変換を検証する。
    //!
    //! 観測（design.md §Implementation Notes → Validation の 4 経路）:
    //! - 即時（`S_OK`＋response）→ `Immediate(HSTRING)` で内容一致（要件 3.2）。
    //! - 遅延（`SHIORI_S_PENDING`＋token）→ `Deferred(CorrelationToken)` でトークン一致（要件 3.3）。
    //! - 未ロード（`SHIORI_E_NOT_LOADED`）→ `Err(NotLoaded)`（要件 2.4）。
    //! - 汎用失敗（`E_FAIL`）→ `Err(RequestFailed(hr))`（要件 3.6）。
    //! - `load`/`unload` の成功/失敗。

    use super::*;
    use crate::interface::{IShiori_Impl, IShioriHost_Impl};
    use std::cell::Cell;
    use windows_core::implement;

    /// `Request` の挙動を選択するモックの動作種別。
    #[derive(Clone, Copy)]
    enum RequestBehavior {
        /// 即時応答。`out_response` に固定文字列を move-out して `S_OK`。
        Immediate,
        /// 遅延。`out_token` に固定トークンを書き込み `SHIORI_S_PENDING`。
        Pending,
        /// 未ロード拒否（`SHIORI_E_NOT_LOADED`）。
        NotLoaded,
        /// 汎用失敗（`E_FAIL`）。
        GenericFailure,
    }

    /// 即時応答で move-out する固定文字列。
    const IMMEDIATE_RESPONSE: &str = "immediate-response-body";
    /// 遅延時に発行する固定相関トークン。
    const PENDING_TOKEN: u64 = 0xDEAD_BEEF;
    /// `load` 失敗時に返す HRESULT（`E_FAIL`）。
    const E_FAIL: HRESULT = HRESULT(0x8000_4005u32 as i32);

    /// 多挙動を選択できるモック脳。`load`/`unload` の呼び出しと `Request` の分岐を制御する。
    #[implement(IShiori)]
    struct MockBrain {
        behavior: RequestBehavior,
        /// `load` が失敗 HRESULT を返すか（true で `E_FAIL`）。
        load_fails: bool,
        /// `unload` が失敗 HRESULT を返すか（true で `SHIORI_E_NOT_LOADED`）。
        unload_fails: bool,
        loaded: Cell<bool>,
        unloaded: Cell<bool>,
    }

    impl MockBrain {
        fn new(behavior: RequestBehavior) -> Self {
            Self {
                behavior,
                load_fails: false,
                unload_fails: false,
                loaded: Cell::new(false),
                unloaded: Cell::new(false),
            }
        }
    }

    impl IShiori_Impl for MockBrain_Impl {
        unsafe fn Load(&self, _host: *mut core::ffi::c_void) -> HRESULT {
            self.loaded.set(true);
            if self.load_fails {
                E_FAIL
            } else {
                HRESULT(0) // S_OK
            }
        }

        unsafe fn Unload(&self) -> HRESULT {
            self.unloaded.set(true);
            if self.unload_fails {
                SHIORI_E_NOT_LOADED
            } else {
                HRESULT(0) // S_OK
            }
        }

        unsafe fn Request(
            &self,
            _input: *const HSTRING,
            out_response: *mut HSTRING,
            out_token: *mut u64,
        ) -> HRESULT {
            match self.behavior {
                RequestBehavior::Immediate => {
                    // 即時応答: callee が確保した HSTRING を out-param へ move-out（所有権規約）。
                    unsafe { core::ptr::write(out_response, HSTRING::from(IMMEDIATE_RESPONSE)) };
                    HRESULT(0) // S_OK
                }
                RequestBehavior::Pending => {
                    // 遅延: 相関トークンを out_token へ。out_response は未書き込み（空）。
                    unsafe { core::ptr::write(out_token, PENDING_TOKEN) };
                    SHIORI_S_PENDING
                }
                RequestBehavior::NotLoaded => SHIORI_E_NOT_LOADED,
                RequestBehavior::GenericFailure => E_FAIL,
            }
        }
    }

    /// テスト用 host（`load` へ渡す sink）。挙動は不問なので最小実装。
    #[implement(IShioriHost)]
    struct DummyHost;

    impl IShioriHost_Impl for DummyHost_Impl {
        unsafe fn Raise(&self, _script: *const HSTRING) -> HRESULT {
            HRESULT(0)
        }
        unsafe fn Complete(&self, _token: u64, _response: *const HSTRING) -> HRESULT {
            HRESULT(0)
        }
    }

    /// 即時応答: `S_OK`＋response が `Immediate(HSTRING)` へ写り内容が一致すること（要件 3.2）。
    #[test]
    fn request_immediate_returns_immediate_with_matching_response() {
        let brain: IShiori = MockBrain::new(RequestBehavior::Immediate).into();
        let content = HSTRING::from("ping");
        let outcome = brain.request(&content).expect("即時応答は Ok であること");
        match outcome {
            RequestOutcome::Immediate(got) => {
                assert_eq!(
                    got,
                    HSTRING::from(IMMEDIATE_RESPONSE),
                    "move-out された応答内容が一致すること"
                );
            }
            other => panic!("expected Immediate, got {other:?}"),
        }
    }

    /// 遅延: `SHIORI_S_PENDING`＋token が `Deferred(CorrelationToken)` へ写りトークンが一致すること（要件 3.3）。
    #[test]
    fn request_pending_returns_deferred_with_matching_token() {
        let brain: IShiori = MockBrain::new(RequestBehavior::Pending).into();
        let content = HSTRING::from("ping");
        let outcome = brain.request(&content).expect("遅延は Ok（成功コード）であること");
        match outcome {
            RequestOutcome::Deferred(token) => {
                assert_eq!(
                    token,
                    CorrelationToken(PENDING_TOKEN),
                    "out_token のトークン値が一致すること"
                );
            }
            other => panic!("expected Deferred, got {other:?}"),
        }
    }

    /// 未ロード: `SHIORI_E_NOT_LOADED` が `Err(NotLoaded)` へ写ること（要件 2.4）。
    #[test]
    fn request_not_loaded_returns_not_loaded_error() {
        let brain: IShiori = MockBrain::new(RequestBehavior::NotLoaded).into();
        let content = HSTRING::from("ping");
        let err = brain.request(&content).expect_err("未ロードは Err であること");
        assert!(
            matches!(err, ShioriError::NotLoaded),
            "未ロードは ShioriError::NotLoaded であること, got {err:?}"
        );
    }

    /// 汎用失敗: その他失敗 HRESULT が `Err(RequestFailed(hr))` へ写り HRESULT を保持すること（要件 3.6）。
    #[test]
    fn request_generic_failure_returns_request_failed_preserving_hresult() {
        let brain: IShiori = MockBrain::new(RequestBehavior::GenericFailure).into();
        let content = HSTRING::from("ping");
        let err = brain.request(&content).expect_err("汎用失敗は Err であること");
        match err {
            ShioriError::RequestFailed(hr) => {
                assert_eq!(hr, E_FAIL, "失敗 HRESULT が保持されること");
            }
            other => panic!("expected RequestFailed, got {other:?}"),
        }
    }

    /// `load`: 成功時 `Ok(())` を返し、raw `Load` が呼ばれること（要件 2.1）。
    #[test]
    fn load_success_returns_ok() {
        let mock = MockBrain::new(RequestBehavior::Immediate);
        let brain: IShiori = mock.into();
        let host: IShioriHost = DummyHost.into();
        let res = brain.load(&host);
        assert!(res.is_ok(), "load 成功は Ok(()) であること: {res:?}");
    }

    /// `load`: 失敗 HRESULT は `Err(LoadFailed(hr))` へ写り HRESULT を保持すること（要件 2.3）。
    #[test]
    fn load_failure_returns_load_failed_preserving_hresult() {
        let mut mock = MockBrain::new(RequestBehavior::Immediate);
        mock.load_fails = true;
        let brain: IShiori = mock.into();
        let host: IShioriHost = DummyHost.into();
        let err = brain.load(&host).expect_err("load 失敗は Err であること");
        match err {
            ShioriError::LoadFailed(hr) => assert_eq!(hr, E_FAIL, "失敗 HRESULT が保持されること"),
            other => panic!("expected LoadFailed, got {other:?}"),
        }
    }

    /// `unload`: 成功時 `Ok(())` を返すこと（要件 2.2）。
    #[test]
    fn unload_success_returns_ok() {
        let brain: IShiori = MockBrain::new(RequestBehavior::Immediate).into();
        let res = brain.unload();
        assert!(res.is_ok(), "unload 成功は Ok(()) であること: {res:?}");
    }

    /// `unload`: 失敗 HRESULT（`SHIORI_E_NOT_LOADED`）が `Err(NotLoaded)` へ写ること。
    #[test]
    fn unload_failure_maps_error() {
        let mut mock = MockBrain::new(RequestBehavior::Immediate);
        mock.unload_fails = true;
        let brain: IShiori = mock.into();
        let err = brain.unload().expect_err("unload 失敗は Err であること");
        assert!(
            matches!(err, ShioriError::NotLoaded),
            "SHIORI_E_NOT_LOADED は NotLoaded へ写ること, got {err:?}"
        );
    }
}
