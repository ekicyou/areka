//! 停止通知（[`KanadeStopped`]）の発行点の檻（areka-P0-emo2-conformance-e2e タスク 6.9・R15.3）。
//!
//! 検証するのは 3 つである:
//!
//! 1. **原因の写し**——`Unloading{cause}` の 5 値がそのまま公開語彙 [`KanadeStopCause`] へ写る。
//!    wildcard 無しの写像ゆえ、内部語彙が増えたときは実装がコンパイルで止まる。
//! 2. **投函**——通知端が生きていれば `KanadeStopped{cause}` がちょうど 1 件届く。
//! 3. **切断**——受信端が落ちていれば `warn!`（`event="stop_notify_failed"`）を 1 件残して
//!    そのまま返る（panic しない・log-first）。
//!
//! いずれも呼出スレッドで同期に走る素の関数の観測であり、アクタースレッドを跨がない。ゆえに
//! スレッドローカルの捕捉窓（`schedule::log_capture`）で足り、全スレッド捕捉の窓口は要らない。

use std::sync::mpsc;

use tracing::Level;

use super::{notify_stop, stop_cause_of};
use crate::msg::{KanadeStopCause, KanadeStopped, MonotonicMs};
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
use crate::schedule::{Phase, State, TermCause};

/// 指定 phase の運行状態を組む（`State::initial()` の phase だけを差し替える）。
fn state_in(phase: Phase) -> State {
    let mut state = State::initial();
    state.phase = phase;
    state.last_now = Some(MonotonicMs(1_000));
    state
}

/// `Unloading{cause}` の 5 値が公開語彙へ 1 対 1 で写る（R15.3 の「原因を問わず」）。
#[test]
fn unloading_cause_maps_to_the_public_vocabulary_for_all_five_values() {
    let pairs = [
        (TermCause::Quit, KanadeStopCause::Quit),
        (TermCause::Forced, KanadeStopCause::Forced),
        (TermCause::CloseSilent, KanadeStopCause::CloseSilent),
        (
            TermCause::DeadlineExceeded,
            KanadeStopCause::DeadlineExceeded,
        ),
        (TermCause::Fault, KanadeStopCause::Fault),
    ];
    for (internal, public) in pairs {
        let state = state_in(Phase::Unloading { cause: internal });
        assert_eq!(
            stop_cause_of(&state),
            Some(public),
            "Unloading の原因は公開語彙へそのまま写る"
        );
    }
}

/// 終了系列でない運行状態からは原因を控えない（`None`）。
#[test]
fn a_state_outside_the_termination_sequence_carries_no_cause() {
    for phase in [
        Phase::Idle,
        Phase::Steady { talk: None },
        Phase::Stopped,
        Phase::ClosePending {
            reason: crate::msg::CloseReason::User,
        },
    ] {
        assert_eq!(
            stop_cause_of(&state_in(phase)),
            None,
            "終了系列の外では原因を控えない"
        );
    }
}

/// 通知端が生きていれば `KanadeStopped{cause}` がちょうど 1 件届き、警告は出ない（R15.3）。
#[test]
fn notify_stop_delivers_exactly_one_message_and_logs_nothing() {
    let (tx, rx) = mpsc::channel::<KanadeStopped>();

    let events = capture(|| notify_stop(Some(&tx), Some(KanadeStopCause::Quit)));

    assert_eq!(
        rx.try_recv(),
        Ok(KanadeStopped {
            cause: KanadeStopCause::Quit
        }),
        "停止通知はちょうど 1 件・原因つきで届く"
    );
    assert!(
        rx.try_recv().is_err(),
        "停止通知は 1 件だけ（2 件目は送らない）"
    );
    assert_not_logged(&events, "stop_notify_failed");
    assert_not_logged(&events, "stop_cause_unknown");
}

/// 通知端が無い構成（既存の呼び手）では何も送らず何も記録しない。
#[test]
fn notify_stop_without_a_sink_is_a_silent_no_op() {
    let events = capture(|| notify_stop(None, Some(KanadeStopCause::Forced)));
    assert!(
        events.is_empty(),
        "通知端が無い構成では記録も発生しない: {events:#?}"
    );
}

/// 受信端が落ちていれば `warn!` 1 件（`event="stop_notify_failed"`・原因つき）を残して返る。
///
/// panic しないことが要点である——ここで落ちると終了系列そのものが完走しなくなる（R15.3）。
#[test]
fn notify_stop_warns_once_when_the_receiver_is_gone_and_does_not_panic() {
    let (tx, rx) = mpsc::channel::<KanadeStopped>();
    drop(rx);

    let events = capture(|| notify_stop(Some(&tx), Some(KanadeStopCause::DeadlineExceeded)));

    let warned = logged_once(&events, Level::WARN, "stop_notify_failed");
    assert_eq!(
        warned.fields.get("cause").map(String::as_str),
        Some("DeadlineExceeded"),
        "警告は原因を載せる: {warned:#?}"
    );
}
