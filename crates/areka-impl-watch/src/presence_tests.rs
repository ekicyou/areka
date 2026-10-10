//! 居る印（ロックファイルを握る・探る）のテスト。`target\` の下の本物のファイルで確かめる。
//!
//! 握る側と探る側は同じプロセスの別々の `File`（Windows の `LockFileEx` はハンドル単位なので、
//! 同じプロセスでも 2 つ目のハンドルは取れない）。時計には依らない: 試しの間の待ちは
//! [`hold_retrying`] へ渡す関数で差し替える。

use std::fs::File;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{LockFilePresence, hold, hold_retrying};
use crate::home::Home;
use crate::plan::Presence;
use crate::state::WaitKind;

/// 一時フォルダを置き場所にした `Home`（フォルダは在る・`alive/` はまだ無い）。
fn home_in(root: &TempPath) -> Home {
    Home {
        dir: root.path().to_path_buf(),
    }
}

/// フォルダの直下に在るものの一覧（並べ替え済み）。
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}

#[test]
fn held_is_present_and_dropped_is_absent_and_the_file_stays() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let probe = LockFilePresence::new(&home);
    let path = home.alive_path("a", WaitKind::Watch);

    let held = hold(&path).expect("握れる").expect("誰も握っていない");
    assert!(probe.is_present("a", WaitKind::Watch), "握っている間は居る");
    // 探りは握っている側のロックを壊さない（何度探っても同じ答え）。
    assert!(probe.is_present("a", WaitKind::Watch));

    drop(held);
    assert!(!probe.is_present("a", WaitKind::Watch), "落とすと居ない");
    assert!(path.is_file(), "ロックファイルは消さない");
}

#[test]
fn hold_creates_the_alive_folder_when_missing() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    assert!(!home.alive_dir().exists());

    let path = home.alive_path("a", WaitKind::Watch);
    let _held = hold(&path).expect("握れる").expect("誰も握っていない");

    assert!(home.alive_dir().is_dir());
    assert_eq!(entries(&home.alive_dir()), vec![path]);
}

#[test]
fn second_hold_is_not_acquired_after_five_tries() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);
    let _first = hold(&path).expect("握れる").expect("誰も握っていない");

    let mut pauses = 0;
    let second = hold_retrying(&path, || pauses += 1).expect("失敗ではない");

    assert!(second.is_none(), "二重に握れた");
    assert_eq!(pauses, 4, "5 回試す（間の待ちは 4 回）");
    // 取れなかった側が落ちても、握っている側の印は残る。
    assert!(LockFilePresence::new(&home).is_present("a", WaitKind::Watch));
}

#[test]
fn hold_gets_the_lock_when_it_is_released_between_tries() {
    // 探りが一瞬ロックを取っていた、の形（設計「探りと watch の開始が同時」）。
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);
    let mut blocker = Some(hold(&path).expect("握れる").expect("誰も握っていない"));

    let mut pauses = 0;
    let held = hold_retrying(&path, || {
        pauses += 1;
        if pauses == 2 {
            blocker = None;
        }
    })
    .expect("失敗ではない");

    assert!(held.is_some(), "解かれた後の試しで取れる");
    assert_eq!(pauses, 2, "取れたらそれ以上試さない");
    assert!(LockFilePresence::new(&home).is_present("a", WaitKind::Watch));
}

/// 進行中の探りの形: 作らずに開いて、共有のロックを取ったままのハンドル。
fn probe_in_progress(path: &Path) -> File {
    let file = File::open(path).expect("開ける");
    file.try_lock_shared().expect("共有のロックが取れる");
    file
}

#[test]
fn two_probes_at_the_same_time_both_answer_absent() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);
    // 握られていないロックファイル（落ちた見張りが残したもの）。
    drop(hold(&path).expect("握れる").expect("誰も握っていない"));
    let other = probe_in_progress(&path);

    // 先の探りがロックを取っている最中でも、後の探りは「居る」と見誤らない。
    assert!(!LockFilePresence::new(&home).is_present("a", WaitKind::Watch));

    drop(other);
}

#[test]
fn a_held_file_is_present_for_probes_at_the_same_time() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);
    let _held = hold(&path).expect("握れる").expect("誰も握っていない");

    // 握られているファイルには、探りの共有のロックも掛からない。
    let other = File::open(&path).expect("開ける");
    assert!(other.try_lock_shared().is_err(), "排他で握れていない");
    assert!(LockFilePresence::new(&home).is_present("a", WaitKind::Watch));
}

