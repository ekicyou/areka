//! load / start / calculate_end_time（research.md §R3: load_start シーム）
//!
//! フル再生サイクル・start/calculate_end_time のエラーパス・time_scale 境界の
//! 特性化テストを束ねる。

// =========================================================================
// 8.1 フル再生サイクルテスト
// =========================================================================
mod full_playback_cycle {
    use super::super::*;

    #[test]
    fn load_start_update_natural_end() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();

        let StartResult {
            group_id, end_time, ..
        } = rt.start("fade_in", 0.0).unwrap();
        assert_eq!(group_id, 1);
        assert!((end_time - 1.0).abs() < 1e-9);

        // t=0.0: 初回 update — opacity=0.0
        let diff = rt.update(0.0).changes;
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].0, opacity_id);
        match &diff[0].1 {
            EvaluatedValue::Float(v) => assert!(*v < 0.01, "expected ~0.0, got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // t=0.5: 中間値 — opacity≈0.5
        let diff = rt.update(0.5).changes;
        assert_eq!(diff.len(), 1);
        match &diff[0].1 {
            EvaluatedValue::Float(v) => assert!((*v - 0.5).abs() < 0.05, "expected ~0.5, got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // t=1.0: 最終値 — opacity=1.0, 自然終了トリガー
        let diff = rt.update(1.0).changes;
        // 自然終了により conclude_internal が呼ばれ、最終値が配信される
        assert!(!diff.is_empty(), "expected final value delivery at t=1.0");
        let opacity_val = diff
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .expect("opacity");
        match &opacity_val.1 {
            EvaluatedValue::Float(v) => assert!((*v - 1.0).abs() < 0.01, "expected ~1.0, got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // t=1.5: 終了後 — 差分なし
        let diff = rt.update(1.5).changes;
        assert!(diff.is_empty(), "expected empty diff after natural end");
    }

    #[test]
    fn storyboard_not_found_error() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();

        let err = rt.start("nonexistent", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::StoryboardNotFound(_)));
    }

    #[test]
    fn start_without_document_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.start("fade_in", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::StoryboardNotFound(_)));
    }

    #[test]
    fn calculate_end_time_does_not_create_instance() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();

        let end_time = rt.calculate_end_time("fade_in", 0.0).unwrap();
        assert!((end_time - 1.0).abs() < 1e-9);

        // subscribe して update しても差分なし（インスタンス未生成）
        let _opacity_id = rt.subscribe("opacity");
        let diff = rt.update(0.5).changes;
        assert!(diff.is_empty());
    }
}

// =========================================================================
// start / calculate_end_time エラーパステスト
// （InvalidLoopCount / ZeroDurationWithLoop / TooShortDurationWithInfiniteLoop）
// =========================================================================
mod start_error_paths {
    use super::super::*;
    use dola::{InterruptionPolicy, Storyboard, StoryboardBuilder};

    /// entry が空（total_base_duration=0）のストーリーボードを持つドキュメント。
    fn empty_entry_doc(sb_name: &str, loop_count: i32) -> DolaDocument {
        let sb = Storyboard {
            time_scale: 1.0,
            loop_count,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset: None,
            entry: vec![],
        };
        let mut storyboard = BTreeMap::new();
        storyboard.insert(sb_name.to_string(), sb);
        DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    /// loop_count を任意指定した 1 変数ドキュメント（duration 指定可）。
    fn loop_count_doc(sb_name: &str, loop_count: i32, duration: f64) -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        );
        let sb = StoryboardBuilder::new()
            .loop_count(loop_count)
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(duration),
                })),
                ..Default::default()
            })
            .build();
        let mut storyboard = BTreeMap::new();
        storyboard.insert(sb_name.to_string(), sb);
        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    #[test]
    fn start_with_zero_loop_count_fails() {
        let mut rt = DolaRuntime::new();
        rt.load_document(loop_count_doc("fade", 0, 1.0)).unwrap();
        let err = rt.start("fade", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidLoopCount(0)));
    }

    #[test]
    fn start_with_negative_loop_count_fails() {
        // -1（無限）以外の負値は無効
        let mut rt = DolaRuntime::new();
        rt.load_document(loop_count_doc("fade", -2, 1.0)).unwrap();
        let err = rt.start("fade", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidLoopCount(-2)));
    }

    #[test]
    fn start_zero_duration_infinite_loop_fails() {
        // entry 空 → total_base_duration=0、loop_count=-1 → ZeroDurationWithLoop
        let mut rt = DolaRuntime::new();
        rt.load_document(empty_entry_doc("empty_sb", -1)).unwrap();
        let err = rt.start("empty_sb", 0.0).unwrap_err();
        assert!(
            matches!(err, RuntimeError::ZeroDurationWithLoop { ref storyboard } if storyboard == "empty_sb"),
            "expected ZeroDurationWithLoop, got {err:?}"
        );
    }

    #[test]
    fn zero_duration_without_loop_is_allowed() {
        // entry 空でも loop_count=1 なら開始可能（end_time = start_time）
        let mut rt = DolaRuntime::new();
        rt.load_document(empty_entry_doc("empty_sb", 1)).unwrap();
        let result = rt.start("empty_sb", 2.0).unwrap();
        assert!((result.end_time - 2.0).abs() < 1e-9);
    }

    #[test]
    fn calculate_end_time_without_document_fails() {
        let rt = DolaRuntime::new();
        let err = rt.calculate_end_time("fade", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::StoryboardNotFound(_)));
    }

    #[test]
    fn calculate_end_time_nonexistent_storyboard_fails() {
        let mut rt = DolaRuntime::new();
        rt.load_document(loop_count_doc("fade", 1, 1.0)).unwrap();
        let err = rt.calculate_end_time("nonexistent", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::StoryboardNotFound(_)));
    }

    #[test]
    fn calculate_end_time_with_zero_loop_count_fails() {
        let mut rt = DolaRuntime::new();
        rt.load_document(loop_count_doc("fade", 0, 1.0)).unwrap();
        let err = rt.calculate_end_time("fade", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidLoopCount(0)));
    }

    #[test]
    fn calculate_end_time_zero_duration_infinite_loop_fails() {
        let mut rt = DolaRuntime::new();
        rt.load_document(empty_entry_doc("empty_sb", -1)).unwrap();
        let err = rt.calculate_end_time("empty_sb", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::ZeroDurationWithLoop { .. }));
    }

    #[test]
    fn calculate_end_time_short_duration_infinite_loop_fails() {
        // duration=0.05 < MIN_LOOP_DURATION(0.1) かつ loop_count=-1
        let mut rt = DolaRuntime::new();
        rt.load_document(loop_count_doc("fade", -1, 0.05)).unwrap();
        let err = rt.calculate_end_time("fade", 0.0).unwrap_err();
        assert!(matches!(
            err,
            RuntimeError::TooShortDurationWithInfiniteLoop { .. }
        ));
    }

    #[test]
    fn pause_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.pause(9999, 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn resume_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.resume(9999, 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }
}

