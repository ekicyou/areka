//! **InProcLibrary**: x64 SHIORI4 DLL のロードと生成入口（`shiori_factory`）解決を担う RAII 機構。
//!
//! helper 側の flat-C プロキシ（`crates/shiori-host32-helper/src/shiori_proxy.rs`）の x64/COM 版で、
//! `shiori_factory`（`extern "system" fn(*mut *mut c_void) -> HRESULT`）1 本を解決し、そこから
//! [`IShioriFactory`](shiori_abi::interface::IShioriFactory) を [`Interface::from_raw`] で興す。
//!
//! ## 確立シーケンス
//! 1. `LoadLibraryExW(dll_path, .., LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)`
//!    （絶対パス必須・依存は DLL のフォルダとシステムの既定から探し、作業フォルダと PATH は見ない）。
//!    失敗（DLL 欠落・不正イメージ・相対パス）→ `error!` 済み `Err(String)`。
//! 2. `GetProcAddress("shiori_factory")`。未解決（シンボル欠落）→ `error!` 済み `Err(String)`。
//!    この時点で HMODULE はロード済みなので、失敗経路でも確実に解放する。手順 1 成功直後に
//!    [`InProcLibrary`] を構築し、以後の失敗はスコープを抜けて Drop（`FreeLibrary`）を走らせる。
//! 3. 解決した proc を `unsafe extern "system" fn(*mut *mut c_void) -> HRESULT` へ transmute し呼出。
//!    失敗 HRESULT／成功だが null out → `error!` 済み `Err(String)`。
//! 4. 成功時のみ `IShioriFactory::from_raw(out)` を興し `(InProcLibrary, IShioriFactory)` を返す。
//!
//! ## FreeLibrary 順序不変条件
//! **DLL が実装する COM オブジェクトの全参照が Release された後にのみ `FreeLibrary` してよい**
//! （違反は UB）。[`InProcLibrary`] 自身は HMODULE のみを保持し、順序は所有者 `InProcBackend` の
//! フィールド宣言順（`shiori`→`host`→`library` の drop 順）で担保する。したがって [`InProcLibrary`] は
//! 生成した [`IShioriFactory`]（および後続の `IShiori`）より後に drop されねばならない。
//!
//! ## スレッド座・COM
//! 全 COM 参照と HMODULE は shiori アクタースレッド常駐（`!Send`）。`CoInitializeEx` は呼ばない
//! （直接 vtable dispatch のみ・アクティベーション/マーシャリング非使用）。
//!
//! ## 失敗経路のログ
//! すべての失敗経路は `Err` を返す前に `error!`（`target: "ghost-shiori-inproc"`）でログし、
//! panic は用いない。

// `#[implement(IShioriHost)]`（InProcHost）が生成する実装トレイト面は COM 規約の PascalCase
// メソッド名（`Raise`/`Complete`/`GetProperty`/`SetProperty`）を要求する。
#![allow(non_snake_case)]

use core::ffi::c_void;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::Path;

use areka_kanade::ShioriBackend;
use areka_parsers::package::ShioriMount;
use shiori_abi::error::{SHIORI_E_PROPERTY_NOT_FOUND, SHIORI_E_UNKNOWN_TOKEN, SHIORI_S_PENDING};
use shiori_abi::interface::{IShiori, IShioriFactory, IShioriHost, IShioriHost_Impl};
use shiori_host32_host::{
    Charset, CharsetPolicy, ExitKind, HelperStatus, Method, RequestError, RequestHeaders,
    ShioriError, ShutdownError, ValueReply, build_request, map_get_reply, parse_response,
    request_of,
};
use tracing::{error, warn};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::Diagnostics::Debug::{
    SEM_FAILCRITICALERRORS, SEM_NOOPENFILEERRORBOX, SetThreadErrorMode, THREAD_ERROR_MODE,
};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LoadLibraryExW,
};
// `Result` は glob 導入しない: `load` は `std::result::Result<_, String>` を返すので、
// COM の `Result<()>` は別名（`ComResult`）で受ける。
use windows::core::{HRESULT, HSTRING, Interface, Result as ComResult, implement, s};

/// tracing target（本モジュールの全ログ発火で共有）。
const LOG_TARGET: &str = "ghost-shiori-inproc";

