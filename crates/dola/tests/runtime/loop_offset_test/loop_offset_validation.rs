//! ループオフセットのバリデーション + NaN 境界特性化
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
    fn v16_range_inverted_error() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: TimeSpan::from_millis(5000),
                max: TimeSpan::from_millis(1000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        let errors = doc.validate().unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            DolaError::LoopOffsetRangeInverted { storyboard, min, max }
            if storyboard == "test_sb"
                && *min == TimeSpan::from_millis(5000)
                && *max == TimeSpan::from_millis(1000)
        )));
    }

    #[test]
    fn valid_scalar_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(TimeSpan::from_millis(3000))),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn valid_range_offset_ok() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: TimeSpan::from_millis(1000),
                max: TimeSpan::from_millis(5000),
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
                min: TimeSpan::from_millis(3000),
                max: TimeSpan::from_millis(3000),
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
            loop_offset: Some(LoopOffset::Scalar(TimeSpan::ZERO)),
            ..make_storyboard_with_offset(None)
        };
        let doc = doc_with_storyboard(sb);
        assert!(doc.validate().is_ok());
    }
}

// =============================================================
// 文書の値の上限（絶対値 2^51 刻み未満）
// =============================================================

mod document_range_tests {
    use super::*;

    const LIMIT: i64 = 1 << 51;

    fn span(ticks: i64) -> TimeSpan {
        TimeSpan::try_from_ticks(ticks).unwrap()
    }

    fn out_of_range_fields(sb: Storyboard) -> Vec<String> {
        doc_with_storyboard(sb)
            .validate()
            .unwrap_err()
            .into_iter()
            .map(|e| match e {
                DolaError::TimeOutOfDocumentRange { field, .. } => field,
                other => panic!("unexpected error: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn scalar_at_the_limit_edge() {
        let ok = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(span(LIMIT - 1))),
            ..make_storyboard_with_offset(None)
        };
        assert!(doc_with_storyboard(ok).validate().is_ok());
        let over = Storyboard {
            loop_offset: Some(LoopOffset::Scalar(span(LIMIT))),
            ..make_storyboard_with_offset(None)
        };
        assert_eq!(out_of_range_fields(over), ["loop_offset"]);
    }

    #[test]
    fn range_min_and_max_beyond_the_limit_are_both_reported() {
        let sb = Storyboard {
            loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
                min: span(LIMIT),
                max: span(LIMIT + 1),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
            ..make_storyboard_with_offset(None)
        };
        assert_eq!(
            out_of_range_fields(sb),
            ["loop_offset.min", "loop_offset.max"]
        );
    }
}
