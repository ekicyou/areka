//! 殻のバッチ実行の翻訳の腕の檻（タスク 3.3）: `Action::Translate` を実行した結果がそのバッチの
//! 「入れ直すもの」になり、SHIORI の往復の結果とは同時に残らない（後に実行した往復の方だけ）。

use std::sync::mpsc;
use std::thread;

use super::execute_actions;
use crate::msg::{EventId, ShioriCall, ShioriMsg, ShioriOutcome};
use crate::schedule::Action;
use crate::schedule::events::SourceEvent;
use crate::schedule::resources::ResourceSink;
use crate::schedule::translate::TranslateRequest;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use crate::talk::TalkCommand;

/// 受けた呼出の ID を順に返す偽の SHIORI（Request には 204、Unload には `Unloaded` を返す）。
fn run_batch(actions: Vec<Action>) -> (super::BatchResult, Vec<String>) {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let helper = thread::spawn(move || {
        let mut ids = Vec::new();
        for msg in shiori_rx {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    let (ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. }) = &call;
                    ids.push(id.as_str().to_string());
                    let _ = reply.send(ShioriOutcome::NoContent);
                }
                ShioriMsg::Unload { reply } => {
                    ids.push("Unload".to_string());
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                _ => {}
            }
        }
        ids
    });
    let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();
    let sink: ResourceSink = Box::new(|_, _| {});
    let result = execute_actions(actions, &shiori_tx, &sakura_tx, &sink, None, (None, None));
    drop(shiori_tx);
    (result, helper.join().expect("fake shiori thread"))
}

fn translate(script: &str) -> Action {
    Action::Translate(TranslateRequest {
        script: script.to_string(),
        source: SourceEvent {
            id: EventId::Static("OnBoot"),
            references: Vec::new(),
        },
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    })
}

#[test]
fn translate_result_becomes_the_reinput_of_the_batch() {
    let (result, ids) = run_batch(vec![translate("\\0元\\e")]);
    assert_eq!(ids, ["OnTranslate"], "翻訳の依頼は OnTranslate を 1 回送る");
    assert!(result.last_reply.is_none(), "SHIORI の応答は入れ直さない");
    assert_eq!(
        result.translated.map(|r| r.ok()),
        Some(Some("\\0元\\e".to_string())),
        "204 は元の台詞（素通しの口）で翻訳の結果になる"
    );
    assert!(!result.stop);
}

#[test]
fn the_later_round_trip_wins_the_reinput() {
    let (result, ids) = run_batch(vec![translate("\\0元\\e"), Action::ShioriUnload]);
    assert_eq!(ids, ["OnTranslate", "Unload"]);
    assert!(result.translated.is_none(), "後の往復が入れ直すものになる");
    assert!(matches!(
        result.last_reply,
        Some((ShioriOutcome::Unloaded, "Unload"))
    ));
}
