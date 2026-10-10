//! 純粋な判断（規則 1〜7・回収・番の決め直し・出来事）。ファイルも時計もプロセスも触らない。
//!
//! 入口は [`apply`] 1 つ。いまの時刻と生死の答え（[`Presence`]）は引数で受けるので、同じ引数なら
//! いつ呼んでも同じ結果になる。状態が変わるたびに出来事（[`Event`]）を 1 件ずつ返し、ログへ
//! 書くのは呼び手の仕事。

// 使い手（store・wait・cli）が載るまで、本番のビルドではここが未使用になる。
// 全部が使われるとこの行が「満たされない expect」の警告になるので、そのとき外す。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "使い手のモジュールは後のタスクで載る")
)]

use crate::state::{
    LastMerge, MergeHolder, MergeRequest, Participant, State, WaitKind, WaitRecord, WatchInfo,
};

/// 生死の口: その識別の、その種類の待ち（見張りを含む）のプロセスがいま居るか。
/// 実装は口の層が持ち、ここは尋ねるだけ。
pub trait Presence {
    fn is_present(&self, id: &str, kind: WaitKind) -> bool;
}

/// 状態を変えるコマンド。識別は「誰について」で、呼んだ者と違ってよい
/// （開発者が他の参加者の取り下げ・離脱を代わりに行える）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// 見張りの開始。参加を兼ねる（無ければ「作業中」で登録・あれば名前とリポジトリを更新）。
    /// `name` を省くと、初めての参加では識別が名前になり、2 度目からは元の名前のまま。
    Watch {
        id: String,
        name: Option<String>,
        repo: String,
        pid: u32,
    },
    /// マージの机の申し込み。参加を兼ねる。同じリポジトリをすでに待っている・持っているなら
    /// 二重に並べず、元の申し込みのままにする。
    Merge {
        id: String,
        name: Option<String>,
        repo: String,
        spec: String,
        bug: bool,
    },
    /// マージが済んだ: 持っている机を空け、直前のマージを記録し、参加を終える。
    Merged { id: String, pr: String, sha: String },
    /// 取り下げ: 申し込みと机を外す。参加者の記録は残す。
    Cancel { id: String },
    /// 離脱: 申し込み・机・待ちの記録を外し、参加者の記録を消す。
    Leave { id: String },
}

/// コマンドが当てはまったか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Applied,
    /// 当てはまらなかった（状態は変えていない）。文は ASCII で、端末へそのまま出す。
    NotApplied(&'static str),
}

/// 状態の変化 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Joined {
        id: String,
    },
    /// `why` は消えた訳の ASCII の名前。
    Left {
        id: String,
        why: &'static str,
    },
    Cancelled {
        id: String,
    },
    MergeRequested {
        repo: String,
        id: String,
        spec: String,
        bug: bool,
    },
    /// マージの番が来た。
    MergeGranted {
        repo: String,
        id: String,
    },
    Merged {
        repo: String,
        id: String,
        pr: String,
        sha: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// 状態が呼ぶ前と違うか（偽なら呼び手は何も書かない）。
    pub changed: bool,
    pub verdict: Verdict,
    pub events: Vec<Event>,
}

/// 判断の入口。`caller` は呼んだ識別、`alive` は生死の口で、どちらも回収が使う。
pub fn apply(
    state: &mut State,
    cmd: &Command,
    _caller: Option<&str>,
    now: u64,
    _alive: &dyn Presence,
) -> Applied {
    let before = state.clone();
    let mut events = Vec::new();
    let verdict = match cmd {
        Command::Watch {
            id,
            name,
            repo,
            pid,
        } => {
            // 状態（作業中・停止要請中・止まった）は変えない。停止要請中・止まったのまま
            // 立て直した見張りは、それを読んで直ちに終わる。
            let participant = join(state, id, name.as_deref(), repo, now, &mut events);
            participant.watch = Some(WatchInfo {
                pid: *pid,
                since: now,
            });
            participant.awaiting_watch_since = None;
            put_wait(
                state,
                WaitRecord {
                    id: id.clone(),
                    kind: WaitKind::Watch,
                    repo: Some(repo.clone()),
                    pid: *pid,
                    since: now,
                },
            );
            Verdict::Applied
        }
        Command::Merge {
            id,
            name,
            repo,
            spec,
            bug,
        } => {
            join(state, id, name.as_deref(), repo, now, &mut events);
            // 二重の判定はリポジトリごと（別のリポジトリの机は独立に申し込める）。
            let desk = state.merge.entry(repo.clone()).or_default();
            let holds = desk.holder.as_ref().is_some_and(|holder| holder.id == *id);
            if !holds && !desk.queue.iter().any(|request| request.id == *id) {
                desk.queue.push(MergeRequest {
                    id: id.clone(),
                    spec: spec.clone(),
                    bug: *bug,
                    requested: now,
                });
                events.push(Event::MergeRequested {
                    repo: repo.clone(),
                    id: id.clone(),
                    spec: spec.clone(),
                    bug: *bug,
                });
            }
            Verdict::Applied
        }
        Command::Merged { id, pr, sha } => {
            let mut held = false;
            for (repo, desk) in &mut state.merge {
                if let Some(holder) = desk.holder.take_if(|holder| holder.id == *id) {
                    held = true;
                    desk.last = Some(LastMerge {
                        pr: pr.clone(),
                        sha: sha.clone(),
                        spec: holder.spec,
                        at: now,
                    });
                    events.push(Event::Merged {
                        repo: repo.clone(),
                        id: id.clone(),
                        pr: pr.clone(),
                        sha: sha.clone(),
                    });
                }
            }
            if held {
                // 完了した spec のセッションは参加を終える。
                depart(state, id, "merged", &mut events);
                Verdict::Applied
            } else {
                Verdict::NotApplied("not the merge holder")
            }
        }
        Command::Cancel { id } => {
            if remove_from(state, id) {
                events.push(Event::Cancelled { id: id.clone() });
            }
            Verdict::Applied
        }
        Command::Leave { id } => {
            depart(state, id, "leave", &mut events);
            Verdict::Applied
        }
    };
    replan(state, now, &mut events);
    Applied {
        changed: *state != before,
        verdict,
        events,
    }
}

