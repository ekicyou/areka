//! `\![execute,install,url]` の取得の兄弟テスト（design「Testing Strategy / URL の取得」・要件 6.1・
//! 6.4・6.6・9.11）。偽の取得口だけを差し、ネットワークへ出ない。一時フォルダは `temp-path-kit`。
//! 通信中の数はテストごとの `static` を渡し、プロセスの数（`PROCESS`）には触れない。

use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use areka_kanade::online::OnlineCounter;
use areka_update::FetchError;
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use tracing::Level;

use super::{
    DownloadError, MakeFetch, download, download_dir, fetch_and_send, spawn_download_with,
};
use crate::install::fetch_url_test_support::FakeFetch;
use crate::install::{InstallOrigin, RawInstallRequest};

const URL: &str = "https://example.com/files/my ghost.nar";
const BODY: &[u8] = b"PK\x03\x04 nar body";
const DAY: Duration = Duration::from_secs(24 * 60 * 60);
/// スレッドの終わりを待つ上限（届けば即座に返る）。
const BOUND: Duration = Duration::from_secs(30);

/// 数を読まないテストへ渡す、誰も読まない通信中の数。
static UNOBSERVED: OnlineCounter = OnlineCounter::new();

fn make(fake: FakeFetch) -> MakeFetch {
    Box::new(move || Ok(Box::new(fake) as Box<dyn areka_update::Fetch>))
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|e| {
                    e.expect("一覧が読める")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// `<pid>-<連番>-<名前>` の形か。
fn assert_name_shape(path: &Path, tail: &str) {
    let name = path.file_name().expect("ファイル名").to_string_lossy();
    let prefix = format!("{}-", std::process::id());
    let serial = name
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix(&format!("-{tail}")))
        .unwrap_or_else(|| panic!("名前の形が違う: {name}"));
    assert!(
        !serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit()),
        "連番が数字でない: {name}"
    );
}

fn set_mtime(path: &Path, at: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_modified(at))
        .expect("更新時刻を据えられる");
}

fn join_bounded(handle: JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(handle.join().is_ok());
    });
    assert_eq!(
        rx.recv_timeout(BOUND),
        Ok(true),
        "install-fetch が終わらないか panic した"
    );
}

#[test]
fn download_dir_is_areka_download_under_os_temp() {
    // OS の一時フォルダの入口はテストから叩かない（temp-path-kit の見張り）＝末尾の形だけを見る。
    let dir = download_dir();
    assert!(dir.is_absolute(), "{}", dir.display());
    assert!(
        dir.ends_with(Path::new("areka").join("download")),
        "{}",
        dir.display()
    );
}

#[test]
fn download_writes_body_under_pid_serial_safe_name() {
    let tmp = TempPath::new("fetch-url-name");
    let dir = tmp.child("download");
    let fake = FakeFetch::new()
        .serve(URL, BODY)
        .serve("https://example.com/dl/", b"empty tail");

    let path = download(URL, &fake, &dir, SystemTime::now()).expect("落とせる");
    assert_eq!(path.parent(), Some(dir.as_path()));
    assert_name_shape(&path, "my_ghost.nar");
    assert_eq!(fs::read(&path).expect("読める"), BODY);

    // 区切りの後ろが空なら download.nar。
    let empty =
        download("https://example.com/dl/", &fake, &dir, SystemTime::now()).expect("落とせる");
    assert_name_shape(&empty, "download.nar");
    assert_ne!(path, empty, "連番で別の名前になる");
}

#[test]
fn fetch_failure_returns_err_and_writes_nothing() {
    for err in [FetchError::Status { code: 500 }, FetchError::Timeout] {
        let tmp = TempPath::new("fetch-url-fail");
        let dir = tmp.child("download");
        let fake = FakeFetch::new().fail(URL, err.clone());

        let result = download(URL, &fake, &dir, SystemTime::now());
        assert!(
            matches!(&result, Err(DownloadError::Fetch(e)) if *e == err),
            "{err:?} で取得の失敗になる: {result:?}"
        );
        assert_eq!(files_in(&dir), Vec::<String>::new(), "{err:?} でファイル 0");
    }
}

