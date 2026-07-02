//! 製品コード（非 `#[cfg(test)]`）の **SHIORI リファレンス脳＋ファクトリ＋C 入口**。
//!
//! 新 ABI（`areka-P0-host32-shiori-load` WS-B）の **正解見本（canonical reference example）**。
//! - [`ReferenceFactory`]（`#[implement(IShioriFactory)]`）: 生成＋load 融合の唯一の生成経路。
//!   `CreateInstance` で [`ReferenceBrain`] を構築（host clone 保持・load_dir/shiori_name 保持し
//!   観測可能に）し、load 完了済みの `IShiori` を `out` へ move-out する（要件 8.3/8.4/1.3）。
//! - [`ReferenceBrain`]（`#[implement(IShiori)]`）: `Get`（即時/遅延の 2 分岐）＋`Notify`（片道通知）
//!   のみを持つ痩身脳。`Load`/`Unload`・「未ロード状態」は存在しない（create 融合＋Drop teardown・
//!   要件 9.1/9.2）。
//! - [`shiori_factory`]: `IShioriFactory` 実体を生成する唯一の純粋C コンストラクタ（要件 8.5/11.2）。
//!   旧 `shiori_create` は残置しない（要件 8.5）。
//!
//! # GET/NOTIFY の SHIORI/3.0 意味対応（要件 9.3）
//! - [`Get`](ReferenceBrain::Get)（GET SHIORI/3.0 後継）: 同期 request。応答を要する。即時応答
//!   （`S_OK`＋エコー move-out）または遅延（`SHIORI_S_PENDING`＋相関トークン、後で `complete`）。
//! - [`Notify`](ReferenceBrain::Notify)（NOTIFY SHIORI/3.0 後継）: 片道通知。応答を返さない。
//!   本見本は**受領ログ**（`notifications`）へ記録し、片道性を観測可能化する（設計判断 (g)）。
//!
//! # 痩身と保持参照モデル（要件 1.3/9.1）
//! `ReferenceBrain` は construction 時に host（clone 保持）・load_dir・shiori_name を確定し、以後
//! **不変フィールド**として保持する。load_dir/shiori_name を保持して観測可能にすることが、`load_dir`
//! が下から上まで per-instance で貫通する（D1）ことの E2E 証拠材料であり、native 脳は「検証または
//! 無視」できる単一 create の正解見本となる（要件 1.3/8.3）。「未ロード状態」は存在しないため
//! `loaded` フラグ・`SHIORI_E_NOT_LOADED` は撤去された（要件 9.1）。
//!
//! # content 不透明・固定／エコー方針（opaque content policy・要件 9.3）
//! content（リクエスト・応答・通知の本文）は **不透明な UTF-16 HSTRING** のまま固定／エコーで
//! 取り回す。本脳は content を **解析・スキーマ検証・分割・デコード・意味づけ・内容分岐しない**。
//! 即時応答は受信 content の純粋なエコー（無加工コピー）、遅延応答・能動通知は呼び出し側供給の
//! 固定／既知文字列をそのまま往復させる。即時／遅延の判別も content ではなく CONTROL フラグ
//! （`defer_next`）で行う。
//!
//! # 上流設計 SSOT への参照
//! アーキテクチャ上の上流設計 SSOT は `doc/COMPAT_ARCHITECTURE.md` §5 を参照（本 doc では内容を
//! 複製しない・参照リンクのみ）。正準 content プロトコルの語彙は完了仕様
//! `areka-P0-shiori-protocol` を唯一の正本として参照する（SSOT の二重定義禁止）。

#![allow(non_snake_case)] // `#[implement(...)]` の生成面が PascalCase メソッドを要求する。

use core::cell::{Cell, RefCell};
use core::ffi::c_void;

use shiori_abi::error::SHIORI_S_PENDING;
use shiori_abi::interface::{
    IShiori, IShiori_Impl, IShioriFactory, IShioriFactory_Impl, IShioriHost,
};
use shiori_abi::outcome::{CorrelationToken, CorrelationTokenAllocator};
use windows_core::{HRESULT, HSTRING, Interface, OutRef, Ref, Result, implement};

/// `S_OK`（成功・即時応答）の HRESULT。
const S_OK: HRESULT = HRESULT(0);

