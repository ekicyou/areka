//! `mcp` の決定論テスト（汲む系・振り分け・準備の前・終了の途中）。

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;

use areka_actor::ReplyError;
use areka_mcp::tools::{
    Pending, ToolCall, ToolRequest, dump_balloon, dump_surface, get_expression_table, get_log,
    get_property, get_status, outcome, raise_event, reload, sakurascript,
};
use areka_mcp::{ToolContent, ToolOutcome};
use bevy_ecs::world::World;

use super::resolve::{ActiveGhost, CANNOT_FIND, NOT_ACTIVE};
use super::*;

fn ghost() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    }
}

/// 解決を通る 8 本（`get_expression_table` と任意の 7 本）。
fn eight(ghost_name: Option<&str>) -> Vec<ToolCall> {
    let g = ghost_name.map(str::to_owned);
    vec![
        ToolCall::GetExpressionTable(get_expression_table::Args {
            ghost_name: g.clone(),
        }),
        ToolCall::GetStatus(get_status::Args {
            ghost_name: g.clone(),
        }),
        ToolCall::GetProperty(get_property::Args {
            property_name: "currentghost.name".to_string(),
            ghost_name: g.clone(),
        }),
        ToolCall::Sakurascript(sakurascript::Args {
            script: r"\0\e".to_string(),
            ghost_name: g.clone(),
            strict: None,
        }),
        ToolCall::RaiseEvent(raise_event::Args {
            event: "OnTest".to_string(),
            references: vec![],
            ghost_name: g.clone(),
            strict: None,
        }),
        ToolCall::Reload(reload::Args {
            target: "shiori".to_string(),
            ghost_name: g.clone(),
        }),
        ToolCall::DumpSurface(dump_surface::Args {
            scope: None,
            surface: None,
            ghost_name: g.clone(),
        }),
        ToolCall::DumpBalloon(dump_balloon::Args {
            scope: None,
            ghost_name: g,
        }),
    ]
}

fn get_log_call() -> ToolCall {
    ToolCall::GetLog(get_log::Args {
        log_type: None,
        ghost_name: None,
        since_id: None,
        max_count: None,
    })
}

/// 振り分けて、その場で届いた答えの本文（まだ答えていなければ None）。
fn dispatch_text(active: Option<&ActiveGhost>, call: ToolCall) -> Option<String> {
    let mut world = World::new();
    let (request, pending) = ToolRequest::new(call);
    dispatch(&mut world, active, request);
    text_of(&pending)
}

fn text_of(pending: &Pending) -> Option<String> {
    let answer = pending.try_answer().ok().flatten()?;
    match answer.outcome.content.first() {
        Some(ToolContent::Text(text)) => Some(text.clone()),
        _ => Some(String::new()),
    }
}

fn ng(reason: &str) -> Option<String> {
    Some(format!("NG:{reason}"))
}

// ---- 振り分け（3.1・3.4・3.5・3.7） ----

#[test]
fn eight_with_no_active_ghost_are_not_active() {
    for call in eight(None) {
        let name = call.name();
        assert_eq!(dispatch_text(None, call), ng(NOT_ACTIVE), "{name}");
    }
}

#[test]
fn get_expression_table_omitted_is_not_active_even_with_one_ghost() {
    let g = ghost();
    for ghost_name in [None, Some("")] {
        let call = ToolCall::GetExpressionTable(get_expression_table::Args {
            ghost_name: ghost_name.map(str::to_owned),
        });
        assert_eq!(dispatch_text(Some(&g), call), ng(NOT_ACTIVE));
    }
}

#[test]
fn eight_with_a_wrong_name_cannot_find() {
    let g = ghost();
    for call in eight(Some("Someone else")) {
        let name = call.name();
        assert_eq!(dispatch_text(Some(&g), call), ng(CANNOT_FIND), "{name}");
    }
}

#[test]
fn get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure() {
    let g = ghost();
    let mut cases: Vec<(Option<&ActiveGhost>, ToolCall)> = vec![(None, get_log_call())];
    // `get_expression_table` を除いた 7 本（省略は起動中の 1 体へ解決する）。
    cases.extend(eight(None).into_iter().skip(1).map(|c| (Some(&g), c)));
    assert_eq!(cases.len(), 8);
    for (active, call) in cases {
        let name = call.name();
        let text = dispatch_text(active, call);
        assert_ne!(text, ng(NOT_ACTIVE), "{name}");
        assert_ne!(text, ng(CANNOT_FIND), "{name}");
    }
}

// ---- 後から答える（6.4・6.6） ----

fn installed() -> (World, mpsc::Sender<ToolRequest>) {
    let mut world = World::new();
    let (tx, rx) = mpsc::channel();
    install(&mut world, rx);
    (world, tx)
}

fn later_pair() -> (ToolRequest, Pending) {
    ToolRequest::new(ToolCall::GetActiveGhostList)
}

#[test]
fn later_answers_on_the_next_frame() {
    let (mut world, _tx) = installed();
    let (request, pending) = later_pair();
    let mut first = true;
    later(&mut world, request.reply, move |_| {
        if std::mem::take(&mut first) {
            None
        } else {
            Some(outcome::value("late"))
        }
    });

    drain(&mut world);
    assert!(
        matches!(pending.try_answer(), Ok(None)),
        "預けたフレームでは未着"
    );
    drain(&mut world);
    assert_eq!(text_of(&pending).as_deref(), Some("late"));
}

#[test]
fn later_answers_in_the_same_frame_when_ready() {
    let (mut world, _tx) = installed();
    let (request, pending) = later_pair();
    later(&mut world, request.reply, |_| Some(outcome::value("now")));

    drain(&mut world);
    assert_eq!(text_of(&pending).as_deref(), Some("now"));
}

