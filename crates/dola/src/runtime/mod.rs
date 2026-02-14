//! # Dola Runtime Engine
//!
//! 指示書の受信 → コンパイル → タイムテーブル管理 → 購読者への差分配信を行う
//! リアクティブ・アニメーション・ランタイム。

mod instance_state;
mod interpolator;
mod types;

pub use instance_state::InstanceState;
pub use interpolator::Interpolator;
pub use types::{EvaluatedValue, RuntimeError, StartResult};