/// `shiori_factory` 生成入口の署名。
///
/// `extern "system"`（COM 標準の呼出規約）。`out` へ refcount 1 の `IShioriFactory` を move-out し `HRESULT` を返す。
type ShioriFactoryFn = unsafe extern "system" fn(*mut *mut c_void) -> HRESULT;

/// DLL を、OS の窓（「正しくないイメージ」0xc000012f など）を出さずに読む。
///
/// `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` で、依存の DLL は読む DLL の
/// フォルダとシステムの既定から探し、作業フォルダと PATH は見ない（DLL の植え込みを塞ぐ）。
/// このフラグは絶対パスを要り、相対パスは OS が断る。
///
/// 既定では、壊れた DLL を読むと OS が利用者の画面に窓を出し、OK が押されるまでこのスレッドを止める。
/// 失敗は `Err` として返り、呼出側が `error!` で記録するので、窓は要らない。呼ぶ間だけこのスレッドの
/// エラーモードに `SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX` を立て、終えたら元へ戻す
/// （プロセス全体の `SetErrorMode` は触らない）。
fn load_library_quiet(wide: &HSTRING) -> windows::core::Result<HMODULE> {
    let mut old = THREAD_ERROR_MODE(0);
    // SAFETY: 引数は定数の正しいフラグと、呼出中生きている `old` へのポインタだけ。
    // 定数の正しいフラグでは失敗しない。万一失敗したら `old` は読めていないので、戻す手順も飛ばす。
    let set = unsafe {
        SetThreadErrorMode(
            SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX,
            Some(&mut old),
        )
    }
    .is_ok();
    // SAFETY: `wide` は呼出中生存する有効な NUL 終端 UTF-16。失敗は Err で返る。
    let result = unsafe {
        LoadLibraryExW(
            wide,
            None,
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        )
    };
    if set {
        // SAFETY: 直前に読み出した元のモードを戻すだけ。
        let _ = unsafe { SetThreadErrorMode(old, None) };
    }
    result
}

/// x64 SHIORI4 DLL のロード RAII。
///
/// ロード済み HMODULE を保持し、Drop で `FreeLibrary` する（best-effort・失敗は `error!` のみ）。
/// `PhantomData<*const ()>` で非 `Send` に固定する。
///
/// **FreeLibrary 順序不変条件**（モジュール doc 参照）は本型では強制しない。順序は所有者
/// （`InProcBackend`）のフィールド宣言順で担保する。
pub(crate) struct InProcLibrary {
    /// ロード元モジュールハンドル。Drop で `FreeLibrary` する（唯一の teardown 経路）。
    module: HMODULE,
    /// `!Send` マーカー（HMODULE が仮に `Send` でも本型を非 `Send` に固定する）。
    _not_send: PhantomData<*const ()>,
}

