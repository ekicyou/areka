//! `os` の兄弟テスト（窓を作らない純粋な部分だけ）。

use super::*;

#[test]
fn utf16_fit_counts_whole_chars() {
    assert_eq!(chars_within_utf16("abc", 2), 2);
    assert_eq!(chars_within_utf16("aあb", 2), 2);
    assert_eq!(chars_within_utf16("abc", 0), 0);
    assert_eq!(chars_within_utf16("abc", 10), 3);
}

#[test]
fn utf16_fit_does_not_split_surrogate_pair() {
    // "a😀b" は UTF-16 で a・上位・下位・b の 4 つ。
    let s = "a😀b";
    assert_eq!(chars_within_utf16(s, 1), 1);
    assert_eq!(chars_within_utf16(s, 2), 1, "対の途中では切らない");
    assert_eq!(chars_within_utf16(s, 3), 2);
    assert_eq!(chars_within_utf16(s, 4), 3);
}