#[test]
fn close_drops_what_later_holds() {
    let (mut world, _tx) = installed();
    let (request, pending) = later_pair();
    later(&mut world, request.reply, |_| None::<ToolOutcome>);
    drain(&mut world);

    close(&mut world);
    assert!(matches!(pending.try_answer(), Err(ReplyError::Dropped)));
}

#[test]
fn abandoned_pair_is_removed_without_polling() {
    let (mut world, _tx) = installed();
    let (request, pending) = later_pair();
    let polls = Rc::new(Cell::new(0usize));
    let counter = polls.clone();
    later(&mut world, request.reply, move |_| {
        counter.set(counter.get() + 1);
        None
    });
    drain(&mut world);
    assert_eq!(polls.get(), 1);

    drop(pending);
    drain(&mut world);
    drain(&mut world);
    assert_eq!(polls.get(), 1, "待つ側の居ない組は覗かない");
    assert!(world.non_send::<McpLater>().0.is_empty());
}

#[test]
fn later_after_close_is_dropped_at_once() {
    let (mut world, _tx) = installed();
    close(&mut world);
    let (request, pending) = later_pair();
    later(&mut world, request.reply, |_| Some(outcome::value("never")));
    assert!(matches!(pending.try_answer(), Err(ReplyError::Dropped)));
}

#[test]
fn later_called_while_polling_is_kept() {
    let (mut world, _tx) = installed();
    let (outer, outer_pending) = later_pair();
    let (inner, inner_pending) = later_pair();
    let mut inner = Some(inner.reply);
    later(&mut world, outer.reply, move |world| {
        let reply = inner.take()?;
        later(world, reply, |_| Some(outcome::value("inner")));
        Some(outcome::value("outer"))
    });

    drain(&mut world);
    assert_eq!(text_of(&outer_pending).as_deref(), Some("outer"));
    drain(&mut world);
    assert_eq!(text_of(&inner_pending).as_deref(), Some("inner"));
}

// ---- 汲む系・準備の前・終了の途中（6.1・6.4・6.5） ----

#[test]
fn request_sent_before_install_is_answered_after_it() {
    let mut world = World::new();
    let (tx, rx) = mpsc::channel();
    let (request, pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    tx.send(request).ok().expect("受け口は生きている");

    install(&mut world, rx);
    drain(&mut world);

    // 置き場の無い World＝0 体＝空の本文。
    let answer = pending
        .try_answer()
        .ok()
        .flatten()
        .expect("汲んだフレームで答える");
    assert_eq!(answer.outcome, outcome::value(""));
}

#[test]
fn drain_resolves_from_the_world() {
    let (mut world, tx) = installed();
    let (request, pending) = ToolRequest::new(eight(None).remove(1));
    tx.send(request).ok().expect("受け口は生きている");

    drain(&mut world);
    assert_eq!(text_of(&pending), ng(NOT_ACTIVE));
}

#[test]
fn close_drops_queued_requests_and_later_sends_fail() {
    let (mut world, tx) = installed();
    let (request, pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    tx.send(request).ok().expect("受け口は生きている");

    close(&mut world);
    assert!(matches!(pending.try_answer(), Err(ReplyError::Dropped)));
    assert!(world.get_non_send::<McpInbox>().is_none());
    assert!(world.get_non_send::<McpLater>().is_none());

    let (request, _pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    assert!(tx.send(request).is_err(), "閉じた後の送りは失敗する");
}

#[test]
fn drain_without_inbox_is_a_no_op() {
    let mut world = World::new();
    drain(&mut world);
    assert!(world.get_non_send::<McpInbox>().is_none());
    assert!(world.get_non_send::<McpLater>().is_none());
}

// ---- 本物の単位で通す（3.8・4.1・6.1） ----

/// 偽の SHIORI で実行系つきの単位を起こし（`register_systems` で汲む系も登録済み）、受け口を置いて
/// `get_active_ghost_list` を送り、`Input` の段を 1 回回すと descript の `name` で答える。
#[test]
fn real_unit_answers_get_active_ghost_list_in_one_frame() {
    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e"))),
    )]);
    rig.boot("A");
    let (tx, rx) = mpsc::channel();
    install(&mut rig.world, rx);
    let (request, pending) = ToolRequest::new(ToolCall::GetActiveGhostList);
    tx.send(request).ok().expect("受け口は生きている");

    rig.world.run_schedule(wintf::ecs::Input);
    let answer = pending.try_answer().ok().flatten();
    let active = resolve::active(&rig.world).map(|g| g.name);
    let down = rig.shutdown();

    let answer = answer.expect("1 フレームで答える");
    assert_eq!(
        (
            answer.outcome.clone(),
            answer.outcome.is_error,
            active,
            down
        ),
        (outcome::value("A"), false, Some(Some("A".to_owned())), true),
        "（答え・isError・起動中のゴーストの名前・降ろせた）"
    );
}

/// LogSink へ倒れた単位（バルーンの根だけが無い＝倒れた先は実行系つきで起きる）も起動中として読める。
#[test]
fn logsink_fallen_unit_is_active() {
    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig};
    use crate::ghost_session::GhostSlot;

    let mut rig = SwitchRig::new(vec![("A", FakeShiori::BalloonMissing)]);
    rig.boot("A");
    let fallen = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(|s| (s.logsink_fallback(), s.runtime().is_some()));
    let active = resolve::active(&rig.world).map(|g| g.name);
    let down = rig.shutdown();

    assert_eq!(
        (fallen, active, down),
        (Some((true, true)), Some(Some("A".to_owned())), true),
        "（倒れた旗と実行系・起動中のゴーストの名前・降ろせた）"
    );
}
