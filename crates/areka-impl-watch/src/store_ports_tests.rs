//! ログの口・読むだけの口・全部消す、のテスト。`target\` の下の本物のファイルで確かめる。
//!
//! ログは、ログの口へ渡された出来事の列で確かめる（ログのファイルの中身は見ない。行き先は
//! プロセスで最初に開いた置き場所になるので、中身の確かめは別プロセスの実機テストが行う）。
//! 組み立ての手は `store_test_support.rs` のものを使う。

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::Store;
use super::test_support::{
    BROKEN_AT_T0, Nobody, T0, block_state_writes, home_in, names, participants, read_state,
    store_in, store_noting_events, store_noting_pauses, tick, watch,
};
use crate::error::WatchError;
use crate::home::Home;
use crate::plan::{Applied, Command, Event, Presence, Verdict, apply};
use crate::state::State;

/// [`T0`] に `clear` が状態ファイルを退避したときの名前。
const CLEARED_AT_T0: &str = "state.json.cleared-20261003T040000Z";

/// マージの申し込み（初めての識別なら 参加 → 申し込み → 番 の 3 件が出る）を本物の判断に通す。
/// 判断の返した出来事の列を返す。
fn merge(id: &str) -> impl FnOnce(&mut State, u64, &dyn Presence) -> (Applied, Vec<Event>) + '_ {
    move |state, now, alive| {
        let command = Command::Merge {
            id: id.to_owned(),
            name: None,
            repo: "areka".to_owned(),
            spec: "spec-x".to_owned(),
            bug: false,
        };
        let applied = apply(state, &command, Some(id), now, alive);
        let events = applied.events.clone();
        (applied, events)
    }
}

/// 「作り直した」の出来事。`backup` は退避先（無かったので作ったときは `None`）。
fn recovered_event(backup: Option<&Path>) -> Event {
    Event::Recovered {
        backup: backup.map(lossy),
    }
}

/// 「全部消した」の出来事。`backup` は退避先（状態ファイルが無かったときは `None`）。
fn cleared_event(backup: Option<&Path>) -> Event {
    Event::Cleared {
        backup: backup.map(lossy),
    }
}

fn lossy(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// 一時ファイルの名前（プロセス番号入り）をフォルダで塞ぐ: 読み物だけが書けなくなる。
fn block_status_writes(home: &Home) {
    let temp = home
        .dir
        .join(format!("status.md.{}.tmp", std::process::id()));
    fs::create_dir(temp).expect("作れる");
}

/// `state.lock` を別のハンドルで握る（落とすと解ける）。
fn hold_state_lock(home: &Home) -> File {
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(home.lock_path())
        .expect("ロックファイルを開ける");
    file.lock().expect("握れる");
    file
}

// ───────── ログの口 ─────────

#[test]
fn the_events_of_the_judgement_reach_the_log_port_one_by_one_in_order() {
    let root = TempPath::under_target("impl-watch-store");
    let (store, noted) = store_noting_events(&root);
    store.with_state(tick).expect("変えられる");
    noted.borrow_mut().clear();

    let returned = store.with_state(merge("a")).expect("変えられる");

    let (repo, id) = ("areka".to_owned(), "a".to_owned());
    assert_eq!(
        returned,
        [
            Event::Joined { id: id.clone() },
            Event::MergeRequested {
                repo: repo.clone(),
                id: id.clone(),
                spec: "spec-x".to_owned(),
                bug: false,
            },
            Event::MergeGranted { repo, id },
        ],
        "判断が返した列（前提）"
    );
    assert_eq!(*noted.borrow(), returned, "同じ列が同じ順で 1 件ずつ渡る");

    // 何も変えなかった呼び出しは、何も渡さない。
    noted.borrow_mut().clear();
    store.with_state(tick).expect("変えずに済む");
    assert_eq!(*noted.borrow(), []);
}

#[test]
fn a_recovery_reaches_the_log_port_once_and_before_the_events_of_the_judgement() {
    // 無かった → 作った。
    let root = TempPath::under_target("impl-watch-store");
    let (store, noted) = store_noting_events(&root);
    let returned = store.with_state(merge("a")).expect("変えられる");
    assert_eq!(returned.len(), 3, "判断が返した列（前提）");
    let mut expected = vec![recovered_event(None)];
    expected.extend(returned);
    assert_eq!(*noted.borrow(), expected);
    // 2 度目からは作り直しではない。
    noted.borrow_mut().clear();
    store.with_state(tick).expect("変えずに済む");
    assert_eq!(*noted.borrow(), []);

    // 壊れていた → 退避した。判断が何も変えなくても 1 度だけ渡る。
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    fs::write(home.state_path(), "not json").expect("置ける");
    let (store, noted) = store_noting_events(&root);
    store.with_state(tick).expect("退避して続けられる");
    store.with_state(tick).expect("変えずに済む");
    assert_eq!(
        *noted.borrow(),
        [recovered_event(Some(&home.dir.join(BROKEN_AT_T0)))]
    );
}

#[test]
fn when_the_write_fails_the_events_of_the_judgement_do_not_reach_the_log_port() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    fs::write(home.state_path(), "not json").expect("置ける");
    block_state_writes(&home);
    let (store, noted) = store_noting_events(&root);

    let got = store.with_state(merge("a"));

    assert!(
        matches!(
            got,
            Err(WatchError::Io {
                op: "write state.json",
                ..
            })
        ),
        "{got:?}"
    );
    // 退避はもう起きているので渡る。書けなかった参加・申し込み・番は起きていないので渡らない。
    assert_eq!(
        *noted.borrow(),
        [recovered_event(Some(&home.dir.join(BROKEN_AT_T0)))]
    );
}

