//! 机と待ち行列の判断（規則 1・3・4・6）の決定論テスト。時刻と生死は引数で渡し、眠らない。
//!
//! 組み立てる状態は、どれも「番を決め直しても動かない」姿にしてある（持ち主の居ない机に
//! 待ち行列を残さない・負荷テストの机は走っている印つき）。主題でない段が状態を動かして、
//! 判定が主題から外れないようにするため。

use super::test_support::{
    FakePresence, PID, REPO, T0, ask_to_stop, hold_merge, hold_running_load, queue_load,
    queue_merge, state_with, wait_record,
};
use super::{Applied, Command, Event, Verdict, apply};
use crate::state::{
    LoadDesk, MergeDesk, Participant, ParticipantStatus, State, WaitKind, WaitRecord, WatchInfo,
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
