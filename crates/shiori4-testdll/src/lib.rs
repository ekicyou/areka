//! x64 SHIORI4 決定論テスト DLL（`shiori4-testdll`）の crate ルート。
//!
//! 本 crate は areka の「実 SHIORI 境界を踏む決定論テストゴースト」の**脳**を担う cdylib であり、
//! 正典イベント集合へ実 emo2 pasta 採取のゴールデンスナップショットを決定論 replay する
//! （design.md "ReplayBrain・ReplayFactory・shiori_factory export"）。本ファイルは task 1.1 の
//! **crate 雛形（scaffold）**であり、以下を確定する:
//!
//! - 契約定数 [`DLL_FILE_NAME`]（出力 DLL ファイル名の単一権威・fixture descript の `shiori,` 行と
//!   e2e の locate/コピーが共有する・design.md §SnapshotTable Service Interface）。
//! - 生成入口 [`shiori_factory`]（production SHIORI4 と同形の `extern "system"` エクスポート ABI 形状・
//!   要件 2.1）＋生成面 [`ReplayFactory`]（`#[implement(IShioriFactory)]`）＋台本脳 [`ReplayBrain`]
//!   （`#[implement(IShiori)]`）。ID 分岐 replay で正典イベントへ凍結スナップショットを決定論応答する
//!   （task 1.4・design.md "ReplayBrain・ReplayFactory・shiori_factory export"）。
//! - `cargo test --workspace` 実行後、ビルド成果物 cdylib が現れる**単一の正準位置**を実測固定する
//!   in-crate spike テスト（要件 1.2・5.4／design.md D-1「単一正準位置・フォールバックなし」）。
//!   **実測結果（task 1.1 spike）**: 正準位置は `target/<profile>/deps/shiori4_testdll.dll`
//!   （= `current_exe()` と同一ディレクトリ）。詳細な実測根拠は spike テスト本体の doc を参照。
//!
//! ## スレッド座・COM（design.md D-6）
//! 全 COM 参照は shiori アクタースレッド常駐で、`CoInitializeEx` は**呼ばない**（直接 vtable dispatch
//! のみ・アクティベーション/マーシャリング非使用）。in-crate 檻は COM 未初期化スレッドで走るが、
//! `windows-core` の `#[implement]` オブジェクトは直接 vtable dispatch ゆえ問題なく通る（reference_brain
//! の in-crate 檻と同律）。
//!
//! ## 依存境界（design.md "Allowed Dependencies"）
//! 本 crate は [`shiori-abi`](shiori_abi) ＋ [`windows-core`](windows_core) のみに依存する。
//! `windows`(Win32)・`tracing`・areka 系 crate へは依存しない（自給・D-2）。[`ReplayBrain`]/
//! [`ReplayFactory`] は `shiori_abi::interface` の COM 面（`IShiori`/`IShioriFactory`/`IShioriHost`）を
//! 実消費する。

// `#[implement(...)]` の生成面が PascalCase メソッドを要求する（reference_brain と同律）。
#![allow(non_snake_case)]

use core::ffi::c_void;

use shiori_abi::interface::{
    IShiori, IShiori_Impl, IShioriFactory, IShioriFactory_Impl, IShioriHost,
};
use windows_core::{HRESULT, HSTRING, Interface, OutRef, Ref, Result, implement};

/// SHIORI/3.0 リクエスト解析・応答分類の純粋ロジック（task 1.2）。
///
/// `parse_request`（request line の別＋`ID:` 抽出・design.md D-4）と `select_response`
/// （収載→凍結応答／未知→204／構造不整合→400・要件 2.2/2.3/2.4）を提供する。項目は
/// crate 内消費（`pub(crate)`）で、[`ReplayBrain::Get`](ReplayBrain) が呼ぶ（task 1.4 で結線済み）。
mod request;

