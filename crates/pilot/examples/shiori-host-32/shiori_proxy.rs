//! host-32 先進坑: **ShioriByteProxy**。
//!
//! `pasta.dll`（PE 0x014C / i686・cdecl flat-C）を `LoadLibraryW`＋`GetProcAddress` で動的
//! ロードし、`load`/`unload`/`request` を解決して駆動する**バイト proxy**。SHIORI3 ロジック
//! （OnBoot 組立・`Value:` parse）は一切持たず、所有権・charset 規約を守ってバイト列を FFI
//! 境界の向こうへ運ぶだけに徹する。
//!
//! ## 確定 ABI（pasta 実ソース `vendors/pasta/crates/pasta_shiori/src/windows.rs` で確認）
//! - `load(hdir: HGLOBAL, len: usize) -> bool`（`#[unsafe(no_mangle)] pub extern "C"`）
//! - `unload() -> bool`（同上）
//! - `request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（`len: &mut usize`＝ABI 同一）
//! - 返り値は Rust `bool`(1 byte) で Win32 BOOL(i32) ではない。
//!
//! ## HGLOBAL 規約（hglobal/mod.rs で確認）
//! - 確保は `GlobalAlloc(GMEM_FIXED=0, len)`。ハンドルは生ポインタとして直接使える
//!   （`h as *mut u8`。GlobalLock 不要）。
//! - **入力 HGLOBAL は DLL(callee) が解放**: `load`/`request` は受領ハンドルを
//!   `ShioriString::capture(..)` し Drop で `GlobalFree` する。
//!   ゆえにホストは入力 HGLOBAL を**二重解放してはならない**。
//! - **`request` 応答 HGLOBAL はホスト(caller) が解放**: DLL は `clone_from_str_nofree`
//!   （has_free=false）で返すため解放しない。ホストが `GlobalFree` する。
//!
//! ## charset 非対称（hglobal/mod.rs で確認）
//! - `load` の ghostdir は **ANSI(Shift_JIS / CP_ACP)**（pasta `to_ansi_str()`＝`CP_ACP`）。
//! - `request` は **UTF-8**（pasta `to_utf8_str()`）。ゆえに本 proxy も load 引数を CP_ACP へ
//!   エンコードして渡す（pasta の `WideCharToMultiByte(CP_ACP, ..)` と対称）。
//!
//! `unsafe` 境界は本コンポーネントに集約する。

use std::path::Path;

use windows::Win32::Foundation::{FreeLibrary, GlobalFree, HGLOBAL};
use windows::Win32::Globalization::{CP_ACP, WC_COMPOSITECHECK, WideCharToMultiByte};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Memory::{GLOBAL_ALLOC_FLAGS, GlobalAlloc};
use windows::core::{HSTRING, PCSTR};

/// `GlobalAlloc` の `GMEM_FIXED`（=0）。pasta も同値で確保する。
/// `GMEM_FIXED` ではハンドル＝先頭ポインタゆえ `h as *mut u8` で直接読み書きできる。
const GMEM_FIXED: GLOBAL_ALLOC_FLAGS = GLOBAL_ALLOC_FLAGS(0);

// flat-C cdecl シグネチャ（pasta windows.rs で確定）。
type LoadFn = unsafe extern "C" fn(hdir: HGLOBAL, len: usize) -> bool;
type UnloadFn = unsafe extern "C" fn() -> bool;
type RequestFn = unsafe extern "C" fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL;

