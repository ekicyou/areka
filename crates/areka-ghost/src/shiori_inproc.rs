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

use core::ffi::c_void;
use std::marker::PhantomData;
use std::path::Path;

use shiori_abi::interface::IShioriFactory;
use tracing::error;
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::core::{HRESULT, HSTRING, Interface, s};

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

#[cfg(test)]
mod tests {
    //! task 2.1 の 4 態様檻（design.md Testing Strategy・要件 3.1/3.5）:
    //! 1. 正常ロード（happy path）——1.1 spike の deps-dir 解決で built cdylib を実ロード。
    //! 2. DLL 欠落——不在パスで `LoadLibraryW` 失敗。
    //! 3. 不正イメージ——非 PE テキストファイルを `.dll` 名でロードし失敗。
    //! 4. シンボル未解決——`shiori_factory` を持たない実 x64 DLL（kernel32.dll）で `GetProcAddress` 失敗。
    //!
    //! いずれの失敗態様も `Err` を返し（silent success を偽装しない・要件 3.5）、取得済み HMODULE は
    //! Drop で解放される。COM は初期化しない（D-6）。

    use super::*;

    /// 態様2（**DLL 欠落**・`DLL欠落`）: 実在しないパスは `LoadLibraryW` が失敗し `Err`
    /// （要件 3.5）。アーティファクト不要の決定論檻。
    #[test]
    fn missing_dll_returns_err() {
        let result = InProcLibrary::load(Path::new(r"Z:\does\not\exist\nope.dll"));
        let err = result.err().expect("欠落 DLL は Err を返すこと");
        assert!(
            err.contains("LoadLibraryW failed"),
            "欠落 DLL はロード失敗として顕在化すること: {err}"
        );
    }

    /// 態様3（**不正イメージ**・`不正イメージ`）: 非 PE テキストを `.dll` 名で置き `LoadLibraryW` に
    /// 拒否させる。一意な一時ファイルを使い、檻の後始末で削除する（決定論・要件 3.5）。
    #[test]
    fn invalid_image_returns_err() {
        // 一意な一時 .dll パス（プロセス id＋nanos で衝突回避）。
        let unique = format!(
            "areka_inproc_invalid_{}_{}.dll",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let path = std::env::temp_dir().join(&unique);
        std::fs::write(&path, b"not a real dll").expect("一時不正イメージを書き出す");

        let result = InProcLibrary::load(&path);

        // 後始末（best-effort）: assert より先に一時ファイルを掃除する。
        let _ = std::fs::remove_file(&path);

        let err = result.err().expect("不正イメージ（非 PE）は Err を返すこと");
        assert!(
            err.contains("LoadLibraryW failed"),
            "不正イメージはロード失敗として顕在化すること: {err}"
        );
    }

    /// 態様4（**シンボル未解決**・`シンボル未解決`）: `kernel32.dll` は必ず存在し `LoadLibraryW` は
    /// 成功するが `shiori_factory` を持たない。ゆえに解決段で `GetProcAddress` 失敗＝`Err`（要件 3.5）。
    /// エラー文言でロード失敗ではなく**シンボル解決失敗**であることを確認する（決定論）。
    #[test]
    fn unresolved_symbol_returns_err() {
        // 名前ロード（system DLL はプロセス常駐・検索パスで解決）。
        let result = InProcLibrary::load(Path::new("kernel32.dll"));
        let err = result.err().expect("shiori_factory を持たない DLL は Err を返すこと");
        // ロード失敗ではなくシンボル解決失敗であることを区別する。
        assert!(
            err.contains("shiori_factory") && err.contains("unresolved"),
            "kernel32 は LoadLibraryW 成功後 shiori_factory の GetProcAddress で失敗すること: {err}"
        );
        assert!(
            !err.contains("LoadLibraryW failed"),
            "kernel32 のロード自体は成功する（失敗はシンボル段）: {err}"
        );
    }

    /// 態様1（**正常ロード**・`正常ロード`）: 1.1 spike の deps-dir 解決で built cdylib を実ロードし、
    /// `shiori_factory` を解決して `IShioriFactory` を得る（要件 3.1）。
    ///
    /// cdylib は `cargo test --workspace` が同一 deps ディレクトリへ uplift する
    /// （`current_exe().parent().join(DLL_FILE_NAME)`・1.1 spike 実測／design.md D-1・tasks.md
    /// Implementation Notes）。deps を pop しない。**不在時は silent skip せず明示 panic**（要件 3.1／
    /// design.md D-1・workspace-artifact 先例）。
    #[test]
    fn happy_path_loads_and_resolves_factory() {
        let test_exe = std::env::current_exe().expect("test executable path is available");
        let deps_dir = test_exe
            .parent()
            .expect("test executable resides in a deps directory");
        let dll_path = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);

        assert!(
            dll_path.exists(),
            "built test DLL が正準位置に不在: {}\n\
             この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置（deps）へ出力する。\
             単独実行時は先に `cargo test --workspace`（または `cargo build -p shiori4-testdll`）を\
             実行すること（フォールバックは設けない・design.md D-1・1.1 spike）。",
            dll_path.display()
        );

        let (library, factory) =
            InProcLibrary::load(&dll_path).expect("built cdylib は正常ロードされ factory を解決すること");

        // 最小の生存確認: 生成した factory を IUnknown へ cast できる（＝有効な COM 参照）。
        let _unknown: windows::core::IUnknown =
            factory.cast().expect("IShioriFactory は IUnknown へ cast 可能な有効 COM 参照であること");

        // FreeLibrary 順序不変条件（モジュール doc）に従い、COM 参照（factory）を先に、
        // ロード済みライブラリを後に解放する。
        drop(_unknown);
        drop(factory);
        drop(library);
    }
}
