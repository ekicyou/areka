//! areka-actor: エンジン非依存なアクター規約と薄いヘルパ群（規約正本）。
//!
//! 本クレートは純粋層（`spawn`/`reply`＝std のみ）と UI ブリッジ層（`ui`）を
//! モジュール境界で分離する。詳細な envelope／停止規約は後続タスクで本モジュールの
//! crate rustdoc に明文化される。

pub mod reply;
pub mod spawn;
pub mod ui;
