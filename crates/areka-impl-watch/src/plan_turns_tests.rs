//! `plan::turns` のテスト: 申し込み・返し・番の決め直し（一度に渡す・追い越し・持ちながら・
//! 返し方・固定点）。支えは親の `plan_test_support.rs` を読むだけ。

use super::super::test_support::{
    PURPOSE, REPO, applied, ask_to_stop, assert_not_applied, cancel, desk_requested, give_back,
    granted, hold_load, hold_merge, hold_sole, hold_test, joined, load_done, load_running,
    queue_load, queue_merge, queue_request, recorded_running, request, resumed, returned, run,
    state_with, stop_for, stop_reported, stopped, test_freed, test_freed_record,
};
use super::replan;
use crate::plan::{Command, Event};
use crate::state::{DeskSet, Participant, ParticipantStatus, SoleHolder, State, TestHolder};
use crate::test_support::T0;

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

    let grants: Vec<&Event> = got
        .events
        .iter()
        .filter(|event| matches!(event, Event::DeskGranted { .. }))
        .collect();
    assert_eq!(grants, [&granted("a", DeskSet::TestSole, &["b"])]);
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

/// ひとり占めだけの番（要件 4.1）: `sole` だけを申し込んだ者は、止まる必要のある全員が止まった
/// ときにひとり占めだけを受け、排他テストの机は誰も持たない。続けて参加した者の `test` は同じ
/// 呼び出しで渡る（ひとり占めは待たれていない）。
#[test]
fn a_sole_only_request_gets_only_the_sole_desk() {
    let mut state = state_with(&["a", "b"]);
    run(&mut state, &request("a", DeskSet::Sole), Some("a"), T0 + 1);

    let got = run(&mut state, &stopped("b"), Some("b"), T0 + 2);

    assert!(
        got.events.contains(&granted("a", DeskSet::Sole, &["b"])),
        "{:?}",
        got.events
    );
    assert_eq!(holders(&state), (None, Some("a")));
    assert_fixed_point(&state, "sole only");

    let got = run(&mut state, &request("d", DeskSet::Test), Some("d"), T0 + 3);

    assert!(
        got.events.contains(&granted("d", DeskSet::Test, &[])),
        "{:?}",
        got.events
    );
    assert_eq!(status(&state, "d"), ParticipantStatus::StopRequested);
    assert_eq!(holders(&state), (Some("d"), Some("a")));
    assert_fixed_point(&state, "joined test");
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
                desk_requested("b", DeskSet::Test),
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

    let got = run(&mut state, &cancel("a"), None, T0 + 3);

    let at = |wanted: &Event| got.events.iter().position(|event| event == wanted);
    let (resumed, grant) = (
        at(&resumed("b", "no-sole")),
        at(&granted("b", DeskSet::Test, &[])),
    );
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
                stop_reported("b"),
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

    assert_eq!(
        got,
        applied(
            true,
            vec![
                test_freed("a", "returned"),
                returned("a", DeskSet::TestSole),
                resumed("b", "no-sole"),
                resumed("c", "no-sole"),
            ]
        )
    );
    assert_eq!(status(&state, "b"), ParticipantStatus::Working);
    assert_eq!(status(&state, "c"), ParticipantStatus::Working);
    assert_eq!(holders(&state), (None, None));
    assert_fixed_point(&state, "resumed");
}

/// 要件 5.5・5.6: 走っている負荷テストの記録は、両方の机が空なら停止要請なしで、走っている印つきの
/// 両方の持ち主にする（記録は参加の申し出を兼ねる）。test だけ・sole だけ・両方（持ち主に走って
/// いる印が在っても無くても）のどれかに持ち主が居れば断り、状態は同じ。断るときは名前・リポジトリの
/// 更新も、参加も、「作業中」への戻しもしない（元のスクリプトは断っても参加させて「作業中」へ戻す）。
#[test]
fn recording_a_running_load_test_needs_both_desks_free() {
    // 両方空き: 参加していない Z の記録。A も B も「作業中」のまま。
    let mut state = state_with(&["A", "B"]);
    let mut expected = state.clone();
    let got = run(&mut state, &load_running("Z"), Some("Z"), T0 + 5);
    expected.participants.insert(
        "Z".to_owned(),
        Participant {
            id: "Z".to_owned(),
            name: "Z".to_owned(),
            repo: REPO.to_owned(),
            since: T0 + 5,
            ..Participant::default()
        },
    );
    expected.test = Some(TestHolder {
        id: "Z".to_owned(),
        purpose: PURPOSE.to_owned(),
        requested: T0 + 5,
        granted: T0 + 5,
    });
    expected.sole = Some(SoleHolder {
        id: "Z".to_owned(),
        purpose: PURPOSE.to_owned(),
        requested: T0 + 5,
        granted: T0 + 5,
        running: true,
        stopped: Vec::new(),
    });
    assert_eq!(state, expected);
    let recorded = recorded_running("Z");
    let events = vec![joined("Z"), recorded];
    assert_eq!(got, applied(true, events));
    assert_fixed_point(&state, "both free");

    // C が test だけ・sole だけ（A は C のために止まっている）・両方を持つ。
    let held = |test: bool, sole: bool| {
        let mut state = state_with(&["A", "C"]);
        if test {
            hold_test(&mut state, "C");
        }
        if sole {
            hold_sole(&mut state, "C", &["A"]);
            stop_for(&mut state, "A", "C");
        }
        state
    };
    let mut marked = held(true, true);
    marked.sole.as_mut().expect("C が持つ").running = true;
    let cases = [
        (held(true, false), "test held"),
        (held(false, true), "sole held"),
        (held(true, true), "both held"),
        (marked, "both held, running"),
    ];
    for (before, label) in cases {
        // A は参加者、C は持ち主、Z は参加していない識別。
        for id in ["A", "C", "Z"] {
            let mut state = before.clone();
            let cmd = Command::LoadRunning {
                id: id.to_owned(),
                name: Some("別の名前".to_owned()),
                repo: "pasta".to_owned(),
                purpose: "別の内容".to_owned(),
            };
            let got = run(&mut state, &cmd, Some(id), T0 + 9);
            assert_eq!(state, before, "{label} {id}");
            assert_not_applied(&got, &format!("{label} {id}"));
        }
    }
}
