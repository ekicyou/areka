//! **ShioriByteProxy**（design.md §339-388・WS-A 新設モジュール）。
//!
//! SHIORI DLL の unsafe FFI（ロード・エクスポート解決・`load` 呼出・解放）を **一点集約**する
//! 常設プロキシ。SHIORI3/SHIORI4 ロジックは一切持たず、所有権・charset 規約を守って
//! 「意味を持たない生バイト列」を FFI 境界の向こうへ運ぶだけに徹する（要件 6.1）。
//!
//! ## 確立シーケンス（design §347）
//! 1. `LoadLibraryW(dll_path)`（呼出側が `load_dir\<shiori_name>` を組んだ **絶対パス**を渡す＝
//!    DLL 検索パス曖昧性の排除）。失敗→[`ProxyError::LoadLibraryFailed`]（R6.1）。
//! 2. `GetProcAddress` で `load`/`unload`/`request` の **3 エクスポートすべて**を解決し fn ポインタ
//!    保持（R4.2）。いずれか欠落→[`ProxyError::EntryNotFound`]（R6.2）。
//! 3. `load_dir` を **ANSI(CP_ACP)**（`WideCharToMultiByte`）で符号化（R4.4）。失敗→
//!    [`ProxyError::EncodingFailed`]。
//! 4. `GlobalAlloc(GMEM_FIXED)` バッファへ書き込み。確保失敗→[`ProxyError::EncodingFailed`]。
//! 5. `load(hdir, len)` **同期**呼出（R4.4）。戻り `false`→[`ProxyError::LoadReturnedFalse`]（R6.3）。
//!
//! ## 所有権規約（design §348・research §9.3・R4.5）
//! `load` へ渡した入力 HGLOBAL は **DLL(callee) が `GlobalFree` する**。ホスト（本モジュール）は
//! **自ら解放しない**（二重解放禁止）。確保したハンドルは `load` 呼出で callee へ move する。
//!
//! ## charset（design §347・research §9.3）
//! `load` の dir は **ANSI(CP_ACP)**（pasta `to_ansi_str()`＝`MultiByteToWideChar` と対称ゆえ本
//! モジュールは `WideCharToMultiByte(CP_ACP, ..)` で符号化）。`request` は UTF-8（下流・本仕様非呼出）。
//!
//! ## Drop teardown（design §358・R2.1/2.2/2.3）
//! `load` 成功済みインスタンスの Drop で best-effort courtesy `unload()` → `FreeLibrary`。結果は
//! 無視する（エラーとして扱わない）。**明示 teardown メソッドは公開しない**（Drop が唯一の経路・R2.1）。
//! 確立途中（手順 2〜5）の失敗は内部で `FreeLibrary` してから `Err` を返す（半構築を露出しない）。
//!
//! ## 観測契約（design §357・R4.7）
//! 「`load` の同期 bool 結果＋無クラッシュ」のみ。DLL 内部スレッド等に前提を置かない。
//!
//! ## unsafe 集約
//! SHIORI FFI の `unsafe` 境界は本モジュールへ集約し、各ブロックに Safety 根拠を文書化する
//! （steering 規約）。32bit の `usize`=32bit 可搬性に留意（R13.3・本モジュールでは `len` が `usize`
//! で自然に閉じる）。
//!
//! ## consume 点
//! 本モジュールは Task 5.1（proxy 本体）で新設され、WndProc からの結線（LOAD トリガ→proxy 確立→
//! `load`→ack 返送）は **Task 6** が行う。それまでは未使用ゆえ [`allow(dead_code)]`。

// Task 6 が WndProc（`main.rs`）から本モジュールを consume するまで、proxy 本体は未使用。
// それによる dead_code 警告を抑止する（結線時に消費される）。
#![allow(dead_code)]

use std::path::Path;

use windows::Win32::Foundation::{FreeLibrary, HGLOBAL, HMODULE};
use windows::Win32::Globalization::{CP_ACP, WideCharToMultiByte};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Memory::{GLOBAL_ALLOC_FLAGS, GlobalAlloc};
use windows::core::{HSTRING, PCSTR, s};

