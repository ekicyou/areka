//! ループ再生 統合テスト（dola-runtime-5-loop Task 5.2）
//!
//! facade 経由でのループ再生エンドツーエンド検証。

use std::collections::BTreeMap;

use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, StartResult};
use dola::{
    AnimationVariableDef, DolaDocument, StoryboardBuilder, StoryboardEntry, TransitionDef,
    TransitionRef, TransitionValue,
};

// =========================================================================
// ヘルパー
// =========================================================================

/// loop_count 付き minimal doc: opacity 0→1 (duration=1.0)
fn loop_doc(sb_name: &str, loop_count: i32) -> DolaDocument {
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

/// 短い duration (0.05s) の loop doc（短周期テスト用）
fn short_duration_loop_doc(sb_name: &str, loop_count: i32) -> DolaDocument {
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
                duration: Some(0.05),
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

/// EvaluatedValue から f64 を取り出す
fn extract_float(val: &EvaluatedValue) -> f64 {
    match val {
        EvaluatedValue::Float(v) => *v,
        other => panic!("expected Float, got {other:?}"),
    }
}

// =========================================================================
// ループなし（loop_count=1）の既存動作互換性
// =========================================================================

#[test]
fn loop_count_1_compatible_with_existing_behavior() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 1);
    rt.load_document(doc).unwrap();

    let StartResult { end_time, .. } = rt.start("fade", 0.0).unwrap();
    assert!((end_time - 1.0).abs() < 1e-9, "end_time should be 1.0");

    // t=0.0: opacity=0.0
    let diff = rt.update(0.0).changes;
    assert!(!diff.is_empty());
    let v = extract_float(&diff[0].1);
    assert!(v < 0.01, "expected ~0.0 at t=0.0, got {v}");

    // t=0.5: opacity≈0.5
    let diff = rt.update(0.5).changes;
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 0.05, "expected ~0.5 at t=0.5, got {v}");

    // t=1.0: 自然終了
    let diff = rt.update(1.0).changes;
    assert!(!diff.is_empty(), "expected final value at t=1.0");
    let opacity = diff
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("opacity");
    let v = extract_float(&opacity.1);
    assert!((v - 1.0).abs() < 0.01, "expected ~1.0, got {v}");

    // t=1.5: 終了後 — 差分なし
    let diff = rt.update(1.5).changes;
    assert!(diff.is_empty(), "expected empty diff after natural end");
}

// =========================================================================
// 有限ループ（loop_count=3）の3周再生
// =========================================================================

#[test]
fn finite_loop_3_cycles() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 3);
    rt.load_document(doc).unwrap();

    let StartResult { end_time, .. } = rt.start("fade", 0.0).unwrap();
    // end_time は1周分（1.0s）。ループ中に advance される
    assert!(
        (end_time - 1.0).abs() < 1e-9,
        "initial end_time should be 1.0 (1 cycle)"
    );

    // 周回1: t=0.0→1.0
    let _ = rt.update(0.0); // opacity=0.0

    let diff = rt.update(0.5).changes; // 周回1の中間 → opacity≈0.5
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 0.05, "cycle 1 t=0.5: {v}");

    // 周回1→2の遷移 (t=1.25): 周回2 の 0.25s 地点 → opacity ≈ 0.25
    // 前回値 0.5 → 0.25 なので diff が発生する
    let diff = rt.update(1.25).changes;
    assert!(
        !diff.is_empty(),
        "should report value change at cycle 2 (t=1.25)"
    );
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.25).abs() < 0.05,
        "cycle 2 t=1.25: expected ~0.25, got {v}"
    );

    // 周回2→3の遷移 (t=2.75): 周回3 の 0.75s 地点 → opacity ≈ 0.75
    let diff = rt.update(2.75).changes;
    assert!(
        !diff.is_empty(),
        "should report value change at cycle 3 (t=2.75)"
    );
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.75).abs() < 0.05,
        "cycle 3 t=2.75: expected ~0.75, got {v}"
    );

    // t=3.0: 3周完了 → Conclude (自然終了)
    let diff = rt.update(3.0).changes;
    // 最終値 (opacity=1.0) が配信される
    let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
    if let Some((_, val)) = opacity {
        let v = extract_float(val);
        assert!((v - 1.0).abs() < 0.01, "expected final value 1.0, got {v}");
    }

    // t=3.5: 終了後 — 差分なし
    let diff = rt.update(3.5).changes;
    assert!(diff.is_empty(), "expected empty diff after loop completion");
}

