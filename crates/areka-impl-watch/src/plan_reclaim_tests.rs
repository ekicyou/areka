//! 回収・待ちの記録・周期の一回りの決定論テスト。時刻と生死は引数で渡し、眠らない。
//!
//! 生死は偽の口（[`FakePresence`]）で、「居ない」は名指ししたものだけ。見張りの印は
//! `WaitKind::Watch` の答えだけで、ほかの種類の答えは待ちの記録の回収にだけ使う。

use super::test_support::{
    FakePresence, PID, PURPOSE, REPO, T0, applied, ask_to_stop, hold_load, hold_merge,
    hold_running_load, queue_load, queue_merge, run, state_with, stop_for, wait_record,
};
use super::{Applied, Command, Event, apply};
use crate::state::{
    LoadDesk, LoadHolder, MergeDesk, MergeHolder, Recent, RecentKind, State, WaitKind, WaitRecord,
};

/// 周期の一回りを 1 回呼ぶ。
fn tick(state: &mut State, caller: Option<&str>, now: u64, presence: &FakePresence) -> Applied {
    apply(state, &Command::Tick, caller, now, presence)
}

/// その識別の見張りだけが居ない机。
fn no_watch(ids: &[&str]) -> FakePresence {
    ids.iter()
        .fold(FakePresence::all_present(), |presence, id| {
            presence.absent(id, WaitKind::Watch)
        })
}

fn register(id: &str, kind: WaitKind, pid: u32, since: u64) -> Command {
    Command::RegisterWait {
        record: WaitRecord {
            pid,
            since,
            ..wait_record(id, kind)
        },
    }
}

fn unregister(id: &str, kind: WaitKind) -> Command {
    Command::UnregisterWait {
        id: id.to_owned(),
        kind,
    }
}

fn reclaimed(id: &str) -> Event {
    Event::Reclaimed {
        id: id.to_owned(),
        why: "watch absent",
    }
}

/// 回収の「最近の出来事」1 件。
fn reclaim_record(id: &str, at: u64) -> Recent {
    Recent {
        at,
        kind: RecentKind::Reclaimed,
        id: Some(id.to_owned()),
        detail: "watch absent".to_owned(),
    }
}

fn wait_registered(id: &str, kind: WaitKind) -> Event {
    Event::WaitRegistered {
        id: id.to_owned(),
        kind,
    }
}

fn wait_removed(id: &str, kind: WaitKind, why: &'static str) -> Event {
    Event::WaitRemoved {
        id: id.to_owned(),
        kind,
        why,
    }
}

/// `requested` に申し込み、`granted` に番の来たマージの持ち主（支えの申し込みと同じ綴り）。
fn merge_holder(id: &str, requested: u64, granted: u64) -> Option<MergeHolder> {
    Some(MergeHolder {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
        requested,
        granted,
    })
}

/// その識別の待ちの記録の種類の一覧（記録の並びの順）。
fn kinds(state: &State, id: &str) -> Vec<WaitKind> {
    let of_id = state.waits.iter().filter(|wait| wait.id == id);
    of_id.map(|wait| wait.kind).collect()
}

fn ids(state: &State) -> Vec<&str> {
    state.participants.keys().map(String::as_str).collect()
}

#[test]
fn a_merge_holder_without_a_watch_is_reclaimed_and_the_next_holder_comes_in_the_same_call() {
    // A は areka の机を持ち、C の持つ pasta の机を待ち、その待ちを走らせている。B は areka を待つ。
    // A の見張りだけが居ない（マージの待ちのプロセスは生きている＝待ちは居る印にならない）。
    let mut state = state_with(&["A", "B", "C"]);
    hold_merge(&mut state, "areka", "A");
    queue_merge(&mut state, "areka", "B", T0 + 1);
    hold_merge(&mut state, "pasta", "C");
    queue_merge(&mut state, "pasta", "A", T0 + 2);
    state.waits.push(wait_record("A", WaitKind::Merge));

    let got = tick(&mut state, Some("B"), T0 + 10, &no_watch(&["A"]));

    // A の机・申し込み・待ちの記録（見張りもマージの待ちも）・参加者の記録が消え、回収が
    // 「最近の出来事」に識別・理由・時刻つきで残る。空いた机の次の番は同じ呼び出しで B に来る。
    let mut expected = state_with(&["B", "C"]);
    expected.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: merge_holder("B", T0 + 1, T0 + 10),
            ..MergeDesk::default()
        },
    );
    hold_merge(&mut expected, "pasta", "C");
    expected.recent = vec![reclaim_record("A", T0 + 10)];
    assert_eq!(state, expected);
    let granted = Event::MergeGranted {
        repo: "areka".to_owned(),
        id: "B".to_owned(),
    };
    assert_eq!(got, applied(true, vec![reclaimed("A"), granted]));
}