#[test]
fn when_a_missing_file_cannot_be_created_nothing_reaches_the_log_port() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    block_state_writes(&home);
    let (store, noted) = store_noting_events(&root);

    let got = store.with_state(merge("a"));

    assert!(
        matches!(
            got,
            Err(WatchError::Io {
                op: "write state.json",
                ..
            })
        ),
        "{got:?}"
    );
    // 作れなかったので、「作った」も、参加・申し込み・番も、起きていない。
    assert_eq!(*noted.borrow(), []);
}

#[test]
fn when_only_the_reading_matter_cannot_be_written_the_events_still_reach_the_log_port() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let (store, noted) = store_noting_events(&root);
    store.with_state(tick).expect("変えられる");
    noted.borrow_mut().clear();
    let reading = fs::read(home.status_path()).expect("読める");
    block_status_writes(&home);

    // 読み物が書けないことは失敗にしない: 判断の返した値がそのまま返る。
    let returned = store.with_state(merge("a")).expect("変えられる");

    assert_eq!(returned.len(), 3, "判断が返した列（前提）");
    // 較正: 読み物は本当に書けていない（塞ぐ前のまま）。
    assert_eq!(fs::read(home.status_path()).expect("読める"), reading);
    // 状態ファイルはもう置き換わっている（変化は起きた）。
    assert_eq!(participants(&home), ["a"]);
    let holder = read_state(&home).merge["areka"].holder.clone();
    assert_eq!(holder.map(|holder| holder.id), Some("a".to_owned()));
    // 起きた変化は、読み物が書けなくてもログの口へ渡っている。
    let (repo, id) = ("areka".to_owned(), "a".to_owned());
    assert_eq!(
        *noted.borrow(),
        [
            Event::Joined { id: id.clone() },
            Event::MergeRequested {
                repo: repo.clone(),
                id: id.clone(),
                spec: "spec-x".to_owned(),
                bug: false,
            },
            Event::MergeGranted { repo, id },
        ]
    );
    assert_eq!(*noted.borrow(), returned);
}

#[test]
fn the_reclaim_of_a_refused_command_reaches_the_log_port() {
    let root = TempPath::under_target("impl-watch-store");
    store_in(&root).with_state(watch("a")).expect("変えられる");
    let (store, noted) = store_noting_events(&root);
    let store = Store {
        presence: Box::new(Nobody),
        ..store
    };

    let verdict = store
        .with_state(|state, now, alive| {
            let command = Command::Stopped { id: "b".to_owned() };
            let applied = apply(state, &command, Some("b"), now, alive);
            let verdict = applied.verdict;
            (applied, verdict)
        })
        .expect("変えられる");

    assert!(matches!(verdict, Verdict::NotApplied(_)), "{verdict:?}");
    assert_eq!(
        *noted.borrow(),
        [Event::Reclaimed {
            id: "a".to_owned(),
            why: "watch absent",
        }]
    );
}

// ───────── 読むだけの口 ─────────

#[test]
fn read_only_of_a_missing_file_is_none_and_creates_nothing() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let (store, noted) = store_noting_events(&root);

    let got = store.read_only().expect("読める");

    assert_eq!(got, None);
    assert_eq!(names(&home.dir), [""; 0], "ロックも状態も読み物も作らない");
    assert_eq!(*noted.borrow(), [], "作り直しではない");
}

