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
            EvaluatedValue::Float(v) => assert!(v.abs() < 1e-9, "expected ~0.0, got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // t=0.5: 中間値 — opacity≈0.5
        let diff = rt.update(0.5).changes;
        assert_eq!(diff.len(), 1);
        match &diff[0].1 {
            EvaluatedValue::Float(v) => assert!((*v - 0.5).abs() < 1e-9, "expected ~0.5, got {v}"),
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
            EvaluatedValue::Float(v) => assert!((*v - 1.0).abs() < 1e-9, "expected ~1.0, got {v}"),
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

    /// 先頭の遅れは長さに入る: delay 0.5・duration 1.0 は 1.5 秒で終わり、t=1.0 は途中。
    #[test]
    fn leading_delay_counts_in_storyboard_duration() {
        let mut doc = simple_float_doc("delayed");
        let sb = doc.storyboard.get_mut("delayed").unwrap();
        let Some(TransitionRef::Inline(tr)) = sb.entry[0].transition.as_mut() else {
            panic!("inline transition expected");
        };
        tr.delay = 0.5;

        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        let StartResult { end_time, .. } = rt.start("delayed", 0.0).unwrap();
        assert_eq!(end_time, 1.5);

        rt.tick(1.0);
        let (_, v) = rt
            .last_result()
            .changes
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .expect("opacity at t=1.0");
        match v {
            EvaluatedValue::Float(v) => assert!((*v - 0.5).abs() < 1e-9, "expected 0.5, got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }
    }

    /// 検証は通ってもコンパイルで落ちる文書は読み込みで拒み、前の文書で start が通る（#32）
    #[test]
    fn load_rejects_compile_failing_document_and_keeps_previous() {
        use dola::KeyframeRef;
        let kf = |name: &str, at: KeyframeRef| StoryboardEntry {
            at: Some(at),
            keyframe: Some(name.to_string()),
            ..Default::default()
        };
        let single = |name: &str| KeyframeRef::Single(name.to_string());
        let cases: [(&str, Vec<StoryboardEntry>); 3] = [
            (
                "variable without transition",
                vec![StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    ..Default::default()
                }],
            ),
            ("empty at", vec![kf("a", KeyframeRef::Multiple(vec![]))]),
            (
                "keyframe cycle",
                vec![kf("a", single("b")), kf("b", single("a"))],
            ),
        ];

        for (label, entry) in cases {
            let mut rt = DolaRuntime::new();
            rt.load_document(simple_float_doc("fade_in")).unwrap();

            let mut bad = simple_float_doc("fade_in");
            bad.storyboard.get_mut("fade_in").unwrap().entry = entry;
            let err = rt.load_document(bad).unwrap_err();
            assert!(
                matches!(err, RuntimeError::CompileError(_)),
                "{label}: {err:?}"
            );

            let StartResult { end_time, .. } = rt
                .start("fade_in", 0.0)
                .unwrap_or_else(|e| panic!("{label}: previous document must start: {e:?}"));
            assert_eq!(end_time, 1.0, "{label}");
        }
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
            .entry(linear("opacity", 0.0, 1.0, duration))
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

    /// 読み込みが数値の規則で拒まれ、その欄と値を返す
    fn rejected_number(rt: &mut DolaRuntime, doc: DolaDocument) -> (String, f64) {
        match rt.load_document(doc).unwrap_err() {
            RuntimeError::CompileError(errs) => match &errs[..] {
                [dola::DolaError::InvalidNumber { field, value, .. }] => (field.clone(), *value),
                other => panic!("unexpected errors: {other:?}"),
            },
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn load_with_zero_loop_count_is_rejected() {
        // 0 と −2 以下の loop_count は、start を待たず読み込みの検証で拒まれる（#29）
        let mut rt = DolaRuntime::new();
        let rejected = rejected_number(&mut rt, loop_count_doc("fade", 0, 1.0));
        assert_eq!(rejected, ("loop_count".to_string(), 0.0));
        assert!(matches!(
            rt.start("fade", 0.0).unwrap_err(),
            RuntimeError::StoryboardNotFound(_)
        ));
    }

    #[test]
    fn load_with_negative_loop_count_is_rejected() {
        // -1（無限）以外の負値は無効
        let mut rt = DolaRuntime::new();
        let rejected = rejected_number(&mut rt, loop_count_doc("fade", -2, 1.0));
        assert_eq!(rejected, ("loop_count".to_string(), -2.0));
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
// time_scale の境界（#29）: 0・負・NaN は読み込みの検証で拒まれる
// （拒まないと end_time が +inf／NaN になり終わらない、負なら即 Conclude する）
// =========================================================================
mod time_scale_boundary {
    use super::super::*;
    use dola::{DolaError, InterruptionPolicy, Storyboard};

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
            .entry(linear("opacity", 0.0, 1.0, duration))
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

    fn assert_rejected(doc: DolaDocument, time_scale: f64) {
        let mut rt = DolaRuntime::new();
        let err = rt.load_document(doc).unwrap_err();
        assert!(
            matches!(
                &err,
                RuntimeError::CompileError(errs)
                if matches!(
                    &errs[..],
                    [DolaError::InvalidNumber { field, value, .. }]
                    if field == "time_scale" && value.to_bits() == time_scale.to_bits()
                )
            ),
            "time_scale = {time_scale}: {err:?}"
        );
    }

    #[test]
    fn non_positive_or_nan_time_scale_is_rejected_at_load() {
        for time_scale in [0.0, -1.0, f64::NAN] {
            assert_rejected(time_scale_doc("fade", time_scale, 1.0), time_scale);
        }
    }

    #[test]
    fn zero_time_scale_with_empty_entry_is_rejected_at_load() {
        // 拒まないと 0 / 0 = NaN の end_time になった組
        assert_rejected(empty_entry_time_scale_doc("empty_sb", 0.0), 0.0);
    }
}
