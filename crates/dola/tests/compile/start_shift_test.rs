//! コンパイルの結果が開始の時刻に依らない事のテスト
//!
//! 同じ文書を違う開始の時刻で始めると、値と引き金が開始の時刻の差だけずれて同じになる。

use dola::runtime::{DolaRuntime, EvaluatedValue, TriggerResult};
use dola::*;

use super::common::{delta_ms, float_var, ms, transition_entry};

/// 倍率 0.5 の親: x は台本の 500〜1500 ms （経った長さで 1〜3 秒）、子の引き金は台本の 750 ms
fn doc() -> DolaDocument {
    let parent = StoryboardBuilder::new()
        .time_scale(0.5)
        .entry(transition_entry("x", ms(500), Some(ms(1000))))
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: Some(KeyframeRef::WithOffset {
                keyframes: KeyframeNames::Single("start".to_string()),
                offset: delta_ms(750),
            }),
            ..Default::default()
        })
        .build();
    let child = StoryboardBuilder::new()
        .entry(transition_entry("y", ms(0), Some(ms(1000))))
        .build();
    DolaDocumentBuilder::new("1.0")
        .variable("x", float_var())
        .variable("y", float_var())
        .storyboard("parent", parent)
        .storyboard("child", child)
        .build()
        .unwrap()
}

/// 刻みごとの値の変化と引き金。引き金の終わりの時刻は開始の時刻からの隔たりにそろえる
type Step = (Vec<(i64, EvaluatedValue)>, Vec<(String, Option<TimeDelta>)>);

fn run(start: TimePoint) -> Vec<Step> {
    let mut runtime = DolaRuntime::new();
    runtime.load_document(doc()).unwrap();
    runtime.subscribe("x");
    runtime.subscribe("y");
    let started = runtime.start("parent", start).unwrap();
    assert_eq!(
        started.end_time.map(|t| t - start),
        Some(delta_ms(3000)),
        "1 周の経った長さ 3 秒で終わる"
    );
    [0, 999, 1000, 1499, 1500, 2000, 2500, 3000, 3500]
        .into_iter()
        .map(|elapsed| {
            runtime.tick(start + ms(elapsed));
            let result = runtime.last_result();
            let triggered = result
                .triggered
                .iter()
                .map(|t| match t {
                    TriggerResult::Started {
                        target_storyboard,
                        start_result,
                        ..
                    } => (
                        target_storyboard.clone(),
                        start_result.end_time.map(|t| t - start),
                    ),
                    TriggerResult::Error { error, .. } => panic!("trigger failed: {error:?}"),
                })
                .collect();
            (result.changes.clone(), triggered)
        })
        .collect()
}

#[test]
fn values_and_triggers_shift_exactly_by_the_start_difference() {
    let at_zero = run(TimePoint::ZERO);
    assert!(
        at_zero.iter().any(|(changes, _)| !changes.is_empty()),
        "値が変わる"
    );
    let fired: Vec<_> = at_zero.iter().flat_map(|(_, t)| t.clone()).collect();
    assert_eq!(
        fired,
        [("child".to_string(), Some(delta_ms(2500)))],
        "引き金は経った長さ 1500 ms で 1 度だけ、子は 1 秒で終わる"
    );
    for start in [TimePoint::from_ticks(370_000_001), TimePoint::from_ticks(1)] {
        assert_eq!(run(start), at_zero, "start {start}");
    }
}