/// 製品コードの最小リファレンス脳（`#[implement(IShiori)]`・痩身）。
///
/// construction 時に host（clone 保持）・load_dir・shiori_name を確定し、以後不変フィールドとして
/// 保持する（要件 1.3/9.1）。`Get`＝即時エコー/遅延、`Notify`＝受領ログ。「未ロード状態」は存在しない。
#[implement(IShiori)]
pub struct ReferenceBrain {
    /// `CreateInstance` で AddRef（clone）保持した host（不変・construction 時確定）。
    ///
    /// 遅延 `complete_pending` および能動 `fire_raise` で **safe `complete`/`raise`** を発火する
    /// 保持 host（vtable 直呼び廃止・要件 12.5）。
    held_host: IShioriHost,
    /// construction 時に束縛した load_dir（不変・観測可能＝D1 貫通の証拠材料・要件 1.3）。
    load_dir: HSTRING,
    /// construction 時に束縛した shiori_name（不変・観測可能・要件 1.3）。
    shiori_name: HSTRING,
    /// 遅延応答の相関トークンアロケータ（単調増加採番）。
    ///
    /// 遅延扱いの `Get` ごとに [`CorrelationTokenAllocator::next`] でトークンを採番する。
    tokens: CorrelationTokenAllocator,
    /// 次の `Get` を遅延扱いにするか（CONTROL フラグ・content は解析しない）。
    ///
    /// `true` のとき次の `Get` は即時エコーではなく `SHIORI_S_PENDING`＋トークンを返す。
    /// 発火時に消費（`false` へ戻す）する one-shot。
    defer_next: Cell<bool>,
    /// 採番済み・未完了の相関トークン（完了まで突合可能に保持・one-shot）。
    ///
    /// 遅延 `Get` が採番したトークンを保持し、[`complete_pending`](ReferenceBrain::complete_pending)
    /// が取り出して（クリアして）`complete` の突合に用いる。
    pending_token: Cell<Option<CorrelationToken>>,
    /// `Notify`（片道通知）の受領ログ（片道性の観測可能化・設計判断 (g)）。
    ///
    /// `Notify` は応答を返さないため、受領した content をここへ記録して `AsImpl` 経由で test/デモが
    /// 観測できるようにする。
    notifications: RefCell<Vec<HSTRING>>,
}

impl ReferenceBrain {
    /// host・load_dir・shiori_name を束縛して脳を生成する（construction 時確定・不変）。
    ///
    /// [`ReferenceFactory::CreateInstance`] がこの経路で脳を構築し、`IShiori` へ move-out する。
    pub fn new(host: IShioriHost, load_dir: HSTRING, shiori_name: HSTRING) -> Self {
        Self {
            held_host: host,
            load_dir,
            shiori_name,
            tokens: CorrelationTokenAllocator::new(),
            defer_next: Cell::new(false),
            pending_token: Cell::new(None),
            notifications: RefCell::new(Vec::new()),
        }
    }

    /// 束縛済みの load_dir（D1 貫通の観測・テスト用・要件 1.3）。
    pub fn load_dir(&self) -> &HSTRING {
        &self.load_dir
    }

    /// 束縛済みの shiori_name（観測・テスト用・要件 1.3）。
    pub fn shiori_name(&self) -> &HSTRING {
        &self.shiori_name
    }

    /// `Notify` の受領ログのスナップショット（片道性の観測・テスト用・設計判断 (g)）。
    pub fn notifications(&self) -> Vec<HSTRING> {
        self.notifications.borrow().clone()
    }

    /// 次の [`Get`](ReferenceBrain::Get) を遅延扱いに武装する（CONTROL・one-shot）。
    ///
    /// content を解析せず即時／遅延を判別するための制御フラグ。武装後の最初の `Get` は
    /// 即時エコーではなく `SHIORI_S_PENDING`＋採番トークンを返し、フラグは消費される。
    pub fn arm_defer_next(&self) {
        self.defer_next.set(true);
    }

