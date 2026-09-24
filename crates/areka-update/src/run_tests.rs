//! 一周の入口・定義ファイル・差分 0・n 件の更新・確定の途中失敗・戻せなかった経路
//! （要件 1.1〜1.4・1.16・1.17・2.5・2.6・4.2〜4.8・5.1・5.3・5.5・5.8・6.1・6.7・7.1・8.1〜8.4・9.3・9.4）。
//! 残りの失敗の経路は `run_fail_tests.rs`（道具はここに 1 か所）。
//!
//! 各経路で、戻り値の形・観測者が受けた進捗の列・レベル別の記録件数・取得口の呼出・
//! 木のバイト単位の同一性を同時に判定する（[`Ran::expect`]・[`Ran::failed`]）。

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

pub(super) const HOME: &str = "http://example.test/ghost/";
pub(super) const MD5_ABC: &str = "900150983cd24fb0d6963f7d28e17f72";

pub(super) struct Fixture {
    pub(super) work: WorkDir,
    pub(super) target: PathBuf,
}

impl Fixture {
    /// 対象フォルダの外（同じ作業フォルダの中の兄弟）。ジャンクションの行き先に使う。
    pub(super) fn outside(&self) -> PathBuf {
        let outside = self.work.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        outside
    }
}

pub(super) fn fixture() -> Fixture {
    let work = WorkDir::new().expect("作業フォルダ");
    let target = work.path().join("target");
    fs::create_dir_all(&target).unwrap();
    Fixture { work, target }
}

pub(super) struct Ran {
    pub(super) result: Result<UpdateOutcome, UpdateError>,
    pub(super) seen: Vec<Progress>,
    pub(super) records: Vec<CapturedEvent>,
}

impl Ran {
    fn count(&self, level: tracing::Level) -> usize {
        self.records.iter().filter(|e| e.level == level).count()
    }

    /// 観測者が受けた進捗の列と、記録の件数 `[info, warn, error]` を同時に判定する（7.1・8.4）。
    /// 失敗 1 回につき error 1 件・警告 1 件につき warn 1 件を、呼び手が件数で書く。
    fn expect(&self, seen: &[Progress], [info, warn, error]: [usize; 3]) {
        assert_eq!(self.seen, seen, "進捗の列");
        use tracing::Level;
        assert_eq!(
            [Level::INFO, Level::WARN, Level::ERROR].map(|l| self.count(l)),
            [info, warn, error],
            "記録の件数 [info, warn, error]: {:#?}",
            self.records
        );
    }

    /// 失敗を取り出し、error の記録がちょうど 1 件で、その欄（更新先・対象・段・理由・
    /// 原因のファイル・戻せたか・作業場所）が戻り値と一致することも判定する（8.1）。
    pub(super) fn failed(&self) -> &UpdateError {
        let err = self.result.as_ref().expect_err("失敗のはず");
        let errors: Vec<&CapturedEvent> = self
            .records
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .collect();
        let [record] = errors[..] else {
            panic!("error の記録は 1 件のはず: {err}: {errors:#?}");
        };
        assert_eq!(record.field("homeurl"), Some(err.homeurl.as_str()));
        assert_eq!(
            record.field("target"),
            Some(err.target.display().to_string().as_str())
        );
        let work = err.work.as_deref().map(|w| w.display().to_string());
        assert_eq!(
            record.field("work"),
            Some(work.unwrap_or_default().as_str())
        );
        assert_eq!(record.field_str("reason"), Some(err.reason.kind()));
        assert_eq!(
            record.field("stage"),
            Some(format!("{:?}", err.stage).as_str())
        );
        assert_eq!(
            record.field_str("file"),
            Some(err.file().unwrap_or_default())
        );
        assert_eq!(
            record.field("rolled_back"),
            Some(err.rolled_back().to_string().as_str())
        );
        err
    }
}

#[allow(clippy::result_large_err)] // 公開の失敗の型をそのまま受ける（lib.rs の run と同じ）。
pub(super) fn go(homeurl: &str, target: &Path, fetch: &FakeFetch) -> Ran {
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

pub(super) fn url(name: &str) -> String {
    format!("{HOME}{name}")
}

/// `updates2.dau` を取って差分が `files` に決まるまでの進捗 2 つ。
fn head(files: &[&str]) -> Vec<Progress> {
    vec![
        Progress::ManifestFetched {
            name: ManifestName::Updates2Dau,
        },
        Progress::DiffDecided {
            files: files.iter().map(|s| s.to_string()).collect(),
        },
    ]
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

/// 相対名 → バイト列の木（フォルダは末尾 `/`・値は空）。
fn tree_of(entries: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_vec()))
        .collect()
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
    // info は開始と終了の 2 件。
    ran.expect(&head(&[]), [2, 0, 0]);
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
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(tree(&f.target), before);
    // 無効 1 件につき warn 1 件。
    ran.expect(&head(&[]), [2, 2, 0]);
}

