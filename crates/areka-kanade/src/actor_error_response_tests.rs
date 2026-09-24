//! エラー応答の写しの檻（shiori-fault-notice 要件 6.1・7.3）。
//!
//! 送出点 `round_trip_request` を直接呼び、SHIORI が返した失敗を呼出の種別ごとにどう写すかを
//! 固定する: エラー応答は GET なら NoContent・NOTIFY なら Notified（記録 1 件）、他の失敗は
//! そのまま Failed（記録なし）。GET 側を取り違えて Notified にすると、OnClose のエラー応答で
//! 閉じる相が進まなくなるので、GET と NOTIFY の両方をここで見る。
//!
//! 往復は呼出スレッドで同期に走るので、記録はスレッドローカルの捕捉窓で拾える。

use super::*;
use crate::schedule::log_capture::{CapturedEvent, capture};
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use std::sync::mpsc;
use std::thread;
use tracing::Level;

/// テスト用 GET 呼出（`OnBoot`・許可表にある固定 ID）。
fn probe_get_call() -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("OnBoot"),
        references: vec!["master".to_string()],
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

/// テスト用 NOTIFY 呼出（`OnInitialize`・許可表にある固定 ID）。
fn probe_notify_call() -> ShioriCall {
    ShioriCall::Notify {
        id: EventId::Static("OnInitialize"),
        references: Vec::new(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

/// 1 件の Request を受け、`failure` を返す偽の shiori を立てて `call` を往復させる。
/// 戻り値は写したあとの応答と、テストスレッドで捕えた記録。
fn round_trip_with_failure(
    call: ShioriCall,
    failure: ShioriFailure,
) -> (ShioriOutcome, Vec<CapturedEvent>) {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let helper = thread::spawn(move || {
        if let Ok(ShioriMsg::Request { call: _, reply }) = shiori_rx.recv() {
            let _ = reply.send(ShioriOutcome::Failed(failure));
        }
    });

    let mut outcome: Option<ShioriOutcome> = None;
    let events = capture(|| {
        outcome = Some(round_trip_request(&shiori_tx, call));
    });
    helper.join().expect("fake shiori thread");
    (outcome.expect("round_trip_request returns"), events)
}

/// 捕えた記録のうち `shiori_error_response`（kanade の WARN）の件数。
fn error_response_records(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.target == "kanade"
                && e.level == Level::WARN
                && e.event.as_deref() == Some("shiori_error_response")
        })
        .count()
}

#[test]
fn error_response_to_get_becomes_no_content_with_one_record() {
    let (outcome, events) = round_trip_with_failure(
        probe_get_call(),
        ShioriFailure::Shiori("500 Internal Server Error".into()),
    );
    assert!(
        matches!(outcome, ShioriOutcome::NoContent),
        "GET のエラー応答は返事なし（NoContent）に写るはず"
    );
    assert_eq!(error_response_records(&events), 1, "記録は 1 往復に 1 件");
}

#[test]
fn error_response_to_notify_becomes_notified_with_one_record() {
    let (outcome, events) = round_trip_with_failure(
        probe_notify_call(),
        ShioriFailure::Shiori("400 Bad Request".into()),
    );
    assert!(
        matches!(outcome, ShioriOutcome::Notified),
        "NOTIFY のエラー応答は通知済み（Notified）に写るはず"
    );
    assert_eq!(error_response_records(&events), 1, "記録は 1 往復に 1 件");
}

#[test]
fn other_failures_stay_failed_without_error_response_record() {
    for call in [probe_get_call(), probe_notify_call()] {
        let (outcome, events) =
            round_trip_with_failure(call, ShioriFailure::Ipc("helper gone".into()));
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Ipc(_))),
            "エラー応答以外の失敗は Failed のまま（Fault の判断は運行表に残す）"
        );
        assert_eq!(
            error_response_records(&events),
            0,
            "エラー応答以外では shiori_error_response を出さない"
        );
    }
}
