//! ShioriByteProxy: SHIORI DLL の unsafe FFI（ロード・エクスポート解決・`load` 呼出・解放）を一点集約する
//! 常設プロキシ。SHIORI3/SHIORI4 のロジックは持たず、所有権・charset の規約を守って
//! 意味を持たない生バイト列を FFI 境界の向こうへ運ぶだけ。
//!
//! ## 確立シーケンス
//! 正典の入口は 4 つ（`loadu`／`load`／`unload`／`request`）。初期化の入口は `loadu`／`load` の
//! どちらか 1 つを選んで 1 回だけ呼ぶ。
//! 1. `LoadLibraryExW(dll_path, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)`
//!    （呼出側が `load_dir\<shiori_name>` を組んだ **絶対パス**を渡す。依存の DLL は読む DLL のフォルダと
//!    システムの既定から探し、作業フォルダと PATH は見ない。相対パスは OS が断る）。
//!    失敗→[`ProxyError::LoadLibraryFailed`]。
//! 2. `GetProcAddress` で初期化の入口 `loadu`・`load` を **任意で**引き、[`choose_init_entry`] で
//!    1 つ選ぶ（`loadu` 優先・`load` だけでも `loadu` だけでも可）。両方無い→
//!    [`ProxyError::EntryNotFound`]`("load")`。続けて `unload`・`request` を **必須で**引く。欠落→
//!    `EntryNotFound("unload")`／`("request")`。
//! 3. 入口別のバイト列を作る（[`init_bytes`]）: `loadu` なら `load_dir` の UTF-8 そのもの、`load` なら
//!    既定コードページ（CP_ACP・`WideCharToMultiByte`）。`load` の枝で表せない字があれば警告を 1 行
//!    出してそれでも渡す。符号化失敗→[`ProxyError::EncodingFailed`]。
//! 4. `GlobalAlloc(GMEM_FIXED)` バッファへ書き込み。確保失敗→[`ProxyError::EncodingFailed`]。
//! 5. 入口名の行（`[helper] SHIORI 初期化の入口: loadu|load`）を 1 行出し、選んだ入口 `(hdir, len)` を
//!    **同期で 1 回だけ**呼ぶ。戻り 0→[`ProxyError::LoadReturnedFalse`]（`loadu` が 0 でも `load` へは
//!    落ちない）。0 以外は成功。
//!
//! ## 所有権規約
//! `loadu`／`load` へ渡した入力 HGLOBAL は **DLL(callee) が `GlobalFree` する**。ホストは
//! **自ら解放しない**（二重解放になる）。確保したハンドルは入口の呼出で callee へ move する。
//!
//! ## charset
//! `loadu` の dir は **UTF-8**、`load` の dir は **ANSI(CP_ACP)**（pasta の `to_ansi_str()`＝`MultiByteToWideChar` と
//! 対称なので `WideCharToMultiByte(CP_ACP, ..)` で符号化する）。`request` は**任意の文字コードのバイト列**で、
//! 意味は x64 側の交渉で決まり、ここでは解釈しない。
//!
//! ## 後始末
//! [`ShioriByteProxy::close`] が `unload()` → `FreeLibrary` を行い、`unload` の偽と `FreeLibrary` の
//! 失敗を種類で返す（UNLOAD の道）。UNLOAD を経ない道は Drop が同じ後始末を行い、失敗を
//! `[helper]` の行で書く（二度閉じない）。確立途中（手順 2〜5）の失敗は内部で `FreeLibrary` してから
//! `Err` を返し（半構築を露出しない）、`FreeLibrary` の失敗は行で書く。
//!
//! ## 観測契約
//! 「初期化の入口の同期 1 バイト結果（0 か否か）＋無クラッシュ」のみ。DLL 内部スレッド等に前提を置かない。
//! 本モジュールが出す行（`crate::helper_log`・無条件）は入口名の 1 行、`load` の枝の表せない字の警告
//! 1 行、応答を読めないとき（`GlobalLock` で開けない・長さが確保を超える）の 1 行、後始末の失敗の行。
//! 確立の失敗の種別の 1 行は `main.rs` が出す。
//!
//! ## unsafe 集約
//! SHIORI FFI の `unsafe` 境界はこのモジュールへ集める。i686 では `usize` が 32bit で、`len` を `usize` で
//! 受ければ ABI と合う。

use std::path::Path;

