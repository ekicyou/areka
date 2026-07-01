//! host-32 先進坑: **ShioriByteProxy**（design.md §445–495）。
//!
//! `pasta.dll`（PE 0x014C / i686・cdecl flat-C）を `LoadLibraryW`＋`GetProcAddress` で動的
//! ロードし、`load`/`unload`/`request` を解決して駆動する**バイト proxy**。SHIORI3 ロジック
//! （OnBoot 組立・`Value:` parse）は一切持たず、所有権・charset 規約を守ってバイト列を FFI
//! 境界の向こうへ運ぶだけに徹する（requirements 3.1–3.4, 4.2 / design §445–495）。
//!
//! ## 確定 ABI（pasta 実ソース `vendors/pasta/crates/pasta_shiori/src/windows.rs` で確認）
//! - `load(hdir: HGLOBAL, len: usize) -> bool`（windows.rs:50・`#[unsafe(no_mangle)] pub extern "C"`）
//! - `unload() -> bool`（windows.rs:63・同上）
//! - `request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（windows.rs:76・`len: &mut usize`＝ABI 同一）
//! - 返り値は Rust `bool`(1 byte) で Win32 BOOL(i32) ではない（design §468）。
//!
//! ## HGLOBAL 規約（hglobal/mod.rs で確認）
//! - 確保は `GlobalAlloc(GMEM_FIXED=0, len)`。ハンドルは生ポインタとして直接使える
//!   （`h as *mut u8`・mod.rs:53–54。GlobalLock 不要）。
//! - **入力 HGLOBAL は DLL(callee) が解放**: `load`/`request` は受領ハンドルを
//!   `ShioriString::capture(..)` し Drop で `GlobalFree` する（mod.rs:31–47, windows.rs:145/157）。
//!   ゆえにホストは入力 HGLOBAL を**二重解放してはならない**。
//! - **`request` 応答 HGLOBAL はホスト(caller) が解放**: DLL は `clone_from_str_nofree`
//!   （has_free=false・mod.rs:81）で返すため解放しない。ホストが `GlobalFree` する。
//!
//! ## charset 非対称（design §492–493・hglobal/mod.rs:122/133 で確認）
//! - `load` の ghostdir は **ANSI(Shift_JIS / CP_ACP)**（pasta `to_ansi_str()`＝`CP_ACP`）。
//! - `request` は **UTF-8**（pasta `to_utf8_str()`）。ゆえに本 proxy も load 引数を CP_ACP へ
//!   エンコードして渡す（pasta の `WideCharToMultiByte(CP_ACP, ..)` と対称）。
//!
//! `unsafe` 境界は本コンポーネントに集約する（design §489）。

#![allow(dead_code)]

use std::path::Path;

use windows::Win32::Foundation::{FreeLibrary, GlobalFree, HGLOBAL};
use windows::Win32::Globalization::{CP_ACP, WC_COMPOSITECHECK, WideCharToMultiByte};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Memory::{GLOBAL_ALLOC_FLAGS, GlobalAlloc};
use windows::core::{HSTRING, PCSTR};

/// `GlobalAlloc` の `GMEM_FIXED`（=0）。pasta も同値で確保する（hglobal/mod.rs:13/53）。
/// `GMEM_FIXED` ではハンドル＝先頭ポインタゆえ `h as *mut u8` で直接読み書きできる。
const GMEM_FIXED: GLOBAL_ALLOC_FLAGS = GLOBAL_ALLOC_FLAGS(0);

// flat-C cdecl シグネチャ（design §469–471・pasta windows.rs:50/63/76 で確定）。
type LoadFn = unsafe extern "C" fn(hdir: HGLOBAL, len: usize) -> bool;
type UnloadFn = unsafe extern "C" fn() -> bool;
type RequestFn = unsafe extern "C" fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL;

