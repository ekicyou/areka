//! 机と待ち行列の判断（規則 1・3・4・6）の決定論テスト。時刻と生死は引数で渡し、眠らない。
//!
//! 組み立てる状態は、番の到来を主題にするテストのほかは、どれも「番を決め直しても動かない」姿に
//! してある（持ち主の居ない机に待ち行列を残さない・負荷テストの机は走っている印つき）。
//! 主題でない段が状態を動かして、判定が主題から外れないようにするため。

use super::test_support::{
    FakePresence, PID, REPO, T0, ask_to_stop, hold_merge, hold_running_load, queue_load,
    queue_merge, state_with, wait_record,
};
use super::{Applied, Command, Event, Verdict, apply};
use crate::state::{
    LastMerge, LoadDesk, MergeDesk, MergeHolder, MergeRequest, Participant, ParticipantStatus,
    State, WaitKind, WaitRecord, WatchInfo,
};

fn watch(id: &str, name: Option<&str>, repo: &str, pid: u32) -> Command {
    Command::Watch {
        id: id.to_owned(),
        name: name.map(str::to_owned),
        repo: repo.to_owned(),
        pid,
    }
}

fn leave(id: &str) -> Command {
    Command::Leave { id: id.to_owned() }
}

fn cancel(id: &str) -> Command {
    Command::Cancel { id: id.to_owned() }
}

/// 名前を省いた、spec が `spec-<識別>` の申し込み（支えの組み立てる申し込みと同じ綴り）。
fn merge(id: &str, repo: &str, bug: bool) -> Command {
    Command::Merge {
        id: id.to_owned(),
        name: None,
        repo: repo.to_owned(),
        spec: format!("spec-{id}"),
        bug,
    }
}

fn merged(id: &str) -> Command {
    Command::Merged {
        id: id.to_owned(),
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
    }
}

fn joined(id: &str) -> Event {
    Event::Joined { id: id.to_owned() }
}

fn merge_requested(id: &str, repo: &str, bug: bool) -> Event {
    Event::MergeRequested {
        repo: repo.to_owned(),
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug,
    }
}

fn merge_granted(id: &str, repo: &str) -> Event {
    Event::MergeGranted {
        repo: repo.to_owned(),
        id: id.to_owned(),
    }
}

/// [`merged`] の出来事。
fn merge_done(id: &str, repo: &str) -> Event {
    Event::Merged {
        repo: repo.to_owned(),
        id: id.to_owned(),
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
    }
}

fn left_merged(id: &str) -> Event {
    Event::Left {
        id: id.to_owned(),
        why: "merged",
    }
}

/// マージの待ち行列の末尾へ、バグの申し込みを足す。
fn queue_bug_merge(state: &mut State, repo: &str, id: &str, requested: u64) {
    queue_merge(state, repo, id, requested);
    let desk = state.merge.get_mut(repo).expect("机が在る");
    desk.queue.last_mut().expect("いま足した").bug = true;
}

/// マージの机の持ち主（リポジトリ, 識別）の一覧。
fn merge_holders(state: &State) -> Vec<(&str, &str)> {
    state
        .merge
        .iter()
        .filter_map(|(repo, desk)| Some((repo.as_str(), desk.holder.as_ref()?.id.as_str())))
        .collect()
}

/// 全員が居る机で、判断を 1 回呼ぶ。
fn run(state: &mut State, cmd: &Command, caller: Option<&str>, now: u64) -> Applied {
    apply(state, cmd, caller, now, &FakePresence::all_present())
}

fn applied(changed: bool, events: Vec<Event>) -> Applied {
    Applied {
        changed,
        verdict: Verdict::Applied,
        events,
    }
}

/// C がマージの机（pasta）と、すでに走っている負荷テストの机を持ち、`waiters` が申し込みの順に
/// その両方を待つ。`ids` は参加者。
fn desks_held_by_c(ids: &[&str], waiters: &[&str]) -> State {
    let mut state = state_with(ids);
    hold_merge(&mut state, "pasta", "C");
    hold_running_load(&mut state, "C");
    for (id, requested) in waiters.iter().zip(T0 + 1..) {
        queue_merge(&mut state, "pasta", id, requested);
        queue_load(&mut state, id, requested);
    }
    state
}

/// A が areka のマージの机を持ち、C の持つ 2 つの机を B の後ろで待ち、マージの待ちを走らせている。
fn a_holds_and_waits() -> State {
    let mut state = desks_held_by_c(&["A", "B", "C"], &["B", "A"]);
    hold_merge(&mut state, "areka", "A");
    state.waits.push(wait_record("A", WaitKind::Merge));
    state
}

