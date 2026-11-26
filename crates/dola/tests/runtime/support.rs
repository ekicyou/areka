//! ランタイムのテストの共通の道具

use dola::{
    AnimationVariableDef, StoryboardEntry, TimePoint, TimeSpan, TransitionDef, TransitionRef,
    TransitionValue,
};

/// ミリ秒の値の時刻。
pub const fn at_ms(ms: i64) -> TimePoint {
    TimePoint::from_ticks(ms * 10_000)
}

/// ミリ秒の値の長さ。
pub const fn ms(ms: u32) -> TimeSpan {
    TimeSpan::from_millis(ms)
}

/// 変数 `var` を `from` から `to` へ `dur` の長さで線形に動かすエントリ。
pub fn linear(var: &str, from: f64, to: f64, dur: TimeSpan) -> StoryboardEntry {
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
