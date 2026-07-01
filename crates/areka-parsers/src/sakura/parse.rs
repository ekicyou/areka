//! 公開 facade（parse）— lexer→decode を結線する単一公開純粋関数。
//!
//! さくらスクリプト文字列を、値正規化済み・順序保持のフラットな型付き命令列
//! `Vec<Instruction>` へ変換する。実体は構文層（`lexer::lex`）と意味層
//! （`decode::decode`）の合成にすぎず、本関数自体は状態も I/O も持たない
//! （依存方向 `model ← lexer ← decode ← parse`）。
//!
//! 契約（design「parse — 公開純粋関数」）:
//! - 空入力 → 空 `Vec`（要件 1.5）。
//! - 入力順を保持した型付き命令列（要件 1.1/1.3）。
//! - 命令を実行・解釈しない（戻り値生成のみ・副作用なし・要件 1.4）。
//! - 同一入力 → 同一出力の純粋・決定的関数（host 非依存・要件 12.2）。
//! - 失敗しない（常に `Vec` を返す・`Result` でない・エラーを送出しない・要件 10.2）。
//!   不正トークン前後の正常命令は欠落しない（局所吸収・全域継続・要件 10.3）。

use super::decode::decode;
use super::lexer::lex;
use super::model::Instruction;

/// さくらスクリプト文字列を値まで decode 済みの型付き命令列へ変換する純粋関数。
///
/// 寛容パススルー: 失敗せず常に `Vec<Instruction>` を返す（要件 10）。`lex` で構文
/// トークン列へ分割し、`decode` で値正規化済み `Instruction` 列へ写像する。
///
/// - Preconditions: `input` は UTF-8（要件 12.1）。
/// - Postconditions: 順序保持の命令列（要件 1.1/1.3）。空入力で空列（要件 1.5）。
/// - Invariants: 純粋・決定的・host 非依存（要件 12.2）。エラーを送出しない（要件 10.2）。
pub fn parse(input: &str) -> Vec<Instruction> {
    decode(lex(input))
}
