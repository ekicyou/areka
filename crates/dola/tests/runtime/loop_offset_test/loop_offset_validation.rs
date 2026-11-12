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
// #29: NaN の loop_offset は V14〜V16 を素通りするので、数値の規則が拒む
// =============================================================

mod nan_boundary_tests {
    use super::*;

    /// validate の誤りが InvalidNumber だけで、その欄の並びを返す
    fn invalid_number_fields(doc: &DolaDocument) -> Vec<String> {
        doc.validate()
            .unwrap_err()
            .into_iter()
            .map(|e| match e {
                DolaError::InvalidNumber { field, .. } => field,
                other => panic!("unexpected error: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn nan_scalar_loop_offset_is_rejected() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(f64::NAN)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert_eq!(invalid_number_fields(&doc), ["loop_offset"]);
    }

    #[test]
    fn nan_range_max_is_rejected_even_with_positive_min() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: 5.0,
                max: f64::NAN,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert_eq!(invalid_number_fields(&doc), ["loop_offset.max"]);
    }

    #[test]
    fn nan_range_min_is_rejected() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: f64::NAN,
                max: 3.0,
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert_eq!(invalid_number_fields(&doc), ["loop_offset.min"]);
    }
}
