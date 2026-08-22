//! **InProcLibrary**（design.md「InProc 結線」§InProcLibrary・要件 3.1/3.5）。
//!
//! x64 SHIORI4 DLL のロードと生成入口（`shiori_factory`）解決を担う RAII 機構。helper 側の
//! flat-C プロキシ（`crates/shiori-host32-helper/src/shiori_proxy.rs`）の **x64/COM 版**であり、
//! `load`/`unload`/`request` の 3 フラット関数ではなく `shiori_factory`
//! （`extern "system" fn(*mut *mut c_void) -> HRESULT`）1 本を解決し、そこから
//! [`IShioriFactory`](shiori_abi::interface::IShioriFactory) を [`Interface::from_raw`] で興す。
//!
//! ## 確立シーケンス（design.md §InProcLibrary Responsibilities）
//! 1. `LoadLibraryW(dll_path)`（絶対パス想定・UTF-16 NUL 終端）。失敗（DLL 欠落・不正イメージ）→
//!    `error!` 済み `Err(String)`（要件 3.5・log-first）。
//! 2. `GetProcAddress("shiori_factory")`。未解決（シンボル欠落）→ `error!` 済み `Err(String)`。
//!    **この時点で HMODULE は既にロード済みゆえ、失敗経路でも確実に解放する**——本関数は手順 1
//!    成功直後に [`InProcLibrary`] を構築し、以後の失敗は `?`/`return` でスコープを抜けることで
//!    Drop（`FreeLibrary`）を確実に走らせる（半構築非露出・取得済みリソースの確実解放）。
//! 3. 解決した proc を `unsafe extern "system" fn(*mut *mut c_void) -> HRESULT` へ transmute し呼出。
//!    失敗 HRESULT／成功だが null out → `error!` 済み `Err(String)`。
//! 4. 成功時のみ `IShioriFactory::from_raw(out)` を興し `(InProcLibrary, IShioriFactory)` を返す。
//!
//! ## FreeLibrary 順序不変条件（design.md §InProcLibrary・要件 3.4）
//! **DLL が実装する COM オブジェクトの全参照が Release された後にのみ `FreeLibrary` してよい**
//! （違反は UB）。[`InProcLibrary`] 自身は HMODULE のみを保持し、この順序保証はバックエンド全体
//! （task 2.3 の `InProcBackend` フィールド宣言順＝`shiori`→`host`→`library` の drop 順）で
//! **構造的**に担保する。したがって [`InProcLibrary`] は生成した [`IShioriFactory`]（および後続の
//! `IShiori`）より **後に** drop されねばならない。本モジュールはこの不変条件を rustdoc として
//! 明文化するに留め、実際の順序は所有者（`InProcBackend`）のフィールド順で保証する。
//!
//! ## スレッド座・COM（design.md D-6）
//! 全 COM 参照と HMODULE は shiori アクタースレッド常駐（`!Send`）。`CoInitializeEx` は**呼ばない**
//! （直接 vtable dispatch のみ・アクティベーション/マーシャリング非使用）。[`InProcLibrary`] は
//! 生ポインタ相当の HMODULE＋非 `Send` マーカーで `!Send` を保つ。
//!
//! ## 失敗経路のログ規律（要件 3.5・memory areka-log-first-no-silent-failure）
//! すべての失敗経路は `Err` を返す前に `error!`（`target: "ghost-shiori-inproc"`）でログする。
//! silent に成功を偽装しない。panic は用いない（総和的に `Err(String)` を返す）。
//!
//! ## consume 点
//! 本モジュールは task 2.1 で新設され、`InProcBackend`／`inproc_connect`（DLL ロード→factory→
//! `IShiori`→backend 写像）からの consume は **task 2.3** が行う。それまで production 経路では
//! 未使用ゆえ [`allow(dead_code)`] で dead_code 警告を抑止する（template `shiori_proxy.rs` と同律・
//! 結線時に消費される）。in-crate 檻は `load` を実消費する。

