//! 公開 facade（parse）— lexer→decode を結線する単一公開純粋関数。
//!
//! デコード済みの surfaces.txt 文字列を、値正規化済み・順序保持のシェルサーフェス
//! モデル `Shell` へ変換する。実体は構文層（`lexer::lex`）と意味層
//! （`decode::decode`）の合成にすぎず、本関数自体は状態も I/O も持たない
//! （依存方向 `model ← lexer ← decode ← parse`）。
//!
//! 契約（design「Facade Layer — parse」）:
//! - 空入力 → 空 `Shell`（surfaces/appends/aliases すべて空・要件 2.2）。
//! - `Result` を返さない（常に `Shell` を返す・subset 外/不正入力は部分認識で吸収・要件 9.3）。
//! - 同一入力 → 同一出力の純粋・決定的関数（副作用なし・host 非依存・要件 2.1/2.4）。
//! - パニックしない（局所吸収・全域継続・要件 9.3）。

use super::decode::decode;
use super::lexer::lex;
use super::model::Shell;

/// デコード済み surfaces.txt 文字列を値まで正規化した `Shell` モデルへ変換する純粋関数。
///
/// 寛容パススルー: `Result` を返さず常に `Shell` を返す（要件 9.3）。`lex` で構文
/// トークン列へ分割し、`decode` で値正規化済みモデルへ写像する。
///
/// - Preconditions: `input` はデコード済み（charset 適用後の）文字列（要件 2.1）。
/// - Postconditions: 空入力で空 `Shell`（要件 2.2）。順序保持のモデル（要件 2.1）。
/// - Invariants: 純粋・決定的・副作用なし・非パニック（要件 2.4/9.3）。
pub fn parse(input: &str) -> Shell {
    decode(lex(input))
}
