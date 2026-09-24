//! `finish_after_run`（task 4.1・要件 3.1・3.2・3.4・6.3・7.6）の単体テスト。
//!
//! `run()` の成否・Fault の有無・後始末の成否の組み合わせで、後始末がちょうど 1 回走ること
//! と、どれか 1 つでも失敗があれば `Err`（終了コード 1）になることを固定する。

use super::finish_after_run;
use std::cell::Cell;
use windows::Win32::Foundation::E_FAIL;
use windows::core::{Error, Result};

fn fail() -> Result<()> {
    Err(Error::from_hresult(E_FAIL))
}

/// `run`・`fault`・後始末の結果を与えて、戻り値と後始末の実行回数を返す。
fn drive(run: Result<()>, fault: bool, cleanup_result: Result<()>) -> (Result<()>, u32) {
    let calls = Cell::new(0);
    let out = finish_after_run(run, fault, || {
        calls.set(calls.get() + 1);
        cleanup_result
    });
    (out, calls.get())
}

/// すべて無事なら成功（終了コード 0）で、後始末は 1 回。
#[test]
fn all_clear_is_ok_and_cleans_up_once() {
    let (out, calls) = drive(Ok(()), false, Ok(()));
    assert!(out.is_ok());
    assert_eq!(calls, 1);
}

/// `run()` が失敗で戻っても後始末は 1 回通り、結果は失敗（要件 3.2・6.3）。
#[test]
fn run_failure_still_cleans_up_once_and_fails() {
    let (out, calls) = drive(fail(), false, Ok(()));
    assert!(out.is_err());
    assert_eq!(calls, 1);
}

/// Fault で終わったら後始末を通したうえで失敗（要件 3.1）。
#[test]
fn fault_cleans_up_once_and_fails() {
    let (out, calls) = drive(Ok(()), true, Ok(()));
    assert!(out.is_err());
    assert_eq!(calls, 1);
}

/// 後始末の失敗は失敗（要件 3.2）。
#[test]
fn cleanup_failure_fails() {
    let (out, calls) = drive(Ok(()), false, fail());
    assert!(out.is_err());
    assert_eq!(calls, 1);
}

/// 失敗が重なっても後始末は 1 回だけで、結果は失敗。
#[test]
fn every_failure_together_cleans_up_once_and_fails() {
    let (out, calls) = drive(fail(), true, fail());
    assert!(out.is_err());
    assert_eq!(calls, 1);
}
