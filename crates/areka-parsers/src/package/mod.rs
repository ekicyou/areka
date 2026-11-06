//! package — 展開済みゴーストパッケージの 2 点マウント（SHIORI / shell）解決。
//!
//! 本 module は parser ファミリ内で唯一 I/O（`std::fs` 読取のみ）と `Result`
//! を持つ。理由＝マウントは物理不在という現実の失敗を持つため（Req 5.1・
//! `sakura` の寛容パースと意図的に非対称）。I/O は `resolve_shell`／`resolve_bind`
//! サブモジュール内に閉じ込め、`model` は純粋な型定義に留める。この逸脱は局所的・意図的であり、
//! 後続 `shell-parse` / `balloon-parse` 実装者が兄弟パーサ規約
//! （`pub fn parse(&str) -> Vec<Model>`・`Result` 無しの寛容パース）を
//! 誤読しないよう、ここに固定する。
//!
//! 公開面の集約。中身は次のとおり:
//! - `model`         : マウントモデル・失敗型の正本（`MountModel` / 付随値型 / `MountError`・
//!   bindgroup の表 `BindGroupDefaults`）
//! - `resolve_shell` : ゴーストの descript の読み + シェル名の確かめ + `shell/<名>` の
//!   存在確認（`resolve_with_shell`・名前だけの `read_ghost_names`・SHIORI の置き場の
//!   綴り `shiori_dir`）
//! - `resolve_bind`  : 選んだシェルの descript からの bindgroup の転記
//!
//! 依存方向は一方向（`model ← resolve_bind ← resolve_shell`）。本 `mod.rs` は解決の関数と
//! モデル型群を公開面へ `pub use` で集約する。

mod model;

#[cfg(test)]
mod resolve_tests;

mod resolve_bind;
#[cfg(test)]
mod resolve_bind_tests;

mod resolve_shell;
#[cfg(test)]
mod resolve_shell_tests;

#[cfg(test)]
mod validation_tests;

pub use model::{
    BindGroupDefaults, BindGroupName, GhostNames, MountError, MountModel, ScopeBinds, ShellMount,
    ShioriMount,
};
pub use resolve_shell::{read_ghost_names, resolve_with_shell, shiori_dir};