#[test]
fn a_load_holder_without_a_watch_is_reclaimed_and_the_others_go_on_in_the_same_call() {
    // A が負荷テストの机を持ち、B はそのために止まっている。C はマージを待っている。
    let mut state = state_with(&["A", "B", "C"]);
    hold_load(&mut state, "A", &["B"]);
    stop_for(&mut state, "B", "A");
    queue_merge(&mut state, REPO, "C", T0 + 1);

    let got = tick(&mut state, Some("C"), T0 + 10, &no_watch(&["A"]));

    // 負荷テストが無くなったので、同じ呼び出しで B が再開し、C にマージの番が来る。
    let mut expected = state_with(&["B", "C"]);
    let b = expected.participants.get_mut("B").expect("B が居る");
    b.since = T0 + 10;
    b.awaiting_watch_since = Some(T0 + 10);
    expected.merge.insert(
        REPO.to_owned(),
        MergeDesk {
            holder: merge_holder("C", T0 + 1, T0 + 10),
            ..MergeDesk::default()
        },
    );
    expected.recent = vec![reclaim_record("A", T0 + 10)];
    assert_eq!(state, expected);
    let resumed = Event::Resumed {
        id: "B".to_owned(),
        why: "no-load",
    };
    let granted = Event::MergeGranted {
        repo: REPO.to_owned(),
        id: "C".to_owned(),
    };
    assert_eq!(got, applied(true, vec![reclaimed("A"), resumed, granted]));
}

#[test]
fn reclaiming_the_merge_holder_lets_the_waiting_load_test_take_its_desk_in_the_same_call() {
    // C の負荷テストは、B が止まっていても、A がマージの机を持っている間は番が来ない。
    let mut state = state_with(&["A", "B", "C"]);
    hold_merge(&mut state, REPO, "A");
    queue_load(&mut state, "C", T0 + 1);
    stop_for(&mut state, "B", "C");

    let got = tick(&mut state, Some("C"), T0 + 10, &no_watch(&["A"]));

    assert_eq!(ids(&state), ["B", "C"]);
    assert_eq!(
        state.load,
        LoadDesk {
            holder: Some(LoadHolder {
                id: "C".to_owned(),
                purpose: PURPOSE.to_owned(),
                requested: T0 + 1,
                granted: T0 + 10,
                running: false,
                stopped: vec!["B".to_owned()],
            }),
            queue: vec![],
        }
    );
    let granted = Event::LoadGranted {
        id: "C".to_owned(),
        stopped: vec!["B".to_owned()],
    };
    assert_eq!(got, applied(true, vec![reclaimed("A"), granted]));
}

#[test]
fn an_ordinary_command_reclaims_first_then_works_then_replans() {
    // 見張りの居ない A が areka の机を持っている。B が申し込む。
    let mut state = state_with(&["A", "B"]);
    hold_merge(&mut state, REPO, "A");
    let cmd = Command::Merge {
        id: "B".to_owned(),
        name: None,
        repo: REPO.to_owned(),
        spec: "spec-B".to_owned(),
        bug: false,
    };

    let got = apply(&mut state, &cmd, Some("B"), T0 + 10, &no_watch(&["A"]));

    // 回収 → 申し込み → 番の決め直しの順なので、B は並んだ同じ呼び出しで持ち主になる。
    assert_eq!(ids(&state), ["B"]);
    assert_eq!(
        state.merge[REPO].holder,
        merge_holder("B", T0 + 10, T0 + 10)
    );
    let requested = Event::MergeRequested {
        repo: REPO.to_owned(),
        id: "B".to_owned(),
        spec: "spec-B".to_owned(),
        bug: false,
    };
    let granted = Event::MergeGranted {
        repo: REPO.to_owned(),
        id: "B".to_owned(),
    };
    assert_eq!(got, applied(true, vec![reclaimed("A"), requested, granted]));
}

#[test]
fn the_caller_is_not_reclaimed_in_its_own_call() {
    // A も B も見張りが居ない。呼んだ識別だけが残る（開発者の呼び出し＝識別なしでは両方消える）。
    // 残った者の見張りの記録は、殺された待ちの記録として消える。
    let absent = "absent";
    let cases = [
        (
            Some("A"),
            vec!["A"],
            vec![reclaimed("B"), wait_removed("A", WaitKind::Watch, absent)],
        ),
        (
            Some("B"),
            vec!["B"],
            vec![reclaimed("A"), wait_removed("B", WaitKind::Watch, absent)],
        ),
        (None, vec![], vec![reclaimed("A"), reclaimed("B")]),
    ];
    for (caller, left, events) in cases {
        let mut state = state_with(&["A", "B"]);

        let got = tick(&mut state, caller, T0 + 10, &no_watch(&["A", "B"]));

        assert_eq!(ids(&state), left, "{caller:?}");
        assert_eq!(state.waits, vec![], "{caller:?}");
        assert_eq!(got, applied(true, events), "{caller:?}");
    }
}