impl InProcLibrary {
    /// DLL をロードし `shiori_factory` を解決して [`IShioriFactory`] を生成する。
    ///
    /// 失敗（欠落 DLL・不正イメージ・シンボル未解決・factory 生成失敗）はいずれも `error!` ログ済みの
    /// `Err(String)` で返す。取得済み HMODULE はどの失敗経路でも Drop（`FreeLibrary`）で解放される。
    ///
    /// 成功時は `(InProcLibrary, IShioriFactory)` を返す。**呼び出し側は返した `IShioriFactory`
    /// （および派生 COM 参照）を [`InProcLibrary`] より先に drop すること**（モジュール doc 参照）。
    ///
    /// # Preconditions
    /// `dll_path` は絶対パス想定（呼び出し側が `load_dir.join(shiori_name)` で組む）。
    /// 呼び出しは shiori アクタースレッド上。`CoInitializeEx` は呼ばない。
    pub(crate) fn load(dll_path: &Path) -> Result<(Self, IShioriFactory), String> {
        let wide = HSTRING::from(dll_path.as_os_str());
        // 成功で得た HMODULE の解放責務は、この直後に構築する InProcLibrary（Drop）へ移る。
        let module = match load_library_quiet(&wide) {
            Ok(m) => m,
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    path = %dll_path.display(),
                    error = %e,
                    "InProc DLL のロードに失敗（DLL 欠落・不正イメージ・相対パス等）"
                );
                return Err(format!(
                    "LoadLibraryExW failed for {}: {e}",
                    dll_path.display()
                ));
            }
        };

        // 以後の失敗は `return` で Drop=FreeLibrary を走らせ、取得済み HMODULE を全失敗経路で解放する。
        let library = InProcLibrary {
            module,
            _not_send: PhantomData,
        };

        // SAFETY: `library.module` は直前にロードした有効ハンドル。`s!` は NUL 終端 C 文字列リテラル。
        let proc = match unsafe { GetProcAddress(library.module, s!("shiori_factory")) } {
            Some(p) => p,
            None => {
                error!(
                    target: LOG_TARGET,
                    path = %dll_path.display(),
                    symbol = "shiori_factory",
                    "InProc 生成入口 shiori_factory が未解決（シンボル欠落）"
                );
                return Err(format!(
                    "GetProcAddress(\"shiori_factory\") unresolved in {}",
                    dll_path.display()
                ));
            }
        };

        // SAFETY: 解決した FARPROC を固定した生成入口署名 ShioriFactoryFn へ transmute する。
        // DLL 側実体（shiori4-testdll／将来の native x64 SHIORI4）が同形の
        // `#[unsafe(no_mangle)] extern "system" fn(*mut *mut c_void) -> HRESULT` を公開する。
        let factory_fn: ShioriFactoryFn =
            unsafe { core::mem::transmute::<_, ShioriFactoryFn>(proc) };

        let mut out: *mut c_void = core::ptr::null_mut();
        // SAFETY: `factory_fn` は上で解決した有効な生成入口。`&mut out` は有効な書込先ポインタ。
        // 呼出は同期で HRESULT を返す（直接 vtable dispatch・CoInitializeEx 不要）。
        let hr = unsafe { factory_fn(&mut out) };
        if hr.is_err() {
            error!(
                target: LOG_TARGET,
                path = %dll_path.display(),
                hr = format_args!("0x{:08X}", hr.0 as u32),
                "InProc factory 生成が失敗 HRESULT を返した"
            );
            return Err(format!(
                "shiori_factory returned failure HRESULT 0x{:08X} for {}",
                hr.0 as u32,
                dll_path.display()
            ));
        }
        if out.is_null() {
            error!(
                target: LOG_TARGET,
                path = %dll_path.display(),
                "InProc factory が成功 HRESULT だが null を返した（half-construct）"
            );
            return Err(format!(
                "shiori_factory returned success but null factory for {}",
                dll_path.display()
            ));
        }

        // SAFETY: `out` は生成入口が成功時に move-out した refcount 1 の IShioriFactory 生ポインタ
        // （上で null と失敗 HRESULT を排除済み）。from_raw は AddRef せず単一参照を adopt する
        // ＝呼び出し側が Release（drop）義務を負う。所有権は返り値へ移る。
        let factory = unsafe { IShioriFactory::from_raw(out) };
        Ok((library, factory))
    }
}

impl Drop for InProcLibrary {
    /// `FreeLibrary` する（best-effort・失敗は `error!` のみで伝播しない）。
    ///
    /// **FreeLibrary 順序不変条件**（モジュール doc）により、本 Drop は DLL 実装 COM オブジェクトの
    /// 全参照が Release された後に走らねばならない。順序は所有者のフィールド宣言順で担保する。
    fn drop(&mut self) {
        // SAFETY: `module` は `load` でロードした有効ハンドル。多重解放は所有（move）で防がれる。
        // FreeLibrary の失敗は error! のみ（best-effort）。
        if let Err(e) = unsafe { FreeLibrary(self.module) } {
            error!(
                target: LOG_TARGET,
                error = %e,
                "InProc DLL の FreeLibrary に失敗（best-effort・teardown 継続）"
            );
        }
    }
}

