//! 判断の中核の子: 机の申し込み・返し・走っている負荷テストの記録・排他テストの机の空け・番の
//! 決め直し（停止要請か再開 → 渡せる申し込みへ渡すを繰り返す → マージの番）と、待ち行列の番の
//! 来る順。親の `plan.rs` と同じく、ファイルも時計もプロセスもログも触らない。
//!
//! 番の決まり（設計「番の決め方」）:
//!
//! 1. 申し込みに並べた机が全部渡せるときだけ、同じ呼び出しの中で全部を渡す（一部だけ渡さない）。
//! 2. 早い順に見て、渡せる最初の申し込みに渡すを、渡せるものが無くなるまで繰り返す（渡せない
//!    申し込みのために机を取っておかない）。
//! 3. 排他テストの机が渡せる: 持ち主が居ない、かつ（ひとり占めを含む申し込みが待たれていない、
//!    または申し込んだ者が「停止要請中」、またはどこかのリポジトリのマージの持ち主）。ひとり占めを
//!    含む申し込み自身の排他テストの机には但し書きを掛けない。
//! 4. ひとり占めが渡せる: 持ち主が居ない、かつその申し込みが候補、かつ止まる必要のある全員が
//!    「止まった」、かつどのリポジトリにもマージの持ち主が居ない。
//! 5. ひとり占めを含まない申し込みは、申し込んだ者が「止まった」なら渡さない。ひとり占めを含む
//!    申し込みは渡し、そのとき「作業中」へ戻す。
//! 6. マージの番は、候補（ひとり占めの持ち主か、ひとり占めを含む申し込み）が在る間は出さない。

use super::{Event, Verdict, drop_where, join, resume};
use crate::state::{
    DeskRequest, DeskSet, MergeDesk, MergeHolder, MergeRequest, ParticipantStatus, Recent,
    RecentKind, SoleHolder, State, StopReason, TestHolder,
};

/// 持ちながらの申し込みの断り（要件 4.2）。
const HELD: &str = "desks are held; only a merge holder may add the test desk";

/// 申し込み（参加を兼ねる）。状態を変えない断りは参加より先に見る。
#[allow(clippy::too_many_arguments)]
pub(super) fn request(
    state: &mut State,
    id: &str,
    name: Option<&str>,
    repo: &str,
    desks: DeskSet,
    purpose: &str,
    now: u64,
    events: &mut Vec<Event>,
) -> Verdict {
    // 1. 自分の申し込みが待っている（要件 2.7・2.8）。
    if let Some(waiting) = state.requests.iter().find(|request| request.id == id) {
        return if waiting.desks == desks {
            Verdict::Applied
        } else {
            Verdict::NotApplied("a different desk request is already waiting")
        };
    }
    // 2. 机を持つ: 同じ組なら直ちに番（要件 2.7・4.2）。
    if let Some(holds) = held(state, id) {
        return if holds == desks {
            Verdict::Applied
        } else {
            Verdict::NotApplied(HELD)
        };
    }
    // 3. マージの机を持つ: 足せるのは排他テストの机だけ（要件 4.1・4.2）。
    if holds_merge(state, id) && desks != DeskSet::Test {
        return Verdict::NotApplied(HELD);
    }
    // 3a. マージの机を待つ（要件 4.8）。
    if waits_merge(state, id) {
        return Verdict::NotApplied(
            "a merge request is waiting; request desks after the merge desk is granted",
        );
    }
    // 4. 参加して末尾へ並ぶ。申し込んだ者の状態（作業中・停止要請中・止まった）は変えない。
    join(state, id, name, repo, now, events);
    state.requests.push(DeskRequest {
        id: id.to_owned(),
        desks,
        purpose: purpose.to_owned(),
        requested: now,
    });
    events.push(Event::DeskRequested {
        id: id.to_owned(),
        desks,
    });
    Verdict::Applied
}

/// 返す。`None` はマージの机のほかの全部（要件 4.3〜4.5）。再開は続く番の決め直しが行う（4.6）。
pub(super) fn give_back(
    state: &mut State,
    id: &str,
    desks: Option<DeskSet>,
    now: u64,
    events: &mut Vec<Event>,
) -> Verdict {
    let Some(holds) = held(state, id) else {
        return match desks {
            Some(_) => Verdict::NotApplied("not the holder of that desk"),
            None => Verdict::NotApplied("holds no desk to return"),
        };
    };
    let give = desks.unwrap_or(holds);
    let owns = (!give.has_test() || holds.has_test()) && (!give.has_sole() || holds.has_sole());
    if !owns {
        return Verdict::NotApplied("not the holder of that desk");
    }
    // 走っている負荷テストの記録は 2 つの机を 1 組で持つ（状態の不変）。排他テストの机だけを
    // 外すと、走っている計測の最中に別の者が排他テストの机を受けられるので、1 組のまま返させる。
    let running = state.sole.as_ref().is_some_and(|sole| sole.running);
    if running && give == DeskSet::Test {
        return Verdict::NotApplied("a running load test returns test and sole together");
    }
    if give.has_sole() {
        state.sole = None;
    }
    if give.has_test() {
        free_test(state, "returned", now, events);
    }
    events.push(Event::DeskReturned {
        id: id.to_owned(),
        desks: give,
    });
    Verdict::Applied
}

