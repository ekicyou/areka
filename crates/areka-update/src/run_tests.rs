//! 一周の入口・定義ファイル・差分 0・n 件の更新・確定の失敗の経路
//! （要件 1.1〜1.4・1.16・1.17・2.5・2.6・4.2〜4.8・5.1・5.3・5.8・6.1・6.7・7.1・8.1〜8.3・9.3）。
//!
//! 各経路で、戻り値の形・観測者が受けた進捗の列・レベル別の記録件数・取得口の呼出・
//! 木のバイト単位の同一性を同時に判定する。

use super::*;
use crate::md5::md5_hex;
use crate::paths::WORK_DIR;
use crate::testkit::{FakeFetch, Pinned, dau, hold, pe_image, pin, tree, txt};
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::WorkDir;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

const HOME: &str = "http://example.test/ghost/";
const MD5_ABC: &str = "900150983cd24fb0d6963f7d28e17f72";

struct Fixture {
    _work: WorkDir,
    target: PathBuf,
}

fn fixture() -> Fixture {
    let work = WorkDir::new().expect("作業フォルダ");
    let target = work.path().join("target");
    fs::create_dir_all(&target).unwrap();
    Fixture {
        _work: work,
        target,
    }
}

struct Ran {
    result: Result<UpdateOutcome, UpdateError>,
    seen: Vec<Progress>,
    records: Vec<CapturedEvent>,
}

impl Ran {
    fn count(&self, level: tracing::Level) -> usize {
        self.records.iter().filter(|e| e.level == level).count()
    }
}

#[allow(clippy::result_large_err)] // 公開の失敗の型をそのまま受ける（lib.rs の run と同じ）。
fn go(homeurl: &str, target: &Path, fetch: &FakeFetch) -> Ran {
    let mut seen = Vec::new();
    let (result, records) = capture(|| {
        run(
            &UpdateRequest { homeurl, target },
            fetch,
            &mut |p: &Progress| seen.push(p.clone()),
        )
    });
    Ran {
        result,
        seen,
        records,
    }
}

fn url(name: &str) -> String {
    format!("{HOME}{name}")
}

