//! 失敗の種類への写し（[`ShioriFault::from_failure`]／[`ShioriFault::from_down`]／
//! [`ShioriFault::unknown`]）のテスト（areka-P0-shiori-fault-notice 要件 1.4・2.2・7.3）。
//!
//! 入口ごとに種類と理由が決まることを固定する。理由は呼出失敗なら `Display` の文言、
//! 死活報告なら受け取った理由そのもの。エラー応答（`ShioriFailure::Shiori`）の腕は
//! 送出点で「返事なし」へ写され、ここへは届かない契約なので判断のテストにしない。

use super::{ShioriDownKind, ShioriFailure, ShioriFault, ShioriFaultKind};

fn fault(kind: ShioriFaultKind, reason: &str) -> ShioriFault {
    ShioriFault {
        kind,
        reason: reason.to_string(),
    }
}

#[test]
fn from_failure_maps_each_reachable_arm_to_its_kind_and_display_reason() {
    let cases = [
        (
            ShioriFailure::Handshake("boom".to_string()),
            fault(
                ShioriFaultKind::ConnectFailed,
                "shiori handshake failure: boom",
            ),
        ),
        (
            ShioriFailure::Timeout("30s".to_string()),
            fault(ShioriFaultKind::Timeout, "shiori request timeout: 30s"),
        ),
        (
            ShioriFailure::Ipc("pipe closed".to_string()),
            fault(
                ShioriFaultKind::Disconnected,
                "shiori ipc failure: pipe closed",
            ),
        ),
        (
            ShioriFailure::Internal("event_id_not_allowed: OnTalk".to_string()),
            fault(
                ShioriFaultKind::Internal,
                "kanade internal violation: event_id_not_allowed: OnTalk",
            ),
        ),
    ];
    for (failure, expected) in cases {
        assert_eq!(ShioriFault::from_failure(&failure), expected, "{failure:?}");
    }
}

#[test]
fn from_down_maps_both_kinds_and_keeps_reason_verbatim() {
    assert_eq!(
        ShioriFault::from_down(ShioriDownKind::ConnectFailed, "no helper".to_string()),
        fault(ShioriFaultKind::ConnectFailed, "no helper"),
    );
    assert_eq!(
        ShioriFault::from_down(ShioriDownKind::HelperExited, "exit 3".to_string()),
        fault(ShioriFaultKind::Disconnected, "exit 3"),
    );
}

#[test]
fn unknown_is_kind_unknown_with_fixed_reason() {
    assert_eq!(
        ShioriFault::unknown(),
        fault(ShioriFaultKind::Unknown, "stop cause unknown"),
    );
}