    /// 保持中の遅延応答を完了し、保持 host へ **safe** `complete(token, response)` を発火する（要件 12.5）。
    ///
    /// 採番済みトークンを `pending_token` から取り出し（クリアして one-shot）、保持 host の
    /// snake_case 安全面 [`IShioriHost::complete`] を呼ぶ（vtable 直呼びを廃止）。`response` は
    /// 不透明 HSTRING のまま渡し、内容を解析しない。
    ///
    /// # 戻り値
    /// host の `complete` の結果（`Ok(())` / [`shiori_abi::error::ShioriError`]）。
    ///
    /// # Panics
    /// 保持中の遅延応答が無い場合は呼び出し前提を満たさないため panic する。
    pub fn complete_pending(
        &self,
        response: &HSTRING,
    ) -> core::result::Result<(), shiori_abi::error::ShioriError> {
        // one-shot: 取り出して以降の二重発火を防ぐ。
        let token = self
            .pending_token
            .take()
            .expect("a deferred request is pending");
        // vtable 直呼び廃止: snake_case 安全面で発火する（要件 12.5）。
        self.held_host.complete(token, response)
    }

    /// 保持 host へ能動通知 **safe** `raise(script)` を発火する（要件 12.5）。
    ///
    /// 保持 host の snake_case 安全面 [`IShioriHost::raise`] を呼ぶ（vtable 直呼びを廃止）。`script` は
    /// 固定または既知の不透明 HSTRING を呼び出し側が供給する。脳は内容を解析・意味づけしない。
    ///
    /// # 戻り値
    /// host の `raise` の結果（`Ok(())` / [`shiori_abi::error::ShioriError`]）。
    pub fn fire_raise(
        &self,
        script: &HSTRING,
    ) -> core::result::Result<(), shiori_abi::error::ShioriError> {
        self.held_host.raise(script)
    }
}

impl IShiori_Impl for ReferenceBrain_Impl {
    /// 同期 request（GET SHIORI/3.0 後継・要件 9.1/9.3）。CONTROL の `defer_next` で即時／遅延を判別する
    /// （content は解析しない）:
    ///
    /// - 遅延武装時（`defer_next == true`）: フラグを消費し、[`CorrelationTokenAllocator::next`]
    ///   で単調増加トークンを採番して `pending_token` に保持・`out_token` へ書き出し、`out_response`
    ///   には何も書かずに [`SHIORI_S_PENDING`]（成功・遅延）を返す。完了は後続の
    ///   [`complete_pending`](ReferenceBrain::complete_pending) が保持 host へ `complete` で発火する。
    /// - 即時時（既定）: 受信 content の即時エコー応答を `out_response` へ move-out し `S_OK` を返す。
    ///
    /// content は不透明 HSTRING（UTF-16）のまま取り回し、`input` を解析・スキーマ検証・分割・
    /// デコード・内容分岐しない。即時エコーは純粋なコピー（受信 content をそのまま往復）である。
    unsafe fn Get(
        &self,
        input: &HSTRING,
        out_response: &mut HSTRING,
        out_token: &mut u64,
    ) -> HRESULT {
        if self.defer_next.get() {
            // 遅延武装: フラグを消費（one-shot）し、単調増加トークンを採番して保持する。
            self.defer_next.set(false);
            let token = self.tokens.next();
            self.pending_token.set(Some(token));
            // `out_token` に相関トークンを書き出す。即時応答文字列は伴わない（out_response 未書込）。
            *out_token = token.0;
            return SHIORI_S_PENDING;
        }
        // 即時: `input` は [in] 借用（読み取りのみ・解放しない）。エコーのため clone で所有 HSTRING を得る。
        // callee 確保の HSTRING を out_response へ move-out（所有権規約・caller 解放）。
        *out_response = input.clone();
        S_OK
    }

    /// 片道通知（NOTIFY SHIORI/3.0 後継・応答なし・要件 9.1/9.3）。
    ///
    /// 受領 content を **受領ログ**（`notifications`）へ clone して記録し、片道性を観測可能化する
    /// （設計判断 (g)）。content は不透明のまま解析しない。
    unsafe fn Notify(&self, input: &HSTRING) -> Result<()> {
        // `[in]` 借用: 保持するため clone して受領ログへ記録する（片道性の観測可能化）。
        self.notifications.borrow_mut().push(input.clone());
        Ok(())
    }
}

/// 新 ABI の生成面（`#[implement(IShioriFactory)]`）。生成＋load 融合の唯一の正解見本（要件 8.3/8.4）。
///
/// `CreateInstance` は [`ReferenceBrain`] を構築（host clone 保持・load_dir/shiori_name 保持し
/// 観測可能に）し、load 完了済みの `IShiori` を `out` へ move-out する。失敗時は out 未書込
/// （半構築非露出・要件 8.6）。本見本は「単一 create の正解見本」として native/下流が参照する。
#[implement(IShioriFactory)]
pub struct ReferenceFactory;

