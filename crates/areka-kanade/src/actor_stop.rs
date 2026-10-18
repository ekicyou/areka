//! 殻の停止と通知の手（`actor.rs` から移した 4 関数・中身は変えていない）。
//!
//! 終了系列の原因の写し（[`stop_cause_of`]）・切替の中身の写し（[`handoff_of`]）・停止通知の投函
//! （[`notify_stop`]）・運行の通知の送出（[`send_notice`]）。殻（`actor.rs`）は同じ名前で使う。

use std::sync::mpsc::Sender;

use crate::change::{ChangeHandoff, KanadeNotice};
use crate::msg::{KanadeStopCause, KanadeStopped, ShioriFault};
use crate::schedule::{Phase, State, TermCause};

/// 運行状態が終了系列（`Unloading{cause}`）に居るなら、その原因を公開語彙へ写す（R15.3）。
/// 運行状態は手放さないので、原因は clone して写す（Fault は種類と理由ごと）。
///
/// 写すのはここ 1 箇所だけである——内部の `TermCause` は `pub(crate)` に閉じたままで、公開面へ
/// 出るのは [`KanadeStopCause`] の 5 値だけになる（DD-9 の露出規律）。終了系列でなければ `None`。
///
/// wildcard を置かないため、`TermCause` に値が増えたときは本表での判断がコンパイル時に要求される。
pub(crate) fn stop_cause_of(state: &State) -> Option<KanadeStopCause> {
    let Phase::Unloading { cause } = &state.phase else {
        return None;
    };
    Some(match cause {
        TermCause::Quit => KanadeStopCause::Quit,
        TermCause::Forced => KanadeStopCause::Forced,
        TermCause::CloseSilent => KanadeStopCause::CloseSilent,
        TermCause::DeadlineExceeded => KanadeStopCause::DeadlineExceeded,
        TermCause::Fault(fault) => KanadeStopCause::Fault(fault.clone()),
    })
}

/// 運行状態の切替の帳簿を、停止通知に載せる切替の中身へ写す（切替の相を経ていなければ `None`）。
pub(crate) fn handoff_of(state: &State) -> Option<ChangeHandoff> {
    state.change.as_ref().map(|change| ChangeHandoff {
        script: change.script.clone(),
    })
}

/// 停止通知を投函する（R15.3・design D15 の 2）。
///
/// `sink` が `None` なら何もしない（通知端を結線していない構成＝既存の呼び手はすべてこちら）。
/// 送出失敗は受信端（UI）が既に落ちていることを意味するだけで異常ではないが、**沈黙で捨てる
/// 経路を作らない**（steering: areka-log-first-no-silent-failure）ため `warn!` を 1 件残す。
/// panic はしない——ここで落ちると終了系列そのものが完走しなくなる。
///
/// `cause` が `None` になるのは、`Unloading` を経ずに `StopSelf` が現れた場合だけである
/// （現在の運行表には存在しない経路）。その場合も通知は出す——窓を閉じる合図としての意味は
/// 原因に依らないためで、原因不明であることは記録に残し、種類 `Unknown` の Fault として送る。
pub(crate) fn notify_stop(
    sink: Option<&Sender<KanadeNotice>>,
    cause: Option<KanadeStopCause>,
    handoff: Option<ChangeHandoff>,
) {
    let Some(tx) = sink else {
        return;
    };
    if cause.is_none() {
        tracing::warn!(
            target: "kanade",
            event = "stop_cause_unknown",
            "終了系列の原因を控えられないまま StopSelf に至った——Fault として通知する"
        );
    }
    let cause = cause.unwrap_or_else(|| KanadeStopCause::Fault(ShioriFault::unknown()));
    // 送出に失敗した値は SendError に載って戻るので、記録にはそれを使う（clone しない）。
    let stopped = KanadeStopped { cause, handoff };
    // 戻ってくるのは送った「停止」そのものなので、腕は 1 つで足りる。
    if let Err(std::sync::mpsc::SendError(KanadeNotice::Stopped(unsent))) =
        tx.send(KanadeNotice::Stopped(stopped))
    {
        tracing::warn!(
            target: "kanade",
            event = "stop_notify_failed",
            cause = ?unsent.cause,
            "停止通知の送出に失敗（受信端＝UI は既に切断）——停止は完走する"
        );
    }
}

/// 運行表の「通知を送る」（[`Action::Notice`](crate::schedule::Action::Notice)）を送出端へ流す。
///
/// 送出端が無い構成（既存の呼び手）では `debug!` を 1 件残すだけ。受け口が消えていれば
/// `warn!`（`event="notice_send_failed"`）を残して続ける（panic しない・運行は止めない）。
pub(crate) fn send_notice(sink: Option<&Sender<KanadeNotice>>, notice: KanadeNotice) {
    let Some(tx) = sink else {
        tracing::debug!(
            target: "kanade",
            event = "notice_no_sink",
            notice = ?notice,
            "運行の通知の送出端が無い——通知は送らない"
        );
        return;
    };
    if let Err(std::sync::mpsc::SendError(unsent)) = tx.send(notice) {
        tracing::warn!(
            target: "kanade",
            event = "notice_send_failed",
            notice = ?unsent,
            "運行の通知の送出に失敗（受信端＝UI は既に切断）——運行は続ける"
        );
    }
}
