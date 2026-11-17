//! 選択肢の範囲の換算。areka の文字の位置は拡張書記素クラスタの序数
//! （`areka_emo_text::state::ChoiceSpan::glyph_range` と同じ単位・切り方は `areka_sakura::cluster`）。
//! UTF-16 の位置は DirectWrite の `hit_test_point` が返す位置の単位。

use areka_sakura::cluster::{cluster_count, clusters};
use std::ops::Range;

pub struct Choice {
    pub number: u32,
    /// 本文の UTF-8 のバイト範囲（クラスタの境目にある）。
    pub bytes: Range<usize>,
}

/// `byte` はクラスタの境目であること。
pub fn byte_to_cluster(text: &str, byte: usize) -> usize {
    cluster_count(&text[..byte])
}

pub fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text[..byte].encode_utf16().count()
}

/// 範囲を越えた序数は文字列の末尾へ。
pub fn cluster_to_byte(text: &str, cluster: usize) -> usize {
    clusters(text).take(cluster).map(str::len).sum()
}

pub fn cluster_to_utf16(text: &str, cluster: usize) -> usize {
    byte_to_utf16(text, cluster_to_byte(text, cluster))
}

pub fn choice_cluster_range(text: &str, c: &Choice) -> Range<usize> {
    byte_to_cluster(text, c.bytes.start)..byte_to_cluster(text, c.bytes.end)
}

/// UTF-16 の位置から UTF-8 のバイト位置へ。サロゲートの途中は字の先頭へ寄せる。
pub fn utf16_to_byte(text: &str, u16pos: usize) -> usize {
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if n + c.len_utf16() > u16pos {
            return i;
        }
        n += c.len_utf16();
    }
    text.len()
}

pub fn choice_utf16_range(text: &str, c: &Choice) -> Range<usize> {
    byte_to_utf16(text, c.bytes.start)..byte_to_utf16(text, c.bytes.end)
}

/// `hit_test_point` の結果（UTF-16 の位置）から選択肢の番号へ。
pub fn choice_at(text: &str, choices: &[Choice], u16pos: usize) -> Option<u32> {
    choices
        .iter()
        .find(|c| choice_utf16_range(text, c).contains(&u16pos))
        .map(|c| c.number)
}

#[cfg(test)]
mod tests {
    use super::*;

    // クラスタ: あ 🍣️ い 👨‍👩‍👧 選
    // バイト:   あ=0..3, 🍣=3..7, FE0F=7..10, い=10..13, 家族=13..31, 選=31..34
    // UTF-16:   あ=0, 🍣=1..3, FE0F=3, い=4, 家族=5..13, 選=13
    const T: &str = "あ🍣\u{FE0F}い👨\u{200D}👩\u{200D}👧選";

    #[test]
    fn cluster_ordinal_is_not_scalar_index() {
        assert_eq!(byte_to_cluster(T, 31), 4);
        assert_ne!(byte_to_cluster(T, 31), T[..31].chars().count()); // スカラー値では 9
        assert_eq!(byte_to_cluster(T, 10), 2); // 🍣️ は 1 クラスタ
        assert_eq!(cluster_to_byte(T, 3), 13);
        assert_eq!(cluster_to_byte(T, 4), 31);
        assert_eq!(cluster_to_byte(T, 99), T.len());
        assert_eq!(cluster_to_utf16(T, 2), 4);
        assert_eq!(cluster_to_utf16(T, 4), 13);
    }

    #[test]
    fn utf16_with_surrogate_pair() {
        assert_eq!(byte_to_utf16(T, 7), 3); // 🍣 は UTF-16 で 2 つ
        assert_eq!(utf16_to_byte(T, 3), 7);
        assert_eq!(utf16_to_byte(T, 2), 3); // サロゲートの途中は字の先頭
        assert_eq!(utf16_to_byte(T, 99), T.len());
    }

    #[test]
    fn hit_maps_to_choice_number() {
        let cs = [
            Choice {
                number: 1,
                bytes: 3..10,
            },
            Choice {
                number: 2,
                bytes: 31..34,
            },
        ];
        assert_eq!(choice_cluster_range(T, &cs[0]), 1..2);
        assert_eq!(choice_cluster_range(T, &cs[1]), 4..5);
        assert_eq!(choice_utf16_range(T, &cs[0]), 1..4);
        assert_eq!(choice_at(T, &cs, 2), Some(1)); // 🍣 の後半
        assert_eq!(choice_at(T, &cs, 3), Some(1)); // FE0F
        assert_eq!(choice_at(T, &cs, 13), Some(2));
        assert_eq!(choice_at(T, &cs, 5), None);
        assert_eq!(choice_at(T, &cs, 0), None);
    }
}
