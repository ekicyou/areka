//! 待ちの終わりの判定の決定論テスト。状態を組み立てて渡すだけで、ファイルも時計も触らない。
//!
//! 組み立てる参加者の名前と負荷テストの内容は日本語（端末へ出てはならない値）。終わりの 1 行は
//! [`judged`] が毎回「ASCII だけ・1 行」を確かめる。

use super::test_support::{T0, load, merge, resume, watch};
use super::{WaitEnd, WaitSpec, judge};
use crate::state::{
    LastMerge, LoadHolder, LoadRequest, MergeDesk, MergeHolder, MergeRequest, Participant,
    ParticipantStatus, State, StopReason,
};

const WORKING: ParticipantStatus = ParticipantStatus::Working;
const STOP_REQUESTED: ParticipantStatus = ParticipantStatus::StopRequested;
const STOPPED: ParticipantStatus = ParticipantStatus::Stopped;

const ALREADY_STOPPED: &str = "already stopped; run stopped --wait or resume";

fn done(line: &str) -> Option<WaitEnd> {
    Some(WaitEnd::Done(line.to_owned()))
}

fn gone(why: &'static str) -> Option<WaitEnd> {
    Some(WaitEnd::Gone(why))
}

/// 状態ファイルが在るときの判定。終わりの 1 行と「消えた」の理由が ASCII だけの 1 行であることを
/// 毎回確かめる。
fn judged(spec: &WaitSpec, state: &State) -> Option<WaitEnd> {
    let end = judge(spec, Some(state));
    let text = match &end {
        Some(WaitEnd::Done(line)) => line.as_str(),
        Some(WaitEnd::Gone(why)) => why,
        None => "",
    };
    assert!(
        text.chars().all(|c| (' '..='~').contains(&c)),
        "not one printable ASCII line: {text:?}"
    );
    end
}

/// 名前が日本語の参加者だけを持つ状態。
fn state_with(people: &[(&str, ParticipantStatus)]) -> State {
    let mut state = State::empty();
    for (id, status) in people {
        state.participants.insert(
            (*id).to_owned(),
            Participant {
                id: (*id).to_owned(),
                name: format!("{id} の見張り役"),
                repo: "areka".to_owned(),
                status: *status,
                since: T0,
                ..Participant::default()
            },
        );
    }
    state
}

/// 停止要請の理由を付ける（内容は日本語）。
fn stop_reason(state: &mut State, id: &str, by: &str) {
    state.participants.get_mut(id).unwrap().stop_reason = Some(StopReason {
        by: by.to_owned(),
        purpose: "負荷の計測 5 回".to_owned(),
    });
}

fn merge_request(id: &str) -> MergeRequest {
    MergeRequest {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
        requested: T0 + 10,
    }
}

fn merge_holder(id: &str) -> Option<MergeHolder> {
    Some(MergeHolder {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
        requested: T0 + 10,
        granted: T0 + 11,
    })
}

fn last_merge() -> Option<LastMerge> {
    Some(LastMerge {
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
        spec: "impl-watch".to_owned(),
        at: 1_791_594_000,
    })
}

fn load_request(id: &str) -> LoadRequest {
    LoadRequest {
        id: id.to_owned(),
        purpose: "負荷の計測 5 回".to_owned(),
        requested: T0 + 20,
    }
}

fn load_holder(id: &str, stopped: &[&str]) -> Option<LoadHolder> {
    Some(LoadHolder {
        id: id.to_owned(),
        purpose: "負荷の計測 5 回".to_owned(),
        requested: T0 + 20,
        granted: T0 + 30,
        running: false,
        stopped: stopped.iter().map(|id| (*id).to_owned()).collect(),
    })
}

// ---- 見張り ----

#[test]
fn watch_keeps_waiting_while_working() {
    let mut state = state_with(&[("A", WORKING), ("B", STOP_REQUESTED)]);
    assert_eq!(judged(&watch("A"), &state), None);

    // 机を持っていても・並んでいても、見張りは終わらない（要件 5.3）。
    state.merge.entry("areka".to_owned()).or_default().holder = merge_holder("A");
    state.load.queue.push(load_request("A"));
    assert_eq!(judged(&watch("A"), &state), None);
    state.load.queue.clear();
    state.load.holder = load_holder("A", &["B"]);
    assert_eq!(judged(&watch("A"), &state), None);
}

#[test]
fn watch_ends_with_who_asked_when_a_stop_is_requested() {
    let mut state = state_with(&[("B", STOP_REQUESTED), ("C", WORKING), ("D", WORKING)]);
    // 停止要請の理由は出したときのまま（いまの候補が D に替わっていても C のまま）。
    stop_reason(&mut state, "B", "C");
    state.load.queue.push(load_request("D"));

    assert_eq!(judged(&watch("B"), &state), done("stop requested by C"));
}

