//! コンパイルパイプライン統合 + E2E 統合
use super::*;

// =============================================================
// コンパイルパイプライン統合テスト
// =============================================================

mod compile_integration_tests {
    use super::*;

    fn doc_with_fade_and_offset(loop_offset: Option<LoopOffset>) -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        );

        let mut storyboard = BTreeMap::new();
        storyboard.insert(
            "blink".to_string(),
            Storyboard {
                time_scale: 1.0,
                loop_count: -1,
                interruption_policy: InterruptionPolicy::Conclude,
                loop_offset,
                entry: vec![linear("opacity", 1.0, 0.0, ms(500))],
            },
        );

        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    #[test]
    fn compile_with_scalar_loop_offset() {
        let doc = doc_with_fade_and_offset(Some(LoopOffset::Scalar(ms(5000))));
        let compiled = compile_storyboard(&doc, "blink").unwrap();
        assert_eq!(compiled.loop_offset, Some(LoopOffset::Scalar(ms(5000))));
        assert_eq!(compiled.loop_count, -1);
    }

    #[test]
    fn compile_with_range_loop_offset() {
        let lo = LoopOffset::Range(LoopOffsetRange {
            min: ms(1000),
            max: ms(5000),
            easing: EasingFunction::Named(EasingName::QuadraticOut),
        });
        let doc = doc_with_fade_and_offset(Some(lo.clone()));
        let compiled = compile_storyboard(&doc, "blink").unwrap();
        assert_eq!(compiled.loop_offset, Some(lo));
    }

    #[test]
    fn compile_without_loop_offset() {
        let doc = doc_with_fade_and_offset(None);
        let compiled = compile_storyboard(&doc, "blink").unwrap();
        assert_eq!(compiled.loop_offset, None);
    }
}

// =============================================================
// E2E 統合テスト
// =============================================================

mod e2e_tests {
    use crate::support::{at_ms, linear, ms};
    use dola::runtime::{DolaRuntime, EvaluatedValue};
    use dola::*;
    use std::collections::BTreeMap;

    /// loop_offset 付きストーリーボードのドキュメントを生成するヘルパー
    fn loop_offset_doc(
        sb_name: &str,
        loop_count: i32,
        loop_offset: Option<LoopOffset>,
    ) -> DolaDocument {
        let mut variable = BTreeMap::new();
        variable.insert(
            "opacity".to_string(),
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        );
        let sb = Storyboard {
            time_scale: 1.0,
            loop_count,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset,
            entry: vec![linear("opacity", 0.0, 1.0, ms(1000))],
        };
        let mut storyboard = BTreeMap::new();
        storyboard.insert(sb_name.to_string(), sb);
        DolaDocument {
            schema_version: "1.0".to_string(),
            variable,
            transition: BTreeMap::new(),
            storyboard,
        }
    }

    fn setup_runtime(doc: DolaDocument) -> (DolaRuntime, i64) {
        let mut rt = DolaRuntime::new();
        let opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();
        (rt, opacity_id)
    }

    /// t まで進め、その回に出た opacity の値を返す（変わらなければ None）
    fn opacity_at(rt: &mut DolaRuntime, opacity_id: i64, t: TimePoint) -> Option<f64> {
        rt.tick(t);
        rt.last_result()
            .changes
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .map(|(_, v)| match v {
                EvaluatedValue::Float(f) => *f,
                other => panic!("expected Float, got {other:?}"),
            })
    }

    #[test]
    fn e2e_no_offset_natural_end() {
        // loop_count=2・待ちなし → 周回は [0,1]・[1,2]、t=2000ms で終わる
        let doc = loop_offset_doc("blink", 2, None);
        let (mut rt, opacity_id) = setup_runtime(doc);
        let result = rt.start("blink", at_ms(0)).unwrap();
        assert_eq!(result.end_time, Some(at_ms(2000))); // 2 周の終わり

        let v = opacity_at(&mut rt, opacity_id, at_ms(250)).expect("opacity at t=250ms");
        assert!((v - 0.25).abs() < 1e-9, "t=250ms: {v}");
        let v = opacity_at(&mut rt, opacity_id, at_ms(1500)).expect("opacity at t=1500ms");
        assert!((v - 0.5).abs() < 1e-9, "t=1500ms: {v}");
        assert!(rt.has_instances(), "t=1500ms は 2 周目の中");

        // 2 周の後は最終値 1.0 を配って終わる
        let v = opacity_at(&mut rt, opacity_id, at_ms(2500)).expect("opacity at t=2500ms");
        assert!((v - 1.0).abs() < 1e-9, "t=2500ms: {v}");
        assert!(!rt.has_instances(), "t=2500ms で終わっている");
    }

