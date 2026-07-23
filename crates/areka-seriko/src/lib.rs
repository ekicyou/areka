//! SERIKO アニメーションエンジン (⑤)。
//!
//! サーフェスの解決・状態・発行・構築・アクターの各責務をモジュールへ分割する。
//! 各モジュールは非公開とし、クレートの公開 API はこの crate root からの `pub use`
//! re-export に集約する（唯一の公開面・idiomatic な re-export パターン）。
//!
//! - 解決層 [`SurfaceResolver`]／[`SurfaceTarget`]: `Emote{key}` を解決結果へ写す純粋層。
//! - 状態層 [`ScopeStates`]／[`ScopeState`]／[`ApplyOutcome`]: per-scope surface 状態と冪等ガード。
//! - 発行層 [`DisplayCommand`]／[`SurfaceOutput`]／[`MockSurfaceOutput`]: emo への表示指令と発行先抽象。
//! - アクター層 [`SerikoMsg`]／[`SerikoSink`]／[`spawn_seriko`]: 独立スレッド稼働・単一発行点。
//! - 構築層 [`build_static_bindset`]: bindgroup default → 静的 `BindSet`（恒等写像）。
//! - bind 解決層 [`BindResolver`]／[`BindNamespace`]／[`scope_namespace`]: `(カテゴリ, パーツ)`
//!   → 着せ替え ID の名前解決と scope→名前空間写像を担う純関数群（parsers 非依存）。

mod actor;
mod bind;
mod output;
mod resolve;
mod state;

pub use actor::{spawn_seriko, SerikoMsg, SerikoSink};
pub use bind::{build_static_bindset, scope_namespace, BindNamespace, BindResolver};
pub use output::{DisplayCommand, MockSurfaceOutput, SurfaceOutput};
pub use resolve::{SurfaceResolver, SurfaceTarget};
pub use state::{ApplyOutcome, ScopeState, ScopeStates};