use windows::Win32::Foundation::{FreeLibrary, GetLastError, GlobalFree, HGLOBAL, HMODULE};
use windows::Win32::Globalization::{
    CP_ACP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS, MultiByteToWideChar, WideCharToMultiByte,
};
use windows::Win32::System::Diagnostics::Debug::{
    SEM_FAILCRITICALERRORS, SEM_NOOPENFILEERRORBOX, SetThreadErrorMode, THREAD_ERROR_MODE,
};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LoadLibraryExW,
};
use windows::Win32::System::Memory::{
    GLOBAL_ALLOC_FLAGS, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::core::{HSTRING, PCSTR, s};

/// `GlobalAlloc` の `GMEM_FIXED`（=0）。pasta と同値で確保する。
/// `GMEM_FIXED` ではハンドル＝先頭ポインタゆえ `h.0 as *mut u8` で直接書き込める。
/// 同じ決まりの写し: testdll・loadu の `lib.rs` の `GMEM_FIXED`（決まりを変えるなら 3 か所とも）。
const GMEM_FIXED: GLOBAL_ALLOC_FLAGS = GLOBAL_ALLOC_FLAGS(0);

/// `loadu`／`load` 共通の flat-C cdecl 署名。
///
/// `i686-pc-windows-msvc` では C ABI＝cdecl。pasta の `extern "C"` と ABI 同一。戻りは **1 バイト整数**
/// （0 = 失敗・0 以外 = 成功・`== 1` とは判定しない）。Rust `bool` で受けると 0／1 以外の値が未定義動作に
/// なり、`i32` で受けると Rust `bool` を返す DLL（pasta）の不定な上位 24 bit を読むため、`u8` で受ける。
type LoadFn = unsafe extern "C" fn(hdir: HGLOBAL, len: usize) -> u8;
/// `unload() -> u8`（引数なし・cdecl）。後始末（`close`・Drop）で呼び、0 は `UnloadReturnedFalse`。
type UnloadFn = unsafe extern "C" fn() -> u8;
/// `request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（`len` は in/out）。
/// [`ShioriByteProxy::request`] が呼ぶ。
type RequestFn = unsafe extern "C" fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL;

/// DLL を、OS の窓（「正しくないイメージ」0xc000012f など）を出さずに読む。
///
/// `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` で、依存の DLL は読む DLL の
/// フォルダとシステムの既定から探し、作業フォルダと PATH は見ない（DLL の植え込みを塞ぐ）。
/// このフラグは絶対パスを要り、相対パスは OS が断る。旗が効くのはこの 1 回の読み込みの静的な依存だけ。
///
/// 既定では、壊れた DLL を読むと OS が利用者の画面に窓を出し、OK が押されるまでこのスレッドを止める。
/// 失敗は `LoadLibraryFailed` として返るので、窓は要らない。呼ぶ間だけこのスレッドのエラーモードに
/// `SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX` を立て、終えたら元へ戻す（プロセス全体の
/// `SetErrorMode` は触らない）。x64 側の `areka-ghost` の `shiori_inproc.rs` と同じ手当て。
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

/// SHIORI DLL 確立の失敗種別。panic せず観測できる形で返す。
// 中身は失敗の 1 行（`main.rs` の `{e:?}`）でだけ読む。rustc は derive した `Debug` を読み手に数えない。
#[allow(dead_code)]
#[derive(Debug)]
pub enum ProxyError {
    /// `LoadLibraryExW` 失敗（DLL 不在・相対パス・ロード失敗・ビットネス不一致など）。
    LoadLibraryFailed(windows::core::Error),
    /// 入口が無い。名札は `"load"`（初期化の入口 `loadu`・`load` が **両方無い**とき）・`"unload"`・
    /// `"request"` の 3 値。
    EntryNotFound(&'static str),
    /// `load_dir` の符号化失敗（`loadu` の UTF-8 化・`load` の ANSI(CP_ACP) 化）、または `GlobalAlloc` 確保失敗。
    EncodingFailed,
    /// 初期化の入口（`loadu`／`load`）が 0 を返した。
    /// `last_error` は返った直後に読んだ OS の最後のエラー。
    /// SHIORI の約束に「失敗したら最後のエラーを置く」は無いので、DLL の中の最後の呼び出しの結果が
    /// 残っていれば読める、という約束のない手がかり（0 や無関係の値もありうる）。
    LoadReturnedFalse { last_error: u32 },
    /// `request` が null／無効 HGLOBAL を返した（応答確保失敗など）。
    RequestFailed,
    /// `request` が書いた応答の長さ `declared` が、応答 HGLOBAL の確保の大きさ `allocated`
    /// （`GlobalSize`・0 は確かめられない）を超えた。確保の外は読まずに解放して返す。
    ResponseLengthMismatch { declared: usize, allocated: usize },
    /// 後始末の `unload` が 0 を返した（[`ShioriByteProxy::close`]）。
    UnloadReturnedFalse,
    /// 後始末の `FreeLibrary` が失敗した（[`ShioriByteProxy::close`]）。
    FreeLibraryFailed(windows::core::Error),
}

/// 選ばれた初期化の入口（`loadu` 優先）。fn ポインタを 1 つだけ持つので、
/// `loadu` と `load` の両方を呼ぶ経路は型の上で存在しない。
enum InitEntry {
    Loadu(LoadFn),
    Load(LoadFn),
}

impl InitEntry {
    /// 入口名（入口名の行と fixture の記録の語）。
    fn name(&self) -> &'static str {
        match self {
            InitEntry::Loadu(_) => "loadu",
            InitEntry::Load(_) => "load",
        }
    }

    /// 選ばれた入口の fn ポインタ（呼び手はこれを 1 回だけ呼ぶ）。
    fn func(&self) -> LoadFn {
        match *self {
            InitEntry::Loadu(f) | InitEntry::Load(f) => f,
        }
    }
}

/// 「`loadu` の有無 × `load` の有無」の 4 通りから呼ぶ入口を 1 つ選ぶ純関数（判断はここだけ）。
/// 両方無いときの名札はテスト `kernel32_yields_entry_not_found` が固定する `"load"`。
/// `Loadu` を選んだときの `load` は捨てる。
fn choose_init_entry(loadu: Option<LoadFn>, load: Option<LoadFn>) -> Result<InitEntry, ProxyError> {
    match (loadu, load) {
        (Some(u), _) => Ok(InitEntry::Loadu(u)),
        (None, Some(l)) => Ok(InitEntry::Load(l)),
        (None, None) => Err(ProxyError::EntryNotFound("load")),
    }
}

/// SHIORI DLL の常設プロキシ。
///
/// 値があれば初期化の入口（`loadu`／`load`）は成功済み（load 済み）。
/// 後始末は [`ShioriByteProxy::close`] か Drop の 1 回だけ。
pub struct ShioriByteProxy {
    /// ロード元モジュールハンドル。後始末の `unload` の後に `FreeLibrary` する。
    module: HMODULE,
    /// 解決済み `unload` エントリ。後始末で呼ぶ。
    unload: UnloadFn,
    /// 解決済み `request` エントリ。[`ShioriByteProxy::request`] が呼ぶ。
    request: RequestFn,
}

impl ShioriByteProxy {
    /// `LoadLibraryExW` → 入口の解決と選択 → 入口別の符号化 → 初期化の入口の呼出まで行い、成功時のみ
    /// load 済みの `Self` を返す。
    ///
    /// - `dll_path`: **絶対パス**。依存の探し先を絞った `LoadLibraryExW` で開く（相対パスは `LoadLibraryFailed`）。
    /// - `load_dir`: 初期化の入口の同期引数（リソース根）。`loadu` へは UTF-8、`load` へは
    ///   **ANSI(CP_ACP)** で符号化して渡す（[`init_bytes`]）。
    ///
    /// 確立途中（解決／符号化／確保／入口の呼出）の失敗は内部で `FreeLibrary(module)` してから `Err` を
    /// 返す（半構築を残さない）。
    ///
    /// Preconditions: 呼出は helper UI スレッド（WndProc）上。
    pub fn load(dll_path: &Path, load_dir: &Path) -> Result<Self, ProxyError> {
        // --- 手順 1: LoadLibraryExW ---
        let wide = HSTRING::from(dll_path.as_os_str());
        let module = load_library_quiet(&wide).map_err(ProxyError::LoadLibraryFailed)?;

        // --- 手順 2: エクスポート解決（無装飾名）---
        // 初期化の入口を任意で引いてから `unload`・`request` を必須で引く。この順なので、何も持たない
        // DLL の最初の失敗は `EntryNotFound("load")` になる。
        // SAFETY(全体): `module` は直前にロードした有効ハンドル。各シンボルは DLL 側が
        // `#[unsafe(no_mangle)] pub extern "cdecl"` で公開する装飾なしの C 名（testdll と同形）。
        // GetProcAddress は未解決なら None を返す。
        let resolve = || -> Result<(InitEntry, UnloadFn, RequestFn), ProxyError> {
            // SAFETY: 解決した FARPROC を、DLL が公開する flat-C cdecl 署名（HGLOBAL/usize 幅）へ transmute
            // する。戻りは `u8` で受ける: cdecl の戻り値は EAX にあり、
            // Rust `bool`（pasta）を返す DLL は下位 1 バイト（AL）だけを書くので、下位 1 バイトだけを読む
            // `u8` が上位 24 bit の不定値を読まない最小の型。天井: C 製 DLL が Win32 `BOOL` として下位
            // バイト 0・上位 bit 非 0 の値（例 `0x100`）を返すと失敗と判定される（正典どおり TRUE(1)／
            // FALSE(0) を返す限り問題無い）。FARPROC は unsafe extern fn() の確定値。
            let as_load = |raw| unsafe { std::mem::transmute::<_, LoadFn>(raw) };
            let loadu = unsafe { GetProcAddress(module, s!("loadu")) }.map(as_load);
            let load = unsafe { GetProcAddress(module, s!("load")) }.map(as_load);
            let entry = choose_init_entry(loadu, load)?;

            let unload_raw = unsafe { GetProcAddress(module, s!("unload")) }
                .ok_or(ProxyError::EntryNotFound("unload"))?;
            let request_raw = unsafe { GetProcAddress(module, s!("request")) }
                .ok_or(ProxyError::EntryNotFound("request"))?;
            // SAFETY: 上と同じ根拠（`unload` の戻りも `u8`）。
            let unload: UnloadFn = unsafe { std::mem::transmute::<_, UnloadFn>(unload_raw) };
            let request: RequestFn = unsafe { std::mem::transmute::<_, RequestFn>(request_raw) };
            Ok((entry, unload, request))
        };

        let (entry, unload, request) = match resolve() {
            Ok(fns) => fns,
            Err(e) => {
                free_after_failed_load(module);
                return Err(e);
            }
        };

        // --- 手順 3〜5: 入口別の符号化 → GlobalAlloc → 初期化の入口の同期呼出 ---
        match Self::encode_alloc_and_load(entry, load_dir) {
            Ok(()) => Ok(Self {
                module,
                unload,
                request,
            }),
            Err(e) => {
                // 入力 HGLOBAL は入口(callee) へ move 済みなので、解放するのは module だけ。
                free_after_failed_load(module);
                Err(e)
            }
        }
    }

    /// 手順 3〜5: 入口別のバイト列（[`init_bytes`]）→ `GlobalAlloc(GMEM_FIXED)` へ書込 → 入口名の行 →
    /// 選んだ入口を同期で 1 回だけ呼ぶ。**入力 HGLOBAL は callee が解放する規約なので自ら解放しない**。
    /// 符号化・確保で失敗したときは入口を呼ばないので入口名の行も出ない（失敗の 1 行は `main.rs`）。
    fn encode_alloc_and_load(entry: InitEntry, load_dir: &Path) -> Result<(), ProxyError> {
        let bytes = init_bytes(&entry, load_dir)?;

        let hdir = global_alloc_copy(&bytes)?;
        let len = bytes.len();

        // 入口名の行は成功・失敗を問わず、呼ぶ直前に 1 回出す。
        crate::helper_log(&format!("{INIT_ENTRY_LOG_PREFIX}{}", entry.name()));
        // SAFETY: `hdir` は `len` バイトの有効 HGLOBAL（GMEM_FIXED ゆえハンドル＝先頭ポインタ）。
        // DLL(callee) は受領ハンドルを `GlobalFree` する規約（testdll の load が実演）なので、
        // ホストは以後 hdir に触れず解放もしない（二重解放になる）。所有権は入口へ move する。
        // 呼出は同期で 1 バイト整数を返す（DLL 内部スレッドに前提を置かない）。
        let ret = unsafe { entry.func()(hdir, len) };
        // 0 以外は成功（`== 1` とは判定しない）。`loadu` が 0 でも `load` へは落ちない。
        if ret != 0 {
            Ok(())
        } else {
            // SAFETY: スレッド局所のエラー値を読むだけ。返った直後、ほかの関数を呼ぶ前に読む
            // （後の呼び出しが上書きするため）。
            let last_error = unsafe { GetLastError() }.0;
            Err(ProxyError::LoadReturnedFalse { last_error })
        }
    }

    /// 保持済みの `request` エクスポートを HGLOBAL の非対称契約で呼ぶ。
    ///
    /// - 入力は `GlobalAlloc(GMEM_FIXED)` へ写して渡す（確保失敗は [`ProxyError::EncodingFailed`]）。
    ///   **入力 HGLOBAL は callee(DLL) が `GlobalFree` する**規約なので、ホストは以後触れず解放もしない。
    /// - `len` は in/out（入力長を渡し、応答長で上書きされる）。
    /// - 応答 HGLOBAL が null／無効なら [`ProxyError::RequestFailed`]。`GlobalSize` が 0 か `*len` より
    ///   小さければ [`ProxyError::ResponseLengthMismatch`]。そうでなければ `*len` バイトを `Vec<u8>` へ写す。
    ///   どちらでも `GlobalUnlock` → `GlobalFree(hres)`（応答はホストが解放する）。
    ///
    /// `unload` は呼ばない（呼ぶのは UNLOAD の道の `close` と、UNLOAD を経ない道の Drop）。
    ///
    /// Preconditions: proxy 確立済み。呼出は helper UI スレッド（WndProc）上。
    /// Postconditions: 応答 HGLOBAL は `GlobalFree` 済み（リーク・二重解放なし）。入力 HGLOBAL は callee 解放。
    pub fn request(&self, req: &[u8]) -> Result<Vec<u8>, ProxyError> {
        let hreq = global_alloc_copy(req)?;
        let mut len: usize = req.len();

        // SAFETY: `hreq` は `req.len()` バイトの有効 HGLOBAL（GMEM_FIXED ゆえハンドル＝先頭ポインタ）。
        // `&mut len` は有効な in/out ポインタ。DLL(callee) は受領した入力ハンドルを `GlobalFree` する規約
        // （testdll の request が実演）なので、ホストは以後 `hreq` に触れず解放もしない
        // （所有権は callee へ move・二重解放になる）。`self.request` は確立時に flat-C cdecl 署名
        // （RequestFn）へ解決済み。呼出は同期で応答 HGLOBAL を返す。
        let hres = unsafe { (self.request)(hreq, &mut len as *mut usize) };

        // 入力は callee へ move 済みなので hreq には触れない。
        if hres.is_invalid() {
            return Err(ProxyError::RequestFailed);
        }

        // `GlobalLock` で先頭の場所にし（動くメモリでも正しく読む）、確保の大きさを確かめてから写す。
        // 応答を確保する側の写し: testdll・loadu の `lib.rs` の `request`。
        // SAFETY: `hres` は DLL が `GlobalAlloc` で確保し所有権が caller へ移った有効ハンドル
        // （上で is_invalid() を確認済み）。失敗は null／0 で返る。
        let (ptr, allocated) = unsafe { (GlobalLock(hres) as *const u8, GlobalSize(hres)) };
        let result = if ptr.is_null() || allocated == 0 || allocated < len {
            // 錠が開かないときも読まずに解放し、種類は同じ Err で返す（呼び手は 500 を返すだけ）。
            crate::helper_log(&if ptr.is_null() {
                format!(
                    "[helper] SHIORI の応答を GlobalLock で開けない（読まずに解放）: declared={len}"
                )
            } else {
                format!(
                    "[helper] SHIORI の応答の長さが確保を超える（読まずに解放）: declared={len} allocated={allocated}"
                )
            });
            Err(ProxyError::ResponseLengthMismatch {
                declared: len,
                allocated,
            })
        } else {
            // SAFETY: `ptr` は `allocated` バイトの確保の先頭で、`len <= allocated` を確かめた。
            // ローカル Vec へのコピーは非重複（別確保）。
            Ok(unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
        };
        // SAFETY: `hres` は上の有効ハンドル。コピー後ゆえ以後参照しない。GlobalUnlock の Err は
        // ロックの数が 0 になった通知も含むので見ない。GlobalFree の Result は best-effort で無視
        // （解放漏れ・二重解放を起こさない）。
        unsafe {
            if !ptr.is_null() {
                let _ = GlobalUnlock(hres);
            }
            let _ = GlobalFree(Some(hres));
        }
        result
    }
}

impl ShioriByteProxy {
    /// 後始末: `unload()` → `FreeLibrary`。`unload` の偽は
    /// [`ProxyError::UnloadReturnedFalse`]、`FreeLibrary` の失敗は [`ProxyError::FreeLibraryFailed`]
    /// で返す（両方なら `FreeLibrary` の失敗を返し、`unload` の偽は `[helper]` の行で残す）。どちらでも DLL は解放を試みた後で、二度閉じない。
    pub fn close(self) -> Result<(), ProxyError> {
        // Drop に同じ後始末をさせない（二度閉じない）。中身は Copy の値だけなので漏れは無い。
        std::mem::ManuallyDrop::new(self).teardown()
    }

    fn teardown(&self) -> Result<(), ProxyError> {
        // SAFETY: `unload` は確立時に解決した有効な flat-C エントリ。引数なし cdecl。
        let unloaded = unsafe { (self.unload)() } != 0;
        // SAFETY: `module` は `load` でロードした有効ハンドル。呼ぶのは `close` か Drop の 1 回だけ。
        if let Err(e) = unsafe { FreeLibrary(self.module) } {
            // 返せる種類は 1 つなので、unload の偽は行で残す。
            if !unloaded {
                crate::helper_log("[helper] SHIORI の unload が偽を返した（FreeLibrary も失敗）");
            }
            return Err(ProxyError::FreeLibraryFailed(e));
        }
        if unloaded {
            Ok(())
        } else {
            Err(ProxyError::UnloadReturnedFalse)
        }
    }
}

impl Drop for ShioriByteProxy {
    /// UNLOAD を経ない道の後始末（`close` と同じ）。
    /// 失敗は `[helper]` の行で書く（エラーとして返す先は無い）。
    fn drop(&mut self) {
        if let Err(e) = self.teardown() {
            crate::helper_log(&format!("[helper] SHIORI の後始末に失敗（Drop）: {e:?}"));
        }
    }
}

/// 確立の途中の失敗の後で DLL を放す。放せなければ `[helper]` の行で書く。
fn free_after_failed_load(module: HMODULE) {
    // SAFETY: `module` は `load` でロードした有効ハンドルで、まだ誰にも move していない。
    if let Err(e) = unsafe { FreeLibrary(module) } {
        crate::helper_log(&format!(
            "[helper] 確立の途中の失敗の後で SHIORI を放せない: {e:?}"
        ));
    }
}

/// `bytes` を `GlobalAlloc(GMEM_FIXED)` で確保した HGLOBAL へコピーして返す（pasta と同じ方式）。
///
/// `GMEM_FIXED` ゆえハンドル＝先頭ポインタで直接書き込める。確保失敗（Err／null）→[`ProxyError::EncodingFailed`]。
/// 返した HGLOBAL の所有権は呼出側（→ `load` の callee へ move）にある。
/// テストでの写し: testdll・loadu の `lib.rs` のテストの `alloc_input`。
fn global_alloc_copy(bytes: &[u8]) -> Result<HGLOBAL, ProxyError> {
    // 0 バイトの確保も有効なので、空でも通す。
    // SAFETY: GlobalAlloc は失敗時 Err を返す（下で map）。GMEM_FIXED ゆえハンドル＝先頭ポインタ。
    let h =
        unsafe { GlobalAlloc(GMEM_FIXED, bytes.len()) }.map_err(|_| ProxyError::EncodingFailed)?;
    if h.is_invalid() {
        return Err(ProxyError::EncodingFailed);
    }
    if !bytes.is_empty() {
        // SAFETY: `h` は `bytes.len()` バイトの確保済み領域。GMEM_FIXED ゆえ `h.0` が先頭ポインタ。
        // src/dst は非重複（別確保）。
        unsafe {
            let dst = h.0 as *mut u8;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        }
    }
    Ok(h)
}

/// コードページ符号化の結果。
struct CodepageEncoded {
    /// 入口へ渡すバイト列（NUL 無し）。
    bytes: Vec<u8>,
    /// 往復で元に戻らない字が 1 つ以上あった（既定文字への置き換えも best-fit も含む）。
    lossy: bool,
}

/// `path` をコードページ `cp` のバイト列へ符号化し、同じ `cp` で UTF-16 へ戻して元と比べる。
/// 本番は `CP_ACP` で呼び、テストは 20127／65001 を渡す。
///
/// 変換は `WideCharToMultiByte` の 2 回呼び・フラグ 0・既定文字は OS 既定。
/// 検出はコードページの値で分岐しない（65001 の機械でも同じ手順）。往路が 0 以下→
/// [`ProxyError::EncodingFailed`]。戻す側が 0 以下（判定できない）→ `lossy = true` に寄せる。
/// 空パスは空・`lossy = false`。
fn encode_with_codepage(cp: u32, path: &Path) -> Result<CodepageEncoded, ProxyError> {
    use std::os::windows::ffi::OsStrExt;
    // OsStr → UTF-16（NUL 終端は付けない＝正味の文字列長を渡す）。
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() {
        return Ok(CodepageEncoded {
            bytes: Vec::new(),
            lossy: false,
        });
    }

    // SAFETY: `wide` は有効な UTF-16 スライス。出力 None＝長さ問い合わせモード。既定フラグ(0)。
    let needed = unsafe { WideCharToMultiByte(cp, 0, &wide, None, PCSTR::null(), None) };
    if needed <= 0 {
        return Err(ProxyError::EncodingFailed);
    }

    let mut buf = vec![0u8; needed as usize];
    // SAFETY: `buf` は `needed` バイトの可変領域（i8 として渡す）。`wide` は有効な UTF-16 スライス。
    // WideCharToMultiByte は `&mut [u8]` を i8 バッファとして受ける（windows 0.62.2 の Some(&mut buf)）。
    let written = unsafe { WideCharToMultiByte(cp, 0, &wide, Some(&mut buf), PCSTR::null(), None) };
    if written <= 0 {
        return Err(ProxyError::EncodingFailed);
    }
    buf.truncate(written as usize);

    // 往復比較: 同じ cp で UTF-16 へ戻し、元と一致しなければ表せない字あり。
    // SAFETY: `buf` は有効なバイト列。出力 None＝長さ問い合わせモード。既定フラグ(0)。
    let back_len = unsafe { MultiByteToWideChar(cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), &buf, None) };
    let lossy = if back_len <= 0 {
        true
    } else {
        let mut back = vec![0u16; back_len as usize];
        // SAFETY: `back` は `back_len` 要素の可変領域。`buf` は有効なバイト列。既定フラグ(0)。
        let got = unsafe {
            MultiByteToWideChar(cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), &buf, Some(&mut back))
        };
        got <= 0 || back[..got as usize] != wide[..]
    };
    Ok(CodepageEncoded { bytes: buf, lossy })
}