#[test]
fn empty_manifest_is_unchanged() {
    let f = fixture();
    let before = tree(&f.target);
    let fetch = FakeFetch::new().serve(&url("updates2.dau"), b"");

    let ran = go(HOME, &f.target, &fetch);

    assert!(matches!(ran.result, Ok(UpdateOutcome::Unchanged { .. })));
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(tree(&f.target), before);
    ran.expect(&head(&[]), [2, 0, 0]);
}

#[test]
fn neither_manifest_is_manifest_missing_and_recorded_once() {
    let f = fixture();
    let before = tree(&f.target);
    let fetch = FakeFetch::new();

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.failed();
    assert_eq!(err.reason.kind(), "ManifestMissing");
    assert_eq!(err.stage, Stage::Manifest);
    assert_eq!(fetch.calls(), vec![url("updates2.dau"), url("updates.txt")]);
    assert_eq!(tree(&f.target), before);
    // info は開始の 1 件だけ（失敗の終了に info を重ねない）。
    ran.expect(&[], [1, 0, 1]);
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
}

#[test]
fn falls_back_to_updates_txt_only_when_updates2_dau_is_absent() {
    let f = fixture();
    fs::write(f.target.join("a.txt"), b"old").unwrap();
    // 手元の古い updates2.dau は定義に無いローカルのファイル＝触らない。
    fs::write(f.target.join("updates2.dau"), b"stale").unwrap();
    let manifest = txt(&[&format!("file,a.txt\x01{MD5_ABC}")]);
    let fetch = FakeFetch::new()
        .serve(&url("updates.txt"), &manifest)
        .serve(&url("a.txt"), b"abc");

    let ran = go(HOME, &f.target, &fetch);

    match ran.result.as_ref().expect("後退した周の更新は成功") {
        UpdateOutcome::Updated {
            manifest: ManifestName::UpdatesTxt,
            placed,
            ..
        } => assert_eq!(placed, &["a.txt"]),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        fetch.calls(),
        vec![url("updates2.dau"), url("updates.txt"), url("a.txt")]
    );
    assert_eq!(
        tree(&f.target),
        tree_of(&[
            ("a.txt", b"abc"),
            ("updates.txt", &manifest),
            ("updates2.dau", b"stale"),
        ]),
        "置かれる定義ファイルは updates.txt・古い updates2.dau はそのまま"
    );
    let mut seen = vec![
        Progress::ManifestFetched {
            name: ManifestName::UpdatesTxt,
        },
        Progress::DiffDecided {
            files: vec!["a.txt".into()],
        },
    ];
    seen.extend(fetched("a.txt", b"abc", 0, 1));
    seen.push(Progress::Committed {
        placed: vec!["a.txt".into()],
    });
    seen.push(Progress::Deleted { removed: vec![] });
    ran.expect(&seen, [2, 0, 0]);
}

#[test]
fn network_failure_on_updates2_dau_does_not_fall_back() {
    let f = fixture();
    let before = tree(&f.target);
    let fetch = FakeFetch::new()
        .fail(&url("updates2.dau"), FetchError::Timeout)
        .serve(&url("updates.txt"), b"");

    let ran = go(HOME, &f.target, &fetch);

    let err = ran.failed();
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
    assert_eq!(err.stage, Stage::Manifest);
    assert_eq!(err.file(), Some("updates2.dau"));
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(tree(&f.target), before);
    ran.expect(&[], [1, 0, 1]);
}

#[test]
fn missing_target_fails_at_entry_without_fetching() {
    let f = fixture();
    let before = tree(&f.target);
    let missing = f.target.join("nope");
    let fetch = FakeFetch::new();

    let ran = go(HOME, &missing, &fetch);

    let err = ran.failed();
    assert!(
        matches!(&err.reason, FailReason::TargetMissing { path } if path == &missing),
        "{err}"
    );
    assert_eq!(err.stage, Stage::Entry);
    assert!(fetch.calls().is_empty());
    assert_eq!(tree(&f.target), before, "対象フォルダを作らない");
    // 開始の info より前に止まる。
    ran.expect(&[], [0, 0, 1]);
}

#[test]
fn target_that_is_a_file_fails_at_entry() {
    let f = fixture();
    let file = f.target.join("file.txt");
    fs::write(&file, b"x").unwrap();
    let before = tree(&f.target);
    let fetch = FakeFetch::new();

    let ran = go(HOME, &file, &fetch);

    let err = ran.failed();
    assert_eq!(err.reason.kind(), "TargetMissing");
    assert_eq!(err.stage, Stage::Entry);
    assert!(fetch.calls().is_empty());
    assert_eq!(tree(&f.target), before);
    ran.expect(&[], [0, 0, 1]);
}

