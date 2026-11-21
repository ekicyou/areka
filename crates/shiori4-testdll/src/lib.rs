//! x64 SHIORI4 決定論テスト DLL（`shiori4-testdll`）の crate ルート。
//!
//! areka の「実 SHIORI 境界を踏む決定論テストゴースト」の**脳**を担う cdylib。正典イベント集合へ
//! 実 emo2 pasta 採取のゴールデンスナップショットを決定論 replay する。
//!
//! - 契約定数 [`DLL_FILE_NAME`]（出力 DLL ファイル名の単一権威）。
//! - 生成入口 [`shiori_factory`]（production SHIORI4 と同形の `extern "system"` エクスポート）＋
//!   生成面 [`ReplayFactory`]＋台本脳 [`ReplayBrain`]。
//! - `cargo test` がビルドした cdylib の場所を決める唯一の口 [`locate_built_dll`]（cargo 1.100 の
//!   新しい置き方にも対応・決め方は `locate` の説明を参照）。
//!
//! ## スレッド座・COM
//! 全 COM 参照は shiori アクタースレッド常駐で、`CoInitializeEx` は**呼ばない**（直接 vtable dispatch
//! のみ・アクティベーション/マーシャリング非使用）。in-crate 檻は COM 未初期化スレッドで走るが、
//! `windows-core` の `#[implement]` オブジェクトは直接 vtable dispatch ゆえ問題なく通る（reference_brain
//! の in-crate 檻と同律）。
//!
//! ## 依存境界
//! 本 crate は [`shiori-abi`](shiori_abi) ＋ [`windows-core`](windows_core) のみに依存する。
//! `windows`(Win32)・`tracing`・areka 系 crate へは依存しない（自給）。

// `#[implement(...)]` の生成面が PascalCase メソッドを要求する（reference_brain と同律）。
#![allow(non_snake_case)]

use core::ffi::c_void;

use shiori_abi::interface::{
    IShiori, IShiori_Impl, IShioriFactory, IShioriFactory_Impl, IShioriHost,
};
use windows_core::{HRESULT, HSTRING, Interface, OutRef, Ref, Result, implement};

/// SHIORI/3.0 リクエスト解析・応答分類の純粋ロジック（収載→凍結応答／未知→204／構造不整合→400）。
mod request;

/// 正典 GET イベントごとの凍結ゴールデンスナップショットの静的表。
///
/// `snapshots/<EventID>.txt`（SHIORI/3.0 応答全文）を `include_str!` でコンパイル時に埋め込み
/// （実行時 I/O ゼロ）、取り込み時に CRLF 正規化する（git EOL 変換への免疫）。
///
/// モジュールは非公開のまま crate ルートへ再エクスポートし、crate 外（areka-ghost の
/// **ドリフト検出 assert**）から届くようにする。
mod snapshot;
pub use snapshot::{PROVISIONAL_MARKER, snapshot_for, snapshots};

/// `cargo test` がビルドしたこの crate の cdylib の場所を決める唯一の口（試験の支え）。
mod locate;
pub use locate::locate_built_dll;

/// 出力 DLL ファイル名（契約定数・単一権威）。
///
/// この値は次の 2 箇所が共有する契約値である:
/// - テストゴースト fixture の `ghost/master/descript.txt` の `shiori,` 行。
/// - [`locate_built_dll`]／fixture への DLL コピー。
///
/// 命名（crate `shiori4-testdll`・`[lib] name = "shiori4_testdll"`）の帰結として、
/// x64 ネイティブビルドの cdylib 出力ファイル名は `shiori4_testdll.dll` になる。
pub const DLL_FILE_NAME: &str = "shiori4_testdll.dll";

/// `S_OK`（成功・即時応答）の HRESULT。
const S_OK: HRESULT = HRESULT(0);

/// `E_POINTER`（NULL/無効ポインタ）の HRESULT。
///
/// 本 crate は `windows`(Win32) へ依存しないため、[`windows_core::HRESULT`] のローカル定数として持つ。
const E_POINTER: HRESULT = HRESULT(0x8000_4003u32 as i32);

/// 実 IShiori 境界を再現的に横断させる決定論 replay 脳。
///
/// 保持状態は**不変データのみ**——`CreateInstance` で AddRef（clone）保持した host（parity 目的で
/// 保持するが**呼び返さない**）と、観測用の `load_dir`／`shiori_name` clone（reference_brain と同律）。
/// 乱数・実時計・環境変数への依存を持たない（i686 testdll の env フック類も持ち込まない）。
#[implement(IShiori)]
pub struct ReplayBrain {
    /// parity のため保持するが、本脳は**呼び返さない**（Raise/Complete/Property 不使用）。
    _held_host: IShioriHost,
    /// 観測用の clone（不変）。
    _load_dir: HSTRING,
    /// 観測用の clone（不変）。
    _shiori_name: HSTRING,
}

