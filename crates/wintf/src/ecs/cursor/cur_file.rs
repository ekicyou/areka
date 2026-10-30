//! `.cur` の目録（ICONDIR と ICONDIRENTRY）の読みと寸法の選び。Win32 を使わない純関数。

use std::ops::Range;

/// `.cur` の目録の 1 項目。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurEntry {
    /// 幅（px）。目録の 0 は 256。
    pub width: u32,
    /// 高さ（px）。目録の 0 は 256。
    pub height: u32,
    /// 当たりの点（x, y）。
    pub hotspot: (u16, u16),
    /// 絵の本体（BMP の DIB か PNG）のファイルの中の範囲。
    pub data: Range<usize>,
}

/// `.cur` の目録が壊れている理由。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CurFileError {
    #[error("目録が短い（{len} バイト）")]
    Short { len: usize },
    #[error("型が 2（カーソル）でない: {kind}")]
    NotCursor { kind: u16 },
    #[error("項目が 1 つも無い")]
    Empty,
    #[error("項目 {index} の絵がファイルの範囲の外を指す")]
    OutOfRange { index: usize },
}

/// 目録の頭（予約・型・数の 3 語）と項目（16 バイトずつ）を読む。どの項目の絵もファイルの
/// 範囲の中に在ることを確かめる。
pub fn read_cur_directory(bytes: &[u8]) -> Result<Vec<CurEntry>, CurFileError> {
    let short = CurFileError::Short { len: bytes.len() };
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    if bytes.len() < 6 {
        return Err(short);
    }
    let kind = u16_at(2);
    if kind != 2 {
        return Err(CurFileError::NotCursor { kind });
    }
    let count = usize::from(u16_at(4));
    if count == 0 {
        return Err(CurFileError::Empty);
    }
    if bytes.len() < 6 + 16 * count {
        return Err(short);
    }
    let side = |b: u8| if b == 0 { 256 } else { u32::from(b) };
    (0..count)
        .map(|index| {
            let at = 6 + 16 * index;
            let len = u32_at(at + 8) as usize;
            let offset = u32_at(at + 12) as usize;
            let end = offset
                .checked_add(len)
                .filter(|&end| end <= bytes.len())
                .ok_or(CurFileError::OutOfRange { index })?;
            Ok(CurEntry {
                width: side(bytes[at]),
                height: side(bytes[at + 1]),
                hotspot: (u16_at(at + 4), u16_at(at + 6)),
                data: offset..end,
            })
        })
        .collect()
}

/// `size` 以上で最も小さい項目、無ければ最も大きい項目。
pub fn pick_cur_entry(entries: &[CurEntry], size: u32) -> Option<&CurEntry> {
    entries
        .iter()
        .filter(|e| e.width >= size)
        .min_by_key(|e| e.width)
        .or_else(|| entries.iter().max_by_key(|e| e.width))
}

#[cfg(test)]
#[path = "cur_file_tests.rs"]
mod cur_file_tests;