/// `GlobalAlloc` の `GMEM_FIXED`（=0）。pasta と同値で確保する（research §9.3）。
/// `GMEM_FIXED` ではハンドル＝先頭ポインタゆえ `h.0 as *mut u8` で直接書き込める。
const GMEM_FIXED: GLOBAL_ALLOC_FLAGS = GLOBAL_ALLOC_FLAGS(0);

/// flat-C cdecl 署名（design §351-355・research §9.2 でバイト正確に固定）。
///
/// `i686-pc-windows-msvc` では C ABI＝cdecl。pasta の `extern "C"` と ABI 同一。戻り `bool` は
/// **Rust bool 1 byte**（Win32 BOOL(i32) ではない）。
type LoadFn = unsafe extern "cdecl" fn(hdir: HGLOBAL, len: usize) -> bool;
/// `unload() -> bool`（引数なし・cdecl）。Drop の courtesy unload で呼ぶ。
type UnloadFn = unsafe extern "cdecl" fn() -> bool;
/// `request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（`len` は in/out）。
/// **本仕様では解決のみ・呼出しない**（呼出 API は下流 host32-request が追加する）。
type RequestFn = unsafe extern "cdecl" fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL;

/// SHIORI DLL 確立の失敗種別（design §366-371）。観測可能な形で返す（panic しない）。
#[derive(Debug)]
pub enum ProxyError {
    /// `LoadLibraryW` 失敗（DLL 不在・ロード失敗・ビットネス不一致など・R6.1）。
    LoadLibraryFailed(windows::core::Error),
    /// `GetProcAddress` が `load`/`unload`/`request` のいずれかを引けなかった（R6.2）。
    EntryNotFound(&'static str),
    /// `load_dir` の ANSI(CP_ACP) 符号化失敗、または `GlobalAlloc` 確保失敗。
    EncodingFailed,
    /// `load` が `false` を返した（クラッシュはせず失敗を返した・R6.3）。
    LoadReturnedFalse,
}

/// SHIORI DLL の常設プロキシ（design §339-388）。
///
/// `Ok` = 3 fn ポインタ常設保持＋`load`→true 済み（＝「load 済み」プロキシ）。`request` fn は保持
/// のみ（呼出 API は下流 host32-request が追加）。Drop が唯一の teardown 経路（明示メソッド非公開）。
pub struct ShioriByteProxy {
    /// ロード元モジュールハンドル。Drop の courtesy unload の後に `FreeLibrary` する。
    module: HMODULE,
    /// 解決済み `load` エントリ（確立時に呼出済み・保持は teardown 対称性のためではなく記録）。
    load: LoadFn,
    /// 解決済み `unload` エントリ。Drop の courtesy unload で呼ぶ（R2.2）。
    unload: UnloadFn,
    /// 解決済み `request` エントリ。**本仕様では保持のみ・呼出しない**（下流 host32-request が消費）。
    request: RequestFn,
}

impl ShioriByteProxy {
    /// `LoadLibraryW` → 3 解決 → ANSI 符号化 → `load` 呼出まで。成功時のみ `Self`（=load 済み）を返す
    /// （design §374-375）。
    ///
    /// - `dll_path`: **絶対パス**（呼出側 Task 6 が `load_dir.join(shiori_name)` を渡す前提）。本
    ///   モジュールは受け取った絶対パスを `LoadLibraryW` で開く＝DLL 検索パス曖昧性の排除（design §381）。
    /// - `load_dir`: `load` の同期引数（リソース根）。**ANSI(CP_ACP)** で符号化して渡す（R4.4）。
    ///
    /// 確立途中（3 解決／符号化／確保／load）の失敗は内部で `FreeLibrary(module)` してから `Err` を
    /// 返す（半構築を残さない・design §358「Err = HMODULE 解放済み・状態残さず」）。
    ///
    /// Preconditions: `dll_path` は絶対パス。呼出は helper UI スレッド（WndProc）上。
    pub fn load(dll_path: &Path, load_dir: &Path) -> Result<Self, ProxyError> {
        // --- 手順 1: LoadLibraryW（絶対パス・UTF-16 null 終端）---
        // HSTRING は NUL 終端の UTF-16 を保持する。失敗→LoadLibraryFailed（R6.1）。
        let wide = HSTRING::from(dll_path.as_os_str());
        // SAFETY: `wide` は呼出中生存する有効な NUL 終端 UTF-16 文字列。LoadLibraryW は失敗時に
        // Err を返す（下で map）。ここで module のライフサイクル所有が本関数へ確立する。
        let module = unsafe { LoadLibraryW(&wide) }.map_err(ProxyError::LoadLibraryFailed)?;

        // --- 手順 2: 3 エクスポート解決（無装飾名）---
        // 途中失敗は module を FreeLibrary してから Err を返すため、クロージャで解決→失敗時解放を集約。
        // SAFETY(全体): `module` は直前にロードした有効ハンドル。各シンボルは DLL 側が
        // `#[unsafe(no_mangle)] pub extern "cdecl"` で公開（research §9・testdll 同形）＝装飾なし C 名。
        // GetProcAddress は未解決時 None を返す（unwrap せず ok_or で観測エラー化）。
        let resolve = || -> Result<(LoadFn, UnloadFn, RequestFn), ProxyError> {
            let load_raw = unsafe { GetProcAddress(module, s!("load")) }
                .ok_or(ProxyError::EntryNotFound("load"))?;
            let unload_raw = unsafe { GetProcAddress(module, s!("unload")) }
                .ok_or(ProxyError::EntryNotFound("unload"))?;
            let request_raw = unsafe { GetProcAddress(module, s!("request")) }
                .ok_or(ProxyError::EntryNotFound("request"))?;

            // SAFETY: 解決した FARPROC を research §9 でバイト照合済みの flat-C cdecl 署名へ transmute
            // する。DLL 側実体（testdll／pasta）と ABI（cdecl・bool 1byte・HGLOBAL/usize 幅）が一致する
            // ことを実装前照合で固定済み（Task 1）。FARPROC は Option<unsafe extern fn()> ゆえ確定値。
            let load: LoadFn = unsafe { std::mem::transmute::<_, LoadFn>(load_raw) };
            let unload: UnloadFn = unsafe { std::mem::transmute::<_, UnloadFn>(unload_raw) };
            let request: RequestFn = unsafe { std::mem::transmute::<_, RequestFn>(request_raw) };
            Ok((load, unload, request))
        };

        let (load, unload, request) = match resolve() {
            Ok(fns) => fns,
            Err(e) => {
                // 半構築を残さない: module を解放してから Err（design §358）。
                // SAFETY: `module` は本関数でロードした有効ハンドルで、まだ誰にも move していない。
                // FreeLibrary の結果は best-effort で無視（確立失敗の後始末）。
                unsafe {
                    let _ = FreeLibrary(module);
                }
                return Err(e);
            }
        };

        // --- 手順 3〜5: ANSI 符号化 → GlobalAlloc → load 同期呼出 ---
        // これらの失敗も module を FreeLibrary してから Err を返す（半構築を残さない）。
        match Self::encode_alloc_and_load(load, load_dir) {
            Ok(()) => Ok(Self {
                module,
                load,
                unload,
                request,
            }),
            Err(e) => {
                // SAFETY: `module` は本関数でロードした有効ハンドルで、Ok を返すまで所有は本関数。
                // 入力 HGLOBAL は encode_alloc_and_load 内で load(callee) へ move 済み（自ら解放しない・
                // R4.5）。ここで解放するのは module のみ。結果は best-effort で無視。
                unsafe {
                    let _ = FreeLibrary(module);
                }
                Err(e)
            }
        }
    }

