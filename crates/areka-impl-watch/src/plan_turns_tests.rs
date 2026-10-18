//! `plan::turns` のテスト: 申し込み・返し・番の決め直し（一度に渡す・追い越し・持ちながら・
//! 返し方・固定点）。支えは親の `plan_test_support.rs` を読むだけ。

use super::super::test_support::{
    PURPOSE, REPO, T0, applied, ask_to_stop, assert_not_applied, give_back, hold_load, hold_merge,
    hold_sole, hold_test, load_done, queue_load, queue_merge, queue_request, request, run,
    state_with, stop_for, stopped, test_freed, test_freed_record,
};
use super::replan;
use crate::plan::{Command, Event};
use crate::state::{DeskSet, ParticipantStatus, State};

/// 机の番が来た出来事。
fn granted(id: &str, desks: DeskSet, stopped: &[&str]) -> Event {
    Event::DeskGranted {
        id: id.to_owned(),
        desks,
        stopped: stopped.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// 机を返した出来事。
fn returned(id: &str, desks: DeskSet) -> Event {
    Event::DeskReturned {
        id: id.to_owned(),
        desks,
    }
}

/// 出来事に机の番が 1 件も無い。
fn no_grant(events: &[Event]) -> bool {
    !events
        .iter()
        .any(|event| matches!(event, Event::DeskGranted { .. }))
}

/// （排他テストの机の持ち主, ひとり占めの持ち主）。
fn holders(state: &State) -> (Option<&str>, Option<&str>) {
    (
        state.test.as_ref().map(|holder| holder.id.as_str()),
        state.sole.as_ref().map(|holder| holder.id.as_str()),
    )
}

/// 待っている申し込みの（識別, 机の組）を並んだ順に。
fn waiting(state: &State) -> Vec<(&str, DeskSet)> {
    let requests = state.requests.iter();
    requests.map(|r| (r.id.as_str(), r.desks)).collect()
}

fn status(state: &State, id: &str) -> ParticipantStatus {
    state.participants[id].status
}

/// 固定点: その状態で番の決め直しを呼んでも、状態も出来事も変わらない（どの場面の終わりにも見る）。
fn assert_fixed_point(state: &State, label: &str) {
    let mut again = state.clone();
    let mut events = Vec::new();
    replan(&mut again, T0 + 100, &mut events);
    assert_eq!(&again, state, "{label}");
    assert!(events.is_empty(), "{label}: {events:?}");
}

/// 一度に渡す（要件 2.3）: 止まる必要のある全員が止まっていて、両方の机が空いていれば、
/// `test,sole` の申し込みは 1 回の呼び出しで両方を受け、出来事 `DeskGranted` は 1 件。
#[test]
fn test_and_sole_are_granted_at_once_in_one_call() {
    let mut state = state_with(&["a", "b"]);
    stop_for(&mut state, "b", "a");

    let got = run(
        &mut state,
        &request("a", DeskSet::TestSole),
        Some("a"),
        T0 + 1,
    );

    let granted: Vec<&Event> = got
        .events
        .iter()
        .filter(|event| matches!(event, Event::DeskGranted { .. }))
        .collect();
    assert_eq!(
        granted,
        [&Event::DeskGranted {
            id: "a".to_owned(),
            desks: DeskSet::TestSole,
            stopped: vec!["b".to_owned()],
        }]
    );
    assert_eq!(state.test.as_ref().map(|h| h.id.as_str()), Some("a"));
    let sole = state.sole.as_ref().expect("ひとり占めの持ち主が居る");
    assert_eq!(
        (sole.id.as_str(), sole.stopped.as_slice()),
        ("a", &["b".to_owned()][..])
    );
    assert!(
        state.requests.is_empty(),
        "渡した申し込みは待ち行列から外れる"
    );
}

/// 固定点: 番の決め直しを 2 度続けて呼んだ 2 度目は、状態も出来事も変えない。
#[test]
fn replan_twice_changes_nothing_the_second_time() {
    let mut state = state_with(&["a", "b", "c"]);
    run(
        &mut state,
        &request("a", DeskSet::TestSole),
        Some("a"),
        T0 + 1,
    );
    run(&mut state, &request("b", DeskSet::Test), Some("b"), T0 + 2);

    let before = state.clone();
    let mut events = Vec::new();
    replan(&mut state, T0 + 3, &mut events);

    assert_eq!(state, before);
    assert!(events.is_empty(), "{events:?}");
}

/// 一部を渡さない（要件 2.2）: 排他テストの机が空いていても、ひとり占めが渡せない間（止まって
/// いない者が居る・マージの持ち主が居る）は、`test,sole` の申し込みにどちらの机も渡さない。
#[test]
fn test_and_sole_get_nothing_while_the_sole_desk_cannot_be_granted() {
    // b がまだ止まっていない。
    let mut not_stopped = state_with(&["a", "b"]);
    let got = run(
        &mut not_stopped,
        &request("a", DeskSet::TestSole),
        Some("a"),
        T0 + 1,
    );
    assert!(no_grant(&got.events), "{:?}", got.events);
    assert_eq!(holders(&not_stopped), (None, None), "not stopped");
    assert_eq!(waiting(&not_stopped), [("a", DeskSet::TestSole)]);
    assert_eq!(status(&not_stopped, "b"), ParticipantStatus::StopRequested);
    assert_fixed_point(&not_stopped, "not stopped");

    // b は止まったが、c が pasta のマージの机を持つ。
    let mut merging = state_with(&["a", "b", "c"]);
    stop_for(&mut merging, "b", "a");
    hold_merge(&mut merging, "pasta", "c");
    let got = run(
        &mut merging,
        &request("a", DeskSet::TestSole),
        Some("a"),
        T0 + 1,
    );
    assert!(no_grant(&got.events), "{:?}", got.events);
    assert_eq!(holders(&merging), (None, None), "merging");
    assert_eq!(waiting(&merging), [("a", DeskSet::TestSole)]);
    assert_fixed_point(&merging, "merging");
}

/// 追い越し（要件 3.3・3.4）: 先の `test,sole` が止まりを待つ間に、後から申し込んだマージの持ち主の
/// `test` が同じ呼び出しで渡る。先の申し込みのために排他テストの机を取っておかない。
#[test]
fn a_later_test_request_of_a_merge_holder_overtakes_a_waiting_load_test() {
    let mut state = state_with(&["a", "b", "c"]);
    hold_merge(&mut state, REPO, "b");
    run(
        &mut state,
        &request("a", DeskSet::TestSole),
        Some("a"),
        T0 + 1,
    );
    assert_eq!(status(&state, "c"), ParticipantStatus::StopRequested);

    let got = run(&mut state, &request("b", DeskSet::Test), Some("b"), T0 + 2);

    assert_eq!(
        got,
        applied(
            true,
            vec![
                Event::DeskRequested {
                    id: "b".to_owned(),
                    desks: DeskSet::Test,
                },
                granted("b", DeskSet::Test, &[]),
            ]
        )
    );
    assert_eq!(holders(&state), (Some("b"), None));
    assert_eq!(waiting(&state), [("a", DeskSet::TestSole)]);
    let merge = state.merge[REPO].holder.as_ref().map(|h| h.id.as_str());
    assert_eq!(merge, Some("b"), "マージの机は持ったまま");
    assert_fixed_point(&state, "overtaken");
}

/// 要件 3.4: ひとり占めを含む申し込みが待たれる間、排他テストの机は「停止要請中」の者とマージの
/// 持ち主にだけ渡り、「作業中」の者（マージの机を待つ者を含む）には渡らない。
#[test]
fn while_a_load_test_waits_the_test_desk_goes_only_to_the_asked_and_merge_holders() {
    // a の負荷テストが待たれ、z はまだ止まっていない（ひとり占めは渡らない）。
    let base = || {
        let mut state = state_with(&["a", "x", "z"]);
        queue_load(&mut state, "a", T0 + 1);
        ask_to_stop(&mut state, "z", "a");
        state
    };

    // マージの机を待つ x は止まる必要が無いので「作業中」のまま。その `test` は渡らない（マージを
    // 待つ者の申し込みは要件 4.8 で断られるので、申し込みは状態を組み立てて置く）。
    let mut working = base();
    queue_merge(&mut working, "pasta", "x", T0);
    queue_request(&mut working, "x", DeskSet::Test, T0 + 2);
    let mut events = Vec::new();
    replan(&mut working, T0 + 3, &mut events);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(status(&working, "x"), ParticipantStatus::Working);
    assert_eq!(holders(&working), (None, None), "working");

    // 停止要請中の z の `test` は渡る。
    let mut asked = base();
    let got = run(&mut asked, &request("z", DeskSet::Test), Some("z"), T0 + 2);
    assert!(got.events.contains(&granted("z", DeskSet::Test, &[])));
    assert_eq!(holders(&asked), (Some("z"), None), "asked");
    assert_fixed_point(&asked, "asked");

    // マージの持ち主の x の `test` は渡る。
    let mut holder = base();
    hold_merge(&mut holder, "pasta", "x");
    let got = run(&mut holder, &request("x", DeskSet::Test), Some("x"), T0 + 2);
    assert!(got.events.contains(&granted("x", DeskSet::Test, &[])));
    assert_eq!(holders(&holder), (Some("x"), None), "merge holder");
    assert_fixed_point(&holder, "merge holder");
}

/// 要件 3.5: 「止まった」の者の `test` は渡らず、ひとり占めの申し込みが消えて再開した同じ呼び出しで
/// 渡る。
#[test]
fn a_stopped_participant_gets_the_test_desk_in_the_call_that_resumes_it() {
    let mut state = state_with(&["a", "b", "c"]);
    queue_load(&mut state, "a", T0 + 1);
    stop_for(&mut state, "b", "a");
    ask_to_stop(&mut state, "c", "a");

    let got = run(&mut state, &request("b", DeskSet::Test), Some("b"), T0 + 2);
    assert!(no_grant(&got.events), "{:?}", got.events);
    assert_eq!(holders(&state), (None, None));
    assert_eq!(status(&state, "b"), ParticipantStatus::Stopped);
    assert_fixed_point(&state, "stopped");

    let cancel = Command::Cancel { id: "a".to_owned() };
    let got = run(&mut state, &cancel, None, T0 + 3);

    let resumed = Event::Resumed {
        id: "b".to_owned(),
        why: "no-sole",
    };
    let at = |wanted: &Event| got.events.iter().position(|event| event == wanted);
    let (resumed, grant) = (at(&resumed), at(&granted("b", DeskSet::Test, &[])));
    assert!(
        resumed.is_some() && grant.is_some() && resumed < grant,
        "{:?}",
        got.events
    );
    assert_eq!(holders(&state), (Some("b"), None));
    assert_fixed_point(&state, "resumed");
}

/// 要件 6.2: 排他テストの机を持つ者の「止まった」の報告で机が空き、出来事 `TestFreed`（訳
/// `stopped`）と最近の出来事が 1 件ずつ残る。空いたので待っていた負荷テストに同じ呼び出しで番が来る。
#[test]
fn reporting_stopped_frees_the_test_desk_and_records_it_once() {
    let mut state = state_with(&["a", "b"]);
    queue_load(&mut state, "a", T0 + 1);
    ask_to_stop(&mut state, "b", "a");
    hold_test(&mut state, "b");

    let got = run(&mut state, &stopped("b"), Some("b"), T0 + 5);

    assert_eq!(
        got,
        applied(
            true,
            vec![
                Event::Stopped { id: "b".to_owned() },
                test_freed("b", "stopped"),
                granted("a", DeskSet::TestSole, &["b"]),
            ]
        )
    );
    assert_eq!(state.recent, [test_freed_record("b", "stopped", T0 + 5)]);
    assert_eq!(holders(&state), (Some("a"), Some("a")));
    assert_fixed_point(&state, "stopped");
}

/// 持ちながらの申し込み（要件 4.1・4.2）: マージの持ち主の `test` は加わる。机を持つ者のほかの組
/// （`test` の持ち主の `test,sole`・`sole`、ひとり占めの持ち主の `test`）と、マージの持ち主の
/// `sole`・`test,sole` は断られ、状態は同じ。
#[test]
fn requests_while_holding_are_accepted_only_for_a_merge_holder_adding_test() {
    let mut state = state_with(&["a"]);
    hold_merge(&mut state, REPO, "a");
    let got = run(&mut state, &request("a", DeskSet::Test), Some("a"), T0 + 1);
    assert!(got.events.contains(&granted("a", DeskSet::Test, &[])));
    assert_eq!(holders(&state), (Some("a"), None));
    assert!(state.merge[REPO].holder.is_some(), "マージの机は持ったまま");
    assert_fixed_point(&state, "merge holder adds test");

    let mut merge_holder = state_with(&["a"]);
    hold_merge(&mut merge_holder, REPO, "a");
    let mut test_holder = state_with(&["a"]);
    hold_test(&mut test_holder, "a");
    let mut sole_holder = state_with(&["a"]);
    hold_sole(&mut sole_holder, "a", &[]);
    let cases = [
        (
            &test_holder,
            DeskSet::TestSole,
            "test holder asks test,sole",
        ),
        (&test_holder, DeskSet::Sole, "test holder asks sole"),
        (&sole_holder, DeskSet::Test, "sole holder asks test"),
        (&merge_holder, DeskSet::Sole, "merge holder asks sole"),
        (
            &merge_holder,
            DeskSet::TestSole,
            "merge holder asks test,sole",
        ),
    ];
    for (before, desks, label) in cases {
        let mut state = before.clone();
        let got = run(&mut state, &request("a", desks), Some("a"), T0 + 1);
        assert_eq!(&state, before, "{label}");
        assert_not_applied(&got, label);
    }
}

/// 要件 2.7・2.8: 同じ組の再申し込みは待ち行列に 1 件のまま、持っていれば直ちに番（状態は同じ）。
/// 待っている者の違う組の申し込みは断る。
#[test]
fn a_repeated_request_keeps_one_entry_and_a_different_set_is_refused() {
    // a は止まっていない b のために `test,sole` を待ち、停止要請中の c は `test` を持つ。
    let mut before = state_with(&["a", "b", "c"]);
    queue_load(&mut before, "a", T0 + 1);
    ask_to_stop(&mut before, "b", "a");
    ask_to_stop(&mut before, "c", "a");
    hold_test(&mut before, "c");
    assert_fixed_point(&before, "before");

    let mut state = before.clone();
    let cmd = request("a", DeskSet::TestSole);
    let got = run(&mut state, &cmd, Some("a"), T0 + 2);
    assert_eq!(got, applied(false, vec![]), "same set while waiting");
    assert_eq!(state, before);

    let mut state = before.clone();
    let got = run(&mut state, &request("c", DeskSet::Test), Some("c"), T0 + 2);
    assert_eq!(got, applied(false, vec![]), "same set while holding");
    assert_eq!(state, before);

    for desks in [DeskSet::Test, DeskSet::Sole] {
        let mut state = before.clone();
        let got = run(&mut state, &request("a", desks), Some("a"), T0 + 2);
        assert_eq!(state, before, "{desks:?}");
        assert_not_applied(&got, "different set while waiting");
    }
}

/// 返し方（要件 4.3〜4.5）: 名指しした机だけを空け、名指しなしは全部を空ける。持っていない机を
/// 返そうとしたら断り、状態は同じ。
#[test]
fn return_frees_the_named_desk_or_everything_and_refuses_desks_not_held() {
    let held = || {
        let mut state = state_with(&["a", "b"]);
        hold_load(&mut state, "a", &["b"]);
        stop_for(&mut state, "b", "a");
        state
    };

    // 排他テストの机だけ: ひとり占めは持ったまま、b は止まったまま。
    let mut state = held();
    let cmd = give_back("a", Some(DeskSet::Test));
    let got = run(&mut state, &cmd, Some("a"), T0 + 1);
    assert_eq!(
        got,
        applied(
            true,
            vec![test_freed("a", "returned"), returned("a", DeskSet::Test)]
        )
    );
    assert_eq!(holders(&state), (None, Some("a")));
    assert_eq!(status(&state, "b"), ParticipantStatus::Stopped);
    assert_eq!(state.recent, [test_freed_record("a", "returned", T0 + 1)]);
    assert_fixed_point(&state, "return test");

    // ひとり占めだけ: 排他テストの机は持ったまま（止まった b の再開は要件 4.6 のテストで見る）。
    let mut state = held();
    let cmd = give_back("a", Some(DeskSet::Sole));
    let got = run(&mut state, &cmd, Some("a"), T0 + 1);
    assert!(got.events.contains(&returned("a", DeskSet::Sole)));
    assert_eq!(holders(&state), (Some("a"), None));
    assert_fixed_point(&state, "return sole");

    // 名指しなし: 両方。
    let mut state = held();
    run(&mut state, &give_back("a", None), Some("a"), T0 + 1);
    assert_eq!(holders(&state), (None, None));
    assert_fixed_point(&state, "return all");

    // 持っていない机。
    let mut test_only = state_with(&["a", "b"]);
    hold_test(&mut test_only, "a");
    let cases = [
        ("a", Some(DeskSet::Sole), "test holder returns sole"),
        ("a", Some(DeskSet::TestSole), "test holder returns both"),
        ("b", Some(DeskSet::Test), "non-holder returns test"),
        ("b", None, "non-holder returns all"),
    ];
    for (id, desks, label) in cases {
        let mut state = test_only.clone();
        let got = run(&mut state, &give_back(id, desks), Some(id), T0 + 1);
        assert_eq!(state, test_only, "{label}");
        assert_not_applied(&got, label);
    }
}

/// 要件 4.6: ひとり占めを返すと、ほかに待たれていなければ止まった者が同じ呼び出しで再開する。
#[test]
fn returning_the_sole_desk_resumes_the_stopped_in_the_same_call() {
    let mut state = state_with(&["a", "b", "c"]);
    hold_load(&mut state, "a", &["b", "c"]);
    stop_for(&mut state, "b", "a");
    stop_for(&mut state, "c", "a");

    let got = run(&mut state, &load_done("a"), Some("a"), T0 + 4);

    let resumed = |id: &str| Event::Resumed {
        id: id.to_owned(),
        why: "no-sole",
    };
    assert_eq!(
        got,
        applied(
            true,
            vec![
                test_freed("a", "returned"),
                returned("a", DeskSet::TestSole),
                resumed("b"),
                resumed("c"),
            ]
        )
    );
    assert_eq!(status(&state, "b"), ParticipantStatus::Working);
    assert_eq!(status(&state, "c"), ParticipantStatus::Working);
    assert_eq!(holders(&state), (None, None));
    assert_fixed_point(&state, "resumed");
}

/// 要件 5.5・5.6: 走っている負荷テストの記録は、両方の机が空なら停止要請なしで両方の持ち主にする。
/// どちらか一方にだけ持ち主が居ても断り、状態は同じ。
#[test]
fn recording_a_running_load_test_needs_both_desks_free() {
    let load_running = |id: &str| Command::LoadRunning {
        id: id.to_owned(),
        name: None,
        repo: REPO.to_owned(),
        purpose: PURPOSE.to_owned(),
    };

    let mut state = state_with(&["a", "b"]);
    let got = run(&mut state, &load_running("a"), Some("a"), T0 + 1);
    let recorded = Event::LoadRunning { id: "a".to_owned() };
    assert_eq!(got, applied(true, vec![recorded]));
    assert_eq!(holders(&state), (Some("a"), Some("a")));
    assert_eq!(status(&state, "b"), ParticipantStatus::Working);
    assert_fixed_point(&state, "running");

    // 一方だけに持ち主が居る（b のひとり占めのために a は止まっている）。
    let mut test_only = state_with(&["a", "b"]);
    hold_test(&mut test_only, "b");
    let mut sole_only = state_with(&["a", "b"]);
    hold_sole(&mut sole_only, "b", &["a"]);
    stop_for(&mut sole_only, "a", "b");
    for (before, label) in [(&test_only, "test held"), (&sole_only, "sole held")] {
        let mut state = before.clone();
        let got = run(&mut state, &load_running("a"), Some("a"), T0 + 1);
        assert_eq!(&state, before, "{label}");
        assert_not_applied(&got, label);
    }
}
