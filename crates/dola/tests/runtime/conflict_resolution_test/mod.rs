#![allow(deprecated)]
//! 競合解決（ConflictResolver）統合テスト
//!
//! Task 6.1〜6.3: detect_overlaps, 各終了戦略, エラーパスと境界条件
//!
//! このモジュールは挙動を変えずに研究シーム（research.md §R3）ごとに分割されている:
//! - `conflict_detection` … 競合検出ロジック + 境界条件
//! - `termination_strategy` … 各終了戦略（Cancel/Conclude/Trim/Compress）+ 混在ケース
//! - `error_handling` … Never 戦略のエラーパスと終了経路の特性化

use crate::support::{float, linear};
use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError};
use dola::{
    AnimationVariableDef, DolaDocument, DolaDocumentBuilder, InterruptionPolicy, StoryboardBuilder,
    StoryboardEntry, TransitionDef, TransitionRef, TransitionValue,
};

mod conflict_detection;
mod error_handling;
mod termination_strategy;

// =========================================================================
// ヘルパー
// =========================================================================

/// 指定 policy で opacity 0→100 (duration=2.0) の SB を生成
fn make_doc_with_policy(sb_name: &str, policy: InterruptionPolicy) -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        .variable(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: Some(0.0),
                max: Some(100.0),
            },
        )
        .storyboard(
            sb_name,
            StoryboardBuilder::new()
                .interruption_policy(policy)
                .entry(linear("opacity", 0.0, 100.0, 2.0))
                .build(),
        )
        .build()
        .unwrap()
}

/// 複数 SB + 複数変数のドキュメント
fn make_multi_sb_doc() -> DolaDocument {
    let sb = |policy| StoryboardBuilder::new().interruption_policy(policy);
    DolaDocumentBuilder::new("1.0")
        .variable("x", float())
        .variable("y", float())
        .variable("z", float())
        // sb_a: Cancel policy, x: 0→100 in 2.0s, y: 0→50 in 2.0s
        .storyboard(
            "sb_a",
            sb(InterruptionPolicy::Cancel)
                .entry(linear("x", 0.0, 100.0, 2.0))
                .entry(linear("y", 0.0, 50.0, 2.0))
                .build(),
        )
        // sb_b: Conclude policy, x: 200→300 in 2.0s
        .storyboard(
            "sb_b",
            sb(InterruptionPolicy::Conclude)
                .entry(linear("x", 200.0, 300.0, 2.0))
                .build(),
        )
        // sb_c: Trim policy, x: 500→600 in 2.0s
        .storyboard(
            "sb_c",
            sb(InterruptionPolicy::Trim)
                .entry(linear("x", 500.0, 600.0, 2.0))
                .build(),
        )
        // sb_new: x: 0→10 in 1.0s (これが新規に start されて、上記と競合する)
        .storyboard(
            "sb_new",
            StoryboardBuilder::new()
                .entry(linear("x", 0.0, 10.0, 1.0))
                .build(),
        )
        .build()
        .unwrap()
}

/// Never policy の SB を含むドキュメント
fn make_never_doc() -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        .variable("x", float())
        .variable("y", float())
        .variable("z", float())
        // sb_never: Never policy, x: 0→100 in 2.0s
        .storyboard(
            "sb_never",
            StoryboardBuilder::new()
                .interruption_policy(InterruptionPolicy::Never)
                .entry(linear("x", 0.0, 100.0, 2.0))
                .build(),
        )
        // sb_new: x: 0→10 in 1.0s
        .storyboard(
            "sb_new",
            StoryboardBuilder::new()
                .entry(linear("x", 0.0, 10.0, 1.0))
                .build(),
        )
        // sb_multi: x,y,z を同時に操作（部分競合テスト用）
        .storyboard(
            "sb_multi",
            StoryboardBuilder::new()
                .entry(linear("x", 0.0, 10.0, 1.0))
                .entry(linear("y", 0.0, 20.0, 1.0))
                .entry(linear("z", 0.0, 30.0, 1.0))
                .build(),
        )
        .build()
        .unwrap()
}