// =========================================================================
// 無限ループ（loop_count=-1）の継続再生
// =========================================================================

#[test]
fn infinite_loop_continues_until_cancel() {
    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", -1);
    rt.load_document(doc).unwrap();

    let StartResult {
        group_id, end_time, ..
    } = rt.start("fade", 0.0).unwrap();
    // end_time は1周分（INFINITY ではない）
    assert!(
        (end_time - 1.0).abs() < 1e-9,
        "end_time = 1.0 (not INFINITY)"
    );

    // 複数周回を進める
    let _ = rt.update(0.0); // opacity=0.0

    // 周回5の中間地点 t=4.5 → effective ≈ 0.5, opacity ≈ 0.5
    let diff = rt.update(4.5).changes;
    assert!(!diff.is_empty(), "should still be playing at t=4.5");
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 0.05, "cycle 5 at t=4.5: {v}");

    // 周回10の 0.25s 地点 t=9.25 → effective ≈ 0.25, opacity ≈ 0.25
    // 前回値 ≈0.5 → 0.25 なので diff 発生
    let diff = rt.update(9.25).changes;
    assert!(!diff.is_empty(), "should still be playing at t=9.25");
    let v = extract_float(&diff[0].1);
    assert!((v - 0.25).abs() < 0.05, "cycle 10 at t=9.25: {v}");

    // Cancel で停止
    rt.cancel(group_id).unwrap();

    // 以降差分なし
    let diff = rt.update(10.0).changes;
    assert!(diff.is_empty(), "expected empty after cancel");
}

// =========================================================================
// 複数周回一括処理（大きな dt）
// =========================================================================

#[test]
fn multi_loop_batch_processing() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 5);
    rt.load_document(doc).unwrap();

    let StartResult { .. } = rt.start("fade", 0.0).unwrap();

    let _ = rt.update(0.0);

    // 一度に3周分進める (t=3.5 → 周回4の 0.5s 地点)
    // loop_count=5 なので、まだ再生中
    let diff = rt.update(3.5).changes;
    assert!(
        !diff.is_empty(),
        "should still be playing after 3 cycles skip"
    );

    // 残り全体進める: t=5.0 → 5周完了 → Conclude
    let diff = rt.update(5.0).changes;
    let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
    if let Some((_, val)) = opacity {
        let v = extract_float(val);
        assert!((v - 1.0).abs() < 0.01, "final value after 5 cycles: {v}");
    }

    // 終了後
    let diff = rt.update(5.5).changes;
    assert!(diff.is_empty(), "expected empty after completion");
}

// =========================================================================
// Pause + ループの正確な再開
// =========================================================================

