//! # Dola Runtime Engine
//!
//! 指示書の受信 → コンパイル → タイムテーブル管理 → 購読者への差分配信を行う
//! リアクティブ・アニメーション・ランタイム。

mod document_store;
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
pub use instance_state::InstanceState;
pub use interpolator::Interpolator;
pub use types::{EvaluatedValue, RuntimeError, StartResult};