#[test]
fn the_stop_requested_the_stopped_and_the_awaiting_are_not_reclaimed() {
    // C の負荷テストが走っている（走っている印つき＝誰にも停止要請を出さない）。A は停止要請中、
    // B は止まった、D は再開してから見張りを立て直すまでの印つき。3 人とも見張りは終わっていて、
    // その記録も無い。
    let mut state = state_with(&["A", "B", "C", "D"]);
    hold_running_load(&mut state, "C");
    ask_to_stop(&mut state, "A", "C");
    stop_for(&mut state, "B", "C");
    state
        .participants
        .get_mut("D")
        .expect("D が居る")
        .awaiting_watch_since = Some(T0 + 1);
    state.waits.retain(|wait| wait.id == "C");
    let before = state.clone();

    // 開発者の呼び出し（識別なし）でも、ほかの参加者の呼び出しでも回収しない。
    for caller in [None, Some("C")] {
        let got = tick(&mut state, caller, T0 + 10, &no_watch(&["A", "B", "D"]));

        assert_eq!(state, before, "{caller:?}");
        assert_eq!(got, applied(false, vec![]), "{caller:?}");
    }
}

#[test]
fn a_load_holder_with_the_awaiting_mark_and_no_watch_is_not_reclaimed() {
    // 止まっていた A が番を受けて「作業中」へ戻った直後: 印つきで、見張りはまだ立て直していない。
    // B は A の負荷テストのために止まっている。
    let mut state = state_with(&["A", "B"]);
    hold_load(&mut state, "A", &["B"]);
    stop_for(&mut state, "B", "A");
    state
        .participants
        .get_mut("A")
        .expect("A が居る")
        .awaiting_watch_since = Some(T0);
    state.waits.retain(|wait| wait.id == "B");
    let before = state.clone();

    let got = tick(&mut state, Some("B"), T0 + 10, &no_watch(&["A"]));

    // 負荷テストの最中に持ち主が外れない。
    assert_eq!(state, before);
    assert_eq!(got, applied(false, vec![]));
}

#[test]
fn an_awaiting_participant_whose_watch_is_present_loses_only_the_mark() {
    let mut state = state_with(&["A", "B"]);
    state
        .participants
        .get_mut("A")
        .expect("A が居る")
        .awaiting_watch_since = Some(T0 + 1);
    let mut expected = state.clone();

    let got = run(&mut state, &Command::Tick, Some("B"), T0 + 10);

    expected
        .participants
        .get_mut("A")
        .expect("A が居る")
        .awaiting_watch_since = None;
    assert_eq!(state, expected);
    assert_eq!(got, applied(true, vec![]));

    // 印が消えた後は、見張りが居なくなれば回収される。
    let got = tick(&mut state, Some("B"), T0 + 20, &no_watch(&["A"]));
    assert_eq!(ids(&state), ["B"]);
    assert_eq!(got, applied(true, vec![reclaimed("A")]));
}

#[test]
fn reclaim_records_come_newest_first_with_their_times() {
    let mut state = state_with(&["A", "B", "C"]);

    tick(&mut state, Some("C"), T0 + 10, &no_watch(&["A"]));
    tick(&mut state, Some("C"), T0 + 20, &no_watch(&["A", "B"]));

    assert_eq!(
        state.recent,
        vec![reclaim_record("B", T0 + 20), reclaim_record("A", T0 + 10)]
    );
}

#[test]
fn a_killed_wait_record_disappears_and_its_request_stays() {
    // A は B の持つ机を待っていて、その待ちのプロセスが殺された（ロックが解かれている）。
    let mut state = state_with(&["A", "B"]);
    hold_merge(&mut state, REPO, "B");
    queue_merge(&mut state, REPO, "A", T0 + 1);
    state.waits.push(wait_record("A", WaitKind::Merge));
    state.waits.push(wait_record("B", WaitKind::Merge));
    let mut expected = state.clone();
    let presence = FakePresence::all_present().absent("A", WaitKind::Merge);

    let got = tick(&mut state, Some("B"), T0 + 10, &presence);

    // 消えるのは、その識別の、その種類の記録だけ。A の見張りの記録も B のマージの待ちの記録も
    // 残る。申し込みと参加者の記録も残る（待ちを始め直せば引き継ぐ）。
    expected
        .waits
        .retain(|wait| (wait.id.as_str(), wait.kind) != ("A", WaitKind::Merge));
    assert_eq!(state, expected);
    assert_eq!(state.waits.len(), 3);
    let removed = wait_removed("A", WaitKind::Merge, "absent");
    assert_eq!(got, applied(true, vec![removed]));
}