/// `CreateInstance` へ渡す areka-ghost 側の最小 `IShioriHost` 実装。
///
/// InProc 経路が実際に消費する能力だけを持つ:
/// - `Raise`: `warn!` で受領を可視化した上で `Ok(())`。InProc に Raise の消費者は無いので実配送しない。
/// - `Complete`: 常に `Err(SHIORI_E_UNKNOWN_TOKEN)`。deferred（`SHIORI_S_PENDING`）非対応で、
///   突合すべき pending 枠を持たない。
/// - `SetProperty` / `GetProperty`: 内部 `HashMap` を往復する。欠落 key は暗黙の空値で続行せず
///   `Err(SHIORI_E_PROPERTY_NOT_FOUND)`（out_value 未書込）にする。
///
/// `RefCell`＋COM 参照ゆえ `!Send`。shiori アクタースレッド常駐で用いる。
#[implement(IShioriHost)]
pub(crate) struct InProcHost {
    /// プロパティストア（`SetProperty` 格納・`GetProperty` 即答）。
    properties: RefCell<HashMap<String, HSTRING>>,
}

impl InProcHost {
    /// 空のプロパティストアを持つ最小 host を生成する。
    ///
    /// 生成物は `.into()` で `IShioriHost` 化して [`IShioriFactory::CreateInstance`] へ渡す。
    pub(crate) fn new() -> Self {
        Self {
            properties: RefCell::new(HashMap::new()),
        }
    }
}

impl IShioriHost_Impl for InProcHost_Impl {
    /// 能動通知（wakeup）。InProc に消費者はいないため実配送せず、`warn!` で受領を可視化して `Ok(())`。
    unsafe fn Raise(&self, script: &HSTRING) -> ComResult<()> {
        warn!(
            target: LOG_TARGET,
            script = %script,
            "InProcHost が Raise（自発通知）を受領したが M1 InProc に消費者はいない（実配送せず・要件 7.4）"
        );
        Ok(())
    }

    /// 遅延応答の完了配送。deferred 非対応で突合する pending 枠を持たないため、
    /// 任意トークンを未知として `Err(SHIORI_E_UNKNOWN_TOKEN)` で拒否する。
    unsafe fn Complete(&self, _token: u64, _response: &HSTRING) -> ComResult<()> {
        Err(SHIORI_E_UNKNOWN_TOKEN.into())
    }

    /// プロパティストアから同期即答する。欠落 key は暗黙の空値で続行せず
    /// `Err(SHIORI_E_PROPERTY_NOT_FOUND)`（out_value 未書込）。
    unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> ComResult<()> {
        match self.properties.borrow().get(&key.to_string()) {
            Some(v) => {
                *out_value = v.clone();
                Ok(())
            }
            None => Err(SHIORI_E_PROPERTY_NOT_FOUND.into()),
        }
    }

    /// プロパティストアへ値を即書きする（`[in]` 借用は保持のため clone）。
    unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> ComResult<()> {
        self.properties
            .borrow_mut()
            .insert(key.to_string(), value.clone());
        Ok(())
    }
}

/// 送出 `Sender` ヘッダ値。`Shiori3Client`/`real_connect` と同じく SSP を詐称せず自身を `"areka"` と名乗る。
const SENDER: &str = "areka";

