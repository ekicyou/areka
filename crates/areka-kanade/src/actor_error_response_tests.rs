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

/// 偽の shiori が受けた名前と Reference（送られなければ None）。
type Sent = Option<(String, Vec<String>)>;

/// 1 件の Request を受け、送られた名前と Reference を控えて `NoContent` を返す偽の shiori を立てて
/// `call` を往復させる。戻り値は応答・偽の shiori が受けた中身（送られなければ None）・捕えた記録。
fn round_trip_recording(call: ShioriCall) -> (ShioriOutcome, Sent, Vec<CapturedEvent>) {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let helper = thread::spawn(move || {
        if let Ok(ShioriMsg::Request { call, reply }) = shiori_rx.recv() {
            let (ShioriCall::Get { id, references, .. }
            | ShioriCall::Notify { id, references, .. }) = call;
            let _ = reply.send(ShioriOutcome::NoContent);
            return Some((id.as_str().to_string(), references));
        }
        None
    });

    let mut outcome: Option<ShioriOutcome> = None;
    let events = capture(|| {
        outcome = Some(round_trip_request(&shiori_tx, call));
    });
    drop(shiori_tx);
    let sent = helper.join().expect("fake shiori thread");
    (outcome.expect("round_trip_request returns"), sent, events)
}

/// 外から頼まれた名前（`EventId::Sstp`）の GET。
fn probe_sstp_call(event: &str, references: &[&str]) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Sstp(event.to_string()),
        references: references.iter().map(|r| r.to_string()).collect(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

/// 送出の檻の `EventId::Sstp` の受理（mcp-kanade-tools 要件 2.1・2.7・6.1）: 検査を満たす名前は
/// 許可の表にも `On` 接頭にも依らず逐語で送られ、警告もエラーも出ない。スケジューラ起源の
/// 恒久禁止（`OnTalk`）も効かない。
#[test]
fn sstp_origin_valid_name_is_sent_verbatim_without_warning() {
    for (event, references) in [
        ("OnTalk", vec![]),
        ("UserDefined", vec!["a\u{1}b", "\tx"]),
        ("OnMenuExec", vec!["", "r1"]),
    ] {
        let (outcome, sent, events) = round_trip_recording(probe_sstp_call(event, &references));
        assert!(
            matches!(outcome, ShioriOutcome::NoContent),
            "{event:?} は送られて偽の shiori の応答が返るはず"
        );
        assert_eq!(
            sent,
            Some((
                event.to_string(),
                references.iter().map(|r| r.to_string()).collect()
            )),
            "名前と Reference は逐語で送られるはず"
        );
        assert!(
            !events
                .iter()
                .any(|e| e.level == Level::WARN || e.level == Level::ERROR),
            "{event:?} の送出は警告もエラーも出さない（要件 2.1）"
        );
    }
}

/// 検査を破る `EventId::Sstp`（名前か Reference に CR・LF・NUL・空の名前）は送らず、今日の檻と同じ
/// `error!`（`event_id_not_allowed`）＋`Failed(Internal)` になる。
#[test]
fn sstp_origin_invalid_name_or_reference_is_not_sent_and_logs_error() {
    for (event, references) in [
        ("On\rX", vec![]),
        ("On\nX", vec![]),
        ("On\0X", vec![]),
        ("", vec![]),
        ("  ", vec![]),
        ("OnX", vec!["a\rb"]),
        ("OnX", vec!["a\nb"]),
        ("OnX", vec!["a\0b"]),
    ] {
        let (outcome, sent, events) = round_trip_recording(probe_sstp_call(event, &references));
        assert!(
            matches!(outcome, ShioriOutcome::Failed(ShioriFailure::Internal(_))),
            "{event:?}／{references:?} は Failed(Internal) に写るはず"
        );
        assert_eq!(sent, None, "{event:?}／{references:?} は送られてはならない");
        assert!(
            events.iter().any(|e| e.target == "kanade"
                && e.level == Level::ERROR
                && e.event.as_deref() == Some("event_id_not_allowed")),
            "{event:?}／{references:?} は event_id_not_allowed の error! を残すはず"
        );
    }
}