#[test]
fn read_only_returns_the_state_without_taking_the_lock_or_writing() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    store_in(&root).with_state(watch("a")).expect("変えられる");
    fs::remove_file(home.status_path()).expect("消せる");
    let before = fs::read(home.state_path()).expect("読める");
    // 誰かが排他を握ったままでも、待たずに読める。
    let _held = hold_state_lock(&home);
    let (store, pauses) = store_noting_pauses(&root);

    let got = store.read_only().expect("読める");

    assert_eq!(got, Some(read_state(&home)));
    let ids: Vec<String> = got.expect("在る").participants.into_keys().collect();
    assert_eq!(ids, ["a"]);
    assert_eq!(*pauses.borrow(), [], "ロックを待っていない");
    assert_eq!(fs::read(home.state_path()).expect("読める"), before);
    assert_eq!(
        names(&home.dir),
        ["state.json", "state.lock"],
        "読み物も書かない"
    );
}

#[test]
fn read_only_fails_on_a_broken_file_and_does_not_set_it_aside() {
    // JSON でない文・JSON だが形が合わない・版の数字が無い。
    let broken = ["not json", r#"{ "version": 1, "participants": [] }"#, "{}"];
    for text in broken {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), text).expect("置ける");
        let (store, noted) = store_noting_events(&root);

        let got = store.read_only();

        assert!(matches!(got, Err(WatchError::Broken)), "{text}: {got:?}");
        assert_eq!(
            fs::read_to_string(home.state_path()).expect("読める"),
            text,
            "ファイルは元のまま"
        );
        assert_eq!(names(&home.dir), ["state.json"], "退避も作成もしない");
        assert_eq!(*noted.borrow(), []);
    }
}

#[test]
fn read_only_fails_on_an_unknown_version_and_leaves_the_file() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let text = r#"{ "version": 2, "participants": "new shape" }"#;
    fs::write(home.state_path(), text).expect("置ける");

    let got = store_in(&root).read_only();

    assert!(
        matches!(got, Err(WatchError::VersionMismatch { found: 2, known: 1 })),
        "{got:?}"
    );
    assert_eq!(fs::read_to_string(home.state_path()).expect("読める"), text);
    assert_eq!(names(&home.dir), ["state.json"]);
}

// ───────── 状態の確認 ─────────

#[test]
fn summary_returns_the_terminal_summary_even_when_the_reading_matter_cannot_be_written() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);
    assert_eq!(store.summary().expect("読める"), None, "状態ファイルが無い");
    store.with_state(watch("a")).expect("変えられる");
    fs::remove_file(home.status_path()).expect("消せる");

    let summary = store
        .summary()
        .expect("読める")
        .expect("状態ファイルが在る");

    assert!(summary.contains("a areka working\n"), "{summary}");
    assert!(
        home.status_path().is_file(),
        "書けるときは読み物を置き換える"
    );

    fs::remove_file(home.status_path()).expect("消せる");
    block_status_writes(&home);

    let got = store.summary().expect("読み物が書けなくても、要約は返る");

    assert_eq!(got, Some(summary));
    // 較正: 読み物は本当に書けていない。
    assert!(!home.status_path().exists());
}

// ───────── 全部消す ─────────

#[test]
fn clear_sets_the_state_file_aside_and_leaves_only_the_cleared_record() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let (store, noted) = store_noting_events(&root);
    store.with_state(merge("a")).expect("変えられる");
    store.with_state(watch("b")).expect("変えられる");
    let before = fs::read(home.state_path()).expect("読める");
    noted.borrow_mut().clear();

    let backup = store.clear().expect("消せる");

    let expected: PathBuf = home.dir.join(CLEARED_AT_T0);
    assert_eq!(backup.as_deref(), Some(expected.as_path()));
    assert_eq!(
        fs::read(&expected).expect("退避ファイルが在る"),
        before,
        "消す前の状態ファイルが中身のまま残る"
    );
    // 状態は版と「消した記録」だけ（参加者・机・直前のマージ・待ちの記録は無い）。
    let state = read_state(&home);
    assert_eq!(state, State::cleared(T0, &expected));
    let mut only_the_record = State::empty();
    only_the_record.recent = state.recent.clone();
    assert_eq!(state, only_the_record);
    assert_eq!(state.recent.len(), 1);
    assert_eq!(
        names(&home.dir),
        ["state.json", CLEARED_AT_T0, "state.lock", "status.md"]
    );
    let status = fs::read_to_string(home.status_path()).expect("読める");
    assert!(
        status.contains(&format!("| cleared | - | {} |", expected.display())),
        "{status}"
    );
    assert!(!status.contains("| a |"), "{status}");
    assert_eq!(*noted.borrow(), [cleared_event(Some(&expected))]);
}

