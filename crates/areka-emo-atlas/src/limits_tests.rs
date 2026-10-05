//! `limits.rs` の兄弟テスト（spec: areka-P0-animated-image-decode 要件 6.1・6.2・6.9〜6.11）。
//!
//! プロセスの環境変数は書き換えない。値は偽の引く関数（名前 → 値）で渡す。

use std::env::VarError;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use super::*;

/// UTF-8 でない値（対になっていないサロゲート）。
fn not_unicode() -> VarError {
    VarError::NotUnicode(OsString::from_wide(&[0x0041, 0xD800]))
}

/// 3 つの名前それぞれに値を返す偽の引く関数（`None`＝未設定・`Some(None)`＝UTF-8 でない）。
fn lookup_of(
    frames: Option<Option<&'static str>>,
    pixels: Option<Option<&'static str>>,
    total: Option<Option<&'static str>>,
) -> impl Fn(&'static str) -> Result<String, VarError> {
    move |name| {
        let v = match name {
            MAX_FRAMES_ENV => frames,
            MAX_PIXELS_ENV => pixels,
            MAX_TOTAL_PIXELS_ENV => total,
            other => panic!("知らない名前を引いた: {other}"),
        };
        match v {
            None => Err(VarError::NotPresent),
            Some(None) => Err(not_unicode()),
            Some(Some(s)) => Ok(s.to_string()),
        }
    }
}

// ---- 名前と既定 ----

#[test]
fn names_and_defaults() {
    assert_eq!(MAX_FRAMES_ENV, "AREKA_ANIMATED_IMAGE_MAX_FRAMES");
    assert_eq!(MAX_PIXELS_ENV, "AREKA_ANIMATED_IMAGE_MAX_PIXELS");
    assert_eq!(
        MAX_TOTAL_PIXELS_ENV,
        "AREKA_ANIMATED_IMAGE_MAX_TOTAL_PIXELS"
    );
    assert_eq!(
        AnimationLimits::default(),
        AnimationLimits {
            max_frames: 1_024,
            max_pixels: 67_108_864,
            max_total_pixels: 268_435_456,
        }
    );
}

// ---- resolve_limit の 6 分岐 ----

#[test]
fn resolve_not_present_is_default_without_warning() {
    assert_eq!(resolve_limit("N", Err(VarError::NotPresent), 5), (5, None));
}

#[test]
fn resolve_positive_number_is_taken() {
    assert_eq!(resolve_limit("N", Ok("42".into()), 5), (42, None));
    // 前後の空白は除いて読む（port.rs の先例）。
    assert_eq!(resolve_limit("N", Ok(" 42 ".into()), 5), (42, None));
}

fn warned(name: &'static str, given: &str) -> Option<LimitWarning> {
    Some(LimitWarning {
        name,
        given: given.to_string(),
    })
}

#[test]
fn resolve_not_a_number_is_default_with_warning() {
    assert_eq!(
        resolve_limit("N", Ok("abc".into()), 5),
        (5, warned("N", "abc"))
    );
}

#[test]
fn resolve_zero_is_default_with_warning() {
    assert_eq!(resolve_limit("N", Ok("0".into()), 5), (5, warned("N", "0")));
}

#[test]
fn resolve_negative_is_default_with_warning() {
    assert_eq!(
        resolve_limit("N", Ok("-3".into()), 5),
        (5, warned("N", "-3"))
    );
}

#[test]
fn resolve_not_unicode_is_default_with_lossy_copy() {
    assert_eq!(
        resolve_limit("N", Err(not_unicode()), 5),
        (5, warned("N", "A\u{FFFD}"))
    );
}

// ---- from_lookup の割り当て ----