// task 2.3 が `InProcBackend`／`inproc_connect` から本モジュールを consume するまで production 経路
// では未使用。それによる dead_code 警告を抑止する（結線時に消費される）。
#![allow(dead_code)]
// `#[implement(IShioriHost)]`（InProcHost）が生成する実装トレイト面は COM 規約の PascalCase
// メソッド名（`Raise`/`Complete`/`GetProperty`/`SetProperty`）を要求する。ABI（interface.rs）と
// 同律で、本モジュール全体で non_snake_case を許可する。
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
    Charset, ExitKind, HelperStatus, Method, RequestError, ShioriError, ShioriRequest,
    ShutdownError, build_request, parse_response,
};
use tracing::{error, warn};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
// `Result` は glob 導入しない——本モジュールの `load` は `std::result::Result<_, String>` を返すため、
// COM の `Result<()>` は別名（`ComResult`）で受け、std `Result` を shadow しない。
use windows::core::{HRESULT, HSTRING, Interface, Result as ComResult, implement, s};

/// tracing target（design.md Monitoring・本モジュールの全ログ発火で共有）。
const LOG_TARGET: &str = "ghost-shiori-inproc";

/// `shiori_factory` 生成入口の署名（design.md §InProcLibrary・shiori4-testdll／reference_brain と同形）。
///
/// `extern "system"`（Windows COM 標準の呼出規約・x64/ARM64 では `extern "C"` と同一 ABI だが COM
/// 整合で正準表記は `system`）。`out` へ refcount 1 の `IShioriFactory` を move-out し `HRESULT` を返す。
type ShioriFactoryFn = unsafe extern "system" fn(*mut *mut c_void) -> HRESULT;

/// x64 SHIORI4 DLL のロード RAII（design.md §InProcLibrary・要件 3.1/3.5）。
///
/// ロード済み HMODULE を保持し、Drop で `FreeLibrary` する（best-effort・失敗は `error!` のみ）。
/// `!Send`（COM 参照＋HMODULE のスレッド常駐・D-6）——`PhantomData<*const ()>` で明示的に非 `Send`。
///
/// **FreeLibrary 順序不変条件**（モジュール doc 参照）: DLL 実装 COM オブジェクトの全参照が Release
/// された後にのみ `FreeLibrary` してよい。本型はその順序を強制しない（HMODULE を持つだけ）——順序は
/// 所有者（`InProcBackend`・task 2.3）のフィールド宣言順で構造的に担保する。
pub(crate) struct InProcLibrary {
    /// ロード元モジュールハンドル。Drop で `FreeLibrary` する（唯一の teardown 経路）。
    module: HMODULE,
    /// `!Send` マーカー（D-6・HMODULE が仮に `Send` でも本型を非 `Send` に固定する）。
    _not_send: PhantomData<*const ()>,
}

