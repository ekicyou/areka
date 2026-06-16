#![allow(deprecated)]
//! DolaRuntime Facade 統合テスト
//!
//! Task 8.1〜8.5: load → start → update → 各終了フロー → 差分配信の
//! エンドツーエンド検証。
//!
//! このモジュールは挙動を変えずに研究シーム（research.md §R3）ごとに分割されている:
//! - `load_start` … load/start/calculate_end_time（成功・エラー・time_scale 境界）
//! - `update_flow` … update ループ挙動（pause/resume・同時再生）
//! - `termination` … conclude/cancel/finish フロー
//! - `diff_delivery` … 差分配信（指示書差し替え・購読 API）

use std::collections::BTreeMap;

use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, StartResult};
use dola::{
    AnimationVariableDef, DolaDocument, StoryboardBuilder, StoryboardEntry, TransitionDef,
    TransitionRef, TransitionValue,
};

mod diff_delivery;
mod load_start;
mod termination;
mod update_flow;

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

/// Float 2 変数 + 2 ストーリーボードの並行再生向けドキュメント。
fn dual_variable_doc() -> DolaDocument {
    let mut variable = BTreeMap::new();
    variable.insert(
        "opacity".to_string(),
        AnimationVariableDef::Float {
            initial: 0.0,
            min: Some(0.0),
            max: Some(1.0),
        },
    );
    variable.insert(
        "scale".to_string(),
        AnimationVariableDef::Float {
            initial: 1.0,
            min: Some(0.0),
            max: Some(2.0),
        },
    );

    // sb_fade: opacity 0→1 in 1.0s
    let sb_fade = StoryboardBuilder::new()
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

    // sb_zoom: scale 1→2 in 2.0s
    let sb_zoom = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("scale".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                from: Some(TransitionValue::Scalar(1.0)),
                to: Some(TransitionValue::Scalar(2.0)),
                relative_to: None,
                easing: None,
                delay: 0.0,
                duration: Some(2.0),
            })),
            ..Default::default()
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("fade".to_string(), sb_fade);
    storyboard.insert("zoom".to_string(), sb_zoom);
    DolaDocument {
        schema_version: "1.0".to_string(),
        variable,
        transition: BTreeMap::new(),
        storyboard,
    }
}
