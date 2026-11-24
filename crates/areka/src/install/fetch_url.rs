//! `\![execute,install,url]` の取得。
//!
//! URL の本文を areka 専用の一時フォルダ（`%TEMP%\areka\download\`）へ落とし、落とし終えたら
//! 出どころ「台本」の生のインストールの要求を窓口へ送る。取得は窓口が最初の URL で起こす長く生きる
//! スレッド `install-fetch` が 1 件ずつ行い、門 `install-fetch` に URL を置く（終了では待たずに
//! 記録だけ残す・落としかけの物は次の取得の 7 日の掃除に任せる）。

use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, SystemTime};

use areka_kanade::online::OnlineCounter;
use areka_session::exit_wait::WorkGate;
use areka_update::{Fetch, FetchError, WinHttpFetch};

use crate::install::worker::DeskAsk;
use crate::install::{InstallOrigin, RawInstallRequest};

/// 取得口の作り口（本番は `WinHttpFetch::new`・テストは偽の取得口）。取得のスレッドが URL ごとに呼ぶ。
pub(crate) type MakeFetch = Arc<dyn Fn() -> Result<Box<dyn Fetch>, FetchError> + Send + Sync>;

/// 取得のスレッドの道具（落とす先・取得口の作り口・通信中の数）。
#[derive(Clone)]
pub(crate) struct FetchTools {
    pub dir: PathBuf,
    pub make: MakeFetch,
    pub online: &'static OnlineCounter,
}

impl FetchTools {
    /// 本番の道具（`download_dir`・`WinHttpFetch`・プロセスの通信中の数）。
    pub(crate) fn real() -> FetchTools {
        FetchTools {
            dir: download_dir(),
            make: Arc::new(|| Ok(Box::new(WinHttpFetch::new()?) as Box<dyn Fetch>)),
            online: &areka_kanade::online::PROCESS,
        }
    }
}

/// これより古いファイルを次の取得のときに消す。
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

/// 取得のスレッド `install-fetch` を起こして `first` から取りに行かせ、次の URL の送出端を返す。
/// 送出端が全部落ちるとスレッドは今の取得を終えて抜ける（join はしない）。起こせなければ
/// `error!(install_fetch_failed)` で `None`。
pub(crate) fn spawn_fetcher(
    first: String,
    tools: FetchTools,
    gate: Arc<WorkGate>,
    raw: Sender<RawInstallRequest>,
    desk: Sender<DeskAsk>,
) -> Option<Sender<String>> {
    let (urls_tx, urls) = mpsc::channel::<String>();
    let label = first.clone();
    // 起こした側の記録の受け手を引き継ぐ（本番では全体の既定と同じ・テストでは捕捉の窓）。
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let spawned = thread::Builder::new()
        .name("install-fetch".to_owned())
        .spawn(move || {
            let _dispatch = tracing::dispatcher::set_default(&dispatch);
            let mut next = Some(first);
            while let Some(url) = next.take().or_else(|| urls.recv().ok()) {
                let _guard = FetchGuard {
                    url: url.clone(),
                    gate: gate.clone(),
                    desk: desk.clone(),
                };
                if !gate.begin(url.clone()) {
                    tracing::warn!(
                        event = "install_fetch_closing",
                        url,
                        "[install] 終了が始まっているので、URL を取りに行きません"
                    );
                    return;
                }
                fetch_and_send(
                    &url,
                    &tools.make,
                    &tools.dir,
                    SystemTime::now(),
                    &raw,
                    tools.online,
                );
            }
        });
    match spawned {
        Ok(_) => Some(urls_tx),
        Err(err) => {
            log_failed(&label, &format!("取得のスレッドを起こせない: {err}"));
            None
        }
    }
}

/// URL 1 件の守り手。どの道で落ちても門を手放して窓口へ取得の終わりを知らせ、倒れたときは先に
/// `error!(install_fetch_failed)` を残す。
struct FetchGuard {
    url: String,
    gate: Arc<WorkGate>,
    desk: Sender<DeskAsk>,
}

impl Drop for FetchGuard {
    fn drop(&mut self) {
        let panicked = thread::panicking();
        if panicked {
            log_failed(
                &self.url,
                &"取得のスレッドが倒れた（次の URL で起こし直す）",
            );
        }
        self.gate.end();
        if self.desk.send(DeskAsk::FetchDone { panicked }).is_err() {
            tracing::debug!(
                event = "install_desk_gone",
                "[install] 窓口が居ないので、取得の終わりを知らせられません"
            );
        }
    }
}

/// スレッドの中身（取得口を作る → 落とす → 送る）。失敗は `error!` 1 件で依頼 0。
/// 関数の間は `online` を立て、どの経路で抜けても戻す。
pub(crate) fn fetch_and_send(
    url: &str,
    make: &MakeFetch,
    dir: &Path,
    now: SystemTime,
    tx: &Sender<RawInstallRequest>,
    online: &'static OnlineCounter,
) {
    let _online = online.begin("install-fetch");
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
    let request = RawInstallRequest::Path {
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

/// 取得の失敗 1 件（URL と理由）。
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