impl InProcLibrary {
    /// DLL をロードし `shiori_factory` を解決して [`IShioriFactory`] を生成する（design.md §InProcLibrary）。
    ///
    /// 失敗（欠落 DLL・不正イメージ・シンボル未解決・factory 生成失敗）はいずれも `error!` ログ済みの
    /// `Err(String)` として返し、silent に成功を偽装しない（要件 3.5・log-first）。取得済み HMODULE は
    /// いかなる失敗経路でも Drop（`FreeLibrary`）で確実に解放される（半構築非露出）。
    ///
    /// 成功時は `(InProcLibrary, IShioriFactory)` を返す。**呼び出し側は FreeLibrary 順序不変条件に従い、
    /// 返した `IShioriFactory`（および派生 COM 参照）を [`InProcLibrary`] より先に drop すること**
    /// （モジュール doc 参照）。
    ///
    /// # Preconditions
    /// `dll_path` は絶対パス想定（呼び出し側が `load_dir.join(shiori_name)` で組む・design.md §InProc 結線）。
    /// 呼び出しは shiori アクタースレッド上（D-6）。`CoInitializeEx` は呼ばない。
    pub(crate) fn load(dll_path: &Path) -> Result<(Self, IShioriFactory), String> {
        // --- 手順 1: LoadLibraryW（絶対パス・UTF-16 NUL 終端）---
        // HSTRING は NUL 終端 UTF-16 を保持する。失敗（DLL 欠落・不正イメージ）→ error! 済み Err。
        let wide = HSTRING::from(dll_path.as_os_str());
        // SAFETY: `wide` は呼出中生存する有効な NUL 終端 UTF-16。LoadLibraryW は失敗時 Err を返す（下で map）。
        // 成功で得た HMODULE の解放責務はこの直後に構築する InProcLibrary（Drop）へ移る。
        let module = match unsafe { LoadLibraryW(&wide) } {
            Ok(m) => m,
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    path = %dll_path.display(),
                    error = %e,
                    "InProc DLL のロードに失敗（DLL 欠落・不正イメージ等）"
                );
                return Err(format!(
                    "LoadLibraryW failed for {}: {e}",
                    dll_path.display()
                ));
            }
        };

        // 手順 1 成功直後に RAII を構築する。以後の失敗は `return`（スコープ離脱）で Drop=FreeLibrary を
        // 確実に走らせる＝取得済み HMODULE を全失敗経路で解放する（半構築非露出）。
        let library = InProcLibrary {
            module,
            _not_send: PhantomData,
        };

        // --- 手順 2: GetProcAddress("shiori_factory") ---
        // SAFETY: `library.module` は直前にロードした有効ハンドル。`s!` は NUL 終端 C 文字列リテラル。
        // GetProcAddress は未解決時 None を返す（unwrap せず観測エラー化）。
        let proc = match unsafe { GetProcAddress(library.module, s!("shiori_factory")) } {
            Some(p) => p,
            None => {
                error!(
                    target: LOG_TARGET,
                    path = %dll_path.display(),
                    symbol = "shiori_factory",
                    "InProc 生成入口 shiori_factory が未解決（シンボル欠落）"
                );
                // library は本 return でスコープを抜け Drop=FreeLibrary される（取得済み HMODULE 解放）。
                return Err(format!(
                    "GetProcAddress(\"shiori_factory\") unresolved in {}",
                    dll_path.display()
                ));
            }
        };

        // --- 手順 3: transmute → 呼出 ---
        // SAFETY: 解決した FARPROC を design.md §InProcLibrary で固定した生成入口署名 ShioriFactoryFn へ
        // transmute する。DLL 側実体（shiori4-testdll／将来の native x64 SHIORI4）が同形の
        // `#[unsafe(no_mangle)] extern "system" fn(*mut *mut c_void) -> HRESULT` を公開する（要件 2.1）。
        let factory_fn: ShioriFactoryFn =
            unsafe { core::mem::transmute::<_, ShioriFactoryFn>(proc) };

        let mut out: *mut c_void = core::ptr::null_mut();
        // SAFETY: `factory_fn` は上で解決した有効な生成入口。`&mut out` は有効な書込先ポインタ。
        // 呼出は同期で HRESULT を返す（D-6・直接 vtable dispatch・CoInitializeEx 不要）。
        let hr = unsafe { factory_fn(&mut out) };
        if hr.is_err() {
            error!(
                target: LOG_TARGET,
                path = %dll_path.display(),
                hr = format_args!("0x{:08X}", hr.0 as u32),
                "InProc factory 生成が失敗 HRESULT を返した"
            );
            // library は本 return でスコープを抜け Drop=FreeLibrary される。
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

        // --- 手順 4: IShioriFactory::from_raw ---
        // SAFETY: `out` は生成入口が成功時に move-out した refcount 1 の IShioriFactory 生ポインタ
        // （上で null と失敗 HRESULT を排除済み）。from_raw は AddRef せず単一参照を adopt する
        // ＝呼び出し側が Release（drop）義務を負う。所有権は返り値へ移る。
        let factory = unsafe { IShioriFactory::from_raw(out) };
        Ok((library, factory))
    }
}

impl Drop for InProcLibrary {
    /// `FreeLibrary`（design.md §InProcLibrary・要件 3.4・best-effort）。
    ///
    /// 失敗は `error!` のみ（best-effort teardown・エラーとして伝播しない）。**FreeLibrary 順序不変条件**
    /// （モジュール doc）: 本 Drop は DLL 実装 COM オブジェクトの全参照が Release された後に走らねば
    /// ならない——その順序保証は所有者のフィールド宣言順で構造的に担保する（本型は強制しない）。
    fn drop(&mut self) {
        // SAFETY: `module` は `load` でロードした有効ハンドル。多重解放は所有（move）で防がれる。
        // FreeLibrary の結果は best-effort で扱い、失敗は error! のみ（silent failure を避ける）。
        if let Err(e) = unsafe { FreeLibrary(self.module) } {
            error!(
                target: LOG_TARGET,
                error = %e,
                "InProc DLL の FreeLibrary に失敗（best-effort・teardown 継続）"
            );
        }
    }
}