/// 走っている負荷テストの記録: 両方の机が空いていれば、停止要請を出さずに両方の持ち主にする
/// （要件 5.5・5.6）。断るときは状態を変えない（参加もさせない）。
pub(super) fn load_running(
    state: &mut State,
    id: &str,
    name: Option<&str>,
    repo: &str,
    purpose: &str,
    now: u64,
    events: &mut Vec<Event>,
) -> Verdict {
    if state.test.is_some() || state.sole.is_some() {
        return Verdict::NotApplied("the test desk or the sole desk already has a holder");
    }
    // マージの机を待つ者は持ち主にしない（要件 5.7。持ち主はマージの待ち行列に居ない形を保つ）。
    if waits_merge(state, id) {
        return Verdict::NotApplied(
            "a merge request is waiting; record the running load after the merge desk is granted",
        );
    }
    let participant = join(state, id, name, repo, now, events);
    events.push(Event::LoadRunning { id: id.to_owned() });
    // 止まっていた者も、自分の負荷テストのために「作業中」へ戻る（番を受けた持ち主と同じ）。
    resume(participant, "load-running", now, events);
    drop_where(&mut state.requests, |request| request.id == id);
    state.test = Some(TestHolder {
        id: id.to_owned(),
        purpose: purpose.to_owned(),
        requested: now,
        granted: now,
    });
    state.sole = Some(SoleHolder {
        id: id.to_owned(),
        purpose: purpose.to_owned(),
        requested: now,
        granted: now,
        running: true,
        stopped: Vec::new(),
    });
    Verdict::Applied
}

/// 排他テストの机を空ける唯一の道: 持ち主を外し、出来事 `TestFreed` と最近の出来事に訳を残す
/// （要件 6.2）。持ち主が居なければ何もしない。
pub(super) fn free_test(state: &mut State, why: &'static str, now: u64, events: &mut Vec<Event>) {
    let Some(holder) = state.test.take() else {
        return;
    };
    events.push(Event::TestFreed {
        id: holder.id.clone(),
        why,
    });
    state.push_recent(Recent {
        at: now,
        kind: RecentKind::TestFreed,
        id: Some(holder.id),
        detail: why.to_owned(),
    });
}

/// 番を決め直す: 候補が在れば停止要請・無ければ再開 → 渡せる最初の申し込みへ渡すを繰り返す →
/// 候補が無ければマージの番。何度呼んでも同じ結果になる。
pub(super) fn replan(state: &mut State, now: u64, events: &mut Vec<Event>) {
    // 候補の有無は見回しの前後で変わらない（見回しはひとり占めを含む申し込みを渡さずに消さず、
    // 渡せば候補は同じ識別のまま持ち主になる）。だから再開を見回しの前に置ける。
    let candidate = candidate(state);
    match &candidate {
        Some((id, purpose, running)) => {
            if !running {
                ask_to_stop(state, id, purpose, now, events);
            }
        }
        None => {
            for participant in state.participants.values_mut() {
                resume(participant, "no-sole", now, events);
            }
        }
    }
    // 渡すたびに机が埋まるか申し込みが減るので、申し込みの数を超えて回らない。
    let candidate_id = candidate.as_ref().map(|(id, ..)| id.as_str());
    while grant_one(state, candidate_id, now, events) {}
    if candidate.is_none() {
        grant_merges(state, now, events);
    }
}

/// 候補: ひとり占めの持ち主、居なければひとり占めを含む申し込みのうち最も早いもの。
/// （識別, 内容, 走っている印）。
fn candidate(state: &State) -> Option<(String, String, bool)> {
    if let Some(sole) = &state.sole {
        return Some((sole.id.clone(), sole.purpose.clone(), sole.running));
    }
    request_order(&state.requests)
        .into_iter()
        .map(|nth| &state.requests[nth])
        .find(|request| request.desks.has_sole())
        .map(|request| (request.id.clone(), request.purpose.clone(), false))
}

/// どこかのリポジトリでマージの机を持つ。
fn holds_merge(state: &State, id: &str) -> bool {
    let holds = |desk: &MergeDesk| desk.holder.as_ref().is_some_and(|h| h.id == id);
    state.merge.values().any(holds)
}

/// どこかのリポジトリでマージの机を待つ。
fn waits_merge(state: &State, id: &str) -> bool {
    let waits = |desk: &MergeDesk| desk.queue.iter().any(|request| request.id == id);
    state.merge.values().any(waits)
}

/// 止まる必要のある参加者: 候補・マージの持ち主・マージ待ち（どのリポジトリでも）を除く全員。
fn need_stop(state: &State, candidate: &str) -> Vec<String> {
    let exempt = |id: &str| id == candidate || holds_merge(state, id) || waits_merge(state, id);
    let ids = state.participants.keys().filter(|id| !exempt(id));
    ids.cloned().collect()
}

