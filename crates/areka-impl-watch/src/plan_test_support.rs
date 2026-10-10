//! `plan` のテストの支え: 偽の生死・状態の組み立て・時刻の定数。

use super::{Applied, Command, Event, Presence, Verdict, apply};
use crate::state::{
    LoadHolder, LoadRequest, MergeHolder, MergeRequest, Participant, ParticipantStatus, State,
    StopReason, WaitKind, WaitRecord, WatchInfo,
};

/// テストの「いま」の起点（UNIX 秒）。後の時刻は `T0 + n` と書く。
pub(super) const T0: u64 = 1_791_000_000;

/// 支えの組み立てる参加者のリポジトリ。
pub(super) const REPO: &str = "areka";

/// 支えの組み立てる見張り・待ちのプロセス番号。
pub(super) const PID: u32 = 1234;

/// 支えの組み立てる負荷テストの内容（停止要請の理由にも同じものを書く）。
pub(super) const PURPOSE: &str = "負荷の計測";

/// 偽の生死。何も言わなければ誰のどの待ちも「居る」と答え、「居ない」は
/// [`FakePresence::absent`] で名指ししたものだけ（生死を主題にしないテストの参加者が、
/// 回収で消えないようにする）。
#[derive(Default)]
pub(super) struct FakePresence {
    absent: Vec<(String, WaitKind)>,
}

impl FakePresence {
    pub(super) fn all_present() -> Self {
        Self::default()
    }

    /// その識別の、その種類の待ちだけを「居ない」にする。
    pub(super) fn absent(mut self, id: &str, kind: WaitKind) -> Self {
        self.absent.push((id.to_owned(), kind));
        self
    }
}

impl Presence for FakePresence {
    fn is_present(&self, id: &str, kind: WaitKind) -> bool {
        !self.absent.iter().any(|(i, k)| i == id && *k == kind)
    }
}

/// 「作業中」で見張りの立っている参加者（見張りの開始が作る姿）だけを持つ状態。
/// 名前は識別と同じ、リポジトリは [`REPO`]、時刻は [`T0`]。
pub(super) fn state_with(ids: &[&str]) -> State {
    let mut state = State::empty();
    for id in ids {
        state.participants.insert(
            (*id).to_owned(),
            Participant {
                id: (*id).to_owned(),
                name: (*id).to_owned(),
                repo: REPO.to_owned(),
                since: T0,
                watch: Some(WatchInfo {
                    pid: PID,
                    since: T0,
                }),
                ..Participant::default()
            },
        );
        state.waits.push(wait_record(id, WaitKind::Watch));
    }
    state
}

/// 支えの定数（[`REPO`]・[`PID`]・[`T0`]）で作った待ちの記録。
pub(super) fn wait_record(id: &str, kind: WaitKind) -> WaitRecord {
    WaitRecord {
        id: id.to_owned(),
        kind,
        repo: Some(REPO.to_owned()),
        pid: PID,
        since: T0,
    }
}

/// マージの待ち行列の末尾へ、バグでない申し込みを足す。
pub(super) fn queue_merge(state: &mut State, repo: &str, id: &str, requested: u64) {
    let desk = state.merge.entry(repo.to_owned()).or_default();
    desk.queue.push(MergeRequest {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
        requested,
    });
}

/// マージの机の持ち主にする。
pub(super) fn hold_merge(state: &mut State, repo: &str, id: &str) {
    let desk = state.merge.entry(repo.to_owned()).or_default();
    desk.holder = Some(MergeHolder {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
        requested: T0,
        granted: T0,
    });
}

/// 負荷テストの待ち行列の末尾へ申し込みを足す。
pub(super) fn queue_load(state: &mut State, id: &str, requested: u64) {
    state.load.queue.push(LoadRequest {
        id: id.to_owned(),
        purpose: PURPOSE.to_owned(),
        requested,
    });
}

/// 番の来た負荷テストの持ち主にする。`stopped` は番が来たときに止まっていた識別。
pub(super) fn hold_load(state: &mut State, id: &str, stopped: &[&str]) {
    state.load.holder = Some(LoadHolder {
        id: id.to_owned(),
        purpose: PURPOSE.to_owned(),
        requested: T0,
        granted: T0,
        running: false,
        stopped: stopped.iter().map(|id| (*id).to_owned()).collect(),
    });
}

/// 「すでに走っている負荷テスト」の持ち主にする（走っている印つき＝誰にも停止要請が出ない）。
pub(super) fn hold_running_load(state: &mut State, id: &str) {
    hold_load(state, id, &[]);
    state.load.holder.as_mut().expect("いま置いた").running = true;
}

/// `by` の負荷テストのための「停止要請中」にする。
pub(super) fn ask_to_stop(state: &mut State, id: &str, by: &str) {
    let participant = state.participants.get_mut(id).expect("参加者が居る");
    participant.status = ParticipantStatus::StopRequested;
    participant.stop_reason = Some(StopReason {
        by: by.to_owned(),
        purpose: PURPOSE.to_owned(),
    });
}

/// `by` の負荷テストのために「止まった」にする。
pub(super) fn stop_for(state: &mut State, id: &str, by: &str) {
    ask_to_stop(state, id, by);
    state.participants.get_mut(id).expect("参加者が居る").status = ParticipantStatus::Stopped;
}

/// 名前を省いた、内容が [`PURPOSE`] の負荷テストの申し込み（支えの組み立てる申し込みと同じ綴り）。
pub(super) fn load_test(id: &str) -> Command {
    Command::LoadTest {
        id: id.to_owned(),
        name: None,
        repo: REPO.to_owned(),
        purpose: PURPOSE.to_owned(),
    }
}

pub(super) fn load_done(id: &str) -> Command {
    Command::LoadDone { id: id.to_owned() }
}

pub(super) fn stopped(id: &str) -> Command {
    Command::Stopped { id: id.to_owned() }
}

/// 全員が居る机で、判断を 1 回呼ぶ。
pub(super) fn run(state: &mut State, cmd: &Command, caller: Option<&str>, now: u64) -> Applied {
    apply(state, cmd, caller, now, &FakePresence::all_present())
}

pub(super) fn applied(changed: bool, events: Vec<Event>) -> Applied {
    Applied {
        changed,
        verdict: Verdict::Applied,
        events,
    }
}

/// 「当てはまらなかった」の返事: 状態を変えず、出来事も無い。断りの文は端末へそのまま出すので ASCII。
pub(super) fn assert_not_applied(got: &Applied, label: &str) {
    assert!(!got.changed, "{label}");
    assert!(got.events.is_empty(), "{label}: {:?}", got.events);
    let Verdict::NotApplied(text) = got.verdict else {
        panic!("{label}: 当てはまらなかった、を返す: {:?}", got.verdict);
    };
    assert!(!text.is_empty() && text.is_ascii(), "{label}: {text}");
}

#[test]
fn fake_presence_answers_present_unless_told_absent() {
    let presence = FakePresence::all_present().absent("A", WaitKind::Merge);
    assert!(presence.is_present("A", WaitKind::Watch));
    assert!(presence.is_present("B", WaitKind::Merge));
    assert!(!presence.is_present("A", WaitKind::Merge));
}
