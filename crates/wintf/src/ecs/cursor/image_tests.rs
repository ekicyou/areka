//! `image` のテスト（同梱の `.cur` から作れる・壊れたバイト列は作らずに断る）。

use super::*;

const HAND: &[u8] = include_bytes!("../../../../areka/cursor/hand.cur");

#[test]
fn creates_bundled_hand_at_32_and_64() {
    for size in [32, 64] {
        let cursor = OwnedCursor::from_cur_bytes(HAND, size).expect("同梱の hand.cur");
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
