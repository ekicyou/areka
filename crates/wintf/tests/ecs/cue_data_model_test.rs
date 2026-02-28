//! ユニットテスト: データモデルの検証（Task 4.4）
//!
//! - CueSheet の start_time 昇順ソート保証
//! - CueSheet の filter_by_actor() 動作
//! - CueCommand の is_barrier() / is_routing_command() 分類
//! - TimedCue のメモリーサイズ（≤ 64B）

use wintf::ecs::cue::*;

// ── CueSheet ──

#[test]
fn cue_sheet_sorts_by_start_time_ascending() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 2.0,
            command: CueCommand::Clear,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.5,
            command: CueCommand::Text("hello".into()),
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 1.0,
            command: CueCommand::Clear,
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
            command: CueCommand::Text("hi".into()),
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 0.5,
            command: CueCommand::Clear,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            command: CueCommand::Clear,
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
            command: CueCommand::Clear,
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            command: CueCommand::Clear,
        },
        Cue {
            actor: ActorKey::from("unyu"),
            start_time: 0.5,
            command: CueCommand::Clear,
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

// ── CueCommand 分類 ──

#[test]
fn cue_command_is_barrier() {
    assert!(CueCommand::WaitForClick { timeout: None }.is_barrier());
    assert!(CueCommand::WaitForChoice { timeout: Some(5.0) }.is_barrier());
    assert!(!CueCommand::Text("hello".into()).is_barrier());
    assert!(!CueCommand::Clear.is_barrier());
    assert!(!CueCommand::Emote {
        key: "smile".into()
    }
    .is_barrier());
}

#[test]
fn cue_command_is_routing_command() {
    assert!(CueCommand::RouteAdd {
        target: CueTarget::Shell,
        to: EntityKey::Spot("s0".into()),
    }
    .is_routing_command());
    assert!(CueCommand::RouteSwitch {
        target: CueTarget::Balloon,
        to: EntityKey::Balloon("b0".into()),
    }
    .is_routing_command());
    assert!(CueCommand::RouteRemove {
        target: CueTarget::Shell,
    }
    .is_routing_command());

    assert!(!CueCommand::Text("hello".into()).is_routing_command());
    assert!(!CueCommand::WaitForClick { timeout: None }.is_routing_command());
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

// ── TimedCue サイズ検証 ──

#[test]
fn timed_cue_size_within_64_bytes() {
    let size = std::mem::size_of::<TimedCue>();
    assert!(
        size <= 64,
        "TimedCue size is {} bytes, exceeding 64 byte limit (NFR-1)",
        size
    );
}
