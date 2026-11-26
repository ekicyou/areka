//! 待ち時間の預け方（[`visibility_wake`]）の決定論テスト。
//!
//! 見るのは引き算だけの純判断 1 つで、時計にも表示層にも触れない。相から預ける呼出が在ることは
//! `crates/areka/src/tick_gate_config_producers_tests.rs` の `balloon_visibility_phase_arms_the_deadline` が見る。

use super::super::BalloonVisibilityState;
use super::super::test_support::{Frame, TIMEOUT, hovered, ms, seen};
use super::{MAX_WAIT, VisibilityWake, visibility_wake};
use std::time::Duration;

/// 計測が動いていないフレームでは何も預けない（起こす相手が居ない）。
#[test]
fn no_measurement_arms_nothing() {
    assert_eq!(
        visibility_wake(Some(ms(1000)), None, false),
        VisibilityWake::None,
        "満了予定が無ければ預けない"
    );
    assert_eq!(
        visibility_wake(None, Some(ms(5000)), false),
        VisibilityWake::None,
        "現在時刻が分からなければ残り時間を出せない"
    );
    assert_eq!(
        visibility_wake(None, None, false),
        VisibilityWake::None,
        "どちらも無ければ預けない"
    );
}

/// 待ち時間が残っていれば、その長さだけ後に起こす。
#[test]
fn remaining_time_becomes_the_wait() {
    assert_eq!(
        visibility_wake(Some(ms(1500)), Some(ms(4000)), false),
        VisibilityWake::After(Duration::from_millis(2500)),
        "残り 2.5 秒"
    );
    assert_eq!(
        visibility_wake(Some(ms(0)), Some(ms(1)), false),
        VisibilityWake::After(Duration::from_millis(1)),
        "1 ミリ秒でも残っていれば期限として預ける"
    );
}

/// 期限が既に来ていて抑止も無いなら、次の画面更新で見直す。
#[test]
fn due_or_overdue_asks_for_the_next_frame() {
    assert_eq!(
        visibility_wake(Some(ms(4000)), Some(ms(4000)), false),
        VisibilityWake::Now,
        "ちょうど到来は「来ている」側（判断中核の `now >= deadline` と同じ向き）"
    );
    assert_eq!(
        visibility_wake(Some(ms(9000)), Some(ms(4000)), false),
        VisibilityWake::Now,
        "超過は次の画面更新で見直す"
    );
}

/// 桁外れに遠い期限は頭打ちにする（時間の長さへ直すときに桁があふれない）。
#[test]
fn absurdly_distant_deadline_is_capped() {
    assert_eq!(
        visibility_wake(Some(ms(0)), Some(ms(u32::MAX)), false),
        VisibilityWake::After(MAX_WAIT),
        "遠すぎる期限は 1 時間で切って預け直す"
    );
    assert_eq!(MAX_WAIT, Duration::from_secs(3600), "頭打ちの長さ");
}

/// 抑止中の超過は何も預けない（抑止の解けは入力が起こす）。期限が先なら残りを預ける。
#[test]
fn overdue_while_suppressed_arms_nothing() {
    assert_eq!(
        visibility_wake(Some(ms(9000)), Some(ms(4000)), true),
        VisibilityWake::None,
        "抑止中の超過"
    );
    assert_eq!(
        visibility_wake(Some(ms(4000)), Some(ms(4000)), true),
        VisibilityWake::None,
        "抑止中のちょうど到来"
    );
    assert_eq!(
        visibility_wake(Some(ms(1000)), Some(ms(4000)), true),
        VisibilityWake::After(Duration::from_millis(3000)),
        "抑止中でも期限が先なら残りを預ける"
    );
}

/// 満了の後もポインタがバルーンの上に残る間は起こさず、外れたフレームで計り直しの期限を預ける。
#[test]
fn a_held_timeout_with_the_pointer_still_on_the_balloon_asks_for_no_wake() {
    let after_timeout = |by: u32| TIMEOUT.checked_add(ms(by)).expect("範囲内");
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(2, false))])
        .at(ms(0))
        .talk_started()
        .ended_at(ms(0))
        .run(&mut state);

    Frame::new(&[(0, hovered(seen(2, true)))])
        .at(after_timeout(1000))
        .run(&mut state);
    assert_eq!(
        visibility_wake(
            Some(after_timeout(1000)),
            state.deadline,
            state.prev_suppressed
        ),
        VisibilityWake::None,
        "満了を過ぎてもポインタが残っている間は預けない"
    );

    Frame::new(&[(0, seen(2, true))])
        .at(after_timeout(2000))
        .run(&mut state);
    assert_eq!(
        visibility_wake(
            Some(after_timeout(2000)),
            state.deadline,
            state.prev_suppressed
        ),
        VisibilityWake::After(TIMEOUT.to_std()),
        "外れたフレームで計り直した期限を預ける"
    );
}
