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
    LastMerge, LoadHolder, LoadRequest, MergeHolder, MergeRequest, Participant, ParticipantStatus,
    Recent, RecentKind, State, StopReason, WaitKind, WaitRecord, WatchInfo,
};

/// 回収の訳。出来事の `why` と「最近の出来事」の `detail` に同じ綴りで載せる。
const WATCH_ABSENT: &str = "watch absent";

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
    /// 負荷テストの机の申し込み。参加を兼ねる。すでに待っている・持っているなら二重に並べず、
    /// 元の申し込みのままにする。申し込んだ者は「作業中」へ戻る。
    LoadTest {
        id: String,
        name: Option<String>,
        repo: String,
        purpose: String,
    },
    /// 「すでに走っている負荷テスト」の記録。参加を兼ねる。机に持ち主が居なければ、停止要請を
    /// 出さずに持ち主にして走っている印を付ける。持ち主が居れば何も変えない（参加もさせない）。
    LoadRunning {
        id: String,
        name: Option<String>,
        repo: String,
        purpose: String,
    },
    /// 負荷テストが済んだ: 持っている机を空ける。
    LoadDone { id: String },
    /// 「止まった」の報告。当てはまるのは「停止要請中」の参加者だけ。
    Stopped { id: String },
    /// 停止要請の取り消し: 「停止要請中」「止まった」の参加者を「作業中」へ戻す。`None` は全員。
    Unstop { id: Option<String> },
    /// 取り下げ: 申し込みと机を外す。参加者の記録は残す。
    Cancel { id: String },
    /// 離脱: 申し込み・机・待ちの記録を外し、参加者の記録を消す。
    Leave { id: String },
    /// 走っている待ち（マージ・負荷テスト・再開）の記録の登録。同じ識別・同じ種類の記録は
    /// 置き換え、種類が違えば並べる。見張りの記録は [`Command::Watch`] が置く。
    RegisterWait { record: WaitRecord },
    /// 待ち・見張りの記録の抹消（正常に終わる待ちが自分で呼ぶ）。記録が無ければ何も変えない。
    UnregisterWait { id: String, kind: WaitKind },
    /// 周期の一回り: 回収と番の決め直しだけを行う。
    Tick,
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
    LoadRequested {
        id: String,
    },
    /// 負荷テストの番が来た。`stopped` はそのとき止まっていた識別。
    LoadGranted {
        id: String,
        stopped: Vec<String>,
    },
    /// 「すでに走っている負荷テスト」が持ち主として記録された。
    LoadRunning {
        id: String,
    },
    LoadDone {
        id: String,
    },
    /// 停止要請が出た。`by` は負荷テストの候補（机の持ち主か、待ち行列の先頭）。
    StopRequested {
        id: String,
        by: String,
    },
    Stopped {
        id: String,
    },
    /// 「作業中」へ戻った。`why` は戻った訳の ASCII の名前。
    Resumed {
        id: String,
        why: &'static str,
    },
    /// 停止要請の取り消しで「作業中」へ戻った。
    Unstopped {
        id: String,
    },
    /// 回収された（机・待ち行列・待ちの記録・参加者の記録が消えた）。`why` は訳の ASCII の文。
    Reclaimed {
        id: String,
        why: &'static str,
    },
    /// 待ち・見張りの記録が置かれた（置き換えを含む）。
    WaitRegistered {
        id: String,
        kind: WaitKind,
    },
    /// 待ち・見張りの記録が消えた。`why` は `ended`（自分で抹消した）か `absent`（殺されていた）。
    WaitRemoved {
        id: String,
        kind: WaitKind,
        why: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// 状態が呼ぶ前と違うか（偽なら呼び手は何も書かない）。
    pub changed: bool,
    pub verdict: Verdict,
    pub events: Vec<Event>,
}