#[test]
fn from_lookup_assigns_each_name_to_its_field() {
    let (limits, warnings) = AnimationLimits::from_lookup(lookup_of(
        Some(Some("7")),
        Some(Some("11")),
        Some(Some("13")),
    ));
    assert_eq!(
        limits,
        AnimationLimits {
            max_frames: 7,
            max_pixels: 11,
            max_total_pixels: 13,
        }
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn from_lookup_one_broken_item_falls_back_alone() {
    let (limits, warnings) = AnimationLimits::from_lookup(lookup_of(
        Some(Some("7")),
        Some(Some("x")),
        Some(Some("13")),
    ));
    assert_eq!(
        limits,
        AnimationLimits {
            max_frames: 7,
            max_pixels: AnimationLimits::default().max_pixels,
            max_total_pixels: 13,
        }
    );
    assert_eq!(
        warnings,
        vec![LimitWarning {
            name: MAX_PIXELS_ENV,
            given: "x".into()
        }]
    );
}

#[test]
fn from_lookup_all_unset_is_default() {
    let (limits, warnings) = AnimationLimits::from_lookup(lookup_of(None, None, None));
    assert_eq!(limits, AnimationLimits::default());
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn from_lookup_not_unicode_falls_back_alone() {
    let (limits, warnings) =
        AnimationLimits::from_lookup(lookup_of(Some(Some("7")), Some(Some("11")), Some(None)));
    assert_eq!(
        limits,
        AnimationLimits {
            max_frames: 7,
            max_pixels: 11,
            max_total_pixels: AnimationLimits::default().max_total_pixels,
        }
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].name, MAX_TOTAL_PIXELS_ENV);
}

// ---- judge の 5 分岐とちょうど上限 ----

fn info(width: u32, height: u32, frame_count: u32) -> AnimationInfo {
    AnimationInfo {
        width,
        height,
        frame_count,
    }
}

/// 一辺 10・余白 1 のページ（絵の一辺は 8 まで入る）。
fn page() -> PackConfig {
    PackConfig {
        page_size: 10,
        padding: 1,
    }
}

/// 枚数 4・画素 4×8×8＝256・合計 1,000。
fn small() -> AnimationLimits {
    AnimationLimits {
        max_frames: 4,
        max_pixels: 256,
        max_total_pixels: 1_000,
    }
}

#[test]
fn judge_fits_returns_pixels() {
    assert_eq!(judge(info(2, 3, 2), small(), 0, page()), Ok(12));
}

#[test]
fn judge_exactly_at_every_limit_fits() {
    // 枚数 4（ちょうど）・画素 4×8×8＝256（ちょうど）・一辺 8＋2＝10（ちょうど）・合計 744＋256＝1,000（ちょうど）。
    assert_eq!(judge(info(8, 8, 4), small(), 744, page()), Ok(256));
}

#[test]
fn judge_frames_over() {
    assert_eq!(
        judge(info(1, 1, 5), small(), 0, page()),
        Err(Exceeded::Frames)
    );
}

#[test]
fn judge_pixels_over() {
    // 4×9×8＝288 > 256（一辺も超えるが、画素が先）。
    assert_eq!(
        judge(info(9, 8, 4), small(), 0, page()),
        Err(Exceeded::Pixels)
    );
}

#[test]
fn judge_page_side_over() {
    // 画素 2×9×1＝18 は収まるが、幅 9＋2＝11 > 10。高さの側も同じ。
    assert_eq!(
        judge(info(9, 1, 2), small(), 0, page()),
        Err(Exceeded::PageSide)
    );
    assert_eq!(
        judge(info(1, 9, 2), small(), 0, page()),
        Err(Exceeded::PageSide)
    );
}

#[test]
fn judge_total_over() {
    // 256 は絵 1 つには収まるが、745＋256＝1,001 > 1,000。
    assert_eq!(
        judge(info(8, 8, 4), small(), 745, page()),
        Err(Exceeded::TotalPixels)
    );
}

#[test]
fn judge_frames_checked_before_pixels() {
    assert_eq!(
        judge(info(100, 100, 5), small(), 0, page()),
        Err(Exceeded::Frames)
    );
}

#[test]
fn judge_page_side_checked_before_total() {
    // 幅 9＋2＝11 > 10 で、合計 1,000＋18 > 1,000 も超えるが、ページの一辺が先。
    assert_eq!(
        judge(info(9, 1, 2), small(), 1_000, page()),
        Err(Exceeded::PageSide)
    );
}

#[test]
fn judge_overflow_is_over() {
    let wide = AnimationLimits {
        max_frames: u64::MAX,
        max_pixels: u64::MAX,
        max_total_pixels: u64::MAX,
    };
    let huge_page = PackConfig {
        page_size: u32::MAX,
        padding: 0,
    };
    // 合計の足し算のあふれ（u64::MAX＋8）。
    assert_eq!(
        judge(info(2, 2, 2), wide, u64::MAX, huge_page),
        Err(Exceeded::TotalPixels)
    );
    // u32::MAX³ は u64 をあふれる。
    assert_eq!(
        judge(info(u32::MAX, u32::MAX, u32::MAX), wide, 0, huge_page),
        Err(Exceeded::Pixels)
    );
}
