//! 各終了戦略の統合テスト（research.md §R3: termination_strategy シーム）

use super::*;

// =========================================================================
// 6.2 各終了戦略の統合テスト
// =========================================================================
mod cancel_strategy {
    use super::super::*;

    /// 周回の待ちの中で競合の Cancel を受けると、凍結値は前の周回の最後の値（1.0）。
    /// 待ちの中は実効時刻が負なので、値の保持（value_time）を外すと凍結値が出ない。
    #[test]
    fn cancel_during_loop_delay_keeps_previous_loop_final() {
        assert_eq!(
            super::y_after_interrupt_in_loop_delay(InterruptionPolicy::Cancel),
            Some(EvaluatedValue::Float(1.0))
        );
    }

    /// Cancel は割り込みの時刻（t=0.5）の値で凍る: y の 1 つ目のセグメントの途中の 5.0。
    #[test]
    fn cancel_freezes_at_interrupt_time() {
        assert_eq!(
            super::y_after_interrupt(InterruptionPolicy::Cancel),
            EvaluatedValue::Float(5.0)
        );
    }

    #[test]
    fn cancel_group_id_all_variables() {
        let doc = make_multi_sb_doc();
        let mut rt = DolaRuntime::new();
        let _x_id = rt.subscribe("x");
        let y_id = rt.subscribe("y");
        rt.load_document(doc).unwrap();

        // Start sb_a (Cancel policy, operates x and y)
        let r1 = rt.start("sb_a", 0.0).unwrap();
        assert!(r1.affected_group_ids.is_empty());

        // Start sb_new (operates x only) at t=0.5
        // → sb_a conflicts on x → Cancel sb_a (both x and y entries removed)
        let r2 = rt.start("sb_new", 0.5).unwrap();
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);

        // sb_a's y values should be frozen at t=0.5 → 12.5 (progress=0.25 of 0→50)
        let changes = rt.update(0.5).changes;
        // y should show frozen value, x shows new SB value
        let (_, y_val) = changes
            .iter()
            .find(|(id, _)| *id == y_id)
            .expect("y is delivered at the cancel time");
        assert_eq!(*y_val, EvaluatedValue::Float(12.5), "y frozen at 12.5");
    }
}

mod conclude_strategy {
    use super::super::*;

    /// Conclude は今のセグメントの最後の値へ飛ぶ: y の 1 つ目のセグメントの終わりの 10.0。
    #[test]
    fn conclude_jumps_to_current_segment_final() {
        assert_eq!(
            super::y_after_interrupt(InterruptionPolicy::Conclude),
            EvaluatedValue::Float(10.0)
        );
    }

    /// 周回の待ちの中で競合の Conclude を受けると、確定値は前の周回の最後の値（1.0）。
    /// 待ちの中は実効時刻が負なので、値の保持（value_time）を外すと確定値が出ない。
    #[test]
    fn conclude_during_loop_delay_keeps_previous_loop_final() {
        assert_eq!(
            super::y_after_interrupt_in_loop_delay(InterruptionPolicy::Conclude),
            Some(EvaluatedValue::Float(1.0))
        );
    }

    #[test]
    fn default_policy_is_conclude() {
        // StoryboardBuilder defaults to Conclude
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "opacity",
                AnimationVariableDef::Float {
                    initial: 0.0,
                    min: Some(0.0),
                    max: Some(100.0),
                },
            )
            // no .interruption_policy() call → defaults to Conclude
            .storyboard(
                "fade",
                StoryboardBuilder::new()
                    .entry(linear("opacity", 0.0, 100.0, 2.0))
                    .build(),
            )
            .build()
            .unwrap();

        let mut rt = DolaRuntime::new();
        let _opacity_id = rt.subscribe("opacity");
        rt.load_document(doc).unwrap();

        let r1 = rt.start("fade", 0.0).unwrap();
        let r2 = rt.start("fade", 0.5).unwrap();
        // Default Conclude → affected
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);
    }
}

mod trim_strategy {
    use super::super::*;

    /// Trim は割り込みの時刻（t=0.5）で切る: y の 1 つ目のセグメントの途中の 5.0。
    #[test]
    fn trim_cuts_at_interrupt_time() {
        assert_eq!(
            super::y_after_interrupt(InterruptionPolicy::Trim),
            EvaluatedValue::Float(5.0)
        );
    }
}

mod compress_strategy {
    use super::super::*;

    /// Compress は台本全体の最後の値へ飛ぶ: y の 2 つ目のセグメントの終わりの 30.0。
    #[test]
    fn compress_jumps_to_all_final_values() {
        assert_eq!(
            super::y_after_interrupt(InterruptionPolicy::Compress),
            EvaluatedValue::Float(30.0)
        );
    }
}

mod edge_cases {
    use super::super::*;

