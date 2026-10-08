//! 公開 facade（parse・parse_noted）— lexer→decode を結線する公開純粋関数。
//!
//! さくらスクリプト文字列を、値正規化済み・順序保持のフラットな型付き命令列
//! `Vec<Instruction>` へ変換する。実体は構文層（`lexer::lex_spanned`）と意味層
//! （`decode::decode_noted`）の合成にすぎず、本関数自体は状態も I/O も持たない
//! （依存方向 `model ← lexer ← decode ← parse`）。
//!
//! 契約（design「parse — 公開純粋関数」）:
//! - 空入力 → 空 `Vec`（要件 1.5）。
//! - 入力順を保持した型付き命令列（要件 1.1/1.3）。
//! - 命令を実行・解釈しない（戻り値生成のみ・副作用なし・要件 1.4）。
//! - 同一入力 → 同一出力の純粋・決定的関数（host 非依存・要件 12.2）。
//! - 失敗しない（常に `Vec` を返す・`Result` でない・エラーを送出しない・要件 10.2）。
//!   不正トークン前後の正常命令は欠落しない（局所吸収・全域継続・要件 10.3）。

use super::decode::decode_noted;
use super::lexer::lex_spanned;
use super::model::{Instruction, Read};

/// さくらスクリプト文字列を値まで decode 済みの型付き命令列へ変換する純粋関数。
///
/// 寛容パススルー: 失敗せず常に `Vec<Instruction>` を返す（要件 10）。`parse_noted` の
/// 結果から命令だけを取り出したもの（経路は 1 本・検査用の写しを作らない）。
///
/// - Preconditions: `input` は UTF-8（要件 12.1）。
/// - Postconditions: 順序保持の命令列（要件 1.1/1.3）。空入力で空列（要件 1.5）。
/// - Invariants: 純粋・決定的・host 非依存（要件 12.2）。エラーを送出しない（要件 10.2）。
pub fn parse(input: &str) -> Vec<Instruction> {
    parse_noted(input)
        .into_iter()
        .map(|read| read.instruction)
        .collect()
}

/// `parse` と同じ命令の列を、命令ごとの台本の中のバイト範囲と、読む段が自分で下した
/// 扱いの印つきで返す純粋関数（mcp-author-tools 要件 3.1・3.11）。
///
/// `lex_spanned` で範囲つきの構文トークン列へ分割し、`decode_noted` で `Read` の列へ
/// 写像する。
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
