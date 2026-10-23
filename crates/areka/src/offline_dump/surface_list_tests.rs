//! `surface_list.rs` の試験（頭の有無・範囲・除外・読めない要素・無い面・重ね・空白）。

use std::collections::BTreeSet;

use log_capture_kit::{CapturedEvent, capture};

use super::read_surface_list;

const UNREADABLE: &str = "offline_dump_surface_list_unreadable";
const MISSING: &str = "offline_dump_surface_missing";

/// 0〜10 と 20・30 を持つシェル（11〜19 と 21〜29 は欠番）。
fn shell() -> BTreeSet<u32> {
    (0..=10).chain([20, 30]).collect()
}

/// 読みと展開を通して、番号の列と記録を返す。
fn run(raw: &str) -> (Vec<u32>, Vec<CapturedEvent>) {
    capture(|| read_surface_list(raw).resolve(&shell()))
}

fn warns(events: &[CapturedEvent], event: &str) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.field_str("event") == Some(event))
        .map(|e| e.field_str("element").unwrap_or_default().to_owned())
        .collect()
}

fn missing(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && e.field_str("event") == Some(MISSING))
        .map(|e| e.field("surface").unwrap_or_default().to_owned())
        .collect()
}

/// 頭の `surface` は付けても省いても、大文字小文字を問わず同じ（3.1）。
#[test]
fn surface_prefix_is_optional_and_case_insensitive() {
    let (ids, events) = run("0,surface1,SURFACE2,Surface3-4,!4");
    assert_eq!(ids, vec![0, 1, 2, 3]);
    assert!(events.is_empty(), "記録なし: {events:?}");
}

/// 頭の `surface` は 1 つだけ外す（2 つ重ねたものは読めない）。
#[test]
fn only_one_surface_prefix_is_removed() {
    let (ids, events) = run("surfacesurface1,2");
    assert_eq!(ids, vec![2]);
    assert_eq!(warns(&events, UNREADABLE), vec!["surfacesurface1"]);
}

/// 範囲は両端を含み、在る面だけになる。範囲の中の欠番は記録しない（3.2・3.4）。
#[test]
fn range_keeps_existing_ids_and_silently_skips_gaps() {
    let (ids, events) = run("9-25");
    assert_eq!(ids, vec![9, 10, 20]);
    assert!(events.is_empty(), "範囲の欠番は記録しない: {events:?}");
}

/// 両端が同じ範囲は 1 つの面。
#[test]
fn range_with_equal_ends_is_one_surface() {
    let (ids, events) = run("5-5");
    assert_eq!(ids, vec![5]);
    assert!(events.is_empty(), "{events:?}");
}

/// 除外と除外の範囲は、書かれた位置に関わらず包含から引く（3.2）。
#[test]
fn exclusions_subtract_from_inclusions_regardless_of_position() {
    let (ids, events) = run("!3,0-10,!5-7,20");
    assert_eq!(ids, vec![0, 1, 2, 4, 8, 9, 10, 20]);
    assert!(events.is_empty(), "{events:?}");
}

/// 除外だけでは何も書き出さない。
#[test]
fn exclusions_alone_select_nothing() {
    let (ids, events) = run("!1,!2-3,!0-4294967295");
    assert!(ids.is_empty());
    assert!(events.is_empty(), "{events:?}");
}

/// 読めない要素（空・数でない・`a > b`）はその要素だけ Warning で飛ばし、ほかは読む（3.5）。
#[test]
fn unreadable_elements_are_skipped_with_a_warning_each() {
    let (ids, events) = run("1,,abc,5-3,!x,-2,3-,surface,2");
    assert_eq!(ids, vec![1, 2]);
    assert_eq!(
        warns(&events, UNREADABLE),
        vec!["", "abc", "5-3", "!x", "-2", "3-", "surface"]
    );
    assert_eq!(events.len(), 7, "読めない要素ごとに 1 件だけ: {events:?}");
}

/// 単独の番号で指した無い面は Warning（3.3）。除外で指した無い面は記録しない。
#[test]
fn missing_single_surface_warns_once() {
    let (ids, events) = run("1,15,15,surface15,!16");
    assert_eq!(ids, vec![1]);
    assert_eq!(missing(&events), vec!["15"], "重ねても 1 件: {events:?}");
    assert_eq!(events.len(), 1, "{events:?}");
}

/// 同じ面を何度指しても 1 度だけ・番号の昇順（3.7）。
#[test]
fn duplicates_are_written_once_in_ascending_order() {
    let (ids, events) = run("20,3,1-4,surface3,2");
    assert_eq!(ids, vec![1, 2, 3, 4, 20]);
    assert!(events.is_empty(), "{events:?}");
}

/// 要素の前後の空白は除く。
#[test]
fn surrounding_whitespace_is_trimmed() {
    let (ids, events) = run(" 1 ,\tsurface2 , 3-4\t, !4 ");
    assert_eq!(ids, vec![1, 2, 3]);
    assert!(events.is_empty(), "{events:?}");
}
