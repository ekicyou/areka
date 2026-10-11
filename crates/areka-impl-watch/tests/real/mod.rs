//! 実機テスト（`tests/real.rs`）の支え: 一時の置き場所 [`Desk`]・立てた子プロセス [`Running`]・
//! 終わった子の姿 [`Done`]・状態ファイルの読み取り。決まり（置き場所は `target\test-roots\` の
//! 下・子は自分の分だけ止める・待ちは上限つき）は `real.rs` の先頭に書いてある。

use std::cell::Cell;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use temp_path_kit::TempPath;

const EXE: &str = env!("CARGO_BIN_EXE_areka-impl-watch");
pub const HOME_ENV: &str = "AREKA_IMPL_WATCH_HOME";
/// 終わるはずの子プロセス・起きるはずの変化を待つ上限。本物の読み直しは 1 秒ごとなので、
/// ふだんは 1〜2 秒で届く。
const LIMIT: Duration = Duration::from_secs(15);
/// 待ちの間の見直しの間隔。
const STEP: Duration = Duration::from_millis(20);

/// `attempt` が値を返すまで、短い間隔で上限まで試す。
fn retry<T>(what: &str, mut attempt: impl FnMut() -> Option<T>) -> T {
    let started = Instant::now();
    loop {
        if let Some(found) = attempt() {
            return found;
        }
        let waited = started.elapsed();
        assert!(waited < LIMIT, "{LIMIT:?} 待っても届かない: {what}");
        std::thread::sleep(STEP);
    }
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{} を読めない: {err}", path.display()))
}

/// 終わった子プロセスの終了コードと、標準出力・標準エラーの全部。
#[derive(Debug, PartialEq, Eq)]
pub struct Done {
    pub code: i32,
    pub out: String,
    pub err: String,
}

pub fn done(code: i32, out: &str, err: &str) -> Done {
    let (out, err) = (out.to_owned(), err.to_owned());
    Done { code, out, err }
}

/// できた（0）。標準出力に `out` だけ。
pub fn ok(out: &str) -> Done {
    done(0, out, "")
}

/// 待ち・見張りが「消えた」で終わるときの姿（標準エラーへ 1 行・終了コード 3）。
pub fn gone(why: &str) -> Done {
    done(3, "", &format!("gone: {why}\n"))
}

/// 1 本のテストの置き場所（`target\test-roots\` の下。落とすと中身ごと消える）。
pub struct Desk {
    pub root: TempPath,
    /// 子プロセスの出力を受けるファイルの連番。
    serial: Cell<u32>,
}

impl Desk {
    pub fn new(label: &str) -> Self {
        let root = TempPath::under_target(label);
        // 置き場所は Windows の形の絶対パスで渡す（相対だと子のカレント基準で作られる）。
        assert!(root.path().is_absolute(), "{}", root.path().display());
        fs::create_dir(root.child("out")).expect("出力の置き場を作れる");
        let serial = Cell::new(0);
        Desk { root, serial }
    }

    /// 置き場所（`AREKA_IMPL_WATCH_HOME` の向け先）。作るのは実行ファイル。
    pub fn home(&self) -> PathBuf {
        self.root.child("home")
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.home().join(name)
    }

    /// 実行ファイルの呼び出し。置き場所は必ずこのテストの一時フォルダへ向ける。
    pub fn command(&self, line: &str) -> Command {
        let mut command = Command::new(EXE);
        command
            .args(line.split_whitespace())
            .env(HOME_ENV, self.home());
        command
    }

