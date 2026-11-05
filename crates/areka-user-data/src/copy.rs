//! フォルダの木を半端なく写す手続き（spec: areka-P0-user-data-root 要件 4.6・4.7）。
//!
//! 書ける根の下の作業の棚（[`crate::COPY_WORK`]）で木を組み、`rename` 1 手で行き先へ入れ替える。
//! 途中で失敗したら作業フォルダを消し、行き先は作らない。元は読むだけ。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// 写した量（記録 `user_data_copied` の値）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyStats {
    pub files: u64,
    pub bytes: u64,
    pub millis: u64,
}

/// 写せなかった理由。
#[derive(Debug)]
pub enum CopyError {
    /// 読み・書き・作業フォルダの作成の失敗（どのパスか・理由）。
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    /// 木の中にリンク（シンボリックリンク・ジャンクション）が在った（辿らない）。
    Link { path: PathBuf },
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopyError::Io { path, error } => write!(f, "io: {}: {error}", path.display()),
            CopyError::Link { path } => write!(f, "link: {}", path.display()),
        }
    }
}

impl std::error::Error for CopyError {}

/// 写しの結果。
#[derive(Debug, PartialEq, Eq)]
pub enum CopyDone {
    Copied(CopyStats),
    /// 入れ替えの時に `to` がすでに在った（別のプロセスが先に写した）。自分の作業は捨てた。
    Raced,
}

/// 作業フォルダの名前の連番（同じプロセスの中で重ねない）。
static NEXT_WORK: AtomicU64 = AtomicU64::new(0);

/// `from` の木を `<work_root>\.copy-work\<pid>-<連番>\` に組み、`to` へ `rename` する。
/// `to` の親は作る。`to` がすでに在れば書かない（上書きしない）。
/// 失敗したら作業フォルダを消し（消せなければ warn `user_data_copy_work_left`）、`to` は作らない。
pub fn copy_tree_atomic(from: &Path, to: &Path, work_root: &Path) -> Result<CopyDone, CopyError> {
    refuse_inside(from, to)?;
    refuse_inside(from, work_root)?;
    if to.exists() {
        return Ok(CopyDone::Raced);
    }
    let started = Instant::now();
    let shelf = work_root.join(crate::COPY_WORK);
    io(&shelf, std::fs::create_dir_all(&shelf))?;
    sweep_stale(&shelf, SystemTime::now());
    let work = make_work_dir(&shelf, || NEXT_WORK.fetch_add(1, Ordering::Relaxed))?;
    let mut stats = CopyStats {
        files: 0,
        bytes: 0,
        millis: 0,
    };
    let built = build(from, &work, &mut stats).and_then(|()| match to.parent() {
        Some(parent) => io(parent, std::fs::create_dir_all(parent)),
        None => Ok(()),
    });
    if let Err(error) = built {
        discard(&work);
        return Err(error);
    }
    let renamed = std::fs::rename(&work, to);
    stats.millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    finish_rename(&work, to, renamed, stats)
}

/// 入れ替えの結果を写しの結果にする。失敗なら作業フォルダを捨てる。
fn finish_rename(
    work: &Path,
    to: &Path,
    result: std::io::Result<()>,
    stats: CopyStats,
) -> Result<CopyDone, CopyError> {
    let Err(error) = result else {
        return Ok(CopyDone::Copied(stats));
    };
    discard(work);
    // ponytail: 「在る」は rename の後に見る。Windows は在る行き先へのフォルダの rename を
    // AccessDenied などで拒み、エラーの種類だけでは先を越されたと見分けられないため。
    if to.exists() {
        Ok(CopyDone::Raced)
    } else {
        Err(CopyError::Io {
            path: to.to_path_buf(),
            error,
        })
    }
}

/// `path` が `from` の下（同じを含む）なら入力の誤りとして断る（写す元の中へ写すと止まらない）。
fn refuse_inside(from: &Path, path: &Path) -> Result<(), CopyError> {
    let outer = io(from, std::path::absolute(from))?;
    let inner = io(path, std::path::absolute(path))?;
    if is_within(&inner, &outer) {
        return Err(CopyError::Io {
            path: path.to_path_buf(),
            error: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "will not copy into the source tree",
            ),
        });
    }
    Ok(())
}

