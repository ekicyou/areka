//! `hash.rs` の在中テスト。
//!
//! 守るのは 5 つ。⑴ 公表テストベクタ 3 本との逐語一致（設計 D-1。自前実装の較正は
//! これが本体で、ここが緑でなければ他の性質は意味を持たない）。公表ベクタは 3 本とも
//! ASCII なので、多バイト入力の期待値 3 本を別に釘付けにして「入力を UTF-8 の
//! バイト列として読む」ことまで守る。⑵ 返り値が常に 16 桁の 16 進小文字であること——
//! 上位の桁が 0 でも 16 文字であること（0 詰め）。
//! ⑶ 同じ入力なら常に同じ値（カタログに焼き込む値の前提）。⑷ 1 文字違えば別の値
//! （要件 8.2 の「比較だけで変更を判定する」がこれに乗る）。⑸ 算法の名前が設計の
//! 綴りどおりであること（カタログ冒頭に記録され、切り替えの検出に使われる）。
//!
//! スナップショットにもファイルにも触らない（要件 6.2）。すべて文字列だけで完結する。

use super::*;

// ---- 公表テストベクタ（設計 D-1）----

#[test]
fn published_vector_empty_string() {
    assert_eq!(content_hash(""), "cbf29ce484222325");
}

#[test]
fn published_vector_single_a() {
    assert_eq!(content_hash("a"), "af63dc4c8601ec8c");
}

#[test]
fn published_vector_foobar() {
    assert_eq!(content_hash("foobar"), "85944171f73967e8");
}

// ---- 多バイト文字の逐語一致（入力を UTF-8 の「バイト列」として読むことの担保）----
//
// 公表テストベクタは 3 本とも ASCII なので、入力をバイト列で読んでも符号位置で読んでも
// 同じ値になり、両者を区別できない。カタログに載るのは日本語の ukadoc 本文であり、
// 要件 8.2 は本文の変更判定をハッシュの等値だけに預けている。符号位置読みに変わると
// 非 ASCII の行だけが黙って別の値になるので、多バイト入力の期待値をここで釘付けにする。
// 下の 3 本の値は Rust 実装とは別に、公表定数（10 進で書いた初期値と乗数）から
// Python で独立に計算して一致を確認したものである。

#[test]
fn multibyte_vector_katakana() {
    assert_eq!(content_hash("さくらスクリプト"), "bffcaa6df1347f24");
}

#[test]
fn multibyte_vector_japanese_sentence() {
    assert_eq!(
        content_hash("ゴーストの表示位置を指定する。"),
        "536f23972df69aed"
    );
}

/// 長い入力の経路も同時に釘付けにする（4,000 文字＝12,000 バイト）。
#[test]
fn multibyte_vector_long_input() {
    assert_eq!(content_hash(&"あ".repeat(4_000)), "61bc3324d6a7a425");
}

// ---- 返り値の形（16 桁の 16 進小文字）----

/// カタログに並ぶのは日本語の見出しと本文なので、多バイト文字と長い文字列を含める。
fn shape_samples() -> Vec<String> {
    let mut samples: Vec<String> = vec![
        String::new(),
        "a".to_owned(),
        "foobar".to_owned(),
        r"\![get,property,ID]".to_owned(),
        "さくらスクリプト".to_owned(),
        "ゴーストの表示位置を指定する。".to_owned(),
        "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html".to_owned(),
    ];
    samples.push("あ".repeat(4_000));
    samples
}

#[test]
fn output_is_sixteen_lowercase_hex_digits() {
    for sample in shape_samples() {
        let got = content_hash(&sample);
        assert_eq!(
            got.len(),
            16,
            "16 文字でなければならない: {sample:?} → {got}"
        );
        assert!(
            got.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "16 進小文字だけでなければならない: {sample:?} → {got}"
        );
    }
}

/// 上位の桁が 0 になる入力を決定的に探し、それでも 16 文字であることを確かめる。
/// 0 詰めを忘れると `format!("{:x}")` は 15 文字を返すので、ここが赤になる。
#[test]
fn leading_zero_nibble_is_zero_padded() {
    let found = (0..10_000u32)
        .map(|i| format!("zero-probe-{i}"))
        .find(|s| content_hash(s).starts_with('0'));
    let probe = found.expect("上位の桁が 0 になる入力が 10,000 本の中に無かった");
    let got = content_hash(&probe);
    assert!(got.starts_with('0'), "探索の前提が崩れている: {got}");
    assert_eq!(got.len(), 16, "0 詰めされていない: {probe:?} → {got}");
}

// ---- 決定性 ----

#[test]
fn same_input_gives_same_hash() {
    for sample in shape_samples() {
        assert_eq!(content_hash(&sample), content_hash(&sample));
    }
}

// ---- 1 文字の違いを取り落とさないこと（要件 8.2）----

#[test]
fn one_byte_difference_changes_hash() {
    assert_ne!(content_hash("foobar"), content_hash("foobaR"));
    assert_ne!(content_hash("foobar"), content_hash("fooba"));
    assert_ne!(content_hash(""), content_hash("\0"));
}

#[test]
fn one_character_difference_in_japanese_changes_hash() {
    let before = "ゴーストの表示位置を指定する。";
    let after = "ゴーストの表示位置を指定した。";
    assert_ne!(content_hash(before), content_hash(after));
}

/// 並びの違いも取り落とさない（和で潰す実装だと同じ値になる）。
#[test]
fn byte_order_matters() {
    assert_ne!(content_hash("ab"), content_hash("ba"));
}

// ---- 算法の名前 ----

#[test]
fn algorithm_name_is_the_spelling_the_catalog_records() {
    assert_eq!(HASH_ALGORITHM, "fnv1a64");
}