#[test]
fn pause_during_loop_resumes_correctly() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 3);
    rt.load_document(doc).unwrap();

    let StartResult { group_id, .. } = rt.start("fade", 0.0).unwrap();

    let _ = rt.update(0.0); // opacity=0.0

    // t=0.5: 周回1の中間 → opacity ≈ 0.5
    let diff = rt.update(0.5).changes;
    assert!(!diff.is_empty());
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 0.05, "before pause: {v}");

    // Pause at t=0.5
    rt.pause(group_id, 0.5).unwrap();

    // t=1.0: Pause 中 → 差分なし
    let diff = rt.update(1.0).changes;
    assert!(diff.is_empty(), "should freeze during pause");

    // Resume at t=1.0 (0.5s pause)
    let _new_end = rt.resume(group_id, 1.0).unwrap();

    // After resume: effective_time = (current_time - loop_start_time - pause_accumulated) * time_scale
    // At t=1.25: (1.25 - 0.0 - 0.5) * 1.0 = 0.75 → opacity ≈ 0.75
    // 前回値 0.5 → 0.75 なので diff 発生
    let diff = rt.update(1.25).changes;
    assert!(!diff.is_empty(), "should be playing after resume");
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.75).abs() < 0.05,
        "t=1.25 after pause: expected ~0.75, got {v}"
    );

    // 周回1 → 2 遷移: end_time = start(0.0) + loop_duration(1.0) + pause(0.5) = 1.5
    // At t=1.75: process_loops advances loop_start_time=1.0, end_time=2.5
    // effective = (1.75 - 1.0 - 0.5) * 1.0 = 0.25 → opacity ≈ 0.25
    // 前回値 0.75 → 0.25 なので diff 発生
    let diff = rt.update(1.75).changes;
    assert!(!diff.is_empty(), "should be in cycle 2 at t=1.75");
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.25).abs() < 0.05,
        "cycle 2 t=1.75: expected ~0.25, got {v}"
    );

    // 3周完了: end_time が 3.5 (0 + 3*1.0 + 0.5) に到達するはず
    // t=3.5: effective = (3.5 - 2.0 - 0.5) * 1.0 = 1.0 → 周回3終了
    let diff = rt.update(3.5).changes;
    let opacity = diff.iter().find(|(id, _)| *id == opacity_id);
    if let Some((_, val)) = opacity {
        let v = extract_float(val);
        assert!((v - 1.0).abs() < 0.01, "final value: {v}");
    }

    // 終了後
    let diff = rt.update(4.0).changes;
    assert!(
        diff.is_empty(),
        "expected empty after loop completion with pause"
    );
}

// =========================================================================
// Cancel + ループの即座停止
// =========================================================================

#[test]
fn cancel_during_loop_stops_immediately() {
    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", -1); // 無限ループ
    rt.load_document(doc).unwrap();

    let StartResult { group_id, .. } = rt.start("fade", 0.0).unwrap();

    let _ = rt.update(0.0);

    // t=5.5: 無限ループ再生中
    let diff = rt.update(5.5).changes;
    assert!(!diff.is_empty(), "should be playing during infinite loop");

    // Cancel
    rt.cancel(group_id).unwrap();

    // 以降差分なし
    let diff = rt.update(6.0).changes;
    assert!(diff.is_empty(), "expected empty after cancel");

    // 再度 Cancel は InvalidGroupId
    let err = rt.cancel(group_id).unwrap_err();
    assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
}

// =========================================================================
// end_time 統一テスト（loop_count=1 vs -1）
// =========================================================================

#[test]
fn end_time_unified_for_single_and_infinite() {
    let mut rt = DolaRuntime::new();

    // loop_count=1
    let doc1 = loop_doc("fade", 1);
    rt.load_document(doc1).unwrap();
    let r1 = rt.start("fade", 0.0).unwrap();

    // loop_count=-1
    let doc2 = loop_doc("fade", -1);
    rt.load_document(doc2).unwrap();
    let r2 = rt.start("fade", 0.0).unwrap();

    // 両方とも end_time = 1.0 (1周分)
    assert!(
        (r1.end_time - r2.end_time).abs() < 1e-9,
        "end_time should be unified: {} vs {}",
        r1.end_time,
        r2.end_time
    );
}

// =========================================================================
// 短周期無限ループエラー
// =========================================================================

#[test]
fn short_duration_infinite_loop_error() {
    let mut rt = DolaRuntime::new();

    let doc = short_duration_loop_doc("fade", -1);
    rt.load_document(doc).unwrap();

    let err = rt.start("fade", 0.0).unwrap_err();
    assert!(
        matches!(err, RuntimeError::TooShortDurationWithInfiniteLoop { .. }),
        "expected TooShortDurationWithInfiniteLoop, got {err:?}"
    );
}

// =========================================================================
// 短周期有限ループ許可
// =========================================================================

#[test]
fn short_duration_finite_loop_allowed() {
    let mut rt = DolaRuntime::new();

    let doc = short_duration_loop_doc("fade", 10);
    rt.load_document(doc).unwrap();

    // 有限ループは短周期でもエラーなし
    let result = rt.start("fade", 0.0);
    assert!(result.is_ok(), "finite loop should allow short duration");
}
