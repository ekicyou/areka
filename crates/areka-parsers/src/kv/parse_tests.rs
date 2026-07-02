//! parse_kv 単体テスト（後勝ち/空行/カンマ無し/trim/CRLF・LF/値中カンマ/空入力）。
//!
//! 公開面（`super::super::parse_kv`）経由で契約を固定する（design「Service Interface」・R7.4）。
//! 各テストは requirements §4.1–4.8／§5.1／§5.3 の Observable を 1 対 1 で検証する。

use super::parse_kv;

/// 後勝ち: 同一キーが複数行に現れると最後の値で上書きされる（R4.3）。
#[test]
fn last_write_wins_for_duplicate_key() {
    let map = parse_kv("k,v1\nk,v2\n");
    assert_eq!(map.get("k").map(String::as_str), Some("v2"));
    assert_eq!(map.len(), 1);
}

/// 空行スキップ: 空行は無視され、正常行のみ格納される（R4.5）。
#[test]
fn blank_lines_are_skipped() {
    let map = parse_kv("\na,1\n\n\nb,2\n\n");
    assert_eq!(map.get("a").map(String::as_str), Some("1"));
    assert_eq!(map.get("b").map(String::as_str), Some("2"));
    assert_eq!(map.len(), 2);
}

/// カンマ無し行スキップ: カンマを含まない行は分割不能ゆえスキップされる（R4.6）。
#[test]
fn lines_without_comma_are_skipped() {
    let map = parse_kv("novalue\nk,v\n");
    assert_eq!(map.get("k").map(String::as_str), Some("v"));
    assert_eq!(map.len(), 1);
}

/// key/value trim: キー・値の前後空白は寛容に除去される（R4.4）。
#[test]
fn key_and_value_are_trimmed() {
    let map = parse_kv("  k  ,  v  \n");
    assert_eq!(map.get("k").map(String::as_str), Some("v"));
    assert_eq!(map.len(), 1);
}

/// CRLF と LF で同結果: 改行様式に依存せず同一マップを返す（R5.1）。
#[test]
fn crlf_and_lf_produce_equal_maps() {
    let lf = parse_kv("a,1\nb,2\nc,3\n");
    let crlf = parse_kv("a,1\r\nb,2\r\nc,3\r\n");
    assert_eq!(lf, crlf);
    assert_eq!(crlf.get("b").map(String::as_str), Some("2"));
}

/// 値中カンマ保持: 最初のカンマのみで分割するため、値中のカンマは保持される（R4.1）。
#[test]
fn value_retains_further_commas() {
    let map = parse_kv("k,a,b,c\n");
    assert_eq!(map.get("k").map(String::as_str), Some("a,b,c"));
    assert_eq!(map.len(), 1);
}

/// 空入力 → 空マップ: 空文字列は空マップを返し panic しない（R5.3）。
#[test]
fn empty_input_yields_empty_map() {
    let map = parse_kv("");
    assert!(map.is_empty());
}

/// 値は文字列のまま保持: 数値化・符号解釈をせず生文字列を保つ（R4.7）。
#[test]
fn value_is_kept_as_raw_string() {
    let map = parse_kv("n,+00123\n");
    assert_eq!(map.get("n").map(String::as_str), Some("+00123"));
}
