//! 製品コード（非 `#[cfg(test)]`）のリファレンス脳。
//!
//! 上流 `areka-P0-shiori-com` の `IShiori` を `#[implement(IShiori)]` で実装する
//! 最小の native リファレンス脳。content（リクエスト・応答・通知の本文）は不透明な
//! HSTRING（UTF-16）のままエコーで取り回し、解析・スキーマ検証・意味づけを行わない
//! （要件 1.4／8.1）。
//!
//! 本タスク時点でライフサイクル（Load/Unload）・ロード状態保持・未ロード拒否（2.1）と、
//! ロード済み Request の即時エコー応答（2.2・受信 content を不解釈のまま往復）を実装する。
//! 遅延応答＋Complete（2.3）、能動通知 Raise（2.4）、`shiori_create` コンストラクタ（3.x）、
//! module-level の完全リファレンス doc（2.6）は後続タスクで配線する。

#![allow(non_snake_case)] // `#[implement(IShiori)]` の生成面が PascalCase メソッドを要求する。

use core::cell::RefCell;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, Ordering};

use shiori_abi::error::SHIORI_E_NOT_LOADED;
use shiori_abi::interface::{IShiori, IShiori_Impl, IShioriHost};
use shiori_abi::outcome::CorrelationTokenAllocator;
use windows_core::{HRESULT, HSTRING, Interface, implement};

/// `S_OK`（成功・即時応答）の HRESULT。
const S_OK: HRESULT = HRESULT(0);

/// 製品コードの最小リファレンス脳（`#[implement(IShiori)]`）。
///
/// ロード状態を [`AtomicBool`] で保持（`StatefulBrain` 踏襲）し、未ロード時の `Request` は
/// [`SHIORI_E_NOT_LOADED`] として判別可能に拒否する（要件 2.3／2.4）。`Load` で受け取った
/// host は AddRef 保持し、`Unload` で Release する（要件 2.1／2.2・保持参照モデル）。
#[implement(IShiori)]
pub struct ReferenceBrain {
    /// ロード状態（`false`=Unloaded／`true`=Loaded）。`StatefulBrain` 踏襲（要件 2.3）。
    loaded: AtomicBool,
    /// `Load` で AddRef 保持した host（`Unload` で Release）。
    ///
    /// 遅延 `Complete`・能動 `Raise` を発火する保持 host（task 2.3／2.4 で消費）。
    #[allow(dead_code)] // host への Raise/Complete 発火は task 2.3/2.4 で消費する。
    held_host: RefCell<Option<IShioriHost>>,
    /// 遅延応答の相関トークンアロケータ。
    ///
    /// task 2.3（遅延応答）でトークン採番に消費する。
    #[allow(dead_code)] // 遅延トークン採番は task 2.3 で消費する。
    tokens: CorrelationTokenAllocator,
}

impl ReferenceBrain {
    /// Unloaded 状態・host 未保持の脳を生成する（要件 2.x 初期状態）。
    ///
    /// テストおよび後続の `shiori_create`（task 3.x）は `ReferenceBrain::new().into()` で
    /// `IShiori` COM ポインタを構築する。
    pub fn new() -> Self {
        Self {
            loaded: AtomicBool::new(false),
            held_host: RefCell::new(None),
            tokens: CorrelationTokenAllocator::new(),
        }
    }
}

impl Default for ReferenceBrain {
    fn default() -> Self {
        Self::new()
    }
}

impl IShiori_Impl for ReferenceBrain_Impl {
    /// host を AddRef 保持し Loaded へ遷移する。成功=`S_OK`（要件 2.1）。
    unsafe fn Load(&self, host: *mut c_void) -> HRESULT {
        // 受け取った raw host を型付き COM 参照として AddRef 保持する（保持参照モデル）。
        // Safety: host は areka が所有する有効な IShioriHost raw ポインタ（呼び出し中有効）。
        // from_raw_borrowed は借用ビューを返すため、保持するには clone で AddRef する。
        let borrowed: Option<&IShioriHost> = unsafe { IShioriHost::from_raw_borrowed(&host) };
        *self.held_host.borrow_mut() = borrowed.cloned();
        self.loaded.store(true, Ordering::SeqCst);
        S_OK
    }

    /// host を Release し Unloaded へ遷移する。成功=`S_OK`（要件 2.2）。
    unsafe fn Unload(&self) -> HRESULT {
        // 保持 host を Release（drop）する（Unload 後は host を呼ばない）。
        *self.held_host.borrow_mut() = None;
        self.loaded.store(false, Ordering::SeqCst);
        S_OK
    }

