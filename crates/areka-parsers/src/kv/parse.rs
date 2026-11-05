//! KV マップ化（内部実装＋公開 facade `parse_kv`・`parse_kv_lowercased`）。
//!
//! 入力を行分割し、各行を最初のカンマで `key`/`value` に分割、trim・後勝ち・
//! 空行/カンマ無し行スキップで素朴な `BTreeMap` を構築する。分類・型付け・
//! 専用スロットは設けず、値は文字列のまま保持する（design「Responsibilities &
//! Constraints」・R4.1–4.8/R5.1/R5.3/R6）。分割の本体は行の番号つきの
//! `parse_kv_with_line` 1 つで、ほかの 2 つはその射影。

use std::collections::BTreeMap;

/// 行の番号つきの key-value の読み。値に 1 始まりの行の番号を添える。
///
/// - 行分割は `str::lines()`（LF/CRLF 両吸収）に委ね、末尾 CR 残りは `trim()` で吸収する（R5.1）。
/// - 各行は最初のカンマ 1 個で分割（`split_once(',')`）。値中の後続カンマは保持される（R4.1）。
/// - カンマ無し行・trim 後にキーが空となる行はスキップ（R4.5/R4.6）。行の番号は飛ばした行も数える。
/// - キー・値は前後空白を trim（R4.4）。同一キーは後勝ちで、行の番号も後の行になる（R4.3）。
pub(crate) fn parse_kv_with_line(text: &str) -> BTreeMap<String, (String, usize)> {
    let mut map = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let Some((raw_key, raw_value)) = line.split_once(',') else {
            continue;
        };
        let key = raw_key.trim();
        if key.is_empty() {
            continue;
        }
        map.insert(key.to_owned(), (raw_value.trim().to_owned(), index + 1));
    }
    map
}

/// デコード済み文字列を素朴な `key,value` フラットマップへ変換する（`parse_kv_with_line` の射影）。
/// 分類・型付けをせず、後勝ち・trim・空行/カンマ無し行スキップ・値は文字列保持・順序非保持。
/// 値は無加工の文字列（数値化・符号解釈なし・R4.7）。順序は保持しない（`BTreeMap`・R4.8）。
/// 空入力 → 空マップ。Result を返さず panic しない（R5.3/R6.1）。
pub fn parse_kv(text: &str) -> BTreeMap<String, String> {
    parse_kv_with_line(text)
        .into_iter()
        .map(|(key, (value, _))| (key, value))
        .collect()
}

/// 鍵を ASCII で小文字にそろえ、空の値の鍵を落とした表（ゴーストの descript の読み）。
///
/// 大文字と小文字の綴りが混ざって同じ鍵が 2 回出たら、後の行が勝つ（鍵の表の順でなく行の順で畳む）。
/// 空の値は畳んだ後に落とす（後の行の空の値は、綴りにかかわらず前の値を消す）。
pub fn parse_kv_lowercased(text: &str) -> BTreeMap<String, String> {
    let mut rows: Vec<(usize, String, String)> = parse_kv_with_line(text)
        .into_iter()
        .map(|(key, (value, line))| (line, key.to_ascii_lowercase(), value))
        .collect();
    rows.sort_unstable_by_key(|(line, _, _)| *line);
    let mut map: BTreeMap<String, String> = rows
        .into_iter()
        .map(|(_, key, value)| (key, value))
        .collect();
    map.retain(|_, value| !value.is_empty());
    map
}
