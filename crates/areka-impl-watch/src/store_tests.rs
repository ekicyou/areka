//! 状態ファイルの排他・読み・置き換え書きのテスト。`target\` の下の本物のファイルで確かめる。
//!
//! 時計と、試しの間の待ちは [`Store`] の欄へ直に差し込む（眠らずに、頼まれた待ちを数える）。
//! 生死は偽の口で答える。本物の眠りを使うのは、スレッドを競わせる 1 本だけ。
//! ログの口・読むだけの口・全部消す、のテストは `store_ports_tests.rs` に在る（組み立ての手は
//! どちらも `store_test_support.rs` のものを使う）。

use std::cell::Cell;
use std::fs::{self, File};
use std::rc::Rc;
use std::sync::{Barrier, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use temp_path_kit::TempPath;

use super::test_support::{
    BROKEN_AT_T0, Everyone, Nobody, T0, block_state_writes, home_in, names, participants,
    read_state, store_in, store_noting_pauses, tick, watch,
};
use super::{Store, open};
use crate::error::WatchError;
use crate::home::Home;
use crate::plan::{Applied, Command, Verdict, apply};
use crate::presence::hold;
use crate::state::{Recent, RecentKind, State, WaitKind};

/// 「何も変えなかった」という判断の答え。
fn unchanged() -> Applied {
    Applied {
        changed: false,
        verdict: Verdict::Applied,
        events: Vec::new(),
    }
}

/// 「作り直した」の記録（時刻は [`T0`]）。
fn recovered(detail: &str) -> Recent {
    Recent {
        at: T0,
        kind: RecentKind::Recovered,
        id: None,
        detail: detail.to_owned(),
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("時計が 1970 年より後")
        .as_secs()
}

// ───────── 読み: 無い・壊れた・版違い ─────────

#[test]
fn missing_file_is_created_and_recorded_once_even_when_nothing_else_changes() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);

    // 判断は何も変えないが、「新しく作った」の記録は書かれる。
    store.with_state(tick).expect("変えられる");

    let mut expected = State::empty();
    expected.push_recent(recovered("created"));
    assert_eq!(read_state(&home), expected);
    assert_eq!(names(&home.dir), ["state.json", "state.lock", "status.md"]);
    let status = fs::read_to_string(home.status_path()).expect("読み物が在る");
    assert!(status.contains("| recovered | - | created |"), "{status}");

    // 2 度目からはファイルが在るので、記録は増えず、何も書かれない。
    let before = fs::read(home.state_path()).expect("読める");
    store.with_state(tick).expect("変えられる");
    assert_eq!(fs::read(home.state_path()).expect("読める"), before);
}

#[test]
fn unreadable_file_is_set_aside_under_a_time_stamped_name_and_recorded() {
    // JSON でない文・途中で切れた JSON・UTF-8 でないバイト列。
    let broken: [&[u8]; 3] = [
        b"not json",
        br#"{ "version": 1, "participants": {"#,
        b"\xff\xfe\x00",
    ];
    for bytes in broken {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), bytes).expect("置ける");

        store_in(&root)
            .with_state(watch("a"))
            .expect("退避して続けられる");

        let backup = home.dir.join(BROKEN_AT_T0);
        assert_eq!(
            fs::read(&backup).expect("退避ファイルが在る"),
            bytes,
            "壊れたファイルは中身のまま残る"
        );
        let state = read_state(&home);
        assert_eq!(
            state.recent,
            [recovered(&format!("backed up: {}", backup.display()))]
        );
        assert_eq!(state.participants.len(), 1, "空から始めて判断が進む");
        assert_eq!(
            names(&home.dir),
            ["state.json", BROKEN_AT_T0, "state.lock", "status.md"]
        );
    }
}

#[test]
fn json_of_the_wrong_shape_is_broken_too() {
    // どれも JSON としては読める。版の数字が無いもの（`{}`・文字の版）も「形が合わない」。
    let wrong = [
        r#"{ "version": 1, "participants": [] }"#,
        r#"{ "version": 1, "waits": [ { "id": "A" } ] }"#,
        r#"[ 1, 2 ]"#,
        r#""text""#,
        r#"{}"#,
        r#"{ "participants": {} }"#,
        r#"{ "version": "1" }"#,
        r#"{ "version": -1 }"#,
    ];
    for text in wrong {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), text).expect("置ける");

        store_in(&root)
            .with_state(tick)
            .unwrap_or_else(|err| panic!("{text}: 退避して続けられる ({err})"));

        let backup = home.dir.join(BROKEN_AT_T0);
        assert_eq!(
            fs::read_to_string(&backup).expect("退避ファイルが在る"),
            text
        );
        let mut expected = State::empty();
        expected.push_recent(recovered(&format!("backed up: {}", backup.display())));
        assert_eq!(read_state(&home), expected, "{text}");
    }
}

