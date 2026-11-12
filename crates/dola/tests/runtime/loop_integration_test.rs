//! ループ再生 統合テスト（dola-runtime-5-loop Task 5.2）
//!
//! facade 経由でのループ再生エンドツーエンド検証。

use crate::support::linear;
use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, StartResult};
use dola::{AnimationVariableDef, DolaDocument, DolaDocumentBuilder, StoryboardBuilder};

// =========================================================================
// ヘルパー
// =========================================================================

/// loop_count 付き minimal doc: opacity 0→1（長さ `duration`）
fn loop_doc(sb_name: &str, loop_count: i32, duration: f64) -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        .variable(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        )
        .storyboard(
            sb_name,
            StoryboardBuilder::new()
                .loop_count(loop_count)
                .entry(linear("opacity", 0.0, 1.0, duration))
                .build(),
        )
        .build()
        .unwrap()
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

    let doc = loop_doc("fade", 1, 1.0);
    rt.load_document(doc).unwrap();

    let StartResult { end_time, .. } = rt.start("fade", 0.0).unwrap();
    assert!((end_time - 1.0).abs() < 1e-9, "end_time should be 1.0");

    // t=0.0: opacity=0.0
    rt.tick(0.0);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty());
    let v = extract_float(&diff[0].1);
    assert!(v.abs() < 1e-9, "expected ~0.0 at t=0.0, got {v}");

    // t=0.5: opacity≈0.5
    rt.tick(0.5);
    let diff = rt.last_result().changes.clone();
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "expected ~0.5 at t=0.5, got {v}");

    // t=1.0: 自然終了
    rt.tick(1.0);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "expected final value at t=1.0");
    let opacity = diff
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("opacity");
    let v = extract_float(&opacity.1);
    assert!((v - 1.0).abs() < 1e-9, "expected ~1.0, got {v}");

    // t=1.5: 終了後 — 差分なし
    rt.tick(1.5);
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "expected empty diff after natural end");
}

// =========================================================================
// 有限ループ（loop_count=3）の3周再生
// =========================================================================

#[test]
fn finite_loop_3_cycles() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 3, 1.0);
    rt.load_document(doc).unwrap();

    let StartResult { end_time, .. } = rt.start("fade", 0.0).unwrap();
    // end_time は1周分（1.0s）。ループ中に advance される
    assert!(
        (end_time - 1.0).abs() < 1e-9,
        "initial end_time should be 1.0 (1 cycle)"
    );

    // 周回1: t=0.0→1.0
    rt.tick(0.0); // opacity=0.0

    rt.tick(0.5);
    let diff = rt.last_result().changes.clone(); // 周回1の中間 → opacity≈0.5
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "cycle 1 t=0.5: {v}");

    // 周回1→2の遷移 (t=1.25): 周回2 の 0.25s 地点 → opacity ≈ 0.25
    // 前回値 0.5 → 0.25 なので diff が発生する
    rt.tick(1.25);
    let diff = rt.last_result().changes.clone();
    assert!(
        !diff.is_empty(),
        "should report value change at cycle 2 (t=1.25)"
    );
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.25).abs() < 1e-9,
        "cycle 2 t=1.25: expected ~0.25, got {v}"
    );

    // 周回2→3の遷移 (t=2.75): 周回3 の 0.75s 地点 → opacity ≈ 0.75
    rt.tick(2.75);
    let diff = rt.last_result().changes.clone();
    assert!(
        !diff.is_empty(),
        "should report value change at cycle 3 (t=2.75)"
    );
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.75).abs() < 1e-9,
        "cycle 3 t=2.75: expected ~0.75, got {v}"
    );

    // t=3.0: 3周完了 → Conclude (自然終了)
    rt.tick(3.0);
    let diff = rt.last_result().changes.clone();
    // 最終値 (opacity=1.0) が配信される
    let (_, val) = diff
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("final opacity after 3 cycles");
    assert_eq!(
        *val,
        EvaluatedValue::Float(1.0),
        "final value after 3 cycles"
    );

    // t=3.5: 終了後 — 差分なし
    rt.tick(3.5);
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "expected empty diff after loop completion");
}