/// `IShiori::Get` の観測（HRESULT＋応答バイト列）を既存の結果語彙 `Result<Option<String>, RequestError>`
/// へ写す純関数。
///
/// `response_bytes` は `out_response` HSTRING の UTF-8 表現で、`hr == S_OK` のときのみ意味を持つ。
/// status の分け方は host-32 の [`map_get_reply`] を呼ぶ（写しを持たない）:
///
/// | 観測 | 写像 |
/// |---|---|
/// | `S_OK`＋parse 成功・status 200 他 | `Ok(Some(value))`／value 欠落は `Ok(None)` |
/// | `S_OK`＋status 204 | `Ok(None)` |
/// | `S_OK`＋status 400/500 または `ErrorLevel` あり | `Err(RequestError::Shiori(ShioriError::Status{..}))` |
/// | `S_OK`＋parse 失敗 | `error!`＋`Err(RequestError::Shiori(ShioriError::Parse))` |
/// | [`SHIORI_S_PENDING`] | `error!`（InProc 非対応）＋`Err(..Parse)`（防衛線・到達しない） |
/// | その他の error HRESULT | `error!`（hr 値つき）＋`Err(..Parse)`（解釈不能を Parse へ集約） |
///
/// `RequestError`/`ShioriError` へ新 variant を足さず、解釈不能はすべて `ShioriError::Parse` へ集約する
/// （詳細は log が正本）。panic せず、COM を要さず全分岐を単体テストできる。
///
/// Value つきの応答は応答の `SecurityLevel` の生の値も [`ValueReply`] で返す（解釈は kanade）。
fn map_get_outcome(hr: HRESULT, response_bytes: &[u8]) -> Result<Option<ValueReply>, RequestError> {
    // 遅延応答（SHIORI_S_PENDING）は「成功」コードだが InProc は非対応。S_OK 判定より先に弾く。
    if hr == SHIORI_S_PENDING {
        error!(
            target: LOG_TARGET,
            "IShiori::Get が SHIORI_S_PENDING（遅延応答）を返した——M1 InProc は遅延応答を非対応（防衛線・要件 7.4）"
        );
        return Err(RequestError::Shiori(ShioriError::Parse));
    }
    // 失敗 HRESULT: 解釈不能ゆえ Parse へ集約（詳細は log が正本）。
    if hr.is_err() {
        error!(
            target: LOG_TARGET,
            hr = format_args!("0x{:08X}", hr.0 as u32),
            "IShiori::Get が失敗 HRESULT を返した（解釈不能→Parse へ集約・D-7）"
        );
        return Err(RequestError::Shiori(ShioriError::Parse));
    }
    // SHIORI/4 in-proc は UTF-8 固定で文字コード交渉を行わない（`CharsetPolicy::Force` ゆえ
    // 応答の `Charset` ヘッダは復号にも採用にも用いない）。
    let parsed = match parse_response(response_bytes, CharsetPolicy::Force(Charset::UTF_8)) {
        Ok(p) => p,
        Err(_) => {
            error!(
                target: LOG_TARGET,
                "IShiori::Get の応答が SHIORI/3.0 として解析不能（codec 契約違反→Parse）"
            );
            return Err(RequestError::Shiori(ShioriError::Parse));
        }
    };
    // 状態の分け方は 32bit の道と同じ 1 つの関数で読む。
    map_get_reply(parsed)
}

/// GET/NOTIFY 共通の request 組み立て（`build_request` → HSTRING）。
///
/// `build_request` は `charset` に渡された文字コードで符号化する。本呼出点は [`Charset::UTF_8`] 固定
/// （SHIORI/4 in-proc は文字コード交渉を持たない）なので、出力は常に有効な UTF-8 であり、下の
/// `from_utf8_lossy` は逐語一致し panic しない。要求は 32bit の経路と同じ共通の手 [`request_of`] で
/// 組む（同じヘッダを同じ順）。`status` は kanade が render 済みの wire 値で、解釈せず codec へ
/// verbatim 透過する。
fn build_input(
    method: Method,
    id: &str,
    references: &[String],
    status: Option<&str>,
    headers: RequestHeaders<'_>,
) -> HSTRING {
    let bytes = build_request(&request_of(
        method,
        id,
        references,
        status,
        headers,
        Charset::UTF_8,
    ))
    .bytes;
    HSTRING::from(String::from_utf8_lossy(&bytes).as_ref())
}

/// `IShiori`（load 完了済み脳）を kanade の [`ShioriBackend`] へ写す正規アダプタ。
///
/// **フィールド宣言順＝drop 順で FreeLibrary 順序不変条件を担保する**（`InProcLibrary` モジュール doc）:
/// `shiori`（DLL 実装 COM 参照）→ `host`（areka-ghost 実装・DLL 非依存）→ `library`（`FreeLibrary`）の
/// 順に drop され、DLL 実装 COM オブジェクトの全参照が Release された後にのみ `FreeLibrary` が走る。
/// カスタム `Drop` は設けない（`unload` 未実行で drop された場合も同順で teardown される）。
///
/// `!Send`（COM 参照と HMODULE がスレッド常駐）。`CoInitializeEx` は呼ばない。
pub(crate) struct InProcBackend {
    /// load 完了済み脳（`IShiori`）。**最初に** drop＝Release される。
    shiori: Option<IShiori>,
    host: Option<IShioriHost>,
    /// ロード済み DLL。**最後に** drop＝FreeLibrary される。
    library: Option<InProcLibrary>,
    /// `unload` 完了フラグ（`status()` の Running/Exited 判別）。
    unloaded: bool,
}

