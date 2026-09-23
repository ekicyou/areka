//! 2 本目の最小 SHIORI DLL fixture（出力名 `shiori_loadu.dll`）。
//!
//! `loadu`／`load`／`unload`／`request` の 4 つを flat-C で公開し、初期化の入口のうち
//! **どちらが呼ばれ、何を受け取ったか**をファイルへ記録する（areka-P0-shiori-loadu 要件 6.1）。
//! 既存 fixture `shiori-host32-testdll`（`shiori.dll`・`load` のみ）は無改変で残す（6.2）。
//!
//! 規約:
//! - 戻りは Win32 `BOOL` と同じ 4 バイト `i32`（既存 fixture の Rust `bool` 1 バイトと対になる）。
//! - **入力 HGLOBAL は callee(DLL) が `GlobalFree` する**（既存 fixture・pasta と同じ）。
//! - 応答 HGLOBAL は `GlobalAlloc(GMEM_FIXED)` で新しく確保し、解放は caller(host)。
//!
//! テスト専用の注入 env（本番の helper は読まない・9.2）:
//! - `HOST32_TESTDLL_LOADU_RECORD`: 記録ファイルのパス。未設定なら書かない。
//!   1 呼出 1 行を追記: `<入口名>\t<受け取ったバイト列の小文字 16 進>\n`。
//! - `HOST32_TESTDLL_LOADU_FAIL`: `"1"` のとき `loadu` が `0` を返す。

use std::io::Write;

use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows::Win32::System::Memory::{GLOBAL_ALLOC_FLAGS, GlobalAlloc};

/// `GlobalAlloc` の `GMEM_FIXED`（=0）。ハンドル＝先頭ポインタゆえ `h.0 as *mut u8` で直接書ける。
const GMEM_FIXED: GLOBAL_ALLOC_FLAGS = GLOBAL_ALLOC_FLAGS(0);

/// 記録ファイルのパスを指す env（テスト専用）。
const ENV_RECORD: &str = "HOST32_TESTDLL_LOADU_RECORD";
/// `loadu` の偽返却を注入する env（テスト専用・`"1"` で有効）。
const ENV_FAIL: &str = "HOST32_TESTDLL_LOADU_FAIL";

/// `request` への固定応答（解決できることだけが要件・往復は対象外）。
const RESP_400: &[u8] = b"SHIORI/3.0 400 Bad Request\r\n\r\n";

/// 入力 HGLOBAL の `len` バイトをコピーしてから callee 解放し、記録を 1 行追記する。
///
/// # Safety
/// `hdir` は呼出側が `GlobalAlloc(GMEM_FIXED)` した `len` バイト以上の有効ハンドル
/// （または null）。所有権は本関数へ移転する。
unsafe fn take_and_record(entry: &str, hdir: HGLOBAL, len: usize) {
    let received: Vec<u8> = if hdir.0.is_null() || len == 0 {
        Vec::new()
    } else {
        // SAFETY: 呼出側契約により hdir は len バイトの有効領域（GMEM_FIXED ゆえ hdir.0 が先頭）。
        unsafe { std::slice::from_raw_parts(hdir.0 as *const u8, len).to_vec() }
    };
    if !hdir.0.is_null() {
        // SAFETY: コピー後に callee 解放（所有権は本関数へ移転済み）。best-effort。
        unsafe {
            let _ = GlobalFree(Some(hdir));
        }
    }
    let Ok(path) = std::env::var(ENV_RECORD) else {
        return;
    };
    let hex: String = received.iter().map(|b| format!("{b:02x}")).collect();
    // 書けなくても DLL は落ちない（記録はテストの観測用）。
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{entry}\t{hex}");
    }
}

/// SHIORI loadu: UTF-8 のパスを受け取る初期化の入口。
///
/// # Safety
/// `hdir` はホストが `GlobalAlloc(GMEM_FIXED)` した `len` バイトの有効ハンドル。所有権は本関数へ移る。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn loadu(hdir: HGLOBAL, len: usize) -> i32 {
    // SAFETY: 呼出側の契約（上記）をそのまま引き継ぐ。
    unsafe { take_and_record("loadu", hdir, len) };
    if std::env::var(ENV_FAIL).as_deref() == Ok("1") {
        0
    } else {
        1
    }
}

/// SHIORI load: 従来の初期化の入口。常に `1`。
///
/// # Safety
/// `hdir` はホストが `GlobalAlloc(GMEM_FIXED)` した `len` バイトの有効ハンドル。所有権は本関数へ移る。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load(hdir: HGLOBAL, len: usize) -> i32 {
    // SAFETY: 呼出側の契約（上記）をそのまま引き継ぐ。
    unsafe { take_and_record("load", hdir, len) };
    1
}

/// SHIORI unload: 常に `1`。
///
/// # Safety
/// 引数を取らない flat-C エクスポート。呼出側の ABI 契約（cdecl）にのみ依存する。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unload() -> i32 {
    1
}

