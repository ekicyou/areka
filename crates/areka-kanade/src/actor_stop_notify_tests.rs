//! 停止通知（[`KanadeStopped`]）の発行点の檻（areka-P0-emo2-conformance-e2e タスク 6.9・R15.3）。
//!
//! 検証するのは 3 つである:
//!
//! 1. **原因の写し**——`Unloading{cause}` の 5 値がそのまま公開語彙 [`KanadeStopCause`] へ写る。
//!    wildcard 無しの写像ゆえ、内部語彙が増えたときは実装がコンパイルで止まる。
//! 2. **投函**——通知端が生きていれば運行の通知の「停止」（`KanadeNotice::Stopped`）がちょうど
//!    1 件届く。切替の中身（`handoff`）は、止まる直前の運行状態の切替の帳簿から写す。
//! 3. **切断**——受信端が落ちていれば `warn!`（`event="stop_notify_failed"`）を 1 件残して
//!    そのまま返る（panic しない・log-first）。
//!
//! いずれも呼出スレッドで同期に走る素の関数の観測であり、アクタースレッドを跨がない。ゆえに
//! スレッドローカルの捕捉窓（`schedule::log_capture`）で足り、全スレッド捕捉の窓口は要らない。

use std::sync::mpsc;

use tracing::Level;

use super::{Drive, drive, execute_actions, notify_stop, stop_cause_of};
use crate::change::{
    CancelReason, ChangeHandoff, ChangeOrigin, ChangeRequest, ChangeTarget, KanadeNotice,
};
use crate::msg::{
    KanadeConfig, KanadeStopCause, KanadeStopped, MonotonicMs, ShioriFault, ShioriFaultKind,
    ShioriMsg, ShioriOutcome,
};
use crate::schedule::change::ChangeState;
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
use crate::schedule::{Action, Input, Phase, State, TermCause};
use crate::talk::TalkCommand;

/// 停止の通知（切替の相を経ていない停止＝切替の中身は無し）。
fn stopped(cause: KanadeStopCause) -> KanadeNotice {
    KanadeNotice::Stopped(KanadeStopped {
        cause,
        handoff: None,
    })
}

/// 指定 phase の運行状態を組む（`State::initial()` の phase だけを差し替える）。
fn state_in(phase: Phase) -> State {
    let mut state = State::initial();
    state.phase = phase;
    state.last_now = Some(MonotonicMs(1_000));
    state
}

/// `Unloading{cause}` の 5 値が公開語彙へ 1 対 1 で写る（R15.3 の「原因を問わず」）。
/// Fault は種類と理由をそのまま運ぶ（要件 2.1）。
#[test]
fn unloading_cause_maps_to_the_public_vocabulary_for_all_five_values() {
    let fault = ShioriFault {
        kind: ShioriFaultKind::ConnectFailed,
        reason: "connect refused".to_string(),
    };
    let pairs = [
        (TermCause::Quit, KanadeStopCause::Quit),
        (TermCause::Forced, KanadeStopCause::Forced),
        (TermCause::CloseSilent, KanadeStopCause::CloseSilent),
        (
            TermCause::DeadlineExceeded,
            KanadeStopCause::DeadlineExceeded,
        ),
        (
            TermCause::Fault(fault.clone()),
            KanadeStopCause::Fault(fault),
        ),
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
            reason: crate::msg::CloseReason::User { scope: 0 },
        },
    ] {
        assert_eq!(
            stop_cause_of(&state_in(phase)),
            None,
            "終了系列の外では原因を控えない"
        );
    }
}

/// 通知端が生きていれば `KanadeNotice::Stopped` がちょうど 1 件届き、警告は出ない（R15.3）。
#[test]
fn notify_stop_delivers_exactly_one_message_and_logs_nothing() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();

    let events = capture(|| notify_stop(Some(&tx), Some(KanadeStopCause::Quit), None));

    assert_eq!(
        rx.try_recv(),
        Ok(stopped(KanadeStopCause::Quit)),
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
    let events = capture(|| notify_stop(None, Some(KanadeStopCause::Forced), None));
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
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    drop(rx);

    let events = capture(|| notify_stop(Some(&tx), Some(KanadeStopCause::DeadlineExceeded), None));

    let warned = logged_once(&events, Level::WARN, "stop_notify_failed");
    assert_eq!(
        warned.fields.get("cause").map(String::as_str),
        Some("DeadlineExceeded"),
        "警告は原因を載せる: {warned:#?}"
    );
}

/// 原因を控えられないまま止まったときは、原因不明の Fault を送り `warn!` を 1 件残す（要件 2.2）。
#[test]
fn notify_stop_without_a_cause_sends_an_unknown_fault() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();

    let events = capture(|| notify_stop(Some(&tx), None, None));

    assert_eq!(
        rx.try_recv(),
        Ok(stopped(KanadeStopCause::Fault(ShioriFault::unknown()))),
        "原因不明は種類 Unknown の Fault として届く"
    );
    logged_once(&events, Level::WARN, "stop_cause_unknown");
}

