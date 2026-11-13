//! `plan` のテストの支え: 偽の生死・状態とコマンドと出来事の組み立て。

use super::{Applied, Command, Event, Presence, Verdict, apply};
use crate::state::{
    DeskRequest, DeskSet, MergeRequest, Participant, ParticipantStatus, Recent, RecentKind, State,
    StopReason, WaitKind, WaitRecord, WatchInfo,
};
use crate::test_support::{T0, merge_holder, sole_holder, test_holder};

/// 支えの組み立てる参加者のリポジトリ。
pub(super) const REPO: &str = "areka";

/// 支えの組み立てる見張り・待ちのプロセス番号。
pub(super) const PID: u32 = 1234;

/// 支えの組み立てる申し込みの内容（停止要請の理由にも同じものを書く）。
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
                watch: Some(WatchInfo {
                    pid: PID,
                    since: T0,
                }),
                ..new_participant(id, id, REPO, T0)
            },
        );
        state.waits.push(wait_record(id, WaitKind::Watch));
    }
    state
}

/// `since` に参加した直後の参加者（「作業中」・見張りも停止要請の訳も無い）。
pub(super) fn new_participant(id: &str, name: &str, repo: &str, since: u64) -> Participant {
    Participant {
        id: id.to_owned(),
        name: name.to_owned(),
        repo: repo.to_owned(),
        since,
        ..Participant::default()
    }
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

/// マージの机の持ち主にする（持ち主は支えの申し込みと同じ綴り）。
pub(super) fn hold_merge(state: &mut State, repo: &str, id: &str) {
    state.merge.entry(repo.to_owned()).or_default().holder = merge_holder(id, T0, T0);
}

/// 机の申し込みの待ち行列の末尾へ、内容が [`PURPOSE`] の申し込みを足す。
pub(super) fn queue_request(state: &mut State, id: &str, desks: DeskSet, requested: u64) {
    state.requests.push(DeskRequest {
        id: id.to_owned(),
        desks,
        purpose: PURPOSE.to_owned(),
        requested,
    });
}

/// 排他テストの机の持ち主にする。
pub(super) fn hold_test(state: &mut State, id: &str) {
    state.test = test_holder(id, PURPOSE, T0, T0);
}

/// 番の来たひとり占めの持ち主にする。`stopped` は番が来たときに止まっていた識別。
pub(super) fn hold_sole(state: &mut State, id: &str, stopped: &[&str]) {
    state.sole = sole_holder(id, PURPOSE, T0, T0, false, stopped);
}

/// 負荷テスト（排他テストの机とひとり占めの 1 組）の番が来た持ち主にする（走っている印なし）。
/// `stopped` は番が来たときに止まっていた識別。
pub(super) fn hold_load(state: &mut State, id: &str, stopped: &[&str]) {
    hold_test(state, id);
    hold_sole(state, id, stopped);
}

/// 机の申し込みの待ち行列の末尾へ、負荷テスト（`test,sole`）の申し込みを足す。
pub(super) fn queue_load(state: &mut State, id: &str, requested: u64) {
    queue_request(state, id, DeskSet::TestSole, requested);
}

/// 「すでに走っている負荷テスト」の持ち主にする（排他テストの机とひとり占めの両方・走っている
/// 印つき＝誰にも停止要請が出ない）。
pub(super) fn hold_running_load(state: &mut State, id: &str) {
    hold_test(state, id);
    hold_sole(state, id, &[]);
    state.sole.as_mut().expect("いま置いた").running = true;
}

/// `by` のひとり占めのための「停止要請中」にする。
pub(super) fn ask_to_stop(state: &mut State, id: &str, by: &str) {
    let participant = state.participants.get_mut(id).expect("参加者が居る");
    participant.status = ParticipantStatus::StopRequested;
    participant.stop_reason = Some(StopReason {
        by: by.to_owned(),
        purpose: PURPOSE.to_owned(),
    });
}

/// `by` のひとり占めのために「止まった」にする。
pub(super) fn stop_for(state: &mut State, id: &str, by: &str) {
    ask_to_stop(state, id, by);
    state.participants.get_mut(id).expect("参加者が居る").status = ParticipantStatus::Stopped;
}

/// 名前を省いた、内容が [`PURPOSE`] の机の申し込み（支えの組み立てる申し込みと同じ綴り）。
pub(super) fn request(id: &str, desks: DeskSet) -> Command {
    Command::Request {
        id: id.to_owned(),
        name: None,
        repo: REPO.to_owned(),
        desks,
        purpose: PURPOSE.to_owned(),
    }
}

/// 返す。`None` はマージの机のほかの全部。
pub(super) fn give_back(id: &str, desks: Option<DeskSet>) -> Command {
    Command::Return {
        id: id.to_owned(),
        desks,
    }
}

/// 負荷テスト（`test,sole`）の申し込み。
pub(super) fn load_test(id: &str) -> Command {
    request(id, DeskSet::TestSole)
}

/// 負荷テストが済んだ: 名指しせずに全部を返す。
pub(super) fn load_done(id: &str) -> Command {
    give_back(id, None)
}

pub(super) fn stopped(id: &str) -> Command {
    Command::Stopped { id: id.to_owned() }
}

/// 名前を省いた、spec が `spec-<識別>` のマージの申し込み（支えの組み立てる申し込みと同じ綴り）。
pub(super) fn merge(id: &str, repo: &str, bug: bool) -> Command {
    Command::Merge {
        id: id.to_owned(),
        name: None,
        repo: repo.to_owned(),
        spec: format!("spec-{id}"),
        bug,
    }
}

/// マージが済んだ（PR は "281"・コミットは "414d43eb"）。
pub(super) fn merged(id: &str) -> Command {
    Command::Merged {
        id: id.to_owned(),
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
    }
}

/// 名前を省いた、内容が [`PURPOSE`] の「すでに走っている負荷テスト」の記録。
pub(super) fn load_running(id: &str) -> Command {
    Command::LoadRunning {
        id: id.to_owned(),
        name: None,
        repo: REPO.to_owned(),
        purpose: PURPOSE.to_owned(),
    }
}

/// 全員が居る机で、判断を 1 回呼ぶ。
pub(super) fn run(state: &mut State, cmd: &Command, caller: Option<&str>, now: u64) -> Applied {
    apply(state, cmd, caller, now, &FakePresence::all_present())
}

/// その種類の待ちの記録が置かれた（置き換えを含む）出来事。
pub(super) fn wait_registered(id: &str, kind: WaitKind) -> Event {
    Event::WaitRegistered {
        id: id.to_owned(),
        kind,
    }
}

/// 見張りの記録が置かれた（置き換えを含む）出来事。
pub(super) fn watch_registered(id: &str) -> Event {
    wait_registered(id, WaitKind::Watch)
}

/// 机の申し込みが並んだ出来事。
pub(super) fn desk_requested(id: &str, desks: DeskSet) -> Event {
    Event::DeskRequested {
        id: id.to_owned(),
        desks,
    }
}

/// 止まったと伝えた出来事。
pub(super) fn stop_reported(id: &str) -> Event {
    Event::Stopped { id: id.to_owned() }
}

/// 「すでに走っている負荷テスト」を記録した出来事。
pub(super) fn recorded_running(id: &str) -> Event {
    Event::LoadRunning { id: id.to_owned() }
}

/// 机の番が来た出来事。`stopped` は番が来たときに止まっていた識別。
pub(super) fn granted(id: &str, desks: DeskSet, stopped: &[&str]) -> Event {
    Event::DeskGranted {
        id: id.to_owned(),
        desks,
        stopped: stopped.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// 見張りの開始。`name` を省くと名前は識別（初回）か元のまま。
pub(super) fn watch(id: &str, name: Option<&str>, repo: &str, pid: u32) -> Command {
    Command::Watch {
        id: id.to_owned(),
        name: name.map(str::to_owned),
        repo: repo.to_owned(),
        pid,
    }
}

/// 参加した出来事。
pub(super) fn joined(id: &str) -> Event {
    Event::Joined { id: id.to_owned() }
}

/// [`merge`] の申し込みが並んだ出来事（spec は `spec-<識別>`）。
pub(super) fn merge_requested(id: &str, repo: &str, bug: bool) -> Event {
    Event::MergeRequested {
        repo: repo.to_owned(),
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug,
    }
}

/// マージの机の番が来た出来事。
pub(super) fn merge_granted(id: &str, repo: &str) -> Event {
    Event::MergeGranted {
        repo: repo.to_owned(),
        id: id.to_owned(),
    }
}

/// [`merged`] の出来事（PR とコミットも [`merged`] と同じ綴り）。
pub(super) fn merge_done(id: &str, repo: &str) -> Event {
    Event::Merged {
        repo: repo.to_owned(),
        id: id.to_owned(),
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
    }
}

/// 離脱。
pub(super) fn leave(id: &str) -> Command {
    Command::Leave { id: id.to_owned() }
}

/// 取り下げ。
pub(super) fn cancel(id: &str) -> Command {
    Command::Cancel { id: id.to_owned() }
}

/// 参加を終えた出来事。`why` は終えた訳の綴り。
pub(super) fn left(id: &str, why: &'static str) -> Event {
    Event::Left {
        id: id.to_owned(),
        why,
    }
}

/// 取り下げた出来事。
pub(super) fn cancelled(id: &str) -> Event {
    Event::Cancelled { id: id.to_owned() }
}

/// 机を返した出来事。
pub(super) fn returned(id: &str, desks: DeskSet) -> Event {
    Event::DeskReturned {
        id: id.to_owned(),
        desks,
    }
}

/// 「作業中」へ戻った出来事。`why` は戻った訳の綴り。
pub(super) fn resumed(id: &str, why: &'static str) -> Event {
    Event::Resumed {
        id: id.to_owned(),
        why,
    }
}

/// 排他テストの机が空いた出来事。`why` は空けた訳の綴り。
pub(super) fn test_freed(id: &str, why: &'static str) -> Event {
    Event::TestFreed {
        id: id.to_owned(),
        why,
    }
}

/// 排他テストの机が空いた「最近の出来事」1 件。
pub(super) fn test_freed_record(id: &str, why: &str, at: u64) -> Recent {
    Recent {
        at,
        kind: RecentKind::TestFreed,
        id: Some(id.to_owned()),
        detail: why.to_owned(),
    }
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