impl ReferenceFactory {
    /// factory を生成する。
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReferenceFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl IShioriFactory_Impl for ReferenceFactory_Impl {
    /// 脳を生成し、load 完了済みの `IShiori` を `out` へ move-out する（要件 8.3/8.4/1.3）。
    ///
    /// - `host` を clone（AddRef）して [`ReferenceBrain`] へ保持させる（共同所有・要件 10.4）。
    /// - `load_dir`/`shiori_name` を clone して脳に保持させ、観測可能にする（D1 貫通の証拠・要件 1.3）。
    /// - 構築物（refcount 1 の `IShiori`）を `out` へ move-out する。
    /// - 失敗時は `out` を書かない（半構築非露出・要件 8.6）。本見本は host 非在時のみ失敗する。
    unsafe fn CreateInstance(
        &self,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
        host: Ref<'_, IShioriHost>,
        out: OutRef<'_, IShiori>,
    ) -> Result<()> {
        // host は Ref 借用。保持するため clone（AddRef）する。host 非在は失敗（out 未書込・R8.6）。
        let host: IShioriHost = host
            .as_ref()
            .ok_or_else(|| windows_core::Error::from(windows::Win32::Foundation::E_POINTER))?
            .clone();
        // load_dir/shiori_name を束縛（観測可能に保持・D1 貫通の証拠・R1.3）。
        let brain: IShiori =
            ReferenceBrain::new(host, load_dir.clone(), shiori_name.clone()).into(); // refcount 1
        // load 完了済み brain を out へ move-out（callee 確保・caller Release）。
        out.write(Some(brain))?;
        Ok(())
    }
}

/// `IShioriFactory` 実体を生成する唯一の純粋C コンストラクタ（新 ABI の生成入口・要件 8.5/11.2）。
///
/// 成功時は参照カウント 1 の `IShioriFactory` を `out` へ move-out し `S_OK` を返す。失敗時は
/// `out` を書き込まず判別可能な失敗 HRESULT を返す（writes-on-success 不変条件）。旧 `shiori_create`
/// は残置しない（要件 8.5）。
///
/// 呼出規約は Windows COM 標準の `extern "system"`（＝`__stdcall`・x64／ARM64 では `extern "C"`
/// と同一 ABI だが、COM ABI 整合で正準表記は `system`）。C リンケージ（非マングル）は
/// `#[unsafe(no_mangle)]` が担保する（edition 2024 形）。本署名は将来 host-32 が
/// `GetProcAddress("shiori_factory")` で引ける形を満たす（正解見本・要件 11.2）。
///
/// # Safety
/// `out` は非 NULL の有効な書込先ポインタであること（呼び出し側が保証）。`out` が NULL の
/// 場合は防御的に `E_POINTER` を返し、何も書き込まない。成功時 `out` が受け取る `IShioriFactory` は
/// 参照カウント 1 で、呼び出し側が `Release`（drop）する義務を負う。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn shiori_factory(out: *mut *mut c_void) -> HRESULT {
    // 前提の防御: out が NULL なら何も書き込まず判別可能な失敗を返す。
    if out.is_null() {
        return windows::Win32::Foundation::E_POINTER;
    }
    // refcount 1 の IShioriFactory を構築し、所有権を out へ move-out する。
    let factory: IShioriFactory = ReferenceFactory::new().into(); // refcount 1
    // Safety: `out` は非 NULL の有効な書込先（呼び出し側保証・上で NULL を排除済み）。
    // into_raw は参照を変えずに所有権を移譲する（caller が単一参照を所有・Release 義務）。
    unsafe { *out = factory.into_raw() };
    S_OK
}

#[cfg(test)]
mod tests {
    //! 新 ABI の正解見本の単体テスト（要件 1.3/8.3/8.4/8.5/8.6/9.1/9.3/11.2）。
    //!
    //! `shiori_factory`→`create`→`get`/`notify`/`complete`/`raise` を安全面メソッドで駆動し、
    //! 痩身（Load/Unload 不在）・保持観測（load_dir/shiori_name/notifications）・即時/遅延・受領ログを検証する。