/// SHIORI request: 入力を callee 解放し、固定の 400 応答を新しいメモリで返す。
///
/// # Safety
/// `req` は callee 解放規約の有効ハンドル（または null）。`len` は有効な in/out ポインタ（null 防御あり）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn request(req: HGLOBAL, len: *mut usize) -> HGLOBAL {
    if !req.0.is_null() {
        // SAFETY: 所有権は callee へ移転済み。best-effort で解放する。
        unsafe {
            let _ = GlobalFree(Some(req));
        }
    }
    // SAFETY: GlobalAlloc は失敗時 Err を返す（下で防御）。
    let hresp = match unsafe { GlobalAlloc(GMEM_FIXED, RESP_400.len() + 1) } {
        Ok(h) if !h.0.is_null() => h,
        _ => {
            if !len.is_null() {
                // SAFETY: len 非 null を確認済み。
                unsafe { *len = 0 };
            }
            return HGLOBAL(std::ptr::null_mut());
        }
    };
    // SAFETY: hresp は RESP_400.len()+1 バイトの確保済み領域（GMEM_FIXED ゆえ h.0 が先頭）。
    unsafe {
        let dst = hresp.0 as *mut u8;
        std::ptr::copy_nonoverlapping(RESP_400.as_ptr(), dst, RESP_400.len());
        *dst.add(RESP_400.len()) = 0;
    }
    if !len.is_null() {
        // SAFETY: len 非 null を確認済み。
        unsafe { *len = RESP_400.len() };
    }
    hresp
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// env `HOST32_TESTDLL_LOADU_*` を触るテストの直列化（プロセス内で env は共有）。
    static ENV_SERIAL: Mutex<()> = Mutex::new(());

    /// `bytes` を `GlobalAlloc(GMEM_FIXED)` へコピーして返す（helper の入力 alloc を模す）。
    fn alloc_input(bytes: &[u8]) -> HGLOBAL {
        // SAFETY: Err／null は expect／assert で顕在化させる（テスト）。
        let h = unsafe { GlobalAlloc(GMEM_FIXED, bytes.len()) }.expect("GlobalAlloc failed");
        assert!(!h.0.is_null(), "GlobalAlloc returned null");
        // SAFETY: h は bytes.len() バイトの確保済み領域（GMEM_FIXED ゆえ h.0 が先頭）。
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), h.0 as *mut u8, bytes.len()) };
        h
    }

    /// 記録 env と偽返却 env を設定した状態で `f` を走らせ、記録ファイルの中身を返す。
    fn with_record(fail: bool, f: impl FnOnce() -> i32) -> (i32, String) {
        let _g = ENV_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let path = std::env::temp_dir().join(format!(
            "shiori_loadu_record_{}_{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // SAFETY: env を触るテストは ENV_SERIAL で直列化しており、同時に env を読み書きする
        // スレッドはこのクレートのテストに存在しない。
        unsafe {
            std::env::set_var(ENV_RECORD, &path);
            if fail {
                std::env::set_var(ENV_FAIL, "1");
            } else {
                std::env::remove_var(ENV_FAIL);
            }
        }
        let ret = f();
        // SAFETY: 同上（ENV_SERIAL 保持中）。
        unsafe {
            std::env::remove_var(ENV_RECORD);
            std::env::remove_var(ENV_FAIL);
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (ret, text)
    }

    const PATH_BYTES: &[u8] = b"C:\\g\xe3\x82\xb4\n";
    const PATH_HEX: &str = "433a5c67e382b40a";

    #[test]
    fn loadu_records_one_line_and_returns_1() {
        let (ret, text) = with_record(false, || {
            // SAFETY: 有効な入力 HGLOBAL と長さ。所有権は loadu へ移る。
            unsafe { loadu(alloc_input(PATH_BYTES), PATH_BYTES.len()) }
        });
        assert_eq!(ret, 1);
        assert_eq!(text, format!("loadu\t{PATH_HEX}\n"));
    }

    #[test]
    fn loadu_returns_0_when_fail_injected() {
        let (ret, text) = with_record(true, || {
            // SAFETY: 有効な入力 HGLOBAL と長さ。所有権は loadu へ移る。
            unsafe { loadu(alloc_input(PATH_BYTES), PATH_BYTES.len()) }
        });
        assert_eq!(ret, 0);
        assert_eq!(text, format!("loadu\t{PATH_HEX}\n"));
    }

    #[test]
    fn load_records_load_line_and_returns_1_even_when_fail_injected() {
        let (ret, text) = with_record(true, || {
            // SAFETY: 有効な入力 HGLOBAL と長さ。所有権は load へ移る。
            unsafe { load(alloc_input(PATH_BYTES), PATH_BYTES.len()) }
        });
        assert_eq!(ret, 1);
        assert_eq!(text, format!("load\t{PATH_HEX}\n"));
    }
}