/// 正典 GET イベントごとの凍結ゴールデンスナップショットの静的表（task 1.3）。
///
/// `snapshots/<EventID>.txt`（SHIORI/3.0 応答全文）を `include_str!` でコンパイル時に埋め込み
/// （実行時 I/O ゼロ・要件 1.5）、取り込み時に CRLF 正規化する（git EOL 変換への免疫・要件 2.5／
/// research.md §7.3）。`snapshot_for(id)` は [`ReplayBrain::Get`](ReplayBrain) が `select_response` の
/// `lookup` クロージャへ無改修で差し込む（task 1.4 で結線済み）。`snapshot_for`／`snapshots`／
/// `PROVISIONAL_MARKER` は `pub`（areka-ghost rlib 面が契約定数・スナップショット表を共有する）。
///
/// モジュール自体は非公開に保ちつつ、crate ルートへ flat 再エクスポート（`shiori4_testdll::snapshot_for`）
/// して crate 外（areka-ghost tests 5.1 の**ドリフト検出 assert**）から到達可能にする（task 5.1・
/// tasks.md「[2.3→5.1 申し送り]」・要件 5.2）。
mod snapshot;
pub use snapshot::{PROVISIONAL_MARKER, snapshot_for, snapshots};

/// 出力 DLL ファイル名（design.md §SnapshotTable Service Interface の契約定数・単一権威）。
///
/// この値は次の 2 箇所が共有する契約値である:
/// - テストゴースト fixture の `ghost/master/descript.txt` の `shiori,` 行（D-8）。
/// - e2e の `locate_built_test_dll()`／fixture への DLL コピー（要件 1.2・5.4）。
///
/// D-8 で確定した命名（crate `shiori4-testdll`・`[lib] name = "shiori4_testdll"`）の帰結として、
/// x64 ネイティブビルドの cdylib 出力ファイル名は `shiori4_testdll.dll` になる。
pub const DLL_FILE_NAME: &str = "shiori4_testdll.dll";

/// `S_OK`（成功・即時応答）の HRESULT。
///
/// [`ReplayBrain::Get`](ReplayBrain) の即時応答と [`shiori_factory`] の成功に用いる（design.md
/// §InterfaceLayer「即時応答: `S_OK`」・要件 2.4 の「COM error を返さない」）。
const S_OK: HRESULT = HRESULT(0);

/// `E_POINTER`（NULL/無効ポインタ）の HRESULT。
///
/// Win32 の `E_POINTER = 0x8000_4003`。本 crate は `windows`(Win32) へ依存しない
/// （design.md "Allowed Dependencies"）ため、[`windows_core::HRESULT`] のローカル定数として保持する。
const E_POINTER: HRESULT = HRESULT(0x8000_4003u32 as i32);

/// 実 IShiori 境界を再現的に横断させる決定論 replay 脳（`#[implement(IShiori)]`・design.md
/// "ReplayBrain・ReplayFactory・shiori_factory export"）。
///
/// 保持状態は**不変データのみ**——`CreateInstance` で AddRef（clone）保持した host（parity 目的で
/// 保持するが**呼び返さない**・Raise/Complete/Property 不使用・要件 7.4）と、観測用の
/// `load_dir`／`shiori_name` clone（reference_brain と同律）。乱数・実時計・環境変数への依存を
/// 持たない（決定論・要件 1.5。i686 testdll の env フック類も持ち込まない）。
#[implement(IShiori)]
pub struct ReplayBrain {
    /// `CreateInstance` で AddRef（clone）保持した host（不変・construction 時確定）。
    ///
    /// parity のため保持するが、本脳は**呼び返さない**（Raise/Complete/Property 不使用・要件 7.4）。
    _held_host: IShioriHost,
    /// construction 時に束縛した load_dir（不変・観測用 clone・reference_brain と同律）。
    _load_dir: HSTRING,
    /// construction 時に束縛した shiori_name（不変・観測用 clone）。
    _shiori_name: HSTRING,
}

impl ReplayBrain {
    /// host・load_dir・shiori_name を束縛して脳を生成する（construction 時確定・不変）。
    ///
    /// [`ReplayFactory::CreateInstance`] がこの経路で脳を構築し、`IShiori` へ move-out する。
    fn new(host: IShioriHost, load_dir: HSTRING, shiori_name: HSTRING) -> Self {
        Self {
            _held_host: host,
            _load_dir: load_dir,
            _shiori_name: shiori_name,
        }
    }
}

