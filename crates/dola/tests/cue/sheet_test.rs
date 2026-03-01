//! CueSheet と compile_sheet のユニットテスト
//!
//! Task 6.3: 昇順ソート、0 ベース正規化、into_entry 3 種変換の検証。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, CueTarget,
    RoutingCommand, compile_sheet,
};

// ============================================================================
// CueSheet 構築テスト
// ============================================================================

#[test]
fn cue_sheet_sorts_by_start_time() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 3.0,
            payload: CueCommand::Clear.into(),
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 1.0,
            payload: CueCommand::Text("first".into()).into(),
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 2.0,
            payload: CueCommand::Text("second".into()).into(),
        },
    ]);

    let times: Vec<f64> = sheet.cues().iter().map(|c| c.start_time).collect();
    assert_eq!(times, vec![1.0, 2.0, 3.0]);
}

#[test]
fn cue_sheet_empty() {
    let sheet = CueSheet::new(vec![]);
    assert!(sheet.is_empty());
    assert_eq!(sheet.len(), 0);
}

#[test]
fn cue_sheet_len() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 0.0,
            payload: CueCommand::Clear.into(),
        },
        Cue {
            actor: ActorKey::from("b"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
        },
    ]);
    assert_eq!(sheet.len(), 2);
    assert!(!sheet.is_empty());
}

// ============================================================================
// Actor フィルタリングテスト
// ============================================================================

#[test]
fn filter_by_actor() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 0.5,
            payload: CueCommand::Emote { key: "grumble".into() }.into(),
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
        },
    ]);

    let sakura_cues = sheet.filter_by_actor(&ActorKey::from("sakura"));
    assert_eq!(sakura_cues.len(), 2);

    let kero_cues = sheet.filter_by_actor(&ActorKey::from("kero"));
    assert_eq!(kero_cues.len(), 1);

    let none_cues = sheet.filter_by_actor(&ActorKey::from("nobody"));
    assert!(none_cues.is_empty());
}

#[test]
fn actors_unique_list() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Clear.into(),
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 0.5,
            payload: CueCommand::Clear.into(),
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
        },
    ]);

    let actors = sheet.actors();
    assert_eq!(actors.len(), 2);
    assert!(actors.contains(&&ActorKey::from("sakura")));
    assert!(actors.contains(&&ActorKey::from("kero")));
}

// ============================================================================
// compile_sheet テスト
// ============================================================================

#[test]
fn compile_sheet_normalizes_to_zero_base() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("a"),
            start_time: 5.0,
            payload: CueCommand::Text("first".into()).into(),
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 7.0,
            payload: CueCommand::Text("second".into()).into(),
        },
        Cue {
            actor: ActorKey::from("a"),
            start_time: 10.0,
            payload: CueCommand::Clear.into(),
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled.len(), 3);

    // 最小 start_time=5.0 を 0 に正規化
    let offsets: Vec<f64> = compiled.iter().map(|c| c.offset).collect();
    assert_eq!(offsets, vec![0.0, 2.0, 5.0]);
}

#[test]
fn compile_sheet_empty() {
    let sheet = CueSheet::new(vec![]);
    let compiled = compile_sheet(&sheet);
    assert!(compiled.is_empty());
}

#[test]
fn compile_sheet_preserves_actor() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 1.0,
            payload: CueCommand::Clear.into(),
        },
    ]);

    let compiled = compile_sheet(&sheet);
    assert_eq!(compiled[0].actor.as_str(), "sakura");
    assert_eq!(compiled[1].actor.as_str(), "kero");
}

// ============================================================================
// into_entry 3 種変換テスト
// ============================================================================

#[test]
fn into_entry_command_becomes_payload() {
    let payload = CuePayload::Command(CueCommand::Text("test".into()));
    let entry = payload.into_entry(1.5);
    match entry {
        dola::cue::Entry::Payload(t, cmd) => {
            assert_eq!(t, 1.5);
            assert!(matches!(cmd, CueCommand::Text(_)));
        }
        _ => panic!("Expected Entry::Payload"),
    }
}

#[test]
fn into_entry_barrier_becomes_barrier() {
    let payload = CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None });
    let entry = payload.into_entry(2.0);
    match entry {
        dola::cue::Entry::Barrier(t, kind) => {
            assert_eq!(t, 2.0);
            assert!(matches!(kind, BarrierKind::WaitForInput { .. }));
        }
        _ => panic!("Expected Entry::Barrier"),
    }
}

#[test]
fn into_entry_routing_becomes_routing() {
    let routing = RoutingCommand::RouteRemove {
        target: CueTarget::Shell,
    };
    let payload = CuePayload::Routing(routing);
    let entry = payload.into_entry(0.0);
    match entry {
        dola::cue::Entry::Routing(t, r) => {
            assert_eq!(t, 0.0);
            assert!(matches!(r, RoutingCommand::RouteRemove { .. }));
        }
        _ => panic!("Expected Entry::Routing"),
    }
}

// ============================================================================
// compile_sheet → TimedSchedule 統合テスト
// ============================================================================

#[test]
fn compile_sheet_to_timed_schedule_integration() {
    use dola::cue::TimedSchedule;

    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 2.0,
            payload: CueCommand::Text("hello".into()).into(),
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 2.5,
            payload: CuePayload::Barrier(BarrierKind::WaitForInput { timeout: None }),
        },
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 3.0,
            payload: CueCommand::Clear.into(),
        },
    ]);

    let compiled = compile_sheet(&sheet);
    let mut sched = TimedSchedule::<CueCommand>::new(10.0); // 絶対時刻 10.0 開始

    for cc in &compiled {
        sched.insert(cc.entry.clone());
    }

    // offset=0.0 → Text
    sched.tick(10.0);
    assert_eq!(sched.ready().len(), 1);
    assert!(matches!(sched.ready()[0], CueCommand::Text(_)));

    // offset=0.5 → Barrier 到達
    sched.tick(10.5);
    assert!(sched.current_barrier().is_some());

    // バリア解除後に Clear 取得
    sched.notify_barrier_resolved(None);
    sched.tick(11.0);
    assert!(matches!(sched.ready()[0], CueCommand::Clear));
}

// ============================================================================
// serde テスト
// ============================================================================

#[test]
fn cue_sheet_serde_roundtrip() {
    let sheet = CueSheet::new(vec![
        Cue {
            actor: ActorKey::from("sakura"),
            start_time: 0.0,
            payload: CueCommand::Text("hello".into()).into(),
        },
        Cue {
            actor: ActorKey::from("kero"),
            start_time: 1.0,
            payload: CuePayload::Barrier(BarrierKind::Timeout { duration: 3.0 }),
        },
    ]);

    let json = serde_json::to_string(&sheet).unwrap();
    let parsed: CueSheet = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed.cues()[0].actor.as_str(), "sakura");
    assert_eq!(parsed.cues()[1].actor.as_str(), "kero");
}
