//! トリガー機能のテスト — CompiledTrigger の fire_time 計算テスト

use super::common::{minimal_trigger_doc, timed_trigger_doc};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry, TimeDelta,
    TimeSpan, TransitionDef, TransitionRef, TransitionValue,
};

const fn ms(millis: u32) -> TimeSpan {
    TimeSpan::from_millis(millis)
}

const fn delta_ms(millis: i64) -> TimeDelta {
    TimeDelta::from_millis_saturating(millis)
}

// ============================================================
// CompiledTrigger の fire_time 計算テスト
// ============================================================

use dola::compile_storyboard;

/// トリガーのみのエントリ（前エントリ連結パターン）→ fire_time = 頭（0）
#[test]
fn trigger_at_start_fire_time() {
    let doc = minimal_trigger_doc("parent", "child");
    let compiled = compile_storyboard(&doc, "parent").unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    assert_eq!(compiled.triggers[0].fire_time, delta_ms(0));
    // トリガーは total_base_duration に寄与しない（親に変数エントリがないので 0）
    assert_eq!(compiled.total_base_duration, ms(0));
}

/// KF 起点トリガー: 1秒のトランジション後にトリガー発火
#[test]
fn trigger_after_keyframe_fire_time() {
    let doc = timed_trigger_doc("parent", "child", None);
    let compiled = compile_storyboard(&doc, "parent").unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    // kf1 は 0+1000 = 1000 ms なので fire_time = 1000 ms
    assert_eq!(
        compiled.triggers[0].fire_time,
        delta_ms(1000),
        "fire_time should be 1.0, got {}",
        compiled.triggers[0].fire_time
    );
    // トリガーは total_base_duration に寄与しない（親の opacity 0→1 の 1 秒だけ）
    assert_eq!(
        compiled.total_base_duration,
        ms(1000),
        "total_base_duration should be 1.0, got {}",
        compiled.total_base_duration
    );
}

/// trigger_start_offset あり
#[test]
fn trigger_with_start_offset_compiled() {
    let doc = timed_trigger_doc("parent", "child", Some(delta_ms(500)));
    let compiled = compile_storyboard(&doc, "parent").unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].start_offset, Some(delta_ms(500)));
}

/// start から offset 秒のキーフレーム参照
fn start_plus(offset: TimeDelta) -> Option<dola::KeyframeRef> {
    Some(dola::KeyframeRef::WithOffset {
        keyframes: dola::KeyframeNames::Single("start".to_string()),
        offset,
    })
}

fn trigger(target: &str, at: Option<dola::KeyframeRef>) -> StoryboardEntry {
    StoryboardEntry {
        trigger_storyboard: Some(target.to_string()),
        at,
        ..Default::default()
    }
}

fn doc_with_children(parent_sb: dola::Storyboard) -> dola::DolaDocument {
    let child_sb = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(1000)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();
    DolaDocumentBuilder::new("1.0")
        .variable(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        )
        .storyboard("parent", parent_sb)
        .storyboard("child_a", child_sb.clone())
        .storyboard("child_b", child_sb)
        .build()
        .expect("doc should be valid")
}

/// 遅く発火するトリガーが配列の前（＝処理の順でも前）にあっても fire_time の順に並ぶ
#[test]
fn late_trigger_earlier_in_array_sorted_by_fire_time() {
    let parent_sb = StoryboardBuilder::new()
        // entry 0: opacity 0→1 (2s), kf = "a"
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(ms(2000)),
                ..Default::default()
            })),
            keyframe: Some("a".to_string()),
            ..Default::default()
        })
        // entry 1: "a"（2000 ms）で発火
        .entry(trigger(
            "child_a",
            Some(dola::KeyframeRef::Single("a".to_string())),
        ))
        // entry 2: start+0.5 で発火
        .entry(trigger("child_b", start_plus(delta_ms(500))))
        .build();

    let compiled = compile_storyboard(&doc_with_children(parent_sb), "parent").unwrap();

    let order: Vec<usize> = compiled
        .triggers
        .iter()
        .map(|t| t.source_entry_index)
        .collect();
    assert_eq!(order, vec![2, 1]);
    assert_eq!(compiled.triggers[0].fire_time, delta_ms(500));
    assert_eq!(compiled.triggers[1].fire_time, delta_ms(2000));
}

/// 同時刻のトリガーは処理の順でなく元のエントリの番号の順に並ぶ
#[test]
fn same_fire_time_triggers_sorted_by_source_entry_index() {
    let parent_sb = StoryboardBuilder::new()
        // entry 0: "k"（後ろの entry 2 が定める＝処理は entry 1 より後）で発火
        .entry(trigger(
            "child_a",
            Some(dola::KeyframeRef::Single("k".to_string())),
        ))
        // entry 1: start+1.0 で発火（依存なし）
        .entry(trigger("child_b", start_plus(delta_ms(1000))))
        // entry 2: 純粋なキーフレーム "k" = start+1.0
        .entry(StoryboardEntry {
            keyframe: Some("k".to_string()),
            at: start_plus(delta_ms(1000)),
            ..Default::default()
        })
        .build();

    let compiled = compile_storyboard(&doc_with_children(parent_sb), "parent").unwrap();

    let order: Vec<usize> = compiled
        .triggers
        .iter()
        .map(|t| t.source_entry_index)
        .collect();
    assert_eq!(order, vec![0, 1]);
    assert_eq!(compiled.triggers[0].fire_time, delta_ms(1000));
    assert_eq!(compiled.triggers[1].fire_time, delta_ms(1000));
}

/// between のトリガーは between の始まり（from）の時刻で発火する
#[test]
fn trigger_between_fires_at_from_keyframe() {
    let kf = |name: &str, offset: TimeDelta| StoryboardEntry {
        keyframe: Some(name.to_string()),
        at: start_plus(offset),
        ..Default::default()
    };
    let parent_sb = StoryboardBuilder::new()
        .entry(kf("a", delta_ms(1000)))
        .entry(kf("b", delta_ms(3000)))
        .entry(kf("c", delta_ms(10000)))
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child_a".to_string()),
            between: Some(dola::BetweenKeyframes {
                from: "a".to_string(),
                to: "b".to_string(),
            }),
            ..Default::default()
        })
        .build();

    let compiled = compile_storyboard(&doc_with_children(parent_sb), "parent").unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].fire_time, delta_ms(1000));
}
