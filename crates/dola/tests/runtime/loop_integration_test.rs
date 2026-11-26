//! ループ再生 統合テスト
//!
//! facade 経由でのループ再生エンドツーエンド検証。

use crate::support::{at_ms, linear, ms};
use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, StartResult};
use dola::{
    AnimationVariableDef, DolaDocument, DolaDocumentBuilder, LoopOffset, LoopOffsetRange,
    StoryboardBuilder, TimePoint, TimeSpan,
};

// =========================================================================
// ヘルパー
// =========================================================================

/// loop_count 付き minimal doc: opacity 0→1（長さ `duration`）
fn loop_doc(sb_name: &str, loop_count: i32, duration: TimeSpan) -> DolaDocument {
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

    let doc = loop_doc("fade", 1, ms(1000));
    rt.load_document(doc).unwrap();

    let StartResult { end_time, .. } = rt.start("fade", at_ms(0)).unwrap();
    assert_eq!(end_time, Some(at_ms(1000)));

    // t=0.0: opacity=0.0
    rt.tick(at_ms(0));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty());
    let v = extract_float(&diff[0].1);
    assert!(v.abs() < 1e-9, "expected ~0.0 at t=0.0, got {v}");

    // t=0.5: opacity≈0.5
    rt.tick(at_ms(500));
    let diff = rt.last_result().changes.clone();
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "expected ~0.5 at t=0.5, got {v}");

    // t=1.0: 自然終了
    rt.tick(at_ms(1000));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "expected final value at t=1.0");
    let opacity = diff
        .iter()
        .find(|(id, _)| *id == opacity_id)
        .expect("opacity");
    let v = extract_float(&opacity.1);
    assert!((v - 1.0).abs() < 1e-9, "expected ~1.0, got {v}");

    // t=1.5: 終了後 — 差分なし
    rt.tick(at_ms(1500));
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

    let doc = loop_doc("fade", 3, ms(1000));
    rt.load_document(doc).unwrap();

    let StartResult { end_time, .. } = rt.start("fade", at_ms(0)).unwrap();
    // 待ちのない有限のループは全体の終わり（3 周分）が決まっている
    assert_eq!(end_time, Some(at_ms(3000)));

    // 周回1: t=0.0→1.0
    rt.tick(at_ms(0)); // opacity=0.0

    rt.tick(at_ms(500));
    let diff = rt.last_result().changes.clone(); // 周回1の中間 → opacity≈0.5
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "cycle 1 t=0.5: {v}");

    // 周回1→2の遷移 (t=1.25): 周回2 の 0.25s 地点 → opacity ≈ 0.25
    // 前回値 0.5 → 0.25 なので diff が発生する
    rt.tick(at_ms(1250));
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
    rt.tick(at_ms(2750));
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
    rt.tick(at_ms(3000));
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
    rt.tick(at_ms(3500));
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

    let doc = loop_doc("fade", -1, ms(1000));
    rt.load_document(doc).unwrap();

    let StartResult {
        group_id, end_time, ..
    } = rt.start("fade", at_ms(0)).unwrap();
    // 終わらないループには全体の終わりが無い
    assert_eq!(end_time, None);

    // 複数周回を進める
    rt.tick(at_ms(0)); // opacity=0.0

    // 周回5の中間地点 t=4.5 → effective ≈ 0.5, opacity ≈ 0.5
    rt.tick(at_ms(4500));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should still be playing at t=4.5");
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "cycle 5 at t=4.5: {v}");

    // 周回10の 0.25s 地点 t=9.25 → effective ≈ 0.25, opacity ≈ 0.25
    // 前回値 ≈0.5 → 0.25 なので diff 発生
    rt.tick(at_ms(9250));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should still be playing at t=9.25");
    let v = extract_float(&diff[0].1);
    assert!((v - 0.25).abs() < 1e-9, "cycle 10 at t=9.25: {v}");

    // Cancel で停止
    rt.cancel(group_id).unwrap();

    // 以降差分なし
    rt.tick(at_ms(10000));
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

    let doc = loop_doc("fade", 5, ms(1000));
    rt.load_document(doc).unwrap();

    let StartResult { .. } = rt.start("fade", at_ms(0)).unwrap();

    rt.tick(at_ms(0));

    // 一度に3周分進める (t=3.5 → 周回4の 0.5s 地点)
    // loop_count=5 なので、まだ再生中
    rt.tick(at_ms(3500));
    let diff = rt.last_result().changes.clone();
    assert!(
        !diff.is_empty(),
        "should still be playing after 3 cycles skip"
    );

    // 残り全体進める: t=5.0 → 5周完了 → Conclude
    rt.tick(at_ms(5000));
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
    rt.tick(at_ms(5500));
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
    let mut doc = loop_doc("fade", 2, ms(1000));
    let entry = &mut doc.storyboard.get_mut("fade").unwrap().entry[0];
    let Some(dola::TransitionRef::Inline(tr)) = entry.transition.as_mut() else {
        panic!("inline transition")
    };
    tr.delay = ms(500);

    let mut rt = DolaRuntime::new();
    let opacity_id = rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt.start("fade", at_ms(0)).unwrap();

    let mut opacity_at = |t: TimePoint| {
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
    let (v, _) = opacity_at(at_ms(1000));
    assert!((v.expect("t=1.0") - 0.5).abs() < 1e-9, "t=1.0: {v:?}");
    // 2 周目の遅れの中は 2 周目の from
    let (v, _) = opacity_at(at_ms(1750));
    assert!(v.expect("t=1.75").abs() < 1e-9, "t=1.75: {v:?}");
    // 2 周目の遷移の中ほど
    let (v, alive) = opacity_at(at_ms(2500));
    assert!((v.expect("t=2.5") - 0.5).abs() < 1e-9, "t=2.5: {v:?}");
    assert!(alive, "t=2.5 はまだ生きている");
    // 2 周目の終わり 3.0 で終わる
    let (_, alive) = opacity_at(at_ms(2990));
    assert!(alive, "t=2.99 はまだ生きている");
    let (v, alive) = opacity_at(at_ms(3000));
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

    let doc = loop_doc("fade", 3, ms(1000));
    rt.load_document(doc).unwrap();

    let StartResult { group_id, .. } = rt.start("fade", at_ms(0)).unwrap();

    rt.tick(at_ms(0)); // opacity=0.0

    // t=0.5: 周回1の中間 → opacity ≈ 0.5
    rt.tick(at_ms(500));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty());
    let v = extract_float(&diff[0].1);
    assert!((v - 0.5).abs() < 1e-9, "before pause: {v}");

    // Pause at t=0.5
    rt.pause(group_id, at_ms(500)).unwrap();

    // t=1.0: Pause 中 → 差分なし
    rt.tick(at_ms(1000));
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "should freeze during pause");

    // Resume at t=1.0 (0.5s pause)
    let _new_end = rt.resume(group_id, at_ms(1000)).unwrap();

    // After resume: effective_time = (current_time - loop_start - pause_accumulated) * time_scale
    // At t=1.25: (1.25 - 0.0 - 0.5) * 1.0 = 0.75 → opacity ≈ 0.75
    // 前回値 0.5 → 0.75 なので diff 発生
    rt.tick(at_ms(1250));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should be playing after resume");
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.75).abs() < 1e-9,
        "t=1.25 after pause: expected ~0.75, got {v}"
    );

    // 周回1 → 2 遷移: 1 周目の終わり = 0.0 + 1.0 + pause(0.5) = 1.5
    // At t=1.75: process_loops が 2 周目を進め、2 周目の終わりは 2.5
    // effective = (1.75 - 1.0 - 0.5) * 1.0 = 0.25 → opacity ≈ 0.25
    // 前回値 0.75 → 0.25 なので diff 発生
    rt.tick(at_ms(1750));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should be in cycle 2 at t=1.75");
    let v = extract_float(&diff[0].1);
    assert!(
        (v - 0.25).abs() < 1e-9,
        "cycle 2 t=1.75: expected ~0.25, got {v}"
    );

    // 3周完了: 全体の終わり 3.5 (0 + 3*1.0 + 0.5) に到達するはず
    // t=3.5: effective = (3.5 - 2.0 - 0.5) * 1.0 = 1.0 → 周回3終了
    rt.tick(at_ms(3500));
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
    rt.tick(at_ms(4000));
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

    let doc = loop_doc("fade", -1, ms(1000)); // 無限ループ
    rt.load_document(doc).unwrap();

    let StartResult { group_id, .. } = rt.start("fade", at_ms(0)).unwrap();

    rt.tick(at_ms(0));

    // t=5.5: 無限ループ再生中
    rt.tick(at_ms(5500));
    let diff = rt.last_result().changes.clone();
    assert!(!diff.is_empty(), "should be playing during infinite loop");

    // Cancel
    rt.cancel(group_id).unwrap();

    // 以降差分なし
    rt.tick(at_ms(6000));
    let diff = rt.last_result().changes.clone();
    assert!(diff.is_empty(), "expected empty after cancel");

    // 再度 Cancel は InvalidGroupId
    let err = rt.cancel(group_id).unwrap_err();
    assert!(matches!(err, RuntimeError::InvalidGroupId(_)));
}