/// `outer` が `inner` の前置きか（部品ごとに大文字小文字を区別せずに比べる・Windows の名前の決まり）。
/// 1 字を 1 字の大文字へ畳む（NTFS と同じ・`areka-ghost` の `catalog_roots::fold_key` と同じ規則）。
/// `to_lowercase` は語末の Σ を ς にし σ と分けてしまう。
fn is_within(inner: &Path, outer: &Path) -> bool {
    let fold = |c: std::path::Component<'_>| -> String {
        let name = c.as_os_str().to_string_lossy();
        name.chars()
            .map(|c| {
                let mut up = c.to_uppercase();
                match (up.next(), up.next()) {
                    (Some(u), None) => u,
                    _ => c,
                }
            })
            .collect()
    };
    let (mut inner, outer) = (inner.components(), outer.components());
    outer
        .into_iter()
        .all(|o| inner.next().is_some_and(|i| fold(i) == fold(o)))
}

/// 棚の直下にこれより古い項目は、落ちたプロセスの残した作業フォルダとして片付ける。
const STALE_WORK_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// 棚の直下で更新時刻が `now` から [`STALE_WORK_AFTER`] 以上前の項目を消す。
/// 読めない項目・時刻の取れない項目は触らない。消せなければ warn を残して続ける。
fn sweep_stale(shelf: &Path, now: SystemTime) {
    let Ok(entries) = std::fs::read_dir(shelf) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if now
            .duration_since(modified)
            .is_ok_and(|age| age >= STALE_WORK_AFTER)
        {
            discard(&entry.path());
        }
    }
}

/// `<shelf>\<pid>-<連番>` を作る。番地が先に在れば（落ちたプロセスの残り）次の番号で取り直す。
fn make_work_dir(shelf: &Path, mut serial: impl FnMut() -> u64) -> Result<PathBuf, CopyError> {
    loop {
        let work = shelf.join(format!("{}-{}", std::process::id(), serial()));
        match std::fs::create_dir(&work) {
            Ok(()) => return Ok(work),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(CopyError::Io { path: work, error }),
        }
    }
}

/// `from` の中身を `to`（在るフォルダ）へ写す。リンクは辿らずに失敗にする。
/// 深い木でスタックを使い切らないよう、再帰でなく積み上げで歩く。
fn build(from: &Path, to: &Path, stats: &mut CopyStats) -> Result<(), CopyError> {
    let mut pending = vec![(from.to_path_buf(), to.to_path_buf())];
    while let Some((from, to)) = pending.pop() {
        for child in io(&from, std::fs::read_dir(&from))? {
            let child = io(&from, child)?;
            let source = child.path();
            let target = to.join(child.file_name());
            let kind = io(&source, child.file_type())?;
            // Windows の std はジャンクション（名前の代理の再解析点）も `is_symlink` に数える。
            if kind.is_symlink() {
                return Err(CopyError::Link { path: source });
            }
            if kind.is_dir() {
                io(&target, std::fs::create_dir(&target))?;
                pending.push((source, target));
            } else {
                stats.bytes += io(&source, std::fs::copy(&source, &target))?;
                stats.files += 1;
                // 複写は読み取り専用の属性も運ぶ。写しへ後で書けるように落とす（`areka-nar` の
                // `copy_existing` と同じ・Windows 専用なので落ちるのは読み取り専用の属性 1 つ）。
                let mut mode = io(&target, std::fs::metadata(&target))?.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                mode.set_readonly(false);
                io(&target, std::fs::set_permissions(&target, mode))?;
            }
        }
    }
    Ok(())
}

/// 作業フォルダ（棚の直下の片付けではファイルも）を消す。消せなければ warn を残して続ける。
fn discard(work: &Path) {
    let removed = if work.is_dir() {
        std::fs::remove_dir_all(work)
    } else {
        std::fs::remove_file(work)
    };
    if let Err(error) = removed
        && work.exists()
    {
        tracing::warn!(
            event = "user_data_copy_work_left",
            dir = %work.display(),
            error = %error,
            "copy work folder could not be removed"
        );
    }
}

fn io<T>(path: &Path, result: std::io::Result<T>) -> Result<T, CopyError> {
    result.map_err(|error| CopyError::Io {
        path: path.to_path_buf(),
        error,
    })
}

#[cfg(test)]
#[path = "copy_tests.rs"]
mod tests;
