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

use crate::support::linear;
use dola::runtime::{DolaRuntime, EvaluatedValue, RuntimeError, StartResult};
use dola::{
    AnimationVariableDef, DolaDocument, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry,
    TransitionDef, TransitionRef, TransitionValue,
};

mod diff_delivery;
mod load_start;
mod termination;
mod update_flow;

// =========================================================================
// ヘルパー
// =========================================================================

/// 値域 [min, max]・初期値 initial の Float 変数。
fn bounded(initial: f64, min: f64, max: f64) -> AnimationVariableDef {
    AnimationVariableDef::Float {
        initial,
        min: Some(min),
        max: Some(max),
    }
}

/// Float 変数 1 つ + 線形 0→1 (duration=1.0) のストーリーボードを持つ最小ドキュメント。
fn simple_float_doc(sb_name: &str) -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        .variable("opacity", bounded(0.0, 0.0, 1.0))
        .storyboard(
            sb_name,
            StoryboardBuilder::new()
                .entry(linear("opacity", 0.0, 1.0, 1.0))
                .build(),
        )
        .build()
        .unwrap()
}

/// Float 2 変数 + 2 ストーリーボードの並行再生向けドキュメント。
fn dual_variable_doc() -> DolaDocument {
    DolaDocumentBuilder::new("1.0")
        .variable("opacity", bounded(0.0, 0.0, 1.0))
        .variable("scale", bounded(1.0, 0.0, 2.0))
        // fade: opacity 0→1 in 1.0s
        .storyboard(
            "fade",
            StoryboardBuilder::new()
                .entry(linear("opacity", 0.0, 1.0, 1.0))
                .build(),
        )
        // zoom: scale 1→2 in 2.0s
        .storyboard(
            "zoom",
            StoryboardBuilder::new()
                .entry(linear("scale", 1.0, 2.0, 2.0))
                .build(),
        )
        .build()
        .unwrap()
}