/// 番を決め直す。何度呼んでも同じ結果になる。
fn replan(state: &mut State, now: u64, events: &mut Vec<Event>) {
    // マージの番: 持ち主の居ないリポジトリごとに、バグ優先 → 申し込みの時刻が早い順で
    // 1 人を持ち主にする。リポジトリ同士は独立。
    for (repo, desk) in &mut state.merge {
        if desk.holder.is_some() {
            continue;
        }
        let order = |(_, request): &(usize, &MergeRequest)| (!request.bug, request.requested);
        let Some((first, _)) = desk.queue.iter().enumerate().min_by_key(order) else {
            continue;
        };
        let request = desk.queue.remove(first);
        events.push(Event::MergeGranted {
            repo: repo.clone(),
            id: request.id.clone(),
        });
        desk.holder = Some(MergeHolder {
            id: request.id,
            spec: request.spec,
            bug: request.bug,
            requested: request.requested,
            granted: now,
        });
    }
}

/// 参加を終える: 申し込み・机・待ちの記録を外し、参加者の記録を消す。何か消えたら出来事を 1 件。
/// 待ちの記録が消えることが、その識別の見張りと待ちが「記録が消えた」で終わる根拠。
fn depart(state: &mut State, id: &str, why: &'static str, events: &mut Vec<Event>) {
    let mut gone = remove_from(state, id);
    gone |= drop_where(&mut state.waits, |wait| wait.id == id);
    gone |= state.participants.remove(id).is_some();
    if gone {
        events.push(Event::Left {
            id: id.to_owned(),
            why,
        });
    }
}

/// 参加: 無ければ「作業中」で登録し、あれば名前（渡されたときだけ）とリポジトリを更新する。
/// 参加者の記録を作るのはここだけ。
fn join<'a>(
    state: &'a mut State,
    id: &str,
    name: Option<&str>,
    repo: &str,
    now: u64,
    events: &mut Vec<Event>,
) -> &'a mut Participant {
    let participant = state.participants.entry(id.to_owned()).or_insert_with(|| {
        events.push(Event::Joined { id: id.to_owned() });
        Participant {
            id: id.to_owned(),
            name: id.to_owned(),
            since: now,
            ..Participant::default()
        }
    });
    if let Some(name) = name {
        participant.name = name.to_owned();
    }
    participant.repo = repo.to_owned();
    participant
}

/// 待ちの記録を置く。同じ識別・同じ種類の記録が在れば、その場で置き換える。
fn put_wait(state: &mut State, record: WaitRecord) {
    let same = |wait: &&mut WaitRecord| wait.id == record.id && wait.kind == record.kind;
    match state.waits.iter_mut().find(same) {
        Some(wait) => *wait = record,
        None => state.waits.push(record),
    }
}

/// その識別の申し込みと机を、マージ（全部のリポジトリ）と負荷テストから外す。外したら真。
fn remove_from(state: &mut State, id: &str) -> bool {
    let mut removed = false;
    for desk in state.merge.values_mut() {
        removed |= drop_where(&mut desk.queue, |request| request.id == id);
        removed |= desk.holder.take_if(|holder| holder.id == id).is_some();
    }
    removed |= drop_where(&mut state.load.queue, |request| request.id == id);
    removed |= state
        .load
        .holder
        .take_if(|holder| holder.id == id)
        .is_some();
    removed
}

/// 当てはまる項目を落とす。1 つでも落としたら真。
fn drop_where<T>(items: &mut Vec<T>, mut gone: impl FnMut(&T) -> bool) -> bool {
    let before = items.len();
    items.retain(|item| !gone(item));
    items.len() != before
}

#[cfg(test)]
#[path = "plan_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "plan_desk_tests.rs"]
mod desk_tests;
