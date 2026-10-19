//! ログの口・読むだけの口・全部消す、のテスト。`target\` の下の本物のファイルで確かめる。
//!
//! ログは、ログの口へ渡された出来事の列で確かめる（ログのファイルの中身は見ない。行き先は
//! プロセスで最初に開いた置き場所になるので、中身の確かめは別プロセスの実機テストが行う）。
//! 組み立ての手は `store_test_support.rs` のものを使う。

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use temp_path_kit::TempPath;

use super::Store;
use super::test_support::{
    BROKEN_AT_T0, Nobody, T0, block_state_writes, home_in, names, participants, read_state,
    refuse_poll, store_in, store_noting_events, store_noting_pauses, tick, watch,
};
use crate::error::{Damage, WatchError};
use crate::home::Home;
use crate::plan::{Applied, Command, Event, Presence, Verdict, apply};
use crate::state::State;
use crate::wait::WaitPort;

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

/// 「作り直した」の出来事。`backup` は退避先、`damage` は退避した訳（無かったので作ったときは
/// どちらも `None`）。
fn recovered_event(backup: Option<&Path>, damage: Option<Damage>) -> Event {
    Event::Recovered {
        backup: backup.map(lossy),
        damage,
    }
}

/// `"not json"` を退避したときの訳。
const NOT_JSON: Damage = Damage::NotJson { line: 1, column: 2 };

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
    let mut expected = vec![recovered_event(None, None)];
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
        [recovered_event(
            Some(&home.dir.join(BROKEN_AT_T0)),
            Some(NOT_JSON)
        )]
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
        [recovered_event(
            Some(&home.dir.join(BROKEN_AT_T0)),
            Some(NOT_JSON)
        )]
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

/// 壊れた状態ファイルの 3 種と、読むだけの道が返す訳。
fn broken_files() -> [(&'static str, Damage); 3] {
    [
        // JSON でない文。
        ("not json", Damage::NotJson { line: 1, column: 2 }),
        // JSON で版も 1 だが、2 行目の欄の型が違う。位置は元のバイト列を読み直して取る。
        (
            "{ \"version\": 2,
  \"participants\": [] }",
            Damage::WrongShape {
                line: 2,
                column: 18,
            },
        ),
        // 版の数字が無い。
        (r#"{ "x": 1 }"#, Damage::NoVersion),
    ]
}

#[test]
fn read_only_fails_on_a_broken_file_with_its_damage_and_does_not_set_it_aside() {
    for (text, damage) in broken_files() {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), text).expect("置ける");
        let (store, noted) = store_noting_events(&root);

        let got = store.read_only();

        match got {
            Err(WatchError::Broken(found)) => assert_eq!(found, damage, "{text}"),
            other => panic!("{text}: {other:?}"),
        }
        let err = store.read_only().expect_err("2 度目も壊れている");
        assert_eq!(err.to_string(), "state file is broken; see impl-watch.log");
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
fn a_change_sets_each_broken_file_aside_and_its_damage_reaches_the_log_port() {
    for (text, damage) in broken_files() {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), text).expect("置ける");
        let (store, noted) = store_noting_events(&root);

        store.with_state(tick).expect("退避して続けられる");

        let backup = home.dir.join(BROKEN_AT_T0);
        assert_eq!(
            fs::read_to_string(&backup).expect("退避ファイルが在る"),
            text
        );
        assert_eq!(
            *noted.borrow(),
            [recovered_event(Some(&backup), Some(damage))],
            "{text}"
        );
        // 形が合わないときの位置は、元のバイト列を読み直して得た 1 以上の行と列。
        if let Damage::WrongShape { line, column } = damage {
            assert!(line >= 1 && column >= 1, "{damage:?}");
        }
    }
}

