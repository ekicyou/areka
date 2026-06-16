//! Integration tests — JSON/TOML/YAML round-trip and E2E tests
//! Tasks 11.2, 11.3, 11.4, 12.1, 12.2

use dola::*;
use std::collections::BTreeMap;

mod domain_integration;
mod e2e_full_flow;
mod serialization_round_trip;

/// 完全な DolaDocument を構築（テスト用）
fn build_complete_document() -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        // 3変数
        .variable(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(1.0),
            },
        )
        .variable(
            "char_count",
            AnimationVariableDef::Integer {
                initial: 0,
                min: Some(0),
                max: None,
                typewriter: Some("こんにちは世界".to_string()),
            },
        )
        .variable(
            "bg_image",
            AnimationVariableDef::Object {
                initial: DynamicValue::Map({
                    let mut m = BTreeMap::new();
                    m.insert(
                        "path".to_string(),
                        DynamicValue::String("default.png".to_string()),
                    );
                    m
                }),
            },
        )
        // 2トランジション
        .transition(
            "fade_in",
            TransitionDef {
                from: None,
                to: Some(TransitionValue::Scalar(1.0)),
                relative_to: None,
                easing: Some(EasingFunction::Named(EasingName::QuadraticInOut)),
                delay: 0.0,
                duration: Some(1.5),
            },
        )
        .transition(
            "typewrite",
            TransitionDef {
                from: None,
                to: Some(TransitionValue::Scalar(7.0)),
                relative_to: None,
                easing: Some(EasingFunction::Named(EasingName::Linear)),
                delay: 0.0,
                duration: Some(3.0),
            },
        )
        // SB1: greeting — 3つの配置パターン
        .storyboard(
            "greeting",
            StoryboardBuilder::new()
                .time_scale(1.0)
                // Entry 1: 前エントリ連結
                .entry(StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    transition: Some(TransitionRef::Named("fade_in".to_string())),
                    keyframe: Some("visible".to_string()),
                    ..Default::default()
                })
                // Entry 2: KF起点 (at = "visible")
                .entry(StoryboardEntry {
                    variable: Some("char_count".to_string()),
                    transition: Some(TransitionRef::Named("typewrite".to_string())),
                    at: Some(KeyframeRef::Single("visible".to_string())),
                    keyframe: Some("text_done".to_string()),
                    ..Default::default()
                })
                // Entry 3: Object型インライントランジション
                .entry(StoryboardEntry {
                    variable: Some("bg_image".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Dynamic(DynamicValue::Map({
                            let mut m = BTreeMap::new();
                            m.insert(
                                "path".to_string(),
                                DynamicValue::String("smile.png".to_string()),
                            );
                            m
                        }))),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: None,
                    })),
                    at: Some(KeyframeRef::Single("text_done".to_string())),
                    ..Default::default()
                })
                .build(),
        )
        // SB2: sync_test — KF間 + 純粋KF
        .storyboard(
            "sync_test",
            StoryboardBuilder::new()
                // Entry 1: 純粋KF
                .entry(StoryboardEntry {
                    keyframe: Some("marker_a".to_string()),
                    ..Default::default()
                })
                // Entry 2: 前エントリ連結
                .entry(StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: Some(EasingFunction::Named(EasingName::Linear)),
                        delay: 0.0,
                        duration: Some(2.0),
                    })),
                    keyframe: Some("marker_b".to_string()),
                    ..Default::default()
                })
                // Entry 3: KF間 (between)
                .entry(StoryboardEntry {
                    variable: Some("opacity".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: None,
                        to: Some(TransitionValue::Scalar(0.0)),
                        relative_to: None,
                        easing: Some(EasingFunction::Named(EasingName::Linear)),
                        delay: 0.0,
                        duration: None,
                    })),
                    between: Some(BetweenKeyframes {
                        from: "marker_a".to_string(),
                        to: "marker_b".to_string(),
                    }),
                    ..Default::default()
                })
                .build(),
        )
        .build()
        .unwrap()
}
