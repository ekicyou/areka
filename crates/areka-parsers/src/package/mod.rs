//! package — 展開済みゴーストパッケージの 2 点マウント（SHIORI / shell）解決。
//!
//! 本 module は parser ファミリ内で唯一 I/O（`std::fs` 読取のみ）と `Result`
//! を持つ。理由＝マウントは物理不在という現実の失敗を持つため（Req 5.1・
//! `sakura` の寛容パースと意図的に非対称）。I/O は `resolve` サブモジュール内
//! に閉じ込め、`model` は純粋な型定義に留める。この逸脱は局所的・意図的であり、
//! 後続 `shell-parse` / `balloon-parse` 実装者が兄弟パーサ規約
//! （`pub fn parse(&str) -> Vec<Model>`・`Result` 無しの寛容パース）を
//! 誤読しないよう、ここに固定する。
//!
//! 公開面集約のスケルトン。後続タスクで以下を順次埋める:
//! - `model`   : マウントモデル・失敗型の正本（`MountModel` / 付随値型 / `MountError`）
//! - `resolve` : ツリー解決 + `std::fs` 存在確認 + descript 読込合成 + 既定
//!   フォールバック（公開 facade `pub fn resolve`）
//!
//! 依存方向は一方向（`model ← resolve`）。本 `mod.rs` は `resolve` と
//! モデル型群を公開面へ `pub use` で集約する。

mod model;
#[cfg(test)]
mod model_tests;

mod resolve;
#[cfg(test)]
mod resolve_tests;

#[cfg(test)]
mod validation_tests;

pub use model::{
    BindGroupDefaults, GhostNames, MountError, MountModel, ShellMount, ShioriMount,
};
pub use resolve::resolve;
