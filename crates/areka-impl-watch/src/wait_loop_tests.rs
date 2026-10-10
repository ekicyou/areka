//! 待ちのループの決定論テスト。偽の口（[`FakePort`]）で回し、実時間を使わない。
//!
//! 偽の口は状態を 1 つ抱え、「状態を変える 1 回」を本物の判断（`plan::apply`）に通す。経過は
//! 眠った回数だけで数え（1 回＝1 秒）、ほかの参加者の動きは「何回目の眠りの後に起きるか」で
//! 並べる。ループが口へ頼んだことは 1 字ずつの跡に残し、順序ごと比べる:
//!
//! `W`＝見張りの開始　`R`＝待ちの記録の登録　`f`＝更新時刻と大きさ　`r`＝読み　`s`＝眠り
//! `T`＝周期の一回り　`U`＝記録の抹消
//!
//! 本物のファイルは、居る印のロックファイル 1 つだけ（`target\` の下。握った印はファイルを
//! 握らずには作れない）。

use std::cell::{Cell, RefCell};
use std::fs::{File, TryLockError};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use temp_path_kit::TempPath;

use super::test_support::{T0, load, merge, resume, watch};
use super::{WaitEnd, WaitPort, WaitSpec, run};
use crate::error::WatchError;
use crate::plan::{Applied, Command, Presence, apply};
use crate::presence::hold;
use crate::state::{State, WaitKind, WaitRecord};

/// 待ちのプロセスの番号（偽の口が答える）。
const OWN_PID: u32 = 4321;

/// 終わらない待ちでテストが止まらないための上限（どのテストもここまで眠らない）。
const SLEEP_LIMIT: u64 = 10_000;