#[test]
fn files_older_than_seven_days_are_swept_before_download() {
    let tmp = TempPath::new("fetch-url-sweep");
    let dir = tmp.child("download");
    fs::create_dir_all(&dir).expect("フォルダを作れる");
    let t0 = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let old = dir.join("old.nar");
    let young = dir.join("young.nar");
    fs::write(&old, b"old").expect("書ける");
    fs::write(&young, b"young").expect("書ける");
    set_mtime(&old, t0);
    set_mtime(&young, t0 + 2 * DAY);

    // `now` を 8 日進める: old は 8 日前、young は 6 日前。
    let fake = FakeFetch::new().serve(URL, BODY);
    let path = download(URL, &fake, &dir, t0 + 8 * DAY).expect("落とせる");

    assert!(!old.exists(), "7 日より古い物は消える");
    assert!(young.exists(), "7 日未満は残る");
    assert!(path.exists(), "今落とした物は残る");
    assert_eq!(files_in(&dir).len(), 2);
}

#[test]
fn thread_round_sends_one_script_request() {
    let tmp = TempPath::new("fetch-url-thread");
    let dir = tmp.child("download");
    let (tx, rx) = mpsc::channel::<RawInstallRequest>();

    let handle = spawn_download_with(
        URL.to_owned(),
        tx,
        dir.clone(),
        make(FakeFetch::new().serve(URL, BODY)),
        &UNOBSERVED,
    )
    .expect("install-fetch が起きる");
    let request = rx.recv_timeout(BOUND).expect("依頼が届く");
    join_bounded(handle);

    assert_eq!(request.origin, InstallOrigin::Script);
    assert_eq!(request.path.parent(), Some(dir.as_path()));
    assert_eq!(fs::read(&request.path).expect("読める"), BODY);
    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Disconnected),
        "依頼は 1 件だけ"
    );
}

#[test]
fn thread_round_failure_sends_nothing() {
    let tmp = TempPath::new("fetch-url-thread-fail");
    let dir = tmp.child("download");
    let (tx, rx) = mpsc::channel::<RawInstallRequest>();

    let handle = spawn_download_with(
        URL.to_owned(),
        tx,
        dir.clone(),
        make(FakeFetch::new().fail(URL, FetchError::Status { code: 500 })),
        &UNOBSERVED,
    )
    .expect("install-fetch が起きる");
    join_bounded(handle);

    assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected), "依頼 0");
    assert_eq!(files_in(&dir), Vec::<String>::new());
}

/// 取得の失敗・取得口を作れない、のどちらも `error!(install_fetch_failed)` 1 件（URL と理由）で
/// 依頼 0（要件 6.4・10.15）。
#[test]
fn failure_logs_one_error_with_url_and_reason_and_sends_nothing() {
    let unavailable: MakeFetch = Box::new(|| Err(FetchError::Other { code: 12007 }));
    let cases = [
        (
            "fetch",
            make(FakeFetch::new().fail(URL, FetchError::Timeout)),
        ),
        ("unavailable", unavailable),
    ];
    for (label, maker) in cases {
        let tmp = TempPath::new("fetch-url-log");
        let dir = tmp.child("download");
        let (tx, rx) = mpsc::channel::<RawInstallRequest>();

        let ((), events) =
            capture(|| fetch_and_send(URL, maker, &dir, SystemTime::now(), &tx, &UNOBSERVED));

        let errors: Vec<_> = events.iter().filter(|e| e.level == Level::ERROR).collect();
        let [error] = errors.as_slice() else {
            panic!("{label}: error! はちょうど 1 件: {errors:?}");
        };
        assert_eq!(
            error.field_str("event"),
            Some("install_fetch_failed"),
            "{label}"
        );
        assert_eq!(error.field_str("url"), Some(URL), "{label}");
        assert!(
            error.field("reason").is_some_and(|r| !r.is_empty()),
            "{label}: 理由が載る: {error:?}"
        );
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "{label}: 依頼 0");
    }
}

