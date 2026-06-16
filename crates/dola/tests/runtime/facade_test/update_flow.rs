//! update ループ挙動（research.md §R3: update_flow シーム）
//!
//! Pause/Resume サイクルと、異なる変数への同時再生を束ねる。

// =========================================================================
// 8.2 Pause/Resume サイクルテスト
// =========================================================================
mod pause_resume_cycle {
    use super::super::*;

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
// 8.4 同時再生テスト
// =========================================================================
mod concurrent_playback {
    use super::super::*;

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
