//! `\_u`・`\_m` の引数の読み（`read_char_code`）の兄弟テスト（script-small-tags 要件 1.1・1.2・
//! 1.4・2.1・2.2・2.4・2.6・6.6）。
//!
//! 設計の表の綴りを 1 行ずつ期待値と並べる。判定の順は 0 → 制御の文字 → U+10FFFF より上 →
//! サロゲート → 字。

use super::CharCodeDrop;
use super::char_code::{CharCodeRead, read_char_code};

fn read(arg: &str) -> CharCodeRead {
    read_char_code(Some(arg))
}

#[test]
fn hex_and_decimal_become_chars() {
    // 1.1・1.2・1.4: 16 進の 0x／0X・大小・先頭の 0・前後の空白、10 進。
    assert_eq!(read("0x3000"), CharCodeRead::Char('\u{3000}'));
    assert_eq!(read("8942"), CharCodeRead::Char('\u{22EE}'));
    assert_eq!(read("0X22ee"), CharCodeRead::Char('\u{22EE}'));
    assert_eq!(read("0x022EE"), CharCodeRead::Char('\u{22EE}'));
    assert_eq!(read(" 0x22EE"), CharCodeRead::Char('\u{22EE}'));
    assert_eq!(read("0x22EE "), CharCodeRead::Char('\u{22EE}'));
    assert_eq!(read("0x1F600"), CharCodeRead::Char('\u{1F600}'));
    assert_eq!(read("65"), CharCodeRead::Char('A'));
    assert_eq!(read("0x5C"), CharCodeRead::Char('\\'));
}

#[test]
fn zero_is_dropped() {
    // 2.1
    assert_eq!(read("0"), CharCodeRead::Drop(CharCodeDrop::Zero));
    assert_eq!(read("0x0"), CharCodeRead::Drop(CharCodeDrop::Zero));
}

#[test]
fn control_chars_are_dropped() {
    // 2.2: U+0001〜U+001F・U+007F〜U+009F。境目の外側（U+0020・U+00A0）は字。
    for arg in ["0x01", "0x09", "0x1F", "0x7F", "0x85", "0x9F"] {
        assert_eq!(
            read(arg),
            CharCodeRead::Drop(CharCodeDrop::Control),
            "{arg}"
        );
    }
    assert_eq!(read("0x20"), CharCodeRead::Char(' '));
    assert_eq!(read("0xA0"), CharCodeRead::Char('\u{A0}'));
}

#[test]
fn above_unicode_range_is_out_of_range() {
    // 2.4: 32 ビットに入らない桁も範囲の外（読めない番号にしない）。
    assert_eq!(
        read("0x110000"),
        CharCodeRead::Drop(CharCodeDrop::OutOfRange)
    );
    assert_eq!(
        read("0xFFFFFFFFFF"),
        CharCodeRead::Drop(CharCodeDrop::OutOfRange)
    );
    assert_eq!(
        read("99999999999999999999"),
        CharCodeRead::Drop(CharCodeDrop::OutOfRange)
    );
    assert_eq!(read("0x10FFFF"), CharCodeRead::Char('\u{10FFFF}'));
}

#[test]
fn surrogates_are_returned_to_the_caller() {
    assert_eq!(read("0xD800"), CharCodeRead::HighSurrogate(0xD800));
    assert_eq!(read("0xD83D"), CharCodeRead::HighSurrogate(0xD83D));
    assert_eq!(read("0xDBFF"), CharCodeRead::HighSurrogate(0xDBFF));
    assert_eq!(read("0xDC00"), CharCodeRead::LowSurrogate(0xDC00));
    assert_eq!(read("0xDE00"), CharCodeRead::LowSurrogate(0xDE00));
    assert_eq!(read("0xDFFF"), CharCodeRead::LowSurrogate(0xDFFF));
}

#[test]
fn unreadable_spellings_are_dropped() {
    // 2.6: 空・0x だけ・ほかの字・符号・引数なし。
    for arg in ["", " ", "0x", "abc", "-1", "+1", "0x12G", "1.5", "0x 12"] {
        assert_eq!(
            read(arg),
            CharCodeRead::Drop(CharCodeDrop::Unreadable),
            "{arg:?}"
        );
    }
    assert_eq!(
        read_char_code(None),
        CharCodeRead::Drop(CharCodeDrop::Unreadable)
    );
}