    /// 子プロセスを立てる。出力はファイルへ受ける（走っている間も読めるように）。
    pub fn spawn(&self, mut command: Command) -> Running<'_> {
        let serial = self.serial.replace(self.serial.get() + 1);
        let out = self.root.child(&format!("out/{serial}.out"));
        let err = self.root.child(&format!("out/{serial}.err"));
        let shown = format!("{command:?}");
        let child = command
            .stdin(Stdio::null())
            .stdout(File::create(&out).expect("標準出力の受け先を作れる"))
            .stderr(File::create(&err).expect("標準エラーの受け先を作れる"))
            .spawn()
            .expect("実行ファイルを起こせる");
        let desk = self;
        Running {
            desk,
            child,
            out,
            err,
            shown,
        }
    }

    pub fn start(&self, line: &str) -> Running<'_> {
        self.spawn(self.command(line))
    }

    /// 直ちに終わるはずのコマンドを呼ぶ。
    pub fn call(&self, line: &str) -> Done {
        self.start(line).end()
    }

    /// 待つコマンドを立て、その待ち・見張りの記録（`id` の `kind`）が状態ファイルに載るまで待つ。
    pub fn waiter(&self, id: &str, kind: &str, line: &str) -> Running<'_> {
        let running = self.start(line);
        let what = format!("{id} の {kind} の記録（{line}）");
        self.until(&what, |state| has_wait(state, id, kind));
        self.settle();
        running
    }

    /// 記録を書いた呼び出しが、ログと読み物まで書き終えるのを待つ。走り続ける子の記録が状態
    /// ファイルに見えた後、その副産物（ログの行・読み物・一時ファイルの無さ）を確かめる前に呼ぶ。
    ///
    /// 実行ファイルは、状態ファイルの置き換え → ログ → 読み物の置き換えの間じゅう `state.lock`
    /// を握る。記録が見えた後でこのロックが取れたなら、その呼び出しは終わっている。取れたら
    /// 直ちに放す（握るのは一瞬。実行ファイルは 10 ms ごとに 10 秒まで試し直す）。`state.lock`
    /// は記録を書いた実行ファイルが作っているので、ここでは作らずに開く。
    pub fn settle(&self) {
        let lock = File::open(self.file("state.lock")).expect("state.lock を開ける");
        retry("state.lock が空く", || lock.try_lock().ok());
        lock.unlock().expect("state.lock を放せる");
    }

    /// 見張りを立てる（参加）。リポジトリは `areka`。
    pub fn watch(&self, id: &str) -> Running<'_> {
        self.waiter(id, "watch", &format!("watch --id {id} --repo areka"))
    }

    /// 状態ファイルが `pred` を満たすまで待ち、そのときの状態を返す。
    pub fn until(&self, what: &str, pred: impl Fn(&Value) -> bool) -> Value {
        retry(what, || {
            let text = fs::read_to_string(self.file("state.json")).ok()?;
            serde_json::from_str(&text).ok().filter(|state| pred(state))
        })
    }

    pub fn state(&self) -> Value {
        self.until("状態ファイルが読める", |_| true)
    }

    pub fn state_text(&self) -> String {
        read(&self.file("state.json"))
    }

    /// 状態ファイルを手で置く（書きかけを読ませないよう、別名に書いてから置き換える）。
    pub fn put_state(&self, text: &str) {
        let temp = self.root.child("put.tmp");
        fs::create_dir_all(self.home()).expect("置き場所を作れる");
        fs::write(&temp, text).expect("状態ファイルの中身を書ける");
        fs::rename(&temp, self.file("state.json")).expect("状態ファイルを置ける");
    }

    pub fn log(&self) -> String {
        read(&self.file("impl-watch.log"))
    }

    /// ログの、水準が `level` で `what` を含む行の数。
    pub fn log_lines(&self, level: &str, what: &str) -> usize {
        let level = format!(" {level} ");
        let log = self.log();
        let found = log
            .lines()
            .filter(|line| line.contains(&level) && line.contains(what));
        found.count()
    }

    /// ログに「`command` の呼び出しで `event` が起きた」の行が在るか。
    pub fn logged(&self, command: &str, event: &str) -> bool {
        let line = format!("[store] state changed command=\"{command}\" event={event}");
        self.log_lines("INFO", &line) > 0
    }

    pub fn assert_logged(&self, command: &str, event: &str) {
        let found = self.logged(command, event);
        let log = self.log();
        assert!(found, "ログに無い: command={command} event={event}\n{log}");
    }

    /// `status` を、端末の要約が `pred` を満たすまで呼ぶ。どの回も 0 で終わり、ASCII だけを出す。
    pub fn status_until(&self, what: &str, pred: impl Fn(&str) -> bool) -> String {
        retry(what, || {
            let end = self.call("status");
            assert_eq!((end.code, end.err.as_str()), (0, ""), "{}", end.out);
            assert!(end.out.is_ascii(), "{}", end.out);
            Some(end.out).filter(|summary| pred(summary))
        })
    }

    pub fn status(&self) -> String {
        self.status_until("status", |_| true)
    }

    /// `tick`（状態を変える呼び出し）を、状態が `pred` を満たすまで呼ぶ。殺した子のロックを
    /// OS が解くまでの間を吸う。
    pub fn tick_until(&self, what: &str, pred: impl Fn(&Value) -> bool) -> Value {
        retry(what, || {
            assert_eq!(self.call("tick"), ok("tick\n"));
            Some(self.state()).filter(|state| pred(state))
        })
    }

    /// 置き場所の下の、名前が `prefix` で始まるファイル。
    pub fn files_named(&self, prefix: &str) -> Vec<PathBuf> {
        let entries = fs::read_dir(self.home()).expect("置き場所を読める");
        let paths = entries.map(|entry| entry.expect("置き場所を読める").path());
        let named = |path: &PathBuf| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with(prefix)
        };
        paths.filter(named).collect()
    }

    /// 書きかけの一時ファイル（`state.json.<pid>.tmp`・`status.md.<pid>.tmp`）が残っていない。
    pub fn assert_no_temp_files(&self) {
        let left: Vec<_> = ["state.json.", "status.md."]
            .iter()
            .flat_map(|prefix| self.files_named(prefix))
            .filter(|path| path.extension().is_some_and(|ext| ext == "tmp"))
            .collect();
        assert_eq!(left, Vec::<PathBuf>::new());
    }
}