impl ReplayBrain {
    fn new(host: IShioriHost, load_dir: HSTRING, shiori_name: HSTRING) -> Self {
        Self {
            _held_host: host,
            _load_dir: load_dir,
            _shiori_name: shiori_name,
        }
    }
}

impl IShiori_Impl for ReplayBrain_Impl {
    /// 同期 request（GET SHIORI/3.0 後継）。**常に即時**応答で `SHIORI_S_PENDING` を返さない。
    ///
    /// 収載→凍結応答／未知・未収載→204／malformed（GET request line 不在・ID 欠落等）→400。
    /// 分岐は総和的な純関数のみで構成され **panic しない**。COM error HRESULT は返さない
    /// （プロトコル水準の 400 が fail-visible 面）。`out_token` は未使用（即時ゆえ）。
    unsafe fn Get(
        &self,
        input: &HSTRING,
        out_response: &mut HSTRING,
        _out_token: &mut u64,
    ) -> HRESULT {
        let request = input.to_string();
        let parsed = request::parse_request(&request);
        let selection = request::select_response(&parsed, snapshot::snapshot_for);
        let response = request::response_text(&selection);
        *out_response = HSTRING::from(response);
        S_OK
    }

    /// 片道通知（NOTIFY SHIORI/3.0 後継・応答なし）。content は解析せず、host へも呼び返さない。
    unsafe fn Notify(&self, _input: &HSTRING) -> Result<()> {
        Ok(())
    }
}

/// 生成＋load 融合の生成面（reference_brain のパターン踏襲）。
///
/// [`CreateInstance`](IShioriFactory_Impl::CreateInstance) は [`ReplayBrain`] を構築し、load 完了済みの
/// `IShiori` を `out` へ move-out する。host `Ref` 欠落時は `out` 未書込で失敗する（半構築非露出）。
#[implement(IShioriFactory)]
pub struct ReplayFactory;

impl ReplayFactory {
    fn new() -> Self {
        Self
    }
}

impl IShioriFactory_Impl for ReplayFactory_Impl {
    /// 脳を生成し、load 完了済みの `IShiori` を `out` へ move-out する。
    ///
    /// - host `Ref` 欠落 → [`E_POINTER`] を返し `out` を書かない（半構築非露出・reference_brain と同律）。
    /// - 成功時: `host` を clone（AddRef）して [`ReplayBrain`] へ保持させ（呼び返さない）、
    ///   refcount 1 の `IShiori` を `out` へ move-out する。
    unsafe fn CreateInstance(
        &self,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
        host: Ref<'_, IShioriHost>,
        out: OutRef<'_, IShiori>,
    ) -> Result<()> {
        let host: IShioriHost = host
            .as_ref()
            .ok_or_else(|| windows_core::Error::from(E_POINTER))?
            .clone();
        let brain: IShiori = ReplayBrain::new(host, load_dir.clone(), shiori_name.clone()).into();
        out.write(Some(brain))?;
        Ok(())
    }
}

/// `IShioriFactory` 実体を生成する純粋C コンストラクタ。
///
/// production SHIORI4 生成入口（`crates/areka/src/reference_brain.rs::shiori_factory`）と同形の
/// エクスポート ABI 形状（`#[unsafe(no_mangle)]`＋`extern "system"`＝Windows COM 標準の呼出規約・
/// x64/ARM64 では `extern "C"` と同一 ABI だが COM 整合で正準表記は `system`）。host-32／InProc 経路が
/// `GetProcAddress("shiori_factory")` で引ける形を満たす。
///
/// # Safety
/// `out` は非 NULL の有効な書込先ポインタであること（呼び出し側が保証）。`out` が NULL の場合は
/// 防御的に [`E_POINTER`] を返し、何も書き込まない。成功時 `out` が受け取る `IShioriFactory` は
/// 参照カウント 1 で、呼び出し側が `Release`（drop）する義務を負う。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn shiori_factory(out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    let factory: IShioriFactory = ReplayFactory::new().into();
    // SAFETY: `out` は非 NULL の有効な書込先（呼び出し側保証・上で NULL を排除済み）。
    // into_raw は参照を変えずに所有権を移譲する（caller が単一参照を所有・Release 義務）。
    unsafe { *out = factory.into_raw() };
    S_OK
}

