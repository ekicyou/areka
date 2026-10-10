//! 停止要請・止まった・再開の判断（規則 2・5・7）の決定論テスト。時刻と生死は引数で渡し、眠らない。

use super::test_support::{
    PID, PURPOSE, REPO, T0, applied, ask_to_stop, assert_not_applied, hold_load, hold_merge,
    hold_running_load, load_done, load_test, queue_load, queue_merge, run, state_with, stop_for,
    stopped, watch_registered,
};
use super::{Command, Event};
use crate::state::{LoadDesk, LoadHolder, Participant, ParticipantStatus, State};

use ParticipantStatus::{StopRequested, Stopped, Working};

fn load_requested(id: &str) -> Event {
    Event::LoadRequested { id: id.to_owned() }
}

fn load_granted(id: &str, stopped: &[&str]) -> Event {
    Event::LoadGranted {
        id: id.to_owned(),
        stopped: stopped.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn load_finished(id: &str) -> Event {
    Event::LoadDone { id: id.to_owned() }
}

fn stop_requested(id: &str, by: &str) -> Event {
    Event::StopRequested {
        id: id.to_owned(),
        by: by.to_owned(),
    }
}

fn stop_reported(id: &str) -> Event {
    Event::Stopped { id: id.to_owned() }
}

fn resumed(id: &str, why: &'static str) -> Event {
    Event::Resumed {
        id: id.to_owned(),
        why,
    }
}

fn recorded_running(id: &str) -> Event {
    Event::LoadRunning { id: id.to_owned() }
}

fn unstopped(id: &str) -> Event {
    Event::Unstopped { id: id.to_owned() }
}

/// 名前を省いた、内容が [`PURPOSE`] の「すでに走っている負荷テスト」の記録。
fn load_running(id: &str) -> Command {
    Command::LoadRunning {
        id: id.to_owned(),
        name: None,
        repo: REPO.to_owned(),
        purpose: PURPOSE.to_owned(),
    }
}

/// 停止要請の取り消し（`None` は全員）。
fn unstop(id: Option<&str>) -> Command {
    Command::Unstop {
        id: id.map(str::to_owned),
    }
}

/// C が「すでに走っている負荷テスト」の机を持ち、A は止まっていて、B は停止要請中のまま
/// （C が記録する前に、別の負荷テストのために出ていた停止要請の残り）。
fn c_runs_with_a_stopped_and_b_asked() -> State {
    let mut state = state_with(&["A", "B", "C"]);
    hold_running_load(&mut state, "C");
    stop_for(&mut state, "A", "C");
    ask_to_stop(&mut state, "B", "C");
    state
}

/// 参加者の（状態, その状態になった時刻, 停止要請の主, 見張りを立て直すまでの印）。
fn seen<'a>(state: &'a State, id: &str) -> (ParticipantStatus, u64, Option<&'a str>, Option<u64>) {
    let participant = &state.participants[id];
    let by = participant.stop_reason.as_ref().map(|r| r.by.as_str());
    (
        participant.status,
        participant.since,
        by,
        participant.awaiting_watch_since,
    )
}

/// C が番の来た負荷テストの机を持ち、A はそのために止まっている。
fn c_holds_with_a_stopped(ids: &[&str]) -> State {
    let mut state = state_with(ids);
    hold_load(&mut state, "C", &["A"]);
    stop_for(&mut state, "A", "C");
    state
}

#[test]
fn a_load_request_asks_the_working_participants_to_stop_with_the_reason() {
    let mut state = state_with(&["A", "B", "C"]);
    let mut expected = state.clone();

    let got = run(&mut state, &load_test("C"), Some("C"), T0 + 5);

    // 候補（C）のほかの「作業中」の全員が、誰の・どんな負荷テストかの理由つきで「停止要請中」になる。
    // まだ誰も止まっていないので、C は待ち行列に並んだまま。
    queue_load(&mut expected, "C", T0 + 5);
    for id in ["A", "B"] {
        ask_to_stop(&mut expected, id, "C");
        expected.participants.get_mut(id).expect("居る").since = T0 + 5;
    }
    assert_eq!(state, expected);
    let reason = state.participants["A"].stop_reason.as_ref().expect("理由");
    assert_eq!(
        (reason.by.as_str(), reason.purpose.as_str()),
        ("C", PURPOSE)
    );
    let events = vec![
        load_requested("C"),
        stop_requested("A", "C"),
        stop_requested("B", "C"),
    ];
    assert_eq!(got, applied(true, events));
}

#[test]
fn a_stop_round_trip_reports_one_event_per_change() {
    let mut state = state_with(&["A", "B", "C"]);
    run(&mut state, &load_test("C"), Some("C"), T0 + 5);

    // 全員が止まるまで番は来ない。
    let got = run(&mut state, &stopped("A"), Some("A"), T0 + 6);
    assert_eq!(got, applied(true, vec![stop_reported("A")]));
    assert_eq!(seen(&state, "A"), (Stopped, T0 + 6, Some("C"), None));
    assert_eq!(state.load.holder, None);

    // 最後の「止まった」と同じ呼び出しで番が来て、そのとき止まっていた識別が控えられる。
    let got = run(&mut state, &stopped("B"), Some("B"), T0 + 7);
    let events = vec![stop_reported("B"), load_granted("C", &["A", "B"])];
    assert_eq!(got, applied(true, events));
    let desk = LoadDesk {
        holder: Some(LoadHolder {
            id: "C".to_owned(),
            purpose: PURPOSE.to_owned(),
            requested: T0 + 5,
            granted: T0 + 7,
            running: false,
            stopped: vec!["A".to_owned(), "B".to_owned()],
        }),
        queue: Vec::new(),
    };
    assert_eq!(state.load, desk);
    // 「作業中」のまま番を受けた持ち主は、何も変わらない（見張りは走り続けている）。
    assert_eq!(seen(&state, "C"), (Working, T0, None, None));

    // 済んだら全員が「作業中」へ戻り、理由が消え、見張りを立て直すまでの印が付く。
    let got = run(&mut state, &load_done("C"), Some("C"), T0 + 20);
    let events = vec![
        load_finished("C"),
        resumed("A", "no-load"),
        resumed("B", "no-load"),
    ];
    assert_eq!(got, applied(true, events));
    let mut expected = state_with(&["A", "B", "C"]);
    for id in ["A", "B"] {
        let participant = expected.participants.get_mut(id).expect("居る");
        participant.since = T0 + 20;
        participant.awaiting_watch_since = Some(T0 + 20);
    }
    assert_eq!(state, expected);
}

#[test]
fn merge_holders_and_merge_waiters_are_not_asked_to_stop_and_the_grant_waits_for_merged() {
    // M が areka のマージの机を持ち、W がその後ろで待っている。S は作業中。
    let mut state = state_with(&["C", "M", "S", "W"]);
    hold_merge(&mut state, "areka", "M");
    queue_merge(&mut state, "areka", "W", T0 + 1);

    // 停止要請が出るのは S だけ（候補・マージの持ち主・マージ待ちには出ない）。
    let got = run(&mut state, &load_test("C"), Some("C"), T0 + 5);
    let events = vec![load_requested("C"), stop_requested("S", "C")];
    assert_eq!(got, applied(true, events));
    for id in ["C", "M", "W"] {
        assert_eq!(seen(&state, id), (Working, T0, None, None), "{id}");
    }

    // 止まる必要のある全員が止まっても、マージの持ち主が居る間は番が来ない。
    let got = run(&mut state, &stopped("S"), Some("S"), T0 + 6);
    assert_eq!(got, applied(true, vec![stop_reported("S")]));
    assert_eq!(state.load.holder, None);

    // 「マージが済んだ」と同じ呼び出しで番が来る。空いたマージの机の番は W に出ない。
    let cmd = Command::Merged {
        id: "M".to_owned(),
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
    };
    let got = run(&mut state, &cmd, Some("M"), T0 + 7);
    let events = vec![
        Event::Merged {
            repo: "areka".to_owned(),
            id: "M".to_owned(),
            pr: "281".to_owned(),
            sha: "414d43eb".to_owned(),
        },
        Event::Left {
            id: "M".to_owned(),
            why: "merged",
        },
        load_granted("C", &["S"]),
    ];
    assert_eq!(got, applied(true, events));
    assert_eq!(state.merge["areka"].holder, None);
    assert_eq!(state.merge["areka"].queue[0].id, "W");
}

#[test]
fn a_participant_who_joins_while_a_load_test_is_wanted_or_held_is_asked_to_stop() {
    // 待たれている（A がまだ止まっていない）とき。
    let mut wanted = state_with(&["A", "C"]);
    queue_load(&mut wanted, "C", T0 + 1);
    ask_to_stop(&mut wanted, "A", "C");
    // 持たれているとき。
    let held = c_holds_with_a_stopped(&["A", "C"]);

    for (mut state, label) in [(wanted, "wanted"), (held, "held")] {
        let cmd = Command::Watch {
            id: "D".to_owned(),
            name: None,
            repo: REPO.to_owned(),
            pid: PID,
        };
        let got = run(&mut state, &cmd, Some("D"), T0 + 9);

        let events = vec![
            Event::Joined { id: "D".to_owned() },
            watch_registered("D"),
            stop_requested("D", "C"),
        ];
        assert_eq!(got, applied(true, events), "{label}");
        assert_eq!(
            seen(&state, "D"),
            (StopRequested, T0 + 9, Some("C"), None),
            "{label}"
        );
    }
}

#[test]
fn the_stop_requested_and_the_stopped_go_back_to_work_when_no_load_test_is_left() {
    // 机が持たれていたとき: 持ち主が「済んだ」と言う。B は後から参加して、まだ止まっていない。
    let mut held = c_holds_with_a_stopped(&["A", "B", "C"]);
    ask_to_stop(&mut held, "B", "C");
    // 待たれていただけのとき: 申し込みが取り下げられる。
    let mut wanted = state_with(&["A", "B", "C"]);
    queue_load(&mut wanted, "C", T0 + 1);
    stop_for(&mut wanted, "A", "C");
    ask_to_stop(&mut wanted, "B", "C");
    let cancel = Command::Cancel { id: "C".to_owned() };
    let cancelled = Event::Cancelled { id: "C".to_owned() };

    for (mut state, cmd, first) in [
        (held, load_done("C"), load_finished("C")),
        (wanted, cancel, cancelled),
    ] {
        let got = run(&mut state, &cmd, Some("C"), T0 + 20);

        let events = vec![first, resumed("A", "no-load"), resumed("B", "no-load")];
        assert_eq!(got, applied(true, events), "{cmd:?}");
        assert_eq!(state.load, LoadDesk::default(), "{cmd:?}");
        for id in ["A", "B"] {
            let back = (Working, T0 + 20, None, Some(T0 + 20));
            assert_eq!(seen(&state, id), back, "{cmd:?} {id}");
        }
        // 止まっていなかった者には、印を付けない。
        assert_eq!(seen(&state, "C"), (Working, T0, None, None), "{cmd:?}");
    }
}

#[test]
fn a_queued_load_test_keeps_everyone_stopped_and_takes_the_desk_next() {
    // C が机を持ち、B がその後ろで待っている。A も B も C のために止まっている。
    let mut state = c_holds_with_a_stopped(&["A", "B", "C"]);
    queue_load(&mut state, "B", T0 + 1);
    stop_for(&mut state, "B", "C");
    let still_stopped = (Stopped, T0, Some("C"), None);

    // C が済んでも、続けて並ぶ B の負荷テストがあるので誰も戻らない。今度は C が止まる番。
    let got = run(&mut state, &load_done("C"), Some("C"), T0 + 20);
    let events = vec![load_finished("C"), stop_requested("C", "B")];
    assert_eq!(got, applied(true, events));
    assert_eq!(seen(&state, "A"), still_stopped);
    assert_eq!(seen(&state, "B"), still_stopped);
    assert_eq!(state.load.holder, None);

    // C が止まると B の番。止まっていた B は「作業中」へ戻り、見張りを立て直すまでの印が付く。
    let got = run(&mut state, &stopped("C"), Some("C"), T0 + 21);
    let events = vec![
        stop_reported("C"),
        load_granted("B", &["A", "C"]),
        resumed("B", "load-granted"),
    ];
    assert_eq!(got, applied(true, events));
    assert_eq!(seen(&state, "A"), still_stopped);
    assert_eq!(seen(&state, "B"), (Working, T0 + 21, None, Some(T0 + 21)));

    // B も済んで、並ぶ負荷テストが無くなったら全員が戻る。
    let got = run(&mut state, &load_done("B"), Some("B"), T0 + 30);
    let events = vec![
        load_finished("B"),
        resumed("A", "no-load"),
        resumed("C", "no-load"),
    ];
    assert_eq!(got, applied(true, events));
}

#[test]
fn the_grant_comes_when_the_last_participant_who_has_not_stopped_leaves() {
    let mut state = state_with(&["A", "B", "C"]);
    queue_load(&mut state, "C", T0 + 1);
    stop_for(&mut state, "A", "C");
    ask_to_stop(&mut state, "B", "C");

    let cmd = Command::Leave { id: "B".to_owned() };
    let got = run(&mut state, &cmd, None, T0 + 9);

    // 止まる必要のある参加者が減ったら、同じ呼び出しで番を決め直す。
    let left = Event::Left {
        id: "B".to_owned(),
        why: "leave",
    };
    assert_eq!(got, applied(true, vec![left, load_granted("C", &["A"])]));
}

#[test]
fn asking_for_a_load_test_while_stopped_or_stop_requested_leaves_the_requester_as_it_was() {
    // C が机を持ち、A は止まっていて、B は後から参加してまだ止まっていない。
    let mut before = c_holds_with_a_stopped(&["A", "B", "C"]);
    ask_to_stop(&mut before, "B", "C");

    for id in ["A", "B"] {
        let mut state = before.clone();

        let got = run(&mut state, &load_test(id), Some(id), T0 + 9);

        // 並ぶだけ: 申し込んだ者の状態・その時刻・理由・印はそのままで、「作業中」へ戻った
        // 出来事も、出し直しの停止要請も無い。
        let mut expected = before.clone();
        queue_load(&mut expected, id, T0 + 9);
        assert_eq!(state, expected, "{id}");
        assert_eq!(got, applied(true, vec![load_requested(id)]), "{id}");
    }
}

/// 止まっている参加者の申し込みが、候補の番を止めない（申し込んだ者を「作業中」へ戻すと、
/// 見張りの終わっている参加者が誰にも知らされずに「停止要請中」へ落ち、番が来なくなる）。
#[test]
fn a_load_request_from_a_stopped_participant_does_not_hold_back_the_candidate() {
    let mut state = state_with(&["A", "B", "C", "X"]);
    run(&mut state, &load_test("C"), Some("C"), T0 + 1);
    run(&mut state, &stopped("A"), Some("A"), T0 + 2);
    run(&mut state, &stopped("B"), Some("B"), T0 + 3);

    // 止まっている B が、自分の負荷テストを申し込む。
    let got = run(&mut state, &load_test("B"), Some("B"), T0 + 4);
    assert_eq!(got, applied(true, vec![load_requested("B")]));
    assert_eq!(seen(&state, "B"), (Stopped, T0 + 3, Some("C"), None));

    // 最後の X が止まった呼び出しで、C に番が来る。B は止まったまま並んでいる。
    let got = run(&mut state, &stopped("X"), Some("X"), T0 + 5);
    let events = vec![stop_reported("X"), load_granted("C", &["A", "B", "X"])];
    assert_eq!(got, applied(true, events));
    assert_eq!(seen(&state, "B"), (Stopped, T0 + 3, Some("C"), None));
    assert_eq!(state.load.queue[0].id, "B");

    // C が済んだら次の候補は B。C が止まった呼び出しで、止まっていた B が番を受けて戻る。
    let got = run(&mut state, &load_done("C"), Some("C"), T0 + 6);
    let events = vec![load_finished("C"), stop_requested("C", "B")];
    assert_eq!(got, applied(true, events));
    let got = run(&mut state, &stopped("C"), Some("C"), T0 + 7);
    let events = vec![
        stop_reported("C"),
        load_granted("B", &["A", "C", "X"]),
        resumed("B", "load-granted"),
    ];
    assert_eq!(got, applied(true, events));
    assert_eq!(seen(&state, "B"), (Working, T0 + 7, None, Some(T0 + 7)));
}

#[test]
fn stopped_from_a_participant_who_was_not_asked_to_stop_changes_nothing() {
    // C は「作業中」の持ち主、A はすでに「止まった」、Z は参加していない。
    for id in ["A", "C", "Z"] {
        let mut state = c_holds_with_a_stopped(&["A", "C"]);
        let before = state.clone();

        let got = run(&mut state, &stopped(id), Some(id), T0 + 10);

        assert_eq!(state, before, "{id}");
        assert_not_applied(&got, id);
    }
}

#[test]
fn recording_a_running_load_test_takes_the_desk_without_asking_anyone_to_stop() {
    // Z は参加していない。記録は参加の申し出を兼ねる。
    let mut state = state_with(&["A", "B"]);
    let mut expected = state.clone();

    let got = run(&mut state, &load_running("Z"), Some("Z"), T0 + 5);

    // A も B も「作業中」のまま、Z が走っている印つきの持ち主になる。
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
    expected.load.holder = Some(LoadHolder {
        id: "Z".to_owned(),
        purpose: PURPOSE.to_owned(),
        requested: T0 + 5,
        granted: T0 + 5,
        running: true,
        stopped: Vec::new(),
    });
    assert_eq!(state, expected);
    let events = vec![Event::Joined { id: "Z".to_owned() }, recorded_running("Z")];
    assert_eq!(got, applied(true, events));
}

#[test]
fn while_the_running_mark_is_on_nobody_is_asked_to_stop_and_no_merge_grant_comes() {
    let mut state = state_with(&["A", "B", "C"]);
    hold_running_load(&mut state, "C");

    // マージの申し込みは並ぶだけで、番は出ない。
    let merge = Command::Merge {
        id: "A".to_owned(),
        name: None,
        repo: "areka".to_owned(),
        spec: "spec-A".to_owned(),
        bug: false,
    };
    let got = run(&mut state, &merge, Some("A"), T0 + 6);
    let requested = Event::MergeRequested {
        repo: "areka".to_owned(),
        id: "A".to_owned(),
        spec: "spec-A".to_owned(),
        bug: false,
    };
    assert_eq!(got, applied(true, vec![requested]));
    assert_eq!(state.merge["areka"].holder, None);

    // 後から参加した D にも停止要請は出ない（印の無い持ち主なら出る）。
    let watch = Command::Watch {
        id: "D".to_owned(),
        name: None,
        repo: REPO.to_owned(),
        pid: PID,
    };
    let got = run(&mut state, &watch, Some("D"), T0 + 7);
    let events = vec![Event::Joined { id: "D".to_owned() }, watch_registered("D")];
    assert_eq!(got, applied(true, events));
    for id in ["A", "B", "C"] {
        assert_eq!(seen(&state, id), (Working, T0, None, None), "{id}");
    }
    assert_eq!(seen(&state, "D"), (Working, T0 + 7, None, None));

    // 持ち主が「済んだ」と言った呼び出しで、待たせていたマージの番が出る。
    let got = run(&mut state, &load_done("C"), Some("C"), T0 + 8);
    let granted = Event::MergeGranted {
        repo: "areka".to_owned(),
        id: "A".to_owned(),
    };
    assert_eq!(got, applied(true, vec![load_finished("C"), granted]));
}

#[test]
fn recording_a_running_load_test_while_the_desk_is_held_changes_nothing() {
    // 持ち主（C）に走っている印が在っても無くても断る。A は止まっている参加者、Z は参加して
    // いない識別。断るときは名前・リポジトリの更新も、参加も、「作業中」への戻しもしない
    // （元のスクリプトは断っても参加させて「作業中」へ戻す）。
    let mut marked = c_holds_with_a_stopped(&["A", "C"]);
    marked.load.holder.as_mut().expect("C が持つ").running = true;
    for (held, label) in [
        (c_holds_with_a_stopped(&["A", "C"]), "plain"),
        (marked, "running"),
    ] {
        for id in ["A", "C", "Z"] {
            let mut state = held.clone();
            let cmd = Command::LoadRunning {
                id: id.to_owned(),
                name: Some("別の名前".to_owned()),
                repo: "pasta".to_owned(),
                purpose: "別の内容".to_owned(),
            };

            let got = run(&mut state, &cmd, Some(id), T0 + 9);

            assert_eq!(state, held, "{label} {id}");
            assert_not_applied(&got, id);
        }
    }
}

#[test]
fn a_queued_requester_who_records_a_running_load_test_leaves_the_queue_and_goes_back_to_work() {
    // A と B が並び、候補は A。B は A のための停止要請中、C は A のために止まっている。
    let mut state = state_with(&["A", "B", "C"]);
    queue_load(&mut state, "A", T0 + 1);
    queue_load(&mut state, "B", T0 + 2);
    ask_to_stop(&mut state, "B", "A");
    stop_for(&mut state, "C", "A");

    let cmd = Command::LoadRunning {
        id: "B".to_owned(),
        name: None,
        repo: REPO.to_owned(),
        purpose: "走っている計測".to_owned(),
    };
    let got = run(&mut state, &cmd, Some("B"), T0 + 9);

    // B は待ち行列から外れて持ち主になり、自分の負荷テストのために「作業中」へ戻る。
    let events = vec![recorded_running("B"), resumed("B", "load-running")];
    assert_eq!(got, applied(true, events));
    let holder = LoadHolder {
        id: "B".to_owned(),
        purpose: "走っている計測".to_owned(),
        requested: T0 + 9,
        granted: T0 + 9,
        running: true,
        stopped: Vec::new(),
    };
    assert_eq!(state.load.holder, Some(holder));
    let queued: Vec<_> = state.load.queue.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(queued, ["A"]);
    assert_eq!(seen(&state, "B"), (Working, T0 + 9, None, Some(T0 + 9)));
    // ほかの参加者は記録の前のまま（新しい停止要請も、再開も無い）。
    assert_eq!(seen(&state, "A"), (Working, T0, None, None));
    assert_eq!(seen(&state, "C"), (Stopped, T0, Some("A"), None));
}

#[test]
fn unstop_puts_the_stop_requested_and_the_stopped_back_to_work() {
    let mut state = c_runs_with_a_stopped_and_b_asked();
    let desk = state.load.clone();

    let got = run(&mut state, &unstop(None), None, T0 + 9);

    // 走っている印の持ち主の間は、取り消した後に停止要請を出し直さない。
    assert_eq!(got, applied(true, vec![unstopped("A"), unstopped("B")]));
    for id in ["A", "B"] {
        let back = (Working, T0 + 9, None, Some(T0 + 9));
        assert_eq!(seen(&state, id), back, "{id}");
    }
    assert_eq!(seen(&state, "C"), (Working, T0, None, None));
    assert_eq!(state.load, desk);
}

#[test]
fn unstop_of_one_identity_leaves_the_others_as_they_were() {
    let mut state = c_runs_with_a_stopped_and_b_asked();

    let got = run(&mut state, &unstop(Some("A")), None, T0 + 9);

    assert_eq!(got, applied(true, vec![unstopped("A")]));
    assert_eq!(seen(&state, "A"), (Working, T0 + 9, None, Some(T0 + 9)));
    assert_eq!(seen(&state, "B"), (StopRequested, T0, Some("C"), None));
}

#[test]
fn unstop_is_followed_by_new_stop_requests_while_a_load_test_without_the_mark_is_wanted() {
    // 待たれているだけのとき。
    let mut wanted = state_with(&["A", "B", "C"]);
    queue_load(&mut wanted, "C", T0 + 1);
    stop_for(&mut wanted, "A", "C");
    ask_to_stop(&mut wanted, "B", "C");
    // 番の来た（印の無い）持ち主が居るとき。B は後から参加して、まだ止まっていない。
    let mut held = c_holds_with_a_stopped(&["A", "B", "C"]);
    ask_to_stop(&mut held, "B", "C");

    for (mut state, label) in [(wanted, "wanted"), (held, "held")] {
        let desk = state.load.clone();

        let got = run(&mut state, &unstop(None), None, T0 + 9);

        // 取り消しと同じ呼び出しの番の決め直しが、停止要請を出し直す。
        let events = vec![
            unstopped("A"),
            unstopped("B"),
            stop_requested("A", "C"),
            stop_requested("B", "C"),
        ];
        assert_eq!(got, applied(true, events), "{label}");
        for id in ["A", "B"] {
            let asked_again = (StopRequested, T0 + 9, Some("C"), Some(T0 + 9));
            assert_eq!(seen(&state, id), asked_again, "{label} {id}");
        }
        assert_eq!(state.load, desk, "{label}");
    }
}

#[test]
fn unstop_with_nobody_to_put_back_changes_nothing() {
    // 全員が「作業中」／名指しした C は「作業中」の持ち主／Z は参加していない。
    for (before, id) in [
        (state_with(&["A", "B"]), None),
        (c_holds_with_a_stopped(&["A", "C"]), Some("C")),
        (c_holds_with_a_stopped(&["A", "C"]), Some("Z")),
    ] {
        let mut state = before.clone();

        let got = run(&mut state, &unstop(id), None, T0 + 9);

        assert_eq!(state, before, "{id:?}");
        assert_not_applied(&got, "unstop");
    }
}
