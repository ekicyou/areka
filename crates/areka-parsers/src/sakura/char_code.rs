//! `\_u`・`\_m` の引数を文字の番号として読む純粋な関数（script-small-tags 要件 1.1〜1.4・2.1・
//! 2.2・2.4・2.6・6.6）。
//!
//! 読み手の中だけで使う。記録は出さず、失敗もしない（捨てる理由を返す）。

use super::model::CharCodeDrop;

/// `\_u`・`\_m` の引数の読み。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharCodeRead {
    /// 出せる番号の字。
    Char(char),
    /// 上位のサロゲートの番号（組にするかは呼び手が決める）。
    HighSurrogate(u16),
    /// 下位のサロゲートの番号（組にするかは呼び手が決める）。
    LowSurrogate(u16),
    /// 捨てる。
    Drop(CharCodeDrop),
}

/// 最初の引数を番号として読む（2 つ目からの引数は呼び手が渡さない）。
///
/// 前後の空白を除き、`0x`／`0X` に続く 16 進か 10 進だけを読む。判定の順は
/// 0 → 制御の文字 → U+10FFFF より上 → サロゲート → 字。
#[cfg_attr(not(test), expect(dead_code, reason = "結線は script-small-tags 2.1"))]
pub(crate) fn read_char_code(arg: Option<&str>) -> CharCodeRead {
    let Some(arg) = arg else {
        return CharCodeRead::Drop(CharCodeDrop::Unreadable);
    };
    let arg = arg.trim();
    let (digits, radix) = match arg.strip_prefix("0x").or_else(|| arg.strip_prefix("0X")) {
        Some(hex) => (hex, 16),
        None => (arg, 10),
    };
    // from_str_radix は先頭の `+` を通すので、桁の字だけかを先に確かめる。
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return CharCodeRead::Drop(CharCodeDrop::Unreadable);
    }
    // 桁は正しいので、失敗は 32 ビットに入らない桁あふれだけ。
    let Ok(code) = u32::from_str_radix(digits, radix) else {
        return CharCodeRead::Drop(CharCodeDrop::OutOfRange);
    };
    match code {
        0 => CharCodeRead::Drop(CharCodeDrop::Zero),
        0x01..=0x1F | 0x7F..=0x9F => CharCodeRead::Drop(CharCodeDrop::Control),
        0x11_0000.. => CharCodeRead::Drop(CharCodeDrop::OutOfRange),
        0xD800..=0xDBFF => CharCodeRead::HighSurrogate(code as u16),
        0xDC00..=0xDFFF => CharCodeRead::LowSurrogate(code as u16),
        // 上の腕で 0・サロゲート・範囲の外を除いたので必ず字になる。
        _ => CharCodeRead::Char(char::from_u32(code).expect("サロゲートと範囲の外は除いた")),
    }
}
