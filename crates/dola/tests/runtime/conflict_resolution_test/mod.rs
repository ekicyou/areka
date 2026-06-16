#![allow(deprecated)]
//! 競合解決（ConflictResolver）統合テスト
//!
//! Task 6.1〜6.3: detect_overlaps, 各終了戦略, エラーパスと境界条件
//!
//! このモジュールは挙動を変えずに研究シーム（research.md §R3）ごとに分割されている:
//! - `conflict_detection` … 競合検出ロジック + 境界条件
//! - `termination_strategy` … 各終了戦略（Cancel/Conclude/Trim/Compress）+ 混在ケース
//! - `error_handling` … Never 戦略のエラーパスと終了経路の特性化

use std::collections::BTreeMap;

use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError};
use dola::{
    AnimationVariableDef, DolaDocument, InterruptionPolicy, StoryboardBuilder, StoryboardEntry,
    TransitionDef, TransitionRef, TransitionValue,
};

mod conflict_detection;
mod error_handling;
mod termination_strategy;

// =========================================================================
// ヘルパー
// =========================================================================

/// 指定 policy で opacity 0→100 (duration=2.0) の SB を生成
fn make_doc_with_policy(sb_name: &str, policy: InterruptionPolicy) -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "opacity".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(100.0),
        },
    );
    let sb = StoryboardBuilder::new()
        .interruption_policy(policy)
        .entry(StoryboardEntry {
            variable: Some("opacity".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(100.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
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

/// 複数 SB + 複数変数のドキュメント
fn make_multi_sb_doc() -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "x".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "y".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "z".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );

    // sb_a: Cancel policy, x: 0→100 in 2.0s, y: 0→50 in 2.0s
    let sb_a = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Cancel)
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(100.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(50.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .build();

    // sb_b: Conclude policy, x: 200→300 in 2.0s
    let sb_b = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Conclude)
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(200.0)),
                to: Some(TransitionValue::Scalar(300.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .build();

    // sb_c: Trim policy, x: 500→600 in 2.0s
    let sb_c = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Trim)
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(500.0)),
                to: Some(TransitionValue::Scalar(600.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .build();

    // sb_new: x: 0→10 in 1.0s (これが新規に start されて、上記と競合する)
    let sb_new = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(10.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("sb_a".to_string(), sb_a);
    storyboard.insert("sb_b".to_string(), sb_b);
    storyboard.insert("sb_c".to_string(), sb_c);
    storyboard.insert("sb_new".to_string(), sb_new);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}

/// Never policy の SB を含むドキュメント
fn make_never_doc() -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "x".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "y".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );
    variable.insert(
        "z".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    );

    // sb_never: Never policy, x: 0→100 in 2.0s
    let sb_never = StoryboardBuilder::new()
        .interruption_policy(InterruptionPolicy::Never)
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(100.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .build();

    // sb_new: x: 0→10 in 1.0s
    let sb_new = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(10.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        })
        .build();

    // sb_multi: x,y,z を同時に操作（部分競合テスト用）
    let sb_multi = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("x".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(10.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            variable: Some("y".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(20.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            variable: Some("z".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(30.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(1.0),
            })),
            ..Default::default()
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("sb_never".to_string(), sb_never);
    storyboard.insert("sb_new".to_string(), sb_new);
    storyboard.insert("sb_multi".to_string(), sb_multi);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}