/// ほかの参加者の動き。
enum Step {
    /// ほかの参加者の呼び出し。本物の判断に通り、変われば状態ファイルが書かれる。
    Call(&'static str, Command),
    /// 同じ呼び出しだが、更新時刻も大きさも変わらなかった書き。
    Quiet(&'static str, Command),
    /// その識別の見張りのプロセスが落ちる（状態ファイルは変わらない）。
    Dies(&'static str),
    /// 状態ファイルが無くなる（`clear` の改名の一瞬）。
    Vanish,
}

/// 失敗の差し込み: 跡の 1 字と眠った回数を見て、失敗させるならその失敗を返す。
type Fault = Box<dyn Fn(char, u64) -> Option<WatchError>>;

struct FakePort {
    /// 状態ファイル（`None`＝無い）。
    state: RefCell<Option<State>>,
    /// 状態ファイルの更新時刻の代わり。書かれるたびに 1 進む。
    stamp: Cell<u64>,
    sleeps: Cell<u64>,
    trace: RefCell<String>,
    /// ループが頼んだ「状態を変える 1 回」と、その呼び手。
    changes: RefCell<Vec<(Command, String)>>,
    /// 「状態を変える 1 回」のたびに、居る印が握られていたか。
    held_at_change: RefCell<Vec<bool>>,
    /// (何回目の眠りの後か, 動き)。
    script: RefCell<Vec<(u64, Step)>>,
    /// 見張りのプロセスが居ない識別。
    dead: RefCell<Vec<&'static str>>,
    fault: Fault,
    /// 居る印のロックファイルの置き場（落とすと消える）。
    root: TempPath,
}

impl FakePort {
    fn new() -> Self {
        FakePort {
            state: RefCell::new(None),
            stamp: Cell::new(0),
            sleeps: Cell::new(0),
            trace: RefCell::default(),
            changes: RefCell::default(),
            held_at_change: RefCell::default(),
            script: RefCell::default(),
            dead: RefCell::default(),
            fault: Box::new(|_, _| None),
            root: TempPath::under_target("impl-watch-wait"),
        }
    }

    fn failing(mut self, fault: impl Fn(char, u64) -> Option<WatchError> + 'static) -> Self {
        self.fault = Box::new(fault);
        self
    }

    /// `sleeps` 回目の眠りの後に起きる動きを足す。
    fn at(&self, sleeps: u64, step: Step) {
        self.script.borrow_mut().push((sleeps, step));
    }

    /// 待ちが始まる前の、参加者の呼び出し。
    fn does(&self, who: &str, cmd: Command) {
        self.apply_as(who, &cmd, true);
    }

    /// 本物の判断に通す。`stamped` なら、書いたときに更新時刻が進む（状態ファイルが無ければ
    /// 作る・変われば書く、は本物の口と同じ）。
    fn apply_as(&self, who: &str, cmd: &Command, stamped: bool) -> Applied {
        let mut state = self.state.borrow_mut();
        let created = state.is_none();
        let state = state.get_or_insert_with(State::empty);
        let applied = apply(state, cmd, Some(who), self.now(), self);
        if stamped && (created || applied.changed) {
            self.stamp.set(self.stamp.get() + 1);
        }
        applied
    }

    fn lock_path(&self) -> PathBuf {
        self.root.path().join("own.lock")
    }

    /// 居る印がいま握られているか（別のハンドルで探る）。
    fn lock_is_held(&self) -> bool {
        let file = File::open(self.lock_path()).expect("ロックファイルが在る");
        match file.try_lock() {
            Ok(()) => false,
            Err(TryLockError::WouldBlock) => true,
            Err(TryLockError::Error(err)) => panic!("探れない: {err}"),
        }
    }

    /// 居る印を握って待ちを回す。
    fn run(&self, spec: &WaitSpec) -> Result<WaitEnd, WatchError> {
        let held = hold(&self.lock_path())
            .expect("握れる")
            .expect("誰も握っていない");
        run(spec, self, held)
    }

    /// 跡に 1 字残し、差し込まれた失敗があれば返す。
    fn note(&self, call: char) -> Result<(), WatchError> {
        self.trace.borrow_mut().push(call);
        (self.fault)(call, self.sleeps.get()).map_or(Ok(()), Err)
    }

    fn trace(&self) -> String {
        self.trace.borrow().clone()
    }

    /// その識別の待ち・見張りの記録の種類（状態に残っているもの）。
    fn waits_of(&self, id: &str) -> Vec<WaitKind> {
        let state = self.state.borrow();
        let waits = state.iter().flat_map(|state| &state.waits);
        waits.filter(|w| w.id == id).map(|w| w.kind).collect()
    }
}

impl Presence for FakePort {
    /// 落ちたと名指しされた見張りの他は、誰のどの待ちも「居る」。
    fn is_present(&self, id: &str, kind: WaitKind) -> bool {
        !(kind == WaitKind::Watch && self.dead.borrow().contains(&id))
    }
}

impl WaitPort for FakePort {
    /// 眠った回数だけが経過（1 回＝1 秒）。
    fn now(&self) -> u64 {
        T0 + self.sleeps.get()
    }

    fn pid(&self) -> u32 {
        OWN_PID
    }

    fn fingerprint(&self) -> Result<Option<(SystemTime, u64)>, WatchError> {
        self.note('f')?;
        let modified = UNIX_EPOCH + Duration::from_secs(self.stamp.get());
        Ok(self.state.borrow().as_ref().map(|_| (modified, 0)))
    }

    fn read(&self) -> Result<Option<State>, WatchError> {
        self.note('r')?;
        Ok(self.state.borrow().clone())
    }

    fn change(&self, cmd: &Command, caller: &str) -> Result<Applied, WatchError> {
        self.changes
            .borrow_mut()
            .push((cmd.clone(), caller.to_owned()));
        self.held_at_change.borrow_mut().push(self.lock_is_held());
        self.note(match cmd {
            Command::Watch { .. } => 'W',
            Command::RegisterWait { .. } => 'R',
            Command::Tick => 'T',
            Command::UnregisterWait { .. } => 'U',
            _ => '?',
        })?;
        Ok(self.apply_as(caller, cmd, true))
    }

    fn sleep(&self) {
        self.trace.borrow_mut().push('s');
        let sleeps = self.sleeps.get() + 1;
        self.sleeps.set(sleeps);
        assert!(sleeps <= SLEEP_LIMIT, "the wait did not end");
        for (_, step) in self.script.borrow().iter().filter(|(at, _)| *at == sleeps) {
            match step {
                Step::Call(who, cmd) => drop(self.apply_as(who, cmd, true)),
                Step::Quiet(who, cmd) => drop(self.apply_as(who, cmd, false)),
                Step::Dies(who) => self.dead.borrow_mut().push(who),
                Step::Vanish => *self.state.borrow_mut() = None,
            }
        }
    }
}

// ---- 参加者の呼び出しと、場面の組み立て ----

fn watching(id: &str) -> Command {
    Command::Watch {
        id: id.to_owned(),
        name: None,
        repo: "areka".to_owned(),
        pid: 1,
    }
}

fn asks_merge(id: &str) -> Command {
    Command::Merge {
        id: id.to_owned(),
        name: None,
        repo: "areka".to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
    }
}

fn merged(id: &str) -> Command {
    Command::Merged {
        id: id.to_owned(),
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
    }
}

fn asks_load(id: &str) -> Command {
    Command::LoadTest {
        id: id.to_owned(),
        name: None,
        repo: "areka".to_owned(),
        purpose: "負荷の計測 5 回".to_owned(),
    }
}

/// A がマージの机を持ち、B がその後ろに並んでいる（2 人とも見張りつき）。
fn b_queued_behind_a() -> FakePort {
    let port = FakePort::new();
    for id in ["A", "B"] {
        port.does(id, watching(id));
        port.does(id, asks_merge(id));
    }
    port
}

/// A と C が見張りつきで参加し、C が負荷テストを申し込んだ（A は「停止要請中」・C は待ち行列）。
fn c_asked_for_a_load_test() -> FakePort {
    let port = FakePort::new();
    port.does("A", watching("A"));
    port.does("C", watching("C"));
    port.does("C", asks_load("C"));
    port
}

fn done(line: &str) -> WaitEnd {
    WaitEnd::Done(line.to_owned())
}

/// 何も変わらない 1 秒 `n` 回分の跡（眠って、更新時刻と大きさを見るだけ）。
fn idle(n: usize) -> String {
    "sf".repeat(n)
}

/// 何も変わらない 30 秒分の跡: 10 回目と 20 回目は読み、30 回目は周期の一回りの後で読む。
fn idle_thirty() -> String {
    let ten = format!("{}sfr", idle(9));
    format!("{ten}{ten}{}sTfr", idle(9))
}

fn unregister(id: &str, kind: WaitKind) -> (Command, String) {
    let cmd = Command::UnregisterWait {
        id: id.to_owned(),
        kind,
    };
    (cmd, id.to_owned())
}

/// 見張り以外の待ちの登録（待ちの記録）。
fn register(id: &str, kind: WaitKind, repo: Option<&str>) -> (Command, String) {
    let record = WaitRecord {
        id: id.to_owned(),
        kind,
        repo: repo.map(str::to_owned),
        pid: OWN_PID,
        since: T0,
    };
    (Command::RegisterWait { record }, id.to_owned())
}

// ---- 終わり方 ----

/// 待ちを始め直したとき、すでに番が来ていれば眠らずに終わる（要件 6.2）。
#[test]
fn ends_without_sleeping_when_the_turn_has_already_come() {
    let port = FakePort::new();
    port.does("B", watching("B"));
    port.does("B", asks_merge("B"));

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(end, done("granted merge repo=areka; last: none"));
    assert_eq!(port.trace(), "RfrU");
    assert_eq!(port.sleeps.get(), 0);
    assert_eq!(port.waits_of("B"), [WaitKind::Watch]);
}

#[test]
fn ends_when_the_turn_comes() {
    let port = b_queued_behind_a();
    port.at(3, Step::Call("A", merged("A")));

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(
        end,
        done("granted merge repo=areka; last: PR#281 414d43eb spec-A 2026-10-03T04:00:03Z")
    );
    // 変わらない 2 秒は読まず、変わった 3 秒目に読んで終わり、最後に自分の記録を消す。
    assert_eq!(port.trace(), format!("Rfr{}sfrU", idle(2)));
    assert_eq!(port.sleeps.get(), 3);
    assert_eq!(port.waits_of("B"), [WaitKind::Watch]);
}

/// 申し込みが取り下げで消えたら「消えた」で終わり、残った待ちの記録は自分で消す。
#[test]
fn ends_as_gone_when_the_request_disappears() {
    let port = b_queued_behind_a();
    let cancel = Command::Cancel { id: "B".to_owned() };
    port.at(2, Step::Call("B", cancel));

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(end, WaitEnd::Gone("request gone"));
    assert_eq!(port.trace(), format!("Rfr{}sfrU", idle(1)));
    assert_eq!(port.waits_of("B"), [WaitKind::Watch]);
}

/// 参加者の記録が離脱で消えたら、見張りは「消えた」で終わる（消す記録が無くても抹消は呼ぶ）。
#[test]
fn a_watch_ends_as_gone_when_the_participant_leaves() {
    let port = FakePort::new();
    let leave = Command::Leave { id: "A".to_owned() };
    port.at(1, Step::Call("A", leave));

    let end = port.run(&watch("A")).expect("終わる");

    assert_eq!(end, WaitEnd::Gone("removed"));
    assert_eq!(port.trace(), "WfrsfrU");
    assert_eq!(port.waits_of("A"), []);
}

/// 状態ファイルが無くなったら（`clear` の改名の一瞬）、次の 1 秒で読んで「消えた」で終わる。
#[test]
fn ends_as_gone_when_the_state_file_vanishes() {
    let port = b_queued_behind_a();
    port.at(4, Step::Vanish);

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(end, WaitEnd::Gone("request gone"));
    assert_eq!(port.trace(), format!("Rfr{}sfrU", idle(3)));
}

// ---- 読み直し ----

/// 更新時刻と大きさが変わっていなければ中身を読まない。10 回目は変わっていなくても読む。
#[test]
fn an_unchanged_file_is_not_read_until_the_tenth_poll() {
    let port = b_queued_behind_a();
    // 3 秒目に番が来るが、更新時刻にも大きさにも映らない。
    port.at(3, Step::Quiet("A", merged("A")));

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(
        end,
        done("granted merge repo=areka; last: PR#281 414d43eb spec-A 2026-10-03T04:00:03Z")
    );
    assert_eq!(port.trace(), format!("Rfr{}sfrU", idle(9)));
    assert_eq!(port.sleeps.get(), 10);
}

/// 10 回目ごとの読みは、始めからの眠りの回数で数える（途中の読みで数え直さない）。
#[test]
fn the_forced_read_comes_on_every_tenth_poll_since_the_start() {
    let port = b_queued_behind_a();
    // 4 秒目の変化（番は来ない）は直ちに読まれる。
    port.at(4, Step::Call("C", watching("C")));
    port.at(11, Step::Quiet("A", merged("A")));

    port.run(&merge("B", "areka")).expect("終わる");

    let expected = format!("Rfr{}sfr{}sfr{}sfrU", idle(3), idle(5), idle(9));
    assert_eq!(port.trace(), expected);
    assert_eq!(port.sleeps.get(), 20);
}

/// 変化は 1 度だけ読む（読んだ後は、その更新時刻と大きさが「変わっていない」の基準になる）。
#[test]
fn a_change_that_does_not_end_the_wait_is_read_once() {
    let port = b_queued_behind_a();
    port.at(2, Step::Call("C", watching("C")));
    port.at(5, Step::Call("A", merged("A")));

    port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(port.trace(), format!("Rfr{}sfr{}sfrU", idle(1), idle(2)));
}

// ---- 周期の一回り ----

#[test]
fn the_periodic_round_comes_on_the_30th_and_the_60th_poll() {
    let port = b_queued_behind_a();
    port.at(61, Step::Call("A", merged("A")));

    port.run(&merge("B", "areka")).expect("終わる");

    let thirty = idle_thirty();
    assert_eq!(port.trace(), format!("Rfr{thirty}{thirty}sfrU"));
    // 周期の一回りの呼び手も、待っている自分。
    let changes = port.changes.borrow();
    let ticks: Vec<_> = changes
        .iter()
        .filter(|(c, _)| *c == Command::Tick)
        .collect();
    assert_eq!(ticks.len(), 2);
    assert!(ticks.iter().all(|(_, caller)| caller == "B"));
}

/// 待っている者しか居ないとき、落ちた持ち主を回収するのは待ちの周期の一回り（設計の論点 3）。
#[test]
fn the_periodic_round_reclaims_a_fallen_holder_and_the_turn_comes() {
    let port = b_queued_behind_a();
    port.at(5, Step::Dies("A"));

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(end, done("granted merge repo=areka; last: none"));
    assert_eq!(port.trace(), format!("Rfr{}U", idle_thirty()));
    assert_eq!(port.sleeps.get(), 30);
}

/// 周期の一回りが失敗（ロックの混雑）しても待ちは終わらず、読み直しも次の周期も続く。
#[test]
fn a_failed_periodic_round_does_not_end_the_wait() {
    let port = b_queued_behind_a().failing(|call, _| (call == 'T').then_some(WatchError::LockBusy));
    port.at(61, Step::Call("A", merged("A")));

    let end = port.run(&merge("B", "areka")).expect("失敗で終わらない");

    assert_eq!(
        end,
        done("granted merge repo=areka; last: PR#281 414d43eb spec-A 2026-10-03T04:01:01Z")
    );
    let thirty = idle_thirty();
    assert_eq!(port.trace(), format!("Rfr{thirty}{thirty}sfrU"));
}

/// 時間の上限は無い（要件 6.1）。何も起きない 1 時間の間、読むのは 10 秒に 1 回、周期の一回りは
/// 30 秒に 1 回だけ。
#[test]
fn an_hour_of_nothing_neither_ends_the_wait_nor_reads_more_than_it_must() {
    let port = b_queued_behind_a();
    port.at(3600, Step::Call("A", merged("A")));

    let end = port.run(&merge("B", "areka")).expect("終わる");

    assert!(matches!(end, WaitEnd::Done(_)));
    assert_eq!(port.sleeps.get(), 3600);
    let trace = port.trace();
    let count = |call| trace.chars().filter(|c| *c == call).count();
    assert_eq!(count('f'), 1 + 3600);
    assert_eq!(count('r'), 1 + 360);
    assert_eq!(count('T'), 120);
}

// ---- 失敗 ----

/// 登録の失敗は待ちの失敗。読みも眠りもせずに返る。
#[test]
fn a_failed_registration_is_the_error() {
    let port = b_queued_behind_a().failing(|call, _| (call == 'R').then_some(WatchError::LockBusy));

    let end = port.run(&merge("B", "areka"));

    assert!(matches!(end, Err(WatchError::LockBusy)), "{end:?}");
    assert_eq!(port.trace(), "R");
    assert!(!port.lock_is_held());
}

/// 読みの失敗（壊れた）は待ちを失敗で終える。自分の記録は消しに行かない（握りが解けるので、
/// 次の状態を変える呼び出しの回収が消す）。
#[test]
fn a_failed_read_ends_the_wait_with_the_error() {
    let port = b_queued_behind_a()
        .failing(|call, sleeps| (call == 'r' && sleeps == 2).then_some(WatchError::Broken));
    port.at(2, Step::Call("C", watching("C")));

    let end = port.run(&merge("B", "areka"));

    assert!(matches!(end, Err(WatchError::Broken)), "{end:?}");
    assert_eq!(port.trace(), format!("Rfr{}sfr", idle(1)));
    assert_eq!(port.waits_of("B"), [WaitKind::Watch, WaitKind::Merge]);
    assert!(!port.lock_is_held());
}

/// 最初の読みの失敗（版違い）も同じ。
#[test]
fn a_version_mismatch_on_the_first_read_is_the_error() {
    let mismatch = || WatchError::VersionMismatch { found: 2, known: 1 };
    let port = b_queued_behind_a().failing(move |call, _| (call == 'r').then(mismatch));

    let end = port.run(&merge("B", "areka"));

    assert!(
        matches!(end, Err(WatchError::VersionMismatch { found: 2, known: 1 })),
        "{end:?}"
    );
    assert_eq!(port.trace(), "Rfr");
}

/// 更新時刻と大きさが取れない失敗も、読みの失敗と同じく待ちを失敗で終える。
#[test]
fn a_failed_fingerprint_ends_the_wait_with_the_error() {
    let denied = || WatchError::Io {
        op: "stat state.json",
        kind: "PermissionDenied".to_owned(),
        code: 5,
    };
    let port =
        b_queued_behind_a().failing(move |call, sleeps| (call == 'f' && sleeps == 4).then(denied));

    let end = port.run(&merge("B", "areka"));

    assert!(
        matches!(end, Err(WatchError::Io { code: 5, .. })),
        "{end:?}"
    );
    assert_eq!(port.trace(), format!("Rfr{}sf", idle(3)));
}

/// 答えの出た待ちは、最後の抹消が失敗しても答えのまま終わる（残った記録は、握りが解けるので
/// 次の状態を変える呼び出しの回収が消す）。
#[test]
fn a_failed_unregister_does_not_change_the_answer() {
    let port = b_queued_behind_a().failing(|call, _| (call == 'U').then_some(WatchError::LockBusy));
    port.at(1, Step::Call("A", merged("A")));

    let end = port.run(&merge("B", "areka")).expect("答えのまま");

    assert!(matches!(end, WaitEnd::Done(_)));
    assert_eq!(port.trace(), "RfrsfrU");
    assert_eq!(port.waits_of("B"), [WaitKind::Watch, WaitKind::Merge]);
    assert!(!port.lock_is_held());
}

// ---- 登録と抹消 ----

/// 見張りは見張りの開始（参加を兼ねる）で登録する。停止要請が出たら終わり、見張りの記録を消す。
#[test]
fn a_watch_registers_by_starting_the_watch() {
    let port = FakePort::new();
    port.at(1, Step::Call("C", watching("C")));
    port.at(2, Step::Call("C", asks_load("C")));
    let spec = WaitSpec::Watch {
        id: "A".to_owned(),
        repo: "pasta".to_owned(),
        name: Some("エーの見張り".to_owned()),
    };

    let end = port.run(&spec).expect("終わる");

    assert_eq!(end, done("stop requested by C"));
    let start = Command::Watch {
        id: "A".to_owned(),
        name: Some("エーの見張り".to_owned()),
        repo: "pasta".to_owned(),
        pid: OWN_PID,
    };
    assert_eq!(
        *port.changes.borrow(),
        [(start, "A".to_owned()), unregister("A", WaitKind::Watch)]
    );
    assert_eq!(port.trace(), "WfrsfrsfrU");
    assert_eq!(port.waits_of("A"), []);
    // 参加者の記録は残る（見張りが終わっただけ）。
    let state = port.state.borrow();
    let participant = &state.as_ref().expect("状態が在る").participants["A"];
    assert_eq!(participant.name, "エーの見張り");
    assert_eq!(participant.repo, "pasta");
}

/// 立て直した見張りは、すでに停止要請が出ていれば眠らずに終わる（要件 5.4）。
#[test]
fn a_restarted_watch_ends_without_sleeping_when_a_stop_is_already_requested() {
    let port = c_asked_for_a_load_test();

    let end = port.run(&watch("A")).expect("終わる");

    assert_eq!(end, done("stop requested by C"));
    assert_eq!(port.trace(), "WfrU");
}

/// マージの待ちは、種類とリポジトリ・プロセス番号・始めた時刻を載せた待ちの記録で登録する。
#[test]
fn a_merge_wait_registers_a_wait_record_with_its_repo() {
    let port = b_queued_behind_a();
    port.at(1, Step::Call("A", merged("A")));

    port.run(&merge("B", "areka")).expect("終わる");

    assert_eq!(
        *port.changes.borrow(),
        [
            register("B", WaitKind::Merge, Some("areka")),
            unregister("B", WaitKind::Merge)
        ]
    );
}

#[test]
fn a_load_wait_registers_a_wait_record_and_ends_when_everyone_has_stopped() {
    let port = c_asked_for_a_load_test();
    let stopped = Command::Stopped { id: "A".to_owned() };
    port.at(2, Step::Call("A", stopped));

    let end = port.run(&load("C")).expect("終わる");

    assert_eq!(end, done("granted load; stopped: A"));
    assert_eq!(
        *port.changes.borrow(),
        [
            register("C", WaitKind::Load, None),
            unregister("C", WaitKind::Load)
        ]
    );
    assert_eq!(port.waits_of("C"), [WaitKind::Watch]);
}

#[test]
fn a_resume_wait_registers_a_wait_record_and_ends_when_back_to_work() {
    let port = c_asked_for_a_load_test();
    port.does("A", Command::Stopped { id: "A".to_owned() });
    let load_done = Command::LoadDone { id: "C".to_owned() };
    port.at(2, Step::Call("C", load_done));

    let end = port.run(&resume("A")).expect("終わる");

    assert_eq!(end, done("resumed"));
    assert_eq!(
        *port.changes.borrow(),
        [
            register("A", WaitKind::Resume, None),
            unregister("A", WaitKind::Resume)
        ]
    );
}

// ---- 居る印・出力 ----

/// 居る印は、登録から最後の抹消まで握ったままで、返るときに解く。
#[test]
fn the_presence_sign_is_held_through_the_last_change_and_released_on_return() {
    let port = b_queued_behind_a();
    port.at(30, Step::Call("A", merged("A")));

    port.run(&merge("B", "areka")).expect("終わる");

    // 登録・周期の一回り・抹消。
    assert_eq!(*port.held_at_change.borrow(), [true, true, true]);
    assert!(!port.lock_is_held());
}

/// ループは何も出さない（終わり方を返すだけ。端末へ出すのは cli の仕事。要件 13.1・13.6）。
#[test]
fn the_loop_writes_nothing_to_the_terminal() {
    let source = include_str!("wait.rs");
    for word in ["print!", "println!", "stdout", "stderr", "dbg!"] {
        assert!(!source.contains(word), "wait.rs writes with {word}");
    }
}

/// 本物の口の読み直しの間隔は 1 秒（番の到来に気付く遅れの上限。本物の眠りそのものは、テストの
/// 組み立てでは眠らずに panic する）。
#[test]
fn the_real_port_polls_every_second() {
    assert_eq!(super::POLL, Duration::from_secs(1));
}