#[cfg(test)]
mod tests {
    //! in-crate 檻。
    //!
    //! cdylib の場所の決め方は `locate` の試験が持つ。ここは `ReplayFactory`→`CreateInstance`→
    //! `Get`/`Notify` の vtable dispatch を実 COM 面で駆動する（reference_brain の流儀）。

    use super::*;

    /// 契約定数 [`DLL_FILE_NAME`] が `shiori4_testdll.dll` に固定されていること。
    #[test]
    fn dll_file_name_is_fixed_to_d8_naming() {
        assert_eq!(
            DLL_FILE_NAME, "shiori4_testdll.dll",
            "出力 DLL ファイル名は D-8 命名に固定されていること"
        );
    }

    // ---- ReplayFactory→CreateInstance→Get/Notify の vtable dispatch 檻 ----

    use core::ffi::c_void;
    use shiori_abi::error::SHIORI_S_PENDING;
    use shiori_abi::interface::{IShiori, IShioriFactory, IShioriHost, IShioriHost_Impl};
    use windows_core::{HSTRING, Interface, Ref, Result, implement};

    /// vtable dispatch 検証用の最小モック host。ReplayBrain は host を保持するが**呼び返さない**
    /// （Raise/Complete/Property 不使用）ため、4 メソッドは自明な受け皿でよい。
    #[implement(IShioriHost)]
    struct MockHost;

    impl IShioriHost_Impl for MockHost_Impl {
        unsafe fn Raise(&self, _script: &HSTRING) -> Result<()> {
            Ok(())
        }
        unsafe fn Complete(&self, _token: u64, _response: &HSTRING) -> Result<()> {
            Ok(())
        }
        unsafe fn GetProperty(&self, _key: &HSTRING, _out_value: &mut HSTRING) -> Result<()> {
            Ok(())
        }
        unsafe fn SetProperty(&self, _key: &HSTRING, _value: &HSTRING) -> Result<()> {
            Ok(())
        }
    }

    /// `shiori_factory` export から refcount 1 の `IShioriFactory` を得るヘルパ。
    fn make_factory() -> IShioriFactory {
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert!(
            hr.is_ok(),
            "shiori_factory は成功時 S_OK, got 0x{:08X}",
            hr.0
        );
        assert!(
            !out.is_null(),
            "成功時は out へ非 NULL の IShioriFactory を書き出すこと"
        );
        // 手渡された単一参照を adopt（from_raw は AddRef しない）。
        unsafe { IShioriFactory::from_raw(out) }
    }

    /// factory 経由で host/load_dir/shiori_name を渡し brain（`IShiori`）を create するヘルパ。
    fn make_brain() -> (IShiori, IShioriHost) {
        let factory = make_factory();
        let host: IShioriHost = MockHost.into();
        let mut out: Option<IShiori> = None;
        unsafe {
            factory
                .CreateInstance(
                    &HSTRING::from("C:/ghost/master"),
                    &HSTRING::from("shiori4-testdll"),
                    (&host).into(),
                    (&mut out).into(),
                )
                .expect("CreateInstance は Ok で IShiori を move-out すること");
        }
        (out.expect("out に brain が move-out されていること"), host)
    }

    /// GET リクエスト（`ID:` 付き）の SHIORI/3.0 テキストを組む。
    fn get_request(id: &str) -> HSTRING {
        HSTRING::from(format!(
            "GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: {id}\r\n\r\n"
        ))
    }

    /// `Get` を駆動し `(hr, out_response, out_token)` を返す。
    fn drive_get(brain: &IShiori, input: &HSTRING) -> (windows_core::HRESULT, HSTRING, u64) {
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        let hr = unsafe { brain.Get(input, &mut out_response, &mut out_token) };
        (hr, out_response, out_token)
    }

    /// `shiori_factory` 成功時に refcount 1 の `IShioriFactory` を out へ move-out し `S_OK` を返す
    /// こと（writes-on-success・reference_brain と同律）。
    #[test]
    fn shiori_factory_outputs_refcount_one_factory_on_success() {
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert!(
            hr.is_ok(),
            "shiori_factory は成功時 S_OK, got 0x{:08X}",
            hr.0
        );
        assert!(!out.is_null(), "成功時は out へ非 NULL を書き出すこと");

        let factory = unsafe { IShioriFactory::from_raw(out) };

        // refcount 1 検証: IUnknown へ cast（+1）してから AddRef/Release を一往復。
        let unk: windows_core::IUnknown = factory.cast().expect("IUnknown へ cast");
        let after_add = unsafe { (Interface::vtable(&unk).AddRef)(unk.as_raw()) };
        let after_rel = unsafe { (Interface::vtable(&unk).Release)(unk.as_raw()) };
        assert_eq!(
            after_add, 3,
            "AddRef は cast 後ベースライン 2 から 3, got {after_add}"
        );
        assert_eq!(after_rel, 2, "Release は 3 から 2, got {after_rel}");
    }

