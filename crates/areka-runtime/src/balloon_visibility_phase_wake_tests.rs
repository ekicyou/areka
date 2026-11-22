//! 待ち時間の預け方（[`visibility_wake`]）の決定論テスト。
//!
//! 見るのは引き算だけの純判断 1 つで、時計にも表示層にも触れない。相の側の配線
//! （どこで呼ぶか）は親の `balloon_visibility_phase_tests.rs` と字面検査が受け持つ。

use super::super::BalloonVisibilityState;
use super::super::test_support::{Frame, hovered, seen};
use super::{MAX_WAIT_SECS, VisibilityWake, visibility_wake};
use std::time::Duration;

/// 計測が動いていないフレームでは何も預けない（起こす相手が居ない）。
#[test]
fn no_measurement_arms_nothing() {
    assert_eq!(
        visibility_wake(Some(1.0), None, false),
        VisibilityWake::None,
        "満了予定が無ければ預けない"
    );
    assert_eq!(
        visibility_wake(None, Some(5.0), false),
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
        visibility_wake(Some(1.5), Some(4.0), false),
        VisibilityWake::After(Duration::from_secs_f64(2.5)),
        "残り 2.5 秒"
    );
    assert_eq!(
        visibility_wake(Some(0.0), Some(0.001), false),
        VisibilityWake::After(Duration::from_secs_f64(0.001)),
        "1 ミリ秒でも残っていれば期限として預ける"
    );
}

/// 期限が既に来ていて抑止も無いなら、次の画面更新で見直す。
#[test]
fn due_or_overdue_asks_for_the_next_frame() {
    assert_eq!(
        visibility_wake(Some(4.0), Some(4.0), false),
        VisibilityWake::Now,
        "ちょうど到来は「来ている」側（判断中核の `now >= deadline` と同じ向き）"
    );
    assert_eq!(
        visibility_wake(Some(9.0), Some(4.0), false),
        VisibilityWake::Now,
        "超過は次の画面更新で見直す"
    );
}

/// 数にならない期限は預けない——満了の比較そのものが成立せず、起こしても答えが変わらない。
#[test]
fn non_finite_deadline_arms_nothing() {
    assert_eq!(
        visibility_wake(Some(1.0), Some(f64::INFINITY), false),
        VisibilityWake::None,
        "無限の期限は永久に満了しない"
    );
    assert_eq!(
        visibility_wake(Some(1.0), Some(f64::NAN), false),
        VisibilityWake::None,
        "非数の期限はどちらの比較も偽になる"
    );
    assert_eq!(
        visibility_wake(Some(f64::NAN), Some(4.0), false),
        VisibilityWake::None,
        "現在時刻が非数でも同じ"
    );
}

/// 桁外れに遠い期限は頭打ちにする（時間の長さへ直すときに桁があふれない）。
#[test]
fn absurdly_distant_deadline_is_capped() {
    assert_eq!(
        visibility_wake(Some(0.0), Some(1.0e30), false),
        VisibilityWake::After(Duration::from_secs_f64(MAX_WAIT_SECS)),
        "遠すぎる期限は 1 時間で切って預け直す"
    );
    assert_eq!(MAX_WAIT_SECS, 3600.0, "頭打ちの長さ");
}

/// 抑止中の超過は何も預けない（抑止の解けは入力が起こす）。期限が先なら残りを預ける。
#[test]
fn overdue_while_suppressed_arms_nothing() {
    assert_eq!(
        visibility_wake(Some(9.0), Some(4.0), true),
        VisibilityWake::None,
        "抑止中の超過"
    );
    assert_eq!(
        visibility_wake(Some(4.0), Some(4.0), true),
        VisibilityWake::None,
        "抑止中のちょうど到来"
    );
    assert_eq!(
        visibility_wake(Some(9.0), Some(4.0), false),
        VisibilityWake::Now,
        "抑止が無い超過は今までどおり"
    );
    assert_eq!(
        visibility_wake(Some(1.0), Some(4.0), true),
        VisibilityWake::After(Duration::from_secs_f64(3.0)),
        "抑止中でも期限が先なら残りを預ける"
    );
}

/// 満了の後もポインタがバルーンの上に残る間は起こさず、外れたフレームで計り直しの期限を預ける。
#[test]
fn a_held_timeout_with_the_pointer_still_on_the_balloon_asks_for_no_wake() {
    const TIMEOUT: f64 = 30.0;
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[(0, seen(2, false))])
        .at(0.0)
        .talk_started()
        .ended_at(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);

    Frame::new(&[(0, hovered(seen(2, true)))])
        .at(TIMEOUT + 1.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        visibility_wake(Some(TIMEOUT + 1.0), state.deadline, state.prev_suppressed),
        VisibilityWake::None,
        "満了を過ぎてもポインタが残っている間は預けない"
    );

    Frame::new(&[(0, seen(2, true))])
        .at(TIMEOUT + 2.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        visibility_wake(Some(TIMEOUT + 2.0), state.deadline, state.prev_suppressed),
        VisibilityWake::After(Duration::from_secs_f64(TIMEOUT)),
        "外れたフレームで計り直した期限を預ける"
    );
}
