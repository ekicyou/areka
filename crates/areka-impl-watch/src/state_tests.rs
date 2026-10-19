//! 状態の型の決定論テスト: JSON の往復・版の読み取り・空の状態・`clear` の後の状態・
//! `recent` の上限。

use std::path::Path;

use serde_json::{Value, json};

use super::{DeskSet, ParticipantStatus, RECENT_MAX, Recent, RecentKind, State, VERSION, WaitKind};

/// 設計「Data Models / 状態ファイル `state.json`（版 2）」の見本そのまま。
const SAMPLE: &str = r#"{
  "version": 2,
  "participants": { "a": { "id": "a", "name": "spec-x", "repo": "areka", "status": "working", "since": 0,
                           "stop_reason": null, "watch": { "pid": 1, "since": 0 }, "awaiting_watch_since": null } },
  "merge": { "areka": { "holder": { "id": "a", "spec": "spec-x", "bug": false, "requested": 0, "granted": 0 },
                        "queue": [], "last": null } },
  "test": { "id": "a", "purpose": "kiro-complete full test", "requested": 0, "granted": 0 },
  "sole": null,
  "requests": [ { "id": "b", "desks": "test,sole", "purpose": "...", "requested": 0 } ],
  "waits": [ { "id": "b", "kind": "request", "repo": null, "pid": 2, "since": 0 } ],
  "recent": [ { "at": 0, "kind": "test-freed", "id": "c", "detail": "stopped" } ]
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
    assert_eq!(state.participants["a"].status, ParticipantStatus::Working);
    assert_eq!(state.test.as_ref().map(|t| t.id.as_str()), Some("a"));
    assert_eq!(state.requests[0].desks, DeskSet::TestSole);
    assert_eq!(state.waits[0].kind, WaitKind::Request);
    assert_eq!(state.recent[0].kind, RecentKind::TestFreed);
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
        // 失敗なしで返す綴りは、状態ファイルの綴りと同じ（表が 2 つに割れたら赤）。
        assert_eq!(value.as_str(), text);
    }
    let waits = [
        (WaitKind::Watch, "watch"),
        (WaitKind::Merge, "merge"),
        (WaitKind::Request, "request"),
        (WaitKind::Resume, "resume"),
    ];
    for (value, text) in waits {
        assert_eq!(spelled(json!(value)), text);
        // ASCII の名前（居る印のファイル名・失敗の文）は、状態ファイルの綴りと同じ。
        assert_eq!(value.as_str(), text);
    }
    let recents = [
        (RecentKind::Reclaimed, "reclaimed"),
        (RecentKind::Recovered, "recovered"),
        (RecentKind::Cleared, "cleared"),
        (RecentKind::TestFreed, "test-freed"),
    ];
    for (value, text) in recents {
        assert_eq!(spelled(json!(value)), text);
        assert_eq!(value.as_str(), text);
    }
    let desks = [
        (DeskSet::Test, "test"),
        (DeskSet::Sole, "sole"),
        (DeskSet::TestSole, "test,sole"),
    ];
    for (value, text) in desks {
        assert_eq!(spelled(json!(value)), text);
        assert_eq!(value.as_str(), text);
    }
}

#[test]
fn empty_state_is_written_with_version_2() {
    assert_eq!(VERSION, 2);
    assert_eq!(
        json!(State::empty()),
        json!({
            "version": 2,
            "participants": {},
            "merge": {},
            "test": null,
            "sole": null,
            "requests": [],
            "waits": [],
            "recent": []
        })
    );
}

#[test]
fn missing_fields_of_the_same_version_still_read() {
    assert_eq!(read(r#"{ "version": 2 }"#), State::empty());

    // 項目の欠けた記録と、この実行ファイルの知らない項目（同じ版の中で後から増えたもの）。
    let state = read(
        r#"{ "version": 2, "added_later": true,
             "participants": { "A": { "id": "A" } },
             "merge": { "areka": {} },
             "test": { "id": "C" },
             "sole": { "id": "C" },
             "requests": [ { "desks": "sole" } ],
             "recent": [ { "kind": "cleared" } ] }"#,
    );
    let a = &state.participants["A"];
    assert_eq!(a.status, ParticipantStatus::Working);
    assert_eq!((a.watch.as_ref(), a.awaiting_watch_since), (None, None));
    assert_eq!(state.merge["areka"], Default::default());
    assert_eq!(state.test.as_ref().map(|t| t.granted), Some(0));
    let sole = state.sole.as_ref().expect("持ち主が居る");
    assert!(!sole.running && sole.stopped.is_empty());
    assert_eq!(state.requests[0].id, "");
    assert_eq!(state.recent[0].id, None);
}

#[test]
fn a_file_without_version_does_not_read_as_the_current_version() {
    assert_ne!(read("{}").version, VERSION);
}

#[test]
fn a_record_without_its_kind_does_not_read() {
    // 種類と机の組は欠けたときに入れる中立の値が無いので、形の合わないファイルとして断る。
    for text in [
        r#"{ "version": 2, "waits": [ { "id": "A" } ] }"#,
        r#"{ "version": 2, "recent": [ { "at": 1 } ] }"#,
        r#"{ "version": 2, "requests": [ { "id": "A" } ] }"#,
        r#"{ "version": 2, "requests": [ { "id": "A", "desks": "merge" } ] }"#,
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
