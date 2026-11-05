//! parse_kv 単体テスト（後勝ち/空行/カンマ無し/trim/CRLF・LF/値中カンマ/空入力）と、
//! 行の番号つきの読み・小文字の表のテスト。
//!
//! 公開面（`super::super::parse_kv`）経由で契約を固定する（design「Service Interface」・R7.4）。
//! 各テストは requirements §4.1–4.8／§5.1／§5.3 の Observable を 1 対 1 で検証する。

use super::{parse_kv, parse_kv_lowercased, parse_kv_with_line};

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

/// キーが空の行（カンマ始まり・空白だけのキー）は飛ばす（R4.5）。
#[test]
fn empty_key_lines_are_skipped() {
    let map = parse_kv(",v\n   ,w\nk,1\n");
    assert_eq!(map.get("k").map(String::as_str), Some("1"));
    assert_eq!(map.len(), 1);
}

/// 鍵を小文字にそろえ、空の値の鍵を落とす。綴りの違う同じ鍵は後の行が勝つ。
#[test]
fn lowercased_folds_keys_and_drops_empty() {
    let map = parse_kv_lowercased("Name,a\nTYPE,\n");
    assert_eq!(map.get("name").map(String::as_str), Some("a"));
    assert_eq!(map.len(), 1);

    // 鍵の表の順（"Name" < "name"）でなく行の順で畳む。
    let later = parse_kv_lowercased("name,a\nName,b\n");
    assert_eq!(later.get("name").map(String::as_str), Some("b"));
    let later = parse_kv_lowercased("Name,a\nname,b\n");
    assert_eq!(later.get("name").map(String::as_str), Some("b"));
}

/// 値とキーの前後の空白を取り、1 始まりの行の番号を添える。
#[test]
fn line_read_trims_key_and_value() {
    let map = parse_kv_with_line("charset,UTF-8\n  a  ,  x y  \n");
    assert_eq!(map.get("a"), Some(&("x y".to_string(), 2)));
}

/// 同じキーが 2 行あれば後の行の値と行の番号になる。カンマの無い行・キーが空の行も数える。
#[test]
fn line_read_same_key_takes_later_line() {
    let map = parse_kv_with_line("a,1\nno comma\n,empty key\na,3\n");
    assert_eq!(map.get("a"), Some(&("3".to_string(), 4)));
    assert_eq!(map.len(), 1);
}

/// CRLF でも行の番号がずれず、値に CR が残らない。
#[test]
fn line_read_crlf_keeps_line_numbers() {
    let map = parse_kv_with_line("a,1\r\n\r\nb,2\r\n");
    assert_eq!(map.get("a"), Some(&("1".to_string(), 1)));
    assert_eq!(map.get("b"), Some(&("2".to_string(), 3)));
}