#[test]
fn hold_started_during_a_probe_gets_the_lock_once_the_probe_is_over() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);
    drop(hold(&path).expect("握れる").expect("誰も握っていない"));
    let mut probing = Some(probe_in_progress(&path));

    let mut pauses = 0;
    let held = hold_retrying(&path, || {
        pauses += 1;
        if let Some(probe) = probing.take_if(|_| pauses == 2) {
            // 閉じるだけでも解けるが、解ける時機を OS 任せにしない。
            probe.unlock().expect("解ける");
        }
    })
    .expect("失敗ではない");

    // 探りの共有のロックが在る間（1・2 回目）は握れず、解けた後の試しで握れる。
    assert!(held.is_some(), "解かれた後の試しで取れる");
    assert_eq!(pauses, 2, "探りの間は握れない・取れたらそれ以上試さない");
    assert!(LockFilePresence::new(&home).is_present("a", WaitKind::Watch));
}

#[test]
fn hold_again_after_drop_reuses_the_same_file() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);

    drop(hold(&path).expect("握れる").expect("誰も握っていない"));
    let again = hold(&path).expect("握れる");

    assert!(again.is_some(), "解かれたファイルは握り直せる");
    assert_eq!(entries(&home.alive_dir()), vec![path]);
}

#[test]
fn probe_without_a_file_is_absent_and_creates_nothing() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let probe = LockFilePresence::new(&home);

    // `alive/` も無い。
    assert!(!probe.is_present("a", WaitKind::Watch));
    assert_eq!(
        entries(root.path()),
        Vec::<PathBuf>::new(),
        "探りが何かを作った"
    );

    // `alive/` は在るがファイルが無い。
    std::fs::create_dir(home.alive_dir()).expect("作れる");
    assert!(!probe.is_present("a", WaitKind::Watch));
    assert_eq!(
        entries(&home.alive_dir()),
        Vec::<PathBuf>::new(),
        "探りがファイルを作った"
    );
}

#[test]
fn probe_of_an_unlocked_file_is_absent_and_leaves_it_unlocked_and_in_place() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let probe = LockFilePresence::new(&home);
    let path = home.alive_path("a", WaitKind::Watch);
    std::fs::create_dir(home.alive_dir()).expect("作れる");
    std::fs::write(&path, b"").expect("書ける");

    assert!(!probe.is_present("a", WaitKind::Watch));
    assert!(path.is_file(), "探りがファイルを消した");

    // 探りはロックを残さない: 直後の握りが 1 回目で取れる。
    let mut pauses = 0;
    let held = hold_retrying(&path, || pauses += 1).expect("握れる");
    assert!(held.is_some());
    assert_eq!(pauses, 0, "探りがロックを握ったままにした");
}

#[test]
fn kinds_and_ids_are_independent() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let probe = LockFilePresence::new(&home);

    let _held = hold(&home.alive_path("a", WaitKind::Merge))
        .expect("握れる")
        .expect("誰も握っていない");

    assert!(probe.is_present("a", WaitKind::Merge));
    for kind in [WaitKind::Watch, WaitKind::Load, WaitKind::Resume] {
        assert!(!probe.is_present("a", kind), "{kind:?} は握っていない");
    }
    assert!(!probe.is_present("b", WaitKind::Merge), "別の識別");

    // 同じ識別の別の種類は、握られている種類に邪魔されずに握れる。
    let watch = hold(&home.alive_path("a", WaitKind::Watch)).expect("握れる");
    assert!(watch.is_some());
    assert!(probe.is_present("a", WaitKind::Watch));
}

#[test]
fn hold_reports_an_io_failure_as_an_error_not_as_not_acquired() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    // ロックファイルの場所にフォルダが在る（ファイルとして開けない）。
    let path = home.alive_path("a", WaitKind::Watch);
    std::fs::create_dir_all(&path).expect("作れる");

    assert!(
        hold(&path).is_err(),
        "開けない失敗を「他のプロセスが握っている」に化かした"
    );
}

/// 開けない理由が「無い」でないとき（ここではフォルダ＝アクセス拒否）、探りは「居る」に倒す。
/// 「居ない」に倒すと、生きている参加者が回収されて机を失う（要件 7.2）。
/// Windows だけ: ほかの OS はフォルダを読み取りで開けるので、この失敗が起きない。
#[cfg(windows)]
#[test]
fn probe_that_cannot_open_an_existing_entry_answers_present() {
    let root = TempPath::under_target("impl-watch-presence");
    let home = home_in(&root);
    let path = home.alive_path("a", WaitKind::Watch);
    std::fs::create_dir_all(&path).expect("作れる");

    assert!(LockFilePresence::new(&home).is_present("a", WaitKind::Watch));
    assert!(path.is_dir(), "探りが在るものを消した");
}