    #[test]
    fn e2e_with_fixed_delay_extends_playback() {
        // loop_count=2・待ち 2 秒固定 → 1 周目 [0,1]・待ち [1,3]・2 周目 [3,4]
        let doc = loop_offset_doc(
            "blink",
            2,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(2000),
                max: ms(2000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("blink", at_ms(0)).unwrap();

        let v = opacity_at(&mut rt, opacity_id, at_ms(500)).expect("opacity at t=500ms");
        assert!((v - 0.5).abs() < 1e-9, "t=500ms: {v}");

        // 周回の終わりを越えて待ちに入る → 待ちの間は 1 周目の最後の値を保つ
        let v = opacity_at(&mut rt, opacity_id, at_ms(1500)).expect("opacity at t=1500ms");
        assert!((v - 1.0).abs() < 1e-9, "t=1500ms: {v}");
        assert_eq!(
            opacity_at(&mut rt, opacity_id, at_ms(2500)),
            None,
            "t=2500ms"
        );

        // 起点が待ちの後（3000ms）にあれば 2 周目の中ほど
        let v = opacity_at(&mut rt, opacity_id, at_ms(3500)).expect("opacity at t=3500ms");
        assert!((v - 0.5).abs() < 1e-9, "t=3500ms: {v}");

        // 2 周目の終わり 4000 ms を越えて終わる → 最終値
        let v = opacity_at(&mut rt, opacity_id, at_ms(4500)).expect("opacity at t=4500ms");
        assert!((v - 1.0).abs() < 1e-9, "t=4500ms: {v}");

        rt.tick(at_ms(5000));
        assert!(rt.last_result().changes.is_empty());
    }

    #[test]
    fn e2e_loop_count_1_ignores_offset() {
        // loop_count=1 + 待ち 10 秒固定 → 待ちは使われず t=1000ms で終わる
        let doc = loop_offset_doc(
            "blink",
            1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(10000),
                max: ms(10000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("blink", at_ms(0)).unwrap();

        let v = opacity_at(&mut rt, opacity_id, at_ms(1500)).expect("opacity at t=1500ms");
        assert!((v - 1.0).abs() < 1e-9, "t=1500ms: {v}");
        assert!(
            !rt.has_instances(),
            "t=1500ms で終わっている（待ちに入らない）"
        );
    }

    #[test]
    fn e2e_infinite_loop_with_offset_continues() {
        // loop_count=-1 + fixed delay 2000ms
        let doc = loop_offset_doc(
            "blink",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(2000),
                max: ms(2000), // fixed 2000ms delay
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("blink", at_ms(0)).unwrap();
        // 周回は [0,1]・[3,4]・[6,7]…・待ちの間は最後の値 1.0 を保つ
        for (t, want) in [
            (500, 0.5),
            (1500, 1.0),
            (3500, 0.5),
            (4500, 1.0),
            (6500, 0.5),
        ] {
            let v = opacity_at(&mut rt, opacity_id, at_ms(t))
                .unwrap_or_else(|| panic!("opacity at t={t}ms"));
            assert!((v - want).abs() < 1e-9, "t={t}: {v}");
        }
    }

    #[test]
    fn e2e_pause_during_delay_preserves_timing() {
        // 3 周・待ち 5 秒固定 → 1 周目 [0,1]・待ち [1,6]・2 周目 [6,7]・待ち [7,12]・3 周目 [12,13]
        let doc = loop_offset_doc(
            "blink",
            3,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(5000),
                max: ms(5000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        let result = rt.start("blink", at_ms(0)).unwrap();
        let group_id = result.group_id;
        assert_eq!(result.end_time, Some(at_ms(13_000)));

        // t=1500ms: 1 周目の終わりを越えて待ちに入る
        rt.tick(at_ms(1500));

        // 待ちの中の t=3000ms で止め、t=5000ms で戻す（止めていた長さ 2 秒）
        rt.pause(group_id, at_ms(3000)).unwrap();
        let new_end = rt.resume(group_id, at_ms(5000)).unwrap();
        // 全体の終わりは止めていた長さだけ後ろへずれる: 13000 + 2000 = 15000 ms
        assert_eq!(new_end, Some(at_ms(15_000)));
    }

    #[test]
    fn e2e_pause_with_infinite_loop_has_no_planned_end() {
        let doc = loop_offset_doc(
            "blink",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(5000),
                max: ms(5000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        let group_id = rt.start("blink", at_ms(0)).unwrap().group_id;
        rt.pause(group_id, at_ms(3000)).unwrap();
        assert_eq!(rt.resume(group_id, at_ms(5000)).unwrap(), None);
    }

    #[test]
    fn e2e_cancel_during_delay() {
        // Cancel during delay period → immediate stop
        let doc = loop_offset_doc(
            "blink",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(10000),
                max: ms(10000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, _opacity_id) = setup_runtime(doc);
        let result = rt.start("blink", at_ms(0)).unwrap();
        let group_id = result.group_id;

        // t=1500ms: advance → in delay period
        rt.tick(at_ms(1500));

        // Cancel → immediate stop
        rt.cancel(group_id).unwrap();

        // After cancel, no more updates
        rt.tick(at_ms(2000));
        let changes = rt.last_result().changes.clone();
        assert!(changes.is_empty());
    }

    #[test]
    fn e2e_without_offset_backward_compatible() {
        // 待ちなし・3 周 → 周回は [0,1]・[1,2]・[2,3]、t=3000ms で終わる
        let doc = loop_offset_doc("fade", 3, None);
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("fade", at_ms(0)).unwrap();

        for (t, want) in [(500, 0.5), (2250, 0.25)] {
            let v = opacity_at(&mut rt, opacity_id, at_ms(t))
                .unwrap_or_else(|| panic!("opacity at t={t}ms"));
            assert!((v - want).abs() < 1e-9, "t={t}: {v}");
        }
        assert!(rt.has_instances(), "t=2250ms は 3 周目の中");

        let v = opacity_at(&mut rt, opacity_id, at_ms(3500)).expect("opacity at t=3500ms");
        assert!((v - 1.0).abs() < 1e-9, "t=3500ms: {v}");
        assert!(!rt.has_instances(), "t=3500ms で終わっている");
    }

    #[test]
    fn e2e_extreme_small_delay() {
        // 待ち 1 ms 固定・3 周 → 周回は [0,1000]・[1001,2001]・[2002,3002] ms
        let doc = loop_offset_doc(
            "fast",
            3,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(1),
                max: ms(1),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("fast", at_ms(0)).unwrap();

        // 待ち [1000,1001] ms・[2001,2002] ms の中は最後の値 1.0 を保つ
        for (t, want) in [
            (5_000_000, 0.5),
            (10_005_000, 1.0),
            (15_000_000, 0.499),
            (20_015_000, 1.0),
        ] {
            let v = opacity_at(&mut rt, opacity_id, TimePoint::from_ticks(t))
                .unwrap_or_else(|| panic!("opacity at t={t} ticks"));
            assert!((v - want).abs() < 1e-9, "t={t}: {v}");
        }

        // 終わり 3002 ms の直前は 3 周目の中で生きている
        let v = opacity_at(&mut rt, opacity_id, at_ms(3000)).expect("opacity at t=3000ms");
        assert!((v - 0.998).abs() < 1e-9, "t=3000ms: {v}");

        // 直後に終わって最終値、その後は差分なし
        let v = opacity_at(&mut rt, opacity_id, at_ms(3003)).expect("opacity at t=3003ms");
        assert!((v - 1.0).abs() < 1e-9, "t=3003ms: {v}");
        rt.tick(at_ms(3010));
        assert!(rt.last_result().changes.is_empty());
    }

    #[test]
    fn e2e_large_delay() {
        // 待ち 100 秒固定・2 周 → 周回は [0,1]・[101,102]
        let doc = loop_offset_doc(
            "slow",
            2,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(100000),
                max: ms(100000),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("slow", at_ms(0)).unwrap();

        let v = opacity_at(&mut rt, opacity_id, at_ms(1500)).expect("opacity at t=1500ms");
        assert!((v - 1.0).abs() < 1e-9, "t=1500ms: {v}");
        // 待ちの中は値を保ったまま生きている
        assert_eq!(
            opacity_at(&mut rt, opacity_id, at_ms(50000)),
            None,
            "t=50000ms"
        );
        assert!(rt.has_instances(), "t=50000ms は待ちの中");

        let v = opacity_at(&mut rt, opacity_id, at_ms(101500)).expect("opacity at t=101500ms");
        assert!((v - 0.5).abs() < 1e-9, "t=101500ms: {v}");

        let v = opacity_at(&mut rt, opacity_id, at_ms(106000)).expect("opacity at t=106000ms");
        assert!((v - 1.0).abs() < 1e-9, "t=106000ms: {v}");
        assert!(!rt.has_instances(), "t=106000ms で終わっている");
    }

    #[test]
    fn e2e_many_loops_with_wait_stay_exact() {
        // 待ち 100 ms 固定・無限 → k 周目は 1100k ms ちょうどに始まる（1 周 1100 ms）
        let doc = loop_offset_doc(
            "precision",
            -1,
            Some(LoopOffset::Range(LoopOffsetRange {
                min: ms(100),
                max: ms(100),
                easing: EasingFunction::Named(EasingName::Linear),
            })),
        );
        let (mut rt, opacity_id) = setup_runtime(doc);
        rt.start("precision", at_ms(0)).unwrap();

        // t=115000ms は 104 周目（起点 114400ms）の 600 ms 目
        let v = opacity_at(&mut rt, opacity_id, at_ms(115000)).expect("opacity at t=115000ms");
        assert!((v - 0.6).abs() < 1e-9, "t=115000ms: {v}");
        assert!(rt.has_instances(), "無限のループは生きている");
    }
}
