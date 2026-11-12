//! ランタイムのテストの共通の道具

use dola::{AnimationVariableDef, StoryboardEntry, TransitionDef, TransitionRef, TransitionValue};

/// 変数 `var` を `from` から `to` へ `dur` 秒で線形に動かすエントリ。
pub fn linear(var: &str, from: f64, to: f64, dur: f64) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(from)),
            to: Some(TransitionValue::Scalar(to)),
            duration: Some(dur),
            ..Default::default()
        })),
        ..Default::default()
    }
}

/// 初期値 0・値域なしの Float 変数。
pub fn float() -> AnimationVariableDef {
    AnimationVariableDef::Float {
        initial: 0.0,
        min: None,
        max: None,
    }
}
