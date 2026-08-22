//! 最小 SHIORI DLL fixture（出力名 `shiori.dll`）。
//!
//! pasta 非依存の決定的 LOAD／request E2E を成立させるための host-32 トラック所有の
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
use windows::Win32::System::Memory::{GLOBAL_ALLOC_FLAGS, GlobalAlloc};

/// `GlobalAlloc` の `GMEM_FIXED`（=0）。pasta／helper と同値で確保する（research §9.3）。
/// `GMEM_FIXED` ではハンドル＝先頭ポインタゆえ `h.0 as *mut u8` で直接書き込める。
const GMEM_FIXED: GLOBAL_ALLOC_FLAGS = GLOBAL_ALLOC_FLAGS(0);

/// テスト用 GET request の `ID`。**host 側 E2E（task 6.1）の契約値**。
///
/// E2E は `get("OnTestValue", ..)` を送出し、fixture は下記 200+Value を返す。
/// この定数を変えると host E2E の assert が破綻するため固定する。
pub const TEST_GET_ID: &str = "OnTestValue";

/// テスト用 NOTIFY request の `ID`。**host 側 E2E（task 6.1）の契約値**。
///
/// E2E は `notify("OnTestNotify", ..)` を送出し、fixture は下記 204 を返す。
/// この定数を変えると host E2E の assert が破綻するため固定する。
pub const TEST_NOTIFY_ID: &str = "OnTestNotify";

/// GET(`OnTestValue`) への固定 200 応答（R6.9）。
///
/// `Value` 本体はさくらスクリプトリテラル `\0\s[0]host32 request roundtrip ok\e`
/// （バックスラッシュは通常 ASCII バイト）。emo2=UTF-8 固定ゆえ非 UTF-8 を混ぜない。
const RESP_GET_200: &[u8] =
    b"SHIORI/3.0 200 OK\r\nCharset: UTF-8\r\nValue: \\0\\s[0]host32 request roundtrip ok\\e\r\n\r\n";

/// NOTIFY(`OnTestNotify`) への固定 204 応答（R6.9）。
const RESP_NOTIFY_204: &[u8] = b"SHIORI/3.0 204 No Content\r\n\r\n";

/// 不明 ID／malformed request line への固定 400 応答（R6.4 の assert 面）。
///
/// 誤って組み立てた request は silent success でなく検出可能な 400 を返す。
const RESP_400: &[u8] = b"SHIORI/3.0 400 Bad Request\r\n\r\n";

/// SHIORI load: 受領 HGLOBAL を callee 解放（二重解放検出器）。
///
/// env `HOST32_TESTDLL_LOAD_FAIL=1` で `false` を強制（R7.2）。
///
/// # Safety
/// ホストが `GlobalAlloc(GMEM_FIXED)` した有効なハンドルであること。
/// callee 解放規約により所有権は本関数へ移転する。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load(hdir: HGLOBAL, _len: usize) -> bool {
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
pub unsafe extern "C" fn unload() -> bool {
    if let Ok(path) = std::env::var("HOST32_TESTDLL_UNLOAD_MARKER") {
        let _ = std::fs::write(&path, b"unloaded");
    }
    true
}

/// 受領 request バイト列から request line（先頭行）と `ID:` ヘッダ値を取り出す純粋関数。
///
/// - 行区切りは CRLF／LF いずれも許容（codec の寛容度を写すが独立実装・R6.8：pilot 非依存）。
/// - ヘッダ名照合は case-insensitive（`ID`／`id`／`Id` を同一視）。
/// - 空行（ヘッダ部終端）以降は走査しない。
///
/// 戻り値: `(request_line, id_value)`。いずれも見つからなければ空文字列。
fn parse_request(bytes: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(bytes);
    // CRLF/LF 両対応: \r を除去してから \n 分割する。
    let normalized = text.replace('\r', "");
    let mut lines = normalized.split('\n');

    let request_line = lines.next().unwrap_or("").trim().to_string();

    let mut id_value = String::new();
    for line in lines {
        // 空行はヘッダ部終端。以降は走査しない。
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("ID")
        {
            id_value = value.trim().to_string();
            // 最初に一致した ID を採用。
            break;
        }
    }
    (request_line, id_value)
}

/// 受領 request line／`ID` から返すべき固定応答バイト列を選ぶ純粋関数（R6.4／R6.9）。
///
/// - `ID == OnTestValue`（かつ request line が `GET SHIORI/3.0`）→ 200+Value。
/// - `ID == OnTestNotify`（かつ request line が `NOTIFY SHIORI/3.0`）→ 204。
/// - それ以外（不明 ID／request line 不整合）→ 400（検出可能な失敗面）。
fn select_response(request_line: &str, id_value: &str) -> &'static [u8] {
    match id_value {
        TEST_GET_ID if request_line == "GET SHIORI/3.0" => RESP_GET_200,
        TEST_NOTIFY_ID if request_line == "NOTIFY SHIORI/3.0" => RESP_NOTIFY_204,
        _ => RESP_400,
    }
}