    /// 未ロード時は [`SHIORI_E_NOT_LOADED`]（out-param 未書込）。ロード時は受信 content の
    /// 即時エコー応答を `out_response` へ move-out し `S_OK` を返す（要件 1.4／3.1／3.2／3.3／3.4／8.1）。
    ///
    /// content は不透明 HSTRING（UTF-16）のまま取り回し、`input` を解析・スキーマ検証・分割・
    /// デコード・内容分岐しない。エコーは純粋なコピー（受信 content をそのまま往復）であり、
    /// 不解釈のまま一致することで content 不透明性を実証する（要件 1.4／8.1）。
    ///
    /// 即時／遅延の判別機構は task 2.3 で導入する。本タスク（2.2）では、ロード済みの Request は
    /// 常に即時（同期）エコー応答とする。
    unsafe fn Request(
        &self,
        input: *const HSTRING,
        out_response: *mut HSTRING,
        _out_token: *mut u64,
    ) -> HRESULT {
        if !self.loaded.load(Ordering::SeqCst) {
            // 未ロード: 有効な処理として受理せず判別可能な失敗を返す。out-param は未書込（要件 2.4）。
            return SHIORI_E_NOT_LOADED;
        }
        // ロード済み: `input` は [in] 借用（読み取りのみ・解放しない）。エコーのため clone で
        // 所有 HSTRING を得る。content は不透明 UTF-16 のまま無加工コピーする（要件 1.4／8.1）。
        // Safety: `input` は呼び出し中有効な HSTRING raw ポインタ（ABI 所有権規約）。
        let echoed = unsafe { (*input).clone() };
        // callee 確保の HSTRING を move-out（所有権規約・caller 解放）。
        // Safety: `out_response` は未初期化の有効な出力スロット（caller 確保・上書き対象）。
        unsafe { core::ptr::write(out_response, echoed) };
        S_OK
    }
}

#[cfg(test)]
mod tests {
    //! ライフサイクル＋未ロード拒否の単体テスト（要件 2.1/2.2/2.3/2.4）。
    //!
    //! `IShiori` COM ポインタ経由で `Load`/`Unload`/`Request` を駆動し、ロード状態遷移と
    //! 未ロード時の `SHIORI_E_NOT_LOADED` を検証する。

    use super::*;
    use core::ffi::c_void;

    /// `Load` へ渡す最小ダミー host（挙動不問・最小実装）。
    #[implement(IShioriHost)]
    struct DummyHost;

    impl shiori_abi::interface::IShioriHost_Impl for DummyHost_Impl {
        unsafe fn Raise(&self, _script: *const HSTRING) -> HRESULT {
            S_OK
        }
        unsafe fn Complete(&self, _token: u64, _response: *const HSTRING) -> HRESULT {
            S_OK
        }
    }

    /// COM ポインタ経由（vtable 直呼び）で `Request` を呼ぶヘルパ（out-param 確保→HRESULT）。
    ///
    /// raw `Request` は ABI モジュール private のため、areka からは vtable 直呼びで到達する
    /// （`shiori_session.rs` の Raise/Complete と同一技法）。
    ///
    /// # Safety
    /// `brain` は有効な `IShiori` COM ポインタ。
    unsafe fn call_request(brain: &IShiori) -> HRESULT {
        let input = HSTRING::from("\\0OnBoot\\e");
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        unsafe {
            (Interface::vtable(brain).Request)(
                brain.as_raw(),
                &input as *const HSTRING,
                &mut out_response as *mut HSTRING,
                &mut out_token as *mut u64,
            )
        }
    }

    /// COM ポインタ経由（vtable 直呼び）で `Request` を呼び、HRESULT と move-out された
    /// 応答 HSTRING の両方を返すヘルパ（即時応答の往復観測用）。
    ///
    /// # Safety
    /// `brain` は有効な `IShiori` COM ポインタ。
    unsafe fn call_request_with(brain: &IShiori, input: &HSTRING) -> (HRESULT, HSTRING) {
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        let hr = unsafe {
            (Interface::vtable(brain).Request)(
                brain.as_raw(),
                input as *const HSTRING,
                &mut out_response as *mut HSTRING,
                &mut out_token as *mut u64,
            )
        };
        (hr, out_response)
    }

    /// COM ポインタ経由（vtable 直呼び）で `Load` を呼ぶヘルパ。
    ///
    /// # Safety
    /// `brain` は有効な `IShiori` COM ポインタ、`host` は有効な host raw ポインタ。
    unsafe fn call_load(brain: &IShiori, host: *mut c_void) -> HRESULT {
        unsafe { (Interface::vtable(brain).Load)(brain.as_raw(), host) }
    }