#[test]
fn watch_start_registers_a_working_participant_with_its_watch_record() {
    let mut state = State::empty();

    let got = run(
        &mut state,
        &watch("A", Some("えー"), "areka", 4321),
        Some("A"),
        T0,
    );

    let mut expected = State::empty();
    expected.participants.insert(
        "A".to_owned(),
        Participant {
            id: "A".to_owned(),
            name: "えー".to_owned(),
            repo: "areka".to_owned(),
            status: ParticipantStatus::Working,
            since: T0,
            stop_reason: None,
            watch: Some(WatchInfo {
                pid: 4321,
                since: T0,
            }),
            awaiting_watch_since: None,
        },
    );
    expected.waits.push(WaitRecord {
        id: "A".to_owned(),
        kind: WaitKind::Watch,
        repo: Some("areka".to_owned()),
        pid: 4321,
        since: T0,
    });
    assert_eq!(state, expected);
    assert_eq!(
        got,
        applied(true, vec![Event::Joined { id: "A".to_owned() }])
    );
}

#[test]
fn watch_start_twice_keeps_one_record_and_updates_name_and_repo() {
    let mut state = State::empty();
    run(
        &mut state,
        &watch("A", Some("old"), "areka", 1),
        Some("A"),
        T0,
    );

    let got = run(
        &mut state,
        &watch("A", Some("new"), "pasta", 2),
        Some("A"),
        T0 + 10,
    );

    assert_eq!(state.participants.len(), 1);
    let a = &state.participants["A"];
    assert_eq!((a.name.as_str(), a.repo.as_str()), ("new", "pasta"));
    // 参加の時刻は初めのまま。見張りの記録と待ちの記録は、新しい見張りのものへ置き換わる。
    assert_eq!(a.since, T0);
    assert_eq!(
        a.watch,
        Some(WatchInfo {
            pid: 2,
            since: T0 + 10
        })
    );
    assert_eq!(
        state.waits,
        vec![WaitRecord {
            id: "A".to_owned(),
            kind: WaitKind::Watch,
            repo: Some("pasta".to_owned()),
            pid: 2,
            since: T0 + 10,
        }]
    );
    // 2 度目は参加ではないので、参加の出来事は出ない。
    assert_eq!(got, applied(true, vec![]));
}

#[test]
fn an_omitted_name_is_the_id_at_first_and_keeps_the_old_name_later() {
    let mut state = State::empty();
    let name = |state: &State| state.participants["A"].name.clone();

    run(&mut state, &watch("A", None, REPO, PID), Some("A"), T0);
    assert_eq!(name(&state), "A");

    run(
        &mut state,
        &watch("A", Some("えー"), REPO, PID),
        Some("A"),
        T0,
    );
    run(&mut state, &watch("A", None, REPO, PID), Some("A"), T0);
    assert_eq!(name(&state), "えー");
}

#[test]
fn repeating_the_same_watch_start_changes_nothing() {
    let mut state = state_with(&["A", "B"]);
    let cmd = watch("A", Some("new"), REPO, 9);
    assert!(run(&mut state, &cmd, Some("A"), T0 + 10).changed);
    let before = state.clone();

    let got = run(&mut state, &cmd, Some("A"), T0 + 10);

    // 待ちの記録はその場で置き換わり、並びも変わらない。
    assert_eq!(state, before);
    assert_eq!(state.waits[0].id, "A");
    assert_eq!(got, applied(false, vec![]));
}

#[test]
fn watch_start_keeps_the_status_and_clears_the_awaiting_mark() {
    for status in [ParticipantStatus::StopRequested, ParticipantStatus::Stopped] {
        // C の負荷テストが待たれていて、A と B に停止要請が出ている（B がまだ止まっていないので
        // 番は来ない）。A は再開の後の「見張りを立て直すまでの印」を付けたまま。
        let mut state = state_with(&["A", "B", "C"]);
        queue_load(&mut state, "C", T0);
        ask_to_stop(&mut state, "A", "C");
        ask_to_stop(&mut state, "B", "C");
        let a = state.participants.get_mut("A").expect("A が居る");
        a.status = status;
        a.awaiting_watch_since = Some(T0 + 1);
        let mut expected = state.clone();

        let got = run(&mut state, &watch("A", None, REPO, 77), Some("A"), T0 + 10);

        // 状態と停止要請の理由はそのまま（立て直した見張りが直ちに終わる根拠）。
        let a = expected.participants.get_mut("A").expect("A が居る");
        a.awaiting_watch_since = None;
        a.watch = Some(WatchInfo {
            pid: 77,
            since: T0 + 10,
        });
        expected.waits[0] = WaitRecord {
            pid: 77,
            since: T0 + 10,
            ..wait_record("A", WaitKind::Watch)
        };
        assert_eq!(state, expected, "{status:?}");
        assert_eq!(got, applied(true, vec![]), "{status:?}");
    }
}