/// SHIORI request: 受領 request を検証し固定 SHIORI/3.0 応答を返す fixture（R6.1／6.2／6.4／6.9）。
///
/// 手順:
/// 1. `*len` バイトを入力 HGLOBAL から読み取り、ローカル `Vec<u8>` へコピー。
/// 2. **入力 HGLOBAL を callee 解放**（`GlobalFree`・二重解放検出器・R6.2）。
/// 3. request line／`ID` を検証（assert 面・R6.4）し固定応答を選択（R6.9）。
///    - GET `OnTestValue` → 200+Value／NOTIFY `OnTestNotify` → 204／その他 → 400。
/// 4. 応答を新規 `GlobalAlloc(GMEM_FIXED)` へコピーし `*len` に応答長を書き戻して返す。
///    応答 HGLOBAL は **caller(host)-free**（本関数は解放しない・R6.2）。
///
/// malformed 入力でも panic せず 400 を返す（fail-visible・非 silent）。
///
/// # Safety
/// 入力 `req` は callee 解放規約の有効ハンドル（GMEM_FIXED ゆえハンドル＝先頭ポインタ）。
/// `len` は呼出側が有効な in/out ポインタを渡す契約だが null 防御を行う。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn request(req: HGLOBAL, len: *mut usize) -> HGLOBAL {
    // 手順 1: 入力長を読み取る（len null 防御）。null なら受領内容を検証できないため
    // 入力解放のみ行い 400 相当の空応答は返せない → null 応答で返す（防御的）。
    let in_len = if len.is_null() {
        0
    } else {
        // SAFETY: len 非 null を確認済み。呼出側が有効 in ポインタを渡す契約。
        unsafe { *len }
    };

    // 入力バイト列をローカルへコピー（GMEM_FIXED ゆえ req.0 が先頭ポインタ）。
    let received: Vec<u8> = if req.0.is_null() || in_len == 0 {
        Vec::new()
    } else {
        // SAFETY: req は in_len バイトの有効領域（helper が GlobalAlloc(GMEM_FIXED) 済み）。
        unsafe { std::slice::from_raw_parts(req.0 as *const u8, in_len).to_vec() }
    };

    // 手順 2: 入力 HGLOBAL を callee 解放（受領後・二重解放検出器・R6.2）。
    // SAFETY: 所有権は callee へ移転済み。best-effort で解放する。
    unsafe {
        let _ = GlobalFree(Some(req));
    }

    // 手順 3: 受領内容を検証し固定応答を選択（R6.4／R6.9）。
    let (request_line, id_value) = parse_request(&received);
    let resp = select_response(&request_line, &id_value);

    // 手順 4: 応答を新規 GlobalAlloc(GMEM_FIXED) へコピーし返す（caller-free 被検証側・R6.2）。
    // spec_dll 推奨に従い len+1 でゼロ終端付き確保（host は len 厳守だが安全側）。
    // SAFETY: GlobalAlloc は失敗時 Err を返す（下で防御）。
    let alloc = unsafe { GlobalAlloc(GMEM_FIXED, resp.len() + 1) };
    let hresp = match alloc {
        Ok(h) if !h.0.is_null() => h,
        _ => {
            // 確保失敗時は null 応答＋len=0（fail-visible・panic せず）。
            if !len.is_null() {
                // SAFETY: len 非 null を確認済み。
                unsafe {
                    *len = 0;
                }
            }
            return HGLOBAL(std::ptr::null_mut());
        }
    };

    // SAFETY: hresp は resp.len()+1 バイトの確保済み領域（GMEM_FIXED ゆえ h.0 が先頭）。
    unsafe {
        let dst = hresp.0 as *mut u8;
        std::ptr::copy_nonoverlapping(resp.as_ptr(), dst, resp.len());
        // 末尾 1 バイトを NUL 終端（host は len 厳守ゆえ optional だが安全側）。
        *dst.add(resp.len()) = 0;
    }

    // 応答長を書き戻す（NUL 終端は含めない・len=resp.len()）。
    if !len.is_null() {
        // SAFETY: len 非 null を確認済み。
        unsafe {
            *len = resp.len();
        }
    }

    hresp
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用: `bytes` を `GlobalAlloc(GMEM_FIXED)` へコピーして HGLOBAL を返す
    /// （helper の入力 alloc を模す）。
    fn alloc_input(bytes: &[u8]) -> HGLOBAL {
        // SAFETY: GlobalAlloc の Err／null は unwrap／assert で顕在化させる（テスト）。
        let h = unsafe { GlobalAlloc(GMEM_FIXED, bytes.len()) }.expect("GlobalAlloc failed");
        assert!(!h.0.is_null(), "GlobalAlloc returned null");
        // SAFETY: h は bytes.len() バイトの確保済み領域（GMEM_FIXED ゆえ h.0 が先頭）。
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), h.0 as *mut u8, bytes.len());
        }
        h
    }

    /// テスト用: 応答 HGLOBAL から `len` バイトを読み取り caller-free する
    /// （host の caller-free を模す・二重解放が起きれば GlobalFree が失敗検出）。
    fn read_and_free(hresp: HGLOBAL, len: usize) -> Vec<u8> {
        assert!(!hresp.0.is_null(), "response HGLOBAL was null");
        // SAFETY: hresp は host が返した len バイトの有効領域（GMEM_FIXED）。
        let bytes = unsafe { std::slice::from_raw_parts(hresp.0 as *const u8, len).to_vec() };
        // caller-free（host 側解放の忠実再現）。
        // SAFETY: hresp は callee(DLL) が alloc し所有権は caller へ移転済み。
        unsafe {
            let _ = GlobalFree(Some(hresp));
        }
        bytes
    }

    /// 与えた request バイト列で `request` を駆動し応答バイト列を返すヘルパ。
    fn roundtrip(req_bytes: &[u8]) -> Vec<u8> {
        let hreq = alloc_input(req_bytes);
        let mut len: usize = req_bytes.len();
        // SAFETY: hreq は len バイトの有効入力 HGLOBAL、&mut len は有効 in/out ポインタ。
        let hresp = unsafe { request(hreq, &mut len as *mut usize) };
        read_and_free(hresp, len)
    }

    #[test]
    fn get_ontestvalue_returns_200_with_value() {
        let req = b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestValue\r\n\r\n";
        let resp = roundtrip(req);
        let text = String::from_utf8(resp).expect("response must be valid UTF-8");
        assert!(
            text.starts_with("SHIORI/3.0 200 OK"),
            "expected 200 status, got: {text:?}"
        );
        assert!(
            text.contains("Value: \\0\\s[0]host32 request roundtrip ok\\e"),
            "expected fixed Value line, got: {text:?}"
        );
    }

    #[test]
    fn notify_ontestnotify_returns_204() {
        let req =
            b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\nSender: areka\r\nID: OnTestNotify\r\n\r\n";
        let resp = roundtrip(req);
        let text = String::from_utf8(resp).expect("response must be valid UTF-8");
        assert!(
            text.starts_with("SHIORI/3.0 204 No Content"),
            "expected 204 status, got: {text:?}"
        );
    }

    #[test]
    fn unknown_id_returns_400() {
        let req = b"GET SHIORI/3.0\r\nCharset: UTF-8\r\nID: OnGarbage\r\n\r\n";
        let resp = roundtrip(req);
        let text = String::from_utf8(resp).expect("response must be valid UTF-8");
        assert!(
            text.starts_with("SHIORI/3.0 400 Bad Request"),
            "expected 400 for unknown ID, got: {text:?}"
        );
    }

    #[test]
    fn malformed_request_line_returns_400() {
        // ID は既知だが request line が壊れている（GET/NOTIFY いずれでもない）→ 400。
        let req = b"BOGUS LINE\r\nID: OnTestValue\r\n\r\n";
        let resp = roundtrip(req);
        let text = String::from_utf8(resp).expect("response must be valid UTF-8");
        assert!(
            text.starts_with("SHIORI/3.0 400 Bad Request"),
            "expected 400 for malformed request line, got: {text:?}"
        );
    }

    #[test]
    fn get_id_case_insensitive_header_name() {
        // ヘッダ名は case-insensitive で照合する（`id:` でも一致）。
        let req = b"GET SHIORI/3.0\r\nid: OnTestValue\r\n\r\n";
        let resp = roundtrip(req);
        let text = String::from_utf8(resp).expect("response must be valid UTF-8");
        assert!(
            text.starts_with("SHIORI/3.0 200 OK"),
            "case-insensitive ID header must match, got: {text:?}"
        );
    }

    #[test]
    fn lf_only_line_endings_tolerated() {
        // CRLF でなく LF のみでも request line／ID を取り出せる（寛容度）。
        let req = b"GET SHIORI/3.0\nID: OnTestValue\n\n";
        let resp = roundtrip(req);
        let text = String::from_utf8(resp).expect("response must be valid UTF-8");
        assert!(
            text.starts_with("SHIORI/3.0 200 OK"),
            "LF-only endings must be tolerated, got: {text:?}"
        );
    }

    #[test]
    fn returned_length_matches_response_body() {
        // *len には NUL 終端を含めない応答本体長が書き戻される。
        let req = b"NOTIFY SHIORI/3.0\r\nID: OnTestNotify\r\n\r\n";
        let hreq = alloc_input(req);
        let mut len: usize = req.len();
        // SAFETY: 有効な入力 HGLOBAL と in/out ポインタ。
        let hresp = unsafe { request(hreq, &mut len as *mut usize) };
        assert_eq!(
            len,
            RESP_NOTIFY_204.len(),
            "*len must equal response body length"
        );
        let _ = read_and_free(hresp, len);
    }
}