#[test]
fn no_difference_writes_nothing_and_does_not_read_delete_txt() {
    let f = fixture();
    fs::write(f.target.join("a.txt"), b"abc").unwrap();
    // 差分 0 の周は delete.txt を読まない＝在っても何も消えない（2.5・6.1）。
    fs::write(f.target.join("delete.txt"), b"a.txt\r\n").unwrap();
    let before = tree(&f.target);
    let fetch = FakeFetch::new().serve(&url("updates2.dau"), &dau(&[&["a.txt", MD5_ABC]], true));

    let ran = go(HOME, &f.target, &fetch);

    let outcome = ran.result.as_ref().expect("差分 0 は成功");
    assert!(
        matches!(
            outcome,
            UpdateOutcome::Unchanged {
                manifest: ManifestName::Updates2Dau
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(
        tree(&f.target),
        before,
        "対象フォルダが 1 バイトでも変わった（作業場所・定義ファイル・delete.txt の適用を含む）"
    );
    assert_eq!(
        ran.seen,
        vec![
            Progress::ManifestFetched {
                name: ManifestName::Updates2Dau
            },
            Progress::DiffDecided { files: vec![] },
        ]
    );
    assert_eq!(ran.count(tracing::Level::ERROR), 0, "{:?}", ran.records);
    assert_eq!(ran.count(tracing::Level::WARN), 0, "{:?}", ran.records);
    assert_eq!(
        ran.count(tracing::Level::INFO),
        2,
        "開始と終了の 2 件: {:?}",
        ran.records
    );
}

#[test]
fn zero_valid_entries_is_unchanged_and_each_invalid_entry_is_warned_once() {
    let f = fixture();
    let before = tree(&f.target);
    // 2 行とも無効（MD5 が無い・`..`）＝有効 0 件（1.16）。
    let fetch = FakeFetch::new().serve(
        &url("updates2.dau"),
        &dau(&[&["a.txt"], &["../x.txt", MD5_ABC]], true),
    );

    let ran = go(HOME, &f.target, &fetch);

    assert!(matches!(
        ran.result,
        Ok(UpdateOutcome::Unchanged {
            manifest: ManifestName::Updates2Dau
        })
    ));
    assert_eq!(tree(&f.target), before);
    assert_eq!(
        ran.count(tracing::Level::WARN),
        2,
        "無効 1 件につき warn 1 件: {:?}",
        ran.records
    );
    assert_eq!(ran.count(tracing::Level::ERROR), 0);
}

#[test]
fn empty_manifest_is_unchanged() {
    let f = fixture();
    let fetch = FakeFetch::new().serve(&url("updates2.dau"), b"");

    let ran = go(HOME, &f.target, &fetch);

    assert!(matches!(ran.result, Ok(UpdateOutcome::Unchanged { .. })));
    assert_eq!(ran.seen[1], Progress::DiffDecided { files: vec![] });
}

#[test]
fn neither_manifest_is_manifest_missing_and_recorded_once() {
    let f = fixture();
    let fetch = FakeFetch::new();

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.result.as_ref().expect_err("両方無ければ失敗");
    assert_eq!(err.reason.kind(), "ManifestMissing");
    assert_eq!(err.stage, Stage::Manifest);
    assert_eq!(fetch.calls(), vec![url("updates2.dau"), url("updates.txt")]);
    assert!(ran.seen.is_empty(), "{:?}", ran.seen);
    assert_eq!(ran.count(tracing::Level::ERROR), 1, "{:?}", ran.records);
    assert_eq!(
        ran.count(tracing::Level::INFO),
        1,
        "開始の 1 件だけ（失敗の終了に info を重ねない）: {:?}",
        ran.records
    );
    let error = ran
        .records
        .iter()
        .find(|e| e.level == tracing::Level::ERROR)
        .unwrap();
    assert_eq!(
        error.field_names_sorted(),
        [
            "detail",
            "file",
            "homeurl",
            "leftovers",
            "message",
            "reason",
            "rolled_back",
            "stage",
            "target",
            "work"
        ]
    );
    assert_eq!(error.field_str("reason"), Some("ManifestMissing"));
}

#[test]
fn falls_back_to_updates_txt_only_when_updates2_dau_is_absent() {
    let f = fixture();
    fs::write(f.target.join("a.txt"), b"abc").unwrap();
    let fetch = FakeFetch::new().serve(
        &url("updates.txt"),
        &txt(&[&format!("file,a.txt\x01{MD5_ABC}")]),
    );

    let ran = go(HOME, &f.target, &fetch);

    assert!(
        matches!(
            ran.result,
            Ok(UpdateOutcome::Unchanged {
                manifest: ManifestName::UpdatesTxt
            })
        ),
        "{:?}",
        ran.result
    );
    assert_eq!(fetch.calls(), vec![url("updates2.dau"), url("updates.txt")]);
    assert_eq!(
        ran.seen[0],
        Progress::ManifestFetched {
            name: ManifestName::UpdatesTxt
        }
    );
}

#[test]
fn network_failure_on_updates2_dau_does_not_fall_back() {
    let f = fixture();
    let fetch = FakeFetch::new()
        .fail(&url("updates2.dau"), FetchError::Timeout)
        .serve(&url("updates.txt"), b"");

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.result.as_ref().expect_err("通信失敗は失敗");
    assert!(
        matches!(
            err.reason,
            FailReason::ManifestFetch {
                name: ManifestName::Updates2Dau,
                source: FetchError::Timeout
            }
        ),
        "{err}"
    );
    assert_eq!(err.file(), Some("updates2.dau"));
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(ran.count(tracing::Level::ERROR), 1, "{:?}", ran.records);
}

#[test]
fn missing_target_fails_at_entry_without_fetching() {
    let f = fixture();
    let missing = f.target.join("nope");
    let fetch = FakeFetch::new();

    let ran = go(HOME, &missing, &fetch);

    let err = ran.result.as_ref().expect_err("対象が無ければ失敗");
    assert!(
        matches!(&err.reason, FailReason::TargetMissing { path } if path == &missing),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Entry);
    assert!(fetch.calls().is_empty());
    assert!(ran.seen.is_empty());
    assert_eq!(ran.count(tracing::Level::ERROR), 1, "{:?}", ran.records);
    assert_eq!(ran.count(tracing::Level::INFO), 0, "{:?}", ran.records);
}

#[test]
fn target_that_is_a_file_fails_at_entry() {
    let f = fixture();
    let file = f.target.join("file.txt");
    fs::write(&file, b"x").unwrap();
    let fetch = FakeFetch::new();

    let ran = go(HOME, &file, &fetch);

    assert_eq!(
        ran.result.as_ref().unwrap_err().reason.kind(),
        "TargetMissing"
    );
    assert!(fetch.calls().is_empty());
}

#[test]
fn non_http_homeurl_fails_at_entry_without_fetching() {
    let f = fixture();
    let fetch = FakeFetch::new();

    let ran = go("ftp://example.test/ghost/", &f.target, &fetch);

    let err = ran.result.as_ref().expect_err("http／https 以外は失敗");
    assert_eq!(err.reason.kind(), "InvalidHomeurl");
    assert_eq!(err.stage, Stage::Entry);
    assert!(fetch.calls().is_empty());
    assert_eq!(ran.count(tracing::Level::ERROR), 1, "{:?}", ran.records);
}

#[test]
fn missing_trailing_slash_is_appended_and_warned_once() {
    let f = fixture();
    let fetch = FakeFetch::new().serve(&url("updates2.dau"), b"");

    let ran = go("http://example.test/ghost", &f.target, &fetch);

    assert!(ran.result.is_ok(), "{:?}", ran.result);
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(ran.count(tracing::Level::WARN), 1, "{:?}", ran.records);
    let warn = ran
        .records
        .iter()
        .find(|e| e.level == tracing::Level::WARN)
        .unwrap();
    assert_eq!(warn.field_str("kind"), Some("HomeurlSlashAppended"));
}

#[test]
fn run_does_not_spawn_a_thread() {
    // 3.7: 観測者は呼び出しスレッドで同期に呼ばれる。
    let f = fixture();
    let fetch = FakeFetch::new().serve(&url("updates2.dau"), b"");
    let caller = std::thread::current().id();
    let mut threads = Vec::new();

    run(
        &UpdateRequest {
            homeurl: HOME,
            target: &f.target,
        },
        &fetch,
        &mut |_: &Progress| threads.push(std::thread::current().id()),
    )
    .unwrap();

    assert_eq!(threads, vec![caller, caller]);
}

/// 要取得 1 件分の進捗 2 つ（`DownloadBegin` → `Md5Compared`・一致）。
fn fetched(file: &str, bytes: &[u8], index: usize, total: usize) -> [Progress; 2] {
    let md5 = md5_hex(bytes);
    [
        Progress::DownloadBegin {
            file: file.to_owned(),
            index,
            total,
        },
        Progress::Md5Compared {
            file: file.to_owned(),
            expected: md5.clone(),
            actual: md5,
            matched: true,
        },
    ]
}

#[test]
fn n_files_are_committed_in_definition_order_then_delete_txt_is_applied() {
    let f = fixture();
    fs::write(f.target.join("b.txt"), b"old b").unwrap();
    fs::write(f.target.join("c.txt"), b"abc").unwrap();
    fs::write(f.target.join("old.txt"), b"stale").unwrap();
    let jp = "日本/a b.txt";
    // delete.txt は更新で届く＝確定の後に適用しないと `old.txt` は消えない（6.1）。
    let files: [(&str, &[u8]); 4] = [
        ("a.txt", b"A"),
        (jp, b"JP"),
        ("b.txt", b"new b"),
        ("delete.txt", b"old.txt\r\n"),
    ];
    let md5s: Vec<String> = files.iter().map(|(_, b)| md5_hex(b)).collect();
    let manifest = dau(
        &[
            &["a.txt", &md5s[0], "charset=UTF-8"],
            &[jp, &md5s[1]],
            &["b.txt", &md5s[2]],
            &["c.txt", MD5_ABC],
            &["delete.txt", &md5s[3]],
        ],
        true,
    );
    let jp_url = url("%E6%97%A5%E6%9C%AC/a%20b.txt");
    let fetch = FakeFetch::new()
        .serve(&url("updates2.dau"), &manifest)
        .serve(&url("a.txt"), files[0].1)
        .serve(&jp_url, files[1].1)
        .serve(&url("b.txt"), files[2].1)
        .serve(&url("c.txt"), b"abc")
        .serve(&url("delete.txt"), files[3].1);

    let ran = go(HOME, &f.target, &fetch);

    let names: Vec<String> = files.iter().map(|(n, _)| n.to_string()).collect();
    let removed = vec![f.target.join("old.txt")];
    match ran.result.as_ref().expect("n 件の更新は成功") {
        UpdateOutcome::Updated {
            manifest: ManifestName::Updates2Dau,
            placed,
            removed: r,
            undeletable,
            leftovers,
        } => {
            assert_eq!(
                placed, &names,
                "置いた一覧は定義の順・定義ファイルを含まない"
            );
            assert_eq!(r, &removed);
            assert!(undeletable.is_empty() && leftovers.is_empty());
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        fetch.calls(),
        vec![
            url("updates2.dau"),
            url("a.txt"),
            jp_url,
            url("b.txt"),
            url("delete.txt")
        ],
        "URL は更新先＋符号化済みのパス・同じ物は取らない"
    );
    let mut expected = vec![
        Progress::ManifestFetched {
            name: ManifestName::Updates2Dau,
        },
        Progress::DiffDecided {
            files: names.clone(),
        },
    ];
    for (i, (name, bytes)) in files.iter().enumerate() {
        expected.extend(fetched(name, bytes, i, files.len()));
    }
    expected.push(Progress::Committed { placed: names });
    expected.push(Progress::Deleted { removed });
    assert_eq!(ran.seen, expected);
    // 作業場所が消え、定義ファイルが対象直下に届いたバイト列のまま置かれ、old.txt が消えた。
    let after: BTreeMap<String, Vec<u8>> = [
        ("a.txt", &b"A"[..]),
        ("b.txt", b"new b"),
        ("c.txt", b"abc"),
        ("delete.txt", b"old.txt\r\n"),
        ("updates2.dau", &manifest),
        ("日本/", b""),
        (jp, b"JP"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_vec()))
    .collect();
    assert_eq!(tree(&f.target), after);
    assert_eq!(ran.count(tracing::Level::ERROR), 0, "{:?}", ran.records);
    assert_eq!(ran.count(tracing::Level::WARN), 0, "{:?}", ran.records);
    assert_eq!(ran.count(tracing::Level::INFO), 2, "{:?}", ran.records);
}

/// `a.dll`（新規）→ `sub/x.txt`（新規・親を作る）→ `b.txt`（既存）の順の定義。
fn three_ending_with_existing(f: &Fixture, a_bytes: &[u8]) -> FakeFetch {
    let manifest = dau(
        &[
            &["a.dll", &md5_hex(a_bytes)],
            &["sub/x.txt", &md5_hex(b"x")],
            &["b.txt", &md5_hex(b"new b")],
        ],
        true,
    );
    fs::write(f.target.join("b.txt"), b"old b").unwrap();
    FakeFetch::new()
        .serve(&url("updates2.dau"), &manifest)
        .serve(&url("a.dll"), a_bytes)
        .serve(&url("sub/x.txt"), b"x")
        .serve(&url("b.txt"), b"new b")
}

#[test]
fn failure_mid_commit_rolls_back_and_cleans_up_the_work_area() {
    let f = fixture();
    let fetch = three_ending_with_existing(&f, b"A");
    let before = tree(&f.target);
    // 退避の `rename` が拒まれる＝3 件目で確定が止まる（5.2・5.4）。
    let held = hold(&f.target.join("b.txt"));

    let ran = go(HOME, &f.target, &fetch);
    drop(held);

    let err = ran.result.as_ref().expect_err("確定の途中で失敗");
    assert_eq!(err.reason.kind(), "CommitWrite", "{err}");
    assert_eq!(err.stage, Stage::Commit);
    assert_eq!(err.file(), Some("b.txt"));
    assert!(err.rolled_back());
    assert!(err.work.is_none() && err.leftovers.is_empty(), "{err:?}");
    assert_eq!(
        tree(&f.target),
        before,
        "置いた 2 件・作った親・作業場所まで開始前と同一"
    );
    assert!(
        !ran.seen
            .iter()
            .any(|p| matches!(p, Progress::Committed { .. } | Progress::Deleted { .. })),
        "{:?}",
        ran.seen
    );
    assert_eq!(ran.count(tracing::Level::ERROR), 1, "{:?}", ran.records);
    assert_eq!(ran.count(tracing::Level::INFO), 1, "{:?}", ran.records);
}

#[test]
fn rollback_failure_keeps_the_work_area_and_returns_its_path() {
    let f = fixture();
    // 1 件目は PE。3 件目を取る時点で `new/a.dll` を写したまま持ち、置いた後の削除を拒ませる
    // （較正は commit_tests・tasks.md 4.2）。3 件目は掴まれていて確定が止まる。
    let fetch = three_ending_with_existing(&f, &pe_image());
    let pinned: Rc<RefCell<Option<Pinned>>> = Rc::default();
    let shelf = f.target.join(WORK_DIR);
    let fetch = {
        let (pinned, shelf, b_url) = (pinned.clone(), shelf.clone(), url("b.txt"));
        fetch.on_get(move |u| {
            if u == b_url {
                let dir = fs::read_dir(&shelf)
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                *pinned.borrow_mut() = Some(pin(&dir.join("new").join("a.dll")));
            }
        })
    };
    let held = hold(&f.target.join("b.txt"));

    let ran = go(HOME, &f.target, &fetch);
    drop(held);
    let was_pinned = pinned.borrow_mut().take().is_some();

    assert!(was_pinned, "注入が走っていない");
    let err = ran.result.as_ref().expect_err("戻せなかった");
    let FailReason::RollbackFailed {
        stuck, restored, ..
    } = &err.reason
    else {
        panic!("{err}");
    };
    // 定義ファイルは最後の 1 件＝止まった時点ではまだ置いていない（5.3）。
    assert_eq!(restored, &["sub/x.txt"]);
    assert_eq!(
        stuck.iter().map(|s| s.file.as_str()).collect::<Vec<_>>(),
        ["a.dll"]
    );
    assert_eq!(err.stage, Stage::Commit);
    assert!(!err.rolled_back());
    let work = err.work.as_ref().expect("作業場所のパスを返す");
    assert!(work.is_dir(), "{}", work.display());
    assert_eq!(work.parent(), Some(shelf.as_path()));
    assert!(err.leftovers.is_empty(), "{err:?}");
    assert_eq!(ran.count(tracing::Level::ERROR), 1, "{:?}", ran.records);
}
