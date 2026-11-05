//! `src` の単体テストが分け合う道具（有界の実行・システム変数・記録の sink・空の sink・
//! ゴーストの fixture・有界の終了・OnBoot を待つ輪）。各テストはここから引き、写しを持たない。
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{self, ShioriMount};
use areka_sakura::contract::{CueSink, SystemVarSnapshot, TalkCue};
use temp_path_kit::TempPath;

use crate::runtime::{
    GhostBootOptions, GhostRuntime, ShioriWiring, SystemVarSource, SystemVarWiring, TickerMode,
};

/// 退役した `default_system_vars()` の代役: `{"username": DEFAULT_USERNAME}` だけの写しを
/// 毎回新しく作って返す（既存テストは既定の username を前提に直接注入する）。
pub(crate) fn test_system_vars() -> SystemVarSource {
    Box::new(|| {
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", areka_sakura::sysvar::DEFAULT_USERNAME);
        snapshot
    })
}

/// 解決できる `root` を空の SHIORI・捨てる sink 2 本・刻みなしで起こす設定。
pub(crate) fn fake_options(root: PathBuf) -> GhostBootOptions {
    GhostBootOptions {
        strict_judges: None,
        ghost_root: root,
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(crate::shiori_wiring::empty_connect())),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    }
}

/// 別スレッドで `f` を走らせ、期限内に終わらなければテストを失敗させる（宙吊りで
/// テストの実行ファイルごと止めない）。`f` の値を返す。`f` の中の panic は「止まった」と取り違えない。
pub(crate) fn run_bounded<T: Send + 'static>(
    what: &str,
    timeout: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (done_tx, done_rx) = sync_channel::<T>(1);
    thread::spawn(move || {
        let _ = done_tx.send(f());
    });
    match done_rx.recv_timeout(timeout) {
        Ok(value) => value,
        Err(RecvTimeoutError::Disconnected) => panic!("'{what}' panicked"),
        Err(RecvTimeoutError::Timeout) => {
            panic!("'{what}' did not complete within {timeout:?} (possible hang)")
        }
    }
}

/// 正規の終了（`CloseReason::System`）を 10 秒の上限つきで待ち、`Ok` を確かめる。
pub(crate) fn shutdown_bounded(runtime: GhostRuntime) {
    run_bounded("shutdown", Duration::from_secs(10), move || {
        let result = runtime.shutdown(areka_kanade::CloseReason::System);
        assert!(result.is_ok(), "shutdown should succeed, got {result:?}");
    });
}

/// `Clone` できる記録の sink（dispatcher の per-talk 注入 `S: Clone` を満たす）。全 cue を
/// 1 つの列へためる（surface／text の両スロットが同じ全 cue を受ける）。
#[derive(Clone)]
pub(crate) struct RecordingSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    pub(crate) fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("records mutex poisoned")
            .push(cue);
    }
}

/// 何もしない `Clone` できる sink（発火の中身を見ないテスト用）。
#[derive(Clone)]
pub(crate) struct NoopSink;

impl CueSink for NoopSink {
    fn emit(&mut self, _cue: TalkCue) {}
}

/// `ghost/master/descript.txt` を `descript`（行の並び）で書き、本番と同じ経路
/// （`package::resolve`・`ShioriMount` は `#[non_exhaustive]`）で `ShioriMount` を得る。
/// 返り値の [`TempPath`] をテストの間だけ生かしておくこと（破棄で中身ごと消える）。
pub(crate) fn shiori_mount(tag: &str, descript: &str) -> (TempPath, ShioriMount) {
    let temp = TempPath::new(tag);
    let root = temp.path().to_path_buf();
    let master_dir = root.join("ghost").join("master");
    std::fs::create_dir_all(&master_dir).expect("fixture master dir 作成に失敗");
    std::fs::create_dir_all(root.join("shell").join("master"))
        .expect("fixture shell dir 作成に失敗");
    std::fs::write(master_dir.join("descript.txt"), descript)
        .expect("fixture descript.txt 書き込みに失敗");

    let mount = package::resolve(&root, DefaultEncoding::Ansi)
        .expect("fixture ghost_root の resolve に失敗");
    (temp, mount.shiori)
}

/// `root` の下に解決できる最小のゴースト（`ghost/master`＝`TestGhost`・`shell/master`＝
/// `TestShell`）を作る。シェル名とシェルのフォルダ名（`master`）は別の値。
pub(crate) fn write_ghost(root: &Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");
    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");
}

/// 偽の SHIORI が受けた GET の列（`(id, references)`）。
pub(crate) type Requests = Arc<Mutex<Vec<(String, Vec<String>)>>>;

/// `OnBoot` が SHIORI へ届くまで（上限 10 秒）待ち、その時点の GET の列を返す。
pub(crate) fn wait_for_on_boot(requests: &Requests) -> Vec<(String, Vec<String>)> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let seen = requests.lock().expect("requests mutex poisoned").clone();
        if seen.iter().any(|(id, _)| id == "OnBoot") {
            return seen;
        }
        assert!(
            Instant::now() < deadline,
            "OnBoot never reached SHIORI; requests so far: {seen:?}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
