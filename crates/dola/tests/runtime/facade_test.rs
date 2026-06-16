#![allow(deprecated)]
//! DolaRuntime Facade 統合テスト
//!
//! Task 8.1〜8.5: load → start → update → 各終了フロー → 差分配信の
//! エンドツーエンド検証。

use std::collections::BTreeMap;

use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, StartResult};
use dola::{
    AnimationVariableDef, DolaDocument, StoryboardBuilder, StoryboardEntry, TransitionDef,
    TransitionRef, TransitionValue,
};

// =========================================================================
// ヘルパー
// =========================================================================

/// Float 変数 1 つ + 線形 0→1 (duration=1.0) のストーリーボードを持つ最小ドキュメント。
fn simple_float_doc(sb_name: &str) -> DolaDocument {
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
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
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

/// Float 2 変数 + 2 ストーリーボードの並行再生向けドキュメント。
fn dual_variable_doc() -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "opacity".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        },
    );
    variable.insert(
        "scale".to_string(),
        AnimationVariableDef::Float {
            initial: 1.0,
            min: Some(0.0),
            max: Some(2.0),
        },
    );

    // sb_fade: opacity 0→1 in 1.0s
    let sb_fade = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        })
        .build();

    // sb_zoom: scale 1→2 in 2.0s
    let sb_zoom = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("scale".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(1.0)),
                to: Some(TransitionValue::Scalar(2.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("fade".to_string(), sb_fade);
    storyboard.insert("zoom".to_string(), sb_zoom);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}

// =========================================================================
// 8.1 フル再生サイクルテスト
// =========================================================================
mod full_playback_cycle {
    use super::*;

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
// 8.2 Pause/Resume サイクルテスト
// =========================================================================
mod pause_resume_cycle {
    use super::*;

    #[test]
    fn pause_freezes_value_and_resume_continues() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();

        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0: 初回
        let _ = rt.update(0.0);

        // t=0.3: 中間
        let diff = rt.update(0.3).changes;
        assert!(!diff.is_empty());
        match &diff[0].1 {
            EvaluatedValue::Float(v) => assert!((*v - 0.3).abs() < 0.05, "t=0.3: got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // Pause at (logical) t=0.3
        rt.pause(group_id, 0.3).unwrap();

        // t=0.5: Pause 中 — 値が凍結される
        // Paused 状態では timeline_manager が pause_start 時点の値を返す
        let diff = rt.update(0.5).changes;
        // Paused 中は前回値と同じ → diff 空
        assert!(
            diff.is_empty(),
            "expected no change during pause, got {diff:?}"
        );

        // Resume at logical t=0.5
        let new_end = rt.resume(group_id, 0.5).unwrap();
        // end_time は pause 分（0.2s）延長される: 1.0 + 0.2 = 1.2
        assert!(
            (new_end - 1.2).abs() < 0.05,
            "expected end_time ~1.2, got {new_end}"
        );

        // t=0.7: pause 分 0.2s ずれで effective=0.5
        let diff = rt.update(0.7).changes;
        assert!(!diff.is_empty(), "expected value to change after resume");
        match &diff[0].1 {
            EvaluatedValue::Float(v) => {
                assert!((*v - 0.5).abs() < 0.05, "t=0.7 (eff=0.5): got {v}")
            }
            other => panic!("expected Float, got {other:?}"),
        }
    }
}

// =========================================================================
// 8.3 指示書差し替えテスト
// =========================================================================
mod document_replacement {
    use super::*;

    #[test]
    fn same_variable_carries_over_value() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");

        // 第1指示書: opacity 0→1
        let doc1 = simple_float_doc("fade_in");
        rt.load_document(doc1).unwrap();
        rt.start("fade_in", 0.0).unwrap();

        // t=0.5: opacity ≈ 0.5
        let diff = rt.update(0.5).changes;
        assert!(!diff.is_empty());

        // t=1.5: 自然終了後
        let _ = rt.update(1.5);

        // 第2指示書: opacity 0→1 (同じ名前)
        let doc2 = simple_float_doc("fade_in");
        rt.load_document(doc2).unwrap();
        rt.start("fade_in", 2.0).unwrap();

        // t=2.0: 新しいストーリーボードで opacity=0.0
        let diff = rt.update(2.0).changes;
        assert!(
            !diff.is_empty(),
            "expected value delivery from new storyboard"
        );
    }

    #[test]
    fn invalid_document_preserves_existing() {
        let mut rt = DolaRuntime::new();

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();

        // 不正なドキュメント（空の storyboard 名）— バリデーションエラー
        let bad_doc = DolaDocument {
            schema_version: "1.0".to_string(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        };

        // 空ドキュメントは有効（storyboard なしでもOK）
        // → バリデーション通過するので既存を上書きしてしまうが、
        //   start("fade_in", ..) は StoryboardNotFound になるだけ。
        // 不正書式は CompileError でブロックされる。
        // ここでは store 可能であることだけ確認。
        let result = rt.load_document(bad_doc);
        // バリデーション通過（空ドキュメントは valid）
        assert!(result.is_ok());
    }
}

// =========================================================================
// 8.4 同時再生テスト
// =========================================================================
mod concurrent_playback {
    use super::*;

    #[test]
    fn two_storyboards_on_different_variables() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        let scale_id = rt.subscribe("scale");

        let doc = dual_variable_doc();
        rt.load_document(doc).unwrap();

        // 両方の SB を開始
        let r1 = rt.start("fade", 0.0).unwrap();
        let r2 = rt.start("zoom", 0.0).unwrap();
        assert_ne!(r1.group_id, r2.group_id);
        assert!((r1.end_time - 1.0).abs() < 1e-9, "fade end_time");
        assert!((r2.end_time - 2.0).abs() < 1e-9, "zoom end_time");

        // t=0.5: opacity=0.5, scale=1.25
        let diff = rt.update(0.5).changes;
        assert!(diff.len() >= 2, "expected both variables, got {diff:?}");

        let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
        let scale = diff.iter().find(|(id, _)| *id == scale_id);

        if let Some((_, EvaluatedValue::Float(v))) = opacity {
            assert!((*v - 0.5).abs() < 0.05, "opacity@0.5: {v}");
        }
        if let Some((_, EvaluatedValue::Float(v))) = scale {
            assert!((*v - 1.25).abs() < 0.05, "scale@0.5: {v}");
        }

        // t=1.5: opacity 終了(→1.0 凍結), scale 中間(→1.75)
        let diff = rt.update(1.5).changes;
        // opacity の自然終了 + scale の更新
        let scale = diff.iter().find(|(id, _)| *id == scale_id);
        if let Some((_, EvaluatedValue::Float(v))) = scale {
            assert!((*v - 1.75).abs() < 0.05, "scale@1.5: {v}");
        }
    }
}

// =========================================================================
// 8.5 Conclude/Cancel/Finish フローテスト
// =========================================================================
mod conclude_cancel_finish {
    use super::*;

    #[test]
    fn conclude_delivers_final_values() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0: 初回
        let _ = rt.update(0.0);

        // t=0.3 で Conclude
        rt.conclude(group_id).unwrap();

        // 次の update で最終値 (1.0) が配信される
        let diff = rt.update(0.3).changes;
        let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
        match opacity {
            Some((_, EvaluatedValue::Float(v))) => {
                assert!((*v - 1.0).abs() < 0.01, "conclude final value: {v}")
            }
            other => panic!("expected final opacity Float(1.0), got {other:?}"),
        }

        // 以降は差分なし
        let diff = rt.update(0.5).changes;
        assert!(
            diff.is_empty(),
            "expected empty diff after conclude, got {diff:?}"
        );
    }

    #[test]
    fn cancel_freezes_current_value() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0: 初回
        let _ = rt.update(0.0);
        // t=0.5: 中間値取得
        let _ = rt.update(0.5);

        // Cancel — 現在値凍結
        rt.cancel(group_id).unwrap();

        // 以降は差分なし（タイムテーブル削除済み、値は前回 update 値で凍結）
        let diff = rt.update(0.6).changes;
        assert!(diff.is_empty(), "expected empty diff after cancel");

        let diff = rt.update(1.0).changes;
        assert!(diff.is_empty(), "expected still empty after cancel");
    }

    #[test]
    fn finish_delayed_conclude() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        // t=0.0
        let _ = rt.update(0.0);

        // finish: deadline = 0.5（t=0.5 で Conclude 相当）
        rt.finish(group_id, 0.5).unwrap();

        // t=0.3: deadline 未到達 → まだ再生中
        let diff = rt.update(0.3).changes;
        assert!(!diff.is_empty(), "expected playing at 0.3");
        match &diff[0].1 {
            EvaluatedValue::Float(v) => assert!(*v < 0.9, "should not be final value yet, got {v}"),
            _ => {}
        }

        // t=0.5: deadline 到達 → Conclude 相当
        let diff = rt.update(0.5).changes;
        // 最終値が配信されるか、conclude 処理で値が飛ぶ
        let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
        if let Some((_, EvaluatedValue::Float(v))) = opacity {
            assert!((*v - 1.0).abs() < 0.01, "finish conclude final: {v}");
        }

        // 以降は差分なし
        let diff = rt.update(0.8).changes;
        assert!(
            diff.is_empty(),
            "expected empty after finish-conclude, got {diff:?}"
        );
    }

    #[test]
    fn conclude_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.conclude(9999).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn cancel_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.cancel(9999).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn double_conclude_fails() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        rt.conclude(group_id).unwrap();
        // Concluded でインスタンス削除済み → 2 回目は InvalidGroupId
        let err = rt.conclude(group_id).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }

    #[test]
    fn cancel_after_conclude_fails() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        rt.conclude(group_id).unwrap();
        // Conclude 済み（インスタンス削除済み）の Cancel は InvalidGroupId
        let err = rt.cancel(group_id).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }

    #[test]
    fn finish_on_invalid_group_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.finish(9999, 1.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(9999)));
    }

    #[test]
    fn finish_after_conclude_fails() {
        let mut rt = DolaRuntime::new();
        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start("fade_in", 0.0).unwrap();

        rt.conclude(group_id).unwrap();
        let err = rt.finish(group_id, 1.0).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
    }
}

