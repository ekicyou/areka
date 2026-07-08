//! # areka-emo-present — emo ⑥ render engine 提示段（三段直列チェーン 3/3）
//!
//! `areka-emo-atlas`（1/3・アトラス正本）→ `areka-emo-compose`（2/3・純粋合成）→
//! 本クレート `areka-emo-present`（3/3・提示段）と連なる emo track の最終段である。
//! 上流が生成した静的合成済みビットマップ `ComposedSurface` を、wintf の WUC
//! （Windows.UI.Composition）表示面へアップロード・提示し、当たり判定用 AlphaMask を
//! ヒットテストへ供給する。合成そのものは上流の責務であり、本段は「表示・キャッシュ・
//! 指令適用・マスク同期」を UI スレッド上で統括する。
//!
//! ## 指令 API 契約正本（command-API contract source of truth）
//!
//! 本クレートは **指令 API（`PresentCommand`）の契約正本**である。surface 切替・
//! 非表示（`\s[-1]` 相当）・キャッシュ無効化を運ぶ指令の形（scope・surface id・
//! BindSet・非表示・reply 口）は本 spec が定義し、下流の seriko-engine が消費する。
//! 指令はメッセージ enum の variant へそのまま転写できる `Send + 'static` 所有形
//! （`areka-actor` の envelope 規約準拠）を採る。`\s[-1]` → 非表示・alias → u32 解決は
//! 発行側（seriko）の責務であり、本段は解決済みの指令を受け取って適用する。
//!
//! ## 依存方向（強制）
//!
//! `areka-parsers → areka-emo-atlas → areka-emo-compose → areka-emo-present`
//! （＋ `wintf`・`areka-actor`）。逆方向（`wintf → emo-present` 等）の import は
//! 実装・レビューでエラーとして扱う。
//!
//! ## 構成
//!
//! 指令 API（[`command`]）・バルーン枠生成（[`balloon`]）は実装済み。合成キャッシュ（`cache`）・
//! 提示統括（`presenter`）・swap chain 供給面（`chain`）・窓装着（`mount`）の各モジュールは、
//! 後続タスクで順次このクレートへ実装される。

/// バルーン枠画像を **シェルと同一の** compose/present 経路へ載せる入力適合層（`BalloonFrameSource`）。
/// `build_balloon_target` を公開し、`attach_target` へ渡す `(EmoWorld, AtlasTable)` を組み上げる。
pub mod balloon;
pub mod cache;
/// swap chain 供給面（`SwapChainPresenter`）。`pub(crate)` 内部モジュール（公開 API ではない）。
/// 後続の `presenter`（`EmoPresenter`）が `crate::chain::SwapChainPresenter` を消費する。
pub(crate) mod chain;
pub mod command;
/// 窓装着・text 層スロット予約・非表示切替（`VisualMount`）。`pub(crate)` 内部モジュール
/// （公開 API ではない）。後続の `presenter`（`EmoPresenter`）が `crate::mount::VisualMount` を消費する。
pub(crate) mod mount;

pub use balloon::build_balloon_target;
pub use cache::{CacheEntry, ComposeCache};
pub use command::{PresentCommand, PresentError, PresentOutcome, TargetId};