// =========================================================================
// 無限ループ（loop_count=-1）の継続再生
// =========================================================================

#[test]
fn infinite_loop_continues_until_cancel() {
    let mut rt = DolaRuntime::new();
    let _opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", -1, 1.0);
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
    rt.tick(0.0); // opacity=0.0

    // 周回5の中間地点 t=4.5 → effective ≈ 0.5, opacity ≈ 0.5
    rt.tick(4.5);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should still be playing at t=4.5");
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "cycle 5 at t=4.5: {v}");

    // 周回10の 0.25s 地点 t=9.25 → effective ≈ 0.25, opacity ≈ 0.25
    // 前回値 ≈0.5 → 0.25 なので diff 発生
    rt.tick(9.25);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should still be playing at t=9.25");
    let v = extract_float(&diff[0].1);
    assert!((v - 0.25).abs() < 1e-9, "cycle 10 at t=9.25: {v}");

    // Cancel で停止
    rt.cancel(group_id).unwrap();

    // 以降差分なし
    rt.tick(10.0);
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "expected empty after cancel");
}

// =========================================================================
// 複数周回一括処理（大きな dt）
// =========================================================================

#[test]
fn multi_loop_batch_processing() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 5, 1.0);
    rt.load_document(doc).unwrap();

    let StartResult { .. } = rt.start("fade", 0.0).unwrap();

    rt.tick(0.0);

    // 一度に3周分進める (t=3.5 → 周回4の 0.5s 地点)
    // loop_count=5 なので、まだ再生中
    rt.tick(3.5);
    let diff = rt.last_result().changes.clone();
    assert!(
        !diff.is_empty(),
        "should still be playing after 3 cycles skip"
    );

    // 残り全体進める: t=5.0 → 5周完了 → Conclude
    rt.tick(5.0);
    let diff = rt.last_result().changes.clone();
    let (_, val) = diff
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("final opacity after 5 cycles");
    assert_eq!(
        *val,
        EvaluatedValue::Float(1.0),
        "final value after 5 cycles"
    );

    // 終了後
    rt.tick(5.5);
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "expected empty after completion");
}

// =========================================================================
// 先頭の遅れは周回ごとにくり返す（長さに入る）
// =========================================================================

/// delay 0.5・duration 1.0・2 周 → 1 周は 1.5 秒で、2 周目も [1.5,2.0] が遅れ・[2.0,3.0] が遷移。
/// 2 周目から遅れを抜いても、長さを遅れの後から数えても落ちる。
#[test]
fn leading_delay_repeats_every_loop() {
    let mut doc = loop_doc("fade", 2, 1.0);
    let entry = &mut doc.storyboard.get_mut("fade").unwrap().entry[0];
    let Some(dola::TransitionRef::Inline(tr)) = entry.transition.as_mut() else {
        panic!("inline transition")
    };
    tr.delay = 0.5;

    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("fade", 0.0).unwrap();

    let mut opacity_at = |t: f64| {
        rt.tick(t);
        let v = rt
            .last_result()
            .changes
            .iter()
            .find(|(id, _)| *id == opacity_id)
            .map(|(_, v)| extract_float(v));
        (v, rt.has_instances())
    };

    // 1 周目の遷移の中ほど
    let (v, _) = opacity_at(1.0);
    assert!((v.expect("t=1.0") - 0.5).abs() < 1e-9, "t=1.0: {v:?}");
    // 2 周目の遅れの中は 2 周目の from
    let (v, _) = opacity_at(1.75);
    assert!(v.expect("t=1.75").abs() < 1e-9, "t=1.75: {v:?}");
    // 2 周目の遷移の中ほど
    let (v, alive) = opacity_at(2.5);
    assert!((v.expect("t=2.5") - 0.5).abs() < 1e-9, "t=2.5: {v:?}");
    assert!(alive, "t=2.5 はまだ生きている");
    // 2 周目の終わり 3.0 で終わる
    let (_, alive) = opacity_at(2.99);
    assert!(alive, "t=2.99 はまだ生きている");
    let (v, alive) = opacity_at(3.0);
    assert!((v.expect("t=3.0") - 1.0).abs() < 1e-9, "t=3.0: {v:?}");
    assert!(!alive, "t=3.0 で終わる");
}

