//! 全体デコード（内部実装＋公開 facade `decode`）。
//!
//! `prescan` の検出結果（または呼び出し側指定の `DefaultEncoding`）に従い、
//! `encoding_rs` でバイト列全体を単一 `String` へデコードする（実装は後続
//! タスク 2.3）。本ファイルはスケルトンであり、公開 fn `decode` の
//! シグネチャ（D6）のみを確定させる。

use super::model::DefaultEncoding;

/// バイト列を charset 宣言（または `default`）に従いデコードして 1 文字列を返す。
/// I/O・環境状態へアクセスしない純粋関数（R3）。Result を返さず panic しない（R2.7/R6.1）。
///
/// スタブ段階では空文字列を返す（後続タスク 2.3 で本実装に差し替え）。
pub fn decode(_bytes: &[u8], _default: DefaultEncoding) -> String {
    String::new()
}