/// 入口名の行の固定語句（実機確認の grep が判定に使う）。後ろに `loadu`／`load` が付く。
const INIT_ENTRY_LOG_PREFIX: &str = "[helper] SHIORI 初期化の入口: ";
/// `load` の枝で表せない字があったときの警告の固定語句。後ろに元のパスが付く。
const LOSSY_PATH_WARN_PREFIX: &str = "[helper] 警告: load_dir に既定コードページで表せない字があり別の字へ置き換えた（loadu が無いため load へ渡す）: ";

/// 入口別のバイト列（呼出の前段・fn ポインタは呼ばない）。
///
/// - `Loadu`: `load_dir` の UTF-8 そのもの（標準ライブラリのみ・置き換え無し・NUL 無し）。UTF-8 に
///   できない（不正な UTF-16）なら [`ProxyError::EncodingFailed`]。表せない字の検出は行わない。
/// - `Load`: 既定コードページ（`encode_with_codepage(CP_ACP, ..)`）。表せない字が
///   あれば警告を 1 行（元のパス付き）出し、それでも同じバイト列を返す（確立は止めない）。
fn init_bytes(entry: &InitEntry, load_dir: &Path) -> Result<Vec<u8>, ProxyError> {
    match entry {
        InitEntry::Loadu(_) => load_dir
            .to_str()
            .map(|s| s.as_bytes().to_vec())
            .ok_or(ProxyError::EncodingFailed),
        InitEntry::Load(_) => {
            let e = encode_with_codepage(CP_ACP, load_dir)?;
            if e.lossy {
                crate::helper_log(&format!("{LOSSY_PATH_WARN_PREFIX}{}", load_dir.display()));
            }
            Ok(e.bytes)
        }
    }
}

#[cfg(test)]
#[path = "shiori_proxy_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shiori_proxy_loadu_tests.rs"]
mod loadu_tests;