impl ShioriBackend for InProcBackend {
    /// ゴースト自身の既定のヘッダ（`Sender` は [`SENDER`]）で [`Self::get_with_headers`] を呼び、値だけを返す。
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.get_with_headers(id, references, status, &RequestHeaders::internal(SENDER))
            .map(|reply| reply.map(|r| r.value))
    }

    /// ゴースト自身の既定のヘッダ（`Sender` は [`SENDER`]）で [`Self::notify_with_headers`] を呼ぶ。
    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.notify_with_headers(id, references, status, &RequestHeaders::internal(SENDER))
    }

    /// 応答を要するイベント（GET）を組み立て `IShiori::Get` を呼び、観測を [`map_get_outcome`] で既存
    /// 語彙へ写す。応答のレベルは生のまま返す。
    fn get_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<Option<ValueReply>, RequestError> {
        let input = build_input(Method::Get, id, references, status, *headers);
        let Some(shiori) = self.shiori.as_ref() else {
            // unload 後の呼び出しは構造上起きないが、panic せず失敗として記録する。
            error!(target: LOG_TARGET, id, "unload 済み backend への GET（IShiori 不在・Parse へ集約）");
            return Err(RequestError::Shiori(ShioriError::Parse));
        };
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        // SAFETY: `shiori` は CreateInstance が move-out した有効な IShiori。out-param は有効な書込先。
        let hr = unsafe { shiori.Get(&input, &mut out_response, &mut out_token) };
        // out_response は S_OK 時のみ有効。意味判定は map_get_outcome が hr で行う。
        let bytes = out_response.to_string().into_bytes();
        map_get_outcome(hr, &bytes)
    }

    /// 片道イベント（NOTIFY）を組み立て `IShiori::Notify` を呼ぶ。応答は運ばない。
    /// `Err` は `error!`＋`Err(RequestError::Shiori(ShioriError::Parse))`（`Shiori3Client::notify` が
    /// 応答を破棄するのと同水準）。
    fn notify_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<(), RequestError> {
        let input = build_input(Method::Notify, id, references, status, *headers);
        let Some(shiori) = self.shiori.as_ref() else {
            error!(target: LOG_TARGET, id, "unload 済み backend への NOTIFY（IShiori 不在・Parse へ集約）");
            return Err(RequestError::Shiori(ShioriError::Parse));
        };
        // SAFETY: `shiori` は有効な IShiori。`input` は有効な借用 HSTRING（片道・応答なし）。
        match unsafe { shiori.Notify(&input) } {
            Ok(()) => Ok(()),
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    id,
                    error = %e,
                    "IShiori::Notify が失敗（片道ゆえ Parse へ集約・D-7）"
                );
                Err(RequestError::Shiori(ShioriError::Parse))
            }
        }
    }

    /// 正規 teardown。宣言順（`shiori`→`host`→`library`）で明示 take＝drop し、DLL 実装 COM 参照を
    /// 全 Release した後に `FreeLibrary`（`library` drop）する。teardown は失敗しない（Release は失敗せず、
    /// FreeLibrary 失敗は `InProcLibrary` の Drop 内 `error!` のみ）ので**常に** `Ok(ExitKind::Clean)`。
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        let _ = self.shiori.take(); // 1) DLL 実装 COM 参照を先に Release
        let _ = self.host.take(); // 2) areka-ghost 実装 host を Release
        let _ = self.library.take(); // 3) 最後に FreeLibrary（順序不変条件）
        self.unloaded = true;
        Ok(ExitKind::Clean)
    }

    /// 死活問い合わせ。別プロセス helper が無く死活監視対象がないため、ロード中は常に `Running`。
    /// `unload` 後は `Exited(ExitKind::Clean)`（run_shiori_loop は unload 後 status を読まない）。
    fn status(&mut self) -> HelperStatus {
        if self.unloaded {
            HelperStatus::Exited(ExitKind::Clean)
        } else {
            HelperStatus::Running
        }
    }
}