#[test]
fn watch_ends_at_once_when_already_stopped() {
    let mut state = state_with(&[("B", STOPPED), ("C", WORKING)]);
    stop_reason(&mut state, "B", "C");

    assert_eq!(judged(&watch("B"), &state), done(ALREADY_STOPPED));
}

#[test]
fn watch_is_gone_when_the_participant_record_is_missing() {
    let state = state_with(&[("B", STOP_REQUESTED)]);

    assert_eq!(judged(&watch("A"), &state), gone("removed"));
}

/// 手で直した状態ファイル: 「停止要請中」なのに理由が無い・誰のものかが空。
/// 見張りは終わり（状態は「作業中」でない）、誰のものかは識別に使えない字 `?` で示す。
#[test]
fn watch_ends_with_a_question_mark_when_the_stop_reason_is_not_recorded() {
    let mut state = state_with(&[("B", STOP_REQUESTED)]);
    assert_eq!(judged(&watch("B"), &state), done("stop requested by ?"));

    stop_reason(&mut state, "B", "");
    assert_eq!(judged(&watch("B"), &state), done("stop requested by ?"));
}

// ---- マージの待ち ----

#[test]
fn merge_keeps_waiting_while_queued() {
    let mut state = state_with(&[("A", WORKING), ("B", WORKING)]);
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: merge_holder("B"),
            queue: vec![merge_request("A")],
            last: last_merge(),
        },
    );

    assert_eq!(judged(&merge("A", "areka"), &state), None);

    // 持ち主が居なくても、並んでいる間は番ではない（番を決めるのは判断の側）。
    state.merge.get_mut("areka").unwrap().holder = None;
    assert_eq!(judged(&merge("A", "areka"), &state), None);
}

#[test]
fn merge_is_granted_with_the_last_merge_on_the_line() {
    let mut state = state_with(&[("A", WORKING), ("B", WORKING)]);
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: merge_holder("A"),
            queue: vec![merge_request("B")],
            last: last_merge(),
        },
    );

    assert_eq!(
        judged(&merge("A", "areka"), &state),
        done("granted merge repo=areka; last: PR#281 414d43eb impl-watch 2026-10-10T01:00:00Z")
    );
}

#[test]
fn merge_is_granted_with_last_none_when_the_repo_has_no_merge_yet() {
    let mut state = state_with(&[("A", WORKING)]);
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: merge_holder("A"),
            ..MergeDesk::default()
        },
    );
    // 別のリポジトリの直前のマージは載せない。
    state.merge.insert(
        "pasta".to_owned(),
        MergeDesk {
            last: last_merge(),
            ..MergeDesk::default()
        },
    );

    assert_eq!(
        judged(&merge("A", "areka"), &state),
        done("granted merge repo=areka; last: none")
    );
}

#[test]
fn merge_is_gone_when_neither_queued_nor_holding_in_this_repo() {
    // 机そのものが無い。
    let mut state = state_with(&[("A", WORKING), ("B", WORKING), ("C", WORKING)]);
    assert_eq!(judged(&merge("A", "areka"), &state), gone("request gone"));

    // この机には他人だけが居る。
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: merge_holder("B"),
            queue: vec![merge_request("C")],
            last: last_merge(),
        },
    );
    assert_eq!(judged(&merge("A", "areka"), &state), gone("request gone"));

    // 別のリポジトリに並んでいる・持っているのは、このリポジトリの待ちには数えない。
    state.merge.insert(
        "pasta".to_owned(),
        MergeDesk {
            queue: vec![merge_request("A")],
            ..MergeDesk::default()
        },
    );
    assert_eq!(judged(&merge("A", "areka"), &state), gone("request gone"));
    state.merge.insert(
        "pasta".to_owned(),
        MergeDesk {
            holder: merge_holder("A"),
            ..MergeDesk::default()
        },
    );
    assert_eq!(judged(&merge("A", "areka"), &state), gone("request gone"));
    assert_eq!(
        judged(&merge("A", "pasta"), &state),
        done("granted merge repo=pasta; last: none")
    );
}

/// 参加者の記録が無ければ、机に名前が残っていても「消えた」（手で直した状態ファイル）。
#[test]
fn merge_is_gone_when_the_participant_record_is_missing() {
    let mut state = state_with(&[("B", WORKING)]);
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            queue: vec![merge_request("A")],
            ..MergeDesk::default()
        },
    );
    assert_eq!(judged(&merge("A", "areka"), &state), gone("request gone"));

    state.merge.get_mut("areka").unwrap().holder = merge_holder("A");
    assert_eq!(judged(&merge("A", "areka"), &state), gone("request gone"));
}

