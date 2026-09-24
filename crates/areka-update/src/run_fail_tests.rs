//! 一周の残りの失敗の経路（要件 4.1・4.5・5.6・8.1・8.4・9.3・9.4・9.6）: 取得失敗・MD5 不一致・
//! 作業場所を作れない・差分の段と確定の段で配下の外。道具は親の `run_tests.rs`。
//!
//! ジャンクションを含む木は `tree` が辿れない（tasks.md 1.3）。木はジャンクションを作る前か
//! 外した後に取る。

use super::*;
use crate::testkit::junction;

#[test]
fn fetch_failure_on_the_second_file_stops_before_the_third() {
    let f = fixture();
    let (fetch, _) = three_ending_with_existing(&f, b"A");
    let fetch = fetch.fail(&url("sub/x.txt"), FetchError::Connect);
    let before = tree(&f.target);

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.failed();
    assert!(
        matches!(
            &err.reason,
            FailReason::FileFetch { file, source: FetchError::Connect } if file == "sub/x.txt"
        ),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Download { index: 1, total: 3 });
    assert_eq!(err.file(), Some("sub/x.txt"));
    assert!(err.rolled_back() && err.work.is_none() && err.leftovers.is_empty());
    assert_eq!(
        fetch.calls(),
        vec![url("updates2.dau"), url("a.dll"), url("sub/x.txt")],
        "3 件目は呼ばれない"
    );
    assert_eq!(tree(&f.target), before, "作業場所まで開始前と同一");
    let mut seen = head(&["a.dll", "sub/x.txt", "b.txt"]);
    seen.extend(fetched("a.dll", b"A", 0, 3));
    seen.push(Progress::DownloadBegin {
        file: "sub/x.txt".into(),
        index: 1,
        total: 3,
    });
    ran.expect(&seen, [1, 0, 1]);
}

#[test]
fn md5_mismatch_is_notified_before_failing_and_stops_fetching() {
    let f = fixture();
    let (fetch, _) = three_ending_with_existing(&f, b"A");
    let fetch = fetch.serve(&url("sub/x.txt"), b"tampered");
    let before = tree(&f.target);

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.failed();
    let (expected, actual) = (md5_hex(b"x"), md5_hex(b"tampered"));
    assert!(
        matches!(
            &err.reason,
            FailReason::Md5Mismatch { file, expected: e, actual: a }
                if file == "sub/x.txt" && e == &expected && a == &actual
        ),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Verify { index: 1, total: 3 });
    assert!(err.rolled_back() && err.work.is_none() && err.leftovers.is_empty());
    assert_eq!(
        fetch.calls(),
        vec![url("updates2.dau"), url("a.dll"), url("sub/x.txt")]
    );
    assert_eq!(tree(&f.target), before);
    // 不一致でも照合の結果を知らせてから失敗する（7.1）。
    let mut seen = head(&["a.dll", "sub/x.txt", "b.txt"]);
    seen.extend(fetched("a.dll", b"A", 0, 3));
    seen.push(Progress::DownloadBegin {
        file: "sub/x.txt".into(),
        index: 1,
        total: 3,
    });
    seen.push(Progress::Md5Compared {
        file: "sub/x.txt".into(),
        expected,
        actual,
        matched: false,
    });
    ran.expect(&seen, [1, 0, 1]);
}

#[test]
fn work_area_blocked_by_a_file_of_the_same_name_fails_before_fetching_files() {
    let f = fixture();
    let (fetch, _) = three_ending_with_existing(&f, b"A");
    // 棚の名前を同名のファイルで塞ぐ（棚を読めず、作れない＝4.1）。
    let shelf = f.target.join(WORK_DIR);
    fs::write(&shelf, b"blocker").unwrap();
    let before = tree(&f.target);

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.failed();
    assert!(
        matches!(&err.reason, FailReason::WorkArea { path, .. } if path == &shelf),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Download { index: 0, total: 3 });
    assert!(err.rolled_back() && err.work.is_none() && err.leftovers.is_empty());
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(tree(&f.target), before, "塞いだファイルごと開始前と同一");
    ran.expect(&head(&["a.dll", "sub/x.txt", "b.txt"]), [1, 0, 1]);
}

