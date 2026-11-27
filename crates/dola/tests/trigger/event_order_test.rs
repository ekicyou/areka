//! 1 回の tick の中の出来事（トリガー・周回の終わり・finish の期限）を時刻の順に処理する。
//! 60fps で回しても、tick を大きく飛ばしても同じ結果になる

use super::common::{at_ms, delta_ms, ms};
use super::runtime_loop_test::three_loop_trigger_doc;
use dola::runtime::{DolaRuntime, EvaluatedValue, TriggerResult};
use dola::{
    AnimationVariableDef, DolaDocument, DolaDocumentBuilder, InterruptionPolicy, KeyframeNames,
    KeyframeRef, StoryboardBuilder, StoryboardEntry, TimeDelta, TimePoint, TimeSpan, TransitionDef,
    TransitionRef, TransitionValue,
};

fn ramp(var: &str, from: f64, to: f64, duration: TimeSpan) -> StoryboardEntry {
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

/// 親は自分の変数 `var` を `loop_len` の長さ動かし、周回の頭から `at` 後に `child` を起こす
fn parent(
    var: &str,
    loop_len: TimeSpan,
    loops: i32,
    at: TimeDelta,
    child: &str,
) -> dola::Storyboard {
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
            .entry(ramp("x", from, from + 1.0, ms(5000)))
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
fn final_x(doc: &DolaDocument, times: &[TimePoint]) -> f64 {
    let mut rt = DolaRuntime::new();
    let x_id = rt.subscribe("x");
    rt.load_document(doc.clone()).unwrap();
    assert_eq!(rt.start("p1", at_ms(0)).unwrap().group_id, 1);
    assert_eq!(rt.start("p2", at_ms(0)).unwrap().group_id, 2);
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

/// 60fps の 1 コマ目から数えた k コマ目の時刻（100ns の刻みへ四捨五入）。
fn frame(k: i64) -> TimePoint {
    TimePoint::from_ticks((k * 10_000_000 + 30) / 60)
}

/// 0 から `end_ms` までの 60fps のコマの時刻。
fn frames_60fps(end_ms: i64) -> Vec<TimePoint> {
    (0..=end_ms * 60 / 1000).map(frame).collect()
}

fn assert_close(actual: f64, expected: f64, var: &str, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{what}: {var} = {actual}, expected {expected}"
    );
}

/// p1 は 900 ms に A、p2 は 100 ms に B を起こす。時刻の上で後の A が B を終わらせる
#[test]
fn triggers_in_one_tick_fire_in_time_order() {
    let doc = two_parent_doc(
        parent("a", ms(1000), 1, delta_ms(900), "A"),
        parent("b", ms(1000), 1, delta_ms(100), "B"),
    );
    // A は 900 ms に起き、1000 ms で 100 ms ぶん進む
    assert_close(final_x(&doc, &frames_60fps(1000)), 10.02, "x", "60fps");
    assert_close(
        final_x(&doc, &[at_ms(0), at_ms(1000)]),
        10.02,
        "x",
        "one tick",
    );
}

/// 周回を飛ばしても、最後に起きるのは時刻の上で最後の A（P1 の 3 周目・2900 ms）
#[test]
fn skipped_loop_triggers_fire_in_time_order_across_parents() {
    let doc = two_parent_doc(
        parent("a", ms(1000), 3, delta_ms(900), "A"),
        parent("b", ms(250), 12, delta_ms(0), "B"),
    );
    // A は 2900 ms に起き、3500 ms で 600 ms ぶん進む
    assert_close(final_x(&doc, &frames_60fps(3500)), 10.12, "x", "60fps");
    assert_close(
        final_x(&doc, &[at_ms(0), at_ms(3500)]),
        10.12,
        "x",
        "one tick",
    );
}

/// finish の期限を同じ tick で越えても、期限より前のトリガーは起きる（500・1500 ms の 2 回）
#[test]
fn triggers_before_finish_deadline_fire_in_the_crossing_tick() {
    let fired = |times: &[TimePoint]| {
        let mut rt = DolaRuntime::new();
        rt.load_document(three_loop_trigger_doc(delta_ms(500)))
            .unwrap();
        let gid = rt.start("parent", at_ms(0)).unwrap().group_id;
        rt.finish(gid, at_ms(2000)).unwrap();
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
    assert_eq!(fired(&frames_60fps(3500)), 2, "60fps");
    assert_eq!(fired(&[at_ms(0), at_ms(3500)]), 2, "one tick");
}

/// 周回の終わりちょうどのトリガーは、一時停止で周回が後ろへずれた後も、ずれた周回の終わりに起きる。
/// 13 コマ目に止めて 14 コマ目に戻すと、周回の終わりは止めた長さ（166,666 tick）だけ後ろへずれる
#[test]
fn loop_end_trigger_fires_after_pause_shift() {
    let fired = |times: &[TimePoint]| {
        let mut rt = DolaRuntime::new();
        rt.load_document(three_loop_trigger_doc(delta_ms(1000)))
            .unwrap();
        let gid = rt.start("parent", at_ms(200)).unwrap().group_id;
        let (pause_at, resume_at) = (frame(13), frame(14));
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
    // 周回の終わりは 1 周目が 1216.6666 ms・以降 1 秒ずつ
    assert_eq!(fired(&frames_60fps(3500)), 3, "60fps");
    assert_eq!(fired(&[at_ms(200), at_ms(3500)]), 3, "one tick");
}

/// トリガーを持たない 1 回きりの Never の X（1200 ms）は、同じ tick の 2500 ms のトリガーより先に終わる。
/// なので、x を動かす子 C はぶつからずに起きる
#[test]
fn finite_loop_end_precedes_later_trigger_in_one_tick() {
    let float = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("a", float())
        .variable("x", float())
        .storyboard(
            "X",
            StoryboardBuilder::new()
                .interruption_policy(InterruptionPolicy::Never)
                .entry(ramp("x", 0.0, 1.0, ms(1200)))
                .build(),
        )
        .storyboard("P", parent("a", ms(3000), 1, delta_ms(2500), "C"))
        .storyboard(
            "C",
            StoryboardBuilder::new()
                .entry(ramp("x", 5.0, 6.0, ms(1000)))
                .build(),
        )
        .build()
        .expect("doc should be valid");
    let triggered = |times: &[TimePoint]| {
        let mut rt = DolaRuntime::new();
        rt.load_document(doc.clone()).unwrap();
        rt.start("X", at_ms(0)).unwrap();
        rt.start("P", at_ms(0)).unwrap();
        let mut all = Vec::new();
        for &t in times {
            rt.tick(t);
            all.extend(rt.last_result().triggered.iter().cloned());
        }
        all
    };
    for (what, times) in [
        ("60fps", frames_60fps(3000)),
        ("one tick", vec![at_ms(0), at_ms(3000)]),
    ] {
        let r = triggered(&times);
        assert_eq!(r.len(), 1, "{what}: {r:?}");
        assert!(
            matches!(r[0], TriggerResult::Started { .. }),
            "{what}: {r:?}"
        );
    }
}

/// トリガーなしの無限の L（Cancel・1 秒・x と y を 0→1）は、2500 ms に起きる子 C（x だけ）とぶつかって終わる。
/// C が触らない y は L の終わりの値（3 周目の半ば・0.5）で止まる。一度に飛ばしても同じ
#[test]
fn infinite_loop_is_current_when_trigger_ends_it_in_one_tick() {
    let float = || AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    };
    let doc = DolaDocumentBuilder::new("1.0")
        .variable("a", float())
        .variable("x", float())
        .variable("y", float())
        .storyboard(
            "L",
            StoryboardBuilder::new()
                .loop_count(-1)
                .interruption_policy(InterruptionPolicy::Cancel)
                .entry(ramp("x", 0.0, 1.0, ms(1000)))
                .entry(ramp("y", 0.0, 1.0, ms(1000)))
                .build(),
        )
        .storyboard("P", parent("a", ms(3000), 1, delta_ms(2500), "C"))
        .storyboard(
            "C",
            StoryboardBuilder::new()
                .entry(ramp("x", 5.0, 6.0, ms(1000)))
                .build(),
        )
        .build()
        .expect("doc should be valid");
    let final_y = |times: &[TimePoint]| {
        let mut rt = DolaRuntime::new();
        let y_id = rt.subscribe("y");
        rt.load_document(doc.clone()).unwrap();
        rt.start("L", at_ms(0)).unwrap();
        rt.start("P", at_ms(0)).unwrap();
        let mut y = f64::NAN;
        for &t in times {
            rt.tick(t);
            for (id, v) in &rt.last_result().changes {
                if *id == y_id
                    && let EvaluatedValue::Float(f) = v
                {
                    y = *f;
                }
            }
        }
        y
    };
    assert_close(final_y(&frames_60fps(3000)), 0.5, "y", "60fps");
    assert_close(final_y(&[at_ms(0), at_ms(3000)]), 0.5, "y", "one tick");
}
