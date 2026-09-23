//! 取得の境界の本物の実装＝WinHTTP（要件 3.3〜3.6）。
//!
//! `unsafe` と WinHTTP の綴りはこのファイルだけに置く。常時テストはこのファイルを呼ばず、
//! 振る舞いは `#[ignore]` の実機の一周で確かめる。

use core::ffi::c_void;

use windows::Win32::Networking::WinHttp::{
    URL_COMPONENTS, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_INTERNET_SCHEME_HTTP, WINHTTP_INTERNET_SCHEME_HTTPS, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpCrackUrl, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};
use windows::core::{Error, PCWSTR, w};

use crate::error::FetchError;
use crate::fetch::Fetch;

/// 1 件の本文の上限。`Content-Length` は信じず、実際に読んだ量で止める。
pub const MAX_BODY_BYTES: usize = 256 * 1024 * 1024;
/// 名前解決 10 s・接続 15 s・送信 30 s・受信 60 s（`WinHttpSetTimeouts` の 4 引数・ms）。
const TIMEOUTS_MS: (i32, i32, i32, i32) = (10_000, 15_000, 30_000, 60_000);
const USER_AGENT: &str = concat!("areka/", env!("CARGO_PKG_VERSION"));

/// `ERROR_WINHTTP_UNRECOGNIZED_SCHEME`（http／https 以外の URL）。
const ERROR_UNRECOGNIZED_SCHEME: u32 = 12006;
/// `ERROR_WINHTTP_INVALID_URL`（空の URL）。
const ERROR_INVALID_URL: u32 = 12005;

/// WinHTTP のハンドル 1 つ。成功・失敗のどの経路でも落ちるときに閉じる。
struct Handle(*mut c_void);

