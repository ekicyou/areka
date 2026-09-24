//! ネットワーク更新の本番クレート（spec: areka-P0-update-engine）。
//!
//! 更新定義ファイル（`updates2.dau`／`updates.txt`）を読み、手元との差分だけを
//! 取得・照合して、ファイル単位で確定する。失敗したら逆順に戻す。
//!
//! # 一周を回す
//!
//! ```rust,no_run
//! use areka_update::{
//!     Fetch, FetchError, Progress, UpdateError, UpdateOutcome, UpdateRequest, WinHttpFetch, run,
//! };
//! use std::path::Path;
//!
//! fn update(homeurl: &str, target: &Path) -> Result<(), FetchError> {
//!     let fetch = WinHttpFetch::new()?;
//!     let boundary: &dyn Fetch = &fetch;
//!     let result: Result<UpdateOutcome, UpdateError> = run(
//!         &UpdateRequest { homeurl, target },
//!         boundary,
//!         &mut |p: &Progress| println!("{p:?}"),
//!     );
//!     match result {
//!         Ok(UpdateOutcome::Unchanged { .. }) => println!("差分 0"),
//!         Ok(UpdateOutcome::Updated { placed, .. }) => println!("{} 件を更新", placed.len()),
//!         Err(err) => println!("{}: {:?}", err.reason.kind(), err.file()),
//!     }
//!     Ok(())
//! }
//! # let _ = update;
//! ```
//!
//! # 記録はここだけが出す
//!
//! 下の層は警告も失敗も値で返す。`run` がそれを `warn!` に写し、失敗は `Err` を返す
//! 直前に [`log_failure`] で `error!` を 1 回だけ出す（要件 8.1〜8.3）。

use std::fs;
use std::path::Path;

mod commit;
mod delete;
mod diff;
mod error;
mod fetch;
mod manifest;
mod md5;
mod outcome;
mod paths;
#[cfg(test)]
mod testkit;
mod urlpath;
mod winhttp;
mod work;

use commit::CommitFailure;
use diff::DiffFailure;
use work::WorkArea;

pub use error::{
    DeleteWhy, FailReason, FetchError, InvalidWhy, Stage, Stuck, UpdateError, UpdateWarning,
};
pub use fetch::Fetch;
pub use outcome::{ManifestName, Progress, Undeletable, UpdateOutcome};
pub use winhttp::{MAX_BODY_BYTES, WinHttpFetch};

/// 一周の入力。`homeurl` は解決済みの更新先 URL（末尾の `/` は無くてもよい）、
/// `target` は定義ファイルのパスの根（ゴーストなら `(myghost)/`）。
pub struct UpdateRequest<'a> {
    pub homeurl: &'a str,
    pub target: &'a Path,
}

/// 一周を同期で進める。自らスレッドを起こさない（3.7）。
///
/// 成功は `UpdateOutcome`（差分 0 と n 件を型で区別）、失敗は `UpdateError`
/// （段・閉じた理由・原因のファイル・戻せたか・残骸・元の内容が残る作業場所）。
/// 失敗の全経路で `tracing::error!` を 1 件だけ出し、同じ内容を戻り値に持つ（8.1）。
///
/// # Errors
///
/// 入口の検査・定義ファイル・差分・取得・照合・確定のいずれかで止まったとき。
// 失敗の型は設計の公開面そのもの（段・理由・残骸・作業場所を値で持つ）。一周に 1 回しか
// 返らないので箱に入れて小さくする得が無い。
#[allow(clippy::result_large_err)]
pub fn run(
    request: &UpdateRequest<'_>,
    fetch: &dyn Fetch,
    observe: &mut dyn FnMut(&Progress),
) -> Result<UpdateOutcome, UpdateError> {
    let mut homeurl = request.homeurl.to_owned();
    let mut area = None;
    walk(request.target, fetch, observe, &mut homeurl, &mut area).map_err(|(stage, reason)| {
        // 戻せなかったときだけ作業場所を残す（元の内容がそこに在る＝5.5）。
        let (work, leftovers) = match area {
            Some(area) if matches!(reason, FailReason::RollbackFailed { .. }) => {
                let (dir, residue) = area.keep();
                (Some(dir), residue)
            }
            Some(area) => (None, area.cleanup()),
            None => (None, Vec::new()),
        };
        leftovers.iter().for_each(|p| warn_leftover(&homeurl, p));
        log_failure(UpdateError {
            homeurl,
            target: request.target.to_path_buf(),
            stage,
            reason,
            leftovers,
            work,
        })
    })
}

/// 段で止まった失敗。`Err` は [`run`] が [`log_failure`] を通して組む。
type Failed = (Stage, FailReason);