impl IShiori_Impl for ReplayBrain_Impl {
    /// 同期 request（GET SHIORI/3.0 後継・design.md "ReplayBrain" `Get` 責務）。**常に即時**応答で
    /// `SHIORI_S_PENDING` を返さない（要件 7.4）。
    ///
    /// HSTRING → String（[`HSTRING::to_string`]）→ [`request::parse_request`]（request line＋`ID:` 抽出）
    /// → [`request::select_response`]（[`snapshot::snapshot_for`] を `lookup` に注入し 収載→凍結応答／
    /// 未知・未収載→204／malformed（GET request line 不在・ID 欠落等）→400）→
    /// [`request::response_text`] で最終応答テキストを解決し `out_response` へ move-out して [`S_OK`]。
    ///
    /// 分岐は総和的な純関数のみで構成され **panic しない**。COM error HRESULT は返さない
    /// （プロトコル水準の 400 が fail-visible 面・要件 2.4）。`out_token` は未使用（即時ゆえ）。
    unsafe fn Get(
        &self,
        input: &HSTRING,
        out_response: &mut HSTRING,
        _out_token: &mut u64,
    ) -> HRESULT {
        // 不透明 content を Rust String へ（ID 行のみ読む・応答は凍結全文を逐語返却・要件 2.5）。
        let request = input.to_string();
        let parsed = request::parse_request(&request);
        // 収載→凍結応答（要件 2.2）／未知→204（要件 2.3）／malformed→400（要件 2.4）。
        // task 1.3 の snapshot_for を lookup クロージャへ無改修で差し込む（脱結合）。
        let selection = request::select_response(&parsed, snapshot::snapshot_for);
        let response = request::response_text(&selection);
        // callee 確保の HSTRING を out_response へ move-out（所有権規約・caller 解放）。常に即時 S_OK。
        *out_response = HSTRING::from(response);
        S_OK
    }

    /// 片道通知（NOTIFY SHIORI/3.0 後継・応答なし・要件 7.4）。受領のみで内容不問・決定論的に `Ok(())`。
    ///
    /// content は不透明のまま解析せず、host へも呼び返さない（受領のみ）。
    unsafe fn Notify(&self, _input: &HSTRING) -> Result<()> {
        // 受領のみ（片道・内容不問・決定論）。
        Ok(())
    }
}

/// 生成＋load 融合の生成面（`#[implement(IShioriFactory)]`・reference_brain のパターン踏襲）。
///
/// [`CreateInstance`](IShioriFactory_Impl::CreateInstance) は [`ReplayBrain`] を構築し、load 完了済みの
/// `IShiori` を `out` へ move-out する。host `Ref` 欠落時は `out` 未書込で失敗する（半構築非露出）。
#[implement(IShioriFactory)]
pub struct ReplayFactory;

impl ReplayFactory {
    /// factory を生成する。
    fn new() -> Self {
        Self
    }
}

impl IShioriFactory_Impl for ReplayFactory_Impl {
    /// 脳を生成し、load 完了済みの `IShiori` を `out` へ move-out する（design.md "ReplayFactory"）。
    ///
    /// - host `Ref` 欠落（`host.as_ref()` が `None`）→ [`E_POINTER`] を返し `out` を書かない
    ///   （半構築非露出・要件 2.1）。reference_brain の host 非在失敗と同律。
    /// - 成功時: `host` を clone（AddRef）して [`ReplayBrain`] へ保持させ（parity・呼び返さない・要件 7.4）、
    ///   `load_dir`/`shiori_name` を clone して束縛し、refcount 1 の `IShiori` を `out` へ move-out する。
    unsafe fn CreateInstance(
        &self,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
        host: Ref<'_, IShioriHost>,
        out: OutRef<'_, IShiori>,
    ) -> Result<()> {
        // host は Ref 借用。欠落は失敗（out 未書込・半構築非露出・要件 2.1）。保持は clone（AddRef）。
        let host: IShioriHost = host
            .as_ref()
            .ok_or_else(|| windows_core::Error::from(E_POINTER))?
            .clone();
        // load_dir/shiori_name を束縛（観測用 clone・reference_brain と同律）。refcount 1 の脳を構築。
        let brain: IShiori =
            ReplayBrain::new(host, load_dir.clone(), shiori_name.clone()).into();
        // load 完了済み brain を out へ move-out（callee 確保・caller Release）。
        out.write(Some(brain))?;
        Ok(())
    }
}

