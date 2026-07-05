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

use windows::Win32::Foundation::{FreeLibrary, GlobalFree, HGLOBAL, HMODULE};
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
    /// `request` が null／無効 HGLOBAL を返した（応答確保失敗等・クラッシュせず失敗を返した）。
    /// silent failure を避けるため他 variant と区別可能な形で返す（R3.2〜3.4 request 失敗面）。
    RequestFailed,
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

    /// 保持済み `request` エクスポートを HGLOBAL 非対称契約で実呼出する（design §435・§HGLOBAL 所有権契約）。
    ///
    /// 手順（design §435）:
    /// 1. `global_alloc_copy(req)` で入力を `GlobalAlloc(GMEM_FIXED)` HGLOBAL 化（`hreq`）。確保失敗は
    ///    [`ProxyError::EncodingFailed`] を伝播（`global_alloc_copy` が返す）。
    /// 2. `len` を in/out で用意（入力長を渡し、応答長で上書きされる）。
    /// 3. `request(hreq, &mut len)` を呼出。**入力 HGLOBAL は callee(DLL) が `GlobalFree` する**規約ゆえ
    ///    ホストは `hreq` に以後触れず・自ら解放もしない（二重解放禁止・R3.3）。所有権は callee へ move。
    /// 4. 応答 HGLOBAL が null／無効なら [`ProxyError::RequestFailed`]（silent failure を避け区別可能に返す）。
    /// 5. 応答 HGLOBAL から `*len` バイトを自前 `Vec<u8>` へコピー → `GlobalFree(hres)`（caller-free・R3.4）
    ///    → bytes を返す。
    ///
    /// `len` は in/out で 32bit `usize` として自然に閉じる（R7.6・i686 で ABI 一致）。`unload` は呼ばない
    /// （Drop courtesy のみ・R3.7）。unsafe FFI は本モジュールへ一点集約する。
    ///
    /// Preconditions: proxy 確立済み（`load`→true）。呼出は helper UI スレッド（WndProc）上。
    /// Postconditions: 応答 HGLOBAL は `GlobalFree` 済み（リーク・二重解放なし）。入力 HGLOBAL は callee 解放。
    pub fn request(&self, req: &[u8]) -> Result<Vec<u8>, ProxyError> {
        // 手順 1: 入力を GMEM_FIXED HGLOBAL 化。確保失敗→EncodingFailed を伝播。
        let hreq = global_alloc_copy(req)?;
        // 手順 2: len は in/out。入力長を渡し、応答長で上書きされる。
        let mut len: usize = req.len();

        // 手順 3: request(hreq, &mut len) 同期呼出。
        // SAFETY: `hreq` は `req.len()` バイトの有効 HGLOBAL（GMEM_FIXED ゆえハンドル＝先頭ポインタ）。
        // `&mut len` は有効な in/out ポインタ。DLL(callee) は受領した入力ハンドルを `GlobalFree` する規約
        // （research §9.3・testdll の request が実演）ゆえ、ホストは以後 `hreq` に触れず・解放もしない
        // （所有権は callee へ move・二重解放禁止・R3.3）。`self.request` は確立時に research §9 で
        // バイト照合済みの flat-C cdecl 署名（RequestFn）へ解決済み。呼出は同期で応答 HGLOBAL を返す。
        let hres = unsafe { (self.request)(hreq, &mut len as *mut usize) };

        // 手順 4: null／無効応答は RequestFailed（入力は既に callee へ move 済み・hreq には触れない）。
        if hres.is_invalid() {
            return Err(ProxyError::RequestFailed);
        }

        // 手順 5: 応答 HGLOBAL から len バイトをコピー → GlobalFree（caller-free・R3.4）。
        // SAFETY: `hres` は DLL が `GlobalAlloc(GMEM_FIXED)` で確保した `len` バイトの有効領域
        // （GMEM_FIXED ゆえ `hres.0` が先頭ポインタ）。上で is_invalid() を確認済み。ローカル Vec への
        // コピーは非重複（別確保）。
        let bytes = unsafe { std::slice::from_raw_parts(hres.0 as *const u8, len).to_vec() };
        // 応答 HGLOBAL は caller(host) が解放する規約（R3.4）。best-effort で解放する。
        // SAFETY: `hres` は callee(DLL) が alloc し所有権は caller へ移転済みの有効ハンドル。コピー後ゆえ
        // 以後参照しない。GlobalFree の Result は best-effort で無視（解放漏れ・二重解放を起こさない）。
        unsafe {
            let _ = GlobalFree(Some(hres));
        }
        Ok(bytes)
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

#[cfg(test)]
mod tests {
    //! Task 5.2: プロキシの i686 単体テスト（design §387 Validation・Testing Strategy Unit #6・
    //! 確定設計判断 (d)）。3 本立て:
    //! 1. 純関数部（ANSI(CP_ACP) 符号化）の決定的検証。
    //! 2. `kernel32.dll` への load で `EntryNotFound` を決定的に証明（エクスポート欠落態様・R6.2）。
    //! 3. testdll を直接 `load`→`drop` し、courtesy unload の実呼出（観測マーカー）と無 panic を確認
    //!    （R2.2・E2E では Drop 経路が実行されない補完）。

    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    /// testdll を実 `load`／`drop` するテスト同士の直列化ガード。
    ///
    /// `unload` は fixture で `HOST32_TESTDLL_UNLOAD_MARKER`（プロセス global env）を読み観測マーカーを
    /// 書く。`testdll_drop_invokes_courtesy_unload` はこの env を set したうえで「Drop 前は marker 未作成」を
    /// assert するため、別の testdll ロードテスト（`testdll_request_roundtrip_get_and_notify`）が並行して
    /// Drop→unload を走らせると env を読んで marker を先行作成し assert を破る。testdll をロードする全テスト
    /// を本 mutex で直列化し、この env レースを排除する（Mutex 汚染は無視して継続）。
    static TESTDLL_SERIAL: Mutex<()> = Mutex::new(());

    // ---------------------------------------------------------------------
    // 1. 純関数部の検証: ANSI(CP_ACP) 符号化
    // ---------------------------------------------------------------------

    /// ASCII パスは CP_ACP でも同一バイト列に符号化される（NUL 終端は付けない＝正味長）。
    /// 環境依存の少ない ASCII で決定的に検証する。
    #[test]
    fn ansi_encode_ascii_is_identity_no_nul() {
        let p = Path::new(r"C:\ghost\master");
        let bytes = ansi_encode(p).expect("ASCII path must encode");
        // ASCII は CP_ACP でそのままの 1 バイト列。NUL 終端規約: ansi_encode は終端を付けない。
        assert_eq!(bytes, br"C:\ghost\master".to_vec());
        // 末尾に NUL が付いていないこと（正味長）。
        assert_ne!(bytes.last(), Some(&0u8));
        assert_eq!(bytes.len(), r"C:\ghost\master".len());
    }

    /// 空パスは空バイト列（防御的許容・確保 0 バイトも有効）。
    #[test]
    fn ansi_encode_empty_is_empty() {
        let bytes = ansi_encode(Path::new("")).expect("empty path encodes to empty");
        assert!(bytes.is_empty());
    }

    /// 日本語混じりは CP_ACP(=日本語ロケールで Shift_JIS)の複数バイトへ符号化され、ASCII 部と
    /// 混在する。バイト内容はロケール依存ゆえ「非空・ASCII 区切り '\\' を含む・元 UTF-8 と異なる」
    /// のみ決定的に確認する（環境依存の強い部分は緩く・純関数が panic せず往復する証拠）。
    #[test]
    fn ansi_encode_mixed_japanese_is_multibyte() {
        let p = Path::new(r"C:\ゴースト");
        let bytes = ansi_encode(p).expect("mixed path must encode without panic");
        assert!(!bytes.is_empty());
        // ASCII 前置部（`C:\`）はそのまま先頭に載る。
        assert_eq!(&bytes[..3], br"C:\");
        // CP_ACP は UTF-8 と異なる符号化ゆえバイト列は元 UTF-8 と一致しない（SJIS 等）。
        assert_ne!(bytes, r"C:\ゴースト".as_bytes().to_vec());
    }

    // ---------------------------------------------------------------------
    // 2. kernel32.dll で EntryNotFound（R6.2・設計判断 (d)）
    // ---------------------------------------------------------------------

    /// `kernel32.dll` は必ず存在し LoadLibraryW は成功するが、`load`/`unload`/`request` の
    /// SHIORI エクスポートを持たない。ゆえに `ShioriByteProxy::load` は解決段で
    /// `ProxyError::EntryNotFound` を**決定的**に返す（エクスポート欠落態様の証明・R6.2）。
    /// 半構築を残さないため内部で FreeLibrary 済み（Err = 状態残さず）。
    #[test]
    fn kernel32_yields_entry_not_found() {
        let dll = Path::new(r"C:\Windows\System32\kernel32.dll");
        let load_dir = std::env::temp_dir(); // 実在 dir（符号化・確保段には到達しない）。
        let result = ShioriByteProxy::load(dll, &load_dir);
        match result {
            Err(ProxyError::EntryNotFound(sym)) => {
                // 最初に解決を試みるのは "load"（3 解決の先頭）。
                assert_eq!(sym, "load", "kernel32 は SHIORI load を持たない");
            }
            // Ok は kernel32 が SHIORI エクスポートを持たない以上あり得ない。他 ProxyError も不可。
            // ShioriByteProxy は Debug 非実装ゆえ Result 全体は整形せず、Err 変種のみ整形する。
            Err(e) => panic!("expected EntryNotFound(\"load\"), got other error {e:?}"),
            Ok(_) => panic!("expected EntryNotFound(\"load\"), but load unexpectedly succeeded"),
        }
    }

    // ---------------------------------------------------------------------
    // 3. testdll 直接 load→drop で courtesy unload 実証（R2.2・E2E 補完）
    // ---------------------------------------------------------------------

    /// testdll の `shiori.dll` を解決する。
    /// 優先順: env `HOST32_TESTDLL_DLL` → `CARGO_MANIFEST_DIR` から target 探索。
    /// **silent skip 禁止**: 見つからなければ明確に panic（設計判断 (d)）。
    fn resolve_testdll() -> PathBuf {
        if let Ok(p) = std::env::var("HOST32_TESTDLL_DLL") {
            let path = PathBuf::from(p);
            assert!(
                path.is_file(),
                "HOST32_TESTDLL_DLL={} が実ファイルでない",
                path.display()
            );
            return path;
        }
        // crates/shiori-host32-helper から見た workspace target。
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target_root = manifest.join("../../target/i686-pc-windows-msvc");
        for profile in ["debug", "release"] {
            let candidate = target_root.join(profile).join("shiori.dll");
            if candidate.is_file() {
                return candidate;
            }
        }
        panic!(
            "testdll shiori.dll not found. まず PowerShell で \
             `cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc` を実行するか、\
             env HOST32_TESTDLL_DLL に絶対パスを設定すること（silent skip 禁止・設計判断 (d)）。\
             探索した base: {}",
            target_root.display()
        );
    }

    /// testdll を直接 `load`→`drop` し、Drop の courtesy `unload` が実呼出されること
    /// （unload 観測マーカーのファイル作成）と、Drop 全体が無 panic で完了することを確認する
    /// （R2.2 の受入証拠・E2E は helper をプロセスごと落とすため Drop が走らない、その補完）。
    ///
    /// env `HOST32_TESTDLL_UNLOAD_MARKER` はプロセス global ゆえ、testdll を load する他テストと
    /// `TESTDLL_SERIAL` で直列化して競合を避ける（本テストのみが marker env を set/remove する）。
    #[test]
    fn testdll_drop_invokes_courtesy_unload() {
        // testdll ロードテスト同士を直列化（marker env レース排除）。汚染ロックは無視して継続。
        let _serial = TESTDLL_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let src_dll = resolve_testdll();

        // 一時 dir を load_dir とし、そこへ shiori.dll をコピー（絶対 dll_path＝load_dir\shiori.dll・
        // cwd=load_dir 慣習の縮図）。テスト固有名で衝突を避ける。
        let unique = format!(
            "host32_proxy_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let load_dir = std::env::temp_dir().join(&unique);
        std::fs::create_dir_all(&load_dir).expect("create temp load_dir");
        let dll_path = load_dir.join("shiori.dll");
        std::fs::copy(&src_dll, &dll_path).expect("copy shiori.dll into load_dir");

        // unload 観測マーカーの一時パス（load 前に未作成であること）。
        let marker = load_dir.join("unload.marker");
        assert!(!marker.exists(), "marker は load 前に未作成");

        // SAFETY: edition 2024 では set_var は unsafe（プロセス global 変更）。本テストは testdll を
        // load する唯一のテストゆえ他テストと競合しない。set は load 前に行う。
        unsafe {
            std::env::set_var("HOST32_TESTDLL_UNLOAD_MARKER", &marker);
        }

        // load 成功（testdll の load は入力 HGLOBAL を GlobalFree し true 返し）。
        let proxy = ShioriByteProxy::load(&dll_path, &load_dir)
            .expect("testdll load must succeed");

        // まだ Drop していないのでマーカーは未作成。
        assert!(!marker.exists(), "unload はまだ呼ばれていない");

        // Drop → courtesy unload 実呼出 → testdll が marker ファイルを作成。無 panic で完了。
        drop(proxy);

        assert!(
            marker.exists(),
            "Drop の courtesy unload が呼ばれ marker が作成されるはず（R2.2）"
        );
        let content = std::fs::read(&marker).expect("read marker");
        assert_eq!(content, b"unloaded", "testdll unload が書くマーカー内容");

        // 後始末（best-effort）: env・一時 dir を掃除。
        // SAFETY: set_var と同じく unsafe。以後 testdll を load するテストは無い。
        unsafe {
            std::env::remove_var("HOST32_TESTDLL_UNLOAD_MARKER");
        }
        let _ = std::fs::remove_dir_all(&load_dir);
    }

    // ---------------------------------------------------------------------
    // 4. testdll 越し request loopback（R3.2/3.3/3.4/3.6・Task 4.1）
    // ---------------------------------------------------------------------

    /// testdll を直接 `load` した proxy に対し `request` を実呼出し、固定 SHIORI/3.0 応答が
    /// 往復することを確認する（design §435 の HGLOBAL 非対称契約）。
    ///
    /// - GET(`OnTestValue`) → 200 OK＋固定 `Value`。
    /// - NOTIFY(`OnTestNotify`) → 204 No Content。
    ///
    /// helper クレートには codec が無いため request バイト列は手書きする。無 panic 完了が
    /// 「入力 callee-free／応答 caller-free」（二重解放なし）の観測証拠。`HOST32_TESTDLL_UNLOAD_MARKER`
    /// は設定しない（プロセス global ゆえ `testdll_drop_invokes_courtesy_unload` が唯一の所有者）。
    #[test]
    fn testdll_request_roundtrip_get_and_notify() {
        // testdll ロードテスト同士を直列化（drop テストの marker env レース排除）。
        let _serial = TESTDLL_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let src_dll = resolve_testdll();

        // 一意な一時 load_dir へ shiori.dll をコピー（絶対 dll_path＝load_dir\shiori.dll）。
        let unique = format!(
            "host32_proxy_request_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let load_dir = std::env::temp_dir().join(&unique);
        std::fs::create_dir_all(&load_dir).expect("create temp load_dir");
        let dll_path = load_dir.join("shiori.dll");
        std::fs::copy(&src_dll, &dll_path).expect("copy shiori.dll into load_dir");

        // load 成功（testdll の load は入力 HGLOBAL を callee-free し true 返し）。
        let proxy =
            ShioriByteProxy::load(&dll_path, &load_dir).expect("testdll load must succeed");

        // GET(OnTestValue) → 200 OK＋固定 Value。
        let get_req =
            b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
        let get_resp = proxy
            .request(get_req)
            .expect("GET request must succeed via testdll");
        let get_text = String::from_utf8(get_resp).expect("GET response must be valid UTF-8");
        assert!(
            get_text.contains("SHIORI/3.0 200 OK"),
            "expected 200 OK, got: {get_text:?}"
        );
        assert!(
            get_text.contains("Value: \\0\\s[0]host32 request roundtrip ok\\e"),
            "expected fixed Value line, got: {get_text:?}"
        );

        // NOTIFY(OnTestNotify) → 204 No Content。
        let notify_req =
            b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestNotify\r\n\r\n";
        let notify_resp = proxy
            .request(notify_req)
            .expect("NOTIFY request must succeed via testdll");
        let notify_text =
            String::from_utf8(notify_resp).expect("NOTIFY response must be valid UTF-8");
        assert!(
            notify_text.contains("SHIORI/3.0 204 No Content"),
            "expected 204 No Content, got: {notify_text:?}"
        );

        // Drop で courtesy unload→FreeLibrary。往復も Drop も無 panic＝二重解放なし（R3.3/3.4）。
        drop(proxy);

        // 後始末（best-effort）: 一時 dir を掃除。
        let _ = std::fs::remove_dir_all(&load_dir);
    }
}
