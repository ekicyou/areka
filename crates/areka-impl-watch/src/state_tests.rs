//! 状態の型の決定論テスト: JSON の往復・版の読み取り・空の状態・`clear` の後の状態・
//! `recent` の上限。

use std::path::Path;

use serde_json::{Value, json};

use super::{ParticipantStatus, RECENT_MAX, Recent, RecentKind, State, VERSION, WaitKind};

/// 設計「Data Models / 状態ファイル `state.json`（版 1）」の見本そのまま。
const SAMPLE: &str = r#"{
  "version": 1,
  "participants": {
    "A": { "id": "A", "name": "A", "repo": "areka", "status": "working", "since": 1791000000,
           "stop_reason": null, "watch": { "pid": 1234, "since": 1791000000 }, "awaiting_watch_since": null }
  },
  "merge": {
    "areka": { "holder": { "id": "A", "spec": "x", "bug": false, "requested": 1791000010, "granted": 1791000011 },
               "queue": [], "last": { "pr": "281", "sha": "414d43eb", "spec": "y", "at": 1790999000 } }
  },
  "load": { "holder": null, "queue": [ { "id": "C", "purpose": "load-flake 5 回", "requested": 1791000020 } ] },
  "waits": [ { "id": "A", "kind": "watch", "repo": "areka", "pid": 1234, "since": 1791000000 } ],
  "recent": [ { "at": 1791000005, "kind": "reclaimed", "id": "Z", "detail": "watch absent" } ]
}"#;

fn read(text: &str) -> State {
    serde_json::from_str(text).expect("状態として読める")
}

fn recent(at: u64) -> Recent {
    Recent {
        at,
        kind: RecentKind::Reclaimed,
        id: Some("Z".to_owned()),
        detail: "watch absent".to_owned(),
    }
}

#[test]
fn design_sample_reads_and_writes_back_the_same() {
    let state = read(SAMPLE);
    let written = serde_json::to_string_pretty(&state).expect("書ける");

    // 項目が 1 つでも読み落とされる・綴りが違うと、書き戻した JSON が見本と食い違う。
    let sample: Value = serde_json::from_str(SAMPLE).expect("見本は JSON");
    let rewritten: Value = serde_json::from_str(&written).expect("書いたものは JSON");
    assert_eq!(rewritten, sample);
    assert_eq!(read(&written), state);

    assert_eq!(state.version, VERSION);
    assert_eq!(state.participants["A"].status, ParticipantStatus::Working);
    assert_eq!(state.waits[0].kind, WaitKind::Watch);
    assert_eq!(state.recent[0].kind, RecentKind::Reclaimed);
}

#[test]
fn enums_are_spelled_in_lowercase_kebab() {
    let spelled = |value: Value| value.as_str().expect("文字列").to_owned();
    let status = [
        (ParticipantStatus::Working, "working"),
        (ParticipantStatus::StopRequested, "stop-requested"),
        (ParticipantStatus::Stopped, "stopped"),
    ];
    for (value, text) in status {
        assert_eq!(spelled(json!(value)), text);
    }
    let waits = [
        (WaitKind::Watch, "watch"),
        (WaitKind::Merge, "merge"),
        (WaitKind::Load, "load"),
        (WaitKind::Resume, "resume"),
    ];
    for (value, text) in waits {
        assert_eq!(spelled(json!(value)), text);
    }
    let recents = [
        (RecentKind::Reclaimed, "reclaimed"),
        (RecentKind::Recovered, "recovered"),
        (RecentKind::Cleared, "cleared"),
    ];
    for (value, text) in recents {
        assert_eq!(spelled(json!(value)), text);
    }
}

#[test]
fn empty_state_is_written_with_version_1() {
    assert_eq!(VERSION, 1);
    assert_eq!(
        json!(State::empty()),
        json!({
            "version": 1,
            "participants": {},
            "merge": {},
            "load": { "holder": null, "queue": [] },
            "waits": [],
            "recent": []
        })
    );
}

#[test]
fn missing_fields_of_the_same_version_still_read() {
    assert_eq!(read(r#"{ "version": 1 }"#), State::empty());

    // 項目の欠けた記録と、この実行ファイルの知らない項目（同じ版の中で後から増えたもの）。
    let state = read(
        r#"{ "version": 1, "added_later": true,
             "participants": { "A": { "id": "A" } },
             "merge": { "areka": {} },
             "load": { "holder": { "id": "C" } },
             "recent": [ { "kind": "cleared" } ] }"#,
    );
    let a = &state.participants["A"];
    assert_eq!(a.status, ParticipantStatus::Working);
    assert_eq!((a.watch.as_ref(), a.awaiting_watch_since), (None, None));
    assert_eq!(state.merge["areka"], Default::default());
    let holder = state.load.holder.as_ref().expect("持ち主が居る");
    assert!(!holder.running && holder.stopped.is_empty());
    assert_eq!(state.recent[0].id, None);
}

#[test]
fn a_file_without_version_does_not_read_as_version_1() {
    assert_ne!(read("{}").version, VERSION);
}

#[test]
fn a_record_without_its_kind_does_not_read() {
    // 種類は欠けたときに入れる中立の値が無いので、形の合わないファイルとして断る。
    for text in [
        r#"{ "version": 1, "waits": [ { "id": "A" } ] }"#,
        r#"{ "version": 1, "recent": [ { "at": 1 } ] }"#,
    ] {
        assert!(serde_json::from_str::<State>(text).is_err(), "{text}");
    }
}

#[test]
fn cleared_state_has_only_the_cleared_record() {
    let backup = Path::new("C:\\置き場\\state.json.cleared-20261010T000000Z");
    let state = State::cleared(1791000100, backup);
    assert_eq!(
        state,
        State {
            recent: vec![Recent {
                at: 1791000100,
                kind: RecentKind::Cleared,
                id: None,
                detail: "C:\\置き場\\state.json.cleared-20261010T000000Z".to_owned(),
            }],
            ..State::empty()
        }
    );
}

#[test]
fn recent_keeps_the_newest_50_newest_first() {
    assert_eq!(RECENT_MAX, 50);
    let mut state = State::empty();
    for at in 0..=50 {
        state.push_recent(recent(at));
    }
    assert_eq!(state.recent.len(), 50);
    assert_eq!(state.recent.first().map(|r| r.at), Some(50));
    assert_eq!(state.recent.last().map(|r| r.at), Some(1));
}