// =========================================================================
// Pause + ループの正確な再開
// =========================================================================

#[test]
fn pause_during_loop_resumes_correctly() {
    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");

    let doc = loop_doc("fade", 3, 1.0);
    rt.load_document(doc).unwrap();

    let StartResult { group_id, .. } = rt.start("fade", 0.0).unwrap();

    rt.tick(0.0); // opacity=0.0

    // t=0.5: 周回1の中間 → opacity ≈ 0.5
    rt.tick(0.5);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty());
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "before pause: {v}");

    // Pause at t=0.5
    rt.pause(group_id, 0.5).unwrap();

    // t=1.0: Pause 中 → 差分なし
    rt.tick(1.0);
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "should freeze during pause");

    // Resume at t=1.0 (0.5s pause)
    let _new_end = rt.resume(group_id, 1.0).unwrap();

    // After resume: effective_time = (current_time - loop_start_time - pause_accumulated) * time_scale
    // At t=1.25: (1.25 - 0.0 - 0.5) * 1.0 = 0.75 → opacity ≈ 0.75
    // 前回値 0.5 → 0.75 なので diff 発生
    rt.tick(1.25);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should be playing after resume");
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.75).abs() < 1e-9,
        "t=1.25 after pause: expected ~0.75, got {v}"
    );

    // 周回1 → 2 遷移: end_time = start(0.0) + loop_duration(1.0) + pause(0.5) = 1.5
    // At t=1.75: process_loops advances loop_start_time=1.0, end_time=2.5
    // effective = (1.75 - 1.0 - 0.5) * 1.0 = 0.25 → opacity ≈ 0.25
    // 前回値 0.75 → 0.25 なので diff 発生
    rt.tick(1.75);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should be in cycle 2 at t=1.75");
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.25).abs() < 1e-9,
        "cycle 2 t=1.75: expected ~0.25, got {v}"
    );

    // 3周完了: end_time が 3.5 (0 + 3*1.0 + 0.5) に到達するはず
    // t=3.5: effective = (3.5 - 2.0 - 0.5) * 1.0 = 1.0 → 周回3終了
    rt.tick(3.5);
    let diff = rt.last_result().changes.clone();
    let (_, val) = diff
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("final opacity after the paused loop");
    assert_eq!(
        *val,
        EvaluatedValue::Float(1.0),
        "final value after the paused loop"
    );

    // 終了後
    rt.tick(4.0);
    let diff = rt.last_result().changes.clone();
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

    let doc = loop_doc("fade", -1, 1.0); // 無限ループ
    rt.load_document(doc).unwrap();

    let StartResult { group_id, .. } = rt.start("fade", 0.0).unwrap();

    rt.tick(0.0);

    // t=5.5: 無限ループ再生中
    rt.tick(5.5);
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should be playing during infinite loop");

    // Cancel
    rt.cancel(group_id).unwrap();

    // 以降差分なし
    rt.tick(6.0);
    let diff = rt.last_result().changes.clone();
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
    let doc1 = loop_doc("fade", 1, 1.0);
    rt.load_document(doc1).unwrap();
    let r1 = rt.start("fade", 0.0).unwrap();

    // loop_count=-1
    let doc2 = loop_doc("fade", -1, 1.0);
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
    // start を待たず読み込みで拒む（裁定 (d)）
    let mut rt = DolaRuntime::new();

    let err = rt.load_document(loop_doc("fade", -1, 0.05)).unwrap_err();
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

    let doc = loop_doc("fade", 10, 0.05);
    rt.load_document(doc).unwrap();

    // 有限ループは短周期でもエラーなし
    let result = rt.start("fade", 0.0);
    assert!(result.is_ok(), "finite loop should allow short duration");
}