/// 段を直列に進める。記録は警告と成功の終了だけ（失敗の記録は [`run`] の 1 か所）。
/// 作業場所を作ったら `slot` に預ける（失敗時に `run` が片付けるか残す）。
fn walk(
    target: &Path,
    fetch: &dyn Fetch,
    observe: &mut dyn FnMut(&Progress),
    homeurl: &mut String,
    slot: &mut Option<WorkArea>,
) -> Result<UpdateOutcome, Failed> {
    // 1. 入口（1.17・1.4）。取得口はまだ呼ばない。
    if !target.is_dir() {
        let path = target.to_path_buf();
        return Err((Stage::Entry, FailReason::TargetMissing { path }));
    }
    if !(homeurl.starts_with("http://") || homeurl.starts_with("https://")) {
        let homeurl = homeurl.clone();
        return Err((Stage::Entry, FailReason::InvalidHomeurl { homeurl }));
    }
    if !homeurl.ends_with('/') {
        homeurl.push('/');
        log_warning(homeurl, &UpdateWarning::HomeurlSlashAppended);
    }
    tracing::info!(
        homeurl = %homeurl,
        target = %target.display(),
        "[areka_update] update started"
    );

    // 2〜3. 定義ファイル（1.1〜1.3）。
    let (name, manifest_bytes) = fetch_manifest(fetch, homeurl)?;
    observe(&Progress::ManifestFetched { name });
    let (manifest, warnings) = manifest::parse(name, &manifest_bytes);
    warnings.iter().for_each(|w| log_warning(homeurl, w));

    // 4. 差分（2.1〜2.6・1.16）。
    let target_real = fs::canonicalize(target).map_err(|source| {
        let path = target.to_path_buf();
        (Stage::Diff, FailReason::LocalUnreadable { path, source })
    })?;
    let need = diff::plan(target, &target_real, &manifest).map_err(|failure| {
        let reason = match failure {
            DiffFailure::Unreadable { path, source } => {
                FailReason::LocalUnreadable { path, source }
            }
            DiffFailure::Escapes { path } => FailReason::EscapesTarget { path },
        };
        (Stage::Diff, reason)
    })?;
    let files: Vec<String> = need
        .iter()
        .map(|&i| manifest.entries[i].local.clone())
        .collect();
    observe(&Progress::DiffDecided {
        files: files.clone(),
    });
    if files.is_empty() {
        log_finished(homeurl, target, "unchanged", 0, 0, 0);
        return Ok(UpdateOutcome::Unchanged {
            manifest: manifest.name,
        });
    }

    // 5. 作業場所（4.1）。
    let total = files.len();
    let created = WorkArea::create(target).map_err(|(path, source)| {
        let stage = Stage::Download { index: 0, total };
        (stage, FailReason::WorkArea { path, source })
    })?;
    let area = slot.insert(created);

    // 6. 1 件ずつ取得 → 照合 → 書く（4.2〜4.7）。バイト列は 1 件ごとに手放す。
    for (index, &i) in need.iter().enumerate() {
        let entry = &manifest.entries[i];
        let file = entry.local.clone();
        observe(&Progress::DownloadBegin {
            file: file.clone(),
            index,
            total,
        });
        let stage = Stage::Download { index, total };
        let bytes = fetch
            .get(&format!("{homeurl}{}", entry.url_path))
            .map_err(|source| {
                let file = file.clone();
                (stage, FailReason::FileFetch { file, source })
            })?;
        let actual = md5::md5_hex(&bytes);
        let matched = actual == entry.md5;
        observe(&Progress::Md5Compared {
            file: file.clone(),
            expected: entry.md5.clone(),
            actual: actual.clone(),
            matched,
        });
        if !matched {
            let expected = entry.md5.clone();
            let reason = FailReason::Md5Mismatch {
                file,
                expected,
                actual,
            };
            return Err((Stage::Verify { index, total }, reason));
        }
        area.put(&entry.local, &bytes)
            .map_err(|(path, source)| (stage, FailReason::WorkArea { path, source }))?;
    }

    // 7〜8. 定義ファイルを最後の 1 件として確定（5.1〜5.7）。
    // 定義ファイルの書き込みは確定の準備なので、失敗は確定の段に数える。
    area.put(manifest.name.file_name(), &manifest_bytes)
        .map_err(|(path, source)| (Stage::Commit, FailReason::WorkArea { path, source }))?;
    let mut to_place = files;
    to_place.push(manifest.name.file_name().to_owned());
    let mut placed = commit::commit(target, &target_real, area, &to_place).map_err(|failure| {
        let reason = match failure {
            CommitFailure::Write { path, source } => FailReason::CommitWrite { path, source },
            CommitFailure::Escapes { path } => FailReason::EscapesTarget { path },
            CommitFailure::RollbackFailed {
                path,
                source,
                restored,
                stuck,
            } => FailReason::RollbackFailed {
                path,
                source,
                restored,
                stuck,
            },
        };
        (Stage::Commit, reason)
    })?;
    // 定義ファイル自身は `manifest` 欄が示す（`placed` には含めない）。
    placed.pop();
    observe(&Progress::Committed {
        placed: placed.clone(),
    });

    // 9. delete.txt（6.1〜6.7）。一周を失敗にしない。
    let report = delete::apply(target, &target_real, manifest.charset);
    report.warnings.iter().for_each(|w| log_warning(homeurl, w));
    for u in &report.undeletable {
        tracing::warn!(
            homeurl = %homeurl,
            kind = "undeletable",
            path = %u.path.display(),
            error = %u.source,
            "[areka_update] could not remove"
        );
    }
    observe(&Progress::Deleted {
        removed: report.removed.clone(),
    });

    // 10. 片付け（4.8）。
    let leftovers = slot.take().map(WorkArea::cleanup).unwrap_or_default();
    leftovers.iter().for_each(|p| warn_leftover(homeurl, p));
    log_finished(
        homeurl,
        target,
        "updated",
        placed.len(),
        report.removed.len(),
        leftovers.len(),
    );
    Ok(UpdateOutcome::Updated {
        manifest: manifest.name,
        placed,
        removed: report.removed,
        undeletable: report.undeletable,
        leftovers,
    })
}