#[test]
fn read_only_fails_on_an_unknown_version_and_leaves_the_file() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let text = r#"{ "version": 1, "participants": "new shape" }"#;
    fs::write(home.state_path(), text).expect("置ける");

    let got = store_in(&root).read_only();

    assert!(
        matches!(got, Err(WatchError::VersionMismatch { found: 1, known: 2 })),
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
        r#"{ "version": 1, "participants": "new shape" }"#,
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

// ───────── 待ちの口の時計 ─────────

/// 時計を [`T0`] に止めた口を本物の待ちの口として渡すと、置かれた待ちの記録の始めた時刻は
/// [`T0`] になる（待ちの記録の時刻も状態ファイルの口と同じ時計から取る）。停止要請中の
/// 参加者の再開の待ちは最初の判定で終わるので、眠らない。
#[test]
fn the_wait_record_takes_its_start_time_from_the_clock_of_the_store() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::presence::hold;
    use crate::state::{Participant, ParticipantStatus};
    use crate::wait::{WaitEnd, WaitSpec, run};

    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let noted = Rc::clone(&seen);
    let state_path = home.state_path();
    let store = Store {
        log: Box::new(move |event| {
            if let Event::WaitRegistered { id, kind } = event {
                let bytes = fs::read(&state_path).expect("状態ファイルが在る");
                let state: State = serde_json::from_slice(&bytes).expect("状態として読める");
                let record = state.waits.iter().find(|w| w.id == *id && w.kind == *kind);
                noted.borrow_mut().push(record.map(|w| w.since));
            }
        }),
        ..store_in(&root)
    };
    store
        .with_state(|state, now, _| {
            let participant = Participant {
                id: "a".to_owned(),
                repo: "areka".to_owned(),
                status: ParticipantStatus::StopRequested,
                since: now,
                ..Participant::default()
            };
            state.participants.insert("a".to_owned(), participant);
            let applied = Applied {
                changed: true,
                verdict: Verdict::Applied,
                events: Vec::new(),
            };
            (applied, ())
        })
        .expect("変えられる");
    let held = hold(&root.path().join("own.lock"))
        .expect("握れる")
        .expect("誰も握っていない");

    let end = run(&WaitSpec::Resume { id: "a".to_owned() }, &store, held).expect("待てる");

    assert!(
        matches!(end, WaitEnd::Done(_)),
        "最初の判定で終わる（前提）: {end:?}"
    );
    assert_eq!(*seen.borrow(), [Some(T0)], "始めた時刻は止めた時刻");
}

/// 待ちのループの 1 秒の眠りは状態ファイルの口の待ちを通る（要件 8.1）。
#[test]
fn the_wait_sleep_asks_the_store_for_one_second() {
    let root = TempPath::new("store-wait-sleep");
    let (store, pauses) = store_noting_pauses(&root);

    WaitPort::sleep(&store);

    assert_eq!(*pauses.borrow(), [Duration::from_secs(1)]);
}

/// 安全網は 1 秒の待ちをその場で止め、眠りを見分ける字を出す（要件 8.1）。
#[test]
fn the_safety_net_refuses_one_second() {
    let stopped_by = std::panic::catch_unwind(|| refuse_poll(Duration::from_secs(1)))
        .expect_err("1 秒の待ちを止めなかった");
    assert_eq!(
        stopped_by.downcast_ref::<&str>(),
        Some(&"a test reached the real 1-second sleep")
    );
}

/// 安全網は試しの間の短い待ちには何もしない（要件 8.1）。
#[test]
fn the_safety_net_lets_short_pauses_through() {
    refuse_poll(Duration::from_millis(20));
}

/// 最後の抹消（[`WaitPort::finish`]）は、居る印を `state.lock` の排他の中で解く: ログの口の中
/// （排他の中）で、印の探りが「居ない」かつ `state.lock` が握られているのが 1 件だけ見える
/// （要件 6.1・6.4）。
#[test]
fn the_last_unregister_releases_the_presence_mark_inside_the_exclusion() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::presence::{LockFilePresence, hold};
    use crate::state::WaitKind;

    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    store_in(&root)
        .with_state(watch("a"))
        .expect("見張りを始められる");
    let held = hold(&home.alive_path("a", WaitKind::Watch))
        .expect("握れる")
        .expect("誰も握っていない");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let noted = Rc::clone(&seen);
    let probe = LockFilePresence::new(&home);
    let lock_path = home.lock_path();
    let store = Store {
        presence: Box::new(LockFilePresence::new(&home)),
        log: Box::new(move |_| {
            let present = probe.is_present("a", WaitKind::Watch);
            let file = File::open(&lock_path).expect("state.lock が在る");
            let locked = match file.try_lock_shared() {
                Ok(()) => {
                    let _ = file.unlock();
                    false
                }
                Err(std::fs::TryLockError::WouldBlock) => true,
                Err(std::fs::TryLockError::Error(err)) => panic!("探れない: {err}"),
            };
            noted.borrow_mut().push((present, locked));
        }),
        ..store_in(&root)
    };
    let unregister = Command::UnregisterWait {
        id: "a".to_owned(),
        kind: WaitKind::Watch,
    };

    WaitPort::finish(&store, &unregister, "a", held).expect("消せる");

    // (印が居るか, state.lock が握られているか)。
    assert_eq!(*seen.borrow(), [(false, true)]);
    assert!(read_state(&home).waits.is_empty(), "記録は消えた");
}