#[test]
fn leave_removes_requests_desks_wait_records_and_the_participant() {
    // 自分で呼んでも、他の参加者や開発者が代わりに呼んでも同じに働く。
    for caller in [Some("A"), Some("B"), None] {
        let mut state = a_holds_and_waits();

        let got = run(&mut state, &leave("A"), caller, T0 + 10);

        let mut expected = desks_held_by_c(&["B", "C"], &["B"]);
        expected
            .merge
            .insert("areka".to_owned(), MergeDesk::default());
        assert_eq!(state, expected, "{caller:?}");
        let left = Event::Left {
            id: "A".to_owned(),
            why: "leave",
        };
        assert_eq!(got, applied(true, vec![left]), "{caller:?}");
    }
}

#[test]
fn cancel_removes_requests_and_desks_but_keeps_the_participant() {
    for caller in [Some("A"), Some("B"), None] {
        let mut state = a_holds_and_waits();

        let got = run(&mut state, &cancel("A"), caller, T0 + 10);

        // 参加者の記録と見張りは残る。走っているマージの待ちの記録も残り、待ちの側が
        // 「申し込みが消えた」と読んで自分で消す。
        let mut expected = desks_held_by_c(&["A", "B", "C"], &["B"]);
        expected
            .merge
            .insert("areka".to_owned(), MergeDesk::default());
        expected.waits.push(wait_record("A", WaitKind::Merge));
        assert_eq!(state, expected, "{caller:?}");
        let cancelled = Event::Cancelled { id: "A".to_owned() };
        assert_eq!(got, applied(true, vec![cancelled]), "{caller:?}");
    }
}

#[test]
fn leave_and_cancel_release_the_load_desk() {
    for cmd in [leave("A"), cancel("A")] {
        let mut state = state_with(&["A", "B"]);
        hold_running_load(&mut state, "A");

        let got = run(&mut state, &cmd, Some("A"), T0 + 10);

        assert_eq!(state.load, LoadDesk::default(), "{cmd:?}");
        assert!(got.changed, "{cmd:?}");
    }
}

#[test]
fn leave_and_cancel_of_an_unknown_identity_change_nothing() {
    for cmd in [leave("Z"), cancel("Z")] {
        let mut state = a_holds_and_waits();
        let before = state.clone();

        let got = run(&mut state, &cmd, Some("Z"), T0 + 10);

        // 参加していない識別の記録を作らない。外すものが無ければ出来事も無い。
        assert_eq!(state, before, "{cmd:?}");
        assert_eq!(got, applied(false, vec![]), "{cmd:?}");
    }
}

#[test]
fn a_merge_request_joins_and_waits_behind_the_holder() {
    // C が areka の机を持っている。参加していない A が、バグの申し込みをする。
    let mut state = state_with(&["C"]);
    hold_merge(&mut state, "areka", "C");
    let mut expected = state.clone();

    let cmd = Command::Merge {
        id: "A".to_owned(),
        name: Some("えー".to_owned()),
        repo: "areka".to_owned(),
        spec: "spec-A".to_owned(),
        bug: true,
    };
    let got = run(&mut state, &cmd, Some("A"), T0 + 5);

    // 申し込みは参加を兼ねる（見張りはまだ無い）。持ち主が居る間は、バグの申し込みでも
    // 持ち主にならず、申し込みの時刻とともに待ち行列に並ぶ。
    expected.participants.insert(
        "A".to_owned(),
        Participant {
            id: "A".to_owned(),
            name: "えー".to_owned(),
            repo: "areka".to_owned(),
            since: T0 + 5,
            ..Participant::default()
        },
    );
    let desk = expected.merge.get_mut("areka").expect("机が在る");
    desk.queue.push(MergeRequest {
        id: "A".to_owned(),
        spec: "spec-A".to_owned(),
        bug: true,
        requested: T0 + 5,
    });
    assert_eq!(state, expected);
    let events = vec![joined("A"), merge_requested("A", "areka", true)];
    assert_eq!(got, applied(true, events));
}