/// `updates2.dau` → 「無い」ときだけ `updates.txt`（1.1〜1.3）。
fn fetch_manifest(fetch: &dyn Fetch, homeurl: &str) -> Result<(ManifestName, Vec<u8>), Failed> {
    for name in [ManifestName::Updates2Dau, ManifestName::UpdatesTxt] {
        match fetch.get(&format!("{homeurl}{}", name.file_name())) {
            Ok(bytes) => return Ok((name, bytes)),
            Err(FetchError::NotFound) => continue,
            Err(source) => {
                return Err((Stage::Manifest, FailReason::ManifestFetch { name, source }));
            }
        }
    }
    Err((Stage::Manifest, FailReason::ManifestMissing {}))
}

/// 警告 1 件を 1 行の `warn!` に写す（8.2）。`detail` は変種の欄を全部持つ。
fn log_warning(homeurl: &str, warning: &UpdateWarning) {
    let kind = match warning {
        UpdateWarning::HomeurlSlashAppended => "HomeurlSlashAppended",
        UpdateWarning::UnknownCharset { .. } => "UnknownCharset",
        UpdateWarning::InvalidEntry { .. } => "InvalidEntry",
        UpdateWarning::DuplicateEntry { .. } => "DuplicateEntry",
        UpdateWarning::DeleteLineIgnored { .. } => "DeleteLineIgnored",
        UpdateWarning::DeleteKindMismatch { .. } => "DeleteKindMismatch",
        UpdateWarning::DeleteFileUnreadable { .. } => "DeleteFileUnreadable",
    };
    tracing::warn!(
        homeurl = %homeurl,
        kind,
        detail = ?warning,
        "[areka_update] warning"
    );
}

/// 片付けられなかった残骸 1 件（4.8・8.2）。
fn warn_leftover(homeurl: &str, path: &Path) {
    tracing::warn!(
        homeurl = %homeurl,
        kind = "leftover",
        path = %path.display(),
        "[areka_update] work area left behind"
    );
}

/// 成功の終了（8.3）。
fn log_finished(
    homeurl: &str,
    target: &Path,
    outcome: &str,
    placed: usize,
    removed: usize,
    leftovers: usize,
) {
    tracing::info!(
        homeurl = %homeurl,
        target = %target.display(),
        outcome,
        placed,
        removed,
        leftovers,
        "[areka_update] update finished"
    );
}

/// 失敗の記録を 1 回だけ出し、受け取った失敗をそのまま返す（8.1）。
/// 失敗の終了はこの 1 行が兼ね、`info!` を重ねない（8.3）。
fn log_failure(err: UpdateError) -> UpdateError {
    tracing::error!(
        homeurl = %err.homeurl,
        target = %err.target.display(),
        stage = ?err.stage,
        reason = err.reason.kind(),
        detail = %err.reason,
        file = err.file().unwrap_or_default(),
        rolled_back = err.rolled_back(),
        work = %err.work.as_deref().map(Path::display).map(|d| d.to_string()).unwrap_or_default(),
        leftovers = err.leftovers.len(),
        "[areka_update] update failed"
    );
    err
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod run_tests;
