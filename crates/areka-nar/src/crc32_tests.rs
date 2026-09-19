//! `crc32` の兄弟テスト。較正値はすべて**外部の出どころ**を持つ（要件 2.6）。
//!
//! 自前の実装で作った値を期待値に書くと、実装が間違っていても緑になる。
//! そこで期待値は ⑴ CRC の公表目録の検査値、⑵ 広く公表されている固定文、
//! ⑶ このリポジトリが持つ実在の zip を 2012 年の別の実装が刻んだ値、
//! の 3 系統から採る。出どころは各テストの説明に書く。

use super::*;

/// CRC 目録の検査値。変種の識別子そのもの。
///
/// 出どころ: CRC の公表目録（CRC-32/ISO-HDLC。別名 CRC-32/ADCCP・PKZIP・
/// gzip・PNG が使うもの）の `check` 欄＝ ASCII 文字列 `123456789` に対する値。
/// 反転多項式 `0xEDB88320`・初期値 `0xFFFFFFFF`・最終 XOR `0xFFFFFFFF`。
/// 変種を取り違えるとこの値だけは必ず食い違うので、較正の要になる。
#[test]
fn catalogue_check_value_identifies_the_iso_hdlc_variant() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
}

/// 空の入力は 0。
///
/// 出どころ: zip の仕様が空エントリ（長さ 0 のフォルダ項目）に刻む値であり、
/// 同じものを下の実在アーカイブの先頭エントリ `2/`（長さ 0）が持っている。
#[test]
fn empty_input_is_zero() {
    assert_eq!(crc32(b""), 0);
}

/// 広く公表されている固定文。43 バイトあるので表の引き先が広く散る。
///
/// 出どころ: CRC-32 の解説で定番の例文として公表されている値。
#[test]
fn published_pangram_vector() {
    assert_eq!(
        crc32(b"The quick brown fox jumps over the lazy dog"),
        0x414F_A339
    );
}

/// このリポジトリが持つ実在の zip に、**別の実装が 2012 年に刻んだ** CRC と突き合わせる。
///
/// `crates/areka/shell/org/2.zip` は areka のシェル素材で、無圧縮（方式 0）の
/// エントリを 8 つ持つ。無圧縮なら局所ヘッダの直後のバイト列がそのまま中身なので、
/// zip の読み手（タスク 3.1）を待たずに数行で取り出せる。期待値は我々が作った
/// ものではなく、配布物に刻まれている値である点がこのテストの値打ち。
#[test]
fn matches_the_crc_stamped_by_a_third_party_encoder_in_a_real_archive() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../areka/shell/org/2.zip"
    ))
    .expect("実在のシェル素材 zip を読めた");

    let mut checked = Vec::new();
    let mut at = 0usize;
    // 局所ヘッダ（`PK\x03\x04`）は先頭から順に並び、各エントリの中身がその直後に続く。
    while at + 30 <= bytes.len() && bytes[at..at + 4] == *b"PK\x03\x04" {
        let u16_at = |o: usize| u16::from_le_bytes([bytes[at + o], bytes[at + o + 1]]) as usize;
        let u32_at = |o: usize| {
            u32::from_le_bytes([
                bytes[at + o],
                bytes[at + o + 1],
                bytes[at + o + 2],
                bytes[at + o + 3],
            ])
        };
        let (flags, method, declared, compressed) = (u16_at(6), u16_at(8), u32_at(14), u32_at(18));
        let head = 30 + u16_at(26) + u16_at(28);
        // ビット 3（データ記述子）が立っていると局所ヘッダの長さと CRC は 0 なので見ない。
        assert_eq!(flags & 0x0008, 0, "この検体はデータ記述子を使わない");
        // 方式 8（deflate）のエントリは伸長の担当（タスク 3.1）に任せ、ここでは飛ばす。
        if method == 0 {
            let body = &bytes[at + head..at + head + compressed as usize];
            checked.push((crc32(body), declared));
        }
        at += head + compressed as usize;
    }

    // 中身のあるエントリが複数採れていなければ、上の走査が空回りしている（母数 0 の緑）。
    let with_content = checked.iter().filter(|(_, d)| *d != 0).count();
    assert!(
        with_content >= 2,
        "中身のあるエントリを 2 つ以上検査したはず: {checked:?}"
    );
    for (computed, declared) in &checked {
        assert_eq!(computed, declared, "刻まれた CRC と一致する: {checked:?}");
    }
}
