//! パスのパーセント符号化の判定・復号・符号化（純関数・要件 1.15）。
// ponytail: 本番の呼び手（manifest）が付くまでの間だけ。manifest から呼んだら外す。
#![cfg_attr(not(test), allow(dead_code))]

/// 全文字 ASCII かつ `%` が必ず 16 進 2 桁を伴うときだけ「符号化済み」。
pub(crate) fn is_encoded(path: &str) -> bool {
    let b = path.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                if !(i + 2 < b.len() && hex(b[i + 1]).is_some() && hex(b[i + 2]).is_some()) {
                    return false;
                }
                i += 3;
            }
            c if c.is_ascii() => i += 1,
            _ => return false,
        }
    }
    true
}

/// `%XX` をバイトへ戻す。文字コードは解釈しない（読み方は呼び手が決める）。
/// 形の崩れた `%` はそのまま残す。
pub(crate) fn decode(path: &str) -> Vec<u8> {
    let b = path.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push((h << 4) | l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// UTF-8 のバイト列を大文字の `%XX` に。未予約文字（`A-Z a-z 0-9 - . _ ~`）と `/` は残す。
pub(crate) fn encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for &c in path.as_bytes() {
        if c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{c:02X}"));
        }
    }
    out
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

#[cfg(test)]
#[path = "urlpath_tests.rs"]
mod tests;