    #[test]
    fn no_side_effects_when_no_conflict() {
        let doc = DolaDocumentBuilder::new("1.0")
            .variable("x", float())
            .variable("y", float())
            .storyboard(
                "sb_x",
                StoryboardBuilder::new()
                    .entry(linear("x", 0.0, 100.0, 2.0))
                    .build(),
            )
            .storyboard(
                "sb_y",
                StoryboardBuilder::new()
                    .entry(linear("y", 0.0, 50.0, 2.0))
                    .build(),
            )
            .build()
            .unwrap();

        let mut rt = DolaRuntime::new();
        let x_id = rt.subscribe("x");
        let y_id = rt.subscribe("y");
        rt.load_document(doc).unwrap();

        rt.start("sb_x", 0.0).unwrap();

        // Update at t=1.0 → x = 50.0
        let changes = rt.update(1.0).changes;
        let x_val = changes.iter().find(|(id, _)| *id == x_id);
        assert!(x_val.is_some());

        // Start sb_y (different variable) → no conflict, no side effects
        let r = rt.start("sb_y", 1.0).unwrap();
        assert!(r.affected_group_ids.is_empty());

        // Both should still work
        let changes = rt.update(1.5).changes;
        // x should still be updating (from sb_x)
        // y should be updating (from sb_y)
        let x_change = changes.iter().find(|(id, _)| *id == x_id);
        let y_change = changes.iter().find(|(id, _)| *id == y_id);
        assert!(
            x_change.is_some() || y_change.is_some(),
            "at least one variable should have changed"
        );
    }

    #[test]
    fn mixed_policies_simultaneous_conflict() {
        let doc = make_multi_sb_doc();
        let mut rt = DolaRuntime::new();
        let _x_id = rt.subscribe("x");
        let _y_id = rt.subscribe("y");
        rt.load_document(doc).unwrap();

        // Start sb_a (Cancel, x+y) at t=0.0
        let r1 = rt.start("sb_a", 0.0).unwrap();
        // Start sb_b (Conclude, x) at t=0.0
        let r2 = rt.start("sb_b", 0.0).unwrap();

        // sb_b conflicts with sb_a on x → sb_a was Cancel'd
        assert_eq!(r2.affected_group_ids, vec![r1.group_id]);

        // Start sb_new (x) at t=0.5 → conflicts with sb_b (Conclude)
        let r3 = rt.start("sb_new", 0.5).unwrap();
        assert_eq!(r3.affected_group_ids, vec![r2.group_id]);
    }
}

/// a（policy）が x と y を [0,1] で回して周回の間に 2 秒待ち、その待ちの中（t=2.0）で
/// x だけを触る b に終わらされたとき、t=1.5 から購読した y に届く値。
fn y_after_interrupt_in_loop_delay(policy: InterruptionPolicy) -> Option<EvaluatedValue> {
    use dola::{EasingFunction, EasingName, LoopOffset, LoopOffsetRange, Storyboard};
    let ramp = |var: &str| linear(var, 0.0, 1.0, 1.0);
    // a: x と y を [0,1] で回し、周回の間に 2 秒待つ（1 周目 [0,1]・待ち [1,3]）
    let a = Storyboard {
        time_scale: 1.0,
        loop_count: -1,
        interruption_policy: policy,
        loop_offset: Some(LoopOffset::Range(LoopOffsetRange {
            min: 2.0,
            max: 2.0,
            easing: EasingFunction::Named(EasingName::Linear),
        })),
        entry: vec![ramp("x"), ramp("y")],
    };
    let b = StoryboardBuilder::new().entry(ramp("x")).build();
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float())
        .variable("y", float())
        .storyboard("a", a)
        .storyboard("b", b)
        .build()
        .unwrap();
    let mut rt = DolaRuntime::new();
    let _x_id = rt.subscribe("x");
    rt.load_document(doc).unwrap();

    let ra = rt.start("a", 0.0).unwrap();
    rt.tick(1.5); // 周回を終えて待ちの中
    // y の購読はここから: 届く y は a の確定値だけ（b は y を触らない）
    let y_id = rt.subscribe("y");
    let rb = rt.start("b", 2.0).unwrap();
    assert_eq!(rb.affected_group_ids, vec![ra.group_id]);

    rt.tick(2.0);
    rt.last_result()
        .changes
        .iter()
        .find(|(id, _)| *id == y_id)
        .map(|(_, v)| v.clone())
}

/// a（policy）が x を [0,2] で動かし、y を [0,1] で 0→10・[1,2] で 10→30 と 2 セグメントで
/// 動かす。t=0.5 に x だけを触る b に終わらされたとき、y に届く a の確定値
/// （b は y を触らないので、届く y は a の戦略が決めた値だけ）。
fn y_after_interrupt(policy: InterruptionPolicy) -> EvaluatedValue {
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("x", float())
        .variable("y", float())
        .storyboard(
            "a",
            StoryboardBuilder::new()
                .interruption_policy(policy)
                .entry(linear("x", 0.0, 100.0, 2.0))
                .entry(linear("y", 0.0, 10.0, 1.0))
                .entry(linear("y", 10.0, 30.0, 1.0))
                .build(),
        )
        .storyboard(
            "b",
            StoryboardBuilder::new()
                .entry(linear("x", 0.0, 10.0, 1.0))
                .build(),
        )
        .build()
        .unwrap();
    let mut rt = DolaRuntime::new();
    let y_id = rt.subscribe("y");
    rt.load_document(doc).unwrap();

    let ra = rt.start("a", 0.0).unwrap();
    rt.tick(0.25); // y = 2.5 を配る（確定値が差分として届くように）
    let rb = rt.start("b", 0.5).unwrap();
    assert_eq!(rb.affected_group_ids, vec![ra.group_id]);

    rt.tick(0.5);
    rt.last_result()
        .changes
        .iter()
        .find(|(id, _)| *id == y_id)
        .map(|(_, v)| v.clone())
        .expect("y に a の確定値が届く")
}
