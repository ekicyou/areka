//! 一周の入口・定義ファイル・差分 0 の経路（要件 1.1〜1.4・1.16・1.17・2.5・2.6・7.1・8.1〜8.3・9.3）。
//!
//! 各経路で、戻り値の形・観測者が受けた進捗の列・レベル別の記録件数・取得口の呼出・
//! 木のバイト単位の同一性を同時に判定する。

use super::*;
use crate::testkit::{FakeFetch, dau, tree, txt};
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::WorkDir;
use std::fs;
use std::path::PathBuf;

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