// =========================================================================
// 全体の終わりの予定の時刻
// =========================================================================

/// 周回の間の待ちを付けた loop_doc（`wait` は (min, max)）。
fn loop_doc_with_wait(
    loop_count: i32,
    duration: TimeSpan,
    wait: (TimeSpan, TimeSpan),
) -> DolaDocument {
    let mut doc = loop_doc("fade", loop_count, duration);
    let (min, max) = wait;
    doc.storyboard.get_mut("fade").unwrap().loop_offset =
        Some(LoopOffset::Range(LoopOffsetRange {
            min,
            max,
            easing: dola::EasingFunction::Named(dola::EasingName::Linear),
        }));
    doc
}

fn planned_end(doc: DolaDocument, start: TimePoint) -> Option<TimePoint> {
    let mut rt = DolaRuntime::new();
    rt.load_document(doc).unwrap();
    rt.start("fade", start).unwrap().end_time
}

#[test]
fn planned_end_time_for_each_loop_shape() {
    let one_sec = ms(1000);
    let start = at_ms(500);

    // 1 回再生: 開始 + 周回の長さ
    assert_eq!(
        planned_end(loop_doc("fade", 1, one_sec), start),
        Some(at_ms(1500))
    );
    // 待ちなしの有限: 周回の長さ × 回数
    assert_eq!(
        planned_end(loop_doc("fade", 3, one_sec), start),
        Some(at_ms(3500))
    );
    // 固定の待ちの有限: 周回の間の待ち（回数 - 1 回）が入る
    assert_eq!(
        planned_end(loop_doc_with_wait(3, one_sec, (ms(250), ms(250))), start),
        Some(at_ms(4000))
    );
    // 引いて決まる待ちの有限: 今は決まらない
    assert_eq!(
        planned_end(loop_doc_with_wait(3, one_sec, (ms(100), ms(900))), start),
        None
    );
    // 無限: 決まらない
    assert_eq!(planned_end(loop_doc("fade", -1, one_sec), start), None);
}

