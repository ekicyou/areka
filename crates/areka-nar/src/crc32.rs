//! CRC-32（表引き）。zip の整合性検査と `.nar` の刻印の両方がこれ 1 本を使う。
//!
//! 変種は zip・gzip・PNG と同じ CRC-32/ISO-HDLC（反転多項式 `0xEDB88320`・
//! 初期値 `0xFFFFFFFF`・最終 XOR `0xFFFFFFFF`・入出力とも反転）。ほかの CRC-32
//! を取り違えると中央ディレクトリに刻まれた値と一致しないので、兄弟テストは
//! 外部の出どころを持つ較正値だけで判定する（要件 2.6）。
//!
//! ハッシュの crate は入れない（設計「Allowed Dependencies」）。

/// 反転多項式から組む 256 語の表。手写しではなくコンパイル時に計算するので、
/// 較正値が触らない行に写し間違いが潜むということが起こらない。
const TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let mut value = index as u32;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 1 == 0 {
                value >> 1
            } else {
                (value >> 1) ^ 0xEDB8_8320
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
};

/// バイト列の CRC-32 を返す。
///
/// 一括で受ける形 1 本だけを置く。コンテナの読み手はエントリの伸長済みバイト列を
/// 既に持っており、`.nar` の刻印もファイルを丸ごと読んだ後に取るので、
/// 分割して与える口は要らない。
///
/// ```
/// assert_eq!(areka_nar::crc32(b"123456789"), 0xCBF4_3926);
/// ```
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc = TABLE[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ u32::MAX
}

#[cfg(test)]
#[path = "crc32_tests.rs"]
mod tests;
