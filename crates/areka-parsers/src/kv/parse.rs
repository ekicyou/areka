//! KV マップ化（内部実装＋公開 facade `parse_kv`）。
//!
//! 入力を行分割し、各行を最初のカンマで `key`/`value` に分割、trim・後勝ち・
//! 空行/カンマ無し行スキップで素朴な `BTreeMap` を構築する。分類・型付け・
//! 専用スロットは設けず、値は文字列のまま保持する（design「Responsibilities &
//! Constraints」・R4.1–4.8/R5.1/R5.3/R6）。

use std::collections::BTreeMap;

/// デコード済み文字列を素朴な `key,value` フラットマップへ変換する。
/// 分類・型付けをせず、後勝ち・trim・空行/カンマ無し行スキップ・値は文字列保持・順序非保持。
/// Result を返さず panic しない（R6.1）。
///
/// - 行分割は `str::lines()`（LF/CRLF 両吸収）に委ね、末尾 CR 残りは `trim()` で吸収する（R5.1）。
/// - 各行は最初のカンマ 1 個で分割（`split_once(',')`）。値中の後続カンマは保持される（R4.1）。
/// - カンマ無し行・trim 後に空となる行はスキップ（R4.5/R4.6）。
/// - キー・値は前後空白を trim（R4.4）。同一キーは後勝ち（`BTreeMap::insert`・R4.3）。
/// - 値は無加工の文字列（数値化・符号解釈なし・R4.7）。順序は保持しない（`BTreeMap`・R4.8）。
/// - 空入力 → 空マップ。いかなる入力でも panic せず `Result` を返さない（R5.3/R6.1）。
pub fn parse_kv(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    for line in text.lines() {
        // カンマ無し行は分割不能ゆえスキップ（R4.6）。最初のカンマ 1 個のみで分割し、
        // 値中の後続カンマは保持する（R4.1）。
        let Some((raw_key, raw_value)) = line.split_once(',') else {
            continue;
        };

        // キー・値の前後空白を trim（R4.4）。末尾 CR 残りもここで吸収（R5.1）。
        let key = raw_key.trim();

        // trim 後にキーが空となる行（空行・カンマ始まり行）はスキップ（R4.5）。
        if key.is_empty() {
            continue;
        }

        // 後勝ちで上書き（R4.3）。値は無加工の文字列のまま格納（R4.7）。
        map.insert(key.to_owned(), raw_value.trim().to_owned());
    }

    map
}