/// InProc 結線の connect closure を構成する。`real_connect`（別プロセス Helper 版）の InProc 版で、
/// 実行は shiori アクタースレッド上で一度だけ（`spawn_shiori_actor` の契約）。手順:
/// 1. `shiori.file` が `None`（DLL 名未解決）→ 即 `Err`（推測しない・`real_connect` と同じ）。
/// 2. `dir` を helper の道と同じ形（絶対・`\`・末尾 1 つ）に整え、[`InProcLibrary::load`]`(dir.join(file))`
///    で x64 DLL をロードし `IShioriFactory` を解決。
/// 3. [`InProcHost::new`] を `IShioriHost` 化。
/// 4. `factory.CreateInstance(load_dir, shiori_name, host)` で load 完了済み `IShiori` を得る。`factory`
///    は直後にスコープを落として Release する（`FreeLibrary` より先）。
/// 5. `shiori`／`host`／`library` を宣言順で保持する [`InProcBackend`] を構築して返す。
///
/// いずれの失敗も `error!` 済みの `Err(String)`（呼び出し側 `spawn_shiori_actor` が `ShioriDown` へ写す）。
/// `pub` なのは、テストの `Recorder` 合成と将来の直接利用の両方に供するため。
pub fn inproc_connect(
    shiori: ShioriMount,
) -> impl FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send + 'static {
    move || {
        // file 未解決なら推測せず即座に失敗する（`real_connect` と同じ）。
        let Some(shiori_name) = shiori.file else {
            error!(
                target: LOG_TARGET,
                dir = %shiori.dir.display(),
                "SHIORI DLL のファイル名が未解決のため InProc 接続できない（推測しない・要件 3.5）"
            );
            return Err(format!(
                "SHIORI DLL のファイル名が未解決のため接続できません（dir={}）",
                shiori.dir.display()
            ));
        };
        // helper の道と同じ形（絶対・`\`・末尾 1 つ）に整える。DLL は絶対パスでしか読まない（#28）。
        let load_dir = crate::shiori_wiring::helper_load_dir(&shiori.dir).map_err(|e| {
            error!(
                target: LOG_TARGET,
                dir = %shiori.dir.display(),
                error = %e,
                "SHIORI のフォルダを絶対パスに整えられない（InProc 接続できない）"
            );
            format!(
                "SHIORI のフォルダを絶対パスに整えられません（dir={}）: {e}",
                shiori.dir.display()
            )
        })?;
        let dll_path = load_dir.join(&shiori_name);

        // x64 DLL をロードし factory を解決（失敗は error! 済み Err(String)）。
        let (library, factory) = InProcLibrary::load(&dll_path)?;

        let host: IShioriHost = InProcHost::new().into();

        let load_dir_h = HSTRING::from(load_dir.as_os_str());
        let shiori_name_h = HSTRING::from(shiori_name.as_str());
        let mut out: Option<IShiori> = None;
        // SAFETY: `factory` は shiori_factory が move-out した有効な IShioriFactory。host は Ref 借用、
        // out は OutRef 書込先（成功時のみ move-out・失敗時は未書込）。
        unsafe {
            factory.CreateInstance(
                &load_dir_h,
                &shiori_name_h,
                (&host).into(),
                (&mut out).into(),
            )
        }
        .map_err(|e| {
            error!(
                target: LOG_TARGET,
                path = %dll_path.display(),
                error = %e,
                "IShioriFactory::CreateInstance が失敗（InProc 接続確立失敗・要件 3.5）"
            );
            format!("CreateInstance failed for {}: {e}", dll_path.display())
        })?;

        // `FreeLibrary`（library drop）より先に Release するため明示 drop する。
        drop(factory);

        let Some(shiori_iface) = out else {
            // 成功 HRESULT だが out 未書込（契約違反）。
            error!(
                target: LOG_TARGET,
                path = %dll_path.display(),
                "CreateInstance は成功したが IShiori が move-out されなかった（契約違反）"
            );
            return Err(format!(
                "CreateInstance succeeded but produced no IShiori for {}",
                dll_path.display()
            ));
        };

        Ok(Box::new(InProcBackend {
            shiori: Some(shiori_iface),
            host: Some(host),
            library: Some(library),
            unloaded: false,
        }) as Box<dyn ShioriBackend>)
    }
}

#[cfg(test)]
#[path = "shiori_inproc_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "shiori_inproc_host_tests.rs"]
mod host_tests;

#[cfg(test)]
#[path = "shiori_inproc_adapter_tests.rs"]
mod adapter_tests;

#[cfg(test)]
#[path = "shiori_inproc_headers_tests.rs"]
mod headers_tests;
