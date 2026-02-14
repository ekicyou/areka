//! # Dola — Declarative Orchestration for Live Animation
//!
//! プラグイン間で共有可能なシリアライズ可能アニメーション宣言フォーマット。
//! Windows Animation Manager の概念（変数・トランジション・キーフレーム・ストーリーボード）を
//! プラットフォーム非依存のデータモデルとして再構成する。

mod builder;
mod compile;
mod document;
mod easing;
mod error;
mod playback;
#[cfg(feature = "runtime")]
pub mod runtime;
mod storyboard;
mod transition;
mod validate;
mod value;
mod variable;

pub use builder::{DolaDocumentBuilder, StoryboardBuilder};
pub use compile::{
    compile_storyboard, CompiledSegment, CompiledStoryboard, CompiledVariableTimeline,
    VariableTypeHint,
};
pub use document::DolaDocument;
pub use easing::{EasingFunction, EasingName, ParametricEasing};
pub use error::DolaError;
pub use playback::{PlaybackState, ScheduleRequest};
pub use storyboard::{
    BetweenKeyframes, InterruptionPolicy, KeyframeNames, KeyframeRef, Storyboard, StoryboardEntry,
};
pub use transition::{TransitionDef, TransitionRef, TransitionValue};
pub use validate::Validate;
pub use value::DynamicValue;
pub use variable::AnimationVariableDef;