/// `IShioriFactory` 実体を生成する純粋C コンストラクタ（production SHIORI4 と同形の生成入口・要件 2.1）。
///
/// production SHIORI4 生成入口（`crates/areka/src/reference_brain.rs::shiori_factory`）と同形の
/// エクスポート ABI 形状（`#[unsafe(no_mangle)]`＋`extern "system"`＝Windows COM 標準の呼出規約・
/// x64/ARM64 では `extern "C"` と同一 ABI だが COM 整合で正準表記は `system`）。host-32／InProc 経路が
/// `GetProcAddress("shiori_factory")` で引ける形を満たす。
///
/// - `out` が NULL → [`E_POINTER`] を返し、何も書き込まない（半構築非露出）。
/// - 成功 → `*out` へ refcount 1 の [`IShioriFactory`]（[`ReplayFactory`] 由来）を move-out し [`S_OK`]。
///
/// # Safety
/// `out` は非 NULL の有効な書込先ポインタであること（呼び出し側が保証）。`out` が NULL の場合は
/// 防御的に [`E_POINTER`] を返し、何も書き込まない。成功時 `out` が受け取る `IShioriFactory` は
/// 参照カウント 1 で、呼び出し側が `Release`（drop）する義務を負う。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn shiori_factory(out: *mut *mut c_void) -> HRESULT {
    // 前提の防御: out が NULL なら何も書き込まず判別可能な失敗を返す（reference_brain と同律）。
    if out.is_null() {
        return E_POINTER;
    }
    // refcount 1 の IShioriFactory を構築し、所有権を out へ move-out する。
    let factory: IShioriFactory = ReplayFactory::new().into();
    // Safety: `out` は非 NULL の有効な書込先（呼び出し側保証・上で NULL を排除済み）。
    // into_raw は参照を変えずに所有権を移譲する（caller が単一参照を所有・Release 義務）。
    unsafe { *out = factory.into_raw() };
    S_OK
}

#[cfg(test)]
mod tests {
    //! crate 雛形の in-crate 檻。
    //!
    //! 中核は **uplift spike**（`built_cdylib_appears_at_single_canonical_deps_dir`）で、
    //! `cargo test` がビルドする cdylib が現れる単一の正準位置を実測固定する
    //! （要件 1.2・5.4／design.md D-1）。併せて task 1.4 の `ReplayFactory`→`CreateInstance`→
    //! `Get`/`Notify` の vtable dispatch を実 COM 面で駆動し、即時 `S_OK`・凍結スナップショット
    //! move-out・未知 204・malformed 400・host 欠落 `E_POINTER`・PENDING 非返却を檻へ入れる
    //! （design.md Testing Strategy Unit Test #3・要件 2.1〜2.5/7.4/1.5・reference_brain の流儀）。

    use super::*;
    use std::path::PathBuf;

