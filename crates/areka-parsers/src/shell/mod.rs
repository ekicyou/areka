//! shell — surfaces.txt パーサ。
//!
//! 公開面の集約。中身は次のとおり:
//! - `model`  : 下流共有 I/O 契約型（Shell ルート＋各型＋opaque NewType）
//! - `lexer`  : 構文層（ブロック/行/ドットキー/CSV/`[id,...]` 配列のトークン化）
//! - `decode` : 意味層（animationN 集約・append 範囲展開・alias 写像・subset 値正規化）
//! - `parse`  : 公開 facade（`pub fn parse(input: &str) -> Shell`）
//!
//! 依存方向は `model ← lexer ← decode ← parse`。本 `mod.rs` は
//! `parse` / `Shell` / 各値型を公開面へ `pub use` で集約する。

mod model;

mod lexer;

#[cfg(test)]
mod lexer_tests;

mod decode;

mod element_method_words;

#[cfg(test)]
mod element_method_words_tests;

#[cfg(test)]
mod decode_tests;

mod parse;

#[cfg(test)]
mod parse_tests;

#[cfg(test)]
mod validation_tests;

// 箱の転記: 画像の読み手とは別に、同じ文面から箱に関わる行だけを
// 原文のまま並べる。`decode::parse_targets` を見出しの読み取りとして共用する。
mod boxes;

#[cfg(test)]
mod boxes_tests;

// 描けない行の転記: 画像の読み手・箱の転記のどちらも拾わない
// element定義の行を原文のまま並べる。記録は入口（`load_shell_target`）が出す。
mod undrawn;

#[cfg(test)]
mod undrawn_tests;

// cursorブレスの転記: 画像の読み手とは別に、同じ文面から cursorブレスだけを
// 原文のまま並べる。検証と記録は下流（`areka-runtime` の `shell_cursor_table`）。
mod cursor;

#[cfg(test)]
mod cursor_tests;

// 表情の表の転記: surfacetable.txt を行の列へ写すだけ。
// lexer・decode・model は使わない。
mod surfacetable;

#[cfg(test)]
mod surfacetable_tests;

// 描画メソッドの 8 語の表と綴りの比べ方: emo-compose と emo-atlas が共有する唯一の正本。
mod pattern_word;

#[cfg(test)]
mod pattern_word_tests;

// 下流は本モジュールからの import だけでモデル型と公開 facade を使え、
// 内部の model/lexer/decode/parse 分割に直接依存しない。
pub use boxes::{BoxBrace, BoxDefinition, BoxElementLine, BoxSurfaceLines, parse_boxes};
pub use cursor::{CursorBrace, parse_cursor_braces};
pub use element_method_words::BLEND_ELEMENT_WORDS;
pub use model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionBounds, CollisionExFault, CollisionName,
    CollisionRegion, CollisionShape, DefRef, DrawMethod, Element, ElementPath, Interval, Pattern,
    Shell, ShellIssue, SortOrder, Surface, SurfaceAlias, SurfaceAppend,
};
pub use parse::{ShellText, parse, parse_all};
pub use pattern_word::{PatternWord, canonical_method_name, pattern_word, same_canonical};
pub use surfacetable::{SurfaceTable, SurfaceTableRow, parse_surfacetable};
pub use undrawn::{UndrawnElementLine, parse_undrawn_elements};
