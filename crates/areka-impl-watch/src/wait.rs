//! 長い待ち（見張り・マージの番・負荷テストの番・再開）の終わりの判定。
//!
//! [`judge`] は状態を見て「終わるか・まだ待つか」を決めるだけで、ファイルも時計もプロセスも
//! ロックも触らない。終わりの 1 行に使うのは識別・リポジトリ・spec・PR・sha・時刻だけで、
//! 名前と内容（日本語になりうる値）は使わない。このファイルの文字列リテラルは全部 ASCII。

// 使い手（待ちのループ・cli）が載るまで、本番のビルドではここが未使用になる。
// 「満たされない expect」の警告が出たら外す。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "the users of this module arrive in later tasks")
)]

use crate::error::escape_path;
use crate::state::{ParticipantStatus, State};
use crate::status::last_merge;

/// 「消えた」の理由: 参加者の記録が無い（離脱・マージ済み・回収・`clear`）。
const REMOVED: &str = "removed";
/// 「消えた」の理由: 申し込みが待ち行列にも持ち主にも無い（取り下げ・離脱・回収・`clear`）。
const REQUEST_GONE: &str = "request gone";

/// 何を待つか。
#[derive(Debug)]
pub enum WaitSpec {
    /// 見張り。`repo` と `name` は見張りの開始（参加）の登録に使い、判定には使わない。
    Watch {
        id: String,
        #[cfg_attr(
            test,
            expect(dead_code, reason = "read by the wait loop, not by judge")
        )]
        repo: String,
        #[cfg_attr(
            test,
            expect(dead_code, reason = "read by the wait loop, not by judge")
        )]
        name: Option<String>,
    },
    /// そのリポジトリのマージの番。
    Merge { id: String, repo: String },
    /// 負荷テストの番。
    Load { id: String },
    /// 再開（「作業中」へ戻る）。
    Resume { id: String },
}

impl WaitSpec {
    fn id(&self) -> &str {
        match self {
            WaitSpec::Watch { id, .. }
            | WaitSpec::Merge { id, .. }
            | WaitSpec::Load { id }
            | WaitSpec::Resume { id } => id,
        }
    }

    /// 参加者の記録が無いときの「消えた」の理由。
    fn gone(&self) -> &'static str {
        match self {
            WaitSpec::Watch { .. } | WaitSpec::Resume { .. } => REMOVED,
            WaitSpec::Merge { .. } | WaitSpec::Load { .. } => REQUEST_GONE,
        }
    }
}

/// 待ちの終わり方。
#[derive(Debug, PartialEq, Eq)]
pub enum WaitEnd {
    /// 番が来た・停止要請が出た・再開した（終了コード 0）。中身は端末へ出す ASCII の 1 行。
    Done(String),
    /// 記録・申し込みが消えた（終了コード 3）。中身は理由の ASCII の短い綴り。
    Gone(&'static str),
}

/// 再開の待ちが終わる状態。設計の条件は「作業中」だけ（「停止要請中」へ出し直されたときの
/// 扱いを変えるなら、ここ 1 か所）。
fn resume_is_over(status: ParticipantStatus) -> bool {
    status == ParticipantStatus::Working
}

/// 状態を見て、待ちが終わるか（`Some`）・まだ待つか（`None`）を決める。
///
/// `state` が `None`（状態ファイルが無い。`clear` や壊れたファイルの退避が改名してから書く
/// 一瞬も含む）は、参加者の記録が無いのと同じ「消えた」。
pub fn judge(spec: &WaitSpec, state: Option<&State>) -> Option<WaitEnd> {
    let found = state.and_then(|state| Some((state, state.participants.get(spec.id())?)));
    let Some((state, participant)) = found else {
        return Some(WaitEnd::Gone(spec.gone()));
    };
    let line = match spec {
        WaitSpec::Watch { .. } => match participant.status {
            ParticipantStatus::Working => return None,
            ParticipantStatus::Stopped => "already stopped; run stopped --wait or resume".into(),
            ParticipantStatus::StopRequested => {
                // 理由の無い・誰のものかが空の停止要請（手で直した状態ファイル）は、識別に
                // 使えない字 `?` で示す。
                let by = participant.stop_reason.as_ref().map(|r| r.by.as_str());
                let by = by.filter(|by| !by.is_empty()).unwrap_or("?");
                format!("stop requested by {by}")
            }
        },
        WaitSpec::Merge { id, repo } => match state.merge.get(repo) {
            Some(desk) if desk.holder.as_ref().is_some_and(|h| h.id == *id) => format!(
                "granted merge repo={repo}; last: {}",
                last_merge(desk.last.as_ref())
            ),
            Some(desk) if desk.queue.iter().any(|request| request.id == *id) => return None,
            _ => return Some(WaitEnd::Gone(REQUEST_GONE)),
        },
        WaitSpec::Load { id } => match &state.load.holder {
            Some(holder) if holder.id == *id => format!(
                "granted load; stopped: {}",
                if holder.stopped.is_empty() {
                    "none".to_owned()
                } else {
                    holder.stopped.join(", ")
                }
            ),
            _ if state.load.queue.iter().any(|request| request.id == *id) => return None,
            _ => return Some(WaitEnd::Gone(REQUEST_GONE)),
        },
        WaitSpec::Resume { .. } if resume_is_over(participant.status) => "resumed".to_owned(),
        WaitSpec::Resume { .. } => return None,
    };
    // 識別・リポジトリ・spec は引数の形の決まりで ASCII だが、手で直した状態ファイルからは
    // 何でも来うる。ASCII の外の字（改行も）を逃がして、行が ASCII の 1 行であることを値に頼らない。
    Some(WaitEnd::Done(escape_path(&line)))
}

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;
