//! shell — surfaces.txt パーサ。
//!
//! 公開面集約のスケルトン。後続タスクで以下を順次追加する:
//! - `model`  : 下流共有 I/O 契約型（Shell ルート＋各型＋opaque NewType）
//! - `lexer`  : 構文層（ブロック/行/ドットキー/CSV/`[id,...]` 配列のトークン化）
//! - `decode` : 意味層（animationN 集約・append 範囲展開・alias 写像・subset 値正規化）
//! - `parse`  : 公開 facade（`pub fn parse(input: &str) -> Shell`）
//!
//! 依存方向は `model ← lexer ← decode ← parse`。本 `mod.rs` は
//! `parse` / `Shell` / 各値型を公開面へ `pub use` で集約する。
//!
//! surfaces.txt は sakura（インライン `\tag[args]` 走査）と構文クラスが異なる
//! （ブロック構造＋ドット付きキー＋行指向 CSV）。ゆえに lexer/decode の実体は
//! 新規だが、四層の骨格・規律・思想は sakura を完全踏襲する。

mod model;

#[cfg(test)]
mod model_tests;

mod lexer;

#[cfg(test)]
mod lexer_tests;

mod decode;

#[cfg(test)]
mod decode_tests;

mod parse;

#[cfg(test)]
mod parse_tests;

#[cfg(test)]
mod validation_tests;

// 箱の転記（areka-P0-shell-balloon）: 画像の読み手とは別に、同じ文面から箱に関わる行だけを
// 原文のまま並べる。`decode::parse_targets` を見出しの読み取りとして共用する。
mod boxes;

#[cfg(test)]
mod boxes_tests;

// 描けない行の転記（areka-P0-element-base-method）: 画像の読み手・箱の転記のどちらも拾わない
// element定義の行を原文のまま並べる。記録は入口（`load_shell_target`）が出す。
mod undrawn;

#[cfg(test)]
mod undrawn_tests;

// 表情の表の転記（areka-P0-mcp-expression-table）: surfacetable.txt を行の列へ写すだけ。
// lexer・decode・model は使わない。
mod surfacetable;

#[cfg(test)]
mod surfacetable_tests;

// 公開面一点集約（要件 11.1）: 下流は本モジュールからの import のみで
// モデル型と公開 facade を消費でき、内部の model/lexer/decode/parse 分割へ
// 直接依存しない。依存方向 `model ← lexer ← decode ← parse` は不変。
pub use boxes::{
    BoxBrace, BoxDefinition, BoxElementLine, BoxSurfaceLines, ShellBoxes, parse_boxes,
};
pub use model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element,
    ElementPath, Interval, Pattern, Shell, SortOrder, Surface, SurfaceAlias, SurfaceAppend,
};
pub use parse::parse;
pub use surfacetable::{SurfaceTable, SurfaceTableRow, parse_surfacetable};
pub use undrawn::{UndrawnElementLine, parse_undrawn_elements};