#[test]
fn registering_a_wait_puts_its_record_and_the_same_kind_replaces_it() {
    let mut state = state_with(&["A"]);
    let registered = || applied(true, vec![wait_registered("A", WaitKind::Merge)]);

    let cmd = register("A", WaitKind::Merge, 10, T0 + 1);
    let got = run(&mut state, &cmd, Some("A"), T0 + 1);
    assert_eq!(got, registered());

    // 待ちを始め直すと、同じ識別・同じ種類の記録がその場で置き換わる。
    let again = register("A", WaitKind::Merge, 11, T0 + 5);
    let got = run(&mut state, &again, Some("A"), T0 + 5);
    assert_eq!(got, registered());
    let merge_wait = WaitRecord {
        pid: 11,
        since: T0 + 5,
        ..wait_record("A", WaitKind::Merge)
    };
    assert_eq!(
        state.waits,
        vec![wait_record("A", WaitKind::Watch), merge_wait]
    );

    // 同じ記録をもう一度登録しても何も変わらない。
    let got = run(&mut state, &again, Some("A"), T0 + 5);
    assert_eq!(got, applied(false, vec![]));
}

#[test]
fn wait_records_of_one_identity_and_different_kinds_stand_side_by_side() {
    let mut state = state_with(&["A", "B"]);
    for kind in [WaitKind::Merge, WaitKind::Load, WaitKind::Resume] {
        run(&mut state, &register("A", kind, PID, T0), Some("A"), T0);
        run(&mut state, &register("B", kind, PID, T0), Some("B"), T0);
    }
    let all = [
        WaitKind::Watch,
        WaitKind::Merge,
        WaitKind::Load,
        WaitKind::Resume,
    ];
    assert_eq!(kinds(&state, "A"), all);
    assert_eq!(kinds(&state, "B"), all);

    // 抹消も、その識別の、その種類の記録だけ。
    let got = run(&mut state, &unregister("A", WaitKind::Load), Some("A"), T0);

    assert_eq!(
        kinds(&state, "A"),
        [WaitKind::Watch, WaitKind::Merge, WaitKind::Resume]
    );
    assert_eq!(kinds(&state, "B"), all);
    let removed = wait_removed("A", WaitKind::Load, "ended");
    assert_eq!(got, applied(true, vec![removed]));
}

#[test]
fn unregistering_a_wait_that_is_not_there_changes_nothing() {
    for cmd in [
        unregister("A", WaitKind::Merge),
        unregister("Z", WaitKind::Watch),
    ] {
        let mut state = state_with(&["A", "B"]);
        let before = state.clone();

        let got = run(&mut state, &cmd, Some("A"), T0 + 10);

        assert_eq!(state, before, "{cmd:?}");
        assert_eq!(got, applied(false, vec![]), "{cmd:?}");
    }
}

#[test]
fn a_registration_is_never_removed_by_the_reclaim_of_the_same_call() {
    // 殺された前の待ちの記録が残っていて、生死の口がまだ「居ない」と答える場合でも、回収は登録より
    // 先なので、消えるのは前の記録だけで、いま登録した記録は残る。
    let mut state = state_with(&["A"]);
    state.waits.push(wait_record("A", WaitKind::Merge));
    let presence = FakePresence::all_present().absent("A", WaitKind::Merge);
    let cmd = register("A", WaitKind::Merge, 99, T0 + 10);

    let got = apply(&mut state, &cmd, Some("A"), T0 + 10, &presence);

    let merge_wait = WaitRecord {
        pid: 99,
        since: T0 + 10,
        ..wait_record("A", WaitKind::Merge)
    };
    assert_eq!(
        state.waits,
        vec![wait_record("A", WaitKind::Watch), merge_wait]
    );
    let events = vec![
        wait_removed("A", WaitKind::Merge, "absent"),
        wait_registered("A", WaitKind::Merge),
    ];
    assert_eq!(got, applied(true, events));
}

#[test]
fn a_tick_with_nothing_to_do_changes_nothing() {
    // 全員に見張りが居て、机は持たれ、待ち行列は番を待っている。
    let mut state = state_with(&["A", "B"]);
    hold_merge(&mut state, REPO, "A");
    queue_merge(&mut state, REPO, "B", T0 + 1);
    let before = state.clone();

    for caller in [None, Some("B")] {
        let got = run(&mut state, &Command::Tick, caller, T0 + 10);

        assert_eq!(state, before, "{caller:?}");
        assert_eq!(got, applied(false, vec![]), "{caller:?}");
    }
}