/// `CreateInstance` へ渡す areka-ghost 側の最小 `IShioriHost` 実装（design.md §InProcHost・要件 3.1/7.4）。
///
/// M1 InProc 経路が実際に消費する能力集合＝要件 7.4 の範囲に等しく、それを超える実配線は持たない:
/// - `Raise`: `warn!` で受領を可視化（握りつぶさない）した上で `Ok(())`。M1 InProc に Raise 消費者は
///   存在しないため実配送しない（要件 7.4・自発通知の網羅は範囲外）。
/// - `Complete`: 常に `Err(SHIORI_E_UNKNOWN_TOKEN)`。deferred（`SHIORI_S_PENDING`）非対応ゆえ突合すべき
///   pending 枠を持たない（要件 7.4・遅延応答の完了は範囲外）。
/// - `SetProperty` / `GetProperty`: 内部 `HashMap` を単純に往復する。欠落 key は暗黙の空値で続行せず
///   `Err(SHIORI_E_PROPERTY_NOT_FOUND)`（out_value 未書込）で判別可能にする。
///
/// areka bin の `ShioriHostSink`（メールボックス・突合枠つき）は能力集合が異なる別物であり移設しない。
/// M2 native 消費時に host 注入シームごと再設計する（design.md §InProcHost・Revalidation Trigger）。
///
/// スレッド座: `RefCell`＋COM 参照ゆえ実際に `!Send`。shiori アクタースレッド常駐で用いる（D-6）。
#[implement(IShioriHost)]
pub(crate) struct InProcHost {
    /// プロパティストア（`SetProperty` 格納・`GetProperty` 即答）。
    properties: RefCell<HashMap<String, HSTRING>>,
}

impl InProcHost {
    /// 空のプロパティストアを持つ最小 host を生成する（design.md §InProcHost）。
    ///
    /// 生成物は `.into()` で `IShioriHost` 化して [`IShioriFactory::CreateInstance`] へ渡す。
    pub(crate) fn new() -> Self {
        Self {
            properties: RefCell::new(HashMap::new()),
        }
    }
}

// windows-core 0.62: `#[implement]` 生成の `InProcHost_Impl` に対し pub vtable メソッドを実装する。
impl IShioriHost_Impl for InProcHost_Impl {
    /// 能動通知（wakeup）。M1 InProc に消費者はいないため実配送せず、`warn!` で受領を可視化して
    /// `Ok(())`（要件 7.4・握りつぶさない）。
    unsafe fn Raise(&self, script: &HSTRING) -> ComResult<()> {
        warn!(
            target: LOG_TARGET,
            script = %script,
            "InProcHost が Raise（自発通知）を受領したが M1 InProc に消費者はいない（実配送せず・要件 7.4）"
        );
        Ok(())
    }

    /// 遅延応答の完了配送。deferred（`SHIORI_S_PENDING`）非対応ゆえ突合する pending 枠を持たず、
    /// 任意トークンを未知として `Err(SHIORI_E_UNKNOWN_TOKEN)` で拒否する（要件 7.4）。
    unsafe fn Complete(&self, _token: u64, _response: &HSTRING) -> ComResult<()> {
        Err(SHIORI_E_UNKNOWN_TOKEN.into())
    }

    /// プロパティストアから同期即答する。欠落 key は暗黙の空値で続行せず
    /// `Err(SHIORI_E_PROPERTY_NOT_FOUND)`（out_value 未書込・design.md §InProcHost）。
    unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> ComResult<()> {
        match self.properties.borrow().get(&key.to_string()) {
            Some(v) => {
                // 存在時のみ値を out へ move-out（callee 確保・caller 解放）。
                *out_value = v.clone();
                Ok(())
            }
            // 欠落 key は判別可能な失敗（out_value は書き込まない）。
            None => Err(SHIORI_E_PROPERTY_NOT_FOUND.into()),
        }
    }

    /// プロパティストアへ値を即書きする（`[in]` 借用は保持のため clone・所有権規約）。
    unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> ComResult<()> {
        self.properties
            .borrow_mut()
            .insert(key.to_string(), value.clone());
        Ok(())
    }
}

/// 送出 `Sender` ヘッダ値（design.md §InProcBackend・`Shiori3Client`/`real_connect` と同じく
/// SSP を詐称せず自身を `"areka"` と正直に名乗る単一値）。
const SENDER: &str = "areka";