    /// **実測固定（uplift spike・task 1.1）**: `cargo test`（`--workspace` 含む）のビルド成果物 cdylib が
    /// 現れる**単一の正準位置**は `target/<profile>/deps/shiori4_testdll.dll`
    /// （= `current_exe()` と同一ディレクトリ）である（要件 1.2・5.4／design.md D-1）。
    ///
    /// **実測根拠（本 spike が確定させた事実）**:
    /// cargo は cdylib を必ず `target/<profile>/deps/` にリンク出力する。`target/<profile>/`（deps の親）
    /// への「uplift（トップレベル複製）」は `cargo build`（primary ビルド）でのみ起き、
    /// `cargo test` / `cargo test --workspace` では**起きない**（`--no-run` 実測で確認）。したがって
    /// 常設ゲート `cargo test --workspace` が確実に生成する単一の正準位置は deps ディレクトリ自身であり、
    /// design.md D-1 の散文が推定した `target/<profile>/`（deps を pop）ではない——D-1 が命じた
    /// 「実装先頭タスクでの uplift 実証 spike で確定した単一の正準位置のみを locate する」に従い、
    /// spike の実測が prose の推定を上書きする（正準位置＝deps・単一・フォールバックなし）。
    ///
    /// 導出は `current_exe()` から決定論的に行う: テストバイナリは
    /// `target/<profile>/deps/<name>-<hash>.exe` に置かれるため、その**親ディレクトリ（deps）**へ
    /// [`DLL_FILE_NAME`] を join した位置が正準 cdylib である（cdylib 出力はハッシュ接尾なしの素名
    /// `shiori4_testdll.dll` 固定）。areka-ghost tests の e2e もこの同一 deps を共有するため、
    /// `locate_built_test_dll()`（task 3 系）はこの導出を逐語再利用できる。
    ///
    /// glob／mtime／`target/<profile>/` へのフォールバックは採らない（design.md D-1・設計討議#1
    /// 「単一正準位置・フォールバックなし」——将来 cargo 挙動変化時に古い DLL を拾って壊れたビルドを
    /// 隠蔽する silent green を防ぐため。挙動変化は下記 assert の明示 panic で即座に顕在化させる）。
    #[test]
    fn built_cdylib_appears_at_single_canonical_deps_dir() {
        // current_exe = target/<profile>/deps/<name>-<hash>.exe
        let test_exe = std::env::current_exe().expect("test executable path is available");

        // 親ディレクトリ = target/<profile>/deps（cdylib もここへ出力される・実測固定の正準ディレクトリ）。
        let deps_dir: PathBuf = test_exe
            .parent()
            .expect("test executable resides in a deps directory")
            .to_path_buf();
        assert_eq!(
            deps_dir.file_name().and_then(|s| s.to_str()),
            Some("deps"),
            "cargo のテストバイナリ配置（target/<profile>/deps/...）を前提とする。\
             layout が変化した場合は本 spike と locate 導出を再実証すること（design.md D-1）。\
             observed test_exe = {}",
            test_exe.display()
        );

        // 正準位置 = target/<profile>/deps/shiori4_testdll.dll（契約定数を rlib 面から参照）。
        let canonical_dll = deps_dir.join(DLL_FILE_NAME);

        assert!(
            canonical_dll.exists(),
            "ビルド成果物 cdylib が正準位置に不在: {}\n\
             この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置へ出力する。\
             単独実行時は先に `cargo build -p shiori4-testdll` を実行すること（フォールバックは設けない・\
             design.md D-1）。",
            canonical_dll.display()
        );
    }

    /// 契約定数 [`DLL_FILE_NAME`] が D-8 の命名（`shiori4_testdll.dll`）に固定されていること（回帰防止）。
    #[test]
    fn dll_file_name_is_fixed_to_d8_naming() {
        assert_eq!(
            DLL_FILE_NAME, "shiori4_testdll.dll",
            "出力 DLL ファイル名は D-8 命名に固定されていること"
        );
    }

    // ---- task 1.4: ReplayFactory→CreateInstance→Get/Notify の vtable dispatch 檻（要件 2.1〜2.5/7.4/1.5・
    //      design.md Testing Strategy Unit Test #3・reference_brain の流儀） ---------------------------

    use core::ffi::c_void;
    use shiori_abi::error::SHIORI_S_PENDING;
    use shiori_abi::interface::{IShiori, IShioriFactory, IShioriHost, IShioriHost_Impl};
    use windows_core::{HSTRING, Interface, Ref, Result, implement};

    /// vtable dispatch 検証用の最小モック host。ReplayBrain は host を保持するが**呼び返さない**
    /// （Raise/Complete/Property 不使用・要件 7.4）ため、4 メソッドは自明な受け皿でよい。
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
        assert!(hr.is_ok(), "shiori_factory は成功時 S_OK, got 0x{:08X}", hr.0);
        assert!(!out.is_null(), "成功時は out へ非 NULL の IShioriFactory を書き出すこと");
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
    /// こと（要件 2.1・writes-on-success・reference_brain と同律）。
    #[test]
    fn shiori_factory_outputs_refcount_one_factory_on_success() {
        let mut out: *mut c_void = core::ptr::null_mut();
        let hr = unsafe { shiori_factory(&mut out) };
        assert!(hr.is_ok(), "shiori_factory は成功時 S_OK, got 0x{:08X}", hr.0);
        assert!(!out.is_null(), "成功時は out へ非 NULL を書き出すこと");

        let factory = unsafe { IShioriFactory::from_raw(out) };

        // refcount 1 検証: IUnknown へ cast（+1）してから AddRef/Release を一往復。
        let unk: windows_core::IUnknown = factory.cast().expect("IUnknown へ cast");
        let after_add = unsafe { (Interface::vtable(&unk).AddRef)(unk.as_raw()) };
        let after_rel = unsafe { (Interface::vtable(&unk).Release)(unk.as_raw()) };
        assert_eq!(after_add, 3, "AddRef は cast 後ベースライン 2 から 3, got {after_add}");
        assert_eq!(after_rel, 2, "Release は 3 から 2, got {after_rel}");
    }

