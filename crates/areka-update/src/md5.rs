//! バイト列の MD5 を OS の CNG で計算する（要件 2.1・2.2・10.2）。
// ponytail: 本番の呼び手（diff・lib）が付くまでの間だけ。diff から呼んだら外す。
#![cfg_attr(not(test), allow(dead_code))]

use std::fmt::Write as _;

use windows::Win32::Security::Cryptography::{BCRYPT_MD5_ALG_HANDLE, BCryptHash};

/// `BCryptHash(BCRYPT_MD5_ALG_HANDLE, None, bytes, &mut [u8; 16])` 1 呼出で、小文字 32 桁を返す。
/// 前提: `bytes.len() <= u32::MAX`（呼び出し側が守る。ローカルの読みは `diff` が 4 GiB 超を
/// `LocalUnreadable` にし、取得の本文は `winhttp` が `MAX_BODY_BYTES` で止める）。
/// `NTSTATUS` が失敗なら致命として `panic!`（Windows 10 以降で擬似ハンドルの MD5 が失敗するのは
/// OS の暗号基盤が壊れているとき＝回復不能・`logging.md` の「panic は致命限定」）。
pub(crate) fn md5_hex(bytes: &[u8]) -> String {
    let mut digest = [0u8; 16];
    // SAFETY: 擬似ハンドル `BCRYPT_MD5_ALG_HANDLE` は Open／Close 不要で常に有効。入力と出力は
    // 生存中のスライスで、束縛が長さを u32 へ変換して渡す。出力 16 バイトは MD5 の長さと一致する。
    let status = unsafe { BCryptHash(BCRYPT_MD5_ALG_HANDLE, None, bytes, &mut digest) };
    if let Err(e) = status.ok() {
        panic!("CNG の MD5 が失敗した（OS の暗号基盤の破損）: {e}");
    }
    digest.iter().fold(String::with_capacity(32), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
#[path = "md5_tests.rs"]
mod tests;
