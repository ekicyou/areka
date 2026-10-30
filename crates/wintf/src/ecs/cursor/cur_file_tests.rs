//! `cur_file` の決定論テスト（目録の頭と項目の読み・壊れた目録・寸法の選び）。

use super::*;

/// 頭（予約・型・数）と項目（幅・高さ・当たりの点・絵の長さ）から目録を組み、絵の本体を
/// 項目の順に後ろへ並べる。絵の本体は長さぶんの 0xAA。
fn build(kind: u16, items: &[(u8, u8, u16, u16, u32)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&(items.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * items.len() as u32;
    for &(w, h, hx, hy, len) in items {
        out.extend_from_slice(&[w, h, 0, 0]);
        out.extend_from_slice(&hx.to_le_bytes());
        out.extend_from_slice(&hy.to_le_bytes());
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += len;
    }
    for &(.., len) in items {
        out.extend(std::iter::repeat_n(0xAA, len as usize));
    }
    out
}

fn entry(size: u32) -> CurEntry {
    CurEntry {
        width: size,
        height: size,
        hotspot: (0, 0),
        data: 0..0,
    }
}

#[test]
fn reads_header_and_entries() {
    let bytes = build(2, &[(32, 32, 5, 7, 10), (0, 0, 40, 41, 3)]);
    let entries = read_cur_directory(&bytes).expect("正しい目録");
    assert_eq!(
        entries,
        vec![
            CurEntry {
                width: 32,
                height: 32,
                hotspot: (5, 7),
                data: 38..48
            },
            CurEntry {
                width: 256,
                height: 256,
                hotspot: (40, 41),
                data: 48..51
            },
        ]
    );
}

#[test]
fn short_header_is_broken() {
    assert_eq!(
        read_cur_directory(&[0, 0, 2, 0]),
        Err(CurFileError::Short { len: 4 })
    );
}

#[test]
fn short_directory_is_broken() {
    let mut bytes = build(2, &[(32, 32, 0, 0, 4)]);
    bytes.truncate(6 + 15);
    assert_eq!(
        read_cur_directory(&bytes),
        Err(CurFileError::Short { len: 21 })
    );
}

#[test]
fn icon_type_is_broken() {
    let bytes = build(1, &[(32, 32, 0, 0, 4)]);
    assert_eq!(
        read_cur_directory(&bytes),
        Err(CurFileError::NotCursor { kind: 1 })
    );
}

#[test]
fn no_entries_is_broken() {
    assert_eq!(read_cur_directory(&build(2, &[])), Err(CurFileError::Empty));
}

#[test]
fn data_out_of_range_is_broken() {
    let mut bytes = build(2, &[(32, 32, 0, 0, 4), (48, 48, 0, 0, 4)]);
    bytes.pop();
    assert_eq!(
        read_cur_directory(&bytes),
        Err(CurFileError::OutOfRange { index: 1 })
    );
}

#[test]
fn offset_overflow_is_out_of_range() {
    let mut bytes = build(2, &[(32, 32, 0, 0, 4)]);
    // 絵の長さを u32 の最大にして、位置 + 長さが溢れる形にする。
    bytes[6 + 8..6 + 12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        read_cur_directory(&bytes),
        Err(CurFileError::OutOfRange { index: 0 })
    );
}

#[test]
fn pick_exact_size() {
    let entries = [entry(32), entry(48), entry(64)];
    assert_eq!(pick_cur_entry(&entries, 48).map(|e| e.width), Some(48));
}

#[test]
fn pick_smallest_above_between_sizes() {
    let entries = [entry(64), entry(32), entry(48)];
    assert_eq!(pick_cur_entry(&entries, 40).map(|e| e.width), Some(48));
}

#[test]
fn pick_largest_when_size_exceeds_max() {
    let entries = [entry(32), entry(128), entry(64)];
    assert_eq!(pick_cur_entry(&entries, 200).map(|e| e.width), Some(128));
}

#[test]
fn pick_from_empty_is_none() {
    assert_eq!(pick_cur_entry(&[], 32), None);
}