/// `IShiori::Get` の観測（HRESULT＋応答バイト列）を既存の結果語彙 `Result<Option<String>, RequestError>`
/// へ機械的に写す**純関数**（design.md「`IShiori::Get` 応答の写像表」・要件 3.2）。
///
/// `response_bytes` は `out_response` HSTRING の UTF-8 表現で、`hr == S_OK` のときのみ意味を持つ。
/// host-32 の `Shiori3Client::map_get_result` と同一規律で写す（status 分類を再実装せず同じ判断へ
/// 機械写像・要件 3.2）:
///
/// | 観測 | 写像 |
/// |---|---|
/// | `S_OK`＋parse 成功・status 200 他 | `Ok(Some(value))`／value 欠落は `Ok(None)` |
/// | `S_OK`＋status 204 | `Ok(None)` |
/// | `S_OK`＋status 400/500 または `ErrorLevel` あり | `Err(RequestError::Shiori(ShioriError::Status{..}))` |
/// | `S_OK`＋parse 失敗 | `error!`＋`Err(RequestError::Shiori(ShioriError::Parse))` |
/// | [`SHIORI_S_PENDING`] | `error!`（M1 InProc 非対応）＋`Err(..Parse)`（防衛線・設計上到達しない・要件 7.4） |
/// | その他の error HRESULT | `error!`（hr 値つき）＋`Err(..Parse)`（解釈不能を Parse へ集約） |
///
/// **語彙不変**（D-7）: `RequestError`/`ShioriError` へ新 variant を足さず、解釈不能はすべて
/// `ShioriError::Parse` へ集約する（詳細は log が正本）。総和的な純関数で **panic しない**——
/// COM を要さず全分岐を単体テストできる。
fn map_get_outcome(hr: HRESULT, response_bytes: &[u8]) -> Result<Option<String>, RequestError> {
    // 遅延応答（SHIORI_S_PENDING）は「成功」コードだが M1 InProc は非対応。S_OK 判定より先に弾く
    // （防衛線・設計上テスト DLL では到達しない・要件 7.4・§7.4）。
    if hr == SHIORI_S_PENDING {
        error!(
            target: LOG_TARGET,
            "IShiori::Get が SHIORI_S_PENDING（遅延応答）を返した——M1 InProc は遅延応答を非対応（防衛線・要件 7.4）"
        );
        return Err(RequestError::Shiori(ShioriError::Parse));
    }
    // 失敗 HRESULT: サポート契約下では解釈不能ゆえ Parse へ集約（詳細は log が正本・D-7）。
    if hr.is_err() {
        error!(
            target: LOG_TARGET,
            hr = format_args!("0x{:08X}", hr.0 as u32),
            "IShiori::Get が失敗 HRESULT を返した（解釈不能→Parse へ集約・D-7）"
        );
        return Err(RequestError::Shiori(ShioriError::Parse));
    }
    // S_OK（即時応答）: 応答バイト列を SHIORI/3.0 として解析し、map_get_result と同一規律で写す。
    let parsed = match parse_response(response_bytes, Charset::Utf8) {
        Ok(p) => p,
        Err(_) => {
            error!(
                target: LOG_TARGET,
                "IShiori::Get の応答が SHIORI/3.0 として解析不能（codec 契約違反→Parse）"
            );
            return Err(RequestError::Shiori(ShioriError::Parse));
        }
    };
    // ① エラー条件を先に判定（400/500/ErrorLevel は SHIORI エラー・map_get_result 同律・要件 3.2）。
    //    error_level を先に見るのは 200 でも ErrorLevel 付きはエラー扱いにするため。
    if parsed.status == 400 || parsed.status == 500 || parsed.error_level.is_some() {
        return Err(RequestError::Shiori(ShioriError::Status {
            status: parsed.status,
            error_level: parsed.error_level,
            error_description: parsed.error_description,
        }));
    }
    // ② 204 は成功・スクリプト無し。
    if parsed.status == 204 {
        return Ok(None);
    }
    // ③ 200・その他の許容 status は Value をそのまま返す（欠落は None）。
    Ok(parsed.value)
}

