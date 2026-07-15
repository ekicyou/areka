//! ユニットテスト: データモデルの検証（Task 4.4）
//!
//! - CueSheet の start_time 昇順ソート保証
//! - CueSheet の filter_by_actor() 動作
//! - CuePayload の分類（Command / Barrier / Routing）
//! - Entry<CueCommand> のメモリーサイズ

use wintf::ecs::cue::*;

// ── CueSheet ──

#[test]
fn cue_sheet_sorts_by_start_time_ascending() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 2.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.5,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let cues = sheet.cues();
    assert_eq!(cues.len(), 3);
    assert!(cues[0].start_time <= cues[1].start_time);
    assert!(cues[1].start_time <= cues[2].start_time);
}

#[test]
fn cue_sheet_filter_by_actor() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hi".into()).into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 0.5,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let sakura_key = ActorKey::from("sakura");
    let sakura_cues = sheet.filter_by_actor(&sakura_key);
    assert_eq!(sakura_cues.len(), 2);
    assert!(sakura_cues.iter().all(|c| c.actor == sakura_key));
}

#[test]
fn cue_sheet_actors_dedup() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 0.5,
            payload: CueCommand::Clear.into(),
            duration: 0.0,
        },
    ]);

    let actors = sheet.actors();
    assert_eq!(actors.len(), 2);
}

#[test]
fn cue_sheet_empty() {
    let sheet = CueSheet::new(vec![]);
    assert!(sheet.is_empty());
    assert_eq!(sheet.len(), 0);
}

// ── CuePayload 分類 ──

#[test]
fn cue_payload_command_variant() {
    let payload: CuePayload = CueCommand::Text("hello".into()).into();
    assert!(matches!(payload, CuePayload::Command(_)));
}

#[test]
fn cue_payload_barrier_variant() {
    let payload: CuePayload = BarrierKind::WaitForInput { timeout: None }.into();
    assert!(matches!(payload, CuePayload::Barrier(_)));

    let payload2: CuePayload = BarrierKind::WaitForChoice { timeout: Some(5.0) }.into();
    assert!(matches!(payload2, CuePayload::Barrier(_)));
}

#[test]
fn cue_payload_routing_variant() {
    let payload: CuePayload = RoutingCommand::RouteAdd {
        target: CueTarget::Shell,
        to: EntityKey::Spot("s0".into()),
    }
    .into();
    assert!(matches!(payload, CuePayload::Routing(_)));

    let payload2: CuePayload = RoutingCommand::RouteRemove {
        target: CueTarget::Shell,
    }
    .into();
    assert!(matches!(payload2, CuePayload::Routing(_)));
}

// ── ActorKey ──

#[test]
fn actor_key_conversions() {
    let key1 = ActorKey::from("sakura");
    let key2 = ActorKey::from(String::from("sakura"));
    assert_eq!(key1, key2);
    assert_eq!(key1.as_str(), "sakura");
    assert_eq!(format!("{}", key1), "sakura");
}

// ── Entry サイズ検証 ──

#[test]
fn entry_size_within_limit() {
    let size = std::mem::size_of::<Entry<CueCommand>>();
    assert!(
        size <= 128,
        "Entry<CueCommand> size is {} bytes, exceeding 128 byte limit",
        size
    );
    eprintln!("[info] Entry<CueCommand> size: {} bytes", size);
}
