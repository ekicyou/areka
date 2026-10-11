//! `store` のテストの支え: 時刻の定数・偽の生死・口の組み立て・置き場所の中身の読み取り。
//!
//! 時計と、試しの間の待ちと、ログの口は [`Store`] の欄へ直に差し込む。

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use temp_path_kit::TempPath;

use super::Store;
use crate::home::Home;
use crate::plan::{Applied, Command, Event, Presence, Verdict, apply};
use crate::state::{State, WaitKind};

/// テストの「いま」（UNIX 秒）。UTC では 2026-10-03T04:00:00Z。
pub(super) const T0: u64 = 1_791_000_000;

/// [`T0`] に壊れた状態ファイルを退避したときの名前。
pub(super) const BROKEN_AT_T0: &str = "state.json.broken-20261003T040000Z";

/// 誰のどの待ちも「居る」と答える偽の生死（参加者が回収で消えない）。
pub(super) struct Everyone;

impl Presence for Everyone {
    fn is_present(&self, _id: &str, _kind: WaitKind) -> bool {
        true
    }
}

/// 誰も「居ない」と答える偽の生死（見張りの無い「作業中」の参加者が回収される）。
pub(super) struct Nobody;

impl Presence for Nobody {
    fn is_present(&self, _id: &str, _kind: WaitKind) -> bool {
        false
    }
}

pub(super) fn home_in(root: &TempPath) -> Home {
    Home {
        dir: root.path().to_path_buf(),
    }
}

/// 時計が [`T0`] で止まり、全員が「居る」、待ちは眠らず、ログは捨てる口。
pub(super) fn store_in(root: &TempPath) -> Store {
    Store {
        home: home_in(root),
        clock: Box::new(|| T0),
        presence: Box::new(Everyone),
        pause: Box::new(|_| {}),
        log: Box::new(|_| {}),
    }
}

/// 頼まれた待ちを眠らずに書き留める口と、その書き留め。
pub(super) fn store_noting_pauses(root: &TempPath) -> (Store, Rc<RefCell<Vec<Duration>>>) {
    let pauses = Rc::new(RefCell::new(Vec::new()));
    let noted = Rc::clone(&pauses);
    let store = Store {
        pause: Box::new(move |pause| noted.borrow_mut().push(pause)),
        ..store_in(root)
    };
    (store, pauses)
}

/// ログの口へ渡された出来事を順に書き留める口と、その書き留め。
pub(super) fn store_noting_events(root: &TempPath) -> (Store, Rc<RefCell<Vec<Event>>>) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let noted = Rc::clone(&events);
    let store = Store {
        log: Box::new(move |event| noted.borrow_mut().push(event.clone())),
        ..store_in(root)
    };
    (store, events)
}

/// 見張りの開始（＝参加）を本物の判断に通す。当てはまったかを返す。
pub(super) fn watch(
    id: &str,
) -> impl FnOnce(&mut State, u64, &dyn Presence) -> (Applied, Verdict) + '_ {
    move |state, now, alive| {
        let command = Command::Watch {
            id: id.to_owned(),
            name: None,
            repo: "areka".to_owned(),
            pid: 1234,
        };
        let applied = apply(state, &command, Some(id), now, alive);
        let verdict = applied.verdict;
        (applied, verdict)
    }
}

/// 周期の一回り（回収する相手が居なければ何も変えない）を本物の判断に通す。
pub(super) fn tick(state: &mut State, now: u64, alive: &dyn Presence) -> (Applied, ()) {
    (apply(state, &Command::Tick, None, now, alive), ())
}

/// フォルダの直下に在るものの名前（並べ替え済み）。
pub(super) fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| {
            entry
                .expect("項目を読める")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    found.sort();
    found
}

pub(super) fn read_state(home: &Home) -> State {
    let bytes = fs::read(home.state_path()).expect("状態ファイルが在る");
    serde_json::from_slice(&bytes).expect("状態ファイルが状態として読める")
}

pub(super) fn participants(home: &Home) -> Vec<String> {
    read_state(home).participants.into_keys().collect()
}

/// 一時ファイルの名前（プロセス番号入り）をフォルダで塞ぐ: 状態ファイルが書けなくなる。
pub(super) fn block_state_writes(home: &Home) {
    let temp = home
        .dir
        .join(format!("state.json.{}.tmp", std::process::id()));
    fs::create_dir(temp).expect("作れる");
}
