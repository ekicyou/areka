//! ブレンドの描き方の判定と 1 度だけの記録（装置なしの純粋なテスト・要件 1.1, 1.2, 1.5, 1.7, 7.9）。

use super::super::MixMode;
use super::emit::{DrawMethod, draw_method, report_once};
use std::sync::atomic::AtomicBool;
use windows::Win32::Graphics::Direct2D::Common::*;

#[test]
fn composite_in_range_draws_image_regardless_of_blend() {
    for c in [0, 12] {
        let mode = D2D1_COMPOSITE_MODE(c);
        for available in [true, false] {
            assert_eq!(
                draw_method(MixMode::Composite(mode), available),
                DrawMethod::Image(mode)
            );
        }
    }
}

#[test]
fn composite_out_of_range() {
    for c in [-1, 13] {
        for available in [true, false] {
            assert_eq!(
                draw_method(MixMode::Composite(D2D1_COMPOSITE_MODE(c)), available),
                DrawMethod::OutOfRange
            );
        }
    }
}

#[test]
fn blend_in_range_draws_blend_when_available() {
    for b in [0, 25] {
        let mode = D2D1_BLEND_MODE(b);
        assert_eq!(
            draw_method(MixMode::Blend(mode), true),
            DrawMethod::Blend(mode)
        );
        assert_eq!(
            draw_method(MixMode::Blend(mode), false),
            DrawMethod::BlendUnavailable
        );
    }
}

#[test]
fn blend_out_of_range_is_checked_before_availability() {
    for b in [-1, 26] {
        for available in [true, false] {
            assert_eq!(
                draw_method(MixMode::Blend(D2D1_BLEND_MODE(b)), available),
                DrawMethod::OutOfRange
            );
        }
    }
}

#[test]
fn report_once_is_true_only_the_first_time() {
    let flag = AtomicBool::new(false);
    let other = AtomicBool::new(false);
    assert!(report_once(&flag));
    assert!(!report_once(&flag));
    assert!(!report_once(&flag));
    // 旗が違えば別の種類の誤りとして初めての 1 度になる。
    assert!(report_once(&other));
}
