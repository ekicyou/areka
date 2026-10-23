//! 要求の出どころの規則の檻（要件 1.1・1.3・1.7・2.2〜2.7・3.3・3.4）。
//!
//! design の写しの表の 5 行・4 つのカテゴリの基本の出どころ・応答のレベルの解釈の 4 通りを見る。

use super::*;
use crate::schedule::log_capture::{capture, logged_once};
use tracing::Level;

/// 要件の値の 4 つのヘッダ（`SecurityOrigin` は常に `null`）。
fn row(security_level: &'static str, sender: &'static str, sender_type: &str) -> OriginWire {
    OriginWire {
        security_level,
        security_origin: "null",
        sender,
        sender_type: sender_type.to_string(),
    }
}

#[test]
fn areka_events_are_areka_internal_local() {
    // 表の 1 行目: 運行表・選択肢・アンカーのイベント（2.2・1.7）。
    for id in [
        EventId::Static("OnBoot"),
        EventId::Static("OnAnchorSelectEx"),
        EventId::Choice("OnMyChoice".to_string()),
    ] {
        assert_eq!(
            RequestOrigin::of_event(&id).wire(),
            row("local", "areka", "internal"),
            "{id:?}"
        );
    }
}

#[test]
fn raise_event_is_mcp_external_sstp() {
    // 表の 2 行目: MCP の raise_event（2.3）。
    let id = EventId::Sstp("OnInstallCompleteEx".to_string());
    assert_eq!(
        RequestOrigin::of_event(&id).wire(),
        row("local", "MCP", "external,sstp")
    );
}

#[test]
fn sakurascript_translate_is_mcp_internal() {
    // 表の 3 行目: MCP の sakurascript の OnTranslate（2.4）。
    let id = EventId::Translate(RequestOrigin::MCP_SEND_TRANSLATE);
    assert_eq!(
        RequestOrigin::of_event(&id).wire(),
        row("local", "MCP", "internal")
    );
    assert_eq!(id.as_str(), "OnTranslate");
}

#[test]
fn ghost_reply_translate_is_areka_internal() {
    // 表の 4 行目: ゴースト自身の応答の OnTranslate（2.5）。
    for source in [
        EventId::Static("OnBoot"),
        EventId::Choice("OnMyChoice".to_string()),
    ] {
        let id = EventId::Translate(RequestOrigin::for_translated(&source));
        assert_eq!(
            RequestOrigin::of_event(&id).wire(),
            row("local", "areka", "internal"),
            "{source:?}"
        );
    }
}

#[test]
fn raise_event_reply_translate_is_areka_external_sstp() {
    // 表の 5 行目: raise_event の応答の OnTranslate（2.6）。
    let source = EventId::Sstp("OnInstallCompleteEx".to_string());
    let id = EventId::Translate(RequestOrigin::for_translated(&source));
    assert_eq!(
        RequestOrigin::of_event(&id).wire(),
        row("local", "areka", "external,sstp")
    );
}

#[test]
fn base_origins_of_the_four_categories() {
    // 応答の台本の基本の出どころ（1.1・1.3・1.7）。
    assert_eq!(
        RequestOrigin::base_of(&EventId::Static("OnBoot")),
        ScriptOrigin::GHOST
    );
    assert_eq!(
        RequestOrigin::base_of(&EventId::Choice("OnMyChoice".to_string())),
        ScriptOrigin::GHOST
    );
    assert_eq!(
        RequestOrigin::base_of(&EventId::Sstp("OnX".to_string())),
        ScriptOrigin::MCP_NOTIFY
    );
    assert_eq!(
        RequestOrigin::base_of(&EventId::Translate(RequestOrigin::MCP_SEND_TRANSLATE)),
        ScriptOrigin::MCP_SEND
    );
}

#[test]
fn response_level_absent_is_local() {
    let events = capture(|| {
        assert_eq!(read_response_level(None, "OnBoot"), SecurityLevel::Local);
    });
    assert!(events.is_empty(), "{events:#?}");
}

#[test]
fn response_level_local_is_local() {
    assert_eq!(
        read_response_level(Some("LOCAL"), "OnBoot"),
        SecurityLevel::Local
    );
}

#[test]
fn response_level_external_is_external_ignoring_case() {
    assert_eq!(
        read_response_level(Some("External"), "OnBoot"),
        SecurityLevel::External
    );
}

#[test]
fn response_level_unreadable_warns_once_and_is_local() {
    let events = capture(|| {
        assert_eq!(
            read_response_level(Some("remote"), "OnBoot"),
            SecurityLevel::Local
        );
    });
    let ev = logged_once(&events, Level::WARN, "response_security_level_unreadable");
    assert_eq!(ev.fields.get("id").map(String::as_str), Some("OnBoot"));
    assert_eq!(ev.fields.get("value").map(String::as_str), Some("remote"));
    assert_eq!(events.len(), 1, "{events:#?}");
}