impl Handle {
    /// 生成関数の戻り値を受け取る。null なら直前のエラー番号を失敗に写す。
    fn new(raw: *mut c_void) -> Result<Handle, FetchError> {
        if raw.is_null() {
            Err(from_os(&Error::from_thread()))
        } else {
            Ok(Handle(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: `self.0` は WinHTTP が返した非 null のハンドルで、閉じるのはここ 1 回だけ。
        // 閉じる失敗は回復の手段が無く、閉じた後にハンドルを使う経路も無いので捨てる。
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

/// セッション（`WinHttpOpen`）を 1 つ持ち回る。呼び出し側が一周ごとに作る。
///
/// 生ポインタを持つので `Send`／`Sync` ではない（作ったスレッドで同期に使う＝3.7）。
pub struct WinHttpFetch {
    session: Handle,
}

impl WinHttpFetch {
    /// `WinHttpOpen(USER_AGENT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null, null, 0)` → `WinHttpSetTimeouts`。
    pub fn new() -> Result<WinHttpFetch, FetchError> {
        let agent = wide(USER_AGENT.encode_utf16());
        // SAFETY: `agent` は NUL 終端の UTF-16 で、呼出の間生きている。proxy と bypass は null
        // （自動のプロキシ設定では指定しない）。フラグ 0 は同期モード。
        let raw = unsafe {
            WinHttpOpen(
                PCWSTR(agent.as_ptr()),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            )
        };
        let session = Handle::new(raw)?;
        let (resolve, connect, send, receive) = TIMEOUTS_MS;
        // SAFETY: `session.0` は今開いた生存中のセッションハンドル。
        unsafe { WinHttpSetTimeouts(session.0, resolve, connect, send, receive) }
            .map_err(|e| from_os(&e))?;
        Ok(WinHttpFetch { session })
    }
}

impl Fetch for WinHttpFetch {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let url: Vec<u16> = url.encode_utf16().collect();
        check_url(&url)?;

        // 1. scheme・host・port・path＋query を分ける。長さに非 0 を入れると、
        //    各欄は `url` の中を指すポインタと長さで返る（複写しない）。
        let mut parts = URL_COMPONENTS {
            dwStructSize: size_of::<URL_COMPONENTS>() as u32,
            dwSchemeLength: u32::MAX,
            dwHostNameLength: u32::MAX,
            dwUrlPathLength: u32::MAX,
            dwExtraInfoLength: u32::MAX,
            ..Default::default()
        };
        // SAFETY: `url` は空でない（`check_url` 済み）ので、長さ `url.len()` の有効なバッファを
        // 指す（長さ 0 の「NUL 終端まで読む」解釈に落ちない）。`url` は呼出の間生きており、
        // `parts` は大きさを設定した有効な構造体。
        unsafe { WinHttpCrackUrl(&url, 0, &mut parts) }.map_err(|e| from_os(&e))?;
        let secure = match parts.nScheme {
            WINHTTP_INTERNET_SCHEME_HTTPS => true,
            WINHTTP_INTERNET_SCHEME_HTTP => false,
            _ => {
                return Err(FetchError::Other {
                    code: ERROR_UNRECOGNIZED_SCHEME,
                });
            }
        };
        // SAFETY: 分解が成功したので、host と path は `url` の中を指し、長さはその内側に収まる
        // （クエリ `lpszExtraInfo` は path の直後に続く）。`url` はこの関数の終わりまで生きる。
        let (host, object) = unsafe {
            (
                wide(
                    slice(parts.lpszHostName.0, parts.dwHostNameLength)
                        .iter()
                        .copied(),
                ),
                wide(
                    slice(
                        parts.lpszUrlPath.0,
                        parts.dwUrlPathLength + parts.dwExtraInfoLength,
                    )
                    .iter()
                    .copied(),
                ),
            )
        };

        // 2. 接続と要求。落ちる順は宣言の逆＝request → connect（6.）。
        // SAFETY: セッションは `self` が生きている間有効。`host` は NUL 終端で呼出の間生きる。
        let connect = Handle::new(unsafe {
            WinHttpConnect(self.session.0, PCWSTR(host.as_ptr()), parts.nPort, 0)
        })?;
        let flags = if secure {
            WINHTTP_FLAG_SECURE
        } else {
            WINHTTP_OPEN_REQUEST_FLAGS(0)
        };
        // SAFETY: `connect` は生存中。`object` は NUL 終端。版・referer・accept は null＝既定。
        let request = Handle::new(unsafe {
            WinHttpOpenRequest(
                connect.0,
                w!("GET"),
                PCWSTR(object.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                core::ptr::null(),
                flags,
            )
        })?;

        // 3. 転送の方針（3xx の自動追随・https → http の降格拒否）と圧縮の扱い（自動伸長しない）は
        //    WinHTTP の既定のまま変えない。

        // 4. 送って受け、状態コードを数値で取る。
        // SAFETY: `request` は生存中。追加のヘッダ・本文は無い。
        unsafe { WinHttpSendRequest(request.0, None, None, 0, 0, 0) }.map_err(|e| from_os(&e))?;
        // SAFETY: `request` は生存中。第 2 引数は予約で null。
        unsafe { WinHttpReceiveResponse(request.0, core::ptr::null_mut()) }
            .map_err(|e| from_os(&e))?;
        let mut code: u32 = 0;
        let mut len = size_of::<u32>() as u32;
        // SAFETY: `code` は 4 バイトで、`len` はその大きさ。名前と添字は使わない（null）。
        unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some((&raw mut code).cast()),
                &mut len,
                core::ptr::null_mut(),
            )
        }
        .map_err(|e| from_os(&e))?;
        status_to_result(code)?;

        // 5. 0 バイトまで読む。上限は実際に読んだ量で判定する。
        let mut body = Vec::new();
        loop {
            let mut available: u32 = 0;
            // SAFETY: `request` は生存中。`available` は書き込み先の u32。
            unsafe { WinHttpQueryDataAvailable(request.0, &mut available) }
                .map_err(|e| from_os(&e))?;
            if available == 0 {
                break;
            }
            if available as usize > MAX_BODY_BYTES - body.len() {
                return Err(FetchError::TooLarge {
                    limit: MAX_BODY_BYTES,
                });
            }
            let start = body.len();
            body.resize(start + available as usize, 0);
            let mut read: u32 = 0;
            // SAFETY: 書き込み先は `body[start..]` の `available` バイトで、確保済み。
            unsafe {
                WinHttpReadData(
                    request.0,
                    body[start..].as_mut_ptr().cast(),
                    available,
                    &mut read,
                )
            }
            .map_err(|e| from_os(&e))?;
            body.truncate(start + read as usize);
            if read == 0 {
                break;
            }
        }
        // 6. `request` → `connect` の順に、この関数を抜けるときに閉じる（`?` の経路も同じ）。
        Ok(body)
    }
}

/// `ptr` から `len` 個の UTF-16 を借りる。
///
/// # Safety
/// `len` が 0 でなければ、`ptr` は `len` 個の有効な `u16` を指していること。
unsafe fn slice<'a>(ptr: *const u16, len: u32) -> &'a [u16] {
    if len == 0 || ptr.is_null() {
        &[]
    } else {
        // SAFETY: 呼び出し側の約束どおり、`ptr` から `len` 個が有効。
        unsafe { core::slice::from_raw_parts(ptr, len as usize) }
    }
}

/// NUL 終端の UTF-16 にする。
fn wide(units: impl Iterator<Item = u16>) -> Vec<u16> {
    units.chain(core::iter::once(0)).collect()
}

/// 状態コードを判定する（3.5）。404 は「無い」、2xx 以外はコード付きの失敗。
/// 3xx は自動追随の後なので、ここに来るのは追随できなかったものだけ。
fn status_to_result(code: u32) -> Result<(), FetchError> {
    match code {
        200..=299 => Ok(()),
        404 => Err(FetchError::NotFound),
        _ => Err(FetchError::Status {
            code: u16::try_from(code).unwrap_or(u16::MAX),
        }),
    }
}

/// OS のエラー（`HRESULT_FROM_WIN32`）の下位 16 ビット＝Win32 エラー番号を写す（3.6）。
fn from_os(e: &Error) -> FetchError {
    from_win32(e.code().0 as u32 & 0xFFFF)
}

/// Win32 エラー番号 → 取得の失敗。表はこの 1 つだけ。
fn from_win32(code: u32) -> FetchError {
    match code {
        12007 => FetchError::NameResolution, // ERROR_WINHTTP_NAME_NOT_RESOLVED
        12029 | 12030 => FetchError::Connect, // ERROR_WINHTTP_CANNOT_CONNECT／CONNECTION_ERROR
        12002 => FetchError::Timeout,        // ERROR_WINHTTP_TIMEOUT
        12175 | 12188 => FetchError::Tls,    // ERROR_WINHTTP_SECURE_FAILURE（_PROXY）
        _ => FetchError::Other { code },
    }
}

/// 空の URL は OS を呼ぶ前に弾く（`WinHttpCrackUrl` は長さ 0 を「NUL 終端まで読む」と解するため、
/// 空のバッファを渡すと確保されていない先を読む）。
fn check_url(url: &[u16]) -> Result<(), FetchError> {
    if url.is_empty() {
        Err(FetchError::Other {
            code: ERROR_INVALID_URL,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "winhttp_tests.rs"]
mod tests;