/// `Action::Notice` を 1 つだけ載せたバッチを実行する（SHIORI・再生へは何も出ない）。
fn run_notice(sink: Option<&mpsc::Sender<KanadeNotice>>, notice: KanadeNotice) -> bool {
    let (shiori_tx, _shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();
    let result = execute_actions(
        vec![Action::Notice(notice)],
        &shiori_tx,
        &sakura_tx,
        &(Box::new(|_, _| {}) as crate::schedule::resources::ResourceSink),
        sink,
        (None, None),
    );
    assert!(
        result.last_reply.is_none(),
        "通知は SHIORI 往復を起こさない"
    );
    result.stop
}

/// 運行表の「通知を送る」は、そのままの値で送出端へ 1 件流れ、運行は止まらない。
#[test]
fn notice_action_is_forwarded_to_the_sink_as_is() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    let cancelled = KanadeNotice::ChangeCancelled {
        reason: CancelReason::UserBreak,
    };

    let events = capture(|| {
        assert!(
            !run_notice(Some(&tx), cancelled.clone()),
            "通知は停止ではない"
        );
    });

    assert_eq!(rx.try_recv(), Ok(cancelled), "通知はそのままの値で届く");
    assert!(rx.try_recv().is_err(), "通知は 1 件だけ");
    assert_not_logged(&events, "notice_send_failed");
}

/// 受け口が消えていれば `warn!(notice_send_failed)` を 1 件残して続ける（panic しない）。
#[test]
fn notice_send_failure_warns_once_and_continues() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    drop(rx);

    let events = capture(|| {
        assert!(!run_notice(Some(&tx), KanadeNotice::Steady));
    });

    logged_once(&events, Level::WARN, "notice_send_failed");
}

/// 送出端を結線していない構成では `debug!` を 1 件残すだけ（黙って捨てない）。
#[test]
fn notice_without_a_sink_is_recorded_at_debug() {
    let events = capture(|| {
        assert!(!run_notice(None, KanadeNotice::Steady));
    });

    logged_once(&events, Level::DEBUG, "notice_no_sink");
    assert_not_logged(&events, "notice_send_failed");
}

/// 運行表の「通知を送る」の定常到達も、そのままの値で送出端へ 1 件流れる（要件 8.8）。
#[test]
fn steady_notice_is_forwarded_to_the_sink() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();

    assert!(
        !run_notice(Some(&tx), KanadeNotice::Steady),
        "通知は停止ではない"
    );

    assert_eq!(rx.try_recv(), Ok(KanadeNotice::Steady), "定常到達が届く");
    assert!(rx.try_recv().is_err(), "通知は 1 件だけ");
}

/// 切替の帳簿を持ったまま終了系列を終えると、停止通知に切替の中身（台本ごと）が載る（要件 3.4）。
///
/// 控えは `drive` が停止原因を控えるのと同じ時点（step の直前）で取る。Unload の応答で
/// `Stopped` へ移った後の停止の実行点まで、その控えが運ばれることを実際の駆動で確かめる。
#[test]
fn stop_notice_carries_the_change_handoff_taken_from_the_state() {
    let (tx, rx) = mpsc::channel::<KanadeNotice>();
    let (shiori_tx, _shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();
    let mut state = state_in(Phase::Unloading {
        cause: TermCause::CloseSilent,
    });
    state.change = Some(ChangeState {
        req: ChangeRequest {
            target: ChangeTarget {
                sakura_name: "B".to_string(),
                name: "ghost B".to_string(),
                dir: "C:/ghost/B".to_string(),
            },
            origin: ChangeOrigin::Manual,
            raise_event: true,
        },
        script: Some(r"\0またね\e".to_string()),
    });

    let outcome = drive(
        &mut state,
        Input::ShioriReply {
            outcome: ShioriOutcome::Unloaded,
            origin: "Unload",
        },
        &KanadeConfig::new("master", "1.0.0"),
        &shiori_tx,
        &sakura_tx,
        &(Box::new(|_, _| {}) as crate::schedule::resources::ResourceSink),
        Some(&tx),
    );

    assert!(matches!(outcome, Drive::Stop), "Unload の応答で停止する");
    assert_eq!(
        rx.try_recv(),
        Ok(KanadeNotice::Stopped(KanadeStopped {
            cause: KanadeStopCause::CloseSilent,
            handoff: Some(ChangeHandoff {
                script: Some(r"\0またね\e".to_string()),
            }),
        })),
        "停止通知は原因と切替の中身を運ぶ"
    );
    assert!(rx.try_recv().is_err(), "停止通知は 1 件だけ");
}