    /// 手順 3〜5: `load_dir` を ANSI(CP_ACP) 符号化 → `GlobalAlloc(GMEM_FIXED)` へ書込 → `load` 同期
    /// 呼出（design §347）。**入力 HGLOBAL は callee 解放規約ゆえ自ら解放しない**（R4.5）。
    fn encode_alloc_and_load(load: LoadFn, load_dir: &Path) -> Result<(), ProxyError> {
        // 手順 3: ANSI(CP_ACP) 符号化。失敗→EncodingFailed。
        let ansi = ansi_encode(load_dir)?;

        // 手順 4: GMEM_FIXED バッファ確保＋書込。確保失敗→EncodingFailed。
        let hdir = global_alloc_copy(&ansi)?;
        let len = ansi.len();

        // 手順 5: load(hdir, len) 同期呼出。
        // SAFETY: `hdir` は `len` バイトの有効 HGLOBAL（GMEM_FIXED ゆえハンドル＝先頭ポインタ）。
        // DLL(callee) は受領ハンドルを `GlobalFree` する規約（research §9.3・testdll の load が実演）
        // ＝ホストは以後 hdir に触れず・解放もしない（二重解放禁止・R4.5）。所有権は load へ move する。
        // 呼出は同期で bool を返す（DLL 内部スレッドに前提を置かない・R4.7）。
        let ok = unsafe { load(hdir, len) };
        if ok {
            Ok(())
        } else {
            Err(ProxyError::LoadReturnedFalse)
        }
    }
}

impl Drop for ShioriByteProxy {
    /// courtesy `unload()`（best-effort・結果無視）→ `FreeLibrary`（design §358・R2.1/2.2/2.3）。
    ///
    /// 明示 teardown メソッドを公開しないため、これが唯一の後始末経路。unload の bool 結果は
    /// 無視し（エラーとして扱わない・ハングはプロセス lifecycle=下流の領分）、続けて DLL を解放する。
    fn drop(&mut self) {
        // SAFETY: `unload` は確立時に解決した有効な flat-C エントリ。引数なし cdecl。結果 bool は
        // best-effort で無視する（R2.2・courtesy unload）。DLL 内部スレッドに前提を置かない（R4.7）。
        unsafe {
            let _ = (self.unload)();
        }
        // SAFETY: `module` は `load` でロードした有効ハンドル。多重 Drop は型（所有）が防ぐ。
        // FreeLibrary の結果は無視（best-effort teardown）。
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}

/// `bytes` を `GlobalAlloc(GMEM_FIXED)` で確保した HGLOBAL へコピーして返す（research §9.3・pasta 同方式）。
///
/// `GMEM_FIXED` ゆえハンドル＝先頭ポインタで直接書き込める。確保失敗（Err／null）→[`ProxyError::EncodingFailed`]。
/// 返した HGLOBAL の所有権は呼出側（→ `load` の callee へ move）にある。
fn global_alloc_copy(bytes: &[u8]) -> Result<HGLOBAL, ProxyError> {
    // 空バッファでも 0 バイト確保は有効（len==0 の load_dir は運用上ないが防御的に許容）。
    // SAFETY: GlobalAlloc は失敗時 Err を返す（下で map）。GMEM_FIXED ゆえハンドル＝先頭ポインタ。
    let h = unsafe { GlobalAlloc(GMEM_FIXED, bytes.len()) }.map_err(|_| ProxyError::EncodingFailed)?;
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

/// `path` を **ANSI(CP_ACP)** バイト列へ符号化する（design §347・research §9.3・pasta `to_ansi_str`
/// と対称の `WideCharToMultiByte(CP_ACP, ..)`）。
///
/// まず `OsStr` → UTF-16、次に CP_ACP へ。空パスは空バイト列を返す。失敗（長さ問い合わせ／変換が
/// 0 以下）→[`ProxyError::EncodingFailed`]。
fn ansi_encode(path: &Path) -> Result<Vec<u8>, ProxyError> {
    use std::os::windows::ffi::OsStrExt;
    // OsStr → UTF-16（NUL 終端は付けない＝正味の文字列長を CP_ACP へ渡す）。
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() {
        return Ok(Vec::new());
    }

    // 必要バイト数を問い合わせる（出力バッファ None＝長さ問い合わせ）。
    // SAFETY: `wide` は有効な UTF-16 スライス。出力 None＝長さ問い合わせモード。既定フラグ(0)。
    let needed = unsafe { WideCharToMultiByte(CP_ACP, 0, &wide, None, PCSTR::null(), None) };
    if needed <= 0 {
        return Err(ProxyError::EncodingFailed);
    }

    let mut buf = vec![0u8; needed as usize];
    // SAFETY: `buf` は `needed` バイトの可変領域（i8 として渡す）。`wide` は有効な UTF-16 スライス。
    // WideCharToMultiByte は `&mut [u8]` を i8 バッファとして受ける（windows 0.62.2 の Some(&mut buf)）。
    let written = unsafe { WideCharToMultiByte(CP_ACP, 0, &wide, Some(&mut buf), PCSTR::null(), None) };
    if written <= 0 {
        return Err(ProxyError::EncodingFailed);
    }
    buf.truncate(written as usize);
    Ok(buf)
}
