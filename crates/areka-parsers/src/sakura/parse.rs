//! 公開 facade（parse・parse_noted）— lexer→decode を結線する公開純粋関数。
//!
//! さくらスクリプト文字列を、値正規化済み・順序保持のフラットな型付き命令列
//! `Vec<Instruction>` へ変換する。実体は構文層と意味層の合成にすぎず、状態も I/O も持たない
//! （依存方向 `model ← lexer ← decode ← parse`）。
//!
//! 空入力は空 `Vec`。失敗しない（常に `Vec` を返し、`Result` でない）。不正トークン前後の
//! 正常命令は欠落しない。

use super::decode::decode_noted;
use super::lexer::lex_spanned;
use super::model::{Instruction, Read};

/// さくらスクリプト文字列を値まで decode 済みの型付き命令列へ変換する純粋関数。
///
/// 失敗せず常に `Vec<Instruction>` を返す。`parse_noted` の結果から命令だけを取り出した
/// もの（経路は 1 本・検査用の写しを作らない）。
///
/// - Preconditions: `input` は UTF-8。
/// - Postconditions: 順序保持の命令列。空入力で空列。
/// - Invariants: 純粋・決定的・host 非依存。エラーを送出しない。
pub fn parse(input: &str) -> Vec<Instruction> {
    parse_noted(input)
        .into_iter()
        .map(|read| read.instruction)
        .collect()
}

/// `parse` と同じ命令の列を、命令ごとの台本の中のバイト範囲と、読む段が自分で下した
/// 扱いの印つきで返す純粋関数。
///
/// - Preconditions: `input` は UTF-8。
/// - Postconditions: 命令の列は `parse(input)` と等しい。`span` は入力の順に並び、
///   重ならず、文字の境界に在る（`&input[span]` が該当の綴り）。隣り合うトークンを
///   畳んだ命令の `span` は畳んだ全部を覆う。
/// - Invariants: 純粋・決定的。失敗しない。記録を出さない。
pub fn parse_noted(input: &str) -> Vec<Read> {
    decode_noted(lex_spanned(input))
}

#[cfg(test)]
#[path = "parse_bare_tag_tests.rs"]
mod bare_tag_tests;

#[cfg(test)]
#[path = "parse_word_boundary_tests.rs"]
mod word_boundary_tests;

#[cfg(test)]
#[path = "parse_noted_tests.rs"]
mod noted_tests;