/// 判断の入口。毎回「回収 → コマンドの処理 → 番の決め直し」の順に行う。`caller` は呼んだ識別
/// （開発者の手の呼び出しは `None`）、`alive` は生死の口で、どちらも回収が使う。
pub fn apply(
    state: &mut State,
    cmd: &Command,
    caller: Option<&str>,
    now: u64,
    alive: &dyn Presence,
) -> Applied {
    let before = state.clone();
    let mut events = Vec::new();
    reclaim(state, caller, now, alive, &mut events);
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
            let record = WaitRecord {
                id: id.clone(),
                kind: WaitKind::Watch,
                repo: Some(repo.clone()),
                pid: *pid,
                since: now,
            };
            put_wait(state, record, &mut events);
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
        Command::LoadTest {
            id,
            name,
            repo,
            purpose,
        } => {
            // 自分の負荷テストのために自分は止まらない（元のスクリプトと同じ）。候補でなければ、
            // この後の番の決め直しが、候補の負荷テストのための停止要請を出し直す。
            let participant = join(state, id, name.as_deref(), repo, now, &mut events);
            resume(participant, "load-request", now, &mut events);
            let desk = &mut state.load;
            let holds = desk.holder.as_ref().is_some_and(|holder| holder.id == *id);
            if !holds && !desk.queue.iter().any(|request| request.id == *id) {
                desk.queue.push(LoadRequest {
                    id: id.clone(),
                    purpose: purpose.clone(),
                    requested: now,
                });
                events.push(Event::LoadRequested { id: id.clone() });
            }
            Verdict::Applied
        }
        Command::LoadRunning {
            id,
            name,
            repo,
            purpose,
        } => {
            // 断るときは状態を変えない（参加もさせない）ので、机を見るのは `join` より先。
            if state.load.holder.is_some() {
                Verdict::NotApplied("the load-test desk already has a holder")
            } else {
                let participant = join(state, id, name.as_deref(), repo, now, &mut events);
                events.push(Event::LoadRunning { id: id.clone() });
                // 止まっていた者も、自分の負荷テストのために「作業中」へ戻る（番を受けた持ち主と同じ）。
                resume(participant, "load-running", now, &mut events);
                drop_where(&mut state.load.queue, |request| request.id == *id);
                state.load.holder = Some(LoadHolder {
                    id: id.clone(),
                    purpose: purpose.clone(),
                    requested: now,
                    granted: now,
                    running: true,
                    stopped: Vec::new(),
                });
                Verdict::Applied
            }
        }
        Command::LoadDone { id } => {
            if state
                .load
                .holder
                .take_if(|holder| holder.id == *id)
                .is_some()
            {
                events.push(Event::LoadDone { id: id.clone() });
                Verdict::Applied
            } else {
                Verdict::NotApplied("not the load-test holder")
            }
        }
        Command::Stopped { id } => match state.participants.get_mut(id) {
            Some(participant) if participant.status == ParticipantStatus::StopRequested => {
                participant.status = ParticipantStatus::Stopped;
                participant.since = now;
                events.push(Event::Stopped { id: id.clone() });
                Verdict::Applied
            }
            _ => Verdict::NotApplied("not asked to stop"),
        },
        Command::Unstop { id } => {
            // 走っている印の無い負荷テストがまだ持たれ・待たれていれば、この後の番の決め直しが
            // 停止要請を出し直す。
            let mut any = false;
            for participant in state.participants.values_mut() {
                let target = id.as_ref().is_none_or(|id| *id == participant.id);
                if target && back_to_work(participant, now) {
                    any = true;
                    events.push(Event::Unstopped {
                        id: participant.id.clone(),
                    });
                }
            }
            if any {
                Verdict::Applied
            } else {
                Verdict::NotApplied("nobody to unstop")
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
        Command::RegisterWait { record } => {
            put_wait(state, record.clone(), &mut events);
            Verdict::Applied
        }
        Command::UnregisterWait { id, kind } => {
            if drop_where(&mut state.waits, |wait| {
                wait.id == *id && wait.kind == *kind
            }) {
                events.push(Event::WaitRemoved {
                    id: id.clone(),
                    kind: *kind,
                    why: "ended",
                });
            }
            Verdict::Applied
        }
        Command::Tick => Verdict::Applied,
    };
    replan(state, now, &mut events);
    Applied {
        changed: *state != before,
        verdict,
        events,
    }
}

/// 回収: 見張りの居ない「作業中」の参加者を、机・待ち行列・待ちの記録ごと外して記録を消し、
/// 「最近の出来事」に残す。続けて、プロセスの居ない（殺された）待ちの記録を消す。
///
/// コマンドの処理より先に行うので、同じ呼び出しで登録する待ちの記録は、ここでは消えない。
fn reclaim(
    state: &mut State,
    caller: Option<&str>,
    now: u64,
    alive: &dyn Presence,
    events: &mut Vec<Event>,
) {
    let mut gone = Vec::new();
    for (id, participant) in &mut state.participants {
        // 「停止要請中」「止まった」は、見張りが終わっているのが正常なので回収しない。
        if participant.status != ParticipantStatus::Working {
            continue;
        }
        let present = alive.is_present(id, WaitKind::Watch);
        if participant.awaiting_watch_since.is_some() {
            // 再開してから見張りを立て直すまでは回収しない。見張りが居ると分かったら印だけ消す。
            if present {
                participant.awaiting_watch_since = None;
            }
        } else if !present && caller != Some(id.as_str()) {
            // 呼んだ識別は同じ呼び出しでは回収しない。
            gone.push(id.clone());
        }
    }
    for id in gone {
        erase(state, &id);
        state.push_recent(Recent {
            at: now,
            kind: RecentKind::Reclaimed,
            id: Some(id.clone()),
            detail: WATCH_ABSENT.to_owned(),
        });
        events.push(Event::Reclaimed {
            id,
            why: WATCH_ABSENT,
        });
    }
    // 殺された待ち・見張りの記録: その識別の、その種類のプロセスが居ないもの（呼んだ識別のものも
    // 同じ。走っている待ちは自分のロックを握っているので「居る」と答えが返る）。
    state.waits.retain(|wait| {
        let present = alive.is_present(&wait.id, wait.kind);
        if !present {
            events.push(Event::WaitRemoved {
                id: wait.id.clone(),
                kind: wait.kind,
                why: "absent",
            });
        }
        present
    });
}

/// 番を決め直す（元のスクリプトの `Invoke-Plan` の写し）: 停止要請の発行 → 負荷テストの番 →
/// 再開 → マージの番。何度呼んでも同じ結果になる。
fn replan(state: &mut State, now: u64, events: &mut Vec<Event>) {
    // 負荷テストが持たれている・待たれている間は、誰も再開せず、どのリポジトリのマージの番も出ない。
    if plan_load(state, now, events) {
        return;
    }
    for participant in state.participants.values_mut() {
        resume(participant, "no-load", now, events);
    }
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

/// 負荷テストの段: 停止要請の発行 → 負荷テストの番。負荷テストが持たれても待たれてもいなければ
/// 何もせずに偽を返す。
fn plan_load(state: &mut State, now: u64, events: &mut Vec<Event>) -> bool {
    let load = &state.load;
    // 待ち行列の先頭＝申し込みの時刻がいちばん早い者（同じ時刻なら行列の前）。
    let by_time = |(_, request): &(usize, &LoadRequest)| request.requested;
    let first = load.queue.iter().enumerate().min_by_key(by_time);
    // 候補＝机の持ち主か、居なければ待ち行列の先頭。
    let (candidate, purpose, running) = match (&load.holder, first) {
        (Some(holder), _) => (holder.id.clone(), holder.purpose.clone(), holder.running),
        (None, Some((_, request))) => (request.id.clone(), request.purpose.clone(), false),
        (None, None) => return false,
    };
    let first = first.map(|(index, _)| index);

    // 止まる必要のある参加者: 候補・マージの持ち主・マージ待ち（どのリポジトリでも）を除く全員。
    // そのうち「作業中」の者に停止要請を出す。すでに走っている負荷テスト（走っている印）は出さない。
    let in_merge = |id: &str| {
        state.merge.values().any(|desk| {
            desk.holder.as_ref().is_some_and(|holder| holder.id == id)
                || desk.queue.iter().any(|request| request.id == id)
        })
    };
    let mut need_stop = Vec::new();
    let mut all_stopped = true;
    for (id, participant) in &mut state.participants {
        if *id == candidate || in_merge(id) {
            continue;
        }
        if participant.status == ParticipantStatus::Working && !running {
            participant.status = ParticipantStatus::StopRequested;
            participant.since = now;
            participant.stop_reason = Some(StopReason {
                by: candidate.clone(),
                purpose: purpose.clone(),
            });
            events.push(Event::StopRequested {
                id: id.clone(),
                by: candidate.clone(),
            });
        }
        all_stopped &= participant.status == ParticipantStatus::Stopped;
        need_stop.push(id.clone());
    }

    // 負荷テストの番: 持ち主なし・どのリポジトリにもマージの持ち主なし・止まる必要のある全員が
    // 「止まった」で、先頭を持ち主にする。
    let merging = state.merge.values().any(|desk| desk.holder.is_some());
    if let Some(first) = first.filter(|_| state.load.holder.is_none() && !merging && all_stopped) {
        let request = state.load.queue.remove(first);
        events.push(Event::LoadGranted {
            id: request.id.clone(),
            stopped: need_stop.clone(),
        });
        // 止まっていた者が番を受けたら、自分の負荷テストのために「作業中」へ戻る。
        if let Some(participant) = state.participants.get_mut(&request.id) {
            resume(participant, "load-granted", now, events);
        }
        state.load.holder = Some(LoadHolder {
            id: request.id,
            purpose: request.purpose,
            requested: request.requested,
            granted: now,
            running: false,
            stopped: need_stop,
        });
    }
    true
}

/// 参加を終える: [`erase`] で消し、何か消えたら出来事を 1 件。
fn depart(state: &mut State, id: &str, why: &'static str, events: &mut Vec<Event>) {
    if erase(state, id) {
        events.push(Event::Left {
            id: id.to_owned(),
            why,
        });
    }
}

/// その識別の申し込み・机・待ちの記録を外し、参加者の記録を消す。何か消えたら真。
/// 待ちの記録が消えることが、その識別の見張りと待ちが「記録が消えた」で終わる根拠。
fn erase(state: &mut State, id: &str) -> bool {
    let mut gone = remove_from(state, id);
    gone |= drop_where(&mut state.waits, |wait| wait.id == id);
    gone |= state.participants.remove(id).is_some();
    gone
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

/// 「停止要請中」「止まった」の参加者を「作業中」へ戻す（戻すのはここだけ）: 理由を消し、見張りを
/// 立て直すまでの印を付ける（見張りは停止要請で終わっているので、立て直すまでは回収しない）。
/// 戻したら真。「作業中」なら何もせず偽。
fn back_to_work(participant: &mut Participant, now: u64) -> bool {
    if participant.status == ParticipantStatus::Working {
        return false;
    }
    participant.status = ParticipantStatus::Working;
    participant.since = now;
    participant.stop_reason = None;
    participant.awaiting_watch_since = Some(now);
    true
}

/// [`back_to_work`] で戻し、戻したら出来事 `Resumed` を 1 件。
fn resume(participant: &mut Participant, why: &'static str, now: u64, events: &mut Vec<Event>) {
    if back_to_work(participant, now) {
        events.push(Event::Resumed {
            id: participant.id.clone(),
            why,
        });
    }
}

/// 待ちの記録を置く。同じ識別・同じ種類の記録が在れば、その場で置き換える（種類が違えば並べる）。
/// 置いて状態が変わったら出来事を 1 件（同じ記録の置き直しは何も変えない）。
fn put_wait(state: &mut State, record: WaitRecord, events: &mut Vec<Event>) {
    let same = |wait: &&mut WaitRecord| wait.id == record.id && wait.kind == record.kind;
    let slot = state.waits.iter_mut().find(same);
    if slot.as_deref() == Some(&record) {
        return;
    }
    events.push(Event::WaitRegistered {
        id: record.id.clone(),
        kind: record.kind,
    });
    match slot {
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

#[cfg(test)]
#[path = "plan_stop_tests.rs"]
mod stop_tests;

#[cfg(test)]
#[path = "plan_reclaim_tests.rs"]
mod reclaim_tests;