/// ロード/解決の失敗種別（design §485・requirements 3.4）。観測可能な形で返す（panic しない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyError {
    /// `LoadLibraryW` 失敗（dll 不在・ビットネス不一致など）。
    LoadLibraryFailed,
    /// `GetProcAddress` が `load`/`unload`/`request` のいずれかを引けなかった。
    EntryNotFound(&'static str),
    /// `load(ghostdir)` が `false` を返した（クラッシュはせず失敗を返した）。
    LoadFailed,
    /// `request` が要求の確保に失敗、応答が null、または応答取得に失敗した。
    RequestFailed,
    /// ghostdir の ANSI(Shift_JIS) エンコードに失敗した。
    EncodeAnsiFailed,
    /// HGLOBAL 確保に失敗した（GlobalAlloc が null を返した）。
    GlobalAllocFailed,
}

/// 解決済みエントリ＋ロード元モジュールハンドル。`Drop` 時に `FreeLibrary` する。
pub struct ShioriEntries {
    module: windows::Win32::Foundation::HMODULE,
    load: LoadFn,
    unload: UnloadFn,
    request: RequestFn,
}

pub struct ShioriByteProxy;

impl ShioriByteProxy {
    /// `pasta.dll` を `LoadLibraryW` でロードし、3 エントリを `GetProcAddress` で解決して
    /// flat-C fn ポインタへ `transmute` する（design §476）。失敗は `ProxyError` で返す。
    pub fn load_dll(dll_path: &Path) -> Result<ShioriEntries, ProxyError> {
        let wide = HSTRING::from(dll_path.as_os_str());
        // SAFETY: `wide` は呼び出し中生存する有効な NUL 終端 UTF-16。
        let module = unsafe { LoadLibraryW(&wide) }.map_err(|_| ProxyError::LoadLibraryFailed)?;

        // SAFETY: `module` は直前にロードした有効ハンドル。各シンボルは pasta が
        // `#[unsafe(no_mangle)] pub extern "C"` で公開（windows.rs:50/63/76）＝装飾なし C 名。
        let load = unsafe { GetProcAddress(module, PCSTR(b"load\0".as_ptr())) }
            .ok_or(ProxyError::EntryNotFound("load"))?;
        let unload = unsafe { GetProcAddress(module, PCSTR(b"unload\0".as_ptr())) }
            .ok_or(ProxyError::EntryNotFound("unload"))?;
        let request = unsafe { GetProcAddress(module, PCSTR(b"request\0".as_ptr())) }
            .ok_or(ProxyError::EntryNotFound("request"))?;

        // SAFETY: pasta 実ソースで確定した flat-C cdecl 署名へ transmute する（design §469–471）。
        let entries = ShioriEntries {
            module,
            load: unsafe { std::mem::transmute::<_, LoadFn>(load) },
            unload: unsafe { std::mem::transmute::<_, UnloadFn>(unload) },
            request: unsafe { std::mem::transmute::<_, RequestFn>(request) },
        };
        Ok(entries)
    }

    /// `ghostdir` を **ANSI(Shift_JIS / CP_ACP)** で HGLOBAL 化し `load(hdir, len)` を呼ぶ
    /// （design §477・charset 非対称）。入力 HGLOBAL は **DLL(callee) が解放**するため
    /// ホストは解放しない（二重解放ガード＝確保したハンドルは callee へ move する）。
    pub fn shiori_load(e: &ShioriEntries, ghostdir: &Path) -> Result<(), ProxyError> {
        let ansi = ansi_encode(ghostdir)?;
        // 入力 HGLOBAL を確保し ANSI バイトを書き込む。所有権は load() の callee へ渡す。
        let h = global_alloc_copy(&ansi)?;
        let len = ansi.len();
        // SAFETY: `h` は `len` バイトの有効 HGLOBAL（GMEM_FIXED ゆえポインタ＝ハンドル）。
        // pasta は `ShioriString::capture(h, len)` で受領し Drop で `GlobalFree` する
        // （windows.rs:145・hglobal mod.rs:31–47）＝ホストは解放しない。
        let ok = unsafe { (e.load)(h, len) };
        if ok {
            Ok(())
        } else {
            Err(ProxyError::LoadFailed)
        }
    }

