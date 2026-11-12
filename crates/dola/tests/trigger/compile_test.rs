//! トリガー機能のテスト — 5.3: CompiledTrigger の fire_time 計算テスト

use super::common::{minimal_trigger_doc, timed_trigger_doc};
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry, TransitionDef,
    TransitionRef, TransitionValue,
};

// ============================================================
// 5.3: CompiledTrigger の fire_time 計算テスト
// ============================================================

use dola::compile_storyboard;

/// トリガーのみのエントリ（前エントリ連結パターン）→ fire_time = start_time
#[test]
fn trigger_at_start_fire_time() {
    let doc = minimal_trigger_doc("parent", "child");
    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    assert_eq!(compiled.triggers[0].fire_time, 0.0);
    // トリガーは total_base_duration に寄与しない（親に変数エントリがないので 0.0）
    assert_eq!(compiled.total_base_duration, 0.0);
}

/// KF 起点トリガー: 1秒のトランジション後にトリガー発火
#[test]
fn trigger_after_keyframe_fire_time() {
    let doc = timed_trigger_doc("parent", "child", None);
    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].target_storyboard, "child");
    // kf1 は 0+1.0 = 1.0 なので fire_time = 1.0
    assert_eq!(
        compiled.triggers[0].fire_time, 1.0,
        "fire_time should be 1.0, got {}",
        compiled.triggers[0].fire_time
    );
    // トリガーは total_base_duration に寄与しない（親の opacity 0→1 の 1 秒だけ）
    assert_eq!(
        compiled.total_base_duration, 1.0,
        "total_base_duration should be 1.0, got {}",
        compiled.total_base_duration
    );
}

/// trigger_start_offset あり
#[test]
fn trigger_with_start_offset_compiled() {
    let doc = timed_trigger_doc("parent", "child", Some(0.5));
    let compiled = compile_storyboard(&doc, "parent", 0.0).unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].start_offset, Some(0.5));
}

/// start_time オフセット付きコンパイル
#[test]
fn trigger_fire_time_with_start_time_offset() {
    let doc = timed_trigger_doc("parent", "child", None);
    let compiled = compile_storyboard(&doc, "parent", 10.0).unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    // fire_time = start_time(10.0) + duration(1.0) = 11.0
    assert_eq!(
        compiled.triggers[0].fire_time, 11.0,
        "fire_time should be 11.0, got {}",
        compiled.triggers[0].fire_time
    );
}

/// start から offset 秒のキーフレーム参照
fn start_plus(offset: f64) -> Option<dola::KeyframeRef> {
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
                duration: Some(1.0),
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

/// 遅く発火するトリガーが配列の前（＝処理の順でも前）にあっても fire_time の順に並ぶ（#48）
#[test]
fn late_trigger_earlier_in_array_sorted_by_fire_time() {
    let parent_sb = StoryboardBuilder::new()
        // entry 0: opacity 0→1 (2s), kf = "a"
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(2.0),
                ..Default::default()
            })),
            keyframe: Some("a".to_string()),
            ..Default::default()
        })
        // entry 1: "a"（2.0）で発火
        .entry(trigger(
            "child_a",
            Some(dola::KeyframeRef::Single("a".to_string())),
        ))
        // entry 2: start+0.5 で発火
        .entry(trigger("child_b", start_plus(0.5)))
        .build();

    let compiled = compile_storyboard(&doc_with_children(parent_sb), "parent", 0.0).unwrap();

    let order: Vec<usize> = compiled
        .triggers
        .iter()
        .map(|t| t.source_entry_index)
        .collect();
    assert_eq!(order, vec![2, 1]);
    assert_eq!(compiled.triggers[0].fire_time, 0.5);
    assert_eq!(compiled.triggers[1].fire_time, 2.0);
}

/// 同時刻のトリガーは処理の順でなく元のエントリの番号の順に並ぶ（#48）
#[test]
fn same_fire_time_triggers_sorted_by_source_entry_index() {
    let parent_sb = StoryboardBuilder::new()
        // entry 0: "k"（後ろの entry 2 が定める＝処理は entry 1 より後）で発火
        .entry(trigger(
            "child_a",
            Some(dola::KeyframeRef::Single("k".to_string())),
        ))
        // entry 1: start+1.0 で発火（依存なし）
        .entry(trigger("child_b", start_plus(1.0)))
        // entry 2: 純粋なキーフレーム "k" = start+1.0
        .entry(StoryboardEntry {
            keyframe: Some("k".to_string()),
            at: start_plus(1.0),
            ..Default::default()
        })
        .build();

    let compiled = compile_storyboard(&doc_with_children(parent_sb), "parent", 0.0).unwrap();

    let order: Vec<usize> = compiled
        .triggers
        .iter()
        .map(|t| t.source_entry_index)
        .collect();
    assert_eq!(order, vec![0, 1]);
    assert_eq!(compiled.triggers[0].fire_time, 1.0);
    assert_eq!(compiled.triggers[1].fire_time, 1.0);
}

/// between のトリガーは between の始まり（from）の時刻で発火する（#31・要件 11.10）
#[test]
fn trigger_between_fires_at_from_keyframe() {
    let kf = |name: &str, offset: f64| StoryboardEntry {
        keyframe: Some(name.to_string()),
        at: start_plus(offset),
        ..Default::default()
    };
    let parent_sb = StoryboardBuilder::new()
        .entry(kf("a", 1.0))
        .entry(kf("b", 3.0))
        .entry(kf("c", 10.0))
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child_a".to_string()),
            between: Some(dola::BetweenKeyframes {
                from: "a".to_string(),
                to: "b".to_string(),
            }),
            ..Default::default()
        })
        .build();

    let compiled = compile_storyboard(&doc_with_children(parent_sb), "parent", 0.0).unwrap();

    assert_eq!(compiled.triggers.len(), 1);
    assert_eq!(compiled.triggers[0].fire_time, 1.0);
}
