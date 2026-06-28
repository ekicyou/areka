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

use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, Ordering};

use shiori_abi::error::{SHIORI_E_NOT_LOADED, SHIORI_S_PENDING};
use shiori_abi::interface::{IShiori, IShiori_Impl, IShioriHost};
use shiori_abi::outcome::{CorrelationToken, CorrelationTokenAllocator};
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
    /// 遅延 `Complete`（task 2.3・[`complete_pending`](ReferenceBrain::complete_pending)）および
    /// 能動 `Raise`（task 2.4・[`fire_raise`](ReferenceBrain::fire_raise)）で vtable 直呼び発火する保持 host。
    held_host: RefCell<Option<IShioriHost>>,
    /// 遅延応答の相関トークンアロケータ（単調増加採番・Decision 5）。
    ///
    /// 遅延扱いの `Request` ごとに [`CorrelationTokenAllocator::next`] でトークンを採番する。
    tokens: CorrelationTokenAllocator,
    /// 次の `Request` を遅延扱いにするか（CONTROL フラグ・content は解析しない）。
    ///
    /// `true` のとき次の `Request` は即時エコーではなく `SHIORI_S_PENDING`＋トークンを返す。
    /// 発火時に消費（`false` へ戻す）する one-shot。デモ（task 3.1）は `AsImpl` で本型へ
    /// ダウンキャストし [`arm_defer_next`](ReferenceBrain::arm_defer_next) で武装する。
    defer_next: Cell<bool>,
    /// 採番済み・未完了の相関トークン（完了まで突合可能に保持・one-shot）。
    ///
    /// 遅延 `Request` が採番したトークンを保持し、[`complete_pending`](ReferenceBrain::complete_pending)
    /// が取り出して（クリアして）`Complete` の突合に用いる。`CorrelationToken` は `Copy`。
    pending_token: Cell<Option<CorrelationToken>>,
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
            defer_next: Cell::new(false),
            pending_token: Cell::new(None),
        }
    }

    /// 次の [`Request`](IShiori_Impl::Request) を遅延扱いに武装する（CONTROL・one-shot）。
    ///
    /// content を解析せず即時／遅延を判別するための制御フラグ。武装後の最初の `Request` は
    /// 即時エコーではなく `SHIORI_S_PENDING`＋採番トークンを返し、フラグは消費される。
    /// デモ／テストは `AsImpl` で本型へダウンキャストしてから本メソッドを呼ぶ。
    pub fn arm_defer_next(&self) {
        self.defer_next.set(true);
    }

    /// 保持中の遅延応答を完了し、保持 host へ `Complete(token, response)` を発火する（要件 4.2／4.4）。
    ///
    /// 採番済みトークンを `pending_token` から取り出し（クリアして one-shot）、保持 host へ
    /// **vtable 直呼び**で `Complete` を発火する（ABI private な raw メソッドへ別クレートから
    /// 到達する唯一の手段・design.md §Architecture「脳→host 呼び出しイディオム」）。`response` は
    /// 不透明 HSTRING のまま渡し、内容を解析しない（要件 1.4／8.1）。
    ///
    /// 戻り値は host の `Complete` が返した HRESULT。host が未保持、または保持中の遅延応答が
    /// 無い場合は呼び出し前提を満たさないため panic する（呼び出し側はライフサイクル上、遅延
    /// `Request` 発行後・`Unload` 前にのみ本メソッドを呼ぶ）。
    pub fn complete_pending(&self, response: &HSTRING) -> HRESULT {
        let held = self.held_host.borrow();
        let host = held.as_ref().expect("host held");
        // one-shot: 取り出して以降の二重発火を防ぐ。
        let token = self
            .pending_token
            .take()
            .expect("a deferred request is pending");
        // Safety: `host` は Load で AddRef 保持した有効な IShioriHost。raw `Complete` は ABI private
        // のため vtable 直呼びで到達する。`response` は本呼び出し中有効な `*const HSTRING`（borrow）。
        unsafe {
            (Interface::vtable(host).Complete)(host.as_raw(), token.0, response as *const HSTRING)
        }
    }

    /// 保持 host へ能動通知 `Raise(script)` を発火する（要件 5.1／5.2）。
    ///
    /// 保持 host へ **vtable 直呼び**で `Raise` を発火する（ABI private な raw メソッドへ別クレートから
    /// 到達する唯一の手段・[`complete_pending`](ReferenceBrain::complete_pending) と同一技法）。`script` は
    /// 固定または既知の不透明 HSTRING を呼び出し側（デモ・task 3.1）が供給する。脳は内容を解析・スキーマ
    /// 検証・意味づけせず、不透明 content のまま渡す（要件 5.2／1.4／8.1）。
    ///
    /// 戻り値は host の `Raise` が返した HRESULT。host が未保持（`Load` 未呼び／`Unload` 済み）の場合は
    /// 呼び出し前提を満たさないため panic する（呼び出し側はライフサイクル上、`Load` 後・`Unload` 前にのみ
    /// 本メソッドを呼ぶ）。
    pub fn fire_raise(&self, script: &HSTRING) -> HRESULT {
        let held = self.held_host.borrow();
        let host = held.as_ref().expect("host held");
        // Safety: `host` は Load で AddRef 保持した有効な IShioriHost。raw `Raise` は ABI private のため
        // vtable 直呼びで到達する。`script` は本呼び出し中有効な `*const HSTRING`（[in] 借用・解放しない）。
        unsafe { (Interface::vtable(host).Raise)(host.as_raw(), script as *const HSTRING) }
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

    /// 未ロード時は [`SHIORI_E_NOT_LOADED`]（out-param 未書込）。ロード時は CONTROL の
    /// `defer_next` で即時／遅延を判別する（content は解析しない・要件 4.1）:
    ///
    /// - 遅延武装時（`defer_next == true`）: フラグを消費し、[`CorrelationTokenAllocator::next`]
    ///   で単調増加トークンを採番して `pending_token` に保持・`out_token` へ書き出し、`out_response`
    ///   には何も書かずに [`SHIORI_S_PENDING`]（成功・遅延）を返す（要件 4.1／4.2）。完了は後続の
    ///   [`complete_pending`](ReferenceBrain::complete_pending) が保持 host へ `Complete` で発火する。
    /// - 即時時（既定）: 受信 content の即時エコー応答を `out_response` へ move-out し `S_OK` を返す。
    ///
    /// content は不透明 HSTRING（UTF-16）のまま取り回し、`input` を解析・スキーマ検証・分割・
    /// デコード・内容分岐しない。即時エコーは純粋なコピー（受信 content をそのまま往復）であり、
    /// 不解釈のまま一致することで content 不透明性を実証する（要件 1.4／8.1）。
    unsafe fn Request(
        &self,
        input: *const HSTRING,
        out_response: *mut HSTRING,
        out_token: *mut u64,
    ) -> HRESULT {
        if !self.loaded.load(Ordering::SeqCst) {
            // 未ロード: 有効な処理として受理せず判別可能な失敗を返す。out-param は未書込（要件 2.4）。
            return SHIORI_E_NOT_LOADED;
        }
        if self.defer_next.get() {
            // 遅延武装: フラグを消費（one-shot）し、単調増加トークンを採番して保持する（Decision 5）。
            self.defer_next.set(false);
            let token = self.tokens.next();
            self.pending_token.set(Some(token));
            // `out_token` に相関トークンを書き出す。即時応答文字列は伴わない（out_response 未書込）。
            // Safety: `out_token` は呼び出し中有効な出力スロット（caller 確保）。
            unsafe { core::ptr::write(out_token, token.0) };
            return SHIORI_S_PENDING;
        }
        // ロード済み・即時: `input` は [in] 借用（読み取りのみ・解放しない）。エコーのため clone で
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

    use crate::shiori_host::{HostMessage, ShioriHostSink};
    use shiori_abi::error::SHIORI_S_PENDING;
    use shiori_abi::outcome::CorrelationToken;

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

    /// COM ポインタ経由（vtable 直呼び）で `Request` を呼び、HRESULT・採番トークン・
    /// move-out された応答 HSTRING を返すヘルパ（遅延応答の観測用）。
    ///
    /// `out_response` は呼び出し前に番兵文字列で初期化しておき、遅延時に「書き込まれていない」
    /// （= 番兵のまま）ことを観測できるようにする。
    ///
    /// # Safety
    /// `brain` は有効な `IShiori` COM ポインタ。
    unsafe fn call_request_observed(
        brain: &IShiori,
        input: &HSTRING,
        response_sentinel: &HSTRING,
    ) -> (HRESULT, u64, HSTRING) {
        let mut out_response = response_sentinel.clone();
        let mut out_token: u64 = u64::MAX;
        let hr = unsafe {
            (Interface::vtable(brain).Request)(
                brain.as_raw(),
                input as *const HSTRING,
                &mut out_response as *mut HSTRING,
                &mut out_token as *mut u64,
            )
        };
        (hr, out_token, out_response)
    }

    /// 遅延武装した `Request` は `SHIORI_S_PENDING`＋採番トークンを返し、`out_response` を
    /// 書き込まないこと（要件 4.1/4.2）。
    ///
    /// - `arm_defer_next()` で次の Request を遅延扱いにする（content を解析せず CONTROL で判別）。
    /// - HRESULT は成功コード `SHIORI_S_PENDING`。
    /// - `out_token` は `CorrelationTokenAllocator` の単調増加採番（初回=0・Decision 5）。
    /// - `out_response` は未書込（番兵のまま）= 即時応答文字列を伴わない。
    #[test]
    fn armed_deferred_request_returns_pending_and_token_without_response() {
        let brain: IShiori = ReferenceBrain::new().into();
        let host: IShioriHost = DummyHost.into();

        let hr_load = unsafe { call_load(&brain, host.as_raw() as *mut c_void) };
        assert!(hr_load.is_ok(), "Load は S_OK を返すこと, got 0x{:08X}", hr_load.0);

        // CONTROL で遅延を武装する（content は不透明・解析しない）。
        let inner = unsafe { windows_core::AsImpl::<ReferenceBrain>::as_impl(&brain) };
        inner.arm_defer_next();

        let sentinel = HSTRING::from("__not_written__");
        let input = HSTRING::from("\\h\\s[0]日本語opaque😶");
        let (hr, out_token, out_response) =
            unsafe { call_request_observed(&brain, &input, &sentinel) };

        assert_eq!(
            hr, SHIORI_S_PENDING,
            "遅延武装した Request は SHIORI_S_PENDING を返すこと, got 0x{:08X}",
            hr.0
        );
        assert_eq!(
            out_token, 0,
            "初回の相関トークンは単調増加採番の 0 であること, got {out_token}"
        );
        assert_eq!(
            out_response, sentinel,
            "遅延応答は即時応答文字列を伴わない（out_response 未書込）こと"
        );
    }

    /// 遅延 Request 発行後に `complete_pending` で保持 host へ `Complete(token, response)` を
    /// 発火し、host が対応トークンと応答文字列を受領すること（要件 4.2/4.4）。
    ///
    /// 観測には [`ShioriHostSink`] を host として用い、メールボックスに投函される
    /// [`HostMessage::Completed`] を取り出して突合する。
    #[test]
    fn complete_pending_fires_complete_on_held_host() {
        let brain: IShiori = ReferenceBrain::new().into();
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();

        let hr_load = unsafe { call_load(&brain, host.as_raw() as *mut c_void) };
        assert!(hr_load.is_ok(), "Load は S_OK を返すこと, got 0x{:08X}", hr_load.0);

        // 遅延武装 → Request で相関トークン T を採番させる。
        let inner = unsafe { windows_core::AsImpl::<ReferenceBrain>::as_impl(&brain) };
        inner.arm_defer_next();

        let sentinel = HSTRING::from("__not_written__");
        let input = HSTRING::from("\\h\\s[0]opaque-request");
        let (hr_req, out_token, _) =
            unsafe { call_request_observed(&brain, &input, &sentinel) };
        assert_eq!(hr_req, SHIORI_S_PENDING, "遅延 Request は SHIORI_S_PENDING を返すこと");

        // sink の突合枠へ採番トークン T をセットする（単一 in-flight・突合準備）。
        let sink_impl = unsafe { windows_core::AsImpl::<ShioriHostSink>::as_impl(&host) };
        sink_impl.set_pending_token(CorrelationToken(out_token));

        // 完了応答を発火する（保持 host へ vtable 直呼び）。
        let response = HSTRING::from("\\h\\s[0]deferred応答😶\\e");
        let hr_complete = inner.complete_pending(&response);
        assert!(
            hr_complete.is_ok(),
            "complete_pending は host から S_OK を受け取ること, got 0x{:08X}",
            hr_complete.0
        );

        // 保持 host が対応トークン T と応答文字列を受領していること。
        assert_eq!(
            sink_impl.try_recv(),
            Some(HostMessage::Completed {
                token: CorrelationToken(out_token),
                response,
            }),
            "保持 host へ対応トークンと応答文字列で Complete が発火すること"
        );
    }

    /// `fire_raise` で保持 host へ能動通知 `Raise(script)` を固定（既知の不透明）文字列で
    /// 発火し、host が同一内容を受領すること（要件 5.1/5.2）。
    ///
    /// `script` はさくらスクリプト様の既知の不透明文字列。脳は内容を解釈せず（要件 5.2）、
    /// host が厳密一致で受領することで content 不透明性（往復）を実証する。観測には
    /// [`ShioriHostSink`] を host として用い、[`HostMessage::Raised`] を突合する。
    #[test]
    fn fire_raise_fires_raise_on_held_host_with_fixed_content() {
        let brain: IShiori = ReferenceBrain::new().into();
        let sink = ShioriHostSink::new();
        let host: IShioriHost = sink.into();

        let hr_load = unsafe { call_load(&brain, host.as_raw() as *mut c_void) };
        assert!(hr_load.is_ok(), "Load は S_OK を返すこと, got 0x{:08X}", hr_load.0);

        // 既知の不透明文字列（さくらスクリプト様）を能動通知として発火する。
        let inner = unsafe { windows_core::AsImpl::<ReferenceBrain>::as_impl(&brain) };
        let script = HSTRING::from("\\h\\s[0]known-opaque-raise");
        let hr_raise = inner.fire_raise(&script);
        assert!(
            hr_raise.is_ok(),
            "fire_raise は host から S_OK を受け取ること, got 0x{:08X}",
            hr_raise.0
        );

        // 保持 host が同一の不透明文字列を受領していること（厳密一致＝往復・不解釈）。
        let sink_impl = unsafe { windows_core::AsImpl::<ShioriHostSink>::as_impl(&host) };
        assert_eq!(
            sink_impl.try_recv(),
            Some(HostMessage::Raised(script)),
            "保持 host へ固定（既知の不透明）文字列で Raise が発火すること"
        );
    }
}
