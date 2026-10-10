//! 長い待ち（見張り・マージの番・負荷テストの番・再開）: 終わりの判定と、待ちのループ。
//!
//! [`judge`] は状態を見て「終わるか・まだ待つか」を決めるだけで、ファイルも時計もプロセスも
//! ロックも触らない。終わりの 1 行に使うのは識別・リポジトリ・spec・PR・sha・時刻だけで、
//! 名前と内容（日本語になりうる値）は使わない。このファイルの文字列リテラルは全部 ASCII。
//!
//! [`run`] は 4 種の待ちに共通のループで、時計・読み・状態を変える 1 回・眠りを口
//! （[`WaitPort`]）に頼む。時間の上限は無く、待っている間は端末へ何も出さない（終わり方を
//! 返すだけで、出すのは呼び手）。

use std::time::{Duration, SystemTime};

use crate::error::{WatchError, escape_path};
use crate::plan::{self, Applied, Command};
use crate::presence::Held;
use crate::state::{ParticipantStatus, State, WaitKind, WaitRecord};
use crate::status::last_merge;
use crate::store::{self, Store};

/// 読み直しの間隔（本物の眠り）。
const POLL: Duration = Duration::from_secs(1);
/// 変わっていなくても中身を読む間隔（眠りの回数）。更新時刻と大きさに映らなかった変化を拾う。
const FULL_READ_EVERY: u64 = 10;
/// 周期の一回りの間隔（眠りの回数）。
const TICK_EVERY: u64 = 30;
// 周期の一回りの後の読みは「10 回目ごとの読み」が兼ねる。
const _: () = assert!(TICK_EVERY.is_multiple_of(FULL_READ_EVERY));

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
        repo: String,
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
    /// 待っている識別。
    pub fn id(&self) -> &str {
        match self {
            WaitSpec::Watch { id, .. }
            | WaitSpec::Merge { id, .. }
            | WaitSpec::Load { id }
            | WaitSpec::Resume { id } => id,
        }
    }

    /// 待ち・見張りの記録の種類（居る印のロックファイルの種類でもある）。
    pub fn kind(&self) -> WaitKind {
        match self {
            WaitSpec::Watch { .. } => WaitKind::Watch,
            WaitSpec::Merge { .. } => WaitKind::Merge,
            WaitSpec::Load { .. } => WaitKind::Load,
            WaitSpec::Resume { .. } => WaitKind::Resume,
        }
    }

    /// 待ちの始めに 1 回行う登録。見張りは見張りの開始（参加を兼ねる）、他は待ちの記録。
    fn register(&self, pid: u32, now: u64) -> Command {
        match self {
            WaitSpec::Watch { id, repo, name } => Command::Watch {
                id: id.clone(),
                name: name.clone(),
                repo: repo.clone(),
                pid,
            },
            WaitSpec::Merge { id, .. } | WaitSpec::Load { id } | WaitSpec::Resume { id } => {
                let repo = match self {
                    WaitSpec::Merge { repo, .. } => Some(repo.clone()),
                    _ => None,
                };
                let record = WaitRecord {
                    id: id.clone(),
                    kind: self.kind(),
                    repo,
                    pid,
                    since: now,
                };
                Command::RegisterWait { record }
            }
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

/// 待ちのループが外へ頼むこと（本物は状態ファイルの口と本物の時計・テストは偽物）。
pub trait WaitPort {
    /// いまの時刻（UNIX 秒）。待ちの記録の「始めた時刻」に載せる。
    fn now(&self) -> u64;
    /// 待っているプロセスの番号。見張り・待ちの記録に載せる（人が読むためで、生死の判定には
    /// 使わない）。
    fn pid(&self) -> u32;
    /// 状態ファイルの（更新時刻, 大きさ）。無ければ `None`。中身は読まない。
    fn fingerprint(&self) -> Result<Option<(SystemTime, u64)>, WatchError>;
    /// 状態を読む（排他を取らない）。状態ファイルが無ければ `None`。
    fn read(&self) -> Result<Option<State>, WatchError>;
    /// 状態を変える 1 回（短い排他の中で判断に通す）。`caller` は呼んだ識別。
    fn change(&self, cmd: &Command, caller: &str) -> Result<Applied, WatchError>;
    /// 読み直しの間の眠り（本物は 1 秒）。
    fn sleep(&self);
}

/// 本物の口: 状態ファイルの口の上に、本物の時計・このプロセスの番号・1 秒の眠りを載せる。
impl WaitPort for Store {
    fn now(&self) -> u64 {
        store::unix_now()
    }

    fn pid(&self) -> u32 {
        std::process::id()
    }

    fn fingerprint(&self) -> Result<Option<(SystemTime, u64)>, WatchError> {
        // 口の同じ名前の関数（`io::Result` を返し、ログを出さない）。失敗はここでログに残す。
        Store::fingerprint(self).map_err(|err| {
            let failure = WatchError::io("stat state.json", &err);
            tracing::error!(error = %failure, "[wait] state file not checked");
            failure
        })
    }

    fn read(&self) -> Result<Option<State>, WatchError> {
        self.read_only()
    }

    fn change(&self, cmd: &Command, caller: &str) -> Result<Applied, WatchError> {
        self.with_state(|state, now, alive| {
            let applied = plan::apply(state, cmd, Some(caller), now, alive);
            (applied.clone(), applied)
        })
    }

    fn sleep(&self) {
        // 常時テストが本物の口に通すのは、直ちに終わる（眠りに入らない）場合だけ。眠りに来たなら
        // 終わりの条件が壊れているので、終わらないテストにせず、その場で赤にする。
        #[cfg(test)]
        panic!("a test reached the real 1-second sleep");
        #[cfg(not(test))]
        std::thread::sleep(POLL);
    }
}

/// 待つ。番が来た・停止要請が出た・再開した・消えた、のどれかで終わる。時間の上限は無い。
///
/// `held` は呼び手が先に握った居る印で、返るまで握ったままにする。手順:
///
/// 1. 登録を 1 回（見張りは見張りの開始・他は待ちの記録）。呼び手は自分の識別。
/// 2. 読んで判定する。終わらなければ眠り、眠るたびに状態ファイルの更新時刻と大きさを見て、
///    変わっていたか [`FULL_READ_EVERY`] 回目なら、また読んで判定する。[`TICK_EVERY`] 回目
///    ごとに、その前に周期の一回りを呼ぶ（回数は始めからの眠りの数）。
/// 3. 終わるときに自分の記録を消す（無ければ何も変わらない）。
///
/// 失敗の扱い: 登録・読み・更新時刻の取得の失敗は待ちの失敗（記録は消しに行かない。握りが
/// 解けるので、殺された待ちと同じく次の呼び出しの回収が消す）。周期の一回りの失敗は待ちを
/// 終えない。答えが出た後の抹消の失敗は、答えを変えない（残った記録は同じく回収が消す）。
pub fn run(spec: &WaitSpec, port: &dyn WaitPort, held: Held) -> Result<WaitEnd, WatchError> {
    let id = spec.id();
    port.change(&spec.register(port.pid(), port.now()), id)?;
    // 読む前に取る: 取ってから読むまでの間の変化は、次の眠りの後に「変わった」と見える。
    let mut seen = port.fingerprint()?;
    let mut polls = 0_u64;
    let mut read_due = true;
    let end = loop {
        if read_due && let Some(end) = judge(spec, port.read()?.as_ref()) {
            break end;
        }
        port.sleep();
        polls += 1;
        if polls.is_multiple_of(TICK_EVERY)
            && let Err(err) = port.change(&Command::Tick, id)
        {
            // 長い待ちを一時の混雑で失敗にしない。
            tracing::warn!(id, error = %err, "[wait] periodic round failed; still waiting");
        }
        let now = port.fingerprint()?;
        read_due = now != seen || polls.is_multiple_of(FULL_READ_EVERY);
        seen = now;
    };
    let unregister = Command::UnregisterWait {
        id: id.to_owned(),
        kind: spec.kind(),
    };
    if let Err(err) = port.change(&unregister, id) {
        tracing::warn!(id, error = %err, "[wait] wait record not removed; left to be reclaimed");
    }
    drop(held);
    Ok(end)
}

#[cfg(test)]
#[path = "wait_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "wait_loop_tests.rs"]
mod loop_tests;