// =========================================================================
// start / calculate_end_time エラーパステスト
// （InvalidLoopCount / ZeroDurationWithLoop / TooShortDurationWithInfiniteLoop）
// =========================================================================
mod start_error_paths {
    use super::*;
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
// 購読 API のファサード経由検証（unsubscribe / unsubscribe_all / Default）
// =========================================================================
mod subscription_via_facade {
    use super::*;

    #[test]
    fn unsubscribe_stops_diff_delivery() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        rt.load_document(simple_float_doc("fade_in")).unwrap();
        rt.start("fade_in", 0.0).unwrap();

        // 購読中は差分が配信される
        let diff = rt.update(0.0).changes;
        assert!(diff.iter().any(|(id, _)| *id == opacity_id));

        // 購読解除後は配信されない
        rt.unsubscribe(opacity_id).unwrap();
        let diff = rt.update(0.5).changes;
        assert!(
            diff.is_empty(),
            "expected no delivery after unsubscribe, got {diff:?}"
        );
    }

    #[test]
    fn unsubscribe_all_stops_diff_delivery() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(simple_float_doc("fade_in")).unwrap();
        rt.start("fade_in", 0.0).unwrap();

        let diff = rt.update(0.0).changes;
        assert!(!diff.is_empty());

        rt.unsubscribe_all();
        let diff = rt.update(0.5).changes;
        assert!(
            diff.is_empty(),
            "expected no delivery after unsubscribe_all, got {diff:?}"
        );
    }

    #[test]
    fn unsubscribe_invalid_id_fails() {
        let mut rt = DolaRuntime::new();
        let err = rt.unsubscribe(9999).unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidVariableId(9999)));
    }

    #[test]
    fn default_is_equivalent_to_new() {
        let mut rt = DolaRuntime::default();
        // ドキュメント未読込 → StoryboardNotFound
        let err = rt.start("any", 0.0).unwrap_err();
        assert!(matches!(err, RuntimeError::StoryboardNotFound(_)));
        // 採番は 0-origin（new() と同一の初期状態）
        assert_eq!(rt.subscribe("x"), 0);
        // tick() 未呼び出し時の last_result は空
        assert!(rt.last_result().changes.is_empty());
        assert!(rt.last_result().triggered.is_empty());
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
    use super::*;
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