    use super::*;
    use core::ffi::c_void;

    use crate::shiori_host::{HostMessage, ShioriHostSink};
    use shiori_abi::interface::IShiori;
    use shiori_abi::outcome::{CorrelationToken, GetOutcome};
    use windows_core::AsImpl;

    /// `shiori_factory` で factory を得て、host/load_dir/shiori_name を渡して brain を create するヘルパ。
    ///
    /// `handle` は create で得た `IShiori`、`host` は sink（観測用）。
    fn make_brain(defer: bool) -> (IShiori, IShioriHost) {
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert!(hr.is_ok(), "shiori_factory は成功時 S_OK, got 0x{:08X}", hr.0);
        assert!(!out.is_null(), "成功時は out へ非 NULL の IShioriFactory を書き出すこと");
        let factory = unsafe { IShioriFactory::from_raw(out) };

        let host: IShioriHost = ShioriHostSink::new().into();
        let brain = factory
            .create(&HSTRING::from("C:/ghost/master"), &HSTRING::from("reference"), &host)
            .expect("create は Ok で IShiori 直返し");
        if defer {
            unsafe { AsImpl::<ReferenceBrain>::as_impl(&brain) }.arm_defer_next();
        }
        (brain, host)
    }

    /// 脳実体参照を取り出す（観測用）。
    fn brain_of(brain: &IShiori) -> &ReferenceBrain {
        unsafe { AsImpl::<ReferenceBrain>::as_impl(brain) }
    }

    /// `shiori_factory` が成功時に refcount 1 の `IShioriFactory` を out へ move-out し `S_OK` を
    /// 返すこと（要件 8.5/11.2・writes-on-success）。
    #[test]
    fn shiori_factory_outputs_refcount_one_factory_on_success() {
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert!(hr.is_ok(), "shiori_factory は成功時 S_OK, got 0x{:08X}", hr.0);
        assert!(!out.is_null(), "成功時は out へ非 NULL を書き出すこと");

        // 手渡された単一参照を adopt（from_raw は AddRef しない）。
        let factory = unsafe { IShioriFactory::from_raw(out) };

        // refcount 1 検証: IUnknown へ cast（+1）してから AddRef/Release を一往復。
        let unk: windows_core::IUnknown = factory.cast().expect("IUnknown へ cast");
        let after_add = unsafe { (Interface::vtable(&unk).AddRef)(unk.as_raw()) };
        let after_rel = unsafe { (Interface::vtable(&unk).Release)(unk.as_raw()) };
        assert_eq!(after_add, 3, "AddRef は cast 後ベースライン 2 から 3, got {after_add}");
        assert_eq!(after_rel, 2, "Release は 3 から 2, got {after_rel}");
    }

    /// `shiori_factory(NULL)` は判別可能な失敗 HRESULT（`E_POINTER`）を返し、out を書き込まないこと。
    #[test]
    fn shiori_factory_with_null_out_returns_failure_without_writing() {
        let hr = unsafe { shiori_factory(core::ptr::null_mut()) };
        assert!(hr.is_err(), "NULL out は失敗 HRESULT, got 0x{:08X}", hr.0);
        assert_eq!(
            hr,
            windows::Win32::Foundation::E_POINTER,
            "NULL out の失敗は判別可能な E_POINTER, got 0x{:08X}",
            hr.0
        );
    }

    /// create が load_dir/shiori_name/host を脳に保持して観測可能にすること（D1 貫通・要件 1.3/8.3）。
    #[test]
    fn create_binds_and_observes_load_dir_and_shiori_name() {
        let (brain, _host) = make_brain(false);
        let inner = brain_of(&brain);
        assert_eq!(
            inner.load_dir(),
            &HSTRING::from("C:/ghost/master"),
            "load_dir が construction 時に束縛され観測可能であること（D1 貫通）"
        );
        assert_eq!(
            inner.shiori_name(),
            &HSTRING::from("reference"),
            "shiori_name が construction 時に束縛され観測可能であること"
        );
    }