// ---- 負荷テストの待ち ----

#[test]
fn load_keeps_waiting_while_queued() {
    let mut state = state_with(&[
        ("A", STOPPED),
        ("B", STOPPED),
        ("C", WORKING),
        ("D", WORKING),
    ]);
    state.load.holder = load_holder("D", &["A", "B"]);
    state.load.queue.push(load_request("C"));

    assert_eq!(judged(&load("C"), &state), None);

    state.load.holder = None;
    assert_eq!(judged(&load("C"), &state), None);
}

#[test]
fn load_is_granted_with_the_stopped_ids_on_the_line() {
    let mut state = state_with(&[
        ("A", STOPPED),
        ("B", STOPPED),
        ("C", WORKING),
        ("D", WORKING),
    ]);
    state.load.holder = load_holder("C", &["A", "B"]);
    state.load.queue.push(load_request("D"));

    assert_eq!(
        judged(&load("C"), &state),
        done("granted load; stopped: A, B")
    );
}

#[test]
fn load_is_granted_with_stopped_none_when_nobody_had_to_stop() {
    let mut state = state_with(&[("C", WORKING)]);
    state.load.holder = load_holder("C", &[]);

    assert_eq!(
        judged(&load("C"), &state),
        done("granted load; stopped: none")
    );
}

#[test]
fn load_is_gone_when_neither_queued_nor_holding() {
    let mut state = state_with(&[("A", WORKING), ("C", WORKING), ("D", WORKING)]);
    assert_eq!(judged(&load("C"), &state), gone("request gone"));

    // 机には他人だけが居る。
    state.load.holder = load_holder("D", &["C"]);
    state.load.queue.push(load_request("A"));
    assert_eq!(judged(&load("C"), &state), gone("request gone"));
}

#[test]
fn load_is_gone_when_the_participant_record_is_missing() {
    let mut state = state_with(&[("A", WORKING)]);
    state.load.queue.push(load_request("C"));
    assert_eq!(judged(&load("C"), &state), gone("request gone"));

    state.load.queue.clear();
    state.load.holder = load_holder("C", &["A"]);
    assert_eq!(judged(&load("C"), &state), gone("request gone"));
}

// ---- 再開の待ち ----

#[test]
fn resume_keeps_waiting_while_stopped_or_stop_requested() {
    let mut state = state_with(&[("A", STOPPED), ("B", STOP_REQUESTED)]);
    stop_reason(&mut state, "A", "C");
    stop_reason(&mut state, "B", "C");

    assert_eq!(judged(&resume("A"), &state), None);
    assert_eq!(judged(&resume("B"), &state), None);
}

#[test]
fn resume_ends_when_back_to_work() {
    let state = state_with(&[("A", WORKING), ("B", STOPPED)]);

    assert_eq!(judged(&resume("A"), &state), done("resumed"));
}

#[test]
fn resume_is_gone_when_the_participant_record_is_missing() {
    let state = state_with(&[("B", STOPPED)]);

    assert_eq!(judged(&resume("A"), &state), gone("removed"));
}

// ---- 状態ファイルが無い・値が ASCII でない ----

/// 状態ファイルが無い（`clear` や退避の改名の一瞬を含む）は、4 種とも「消えた」。
#[test]
fn every_kind_is_gone_when_there_is_no_state_file() {
    assert_eq!(judge(&watch("A"), None), gone("removed"));
    assert_eq!(judge(&merge("A", "areka"), None), gone("request gone"));
    assert_eq!(judge(&load("A"), None), gone("request gone"));
    assert_eq!(judge(&resume("A"), None), gone("removed"));
}

/// 手で直した状態ファイルから ASCII の外の識別・リポジトリ・spec が来ても、行は ASCII のまま。
#[test]
fn lines_stay_ascii_when_hand_edited_values_are_not_ascii() {
    let mut state = state_with(&[("A", WORKING), ("B", STOP_REQUESTED)]);
    stop_reason(&mut state, "B", "係");
    assert_eq!(
        judged(&watch("B"), &state),
        done("stop requested by \\u{4fc2}")
    );

    let mut last = last_merge();
    last.as_mut().unwrap().spec = "係".to_owned();
    state.merge.insert(
        "倉".to_owned(),
        MergeDesk {
            holder: merge_holder("A"),
            queue: Vec::new(),
            last,
        },
    );
    assert_eq!(
        judged(&merge("A", "倉"), &state),
        done("granted merge repo=\\u{5009}; last: PR#281 414d43eb \\u{4fc2} 2026-10-10T01:00:00Z")
    );

    state.load.holder = load_holder("A", &["係", "B"]);
    assert_eq!(
        judged(&load("A"), &state),
        done("granted load; stopped: \\u{4fc2}, B")
    );
}