    /// `shiori_factory(NULL)` は判別可能な失敗（`E_POINTER`）を返し out を書き込まないこと
    /// （半構築非露出）。
    #[test]
    fn shiori_factory_with_null_out_returns_e_pointer_without_writing() {
        let hr = unsafe { shiori_factory(core::ptr::null_mut()) };
        assert_eq!(
            hr, E_POINTER,
            "NULL out に対し shiori_factory は E_POINTER を返すこと, got 0x{:08X}",
            hr.0
        );
    }

    /// 収載 GET id（`OnFirstBoot`）→ `S_OK`＋凍結スナップショット全文を即時 move-out すること
    /// （ID 分岐 replay が実 vtable を横断して働く証拠）。
    #[test]
    fn get_listed_id_returns_frozen_snapshot_immediately() {
        let (brain, _host) = make_brain();
        let (hr, out_response, _token) = drive_get(&brain, &get_request("OnFirstBoot"));
        assert_eq!(hr, S_OK, "収載 GET は即時 S_OK, got 0x{:08X}", hr.0);
        let expected =
            crate::snapshot::snapshot_for("OnFirstBoot").expect("OnFirstBoot は収載されていること");
        assert_eq!(
            out_response,
            HSTRING::from(expected),
            "out_response は凍結スナップショット全文（逐語）であること"
        );
    }

    /// 未知 GET id → `S_OK`＋204 応答（silent success でなく明示 No Content）。
    #[test]
    fn get_unknown_id_returns_204_immediately() {
        let (brain, _host) = make_brain();
        let (hr, out_response, _token) = drive_get(&brain, &get_request("OnNeverSeenEvent"));
        assert_eq!(hr, S_OK, "未知 GET は即時 S_OK, got 0x{:08X}", hr.0);
        assert!(
            out_response
                .to_string()
                .starts_with("SHIORI/3.0 204 No Content"),
            "未知 ID は 204 応答であること: {out_response}"
        );
    }

    /// malformed 入力 → `S_OK`＋400 応答（COM error でも panic でもない）。
    #[test]
    fn get_malformed_input_returns_400_immediately_without_com_error_or_panic() {
        let (brain, _host) = make_brain();
        let (hr, out_response, _token) = drive_get(&brain, &HSTRING::from("BOGUS GARBAGE LINE"));
        assert_eq!(
            hr, S_OK,
            "malformed でも COM error でなく即時 S_OK（400 が fail-visible 面）, got 0x{:08X}",
            hr.0
        );
        assert!(
            out_response
                .to_string()
                .starts_with("SHIORI/3.0 400 Bad Request"),
            "malformed 入力は 400 応答であること: {out_response}"
        );
    }

    /// `Get` は収載/未知/malformed の全経路で即時 `S_OK` を返し、`SHIORI_S_PENDING` を返さないこと。
    #[test]
    fn get_never_returns_pending_on_any_path() {
        let (brain, _host) = make_brain();
        for input in [
            get_request("OnFirstBoot"),
            get_request("OnNeverSeenEvent"),
            HSTRING::from("BOGUS GARBAGE LINE"),
            HSTRING::from(""),
            get_request(""),
        ] {
            let (hr, _resp, _token) = drive_get(&brain, &input);
            assert_eq!(
                hr, S_OK,
                "全経路で即時 S_OK, got 0x{:08X} for {input}",
                hr.0
            );
            assert_ne!(
                hr, SHIORI_S_PENDING,
                "Get は遅延（PENDING）を返さないこと: {input}"
            );
        }
    }

    /// `Notify`（片道通知）は内容不問で決定論的に `Ok(())` を返すこと。
    #[test]
    fn notify_is_receipt_only_and_returns_ok() {
        let (brain, _host) = make_brain();
        unsafe {
            brain.Notify(&HSTRING::from(
                "NOTIFY SHIORI/3.0\r\nID: OnInitialize\r\n\r\n",
            ))
        }
        .expect("Notify は Ok（片道・応答なし）");
        unsafe { brain.Notify(&HSTRING::from("arbitrary opaque garbage")) }
            .expect("Notify は内容不問で Ok");
    }

    /// host `Ref` 欠落の `CreateInstance` は `Err` かつ out 未書込（半構築非露出）。
    #[test]
    fn create_without_host_fails_without_exposing_half_construct() {
        let factory = make_factory();
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
        assert!(
            brain_out.is_none(),
            "失敗時は out 未書込であること（半構築非露出）"
        );
    }
}