#[test]
fn writing_a_fetched_file_into_the_work_area_fails_with_its_name() {
    let f = fixture();
    let (fetch, _) = three_ending_with_existing(&f, b"A");
    // 2 件目を取る最中に、作業場所の書き先を同名のフォルダで塞ぐ。
    let fetch = {
        let (shelf, x_url) = (f.target.join(WORK_DIR), url("sub/x.txt"));
        fetch.on_get(move |u| {
            if u == x_url {
                let dir = fs::read_dir(&shelf).unwrap().next().unwrap().unwrap();
                fs::create_dir_all(dir.path().join("new/sub/x.txt")).unwrap();
            }
        })
    };
    let before = tree(&f.target);

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.failed();
    assert_eq!(err.reason.kind(), "WorkArea", "{err}");
    assert_eq!(err.stage, Stage::Download { index: 1, total: 3 });
    assert_eq!(err.file(), Some("sub/x.txt"));
    assert!(err.rolled_back() && err.work.is_none() && err.leftovers.is_empty());
    assert_eq!(tree(&f.target), before, "作業場所まで開始前と同一");
    let mut seen = head(&["a.dll", "sub/x.txt", "b.txt"]);
    seen.extend(fetched("a.dll", b"A", 0, 3));
    seen.extend(fetched("sub/x.txt", b"x", 1, 3));
    ran.expect(&seen, [1, 0, 1]);
}

#[test]
fn junction_out_of_the_target_fails_at_the_diff_stage() {
    let f = fixture();
    fs::write(f.target.join("keep.txt"), b"keep").unwrap();
    let outside = f.outside();
    fs::write(outside.join("x.txt"), b"outside").unwrap();
    let (before, outside_before) = (tree(&f.target), tree(&outside));
    let link = f.target.join("link");
    junction(&link, &outside);
    let fetch = FakeFetch::new()
        .serve(
            &url("updates2.dau"),
            &dau(&[&["link/x.txt", &md5_hex(b"new")]], true),
        )
        .serve(&url("link/x.txt"), b"new");

    let ran = go(HOME, &f.target, &fetch);
    fs::remove_dir(&link).expect("ジャンクションを外せる");

    let err = ran.failed();
    assert!(
        matches!(&err.reason, FailReason::EscapesTarget { path } if path == &link.join("x.txt")),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Diff);
    assert_eq!(err.file(), Some("link/x.txt"));
    assert_eq!(
        fetch.calls(),
        vec![url("updates2.dau")],
        "定義ファイルの 1 回だけ"
    );
    assert_eq!(tree(&f.target), before);
    assert_eq!(tree(&outside), outside_before, "外は 1 バイトも変わらない");
    ran.expect(
        &[Progress::ManifestFetched {
            name: ManifestName::Updates2Dau,
        }],
        [1, 0, 1],
    );
}

#[test]
fn parent_swapped_for_a_junction_after_the_diff_fails_at_commit_and_rolls_back() {
    let f = fixture();
    fs::write(f.target.join("a.txt"), b"old a").unwrap();
    // 差分の時点では配下の実在するフォルダ（5.6＝確定の前にも確かめる）。
    let sub = f.target.join("sub");
    fs::create_dir(&sub).unwrap();
    let outside = f.outside();
    let before = tree(&f.target);
    let fetch = FakeFetch::new()
        .serve(
            &url("updates2.dau"),
            &dau(
                &[
                    &["a.txt", &md5_hex(b"new a")],
                    &["sub/b.txt", &md5_hex(b"b")],
                ],
                true,
            ),
        )
        .serve(&url("a.txt"), b"new a")
        .serve(&url("sub/b.txt"), b"b");
    // 2 件目を取る最中に、親をジャンクションへ差し替える。
    let fetch = {
        let (sub, outside, b_url) = (sub.clone(), outside.clone(), url("sub/b.txt"));
        fetch.on_get(move |u| {
            if u == b_url {
                fs::remove_dir(&sub).unwrap();
                junction(&sub, &outside);
            }
        })
    };

    let ran = go(HOME, &f.target, &fetch);
    let swapped = fs::symlink_metadata(&sub).is_ok_and(|m| m.file_type().is_symlink());
    fs::remove_dir(&sub).expect("ジャンクションを外せる");

    assert!(swapped, "注入が走っていない");
    let err = ran.failed();
    assert!(
        matches!(&err.reason, FailReason::EscapesTarget { path } if path == &sub.join("b.txt")),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Commit);
    assert_eq!(err.file(), Some("sub/b.txt"));
    assert!(err.rolled_back() && err.work.is_none() && err.leftovers.is_empty());
    // 1 件目は元の内容へ戻り、作業場所も消えている（差し替えた `sub/` だけが無い）。
    let mut expected = before;
    expected.remove("sub/");
    assert_eq!(tree(&f.target), expected);
    assert!(tree(&outside).is_empty(), "外に何も作らない");
    let mut seen = head(&["a.txt", "sub/b.txt"]);
    seen.extend(fetched("a.txt", b"new a", 0, 2));
    seen.extend(fetched("sub/b.txt", b"b", 1, 2));
    ran.expect(&seen, [1, 0, 1]);
}
