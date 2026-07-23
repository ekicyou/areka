//! SERIKO アニメーションエンジン (⑤)。
//!
//! サーフェスの解決・状態・発行・構築・アクターの各責務をモジュールへ分割する。
//! 各モジュールは非公開とし、クレートの公開 API はこの crate root からの `pub use`
//! re-export に集約する（唯一の公開面・idiomatic な re-export パターン）。
//!
//! - 解決層 [`SurfaceResolver`]／[`SurfaceTarget`]: `Emote{key}` を解決結果へ写す純粋層。
//! - 状態層 [`ScopeStates`]／[`ScopeState`]／[`ApplyOutcome`]／[`BindApplyOutcome`]: per-scope
//!   surface 状態・冪等ガードと、動的 bind 適用結果に応じた表示発行の判定。
//! - 発行層 [`DisplayCommand`]／[`SurfaceOutput`]／[`MockSurfaceOutput`]: emo への表示指令と発行先抽象。
//! - アクター層 [`SerikoMsg`]／[`SerikoSink`]／[`spawn_seriko`]: 独立スレッド稼働・単一発行点。
//! - 構築層 [`build_static_bindset`]: bindgroup default → 静的 `BindSet`（恒等写像）。
//! - アニメ定義表 [`AnimationTable`]／[`LoopAnimation`]／[`LoopTrigger`]／[`LoopFrame`]: `EmoWorld`
//!   からの boot 時不変スナップショット（`Random`/`BindRandom` のみ採録・method 構築時 1 回解決）。
//! - bind 解決層 [`BindResolver`]／[`BindNamespace`]／[`scope_namespace`]: `(カテゴリ, パーツ)`
//!   → 着せ替え ID の名前解決と scope→名前空間写像を担う純関数群（parsers 非依存）。
//! - bind 類別層 [`BindDirective`]／[`parse_bind_directive`]: `\![bind,...]` トークン列を
//!   Apply／Toggle／CategoryWide／Malformed へ類別する純関数（severity 付与は actor 責務）。

mod actor;
mod bind;
mod output;
mod resolve;
mod state;
mod table;

pub use actor::{spawn_seriko, SerikoMsg, SerikoSink};
pub use bind::{
    accumulate, build_static_bindset, parse_bind_directive, scope_namespace, BindDirective,
    BindNamespace, BindResolver,
};
pub use output::{DisplayCommand, MockSurfaceOutput, SurfaceOutput};
pub use resolve::{SurfaceResolver, SurfaceTarget};
pub use state::{
    ApplyOutcome, BindApplyOutcome, PatternApplyOutcome, ScopeState, ScopeStates, Slot,
};
pub use table::{AnimationTable, LoopAnimation, LoopFrame, LoopTrigger};