    /// COM ポインタ経由（vtable 直呼び）で `Unload` を呼ぶヘルパ。
    ///
    /// # Safety
    /// `brain` は有効な `IShiori` COM ポインタ。
    unsafe fn call_unload(brain: &IShiori) -> HRESULT {
        unsafe { (Interface::vtable(brain).Unload)(brain.as_raw()) }
    }

    /// 未ロード（生成直後）の `Request` は `SHIORI_E_NOT_LOADED` を返すこと（要件 2.3/2.4）。
    #[test]
    fn request_before_load_returns_not_loaded() {
        let brain: IShiori = ReferenceBrain::new().into();
        let hr = unsafe { call_request(&brain) };
        assert_eq!(
            hr, SHIORI_E_NOT_LOADED,
            "未ロード状態の Request は SHIORI_E_NOT_LOADED を返すこと, got 0x{:08X}",
            hr.0
        );
    }

    /// `Load`→`Request`→`Unload`→`Request` のロード状態遷移を検証する（要件 2.1/2.2/2.3）。
    ///
    /// - ロード後の `Request` は `SHIORI_E_NOT_LOADED` を返さない（`S_OK`）。
    /// - `Unload` 後の `Request` は再び `SHIORI_E_NOT_LOADED` を返す。
    /// - `Load` で host を AddRef 保持し、`Unload` で Release すること（保持参照モデル）。
    #[test]
    fn load_then_request_then_unload_transitions_loaded_state() {
        let brain: IShiori = ReferenceBrain::new().into();
        let host: IShioriHost = DummyHost.into();

        // Load: host を渡してロードする（成功=S_OK）。
        let hr_load = unsafe { call_load(&brain, host.as_raw() as *mut c_void) };
        assert!(hr_load.is_ok(), "Load は S_OK を返すこと, got 0x{:08X}", hr_load.0);

        // 脳が host を AddRef 保持していること（保持参照モデル・要件 2.1）。
        let inner = unsafe { windows_core::AsImpl::<ReferenceBrain>::as_impl(&brain) };
        assert!(
            inner.held_host.borrow().is_some(),
            "Load で host を AddRef 保持していること"
        );

        // ロード後の Request は NOT_LOADED を返さない（S_OK で受理される・要件 2.3）。
        let hr_req = unsafe { call_request(&brain) };
        assert_ne!(
            hr_req, SHIORI_E_NOT_LOADED,
            "ロード後の Request は NOT_LOADED を返さないこと"
        );
        assert!(hr_req.is_ok(), "ロード後の Request は S_OK であること, got 0x{:08X}", hr_req.0);

        // Unload: host を Release し Unloaded へ遷移（成功=S_OK・要件 2.2）。
        let hr_unload = unsafe { call_unload(&brain) };
        assert!(hr_unload.is_ok(), "Unload は S_OK を返すこと, got 0x{:08X}", hr_unload.0);
        assert!(
            inner.held_host.borrow().is_none(),
            "Unload で host を Release していること"
        );

        // Unload 後の Request は再び NOT_LOADED（要件 2.3/2.4）。
        let hr_after = unsafe { call_request(&brain) };
        assert_eq!(
            hr_after, SHIORI_E_NOT_LOADED,
            "Unload 後の Request は再び SHIORI_E_NOT_LOADED を返すこと"
        );
    }

    /// ロード済みの即時応答が受信 content の不解釈エコーであること（要件 1.4/3.1/3.2/3.3/3.4/8.1）。
    ///
    /// 解析・スキーマ検証を試みれば破綻する不透明 content（さくらスクリプト様＋非 ASCII＋絵文字）を
    /// 渡し、`S_OK`＋`out_response == input` を厳密一致で検証する。content を UTF-16 のまま
    /// 往復させ、内容を不解釈のまま一致させることで不透明性を実証する。
    #[test]
    fn loaded_request_echoes_opaque_content_unchanged() {
        let brain: IShiori = ReferenceBrain::new().into();
        let host: IShioriHost = DummyHost.into();

        let hr_load = unsafe { call_load(&brain, host.as_raw() as *mut c_void) };
        assert!(hr_load.is_ok(), "Load は S_OK を返すこと, got 0x{:08X}", hr_load.0);

        // 解析・検証を試みれば破綻する不透明 content（さくらスクリプト様・非 ASCII・絵文字）。
        let input = HSTRING::from("\\h\\s[0]日本語opaque😶");
        let (hr, out_response) = unsafe { call_request_with(&brain, &input) };

        assert_eq!(hr, S_OK, "ロード済みの即時応答は S_OK を返すこと, got 0x{:08X}", hr.0);
        assert_eq!(
            out_response, input,
            "即時応答は受信 content の不解釈エコー（厳密一致）であること"
        );
    }
}