/// 成功は `install_fetch_done` の上で送る（警告・失敗 0）。
#[test]
fn success_logs_done_without_warnings() {
    let tmp = TempPath::new("fetch-url-done");
    let dir = tmp.child("download");
    let (tx, rx) = mpsc::channel::<RawInstallRequest>();

    let ((), events) = capture(|| {
        fetch_and_send(
            URL,
            make(FakeFetch::new().serve(URL, BODY)),
            &dir,
            SystemTime::now(),
            &tx,
            &UNOBSERVED,
        )
    });

    assert!(events.iter().all(|e| e.level > Level::WARN), "{events:?}");
    assert!(
        events
            .iter()
            .any(|e| e.field_str("event") == Some("install_fetch_done")),
        "{events:?}"
    );
    assert_eq!(rx.try_recv().map(|r| r.origin), Ok(InstallOrigin::Script));
}

/// 取得口を作る時点で通信中の数は立っていて、取得口を作れない・取得の失敗・書けない・送れない・
/// 成功、のどの経路で抜けても 0 に戻る（要件 2.2・2.3）。経路を踏んだことは記録で確かめる。
#[test]
fn online_is_raised_during_the_fetch_and_restored_on_every_exit() {
    static ONLINE: OnlineCounter = OnlineCounter::new();
    let seen = Arc::new(AtomicBool::new(false));
    // 取得口を作る閉包の中で数を読む（関数の冒頭で立っていれば真）。
    let probe = |fake: Option<FakeFetch>| -> MakeFetch {
        let seen = Arc::clone(&seen);
        Box::new(move || {
            seen.store(ONLINE.is_online(), Ordering::SeqCst);
            match fake {
                Some(fake) => Ok(Box::new(fake) as Box<dyn areka_update::Fetch>),
                None => Err(FetchError::Other { code: 12007 }),
            }
        })
    };
    let served = || Some(FakeFetch::new().serve(URL, BODY));
    let cases = [
        ("unavailable", probe(None), false, "install_fetch_failed"),
        (
            "fetch",
            probe(Some(FakeFetch::new().fail(URL, FetchError::Timeout))),
            false,
            "install_fetch_failed",
        ),
        ("unwritable", probe(served()), false, "install_fetch_failed"),
        (
            "unsendable",
            probe(served()),
            false,
            "install_fetch_send_failed",
        ),
        ("success", probe(served()), true, "install_fetch_done"),
    ];
    for (label, maker, success, expected) in cases {
        let tmp = TempPath::new("fetch-url-online");
        let dir = tmp.child("download");
        if label == "unwritable" {
            // フォルダの場所にファイルを置く＝フォルダを作れない。
            fs::write(&dir, b"not a dir").expect("書ける");
        }
        let (tx, rx) = mpsc::channel::<RawInstallRequest>();
        // 成功の他は受信端を落とす（送るのは成功と「送れない」だけ＝送れない経路になる）。
        let rx = success.then_some(rx);
        seen.store(false, Ordering::SeqCst);

        let ((), events) =
            capture(|| fetch_and_send(URL, maker, &dir, SystemTime::now(), &tx, &ONLINE));

        assert!(
            seen.load(Ordering::SeqCst),
            "{label}: 取得口を作る時点で数が立っている"
        );
        assert!(!ONLINE.is_online(), "{label}: 抜けたら数は 0");
        assert!(
            events
                .iter()
                .any(|e| e.field_str("event") == Some(expected)),
            "{label}: {expected} の経路を踏む: {events:?}"
        );
        if let Some(rx) = rx {
            assert_eq!(
                rx.try_recv().map(|r| r.origin),
                Ok(InstallOrigin::Script),
                "{label}"
            );
        }
    }
}
