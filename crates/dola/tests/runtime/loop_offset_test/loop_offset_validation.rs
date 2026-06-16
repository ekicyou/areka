//! ループオフセットのバリデーション + NaN 境界特性化（Task 6.4 で分割）
//!
//! 共有ヘルパー `doc_with_storyboard` / `make_storyboard_with_offset` は
//! 親モジュール（mod.rs）に定義され、`super::*` 経由で参照する。
use super::*;

// =============================================================
// V14-V17: ループオフセットバリデーション
// =============================================================

mod validation_tests {
    use super::*;

    #[test]
    fn v14_negative_min_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: -1.0,
                max: 5.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetNegativeMin { storyboard, value }
            if storyboard == "test_sb" && *value == -1.0
        )));
    }

    #[test]
    fn v15_negative_max_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 0.0,
                max: -3.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetNegativeMax { storyboard, value }
            if storyboard == "test_sb" && *value == -3.0
        )));
    }

    #[test]
    fn v15_negative_scalar_error() {
        // スカラー短縮形の負値 → max が負 → V15
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(-2.0)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetNegativeMax { storyboard, value }
            if storyboard == "test_sb" && *value == -2.0
        )));
    }

    #[test]
    fn v16_range_inverted_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 5.0,
                max: 1.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetRangeInverted { storyboard, min, max }
            if storyboard == "test_sb" && *min == 5.0 && *max == 1.0
        )));
    }

    #[test]
    fn valid_scalar_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(3.0)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn valid_range_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 1.0,
                max: 5.0,
                easing: EasingFunction::Named(EasingName::QuadraticOut),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn no_offset_ok() {
        let sb = make_storyboard_with_offset(None);
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn min_equals_max_ok() {
        // min == max は合法（固定遅延）
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 3.0,
                max: 3.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn zero_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(0.0)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }
}

// =============================================================
// D3-V 特性化: NaN loop_offset のバリデーション素通り
// =============================================================

mod nan_boundary_tests {
    use super::*;

    #[test]
    fn nan_scalar_loop_offset_passes_validation() {
        // 特性化: Scalar(NaN) は V15（max < 0.0 = false）を素通りして validate() に
        // 合格する（数値フィールドの有限性検証の欠如 — P14 参照）。
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(f64::NAN)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn nan_range_max_passes_validation_even_with_positive_min() {
        // 特性化: max=NaN は V15（NaN < 0.0 = false）と V16（min > NaN = false）の
        // 双方を素通りするため、範囲の逆転・退化が検出されないまま合格する（P14 参照）。
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 5.0,
                max: f64::NAN,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn nan_range_min_passes_validation() {
        // 特性化: min=NaN も V14（NaN < 0.0 = false）・V16（NaN > max = false）を
        // 素通りして合格する（P14 参照）。
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: f64::NAN,
                max: 3.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }
}
