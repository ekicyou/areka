//! update ループ挙動
//!
//! Pause/Resume サイクルと、異なる変数への同時再生を束ねる。

// =========================================================================
// Pause/Resume サイクルテスト
// =========================================================================
mod pause_resume_cycle {
    use super::super::*;

    #[test]
    fn pause_freezes_value_and_resume_continues() {
        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");

        let doc = simple_float_doc("fade_in");
        rt.load_document(doc).unwrap();

        let StartResult { group_id, .. } = rt.start("fade_in", at_ms(0)).unwrap();

        // t=0.0: 初回
        rt.tick(at_ms(0));

        // t=0.3: 中間
        rt.tick(at_ms(300));
        let diff = rt.last_result().changes.clone();
        assert!(!diff.is_empty());
        match &diff[0].1 {
            EvaluatedValue::Float(v) => assert!((*v - 0.3).abs() < 1e-9, "t=0.3: got {v}"),
            other => panic!("expected Float, got {other:?}"),
        }

        // Pause at (logical) t=0.3
        rt.pause(group_id, at_ms(300)).unwrap();

        // t=0.5: Pause 中 — 値が凍結される
        // Paused 状態では timeline_manager が pause_start 時点の値を返す
        rt.tick(at_ms(500));
        let diff = rt.last_result().changes.clone();
        // Paused 中は前回値と同じ → diff 空
        assert!(
            diff.is_empty(),
            "expected no change during pause, got {diff:?}"
        );

        // Resume at logical t=0.5
        let new_end = rt.resume(group_id, at_ms(500)).unwrap();
        // 終わりの予定は pause 分（0.2s）延長される: 1.0 + 0.2 = 1.2
        assert_eq!(new_end, Some(at_ms(1200)));

        // t=0.7: pause 分 0.2s ずれで effective=0.5
        rt.tick(at_ms(700));
        let diff = rt.last_result().changes.clone();
        assert!(!diff.is_empty(), "expected value to change after resume");
        match &diff[0].1 {
            EvaluatedValue::Float(v) => {
                assert!((*v - 0.5).abs() < 1e-9, "t=0.7 (eff=0.5): got {v}")
            }
            other => panic!("expected Float, got {other:?}"),
        }
    }

    fn opacity(rt: &DolaRuntime) -> Option<f64> {
        match rt.last_result().changes.first() {
            Some((_, EvaluatedValue::Float(v))) => Some(*v),
            None => None,
            Some((_, other)) => panic!("expected Float, got {other:?}"),
        }
    }

    fn assert_opacity(rt: &DolaRuntime, expected: f64) {
        let v = opacity(rt).expect("expected a change");
        assert!((v - expected).abs() < 1e-9, "expected {expected}, got {v}");
    }

    fn started(doc: DolaDocument, name: &str) -> (DolaRuntime, u64) {
        let mut rt = DolaRuntime::new();
        rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        let StartResult { group_id, .. } = rt.start(name, at_ms(0)).unwrap();
        (rt, group_id)
    }

    #[test]
    fn value_right_after_resume_continues_from_pause_point() {
        let (mut rt, gid) = started(simple_float_doc("fade_in"), "fade_in");
        rt.tick(at_ms(300));
        assert_opacity(&rt, 0.3);
        rt.pause(gid, at_ms(300)).unwrap();
        assert_eq!(rt.resume(gid, at_ms(800)).unwrap(), Some(at_ms(1500)));
        rt.tick(at_ms(800));
        assert_eq!(opacity(&rt), None);
        rt.tick(at_ms(900));
        assert_opacity(&rt, 0.4);
    }

    #[test]
    fn paused_value_is_the_same_at_any_time_and_does_not_end() {
        let (mut rt, gid) = started(simple_float_doc("fade_in"), "fade_in");
        rt.tick(at_ms(300));
        rt.pause(gid, at_ms(300)).unwrap();
        for t in [400, 1000, 60_000] {
            rt.tick(at_ms(t));
            assert_eq!(opacity(&rt), None, "t={t}");
        }
        assert!(rt.has_instances());
    }

    #[test]
    fn resume_before_pause_time_continues_from_pause_point() {
        let (mut rt, gid) = started(simple_float_doc("fade_in"), "fade_in");
        rt.tick(at_ms(500));
        assert_opacity(&rt, 0.5);
        rt.pause(gid, at_ms(500)).unwrap();
        assert_eq!(rt.resume(gid, at_ms(300)).unwrap(), Some(at_ms(800)));
        rt.tick(at_ms(300));
        assert_eq!(opacity(&rt), None);
        rt.tick(at_ms(400));
        assert_opacity(&rt, 0.6);
    }

    #[test]
    fn finish_deadline_takes_effect_while_paused() {
        let (mut rt, gid) = started(simple_float_doc("fade_in"), "fade_in");
        rt.tick(at_ms(300));
        rt.pause(gid, at_ms(300)).unwrap();
        rt.finish(gid, at_ms(600)).unwrap();
        rt.tick(at_ms(599));
        assert_eq!(opacity(&rt), None);
        assert!(rt.has_instances());
        rt.tick(at_ms(600));
        assert_opacity(&rt, 1.0);
        assert!(!rt.has_instances());
    }

    #[test]
    fn resume_returns_deadline_when_it_is_earlier_than_end() {
        let (mut rt, gid) = started(simple_float_doc("fade_in"), "fade_in");
        rt.pause(gid, at_ms(300)).unwrap();
        rt.finish(gid, at_ms(900)).unwrap();
        assert_eq!(rt.resume(gid, at_ms(500)).unwrap(), Some(at_ms(900)));
    }

