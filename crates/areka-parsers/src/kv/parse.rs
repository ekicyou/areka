//! KV マップ化（内部実装＋公開 facade `parse_kv`）。
//!
//! 入力を行分割し、各行を最初のカンマで `key`/`value` に分割、trim・後勝ち・
//! 空行/カンマ無し行スキップで素朴な `BTreeMap` を構築する（実装は後続タスク
//! 3.1）。本ファイルはスケルトンであり、公開 fn `parse_kv` のシグネチャのみ
//! を確定させる。

use std::collections::BTreeMap;

/// デコード済み文字列を素朴な `key,value` フラットマップへ変換する。
/// 分類・型付けをせず、後勝ち・trim・空行/カンマ無し行スキップ・値は文字列保持・順序非保持。
/// Result を返さず panic しない（R6.1）。
///
/// スタブ段階では空マップを返す（後続タスク 3.1 で本実装に差し替え）。
pub fn parse_kv(_text: &str) -> BTreeMap<String, String> {
    BTreeMap::new()
}
