//! `dwrite` の非公開の判定 `sizing_outcome`（要件 4.1・4.2・5.2）。大きさだけの
//! `GetClusterMetrics` の結果を「塊が 0」「入れ物が足りない」「ほかの失敗」に読み分け、
//! ほかの失敗が空の成功に化けず `error!` を 1 行残して `Err` になることを確かめる。

use super::*;
use log_capture_kit::count_levels;
use windows::Win32::Foundation::E_FAIL;

#[test]
fn sizing_outcome_ok_means_no_clusters() {
    let (r, counts) = count_levels(|| sizing_outcome(Ok(())));
    assert!(!r.unwrap());
    assert_eq!(counts.error, 0);
}

#[test]
fn sizing_outcome_insufficient_buffer_means_continue() {
    let (r, counts) =
        count_levels(|| sizing_outcome(Err(ERROR_INSUFFICIENT_BUFFER.to_hresult().into())));
    assert!(r.unwrap());
    assert_eq!(counts.error, 0);
}

#[test]
fn sizing_outcome_other_failure_is_err_with_one_error_log() {
    let (r, counts) = count_levels(|| sizing_outcome(Err(E_FAIL.into())));
    assert_eq!(r.unwrap_err().code(), E_FAIL);
    assert_eq!(counts.error, 1);
}