#[test]
fn a_second_break_in_the_same_second_does_not_overwrite_the_first_backup() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);

    fs::write(home.state_path(), "first").expect("置ける");
    store.with_state(tick).expect("退避して続けられる");
    fs::write(home.state_path(), "second").expect("置ける");
    store.with_state(tick).expect("退避して続けられる");

    let first = home.dir.join(BROKEN_AT_T0);
    let second = home.dir.join(format!("{BROKEN_AT_T0}-1"));
    assert_eq!(fs::read_to_string(&first).expect("在る"), "first");
    assert_eq!(fs::read_to_string(&second).expect("在る"), "second");
    assert_eq!(
        read_state(&home).recent,
        [recovered(&format!("backed up: {}", second.display()))]
    );
}

#[test]
fn unknown_version_fails_without_reading_or_touching_the_file() {
    // 版の数字が在って、この実行ファイルの知らないもの（0 も含む）。中身の形は見ない。
    let cases = [
        (r#"{ "version": 2, "participants": {} }"#, 2),
        (r#"{ "version": 0 }"#, 0),
        (r#"{ "version": 7, "participants": "new shape" }"#, 7),
    ];
    for (text, version) in cases {
        let root = TempPath::under_target("impl-watch-store");
        let home = home_in(&root);
        fs::write(home.state_path(), text).expect("置ける");
        let called = Cell::new(false);

        let got = store_in(&root).with_state(|_, _, _| {
            called.set(true);
            (unchanged(), ())
        });

        assert!(
            matches!(got, Err(WatchError::VersionMismatch { found, known: 1 }) if found == version),
            "{text}: {got:?}"
        );
        assert!(!called.get(), "判断は呼ばれない");
        assert_eq!(
            fs::read_to_string(home.state_path()).expect("読める"),
            text,
            "ファイルは元のまま"
        );
        // 退避も読み物も一時ファイルも作らない。
        assert_eq!(names(&home.dir), ["state.json", "state.lock"]);
    }
}

#[test]
fn a_state_file_that_cannot_be_opened_fails_and_is_not_set_aside() {
    // 状態ファイルの名前のフォルダ: 読みが OS の失敗になる（「壊れている」とは見なさない）。
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    fs::create_dir(home.state_path()).expect("作れる");

    let got = store_in(&root).with_state(tick);

    assert!(
        matches!(
            got,
            Err(WatchError::Io {
                op: "read state.json",
                ..
            })
        ),
        "{got:?}"
    );
    assert_eq!(names(&home.dir), ["state.json", "state.lock"]);
    assert!(home.state_path().is_dir());
}

// ───────── 書き: 変わったときだけ・置き換え ─────────

#[test]
fn a_change_is_written_and_no_temp_file_remains() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);

    let verdict = store.with_state(watch("a")).expect("変えられる");
    store.with_state(watch("b")).expect("変えられる");

    assert_eq!(verdict, Verdict::Applied, "判断の答えは呼び手へ返る");
    assert_eq!(participants(&home), ["a", "b"]);
    assert_eq!(names(&home.dir), ["state.json", "state.lock", "status.md"]);
}

#[test]
fn status_md_is_written_after_a_change_with_the_participant_and_the_time() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);

    store_in(&root)
        .with_state(|state, now, alive| {
            let command = Command::Watch {
                id: "a".to_owned(),
                name: Some("実装 A".to_owned()),
                repo: "areka".to_owned(),
                pid: 1234,
            };
            (apply(state, &command, Some("a"), now, alive), ())
        })
        .expect("変えられる");

    let status = fs::read_to_string(home.status_path()).expect("読み物が UTF-8 で在る");
    assert!(
        status.contains("generated: 2026-10-03T04:00:00Z"),
        "{status}"
    );
    assert!(
        status.contains("| a | 実装 A | areka | working |"),
        "{status}"
    );
}