#[test]
fn a_repeated_merge_request_keeps_the_first_one() {
    // A は areka の机を持ち、pasta の机を C の後ろで待っている。どちらへ申し込み直しても
    // （バグの印と時刻を変えても）元の申し込みのまま。
    for repo in ["areka", "pasta"] {
        let mut state = state_with(&["A", "C"]);
        hold_merge(&mut state, "areka", "A");
        hold_merge(&mut state, "pasta", "C");
        queue_merge(&mut state, "pasta", "A", T0 + 1);
        state.participants.get_mut("A").expect("A が居る").repo = repo.to_owned();
        let before = state.clone();

        let got = run(&mut state, &merge("A", repo, true), Some("A"), T0 + 9);

        assert_eq!(state, before, "{repo}");
        assert_eq!(got, applied(false, vec![]), "{repo}");
    }
}

#[test]
fn the_next_holder_is_a_bug_request_first_then_the_earlier_request() {
    // 待ち行列の並びは、バグの順からも時刻の順からもわざと外してある。
    let mut state = state_with(&["A", "B", "C", "D", "E"]);
    hold_merge(&mut state, "areka", "C");
    queue_merge(&mut state, "areka", "A", T0 + 2);
    queue_bug_merge(&mut state, "areka", "B", T0 + 4);
    queue_bug_merge(&mut state, "areka", "D", T0 + 3);
    queue_merge(&mut state, "areka", "E", T0 + 1);

    // 持ち主が机を手放すたびに、次の番が同じ呼び出しで出る。
    let mut order = Vec::new();
    for now in T0 + 10..T0 + 14 {
        let holder = merge_holders(&state)[0].1.to_owned();
        run(&mut state, &cancel(&holder), None, now);
        order.push(merge_holders(&state)[0].1.to_owned());
    }

    assert_eq!(order, ["D", "B", "E", "A"]);
    // 番の時刻は「いま」。申し込みの中身はそのまま持ち主へ写る。
    let desk = MergeDesk {
        holder: Some(MergeHolder {
            id: "A".to_owned(),
            spec: "spec-A".to_owned(),
            bug: false,
            requested: T0 + 2,
            granted: T0 + 13,
        }),
        ..MergeDesk::default()
    };
    assert_eq!(state.merge["areka"], desk);
}

#[test]
fn two_repos_are_held_at_the_same_time() {
    let mut state = State::empty();
    run(&mut state, &merge("A", "areka", false), Some("A"), T0);

    let got = run(&mut state, &merge("B", "pasta", false), Some("B"), T0 + 1);

    // areka に持ち主が居ても、pasta の番は同じ呼び出しで出る。
    assert_eq!(merge_holders(&state), [("areka", "A"), ("pasta", "B")]);
    let events = vec![
        joined("B"),
        merge_requested("B", "pasta", false),
        merge_granted("B", "pasta"),
    ];
    assert_eq!(got, applied(true, events));

    // 別のリポジトリの机を持っていても申し込める: areka を持ったまま、pasta の待ち行列に並ぶ。
    run(&mut state, &merge("A", "pasta", false), Some("A"), T0 + 2);
    assert_eq!(merge_holders(&state), [("areka", "A"), ("pasta", "B")]);
    assert_eq!(state.merge["pasta"].queue.len(), 1);
    assert_eq!(state.merge["pasta"].queue[0].id, "A");
}

#[test]
fn waiting_in_one_repo_does_not_block_a_request_for_another() {
    // 二重の判定はリポジトリごと。C が areka の机を持ち、A はその待ち行列で待っている。
    let mut state = state_with(&["A", "C"]);
    hold_merge(&mut state, "areka", "C");
    queue_merge(&mut state, "areka", "A", T0 + 1);
    let areka = state.merge["areka"].clone();

    let got = run(&mut state, &merge("A", "pasta", false), Some("A"), T0 + 5);

    // pasta は別の机なので二重に当たらず、空いているから同じ呼び出しで A の番になる。
    // areka の待ち行列の申し込みはそのまま。
    assert_eq!(merge_holders(&state), [("areka", "C"), ("pasta", "A")]);
    assert_eq!(state.merge["areka"], areka);
    let events = vec![
        merge_requested("A", "pasta", false),
        merge_granted("A", "pasta"),
    ];
    assert_eq!(got, applied(true, events));
}

