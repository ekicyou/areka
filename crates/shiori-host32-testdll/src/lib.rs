//! 最小 SHIORI DLL fixture（出力名 `shiori.dll`）。
//!
//! pasta 非依存の決定的 LOAD E2E を成立させるための host-32 トラック所有の
//! 最小 SHIORI DLL。flat-C の `load`／`unload`／`request` 3 エクスポートを実装する。
//!
//! 署名・所有権規約は正確源（`vendors/pasta/crates/pasta_shiori/src/windows.rs`・
//! research.md §9）に忠実に一致させる:
//! - `load(hdir: HGLOBAL, len: usize) -> bool`／`unload() -> bool`／
//!   `request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（cdecl・戻り `bool` は Rust bool 1 byte）。
//! - **入力 HGLOBAL は callee(DLL) が `GlobalFree` する**（`ShioriString::capture` の `has_free:true`
//!   ＝Drop で GlobalFree の忠実再現）。ホスト側が誤って二重解放したら検出できる。
//! - シンボルは `#[unsafe(no_mangle)]` で無装飾。

use windows::Win32::Foundation::{GlobalFree, HGLOBAL};

/// SHIORI load: 受領 HGLOBAL を callee 解放（二重解放検出器）。
///
/// env `HOST32_TESTDLL_LOAD_FAIL=1` で `false` を強制（R7.2）。
///
/// # Safety
/// ホストが `GlobalAlloc(GMEM_FIXED)` した有効なハンドルであること。
/// callee 解放規約により所有権は本関数へ移転する。
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn load(hdir: HGLOBAL, _len: usize) -> bool {
    // 入力 HGLOBAL を callee 解放（pasta 規約の忠実再現・二重解放検出器）。
    // SAFETY: 呼出側が GlobalAlloc(GMEM_FIXED) した有効ハンドルを渡す契約。
    // 所有権は callee へ移転済み。GlobalFree の Result は best-effort で無視する。
    unsafe {
        let _ = GlobalFree(Some(hdir));
    }
    if std::env::var("HOST32_TESTDLL_LOAD_FAIL").as_deref() == Ok("1") {
        return false;
    }
    true
}

/// SHIORI unload: `true` 返し。
///
/// env `HOST32_TESTDLL_UNLOAD_MARKER` 指定時はそのファイルパスを作成し、
/// courtesy unload の実呼出を観測可能化する（Drop teardown テストの決定的証拠・
/// R2.2・validation issue #2）。
///
/// # Safety
/// 引数を取らない flat-C エクスポート。呼出側の ABI 契約（cdecl）にのみ依存する。
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn unload() -> bool {
    if let Ok(path) = std::env::var("HOST32_TESTDLL_UNLOAD_MARKER") {
        let _ = std::fs::write(&path, b"unloaded");
    }
    true
}

/// SHIORI request: 最小 stub（本仕様では呼ばれないが解決対象・R4.2）。
///
/// 入力 HGLOBAL を callee 解放し、null 応答＋`len=0` を返す。
///
/// # Safety
/// 入力 `req` は callee 解放規約の有効ハンドル。`len` は呼出側が有効な
/// out ポインタを渡す契約だが、null 防御を行う。
#[unsafe(no_mangle)]
pub unsafe extern "cdecl" fn request(req: HGLOBAL, len: *mut usize) -> HGLOBAL {
    // SAFETY: 入力 req は callee 解放規約（所有権移転済み）。best-effort で解放する。
    unsafe {
        let _ = GlobalFree(Some(req));
    }
    // SAFETY: len が非 null のときのみ書き込む（null 防御）。
    if !len.is_null() {
        unsafe {
            *len = 0;
        }
    }
    HGLOBAL(std::ptr::null_mut())
}
