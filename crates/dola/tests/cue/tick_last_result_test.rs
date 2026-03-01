#![allow(deprecated)]
//! DolaRuntime tick/last_result テスト
//!
//! Task 6.4: tick() + last_result() の旧 update() との等価性、冪等性、後方互換の検証。

use std::collections::BTreeMap;

use dola::runtime::{DolaRuntime, EvaluatedValue};
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

// =========================================================================
// tick + last_result が旧 update と同等結果を返す
// =========================================================================

#[test]
fn tick_last_result_equivalent_to_update() {
    // update() で実行するランタイム
    let mut rt_update = DolaRuntime::new();
    let opacity_id_u = rt_update.subscribe("opacity");
    rt_update.load_document(simple_float_doc("fade")).unwrap();
    rt_update.start("fade", 0.0).unwrap();

    // tick() + last_result() で実行するランタイム
    let mut rt_tick = DolaRuntime::new();
    let opacity_id_t = rt_tick.subscribe("opacity");
    rt_tick.load_document(simple_float_doc("fade")).unwrap();
    rt_tick.start("fade", 0.0).unwrap();

    assert_eq!(opacity_id_u, opacity_id_t);

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let update_result = rt_update.update(t);
        rt_tick.tick(t);
        let tick_result = rt_tick.last_result();

        // 変更数が同一
        assert_eq!(
            update_result.changes.len(),
            tick_result.changes.len(),
            "changes count mismatch at t={t}"
        );

        // 各変更値が一致
        for (i, ((uid, uval), (tid, tval))) in update_result
            .changes
            .iter()
            .zip(tick_result.changes.iter())
            .enumerate()
        {
            assert_eq!(uid, tid, "variable id mismatch at t={t}, index={i}");
            match (uval, tval) {
                (EvaluatedValue::Float(a), EvaluatedValue::Float(b)) => {
                    assert!(
                        (a - b).abs() < 1e-9,
                        "float value mismatch at t={t}: update={a}, tick={b}"
                    );
                }
                _ => {
                    // 他の型は直接比較
                    assert_eq!(uval, tval, "value mismatch at t={t}, index={i}");
                }
            }
        }
    }
}

// =========================================================================
// last_result 冪等性（tick なしで同一結果を返す）
// =========================================================================

#[test]
fn last_result_idempotent_without_tick() {
    let mut rt = DolaRuntime::new();
    rt.subscribe("opacity");
    rt.load_document(simple_float_doc("fade")).unwrap();
    rt.start("fade", 0.0).unwrap();

    rt.tick(0.5);

    let result1 = rt.last_result().clone();
    let result2 = rt.last_result().clone();

    // 同一内容
    assert_eq!(result1.changes.len(), result2.changes.len());
    for ((id1, v1), (id2, v2)) in result1.changes.iter().zip(result2.changes.iter()) {
        assert_eq!(id1, id2);
        assert_eq!(v1, v2);
    }
}

// =========================================================================
// last_result は初期状態で空
// =========================================================================

#[test]
fn last_result_empty_before_first_tick() {
    let rt = DolaRuntime::new();
    let result = rt.last_result();
    assert!(result.changes.is_empty());
    assert!(result.triggered.is_empty());
}

// =========================================================================
// deprecated update() 後方互換
// =========================================================================

#[test]
fn deprecated_update_still_works() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(simple_float_doc("fade")).unwrap();
    rt.start("fade", 0.0).unwrap();

    // deprecated update() は内部で tick + last_result().clone() を呼ぶ
    let result = rt.update(0.5);
    assert!(!result.changes.is_empty());
    let (id, val) = &result.changes[0];
    assert_eq!(*id, opacity_id);
    match val {
        EvaluatedValue::Float(v) => {
            assert!((*v - 0.5).abs() < 0.05, "expected ~0.5, got {v}");
        }
        other => panic!("expected Float, got {other:?}"),
    }
}

// =========================================================================
// tick 後に last_result が更新される
// =========================================================================

#[test]
fn tick_updates_last_result() {
    let mut rt = DolaRuntime::new();
    rt.subscribe("opacity");
    rt.load_document(simple_float_doc("fade")).unwrap();
    rt.start("fade", 0.0).unwrap();

    rt.tick(0.0);
    let r0 = rt.last_result().clone();

    rt.tick(0.5);
    let r1 = rt.last_result().clone();

    // t=0.0 と t=0.5 で値が異なるはず
    assert!(!r0.changes.is_empty());
    assert!(!r1.changes.is_empty());

    // opacity の値が異なる
    let val0 = match &r0.changes[0].1 {
        EvaluatedValue::Float(v) => *v,
        _ => panic!("expected Float"),
    };
    let val1 = match &r1.changes[0].1 {
        EvaluatedValue::Float(v) => *v,
        _ => panic!("expected Float"),
    };
    assert!(
        (val1 - val0).abs() > 0.1,
        "values should differ: {val0} vs {val1}"
    );
}
