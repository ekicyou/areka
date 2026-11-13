//! # Dola — Declarative Orchestration for Live Animation
//!
//! プラグイン間で共有可能なシリアライズ可能アニメーション宣言フォーマット。
//! Windows Animation Manager の概念（変数・トランジション・キーフレーム・ストーリーボード）を
//! プラットフォーム非依存のデータモデルとして再構成する。

mod builder;
mod compile;
pub mod cue;
mod document;
mod easing;
mod error;
pub mod runtime;
mod storyboard;
mod transition;
mod validate;
mod value;
mod variable;

pub use builder::{DolaDocumentBuilder, StoryboardBuilder};
pub use compile::{
    CompiledSegment, CompiledStoryboard, CompiledTrigger, CompiledVariableTimeline,
    VariableTypeHint, compile_storyboard,
};
pub use document::DolaDocument;
pub use easing::{EasingFunction, EasingName, ParametricEasing};
pub use error::DolaError;
pub use storyboard::{
    BetweenKeyframes, InterruptionPolicy, KeyframeNames, KeyframeRef, LoopOffset, LoopOffsetRange,
    Storyboard, StoryboardEntry,
};
#[cfg(feature = "toml")]
pub use toml::{de::Error as TomlReadError, ser::Error as TomlWriteError};
pub use transition::{TransitionDef, TransitionRef, TransitionValue};
pub use value::DynamicValue;
pub use variable::AnimationVariableDef;