/// GET/NOTIFY 共通の request 組み立て（`build_request` → HSTRING）。
///
/// `build_request` の出力は shiori3 codec 契約により常に有効な UTF-8（要件 1.6）ゆえ、
/// `from_utf8_lossy` は逐語一致し panic しない。`sender` は [`SENDER`]（`"areka"`）固定、
/// `charset` は [`Charset::Utf8`] 固定（本仕様の唯一の実符号化）。`status` は kanade が render 済みの
/// wire 値で、解釈せず codec へ verbatim 透過する（DD-IT-1 語彙非漏洩）。
fn build_input(method: Method, id: &str, references: &[String], status: Option<&str>) -> HSTRING {
    let bytes = build_request(&ShioriRequest {
        method,
        id,
        references,
        sender: SENDER,
        status,
        charset: Charset::Utf8,
    });
    HSTRING::from(String::from_utf8_lossy(&bytes).as_ref())
}

/// `IShiori`（load 完了済み脳）を kanade の [`ShioriBackend`] へ機械写像する正規アダプタ
/// （design.md §InProcBackend・要件 3.2〜3.4/7.1）。
///
/// **フィールド宣言順＝drop 順で FreeLibrary 順序不変条件を構造的に担保する**（D-6・`InProcLibrary`
/// モジュール doc）: `shiori`（DLL 実装 COM 参照）→ `host`（areka-ghost 実装・DLL 非依存）→
/// `library`（`FreeLibrary`）の順に drop され、DLL 実装 COM オブジェクトの全参照が Release された後に
/// のみ `FreeLibrary` が走る。カスタム `Drop` は設けない——Rust のフィールド宣言順 drop がこの不変条件
/// そのものだからである（`unload` 未実行で drop された場合も同順で teardown される）。
///
/// `!Send`（`IShiori`/`IShioriHost` COM 参照＋`InProcLibrary` の HMODULE がスレッド常駐・D-6）。
/// `CoInitializeEx` は呼ばない（直接 vtable dispatch のみ・D-6）。
pub(crate) struct InProcBackend {
    /// load 完了済み脳（`IShiori`）。**最初に** drop＝Release される（DLL 実装 COM 参照）。
    shiori: Option<IShiori>,
    /// `CreateInstance` へ渡した areka-ghost 側 host（DLL 非依存）。2 番目に drop される。
    host: Option<IShioriHost>,
    /// ロード済み DLL（`FreeLibrary` は RAII）。**最後に** drop＝FreeLibrary される（順序不変条件）。
    library: Option<InProcLibrary>,
    /// `unload` 完了フラグ（`status()` の Running/Exited 判別・D-7）。
    unloaded: bool,
}

impl ShioriBackend for InProcBackend {
    /// 応答を要するイベント（GET）を組み立て `IShiori::Get` を呼び、観測を [`map_get_outcome`] で既存
    /// 語彙へ写す（要件 3.2）。
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        let input = build_input(Method::Get, id, references, status);
        let Some(shiori) = self.shiori.as_ref() else {
            // unload 後の呼び出しは構造上起きない（run_shiori_loop は unload 後 get しない）が、panic
            // せず log-first で失敗化する（要件 3.5・areka-log-first-no-silent-failure）。
            error!(target: LOG_TARGET, id, "unload 済み backend への GET（IShiori 不在・Parse へ集約）");
            return Err(RequestError::Shiori(ShioriError::Parse));
        };
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        // SAFETY: `shiori` は CreateInstance が move-out した有効な IShiori。out-param は有効な書込先。
        // 直接 vtable dispatch（D-6・CoInitializeEx 不要）。
        let hr = unsafe { shiori.Get(&input, &mut out_response, &mut out_token) };
        // out_response（S_OK 時のみ有効）を UTF-8 バイト列へ。意味判定は map_get_outcome が hr で行う。
        let bytes = out_response.to_string().into_bytes();
        map_get_outcome(hr, &bytes)
    }

    /// 片道イベント（NOTIFY）を組み立て `IShiori::Notify` を呼ぶ（要件 3.2）。応答は運ばない片道性で、
    /// `Err` は `error!`＋`Err(RequestError::Shiori(ShioriError::Parse))`（`Shiori3Client::notify` が
    /// 応答を破棄するのと同水準・D-7 語彙不変）。
    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        let input = build_input(Method::Notify, id, references, status);
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

    /// 正規 teardown（要件 3.4・D-7）。宣言順（`shiori`→`host`→`library`）で明示 take＝drop し、DLL 実装
    /// COM 参照を全 Release した後に `FreeLibrary`（`library` drop）する。teardown は構造的に不能失敗
    /// （Release は失敗しない・FreeLibrary 失敗は `InProcLibrary` の Drop 内 `error!` のみ）ゆえ**常に**
    /// `Ok(ExitKind::Clean)`（D-7）。
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        // 明示 teardown を宣言順で行う（各 take() の戻り値は文末で即 drop＝解放）。
        let _ = self.shiori.take(); // 1) DLL 実装 COM 参照を先に Release
        let _ = self.host.take(); // 2) areka-ghost 実装 host を Release
        let _ = self.library.take(); // 3) 最後に FreeLibrary（順序不変条件）
        self.unloaded = true;
        Ok(ExitKind::Clean)
    }

    /// 死活問い合わせ（要件 3.3・D-7）。別プロセス helper が存在せず死活監視対象がないため、ロード中は
    /// 常に `Running`。`unload` 後は正直な語彙として `Exited(ExitKind::Clean)` を返す（run_shiori_loop は
    /// unload 後 status を読まないため挙動影響なし・D-7）。
    fn status(&mut self) -> HelperStatus {
        if self.unloaded {
            HelperStatus::Exited(ExitKind::Clean)
        } else {
            HelperStatus::Running
        }
    }
}