    /// host 非在の create は失敗し out 未書込（半構築非露出・要件 8.6）。
    #[test]
    fn create_without_host_fails_without_exposing_half_construct() {
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert!(hr.is_ok());
        let factory = unsafe { IShioriFactory::from_raw(out) };

        // CreateInstance を host=None（Ref 空）で直呼びし、Err かつ out 未書込を確認する。
        let mut brain_out: Option<IShiori> = None;
        let result = unsafe {
            factory.CreateInstance(
                &HSTRING::from("dir"),
                &HSTRING::from("name"),
                Ref::from(None),
                (&mut brain_out).into(),
            )
        };
        assert!(result.is_err(), "host 非在の create は Err（半構築非露出）");
        assert!(brain_out.is_none(), "失敗時は out 未書込であること（半構築非露出・R8.6）");
    }

    /// 即時 `get` が受信 content の不解釈エコーであること（要件 9.3）。
    #[test]
    fn immediate_get_echoes_opaque_content_unchanged() {
        let (brain, _host) = make_brain(false);
        let input = HSTRING::from("\\h\\s[0]日本語opaque😶");
        let outcome = brain.get(&input).expect("即時 get は Ok");
        assert_eq!(
            outcome,
            GetOutcome::Immediate(input),
            "即時応答は受信 content の不解釈エコー（厳密一致）であること"
        );
    }

    /// 遅延武装した `get` は `Deferred(token)` を返し、`complete_pending` で保持 host へ
    /// `complete` が発火すること（要件 9.1/9.3/12.5）。
    #[test]
    fn deferred_get_then_complete_pending_fires_complete_on_held_host() {
        let (brain, host) = make_brain(true);
        let inner = brain_of(&brain);

        let outcome = brain
            .get(&HSTRING::from("\\h\\s[0]opaque-request"))
            .expect("遅延 get は Ok");
        let token = match outcome {
            GetOutcome::Deferred(t) => t,
            other => panic!("遅延武装後の get は Deferred, got {other:?}"),
        };
        assert_eq!(token, CorrelationToken(0), "初回の相関トークンは単調増加採番の 0");

        // sink の突合枠へ採番トークンをセットする（突合準備）。
        let sink_impl = unsafe { AsImpl::<ShioriHostSink>::as_impl(&host) };
        sink_impl.set_pending_token(token);

        // 完了応答を safe `complete` で発火する（vtable 直呼び廃止）。
        let response = HSTRING::from("\\h\\s[0]deferred応答😶\\e");
        inner
            .complete_pending(&response)
            .expect("complete_pending は host から Ok を受け取ること");

        // 保持 host が対応トークンと応答文字列を受領していること。
        assert_eq!(
            sink_impl.try_recv(),
            Some(HostMessage::Completed { token, response }),
            "保持 host へ対応トークンと応答文字列で complete が発火すること"
        );
    }

    /// `fire_raise` で保持 host へ能動通知 `raise(script)` を固定文字列で発火し、host が受領すること
    /// （要件 12.5）。
    #[test]
    fn fire_raise_fires_raise_on_held_host_with_fixed_content() {
        let (brain, host) = make_brain(false);
        let inner = brain_of(&brain);

        let script = HSTRING::from("\\h\\s[0]known-opaque-raise");
        inner
            .fire_raise(&script)
            .expect("fire_raise は host から Ok を受け取ること");

        let sink_impl = unsafe { AsImpl::<ShioriHostSink>::as_impl(&host) };
        assert_eq!(
            sink_impl.try_recv(),
            Some(HostMessage::Raised(script)),
            "保持 host へ固定文字列で raise が発火すること"
        );
    }

    /// `notify`（片道通知）が受領ログへ記録され、応答を返さないこと（片道性・要件 9.3・設計判断 (g)）。
    #[test]
    fn notify_records_receipt_log_and_returns_no_response() {
        let (brain, _host) = make_brain(false);
        let inner = brain_of(&brain);

        assert!(inner.notifications().is_empty(), "初期状態の受領ログは空");

        let n1 = HSTRING::from("NOTIFY SHIORI/3.0 OnFirstBoot");
        let n2 = HSTRING::from("NOTIFY SHIORI/3.0 OnOtherGhostBooted😶");
        brain.notify(&n1).expect("notify は Ok（片道・応答なし）");
        brain.notify(&n2).expect("notify は Ok");

        // 受領ログに順序どおり記録されること（片道性の観測可能化）。
        assert_eq!(
            inner.notifications(),
            vec![n1, n2],
            "notify の受領 content が順序どおり受領ログへ記録されること"
        );
    }
}