#[test]
fn non_http_homeurl_fails_at_entry_without_fetching() {
    let f = fixture();
    let before = tree(&f.target);
    let fetch = FakeFetch::new();

    let ran = go("ftp://example.test/ghost/", &f.target, &fetch);

    let err = ran.failed();
    assert_eq!(err.reason.kind(), "InvalidHomeurl");
    assert_eq!(err.stage, Stage::Entry);
    assert!(fetch.calls().is_empty());
    assert_eq!(tree(&f.target), before);
    ran.expect(&[], [0, 0, 1]);
}

#[test]
fn missing_trailing_slash_is_appended_and_warned_once() {
    let f = fixture();
    let before = tree(&f.target);
    let fetch = FakeFetch::new().serve(&url("updates2.dau"), b"");

    let ran = go("http://example.test/ghost", &f.target, &fetch);

    assert!(
        matches!(ran.result, Ok(UpdateOutcome::Unchanged { .. })),
        "{:?}",
        ran.result
    );
    assert_eq!(fetch.calls(), vec![url("updates2.dau")]);
    assert_eq!(tree(&f.target), before);
    ran.expect(&head(&[]), [2, 1, 0]);
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
    let mut expected = head(&files.map(|(n, _)| n));
    for (i, (name, bytes)) in files.iter().enumerate() {
        expected.extend(fetched(name, bytes, i, files.len()));
    }
    expected.push(Progress::Committed { placed: names });
    expected.push(Progress::Deleted { removed });
    ran.expect(&expected, [2, 0, 0]);
    // 作業場所が消え、定義ファイルが対象直下に届いたバイト列のまま置かれ、old.txt が消えた。
    let after = tree_of(&[
        ("a.txt", b"A"),
        ("b.txt", b"new b"),
        ("c.txt", b"abc"),
        ("delete.txt", b"old.txt\r\n"),
        ("updates2.dau", &manifest),
        ("日本/", b""),
        (jp, b"JP"),
    ]);
    assert_eq!(tree(&f.target), after);
}

/// `a.dll`（新規）→ `sub/x.txt`（新規・親を作る）→ `b.txt`（既存）の順の定義と、その定義ファイル。
pub(super) fn three_ending_with_existing(f: &Fixture, a_bytes: &[u8]) -> (FakeFetch, Vec<u8>) {
    let manifest = dau(
        &[
            &["a.dll", &md5_hex(a_bytes)],
            &["sub/x.txt", &md5_hex(b"x")],
            &["b.txt", &md5_hex(b"new b")],
        ],
        true,
    );
    fs::write(f.target.join("b.txt"), b"old b").unwrap();
    let fetch = FakeFetch::new()
        .serve(&url("updates2.dau"), &manifest)
        .serve(&url("a.dll"), a_bytes)
        .serve(&url("sub/x.txt"), b"x")
        .serve(&url("b.txt"), b"new b");
    (fetch, manifest)
}

/// [`three_ending_with_existing`] の 3 件を全部取って照合し終えるまでの進捗。
fn three_fetched(a_bytes: &[u8]) -> Vec<Progress> {
    let mut seen = head(&["a.dll", "sub/x.txt", "b.txt"]);
    seen.extend(fetched("a.dll", a_bytes, 0, 3));
    seen.extend(fetched("sub/x.txt", b"x", 1, 3));
    seen.extend(fetched("b.txt", b"new b", 2, 3));
    seen
}

#[test]
fn failure_mid_commit_rolls_back_and_cleans_up_the_work_area() {
    let f = fixture();
    let (fetch, _) = three_ending_with_existing(&f, b"A");
    let before = tree(&f.target);
    // 退避の `rename` が拒まれる＝3 件目で確定が止まる（5.2・5.4）。
    let held = hold(&f.target.join("b.txt"));

    let ran = go(HOME, &f.target, &fetch);
    drop(held);

    let err = ran.failed();
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
    // `Committed`・`Deleted` は来ない。
    ran.expect(&three_fetched(b"A"), [1, 0, 1]);
}