/// InProc 結線の connect closure を構成する（design.md §InProcBackend Service Interface・要件 3.1/3.5/7.1）。
///
/// `real_connect`（別プロセス Helper 版）の **InProc 版**であり、実行は shiori アクタースレッド上で
/// 一度だけ（`spawn_shiori_actor` 既存契約）。手順:
/// 1. `shiori.file` が `None`（DLL 名未解決）→ **即 `Err`**（推測しない・real_connect と同律・要件 3.5）。
/// 2. [`InProcLibrary::load`]`(dir.join(file))` で x64 DLL をロードし `IShioriFactory` を解決。
/// 3. [`InProcHost::new`] を `IShioriHost` 化。
/// 4. `factory.CreateInstance(load_dir, shiori_name, host)` で load 完了済み `IShiori` を得る。`factory`
///    は直後にスコープを落として Release する（いかなる `FreeLibrary` よりも先・design.md）。
/// 5. `shiori`／`host`／`library` を宣言順で保持する [`InProcBackend`] を構築して返す。
///
/// いずれの失敗も `error!` 済みの `Err(String)`（呼び出し側 `spawn_shiori_actor` が `ShioriDown` へ写す・
/// 要件 3.5）。`pub`（D-3・テストの `Recorder` 合成と M2 の直接利用の両方に供する・要件 7.1）。
pub fn inproc_connect(
    shiori: ShioriMount,
) -> impl FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send + 'static {
    move || {
        // 1. file 未解決なら推測せず即座に失敗（design.md「推測しない」・real_connect と同律・要件 3.5）。
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
        let load_dir = shiori.dir;
        let dll_path = load_dir.join(&shiori_name);

        // 2. x64 DLL をロードし factory を解決（失敗は error! 済み Err(String)・要件 3.5）。
        let (library, factory) = InProcLibrary::load(&dll_path)?;

        // 3. areka-ghost 側 host を IShioriHost 化（CreateInstance へ Ref 渡し・callee clone）。
        let host: IShioriHost = InProcHost::new().into();

        // 4. CreateInstance（生成＋load 融合）で load 完了済み IShiori を move-out する。
        let load_dir_h = HSTRING::from(load_dir.as_os_str());
        let shiori_name_h = HSTRING::from(shiori_name.as_str());
        let mut out: Option<IShiori> = None;
        // SAFETY: `factory` は shiori_factory が move-out した有効な IShioriFactory。host は Ref 借用、
        // out は OutRef 書込先（成功時のみ move-out・失敗時 out 未書込＝半構築非露出）。
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

        // factory は CreateInstance 直後に不要。`FreeLibrary`（library drop）より先に Release するため
        // 明示 drop する（design.md「factory は CreateInstance 直後にスコープ落ちで Release」）。
        drop(factory);

        let Some(shiori_iface) = out else {
            // 成功 HRESULT だが out 未書込（契約違反）——log-first で失敗化する（要件 3.5）。
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

        // 5. 宣言順（drop 順＝teardown 順）で資材を保持する backend を返す。
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
mod tests;

#[cfg(test)]
#[path = "shiori_inproc_host_tests.rs"]
mod host_tests;

#[cfg(test)]
#[path = "shiori_inproc_adapter_tests.rs"]
mod adapter_tests;
