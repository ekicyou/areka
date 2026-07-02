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

// 公開面集約は後続タスク 2.1（model 型）/ 5.1（parse facade）で
// `pub use model::{...};` および `pub use parse::parse;` を追加する。
// スケルトン段階では実型が存在しないため、ここでは宣言しない。
