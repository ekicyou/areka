//! `plan::turns` のテスト: 申し込み・返し・番の決め直し（一度に渡す・追い越し・持ちながら・
//! 返し方・固定点）。支えは親の `plan_test_support.rs` を読むだけ。

use super::super::test_support::{T0, request, run, state_with, stop_for};
use super::replan;
use crate::plan::Event;
use crate::state::DeskSet;

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
