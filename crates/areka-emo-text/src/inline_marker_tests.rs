use std::sync::Arc;

use areka_emo_present::balloon_marker::MarkerImage;
use areka_sakura::contract::{CueCommand, FONT_TAG_CARRIER};

use super::*;

fn cell(left: f32, top: f32, right: f32, bottom: f32) -> CellRect {
    CellRect {
        left,
        top,
        right,
        bottom,
    }
}

#[test]
fn marker_glyph_and_stand_in_are_one_utf16_unit_each() {
    assert_eq!(MARKER_GLYPH, "\u{FFFC}");
    assert_eq!(MARKER_STAND_IN, "\u{3000}");
    assert_eq!(MARKER_GLYPH.encode_utf16().count(), 1);
    assert_eq!(MARKER_STAND_IN.encode_utf16().count(), 1);
}

#[test]
fn stand_in_replaces_only_the_marker_glyph() {
    assert_eq!(stand_in(MARKER_GLYPH), "\u{3000}");
    assert_eq!(stand_in("あ"), "あ");
    assert_eq!(stand_in("\u{3000}"), "\u{3000}");
    assert_eq!(stand_in(""), "");
    // 目印の字を含むだけの別の字は替えない（目印の字は 1 クラスタとして積まれる）。
    assert_eq!(stand_in("\u{FFFC}a"), "\u{FFFC}a");
}

#[test]
fn is_marker_carrier_sees_only_the_marker_command() {
    assert!(is_marker_carrier(&CueCommand::command_carrier("*", vec![])));
    assert!(!is_marker_carrier(&CueCommand::command_carrier(
        FONT_TAG_CARRIER,
        vec!["color".into(), "red".into()],
    )));
    assert!(!is_marker_carrier(&CueCommand::command_carrier(
        "bind",
        vec![]
    )));
    assert!(!is_marker_carrier(&CueCommand::Text("*".into())));
}

#[test]
fn dot_is_half_the_font_size_at_the_cell_center() {
    // 字の大きさ 20 で直径 10（SSP の測り）。
    let (center, radius) = dot_in_cell(cell(10.0, 30.0, 30.0, 56.0), 20.0);
    assert_eq!(center, (20.0, 43.0));
    assert_eq!(radius * 2.0, 10.0);
    assert_eq!(DOT_OPACITY, 0.64);
}

#[test]
fn small_image_is_placed_at_native_size_in_the_center() {
    let r = image_in_cell(cell(0.0, 0.0, 20.0, 26.0), 8, 6);
    assert_eq!(r, cell(6.0, 10.0, 14.0, 16.0));
}

#[test]
fn large_image_shrinks_keeping_aspect_ratio() {
    // 40x20 の絵を 20x26 の枠へ: 幅で決まる 0.5 倍＝20x10、縦は真ん中。
    let r = image_in_cell(cell(0.0, 0.0, 20.0, 26.0), 40, 20);
    assert_eq!(r, cell(0.0, 8.0, 20.0, 18.0));
    // 高さではみ出す絵は高さで決まる。
    let r = image_in_cell(cell(0.0, 0.0, 20.0, 20.0), 10, 40);
    assert_eq!(r, cell(7.5, 0.0, 12.5, 20.0));
}

#[test]
fn look_defaults_to_dot_and_compares_images() {
    assert_eq!(InlineMarkerLook::default(), InlineMarkerLook::Dot);
    let img = Arc::new(MarkerImage {
        width: 1,
        height: 1,
        bgra: vec![0, 0, 0, 255],
    });
    let look = InlineMarkerLook::Image(img.clone());
    assert_eq!(look.clone(), InlineMarkerLook::Image(img));
    assert_ne!(look, InlineMarkerLook::Dot);
}