#[test]
fn clear_without_a_state_file_still_writes_the_empty_state_and_returns_none() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let (store, noted) = store_noting_events(&root);

    let backup = store.clear().expect("消せる");

    assert_eq!(backup, None);
    assert_eq!(read_state(&home), State::cleared(T0, Path::new("none")));
    assert_eq!(names(&home.dir), ["state.json", "state.lock", "status.md"]);
    assert_eq!(*noted.borrow(), [cleared_event(None)], "作り直しではない");
}

#[test]
fn clear_does_not_read_the_old_file_so_it_works_over_an_unknown_version_and_a_broken_file() {
    let unreadable = [
        r#"{ "version": 2, "participants": "new shape" }"#,
        "not json",
    ];
    for text in unreadable {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), text).expect("置ける");
        let (store, noted) = store_noting_events(&root);

        let backup = store.clear().expect("消せる").expect("退避した");

        assert_eq!(backup, home.dir.join(CLEARED_AT_T0));
        assert_eq!(fs::read_to_string(&backup).expect("在る"), text, "{text}");
        assert_eq!(read_state(&home), State::cleared(T0, &backup));
        // 「壊れていた」の退避ではない。
        assert_eq!(
            names(&home.dir),
            ["state.json", CLEARED_AT_T0, "state.lock", "status.md"]
        );
        assert_eq!(*noted.borrow(), [cleared_event(Some(&backup))]);
    }
}

#[test]
fn a_second_clear_in_the_same_second_does_not_overwrite_the_first_backup() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);
    store.with_state(watch("a")).expect("変えられる");
    let original = fs::read(home.state_path()).expect("読める");

    let first = store.clear().expect("消せる").expect("退避した");
    let cleared = fs::read(home.state_path()).expect("読める");
    let second = store.clear().expect("消せる").expect("退避した");

    assert_eq!(first, home.dir.join(CLEARED_AT_T0));
    assert_eq!(second, home.dir.join(format!("{CLEARED_AT_T0}-1")));
    assert_eq!(fs::read(&first).expect("在る"), original);
    assert_eq!(fs::read(&second).expect("在る"), cleared);
    assert_eq!(read_state(&home), State::cleared(T0, &second));
}

#[test]
fn clear_runs_inside_the_exclusion() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    store_in(&root).with_state(watch("a")).expect("変えられる");
    let before = fs::read(home.state_path()).expect("読める");
    let held = hold_state_lock(&home);
    let (store, noted) = store_noting_events(&root);

    let got = store.clear();

    assert!(matches!(got, Err(WatchError::LockBusy)), "{got:?}");
    assert_eq!(fs::read(home.state_path()).expect("読める"), before);
    assert_eq!(names(&home.dir), ["state.json", "state.lock", "status.md"]);
    assert_eq!(*noted.borrow(), []);

    drop(held);
    store.clear().expect("放された後は消せる");
    assert_eq!(participants(&home), [""; 0]);
}

#[test]
fn when_only_the_reading_matter_cannot_be_written_clear_still_returns_the_backup() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let (store, noted) = store_noting_events(&root);
    store.with_state(watch("a")).expect("変えられる");
    noted.borrow_mut().clear();
    let reading = fs::read(home.status_path()).expect("読める");
    block_status_writes(&home);

    let backup = store
        .clear()
        .expect("読み物が書けなくても、消した答えは返る");

    let expected = home.dir.join(CLEARED_AT_T0);
    assert_eq!(backup.as_deref(), Some(expected.as_path()));
    assert_eq!(read_state(&home), State::cleared(T0, &expected));
    assert_eq!(*noted.borrow(), [cleared_event(Some(&expected))]);
    // 較正: 読み物は本当に書けていない（塞ぐ前のまま）。
    assert_eq!(fs::read(home.status_path()).expect("読める"), reading);
}

#[test]
fn when_the_empty_state_cannot_be_written_the_backup_of_clear_remains() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    store_in(&root).with_state(watch("a")).expect("変えられる");
    let before = fs::read(home.state_path()).expect("読める");
    block_state_writes(&home);
    let (store, noted) = store_noting_events(&root);

    let got = store.clear();

    assert!(
        matches!(
            got,
            Err(WatchError::Io {
                op: "write state.json",
                ..
            })
        ),
        "{got:?}"
    );
    // 消す前の中身は退避先に残り、退避したことはログの口へ渡っている。
    let backup = home.dir.join(CLEARED_AT_T0);
    assert_eq!(fs::read(&backup).expect("退避ファイルが在る"), before);
    assert_eq!(*noted.borrow(), [cleared_event(Some(&backup))]);
}