// =========================================================================
// D1a-V: time_scale 境界の特性化テスト（現行挙動の固定）
//
// time_scale はスキーマ/コンパイル時に正値検証されていない
// （facade::compile_and_validate の NOTE / report/proposals.md P8 参照）。
// 本モジュールは「検証追加（挙動変更）前の現行挙動」を回帰検知器として固定する。
// P8 実装時はこれらのテストを新しいエラー仕様へ置き換えること。
// =========================================================================
mod time_scale_boundary {
    use super::super::*;
    use dola::{InterruptionPolicy, Storyboard};

    /// time_scale を任意指定した 1 変数ドキュメント（duration 指定可）。
    fn time_scale_doc(sb_name: &str, time_scale: f64, duration: f64) -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        );
        let sb = StoryboardBuilder::new()
            .time_scale(time_scale)
            .entry(StoryboardEntry {
                variable: Some("opacity".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    relative_to: None,
                    easing: None,
                    delay: 0.0,
                    duration: Some(duration),
                })),
                ..Default::default()
            })
            .build();
        let mut storyboard = BTreeMap::new();
        storyboard.insert(sb_name.to_string(), sb);
        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    /// entry 空（total_base_duration=0）+ time_scale 任意指定のドキュメント。
    fn empty_entry_time_scale_doc(sb_name: &str, time_scale: f64) -> DolaDocument {
        let sb = Storyboard {
            time_scale,
            loop_count: 1,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset: None,
            entry: vec![],
        };
        let mut storyboard = BTreeMap::new();
        storyboard.insert(sb_name.to_string(), sb);
        DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    #[test]
    fn time_scale_zero_start_succeeds_with_infinite_end_time() {
        // 現行挙動: time_scale=0 は検証されず start は成功し、
        // loop_duration = duration / 0 = +inf → end_time = +inf となる
        let mut rt = DolaRuntime::new();
        rt.load_document(time_scale_doc("fade", 0.0, 1.0)).unwrap();
        let result = rt.start("fade", 0.0).unwrap();
        assert!(
            result.end_time.is_infinite() && result.end_time > 0.0,
            "expected +inf end_time, got {}",
            result.end_time
        );
    }

    #[test]
    fn time_scale_zero_instance_never_concludes_naturally() {
        // 現行挙動: end_time = +inf のため自然終了せず、巨大な経過時刻後も
        // インスタンスは生存し続ける（pause が成功することで観測）
        let mut rt = DolaRuntime::new();
        rt.subscribe("opacity");
        rt.load_document(time_scale_doc("fade", 0.0, 1.0)).unwrap();
        let result = rt.start("fade", 0.0).unwrap();
        rt.update(1e9);
        assert!(
            rt.pause(result.group_id, 1e9).is_ok(),
            "instance should still be alive after huge time advance"
        );
    }

    #[test]
    fn calculate_end_time_with_zero_time_scale_is_infinite() {
        let mut rt = DolaRuntime::new();
        rt.load_document(time_scale_doc("fade", 0.0, 1.0)).unwrap();
        let end_time = rt.calculate_end_time("fade", 5.0).unwrap();
        assert!(
            end_time.is_infinite() && end_time > 0.0,
            "expected +inf end_time, got {end_time}"
        );
    }

    #[test]
    fn time_scale_zero_with_zero_duration_yields_nan_end_time() {
        // 現行挙動: total_base_duration=0 かつ time_scale=0 → 0/0 = NaN。
        // NaN はバリデーション（== 0.0 / < MIN）を素通りし end_time = NaN となる
        let mut rt = DolaRuntime::new();
        rt.load_document(empty_entry_time_scale_doc("empty_sb", 0.0))
            .unwrap();
        let result = rt.start("empty_sb", 0.0).unwrap();
        assert!(
            result.end_time.is_nan(),
            "expected NaN end_time, got {}",
            result.end_time
        );
    }

    #[test]
    fn negative_time_scale_start_succeeds_and_concludes_immediately() {
        // 現行挙動: time_scale=-1 → loop_duration=-1 → end_time = start_time - 1.0。
        // 最初の update で current_time >= end_time となり即 Conclude される
        let mut rt = DolaRuntime::new();
        rt.load_document(time_scale_doc("fade", -1.0, 1.0)).unwrap();
        let result = rt.start("fade", 0.0).unwrap();
        assert!((result.end_time - (-1.0)).abs() < 1e-9);
        rt.update(0.0);
        // Conclude 済み → pause は InvalidGroupId
        let err = rt.pause(result.group_id, 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }
}