#[test]
fn nothing_is_written_when_the_judgement_says_unchanged() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let now = Rc::new(Cell::new(T0));
    let clock = Rc::clone(&now);
    let store = Store {
        clock: Box::new(move || clock.get()),
        ..store_in(&root)
    };
    store.with_state(watch("a")).expect("変えられる");
    let state_before = fs::read(home.state_path()).expect("読める");
    let status_before = fs::read(home.status_path()).expect("読める");
    let fingerprint_before = store.fingerprint().expect("見られる");

    // 1 分後。書き直せば読み物の時刻が変わり、状態を書けば（わざと足した）参加者が載る。
    now.set(T0 + 60);
    let seen = store
        .with_state(|state, now, _| {
            state
                .participants
                .insert("ghost".to_owned(), Default::default());
            (unchanged(), now)
        })
        .expect("変えずに済む");

    assert_eq!(seen, T0 + 60, "判断には注入した時計の時刻が渡る");
    assert_eq!(fs::read(home.state_path()).expect("読める"), state_before);
    assert_eq!(fs::read(home.status_path()).expect("読める"), status_before);
    assert_eq!(store.fingerprint().expect("見られる"), fingerprint_before);
}

#[test]
fn writing_is_decided_by_changed_and_not_by_the_verdict() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    store_in(&root).with_state(watch("a")).expect("変えられる");

    // 見張りの無い a が居るときの、b の当てはまらない「止まった」: 断られるが a は回収される。
    let store = Store {
        presence: Box::new(Nobody),
        ..store_in(&root)
    };
    let (changed, verdict) = store
        .with_state(|state, now, alive| {
            let command = Command::Stopped { id: "b".to_owned() };
            let applied = apply(state, &command, Some("b"), now, alive);
            let seen = (applied.changed, applied.verdict);
            (applied, seen)
        })
        .expect("変えられる");

    assert!(changed);
    assert!(matches!(verdict, Verdict::NotApplied(_)), "{verdict:?}");
    let state = read_state(&home);
    assert!(state.participants.is_empty(), "回収の分は書かれている");
    assert_eq!(state.recent[0].kind, RecentKind::Reclaimed);
    let status = fs::read_to_string(home.status_path()).expect("読める");
    assert!(status.contains("| reclaimed | a |"), "{status}");
}

#[test]
fn a_failed_write_of_the_state_leaves_the_original_files_as_they_were() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);
    store.with_state(watch("a")).expect("変えられる");
    let state_before = fs::read(home.state_path()).expect("読める");
    let status_before = fs::read(home.status_path()).expect("読める");
    block_state_writes(&home);

    let got = store.with_state(watch("b"));

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
    assert_eq!(fs::read(home.state_path()).expect("読める"), state_before);
    assert_eq!(fs::read(home.status_path()).expect("読める"), status_before);
}

#[test]
fn a_failed_replace_is_retried_five_times_and_then_removes_the_temp_file() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    // 読み物の名前の、空でないフォルダ: 置き換え（名前の付け替え）が毎回失敗する。
    fs::create_dir(home.status_path()).expect("作れる");
    let kept = home.status_path().join("kept.txt");
    fs::write(&kept, "kept").expect("置ける");
    let (store, pauses) = store_noting_pauses(&root);

    let got = store.with_state(watch("a"));

    assert!(
        matches!(
            got,
            Err(WatchError::Io {
                op: "write status.md",
                ..
            })
        ),
        "{got:?}"
    );
    // 5 回試し、間に 20 ms を 4 回挟む。
    assert_eq!(*pauses.borrow(), [Duration::from_millis(20); 4]);
    assert_eq!(
        names(&home.dir),
        ["state.json", "state.lock", "status.md"],
        "一時ファイルは残らない"
    );
    assert_eq!(fs::read_to_string(&kept).expect("元のまま"), "kept");
}

// ───────── 排他 ─────────

#[test]
fn while_another_thread_holds_the_lock_the_change_fails_at_the_limit() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let lock_path = home.lock_path();
    let (locked_tx, locked_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder = std::thread::spawn(move || {
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(lock_path)
            .expect("ロックファイルを開ける");
        file.lock().expect("握れる");
        locked_tx.send(()).expect("知らせられる");
        // 放せと言われる（または相手が落ちる）まで握っておく。
        let _ = release_rx.recv();
        file.unlock().expect("放せる");
    });
    locked_rx.recv().expect("握ったと知らされる");
    let (store, pauses) = store_noting_pauses(&root);
    let called = Cell::new(false);

    let got = store.with_state(|_, _, _| {
        called.set(true);
        (unchanged(), ())
    });
    drop(release_tx);
    holder.join().expect("握っていたスレッドが終わる");

    assert!(matches!(got, Err(WatchError::LockBusy)), "{got:?}");
    // 10 ms 間隔で、合わせて 10 秒ぶん待とうとした（眠ってはいない）。
    let pauses = pauses.borrow();
    assert!(
        pauses
            .iter()
            .all(|pause| *pause == Duration::from_millis(10))
    );
    assert_eq!(pauses.iter().sum::<Duration>(), Duration::from_secs(10));
    assert!(!called.get(), "判断は呼ばれない");
    assert_eq!(names(&home.dir), ["state.lock"], "何も読み書きしていない");

    // 放された後は通る（排他は呼び出しの間だけ）。
    store.with_state(watch("a")).expect("変えられる");
    store.with_state(watch("b")).expect("続けて変えられる");
    assert_eq!(participants(&home), ["a", "b"]);
}