    /// `req`（**UTF-8** SHIORI3 ワイヤ）を HGLOBAL 化し `request(req, &mut len)` を呼ぶ
    /// （design §479–481）。`len` は in/out: 呼び出し前に要求長を入れ、応答長で上書きされる
    /// （pasta `request_impl` が `*len` を入力長として読む・windows.rs:118）。入力 HGLOBAL は
    /// DLL が解放。**応答 HGLOBAL はホストが `GlobalFree`** する（mod.rs:81 nofree）。
    pub fn shiori_request(e: &ShioriEntries, req: &[u8]) -> Result<Vec<u8>, ProxyError> {
        let hreq = global_alloc_copy(req)?;
        // in/out: 入力長を渡し、応答長で上書きされる。
        let mut len: usize = req.len();
        // SAFETY: `hreq` は `req.len()` バイトの有効 HGLOBAL。pasta は capture して Drop で解放
        // （windows.rs:157）＝ホストは入力を解放しない。応答 HGLOBAL の所有権はホストへ移る。
        let hres = unsafe { (e.request)(hreq, &mut len as *mut usize) };
        if hres.is_invalid() {
            return Err(ProxyError::RequestFailed);
        }
        // 応答 HGLOBAL（GMEM_FIXED ゆえポインタ＝ハンドル）から len バイトを Vec へコピーし、
        // 直ちに `GlobalFree` する（応答はホスト所有・mod.rs:81 nofree）。
        // SAFETY: `hres` は pasta が `clone_from_slice_impl`(GMEM_FIXED) で確保した有効領域、
        // `len` はその実バイト長（windows.rs:119 で書き戻される）。
        let out = unsafe {
            let p = hres.0 as *const u8;
            std::slice::from_raw_parts(p, len).to_vec()
        };
        // SAFETY: 応答 HGLOBAL の所有権はホスト＝ここで一度だけ解放する。
        unsafe {
            let _ = GlobalFree(Some(hres));
        }
        Ok(out)
    }

    /// `unload()` を呼び、続けて `FreeLibrary` する（design §482）。
    pub fn shiori_unload(e: &ShioriEntries) -> Result<(), ProxyError> {
        // SAFETY: `unload` は解決済み flat-C エントリ。
        let _ok = unsafe { (e.unload)() };
        // pasta の unload は内部エラーでも true を返す設計（windows.rs:96–105）。
        // 先進坑では unload 自体の bool 失敗は致命としない（ライブラリ解放を優先）。
        Ok(())
    }
}