/// 引いて決まる待ちの有限は、最後の周回に入った後の resume で終わりの時刻を返す。
#[test]
fn drawn_wait_finite_loop_resume_returns_end_only_in_last_lap() {
    const LAP_TICKS: i64 = 10_000_000;
    let mut rt = DolaRuntime::new();
    rt.subscribe("opacity");
    rt.load_document(loop_doc_with_wait(3, ms(1000), (ms(100), ms(200))))
        .unwrap();

    let StartResult {
        group_id, end_time, ..
    } = rt.start("fade", at_ms(0)).unwrap();
    assert_eq!(end_time, None);

    // 周回 1 で 100ms 止める
    rt.tick(at_ms(500));
    rt.pause(group_id, at_ms(500)).unwrap();
    assert_eq!(rt.resume(group_id, at_ms(600)).unwrap(), None);

    // 周回 3 は待ちが最長でも 2500ms までに始まり、最短でも 3300ms までは続く
    let now = at_ms(3000);
    rt.tick(now);
    let v = extract_float(&rt.last_result().changes[0].1);
    assert!(v > 0.0 && v < 1.0, "expected inside a lap, got {v}");
    // 線形 0→1 の値は周回の中の経った長さ ÷ 周回の長さ（tick の比）なので、tick へ戻せば正確
    let lap_start = now.as_ticks() - (v * LAP_TICKS as f64).round() as i64;
    assert!(
        (at_ms(2300).as_ticks()..=at_ms(2500).as_ticks()).contains(&lap_start),
        "expected the last lap, started at tick {lap_start}"
    );

    rt.pause(group_id, now).unwrap();
    assert_eq!(
        rt.resume(group_id, at_ms(3250)).unwrap(),
        Some(TimePoint::from_ticks(
            lap_start + LAP_TICKS + at_ms(250).as_ticks()
        ))
    );
}

// =========================================================================
// 短周期無限ループエラー
// =========================================================================

#[test]
fn short_duration_infinite_loop_error() {
    // start を待たず読み込みで拒む
    let mut rt = DolaRuntime::new();

    let err = rt.load_document(loop_doc("fade", -1, ms(50))).unwrap_err();
    assert!(
        matches!(err, RuntimeError::TooShortDurationWithInfiniteLoop { .. }),
        "expected TooShortDurationWithInfiniteLoop, got {err:?}"
    );
}

/// 下限は台本の長さでなく、倍率を通した 1 周の経った長さ（100 万 tick）で見る。
/// 倍率 0.5 なら台本 500,000 tick の周回は経った長さが 1,000,000 tick で始まり、499,999 tick は拒む。
#[test]
fn infinite_loop_lower_bound_uses_lap_length_through_time_scale() {
    let script_ticks = |ticks: i64| TimeSpan::try_from_ticks(ticks).unwrap();
    let half_speed_doc = |ticks: i64| {
        let mut doc = loop_doc("fade", -1, script_ticks(ticks));
        doc.storyboard.get_mut("fade").unwrap().time_scale = 0.5;
        doc
    };

    let mut rt = DolaRuntime::new();
    rt.load_document(half_speed_doc(500_000)).unwrap();
    assert!(rt.start("fade", at_ms(0)).is_ok());

    let mut rt = DolaRuntime::new();
    let err = rt.load_document(half_speed_doc(499_999)).unwrap_err();
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

    let doc = loop_doc("fade", 10, ms(50));
    rt.load_document(doc).unwrap();

    // 有限ループは短周期でもエラーなし
    let result = rt.start("fade", at_ms(0));
    assert!(result.is_ok(), "finite loop should allow short duration");
}