#[test]
fn concurrent_changes_from_several_threads_are_all_kept() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let ids: Vec<String> = (0..8).map(|n| format!("s{n}")).collect();
    let start = Barrier::new(ids.len());

    std::thread::scope(|scope| {
        for id in &ids {
            let (dir, start) = (&home.dir, &start);
            scope.spawn(move || {
                // 口はスレッドごと（別々のプロセスが別々に開く姿）。ロックの待ちは本物の眠り。
                let store = Store {
                    home: Home { dir: dir.clone() },
                    clock: Box::new(|| T0),
                    presence: Box::new(Everyone),
                    pause: Box::new(std::thread::sleep),
                    log: Box::new(|_| {}),
                };
                start.wait();
                store.with_state(watch(id)).expect("変えられる");
            });
        }
    });

    assert_eq!(participants(&home), ids, "どのスレッドの参加も残る");
    assert_eq!(
        read_state(&home).recent,
        [recovered("created")],
        "作ったのは 1 度だけ"
    );
    assert_eq!(names(&home.dir), ["state.json", "state.lock", "status.md"]);
}

// ───────── 待ちの読み直し用の口・本物の口 ─────────

#[test]
fn fingerprint_is_none_without_a_file_and_changes_with_every_write() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let store = store_in(&root);
    assert_eq!(store.fingerprint().expect("見られる"), None);
    assert_eq!(names(&home.dir), [""; 0], "見るだけで何も作らない");

    store.with_state(watch("a")).expect("変えられる");
    let first = store.fingerprint().expect("見られる").expect("在る");
    let meta = fs::metadata(home.state_path()).expect("在る");
    assert_eq!(first, (meta.modified().expect("時刻が取れる"), meta.len()));

    store.with_state(watch("b")).expect("変えられる");
    let second = store.fingerprint().expect("見られる").expect("在る");
    assert_ne!(second, first);
    assert!(second.1 > first.1, "参加者が増えたぶん大きい");
}

#[test]
fn open_wires_the_real_clock_and_the_lock_file_presence() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    let _held = hold(&home.alive_path("a", WaitKind::Watch))
        .expect("握れる")
        .expect("誰も握っていない");
    let store = open(home_in(&root), "test").expect("開ける");

    let before = unix_now();
    let (now, a, b) = store
        .with_state(|_, now, alive| {
            let a = alive.is_present("a", WaitKind::Watch);
            let b = alive.is_present("b", WaitKind::Watch);
            (unchanged(), (now, a, b))
        })
        .expect("変えられる");
    let after = unix_now();

    assert!(
        (before..=after).contains(&now),
        "{before} <= {now} <= {after}"
    );
    assert!(a, "握られている印は居る");
    assert!(!b, "印の無い識別は居ない");
}

#[test]
fn open_can_be_called_again_in_the_same_process_and_each_home_gets_a_log_file() {
    // ログの受け手が据わるのはプロセスで 1 度だけ。2 度目からの `open` も落ちずに使える。
    let roots = [
        TempPath::under_target("impl-watch-store"),
        TempPath::under_target("impl-watch-store"),
    ];
    for root in &roots {
        let home = home_in(root);
        let store = open(home_in(root), "test").expect("開ける");
        assert_eq!(names(&home.dir), ["impl-watch.log"], "開くだけならログだけ");

        store.with_state(watch("a")).expect("変えられる");

        assert_eq!(participants(&home), ["a"]);
        assert_eq!(
            names(&home.dir),
            ["impl-watch.log", "state.json", "state.lock", "status.md"]
        );
    }
}

#[test]
fn open_fails_when_the_log_file_cannot_be_opened() {
    let root = TempPath::under_target("impl-watch-store");
    let home = home_in(&root);
    fs::create_dir(home.log_path()).expect("作れる");

    let got = open(home_in(&root), "test").map(|_| ());

    assert!(
        matches!(
            got,
            Err(WatchError::Io {
                op: "open impl-watch.log",
                ..
            })
        ),
        "{got:?}"
    );
}
