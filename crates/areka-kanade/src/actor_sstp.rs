//! 殻の SSTP の 2 つの入口の返事（返事の控え・最初の往復の写し・返事の組み立てと送り）。
//!
//! 振り分けの腕が運行表へ渡す前の状態で受け付けの判定と返信端を控え（[`SstpReply`]）、一括の処理の
//! 後に [`answer`] で 1 回だけ送る。最初の往復の写し（[`FirstReply`]）は汎用の通知の入口の返事
//! （[`RaiseOutcome`]）にも使う。

use areka_actor::ReplySender;

use crate::change::RaiseOutcome;
use crate::msg::ShioriOutcome;
use crate::sstp::{SstpNotifyOutcome, SstpSendOutcome};

/// 最初の一括の往復の結果（`ShioriOutcome` は複製できないので殻が写す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FirstReply {
    Script(String),
    NoReply,
    Failed,
}

/// 空でない（trim して 1 文字以上の）`Value` → Script、空・空白だけの `Value`・`NoContent`・
/// `Notified`・`Unloaded` → NoReply、`Failed` → Failed。
///
/// エラー応答は送出点（`round_trip_request`）で既に 204／NOTIFY の完了へ写っているので返事なしになる。
/// `Unloaded` は通知の入口の往復からは生じないが、写しの表を閉じるため返事なしに置く。
pub(crate) fn first_reply_of(outcome: &ShioriOutcome) -> FirstReply {
    match outcome {
        ShioriOutcome::Value(script) if !script.trim().is_empty() => {
            FirstReply::Script(script.clone())
        }
        ShioriOutcome::Value(_)
        | ShioriOutcome::NoContent
        | ShioriOutcome::Notified
        | ShioriOutcome::Unloaded => FirstReply::NoReply,
        ShioriOutcome::Failed(_) => FirstReply::Failed,
    }
}

impl FirstReply {
    /// `RaiseEvent` の返事への写し（空でない台本 → Script・返事なし → NoReply・失敗 → Failed）。
    pub(crate) fn raise_outcome(&self) -> RaiseOutcome {
        match self {
            FirstReply::Script(_) => RaiseOutcome::Script,
            FirstReply::NoReply => RaiseOutcome::NoReply,
            FirstReply::Failed => RaiseOutcome::Failed,
        }
    }
}

/// 外から頼まれたイベントの「受け付けた」の控え: 普段の状態（運行表の腕と同じ
/// [`crate::schedule::sstp::accepts`]）で、かつ名前と Reference が検査を満たすときだけ真。
///
/// 検査を破った知らせは運行表の腕が `warn!` 1 件だけ残して送らないので、受け付けなかったと控え、
/// 殻の「受け付けたのに往復が無い」の `warn!` を重ねない。
pub(crate) fn notify_accepted(
    state: &crate::schedule::State,
    event: &str,
    references: &[String],
) -> bool {
    crate::schedule::sstp::accepts(state) && crate::sstp::validate_notify(event, references).is_ok()
}

/// 返事の控え（運行表へ渡す前の状態での受け付けの判定と返信端）。
pub(crate) enum SstpReply {
    Send {
        reply: ReplySender<SstpSendOutcome>,
        accepted: bool,
    },
    Notify {
        reply: ReplySender<SstpNotifyOutcome>,
        accepted: bool,
    },
}

/// 一括の処理の後に返事を組んで 1 回だけ送る。`stopped` は一括の処理が止まりで終わったこと。
///
/// 受け手が居ない（待ちの上限で先に答えた）のは異常ではないので `debug!` だけで運行を続ける。
pub(crate) fn answer(pending: SstpReply, stopped: bool, first: Option<&FirstReply>) {
    let sent = match pending {
        SstpReply::Send { reply, accepted } => {
            let outcome = match (stopped, accepted) {
                (true, _) => SstpSendOutcome::Failed,
                (false, true) => SstpSendOutcome::Accepted,
                (false, false) => SstpSendOutcome::NotAccepted,
            };
            reply.send(outcome).is_ok()
        }
        SstpReply::Notify { reply, accepted } => {
            let outcome = match (stopped, accepted, first) {
                (true, _, _) => SstpNotifyOutcome::Failed,
                (false, false, _) => SstpNotifyOutcome::NotAccepted,
                (false, true, Some(FirstReply::Script(s))) => SstpNotifyOutcome::Script(s.clone()),
                (false, true, Some(FirstReply::NoReply)) => SstpNotifyOutcome::NoScript,
                (false, true, Some(FirstReply::Failed)) => SstpNotifyOutcome::Failed,
                (false, true, None) => {
                    // 受け付けたなら運行表は必ず GET を 1 回出すので、構造上は起きない。
                    tracing::warn!(target: "kanade", event = "sstp_notify_no_round_trip", "受け付けた外からのイベントに往復が無い——受け付けなかったと答える");
                    SstpNotifyOutcome::NotAccepted
                }
            };
            reply.send(outcome).is_ok()
        }
    };
    if !sent {
        tracing::debug!(target: "kanade", event = "sstp_reply_dropped", "SSTP の結果の受け手が居ない——返事は捨てて運行を続ける");
    }
}
