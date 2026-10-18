//! 判断の中核の子: 番の決め直し（停止要請 → 負荷テストの段 → 再開 → マージの番）と、待ち行列の
//! 番の来る順。親の `plan.rs` と同じく、ファイルも時計もプロセスもログも触らない。

use super::{Event, resume};
use crate::state::{
    LoadHolder, LoadRequest, MergeHolder, MergeRequest, ParticipantStatus, State, StopReason,
};

/// 番を決め直す（元のスクリプトの `Invoke-Plan` の写し）: 停止要請の発行 → 負荷テストの番 →
/// 再開 → マージの番。何度呼んでも同じ結果になる。
pub(super) fn replan(state: &mut State, now: u64, events: &mut Vec<Event>) {
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

/// 負荷テストの段: 停止要請の発行 → 負荷テストの番。負荷テストが持たれても待たれてもいなければ
/// 何もせずに偽を返す。
fn plan_load(state: &mut State, now: u64, events: &mut Vec<Event>) -> bool {
    let load = &state.load;
    // 待ち行列の先頭＝番の来る順の先頭。
    let first = load_order(&load.queue)
        .first()
        .map(|&index| (index, &load.queue[index]));
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

/// マージの待ち行列の、番の来る順の添字: バグ優先 → 申し込みの早い順・同点は行列の前。
/// 番を選ぶ鍵はここにしか書かない（読み物と `pos=` も同じ順で並べる）。
pub fn merge_order(queue: &[MergeRequest]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..queue.len()).collect();
    order.sort_by_key(|&index| (!queue[index].bug, queue[index].requested));
    order
}

/// 負荷テストの待ち行列の、番の来る順の添字: 申し込みの早い順・同点は行列の前。
pub fn load_order(queue: &[LoadRequest]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..queue.len()).collect();
    order.sort_by_key(|&index| queue[index].requested);
    order
}
