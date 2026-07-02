//! `DefaultEncoding` の派生・構築・固定写像テスト（タスク 2.1）。

use super::model::DefaultEncoding;

/// 構築と最小派生（Clone / Copy / Debug / PartialEq / Eq）を検証する。
#[test]
fn default_encoding_construction_and_derives() {
    let ansi = DefaultEncoding::Ansi;
    let utf8 = DefaultEncoding::Utf8;

    // Copy（ムーブ後も元が使える）。
    let ansi_copy = ansi;
    let utf8_copy = utf8;

    // Clone。
    #[allow(clippy::clone_on_copy)]
    let ansi_clone = ansi.clone();

    // PartialEq / Eq。
    assert_eq!(ansi, ansi_copy);
    assert_eq!(ansi, ansi_clone);
    assert_eq!(utf8, utf8_copy);
    assert_ne!(ansi, utf8);

    // Debug（variant 名を含む）。
    assert_eq!(format!("{ansi:?}"), "Ansi");
    assert_eq!(format!("{utf8:?}"), "Utf8");
}

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

/// 写像は決定的（同一入力は同一実体）。
#[test]
fn to_encoding_is_deterministic() {
    assert!(std::ptr::eq(
        DefaultEncoding::Ansi.to_encoding(),
        DefaultEncoding::Ansi.to_encoding(),
    ));
    assert!(std::ptr::eq(
        DefaultEncoding::Utf8.to_encoding(),
        DefaultEncoding::Utf8.to_encoding(),
    ));
    // 2 値は別実体へ写像される。
    assert!(!std::ptr::eq(
        DefaultEncoding::Ansi.to_encoding(),
        DefaultEncoding::Utf8.to_encoding(),
    ));
}
