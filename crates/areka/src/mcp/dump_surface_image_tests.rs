//! PNG を base64 にする決定論テスト（要件 5.1・5.5）。乗算を戻す・PNG の検査は `png_image_tests.rs`。

use super::*;

#[test]
fn base64_known_vectors() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    // 標準の文字集合の 62・63 番＝`+`・`/`（URL 用の `-`・`_` ではない）。
    assert_eq!(base64(&[0xFB, 0xEF, 0xBE]), "++++");
    assert_eq!(base64(&[0xFF, 0xFF, 0xFF]), "////");
    assert_eq!(base64(&[0xFB, 0xFF]), "+/8=");
}

#[test]
fn png_base64_has_no_line_breaks_for_large_image() {
    let s = png_base64(&[1u8, 2, 3, 4].repeat(64 * 64), 64, 64);
    assert!(!s.contains('\n') && !s.contains('\r'), "改行なし");
    assert_eq!(s.len() % 4, 0, "詰めありで 4 の倍数");
    assert!(s.starts_with("iVBORw0KGgo"), "PNG の署名の base64 で始まる");
}