impl Drop for ShioriEntries {
    fn drop(&mut self) {
        // SAFETY: `module` は `load_dll` でロードした有効ハンドル。多重 Drop は型が防ぐ。
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}

/// `bytes` を `GlobalAlloc(GMEM_FIXED)` で確保した HGLOBAL へコピーして返す（pasta と同方式・
/// mod.rs:50–58）。`GMEM_FIXED` ゆえハンドル＝先頭ポインタで直接書き込める。
fn global_alloc_copy(bytes: &[u8]) -> Result<HGLOBAL, ProxyError> {
    // SAFETY: GlobalAlloc は失敗時 null（HGLOBAL(null)）を返す＝下で検査する。
    let h = unsafe { GlobalAlloc(GMEM_FIXED, bytes.len()) }.map_err(|_| ProxyError::GlobalAllocFailed)?;
    if h.is_invalid() {
        return Err(ProxyError::GlobalAllocFailed);
    }
    // SAFETY: `h` は `bytes.len()` バイトの確保済み領域。GMEM_FIXED ゆえ `h.0` が先頭ポインタ。
    unsafe {
        let dst = h.0 as *mut u8;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
    }
    Ok(h)
}

/// `path` を **ANSI(Shift_JIS / CP_ACP)** バイト列へエンコードする（pasta `to_ansi_str` の逆＝
/// `WideCharToMultiByte(CP_ACP, ..)`・hglobal/windows_api.rs:133 と対称）。先進坑の ghostdir は
/// ASCII パスゆえ ANSI≡UTF-8 バイト等価だが、正準は ANSI ゆえ CP_ACP で確実に往復させる
/// （design §493・本坑/非 ASCII 対応の足場）。
fn ansi_encode(path: &Path) -> Result<Vec<u8>, ProxyError> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() {
        return Ok(Vec::new());
    }
    // 必要バイト数を問い合わせる。
    // SAFETY: `wide` は有効な UTF-16 スライス。出力 null＝長さ問い合わせ。
    let needed = unsafe {
        WideCharToMultiByte(CP_ACP, WC_COMPOSITECHECK, &wide, None, PCSTR::null(), None)
    };
    if needed <= 0 {
        return Err(ProxyError::EncodeAnsiFailed);
    }
    let mut buf = vec![0u8; needed as usize];
    // SAFETY: `buf` は `needed` バイトの可変領域。`wide` は有効。
    let written = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            WC_COMPOSITECHECK,
            &wide,
            Some(&mut buf),
            PCSTR::null(),
            None,
        )
    };
    if written <= 0 {
        return Err(ProxyError::EncodeAnsiFailed);
    }
    buf.truncate(written as usize);
    Ok(buf)
}

/// helper の `main()` から呼べる i686 セルフテスト観測（go 基準(1) precursor・requirements 3.3）。
///
/// `cargo test` 経由の観測は同 example の `ipc.rs` `#[cfg(test)]` が i686 でビルド不能
/// （`usize >> 32` overflow・本タスク境界外＝ipc.rs は改変不可）なため、helper バイナリの
/// 実行時セルフテストとしても `load(ghostdir)` 無 crash 完了を観測できるようにする。
/// 戻り値: 成否メッセージ（親が標準出力で観測）。
pub fn selftest_load(ghostdir: &Path) -> Result<String, ProxyError> {
    let dll = ghostdir.join("pasta.dll");
    let entries = ShioriByteProxy::load_dll(&dll)?;
    ShioriByteProxy::shiori_load(&entries, ghostdir)?;
    let _ = ShioriByteProxy::shiori_unload(&entries);
    Ok(format!(
        "selftest_load OK: loaded {} and load(ghostdir={}) completed without crash",
        dll.display(),
        ghostdir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ghostdir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("shiori-host-32")
            .join("fixtures")
            .join("emo2")
            .join("ghost")
            .join("master")
    }

    /// go 基準(1) precursor（requirements 3.1–3.3）: i686 helper 内で pasta.dll をロードし
    /// 3 エントリを解決、`load(ghostdir)` がクラッシュせず完了することを観測する。
    /// 完了後 `unload` で pasta の actor スレッド／ライブラリを後始末する。
    ///
    /// 注意: 同 example の ipc.rs `#[cfg(test)]` の i686 overflow により `cargo test` 自体が
    /// ビルド不能なため、実走観測は helper `main()` の `--selftest-load`（selftest_load）でも行う。
    #[test]
    fn load_pasta_dll_and_call_load_without_crash() {
        let dir = ghostdir();
        let dll = dir.join("pasta.dll");
        let entries = ShioriByteProxy::load_dll(&dll).expect("load_dll should resolve 3 entries");
        ShioriByteProxy::shiori_load(&entries, &dir).expect("shiori_load should complete (true)");
        ShioriByteProxy::shiori_unload(&entries).expect("shiori_unload");
    }

    /// ANSI(CP_ACP) エンコードが ASCII パスで UTF-8 バイト等価になること（design §493）。
    #[test]
    fn ansi_encode_ascii_is_byte_equivalent() {
        let p = Path::new("C:/ghost/master/");
        let ansi = ansi_encode(p).expect("ansi_encode");
        assert_eq!(ansi, p.to_string_lossy().as_bytes());
    }
}