/// 立てた子プロセス。落とすときに、この子だけを止めて待つ。
pub struct Running<'d> {
    desk: &'d Desk,
    child: Child,
    out: PathBuf,
    err: PathBuf,
    shown: String,
}

impl Running<'_> {
    /// 終わるのを上限まで待ち、終了コードと出力を返す。
    pub fn end(&mut self) -> Done {
        self.end_within(LIMIT)
    }

    pub fn end_within(&mut self, limit: Duration) -> Done {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("子の終わりを見られる") {
                let code = status.code().expect("終了コードが在る");
                return done(code, &read(&self.out), &read(&self.err));
            }
            if started.elapsed() >= limit {
                let state = fs::read_to_string(self.desk.file("state.json"));
                panic!("{limit:?} 経っても終わらない: {}\n{state:?}", self.shown);
            }
            std::thread::sleep(STEP);
        }
    }

    /// まだ走っていて、何も出していない（待っている間はセッションを起こさない。要件 13.1）。
    pub fn assert_waiting_silently(&mut self) {
        let ended = self.child.try_wait().expect("子の終わりを見られる");
        let (said, shown) = ((read(&self.out), read(&self.err)), &self.shown);
        assert!(
            ended.is_none(),
            "待っているはずが終わった: {shown} {ended:?} {said:?}"
        );
        assert_eq!(said, (String::new(), String::new()), "{shown}");
    }

    /// 殺す（落ちたセッションの代わり）。
    pub fn kill(&mut self) {
        // すでに終わっている子では失敗するだけ。
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.kill();
    }
}

pub fn has_wait(state: &Value, id: &str, kind: &str) -> bool {
    let waits = state["waits"].as_array();
    waits.is_some_and(|waits| waits.iter().any(|w| w["id"] == id && w["kind"] == kind))
}

pub fn has_participant(state: &Value, id: &str) -> bool {
    !state["participants"][id].is_null()
}

pub fn count(list: &Value) -> usize {
    list.as_array().map_or(0, Vec::len)
}

/// マージの番が来た 1 行で、直前のマージが `last`（`PR#<n> <sha> <spec>`）と時刻で載っている。
pub fn assert_granted_after(end: &Done, last: &str) {
    let head = format!("granted merge repo=areka; last: {last} 20");
    let one_line = end.out.ends_with("Z\n") && end.out.lines().count() == 1;
    assert_eq!((end.code, end.err.as_str()), (0, ""), "{end:?}");
    assert!(end.out.starts_with(&head) && one_line, "{end:?}");
}
