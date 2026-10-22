//! 殻が送る `OnTranslate` の要求の出どころ（要件 1.5・2.4〜2.6）。
//!
//! ゴースト自身・`raise_event` の応答の台本と外からの台本の 3 つで、偽の SHIORI が受けた要求の
//! カテゴリから導く 4 つのヘッダが要件どおりになり、`Reference*` の並びが今のままであることを確かめる。

use std::sync::mpsc;
use std::thread;

use super::*;
use crate::msg::{EventId, ShioriCall, ShioriMsg, ShioriOutcome};
use crate::request_origin::{OriginWire, RequestOrigin};
use crate::schedule::events::SourceEvent;
use crate::schedule::translate::{ScriptSource, TranslateRequest};
use crate::shiori::real::ABSENT_REFERENCE;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use crate::translate::TranslateSeams;

/// 依頼を 204 を返す偽の SHIORI へ向けて実行し、受けた GET の ID と Reference を返す。
fn sent(source: ScriptSource) -> (EventId, Vec<String>) {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let helper = thread::spawn(move || match shiori_rx.recv() {
        Ok(ShioriMsg::Request { call, reply }) => {
            let _ = reply.send(ShioriOutcome::NoContent);
            Some(call)
        }
        _ => None,
    });
    let seams = TranslateSeams {
        expand: Box::new(|script| script.to_owned()),
        makoto: Box::new(|script, _| script.to_owned()),
    };
    let request = TranslateRequest {
        script: "\\e".to_string(),
        source,
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    };
    let _ = run_translate(request, &shiori_tx, &seams);
    drop(shiori_tx);
    match helper.join().expect("fake shiori thread") {
        Some(ShioriCall::Get { id, references, .. }) => (id, references),
        other => panic!(
            "OnTranslate の GET が届いていない（何か届いた: {}）",
            other.is_some()
        ),
    }
}

fn reply_of(id: EventId) -> ScriptSource {
    ScriptSource::Event(SourceEvent {
        id,
        references: vec!["r0".to_string()],
    })
}

fn wire(level: &'static str, sender: &'static str, sender_type: &str) -> OriginWire {
    OriginWire {
        security_level: level,
        security_origin: "null",
        sender,
        sender_type: sender_type.to_string(),
    }
}

#[test]
fn ghost_reply_translate_carries_areka_internal() {
    // 要件 2.5: ゴースト自身のイベント（運行表・選択肢）の応答の翻訳。
    for source in [
        EventId::Static("OnBoot"),
        EventId::Choice("OnMyChoice".to_string()),
    ] {
        let (id, references) = sent(reply_of(source.clone()));
        assert_eq!(
            id,
            EventId::Translate(RequestOrigin::for_translated(&source))
        );
        assert_eq!(
            RequestOrigin::of_event(&id).wire(),
            wire("local", "areka", "internal"),
            "{source:?}"
        );
        assert_eq!(references[1], ABSENT_REFERENCE, "Reference1 は今どおり欠番");
    }
}

#[test]
fn raise_event_reply_translate_carries_areka_external_sstp() {
    // 要件 2.6: raise_event の応答の翻訳（Reference1 は今どおり欠番）。
    let source = EventId::Sstp("OnInstallCompleteEx".to_string());
    let (id, references) = sent(reply_of(source.clone()));
    assert_eq!(
        id,
        EventId::Translate(RequestOrigin::for_translated(&source))
    );
    assert_eq!(
        RequestOrigin::of_event(&id).wire(),
        wire("local", "areka", "external,sstp")
    );
    assert_eq!(references[1], ABSENT_REFERENCE);
    assert_eq!(references[2], "OnInstallCompleteEx");
}

#[test]
fn sstp_send_translate_carries_mcp_internal() {
    // 要件 2.4: 外からの台本の翻訳（Reference1 は今どおり `sstp-send,owned`）。
    let (id, references) = sent(ScriptSource::SstpSend);
    assert_eq!(id, EventId::Translate(RequestOrigin::MCP_SEND_TRANSLATE));
    assert_eq!(
        RequestOrigin::of_event(&id).wire(),
        wire("local", "MCP", "internal")
    );
    assert_eq!(
        references,
        vec!["\\e".to_string(), "sstp-send,owned".to_string()]
    );
}