#[test]
fn rollback_failure_keeps_the_work_area_and_returns_its_path() {
    let f = fixture();
    // 1 件目は PE。3 件目を取る時点で `new/a.dll` を写したまま持ち、置いた後の削除を拒ませる
    // （較正は commit_tests・tasks.md 4.2）。3 件目は掴まれていて確定が止まる。
    let pe = pe_image();
    let (fetch, manifest) = three_ending_with_existing(&f, &pe);
    let before = tree(&f.target);
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
    let err = ran.failed();
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
    // 戻せた一覧と戻せなかった一覧は error の記録にも載る（5.5）。
    let record = ran
        .records
        .iter()
        .find(|e| e.level == tracing::Level::ERROR);
    let detail = record.and_then(|e| e.field("detail")).unwrap_or_default();
    assert!(
        detail.contains("a.dll") && detail.contains("sub/x.txt"),
        "{detail}"
    );
    assert_eq!(err.stage, Stage::Commit);
    assert!(!err.rolled_back());
    let work = err.work.as_ref().expect("作業場所のパスを返す");
    assert!(work.is_dir(), "{}", work.display());
    assert_eq!(work.parent(), Some(shelf.as_path()));
    assert!(err.leftovers.is_empty(), "{err:?}");
    ran.expect(&three_fetched(&pe), [1, 0, 1]);
    // 作業場所の外は、消せなかった a.dll だけが開始前と違う（sub/x.txt と sub/ は戻した）。
    let mut expected = before;
    expected.insert("a.dll".into(), pe);
    let outside_work: BTreeMap<String, Vec<u8>> = tree(&f.target)
        .into_iter()
        .filter(|(k, _)| !k.starts_with(&format!("{WORK_DIR}/")))
        .collect();
    assert_eq!(outside_work, expected);
    // 作業場所には、まだ置いていない b.txt と定義ファイルが届いた中身のまま残る。
    assert_eq!(
        tree(work),
        tree_of(&[
            ("new/", b""),
            ("new/b.txt", b"new b"),
            ("new/sub/", b""),
            ("new/updates2.dau", &manifest),
            ("old/", b""),
        ])
    );
}

/// 1 回の成功した周で警告の 7 種と、取り除けなかった物・残骸を全て出し、種ごとに
/// warn がちょうど 1 行で理由の欄を持つことを判定する（8.2）。
#[test]
fn every_warning_kind_is_recorded_once() {
    let f = fixture();
    // 取り除けない物・種別の食い違い（ファイルの行に同名のフォルダ）・読めない delete2.txt。
    fs::write(f.target.join("held.txt"), b"held").unwrap();
    fs::create_dir(f.target.join("dir.txt")).unwrap();
    fs::create_dir(f.target.join("delete2.txt")).unwrap();
    // 戻せなかった他の走行の元の内容＝棚に残す残骸（4.8・5.5）。
    let other = f.target.join(WORK_DIR).join("9-9");
    fs::create_dir_all(other.join("old")).unwrap();
    fs::write(other.join("old/b.txt"), b"b").unwrap();
    let delete: &[u8] = b"../x.txt\r\ndir.txt\r\nheld.txt\r\n";
    let manifest = dau(
        &[
            &["a.txt", &md5_hex(b"A"), "charset=bogus"],
            &["a.txt", &md5_hex(b"A")],
            &["bad.txt"],
            &["delete.txt", &md5_hex(delete)],
        ],
        true,
    );
    let fetch = FakeFetch::new()
        .serve(&url("updates2.dau"), &manifest)
        .serve(&url("a.txt"), b"A")
        .serve(&url("delete.txt"), delete);
    let held = hold(&f.target.join("held.txt"));

    let ran = go("http://example.test/ghost", &f.target, &fetch);
    drop(held);

    match ran.result.as_ref().expect("警告だけの周は成功") {
        UpdateOutcome::Updated {
            undeletable,
            leftovers,
            ..
        } => {
            assert_eq!(
                undeletable.iter().map(|u| &u.path).collect::<Vec<_>>(),
                [&f.target.join("held.txt")]
            );
            assert_eq!(leftovers, &[other]);
        }
        outcome => panic!("{outcome:?}"),
    }
    let names = ["a.txt", "delete.txt"];
    let mut seen = head(&names);
    seen.extend(fetched("a.txt", b"A", 0, 2));
    seen.extend(fetched("delete.txt", delete, 1, 2));
    seen.push(Progress::Committed {
        placed: names.map(String::from).to_vec(),
    });
    seen.push(Progress::Deleted { removed: vec![] });
    ran.expect(&seen, [2, 9, 0]);
    let warns: Vec<&CapturedEvent> = ran
        .records
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for w in &warns {
        *kinds.entry(w.field_str("kind").unwrap_or("?")).or_default() += 1;
        assert!(
            w.field("detail").or(w.field("path")).is_some(),
            "理由の欄が無い: {w:?}"
        );
    }
    let expected: BTreeMap<&str, usize> = [
        "HomeurlSlashAppended",
        "UnknownCharset",
        "InvalidEntry",
        "DuplicateEntry",
        "DeleteLineIgnored",
        "DeleteKindMismatch",
        "DeleteFileUnreadable",
        "undeletable",
        "leftover",
    ]
    .into_iter()
    .map(|k| (k, 1))
    .collect();
    assert_eq!(kinds, expected);
}

#[path = "run_fail_tests.rs"]
mod fail;