#[test]
fn every_freed_repo_gets_its_next_holder_in_the_same_call() {
    // A が areka と pasta の机を両方持ち、それぞれに 1 人ずつ待っている。
    for (cmd, gone) in [
        (
            leave("A"),
            Event::Left {
                id: "A".to_owned(),
                why: "leave",
            },
        ),
        (cancel("A"), Event::Cancelled { id: "A".to_owned() }),
    ] {
        let mut state = state_with(&["A", "B", "C"]);
        hold_merge(&mut state, "areka", "A");
        queue_merge(&mut state, "areka", "B", T0 + 1);
        hold_merge(&mut state, "pasta", "A");
        queue_merge(&mut state, "pasta", "C", T0 + 2);

        let got = run(&mut state, &cmd, Some("A"), T0 + 10);

        // 空いた 2 つの机の番が、1 回の呼び出しで両方出る。
        assert_eq!(
            merge_holders(&state),
            [("areka", "B"), ("pasta", "C")],
            "{cmd:?}"
        );
        let events = vec![
            gone,
            merge_granted("B", "areka"),
            merge_granted("C", "pasta"),
        ];
        assert_eq!(got, applied(true, events), "{cmd:?}");
    }
}

#[test]
fn merged_frees_the_desk_records_the_last_merge_and_ends_the_participation() {
    // A が areka の机を持ち、B がその後ろで待つ。A は C の持つ pasta の机にも並び、
    // マージの待ちを走らせている。
    let mut state = state_with(&["A", "B", "C"]);
    hold_merge(&mut state, "areka", "A");
    queue_merge(&mut state, "areka", "B", T0 + 1);
    hold_merge(&mut state, "pasta", "C");
    queue_merge(&mut state, "pasta", "A", T0 + 2);
    state.waits.push(wait_record("A", WaitKind::Merge));

    let got = run(&mut state, &merged("A"), Some("A"), T0 + 10);

    // 参加者の記録も、見張りと待ちの記録も、ほかの机の申し込みも消える。
    // 空いた机の次の番（B）は同じ呼び出しで出る。
    let mut expected = state_with(&["B", "C"]);
    hold_merge(&mut expected, "pasta", "C");
    expected.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: Some(MergeHolder {
                id: "B".to_owned(),
                spec: "spec-B".to_owned(),
                bug: false,
                requested: T0 + 1,
                granted: T0 + 10,
            }),
            queue: Vec::new(),
            last: Some(LastMerge {
                pr: "281".to_owned(),
                sha: "414d43eb".to_owned(),
                spec: "spec-A".to_owned(),
                at: T0 + 10,
            }),
        },
    );
    assert_eq!(state, expected);
    let events = vec![
        merge_done("A", "areka"),
        left_merged("A"),
        merge_granted("B", "areka"),
    ];
    assert_eq!(got, applied(true, events));
}

#[test]
fn merged_by_a_non_holder_changes_nothing() {
    // B は待っているだけ、Z は参加していない。
    for id in ["B", "Z"] {
        let mut state = state_with(&["A", "B"]);
        hold_merge(&mut state, "areka", "A");
        queue_merge(&mut state, "areka", "B", T0 + 1);
        let before = state.clone();

        let got = run(&mut state, &merged(id), Some(id), T0 + 10);

        assert_eq!(state, before, "{id}");
        assert!(!got.changed, "{id}");
        assert!(got.events.is_empty(), "{id}: {:?}", got.events);
        // 断りの文は端末へそのまま出すので ASCII。
        let Verdict::NotApplied(text) = got.verdict else {
            panic!("{id}: 当てはまらなかった、を返す: {:?}", got.verdict);
        };
        assert!(!text.is_empty() && text.is_ascii(), "{id}: {text}");
    }
}

#[test]
fn a_merge_round_trip_reports_one_event_per_change() {
    let mut state = State::empty();

    // 空いている机なら、申し込みと同じ呼び出しで番が来る。
    let got = run(&mut state, &merge("A", "areka", false), Some("A"), T0);
    let events = vec![
        joined("A"),
        merge_requested("A", "areka", false),
        merge_granted("A", "areka"),
    ];
    assert_eq!(got, applied(true, events));

    let got = run(&mut state, &merge("B", "areka", true), Some("B"), T0 + 1);
    let events = vec![joined("B"), merge_requested("B", "areka", true)];
    assert_eq!(got, applied(true, events));

    let got = run(&mut state, &merged("A"), Some("A"), T0 + 2);
    let events = vec![
        merge_done("A", "areka"),
        left_merged("A"),
        merge_granted("B", "areka"),
    ];
    assert_eq!(got, applied(true, events));

    let got = run(&mut state, &merged("B"), Some("B"), T0 + 3);
    let events = vec![merge_done("B", "areka"), left_merged("B")];
    assert_eq!(got, applied(true, events));
    assert!(state.participants.is_empty());
}