/// ロード/解決の失敗種別。観測可能な形で返す（panic しない）。
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
    /// flat-C fn ポインタへ `transmute` する。失敗は `ProxyError` で返す。
    /// エントリが引けなかったら、読んだ DLL を `FreeLibrary` してから返す。
    pub fn load_dll(dll_path: &Path) -> Result<ShioriEntries, ProxyError> {
        let wide = HSTRING::from(dll_path.as_os_str());
        // SAFETY: `wide` は呼び出し中生存する有効な NUL 終端 UTF-16。
        let module = unsafe { LoadLibraryW(&wide) }.map_err(|_| ProxyError::LoadLibraryFailed)?;

        // SAFETY: `module` は直前にロードした有効ハンドル。各シンボルは pasta が
        // `#[unsafe(no_mangle)] pub extern "C"` で公開＝装飾なし C 名。
        let entry = |name: &'static str, cname: &[u8]| {
            unsafe { GetProcAddress(module, PCSTR(cname.as_ptr())) }.ok_or_else(|| {
                // SAFETY: `module` は上で読んだ有効ハンドルで、まだ誰にも渡していない。
                unsafe {
                    let _ = FreeLibrary(module);
                }
                ProxyError::EntryNotFound(name)
            })
        };
        let load = entry("load", b"load\0")?;
        let unload = entry("unload", b"unload\0")?;
        let request = entry("request", b"request\0")?;

        // SAFETY: pasta 実ソースで確定した flat-C cdecl 署名へ transmute する。
        let entries = ShioriEntries {
            module,
            load: unsafe { std::mem::transmute::<_, LoadFn>(load) },
            unload: unsafe { std::mem::transmute::<_, UnloadFn>(unload) },
            request: unsafe { std::mem::transmute::<_, RequestFn>(request) },
        };
        Ok(entries)
    }

    /// `ghostdir` を **ANSI(Shift_JIS / CP_ACP)** で HGLOBAL 化し `load(hdir, len)` を呼ぶ。
    /// 入力 HGLOBAL は **DLL(callee) が解放**するので、ホストは解放しない。
    pub fn shiori_load(e: &ShioriEntries, ghostdir: &Path) -> Result<(), ProxyError> {
        let ansi = ansi_encode(ghostdir)?;
        let h = global_alloc_copy(&ansi)?;
        let len = ansi.len();
        // SAFETY: `h` は `len` バイトの有効 HGLOBAL（GMEM_FIXED ゆえポインタ＝ハンドル）。
        // pasta は `ShioriString::capture(h, len)` で受領し Drop で `GlobalFree` する＝ホストは解放しない。
        let ok = unsafe { (e.load)(h, len) };
        if ok {
            Ok(())
        } else {
            Err(ProxyError::LoadFailed)
        }
    }

    /// `req`（**UTF-8** SHIORI3 ワイヤ）を HGLOBAL 化し `request(req, &mut len)` を呼ぶ。
    /// `len` は in/out: 呼び出し前に要求長を入れ、応答長で上書きされる（pasta `request_impl` が
    /// `*len` を入力長として読む）。入力 HGLOBAL は DLL が解放。**応答 HGLOBAL はホストが
    /// `GlobalFree`** する。
    pub fn shiori_request(e: &ShioriEntries, req: &[u8]) -> Result<Vec<u8>, ProxyError> {
        let hreq = global_alloc_copy(req)?;
        let mut len: usize = req.len();
        // SAFETY: `hreq` は `req.len()` バイトの有効 HGLOBAL。pasta は capture して Drop で解放
        // ＝ホストは入力を解放しない。応答 HGLOBAL の所有権はホストへ移る。
        let hres = unsafe { (e.request)(hreq, &mut len as *mut usize) };
        if hres.is_invalid() {
            return Err(ProxyError::RequestFailed);
        }
        // SAFETY: `hres` は pasta が `clone_from_slice_impl`(GMEM_FIXED) で確保した有効領域
        // （ポインタ＝ハンドル）、`len` はその実バイト長（pasta が書き戻す）。
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

    /// `unload()` を呼ぶ。DLL の解放は `ShioriEntries` が捨てられるときにする。
    pub fn shiori_unload(e: &ShioriEntries) {
        // SAFETY: `unload` は解決済み flat-C エントリ。
        // pasta の unload は内部エラーでも true を返す設計ゆえ戻り値は見ない。
        let _ = unsafe { (e.unload)() };
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

/// `bytes` を `GlobalAlloc(GMEM_FIXED)` で確保した HGLOBAL へコピーして返す（pasta と同方式）。
fn global_alloc_copy(bytes: &[u8]) -> Result<HGLOBAL, ProxyError> {
    // SAFETY: GlobalAlloc は失敗時 null（HGLOBAL(null)）を返す＝下で検査する。
    let h = unsafe { GlobalAlloc(GMEM_FIXED, bytes.len()) }
        .map_err(|_| ProxyError::GlobalAllocFailed)?;
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
/// `WideCharToMultiByte(CP_ACP, ..)`）。先進坑の ghostdir は ASCII パスで ANSI と UTF-8 が
/// 同じバイトになるが、正準は ANSI なので CP_ACP で往復させる（非 ASCII 対応の足場）。
fn ansi_encode(path: &Path) -> Result<Vec<u8>, ProxyError> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.is_empty() {
        return Ok(Vec::new());
    }
    // SAFETY: `wide` は有効な UTF-16 スライス。出力 null＝長さ問い合わせ。
    let needed =
        unsafe { WideCharToMultiByte(CP_ACP, WC_COMPOSITECHECK, &wide, None, PCSTR::null(), None) };
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

/// helper の `main()` から呼ぶ i686 セルフテスト。`load(ghostdir)` がクラッシュせず完了するかを見て、
/// 成否メッセージを返す。
pub fn selftest_load(ghostdir: &Path) -> Result<String, ProxyError> {
    let dll = ghostdir.join("pasta.dll");
    let entries = ShioriByteProxy::load_dll(&dll)?;
    ShioriByteProxy::shiori_load(&entries, ghostdir)?;
    ShioriByteProxy::shiori_unload(&entries);
    Ok(format!(
        "selftest_load OK: loaded {} and load(ghostdir={}) completed without crash",
        dll.display(),
        ghostdir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// i686 helper 内で pasta.dll をロードし 3 エントリを解決、`load(ghostdir)` がクラッシュせず
    /// 完了することを確かめる。
    ///
    /// 本物の 32bit `pasta.dll` を読むので x64 では走れない（i686 の組み立てだけで走る）。
    #[cfg(target_arch = "x86")]
    #[test]
    fn load_pasta_dll_and_call_load_without_crash() {
        let dir = crate::sample_support::emo2_ghost_master();
        let dll = dir.join("pasta.dll");
        let entries = ShioriByteProxy::load_dll(&dll).expect("load_dll should resolve 3 entries");
        ShioriByteProxy::shiori_load(&entries, &dir).expect("shiori_load should complete (true)");
        ShioriByteProxy::shiori_unload(&entries);
    }

    /// エントリが引けない DLL を読んだら、`load_dll` は解放してから `EntryNotFound("load")` を返す。
    /// `msimg32.dll` は System32（i686 では SysWOW64）にあり `load`／`unload`／`request` を出さず、
    /// テストの exe は読まない。前後で `GetModuleHandleW` が無いことで解放を確かめる。
    #[test]
    fn load_dll_frees_the_dll_when_an_entry_is_missing() {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        let name = HSTRING::from("msimg32.dll");
        // SAFETY: `name` は呼び出し中生存する NUL 終端 UTF-16。
        let loaded = || unsafe { GetModuleHandleW(&name) }.is_ok();
        assert!(!loaded(), "msimg32.dll must not be loaded before the test");
        let got = ShioriByteProxy::load_dll(Path::new("msimg32.dll"));
        assert_eq!(got.err(), Some(ProxyError::EntryNotFound("load")));
        assert!(
            !loaded(),
            "load_dll must free the DLL when an entry is missing"
        );
    }

    /// ANSI(CP_ACP) エンコードが ASCII パスで UTF-8 バイト等価になること。
    #[test]
    fn ansi_encode_ascii_is_byte_equivalent() {
        let p = Path::new("C:/ghost/master/");
        let ansi = ansi_encode(p).expect("ansi_encode");
        assert_eq!(ansi, p.to_string_lossy().as_bytes());
    }
}
