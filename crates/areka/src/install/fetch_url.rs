//! `\![execute,install,url]` の取得（design「`\![execute,install,url]`」）。
//!
//! URL の本文を areka 専用の一時フォルダ（`%TEMP%\areka\download\`）へ落とし、落とし終えたら
//! 出どころ「台本」の生のインストールの要求を窓口へ送る。取得は短命のスレッド `install-fetch`
//! で行い、門は持たない（終了で待たない＝要件 7.5・落としかけの物は次の取得の 7 日の掃除に任せる）。
//! 使うのは `areka_update::{Fetch, WinHttpFetch, FetchError}` と `install::{RawInstallRequest,
//! InstallOrigin}` だけ。
// 呼び手（`emo2_boot/install_cue.rs` の `url` の腕）は 4.2 で結ぶ。そこでこの行を外す。
#![allow(dead_code)]

use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use areka_update::{Fetch, FetchError, WinHttpFetch};

use crate::install::{InstallOrigin, RawInstallRequest};

/// 取得口の作り方（本番は `WinHttpFetch::new`・テストは偽の取得口）。取得口はスレッドの中で作る。
pub(crate) type MakeFetch = Box<dyn FnOnce() -> Result<Box<dyn Fetch>, FetchError> + Send>;

/// これより古いファイルを次の取得のときに消す（要件 6.6）。
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// 区切りの後ろが空だったときの名前。
const FALLBACK_NAME: &str = "download.nar";

/// プロセスの中で落とした順の連番（名前の衝突を避ける）。
static SERIAL: AtomicU32 = AtomicU32::new(0);

/// 落とすことの失敗。
#[derive(Debug, thiserror::Error)]
pub(crate) enum DownloadError {
    #[error("取得に失敗: {0}")]
    Fetch(#[from] FetchError),
    #[error("書けない: {path} ({source})")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// areka 専用の一時フォルダ（`%TEMP%\areka\download\`）。
pub(crate) fn download_dir() -> PathBuf {
    std::env::temp_dir().join("areka").join("download")
}

/// 7 日より古いファイルを消し、`url` の本文を `<pid>-<連番>-<名前>` に書く。名前は URL の末尾の
/// 区切り以降から `[A-Za-z0-9._-]` 以外を `_` にし、空なら `download.nar`。取得の失敗はファイル 0。
pub(crate) fn download(
    url: &str,
    fetch: &dyn Fetch,
    dir: &Path,
    now: SystemTime,
) -> Result<PathBuf, DownloadError> {
    fs::create_dir_all(dir).map_err(|source| DownloadError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    sweep_old(dir, now);
    let bytes = fetch.get(url)?;
    let path = dir.join(file_name(url));
    fs::write(&path, bytes).map_err(|source| DownloadError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// `<pid>-<連番>-<URL の末尾の安全な名前>`。
fn file_name(url: &str) -> String {
    let tail = url.rsplit('/').next().unwrap_or_default();
    let safe: String = tail
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        FALLBACK_NAME
    } else {
        &safe
    };
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    format!("{}-{serial}-{safe}", std::process::id())
}

/// `dir` の直下で `modified` が `now - 7 日` より古いファイルを消し、消した数を返す。
/// 消せなければ `debug!` で飛ばす（次の取得でまた試す）。
fn sweep_old(dir: &Path, now: SystemTime) -> usize {
    let Some(limit) = now.checked_sub(KEEP) else {
        return 0;
    };
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            tracing::debug!(
                event = "install_fetch_sweep_skipped",
                path = %dir.display(),
                error = %err,
                "[install] 一時フォルダの一覧を読めないので、古いファイルの掃除を飛ばします"
            );
            return 0;
        }
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .ok()
            .filter(fs::Metadata::is_file)
            .and_then(|m| m.modified().ok())
            .is_some_and(|modified| modified < limit);
        if !old {
            continue;
        }
        let path = entry.path();
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(err) => tracing::debug!(
                event = "install_fetch_sweep_skipped",
                path = %path.display(),
                error = %err,
                "[install] 古い一時ファイルを消せないので、次の取得に任せます"
            ),
        }
    }
    tracing::info!(
        event = "install_fetch_swept",
        dir = %dir.display(),
        count = removed,
        "[install] 7 日より古い一時ファイルを消しました"
    );
    removed
}

/// 短命のスレッド `install-fetch` を起こし、落とし終えたら `RawInstallRequest { origin: Script }` を送る。
/// スレッドは待たない（終了でも待たない）。
pub(crate) fn spawn_download(url: String, tx: Sender<RawInstallRequest>) {
    let make: MakeFetch = Box::new(|| Ok(Box::new(WinHttpFetch::new()?) as Box<dyn Fetch>));
    // 手放す＝待たない（要件 7.5）。
    drop(spawn_download_with(url, tx, download_dir(), make));
}

/// [`spawn_download`] の一時フォルダと取得口の作り方を差し替える口。起こせなければ
/// `error!(install_fetch_failed)` で `None`。
pub(crate) fn spawn_download_with(
    url: String,
    tx: Sender<RawInstallRequest>,
    dir: PathBuf,
    make: MakeFetch,
) -> Option<JoinHandle<()>> {
    let spawned = thread::Builder::new()
        .name("install-fetch".to_owned())
        .spawn({
            let url = url.clone();
            move || fetch_and_send(&url, make, &dir, SystemTime::now(), &tx)
        });
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            log_failed(&url, &format!("取得のスレッドを起こせない: {err}"));
            None
        }
    }
}

/// スレッドの中身（取得口を作る → 落とす → 送る）。失敗は `error!` 1 件で依頼 0。
pub(crate) fn fetch_and_send(
    url: &str,
    make: MakeFetch,
    dir: &Path,
    now: SystemTime,
    tx: &Sender<RawInstallRequest>,
) {
    tracing::info!(
        event = "install_fetch_begin",
        url,
        dir = %dir.display(),
        "[install] URL の取得を始めます"
    );
    let fetch = match make() {
        Ok(fetch) => fetch,
        Err(err) => return log_failed(url, &format!("取得口を作れない: {err}")),
    };
    let path = match download(url, fetch.as_ref(), dir, now) {
        Ok(path) => path,
        Err(err) => return log_failed(url, &err),
    };
    tracing::info!(
        event = "install_fetch_done",
        url,
        path = %path.display(),
        "[install] URL を落としました。インストールへ渡します"
    );
    let request = RawInstallRequest {
        path,
        origin: InstallOrigin::Script,
    };
    if let Err(err) = tx.send(request) {
        tracing::warn!(
            event = "install_fetch_send_failed",
            url,
            error = %err,
            "[install] 窓口の受信端が無いので、落とした書庫をインストールへ渡せません"
        );
    }
}

/// 取得の失敗 1 件（URL と理由・要件 6.4・10.15）。
fn log_failed(url: &str, reason: &dyn Display) {
    tracing::error!(
        event = "install_fetch_failed",
        url,
        reason = %reason,
        "[install] URL を落とせません。インストールしません"
    );
}

#[cfg(test)]
#[path = "fetch_url_tests.rs"]
mod tests;
