//! `DefaultEncoding` の固定写像テスト（D6）。

use super::model::DefaultEncoding;

/// `Ansi` は CP932（Shift_JIS）へ固定写像される（D6・環境非依存）。
#[test]
fn ansi_maps_to_shift_jis() {
    let enc = DefaultEncoding::Ansi.to_encoding();
    // ラベル同一性。
    assert_eq!(enc.name(), "Shift_JIS");
    // 静的定数との実体同一性（ポインタ一致）。
    assert!(std::ptr::eq(enc, encoding_rs::SHIFT_JIS));
}

/// `Utf8` は UTF-8 へ固定写像される（D6・環境非依存）。
#[test]
fn utf8_maps_to_utf_8() {
    let enc = DefaultEncoding::Utf8.to_encoding();
    assert_eq!(enc.name(), "UTF-8");
    assert!(std::ptr::eq(enc, encoding_rs::UTF_8));
}
