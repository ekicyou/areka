//! 運行表の SSTP の入口の決定論テスト（`schedule/sstp.rs` の兄弟）。
//!
//! 今は `events.rs` の 2 つの組み立て（`sstp_notify`・`on_translate_sstp`）を固定する。

use crate::msg::{EventId, ShioriCall};
use crate::schedule::events::{REF1_SSTP_SEND_OWNED, on_translate_sstp, sstp_notify};
use crate::status::{ExecutionSnapshot, ExecutionStatus};

fn strings(refs: &[&str]) -> Vec<String> {
    refs.iter().map(|r| r.to_string()).collect()
}

/// 任意の名前の GET は `EventId::Sstp` で、渡された Reference をそのまま載せる（要件 2.1・2.2）。
#[test]
fn sstp_notify_is_a_get_with_the_given_references_verbatim() {
    let cases: [&[&str]; 4] = [&[], &["a"], &["", "x", ""], &["0\u{1}1", "改\r行"]];
    for refs in cases {
        match sstp_notify(
            "OnNotListed_ほげ".to_string(),
            strings(refs),
            &ExecutionSnapshot::INACTIVE,
        ) {
            ShioriCall::Get {
                id,
                references,
                status,
            } => {
                assert_eq!(id, EventId::Sstp("OnNotListed_ほげ".to_string()));
                assert_eq!(references, strings(refs), "Reference={refs:?}");
                assert_eq!(
                    status,
                    ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE)
                );
            }
            ShioriCall::Notify { .. } => panic!("SstpNotify は GET で送る"),
        }
    }
}

/// 許可の表に在る名前でも Reference を補わない（0 個なら 0 個・要件 2.2）。
#[test]
fn sstp_notify_does_not_pad_references_for_listed_names() {
    for name in ["OnBalloonTimeout", "OnAnchorSelect", "OnBoot"] {
        match sstp_notify(name.to_string(), Vec::new(), &ExecutionSnapshot::INACTIVE) {
            ShioriCall::Get { id, references, .. } => {
                assert_eq!(id, EventId::Sstp(name.to_string()));
                assert!(references.is_empty(), "{name}: Reference を補わない");
            }
            ShioriCall::Notify { .. } => panic!("SstpNotify は GET で送る"),
        }
    }
}

/// 実行状態は渡した snapshot から導く（話し中なら `talking`）。
#[test]
fn sstp_notify_derives_status_from_the_snapshot() {
    let snap = ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    };
    match sstp_notify("OnX".to_string(), Vec::new(), &snap) {
        ShioriCall::Get { status, .. } => assert_eq!(status, ExecutionStatus::derive(&snap)),
        ShioriCall::Notify { .. } => panic!("SstpNotify は GET で送る"),
    }
}

/// SSTP SEND の台本の `OnTranslate`: Ref0＝展開済み・Ref1＝`sstp-send,owned`・Ref2 以降なし（要件 4.4）。
#[test]
fn on_translate_sstp_has_reference0_and_reference1_only() {
    assert_eq!(REF1_SSTP_SEND_OWNED, "sstp-send,owned");
    let status = ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE);
    match on_translate_sstp("\\h外からの台本\\e", status.clone()) {
        ShioriCall::Get {
            id,
            references,
            status: got,
        } => {
            assert_eq!(id, EventId::Static("OnTranslate"));
            assert_eq!(
                references,
                vec![
                    "\\h外からの台本\\e".to_string(),
                    "sstp-send,owned".to_string()
                ]
            );
            assert_eq!(got, status);
        }
        ShioriCall::Notify { .. } => panic!("OnTranslate は GET で送る"),
    }
}
