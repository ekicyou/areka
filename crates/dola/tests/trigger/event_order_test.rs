//! 1 回の tick の中の出来事（トリガー・周回の終わり・finish の期限）を時刻の順に処理する（R3-01・R3-02）。
//! 60fps で回しても、tick を大きく飛ばしても同じ結果になる

use super::runtime_test::three_loop_trigger_doc;
use dola::runtime::{DolaRuntime, EvaluatedValue, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocument, DolaDocumentBuilder, InterruptionPolicy, KeyframeNames,
    KeyframeRef, StoryboardBuilder, StoryboardEntry, TransitionDef, TransitionRef, TransitionValue,
};

fn ramp(var: &str, from: f64, to: f64, duration: f64) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(from)),
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(duration),
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// 親は自分の変数 `var` を `loop_len` 秒動かし、周回の頭から `at` 秒で `child` を起こす
fn parent(var: &str, loop_len: f64, loops: i32, at: f64, child: &str) -> dola::Storyboard {
    StoryboardBuilder::new()
        .loop_count(loops)
        .entry(ramp(var, 0.0, 1.0, loop_len))
        .entry(StoryboardEntry {
            trigger_storyboard: Some(child.to_string()),
            at: Some(KeyframeRef::WithOffset {
                keyframes: KeyframeNames::Single("start".to_string()),
                offset: at,
            }),
            ..Default::default()
        })
        .build()
}

/// 子 A は x を 10→11、子 B は x を 20→21（どちらも 5 秒・Cancel）
fn two_parent_doc(p1: dola::Storyboard, p2: dola::Storyboard) -> DolaDocument {
    let float = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    let child = |from: f64| {
        StoryboardBuilder::new()
            .interruption_policy(InterruptionPolicy::Cancel)
            .entry(ramp("x", from, from + 1.0, 5.0))
            .build()
    };
    DolaDocumentBuilder::new("1.0")
        .variable("a", float())
        .variable("b", float())
        .variable("x", float())
        .storyboard("p1", p1)
        .storyboard("p2", p2)
        .storyboard("A", child(10.0))
        .storyboard("B", child(20.0))
        .build()
        .expect("doc should be valid")
}

/// p1（group 1）・p2（group 2）を 0 秒に起こし、`times` の順に tick して最後の x を返す
fn final_x(doc: &DolaDocument, times: &[f64]) -> f64 {
    let mut rt = DolaRuntime::new();
    let x_id = rt.subscribe("x");
    rt.load_document(doc.clone()).unwrap();
    assert_eq!(rt.start("p1", 0.0).unwrap().group_id, 1);
    assert_eq!(rt.start("p2", 0.0).unwrap().group_id, 2);
    let mut x = f64::NAN;
    for &t in times {
        rt.tick(t);
        for (id, v) in &rt.last_result().changes {
            if *id == x_id
                && let EvaluatedValue::Float(f) = v
            {
                x = *f;
            }
        }
    }
    x
}

fn frames_60fps(end: f64) -> Vec<f64> {
    (0..=(end * 60.0).round() as u32)
        .map(|k| k as f64 / 60.0)
        .collect()
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{what}: x = {actual}, expected {expected}"
    );
}

/// p1 は 0.9 秒に A、p2 は 0.1 秒に B を起こす。時刻の上で後の A が B を終わらせる
#[test]
fn triggers_in_one_tick_fire_in_time_order() {
    let doc = two_parent_doc(parent("a", 1.0, 1, 0.9, "A"), parent("b", 1.0, 1, 0.1, "B"));
    // A は 0.9 秒に起き、1.0 秒で 0.1 秒ぶん進む
    assert_close(final_x(&doc, &frames_60fps(1.0)), 10.02, "60fps");
    assert_close(final_x(&doc, &[0.0, 1.0]), 10.02, "one tick");
}

/// 周回を飛ばしても、最後に起きるのは時刻の上で最後の A（P1 の 3 周目・2.9 秒）
#[test]
fn skipped_loop_triggers_fire_in_time_order_across_parents() {
    let doc = two_parent_doc(
        parent("a", 1.0, 3, 0.9, "A"),
        parent("b", 0.25, 12, 0.0, "B"),
    );
    // A は 2.9 秒に起き、3.5 秒で 0.6 秒ぶん進む
    assert_close(final_x(&doc, &frames_60fps(3.5)), 10.12, "60fps");
    assert_close(final_x(&doc, &[0.0, 3.5]), 10.12, "one tick");
}

/// finish の期限を同じ tick で越えても、期限より前のトリガーは起きる（0.5・1.5 秒の 2 回）
#[test]
fn triggers_before_finish_deadline_fire_in_the_crossing_tick() {
    let fired = |times: &[f64]| {
        let mut rt = DolaRuntime::new();
        rt.load_document(three_loop_trigger_doc(0.5)).unwrap();
        let gid = rt.start("parent", 0.0).unwrap().group_id;
        rt.finish(gid, 2.0).unwrap();
        let mut n = 0;
        for &t in times {
            rt.tick(t);
            for r in &rt.last_result().triggered {
                assert!(
                    matches!(r, TriggerResult::Started { .. }),
                    "trigger failed: {r:?}"
                );
                n += 1;
            }
        }
        assert!(!rt.has_instances(), "parent and children should all end");
        n
    };
    assert_eq!(fired(&frames_60fps(3.5)), 2, "60fps");
    assert_eq!(fired(&[0.0, 3.5]), 2, "one tick");
}

/// 周回の終わりちょうどのトリガーは、一時停止の後に壁時計の時刻が end_time を丸めで越えても起きる。
/// 13/60 秒に止めて 14/60 秒に戻すと、トリガーの式の値が end_time より 1 ulp 後ろになる
#[test]
fn loop_end_trigger_survives_pause_rounding() {
    let fired = |times: &[f64]| {
        let mut rt = DolaRuntime::new();
        rt.load_document(three_loop_trigger_doc(1.0)).unwrap();
        let gid = rt.start("parent", 0.2).unwrap().group_id;
        let (pause_at, resume_at) = (13.0 / 60.0, 14.0 / 60.0);
        let (mut paused, mut resumed) = (false, false);
        let mut n = 0;
        for &t in times {
            if t >= pause_at && !paused {
                rt.pause(gid, pause_at).unwrap();
                paused = true;
            }
            if t >= resume_at && !resumed {
                rt.resume(gid, resume_at).unwrap();
                resumed = true;
            }
            rt.tick(t);
            n += rt.last_result().triggered.len();
        }
        n
    };
    // 周回の終わりは約 1.217・2.217・3.217 秒
    assert_eq!(fired(&frames_60fps(3.5)), 3, "60fps");
    assert_eq!(fired(&[0.2, 3.5]), 3, "one tick");
}