    #[test]
    fn resume_returns_end_when_it_is_earlier_than_deadline() {
        let (mut rt, gid) = started(simple_float_doc("fade_in"), "fade_in");
        rt.pause(gid, at_ms(300)).unwrap();
        rt.finish(gid, at_ms(5000)).unwrap();
        assert_eq!(rt.resume(gid, at_ms(500)).unwrap(), Some(at_ms(1200)));
    }

    #[test]
    fn resume_of_endless_loop_returns_deadline() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable("opacity", bounded(0.0, 0.0, 1.0))
            .storyboard(
                "pulse",
                StoryboardBuilder::new()
                    .loop_count(-1)
                    .entry(linear("opacity", 0.0, 1.0, ms(1000)))
                    .build(),
            )
            .build()
            .unwrap();
        let (mut rt, gid) = started(doc, "pulse");
        rt.pause(gid, at_ms(300)).unwrap();
        assert_eq!(rt.resume(gid, at_ms(500)).unwrap(), None);
        rt.pause(gid, at_ms(600)).unwrap();
        rt.finish(gid, at_ms(5000)).unwrap();
        assert_eq!(rt.resume(gid, at_ms(700)).unwrap(), Some(at_ms(5000)));
    }
}

// =========================================================================
// 同時再生テスト
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
        let r1 = rt.start("fade", at_ms(0)).unwrap();
        let r2 = rt.start("zoom", at_ms(0)).unwrap();
        assert_ne!(r1.group_id, r2.group_id);
        assert_eq!(r1.end_time, Some(at_ms(1000)), "fade end_time");
        assert_eq!(r2.end_time, Some(at_ms(2000)), "zoom end_time");

        // t=0.5: opacity=0.5, scale=1.25
        rt.tick(at_ms(500));
        let diff = rt.last_result().changes.clone();
        assert!(diff.len() >= 2, "expected both variables, got {diff:?}");

        let (_, opacity) = diff
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .expect("opacity@0.5");
        let (_, scale) = diff
            .iter()
            .find(|(id, _)| *id == scale_id)
            .expect("scale@0.5");
        assert_eq!(*opacity, EvaluatedValue::Float(0.5), "opacity@0.5");
        assert_eq!(*scale, EvaluatedValue::Float(1.25), "scale@0.5");

        // t=1.5: opacity 終了(→1.0 凍結), scale 中間(→1.75)
        rt.tick(at_ms(1500));
        let diff = rt.last_result().changes.clone();
        // opacity の自然終了 + scale の更新
        let (_, scale) = diff
            .iter()
            .find(|(id, _)| *id == scale_id)
            .expect("scale@1.5");
        assert_eq!(*scale, EvaluatedValue::Float(1.75), "scale@1.5");
    }
}

// =========================================================================
// 値の保持
// =========================================================================
mod value_hold {
    use super::super::*;
    use std::collections::HashMap;

    /// 1 つのストーリーボードに opacity 0→1（1 秒）と scale 1→2（2 秒）。
    fn short_and_long_doc() -> DolaDocument {
        let mut doc = dual_variable_doc();
        let sb = StoryboardBuilder::new()
            .entry(linear("opacity", 0.0, 1.0, ms(1000)))
            .entry(linear("scale", 1.0, 2.0, ms(2000)))
            .build();
        doc.storyboard.clear();
        doc.storyboard.insert("both".to_string(), sb);
        doc
    }

    #[test]
    fn shorter_variable_holds_final_value_until_instance_ends() {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        rt.load_document(short_and_long_doc()).unwrap();
        rt.start("both", at_ms(0)).unwrap();

        // 受け手が持つ最後の値
        let mut seen: HashMap<i64, EvaluatedValue> = HashMap::new();
        let mut tick = |rt: &mut DolaRuntime, t: i64| {
            rt.tick(at_ms(t));
            for (id, v) in &rt.last_result().changes {
                seen.insert(*id, v.clone());
            }
            seen.get(&opacity_id).cloned()
        };

        assert_eq!(tick(&mut rt, 992), Some(EvaluatedValue::Float(0.992)));
        // opacity の最後のセグメントを過ぎても、インスタンスは生きている → 最後の値
        assert_eq!(tick(&mut rt, 1008), Some(EvaluatedValue::Float(1.0)));
        // t=2.0 で Conclude した後も 1.0 のまま
        assert_eq!(tick(&mut rt, 2000), Some(EvaluatedValue::Float(1.0)));
        assert_eq!(tick(&mut rt, 2500), Some(EvaluatedValue::Float(1.0)));
    }
}

// =========================================================================
// 生きたインスタンスの有無
// =========================================================================
mod has_instances {
    use super::super::*;

    /// delay の間は変化が 0 件でもインスタンスは生きている → 真。Conclude の後は偽。
    #[test]
    fn has_instances_stays_true_during_delay() {
        let mut doc = simple_float_doc("delayed");
        let sb = doc.storyboard.get_mut("delayed").unwrap();
        let Some(TransitionRef::Inline(tr)) = sb.entry[0].transition.as_mut() else {
            panic!("inline transition expected");
        };
        tr.delay = ms(1000);

        let mut rt = DolaRuntime::new();
        rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        assert!(!rt.has_instances());

        let StartResult { group_id, .. } = rt.start("delayed", at_ms(0)).unwrap();
        rt.tick(at_ms(0));
        for t in [300, 600, 900] {
            rt.tick(at_ms(t));
            assert!(
                rt.last_result().changes.is_empty(),
                "t={t}ms: delay 中は変化なし"
            );
            assert!(rt.has_instances(), "t={t}ms: delay 中も生きている");
        }

        rt.conclude(group_id).unwrap();
        assert!(!rt.has_instances());
    }
}