    /// `shiori_factory(NULL)` は判別可能な失敗（`E_POINTER`）を返し out を書き込まないこと（要件 2.1・
    /// 半構築非露出）。
    #[test]
    fn shiori_factory_with_null_out_returns_e_pointer_without_writing() {
        let hr = unsafe { shiori_factory(core::ptr::null_mut()) };
        assert_eq!(
            hr, E_POINTER,
            "NULL out に対し shiori_factory は E_POINTER を返すこと, got 0x{:08X}",
            hr.0
        );
    }

    /// 収載 GET id（`OnBoot`）→ `S_OK`＋凍結スナップショット全文を即時 move-out すること
    /// （ID 分岐 replay が実 vtable を横断して働く証拠・要件 2.2/1.5）。
    #[test]
    fn get_listed_id_returns_frozen_snapshot_immediately() {
        let (brain, _host) = make_brain();
        let (hr, out_response, _token) = drive_get(&brain, &get_request("OnBoot"));
        assert_eq!(hr, S_OK, "収載 GET は即時 S_OK, got 0x{:08X}", hr.0);
        let expected = crate::snapshot::snapshot_for("OnBoot").expect("OnBoot は収載されていること");
        assert_eq!(
            out_response,
            HSTRING::from(expected),
            "out_response は凍結スナップショット全文（逐語）であること"
        );
    }

    /// 未知 GET id → `S_OK`＋204 応答（silent success でなく明示 No Content・要件 2.3）。
    #[test]
    fn get_unknown_id_returns_204_immediately() {
        let (brain, _host) = make_brain();
        let (hr, out_response, _token) = drive_get(&brain, &get_request("OnNeverSeenEvent"));
        assert_eq!(hr, S_OK, "未知 GET は即時 S_OK, got 0x{:08X}", hr.0);
        assert!(
            out_response.to_string().starts_with("SHIORI/3.0 204 No Content"),
            "未知 ID は 204 応答であること: {out_response}"
        );
    }

    /// malformed 入力 → `S_OK`＋400 応答（COM error でも panic でもない・要件 2.4）。
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
            out_response.to_string().starts_with("SHIORI/3.0 400 Bad Request"),
            "malformed 入力は 400 応答であること: {out_response}"
        );
    }

    /// `Get` は決して `SHIORI_S_PENDING` を返さない（常に即時・要件 7.4）——収載/未知/malformed 全経路で
    /// 成功即時 `S_OK` であること。
    #[test]
    fn get_never_returns_pending_on_any_path() {
        let (brain, _host) = make_brain();
        for input in [
            get_request("OnBoot"),
            get_request("OnNeverSeenEvent"),
            HSTRING::from("BOGUS GARBAGE LINE"),
            HSTRING::from(""),
            get_request(""),
        ] {
            let (hr, _resp, _token) = drive_get(&brain, &input);
            assert_eq!(hr, S_OK, "全経路で即時 S_OK, got 0x{:08X} for {input}", hr.0);
            assert_ne!(hr, SHIORI_S_PENDING, "Get は遅延（PENDING）を返さないこと: {input}");
        }
    }

    /// `Notify`（片道通知）は内容不問で決定論的に `Ok(())` を返すこと（受領のみ・要件 7.4）。
    #[test]
    fn notify_is_receipt_only_and_returns_ok() {
        let (brain, _host) = make_brain();
        unsafe { brain.Notify(&HSTRING::from("NOTIFY SHIORI/3.0\r\nID: OnInitialize\r\n\r\n")) }
            .expect("Notify は Ok（片道・応答なし）");
        // 内容不問: garbage でも Ok。
        unsafe { brain.Notify(&HSTRING::from("arbitrary opaque garbage")) }
            .expect("Notify は内容不問で Ok");
    }

    /// host `Ref` 欠落の `CreateInstance` は `Err` かつ out 未書込（半構築非露出・要件 2.1）。
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
        assert!(brain_out.is_none(), "失敗時は out 未書込であること（半構築非露出）");
    }
}
