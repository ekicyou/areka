//! # Dola Runtime Engine
//!
//! 指示書の受信から購読者への差分配信までを行うリアクティブ・アニメーション・ランタイム。

mod conflict_resolver;
mod facade;
mod instance_manager;
mod instance_state;
mod interpolator;
mod loop_controller;
mod subscription_manager;
mod timeline_manager;
mod types;

#[cfg(target_os = "windows")]
pub mod clock;

pub use facade::DolaRuntime;
pub use types::{EvaluatedValue, RuntimeError, StartResult, TriggerResult, UpdateResult};