/// 止まる必要のある参加者のうち「作業中」の者に停止要請を出す（後から参加した者を含む）。
fn ask_to_stop(
    state: &mut State,
    candidate: &str,
    purpose: &str,
    now: u64,
    events: &mut Vec<Event>,
) {
    for id in need_stop(state, candidate) {
        let Some(participant) = state.participants.get_mut(&id) else {
            continue;
        };
        if participant.status == ParticipantStatus::Working {
            participant.status = ParticipantStatus::StopRequested;
            participant.since = now;
            participant.stop_reason = Some(StopReason {
                by: candidate.to_owned(),
                purpose: purpose.to_owned(),
            });
            events.push(Event::StopRequested {
                id,
                by: candidate.to_owned(),
            });
        }
    }
}

/// 申し込みの早い順に見て、渡せる最初の申し込みへ並べた机を全部渡す。渡したら真。
fn grant_one(
    state: &mut State,
    candidate: Option<&str>,
    now: u64,
    events: &mut Vec<Event>,
) -> bool {
    let first = request_order(&state.requests)
        .into_iter()
        .find(|&nth| grantable(state, &state.requests[nth], candidate));
    let Some(nth) = first else {
        return false;
    };
    let request = state.requests.remove(nth);
    let sole = request.desks.has_sole();
    // ひとり占めの番では、止まる必要のある全員が「止まった」（決まり 4）なので、それが止まった一覧。
    let stopped = if sole {
        need_stop(state, &request.id)
    } else {
        Vec::new()
    };
    events.push(Event::DeskGranted {
        id: request.id.clone(),
        desks: request.desks,
        stopped: stopped.clone(),
    });
    if sole {
        // 止まっていた者が番を受けたら、自分の負荷テストのために「作業中」へ戻る（決まり 5）。
        if let Some(participant) = state.participants.get_mut(&request.id) {
            resume(participant, "sole-granted", now, events);
        }
        state.sole = Some(SoleHolder {
            id: request.id.clone(),
            purpose: request.purpose.clone(),
            requested: request.requested,
            granted: now,
            running: false,
            stopped,
        });
    }
    if request.desks.has_test() {
        state.test = Some(TestHolder {
            id: request.id,
            purpose: request.purpose,
            requested: request.requested,
            granted: now,
        });
    }
    true
}

/// その申し込みに、並べた机が全部渡せるか（決まり 1・3〜5）。
fn grantable(state: &State, request: &DeskRequest, candidate: Option<&str>) -> bool {
    if request.desks.has_test() && state.test.is_some() {
        return false;
    }
    let status = |id: &str| {
        let participant = state.participants.get(id);
        participant.map_or(ParticipantStatus::Working, |participant| participant.status)
    };
    if request.desks.has_sole() {
        let merging = state.merge.values().any(|desk| desk.holder.is_some());
        let stopped = |id: &String| status(id) == ParticipantStatus::Stopped;
        return state.sole.is_none()
            && candidate == Some(request.id.as_str())
            && !merging
            && need_stop(state, &request.id).iter().all(stopped);
    }
    // 排他テストの机だけの申し込み。
    let own = status(&request.id);
    let sole_waited = state.requests.iter().any(|other| other.desks.has_sole());
    own != ParticipantStatus::Stopped
        && (!sole_waited
            || own == ParticipantStatus::StopRequested
            || holds_merge(state, &request.id))
}

/// マージの番: 持ち主の居ないリポジトリごとに、バグ優先 → 申し込みの時刻が早い順で 1 人を
/// 持ち主にする。リポジトリ同士は独立。
fn grant_merges(state: &mut State, now: u64, events: &mut Vec<Event>) {
    for (repo, desk) in &mut state.merge {
        if desk.holder.is_some() {
            continue;
        }
        let Some(&first) = merge_order(&desk.queue).first() else {
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

/// その識別が持つ排他テストの机・ひとり占めの組（持っていなければ `None`）。`wait` と `cli` も使う。
pub fn held(state: &State, id: &str) -> Option<DeskSet> {
    let test = state.test.as_ref().is_some_and(|holder| holder.id == id);
    let sole = state.sole.as_ref().is_some_and(|holder| holder.id == id);
    DeskSet::from_parts(test, sole)
}

/// マージの待ち行列の、番の来る順の添字: バグ優先 → 申し込みの早い順・同点は行列の前。
/// 番を選ぶ鍵はここにしか書かない（読み物と `pos=` も同じ順で並べる）。
pub fn merge_order(queue: &[MergeRequest]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..queue.len()).collect();
    order.sort_by_key(|&index| (!queue[index].bug, queue[index].requested));
    order
}

/// 机の申し込みの来る順の添字: 申し込みの早い順・同点は行列の前（読み物と `pos=` も同じ順）。
pub fn request_order(requests: &[DeskRequest]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..requests.len()).collect();
    order.sort_by_key(|&index| requests[index].requested);
    order
}

#[cfg(test)]
#[path = "plan_turns_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "plan_liveness_tests.rs"]
mod liveness_tests;
