//! `image` のテスト（正しい `.cur` から作れる・壊れたバイト列は作らずに断る）。

use super::*;

/// 1 寸法ぶんの DIB（BITMAPINFOHEADER・32bpp の色・1bpp の抜き）。高さは色と抜きの 2 倍で書く。
fn dib(side: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(side as i32).to_le_bytes());
    b.extend_from_slice(&(2 * side as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    b.extend(std::iter::repeat_n(0xFFu8, (side * side * 4) as usize));
    let mask_row = side.div_ceil(32) * 4;
    b.extend(std::iter::repeat_n(0u8, (mask_row * side) as usize));
    b
}

/// 32 と 64 の 2 項目を持つ正しい `.cur`（当たりの点は (1, 1)）。
fn two_size_cur() -> Vec<u8> {
    let images = [dib(32), dib(64)];
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 2, 0, 2, 0]);
    let mut offset = 6 + 16 * images.len();
    for (side, image) in [32u8, 64].iter().zip(&images) {
        out.extend_from_slice(&[*side, *side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&(image.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in &images {
        out.extend_from_slice(image);
    }
    out
}

#[test]
fn creates_two_size_cur_at_32_and_64() {
    let bytes = two_size_cur();
    for size in [32, 64] {
        let cursor = OwnedCursor::from_cur_bytes(&bytes, size).expect("正しい .cur");
        assert!(!cursor.handle().is_invalid());
    }
}

#[test]
fn broken_bytes_are_refused_before_win32() {
    let err = OwnedCursor::from_cur_bytes(&[0, 0, 1, 0], 32).unwrap_err();
    assert!(matches!(
        err,
        CursorError::CurFile(cur_file::CurFileError::Short { len: 4 })
    ));
}

#[test]
fn missing_file_is_load_error() {
    let err = OwnedCursor::load_file(Path::new("no-such-cursor.cur")).unwrap_err();
    assert!(matches!(err, CursorError::Load { .. }));
}
