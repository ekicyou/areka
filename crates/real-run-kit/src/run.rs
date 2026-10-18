//! 走りの取っ手（Run）・既定の時間・出力の読み口・破棄の順（要件 1.8・3.2〜3.6・4.2・4.9・5.1〜5.5）。
//!
//! 待ち・呼び出し・閉じ方は下の段（output・mcp・close）に任せ、既定の時間の各欄を引数として渡す。
//! 破棄では、子が生きていれば静かに閉じてから（5.5）、欄の破棄として回の置き場が残すか消すかを決める。

use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use crate::close::{self, Ended};
use crate::evidence::RunDirs;
use crate::mcp::{McpChannel, McpError, ToolReply, Waits};
use crate::output::{self, ChildExited, WaitEnd};

/// 既定の時間（D-13）。`safety_exit < deadline` を兄弟のテストが固定する（1.8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// 各待ちの始まりから数える締切（今の番犬と同じ 60 秒）。
    pub deadline: Duration,
    /// 子の起動から数える保険の自動終了（締切より短い＝窓口が止めるより先に子が自分で終わる）。
    pub safety_exit: Duration,
    /// 出力・子の状態を読み直す間隔。
    pub poll: Duration,
    /// `call_until` の呼び直しの間隔。
    pub call_retry: Duration,
    /// 閉じる知らせを送り直す間隔。
    pub close_retry: Duration,
    /// MCP の輸送の失敗の後に子の終わりを観測する上限。
    pub exit_grace: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            deadline: Duration::from_secs(60),
            safety_exit: Duration::from_secs(45),
            poll: Duration::from_millis(50),
            call_retry: Duration::from_millis(250),
            close_retry: Duration::from_secs(1),
            exit_grace: Duration::from_secs(2),
        }
    }
}

/// 走り 1 回の取っ手。1 つの試験のスレッドだけが使う。
pub struct Run {
    child: Child,
    stdout: PathBuf,
    stderr: PathBuf,
    mcp: McpChannel,
    limits: Limits,
    port: u16,
    /// 最初に観測した終わり（`close`・`wait_exit` の 2 度目からはこれを返す）。
    ended: Option<Ended>,
    /// 回の置き場。`Run::drop` の本体（子を止める）の後に欄として破棄され、残すか消すかを決める。
    dirs: RunDirs,
}

impl Run {
    /// 起こした子と回の置き場から取っ手を作る（起こすのは launch）。
    pub(crate) fn new(child: Child, dirs: RunDirs, port: u16, limits: Limits) -> Run {
        Run {
            child,
            stdout: dirs.stdout_log(),
            stderr: dirs.stderr_log(),
            mcp: McpChannel::new(port),
            limits,
            port,
            ended: None,
            dirs,
        }
    }

    /// 子のプロセスの番号。
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// 子へ渡した MCP の番号。
    pub fn mcp_port(&self) -> u16 {
        self.port
    }

    /// 回の置き場。
    pub fn run_dir(&self) -> &Path {
        self.dirs.run()
    }

    /// 子の標準出力のファイル。
    pub fn stdout_path(&self) -> &Path {
        &self.stdout
    }

    /// 子の標準エラーのファイル。
    pub fn stderr_path(&self) -> &Path {
        &self.stderr
    }

    /// 標準出力の全文。走りの途中でも終わった後でも読める（3.6）。読めない字は U+FFFD。
    pub fn stdout(&self) -> String {
        read_lossy(&self.stdout)
    }

    /// 標準エラーの全文（`stdout` と同じ読み方）。
    pub fn stderr(&self) -> String {
        read_lossy(&self.stderr)
    }

    /// 標準出力・改行・標準エラー。
    pub fn output(&self) -> String {
        format!("{}\n{}", self.stdout(), self.stderr())
    }

    /// 出力（両方）に `needle` が現れるまで待つ（3.2）。子が先に終われば `Err`（3.5）。
    ///
    /// # Panics
    ///
    /// 締切までに現れないとき（待っていた文字列・待った時間・ファイルの場所と末尾＝3.4）。
    pub fn wait_for(&mut self, needle: &str) -> Result<(), ChildExited> {
        let files = [self.stdout.as_path(), self.stderr.as_path()];
        let deadline = Instant::now() + self.limits.deadline;
        match output::wait_any(
            files,
            &[needle],
            &mut self.child,
            deadline,
            self.limits.poll,
        ) {
            Ok(_) => Ok(()),
            Err(WaitEnd::ChildExited(exited)) => Err(exited),
            Err(WaitEnd::Deadline) => panic!(
                "{}",
                output::failure_text(&[needle], self.limits.deadline, files)
            ),
        }
    }

    /// ツールを 1 回呼ぶ（4.2〜4.8）。`args_json` は JSON の object の文（`"{}"` 可・読めなければ panic）。
    pub fn call(&mut self, tool: &str, args_json: &str) -> Result<ToolReply, McpError> {
        let files = [self.stdout.as_path(), self.stderr.as_path()];
        let waits = waits(&self.limits);
        self.mcp
            .call(&mut self.child, files, tool, args_json, waits)
    }

    /// 答えの文に `needle` が現れるまで呼び直す（4.9）。
    ///
    /// # Panics
    ///
    /// 締切までに現れないとき（最後の答えを添える）。
    pub fn call_until(
        &mut self,
        tool: &str,
        args_json: &str,
        needle: &str,
    ) -> Result<ToolReply, McpError> {
        let files = [self.stdout.as_path(), self.stderr.as_path()];
        let waits = waits(&self.limits);
        self.mcp
            .call_until(&mut self.child, files, tool, args_json, needle, waits)
    }

    /// 利用者と同じ閉じ方で閉じる（5.1・5.3）。2 度目からは同じ終わりを返す。
    ///
    /// # Panics
    ///
    /// 締切までに終わらないとき（子を止めて回収してから＝5.4）。
    pub fn close(&mut self) -> Ended {
        if let Some(ended) = &self.ended {
            return ended.clone();
        }
        let files = [self.stdout.as_path(), self.stderr.as_path()];
        let l = self.limits;
        let ended = close::close(&mut self.child, files, l.deadline, l.poll, l.close_retry);
        self.ended.insert(ended).clone()
    }

    /// 閉じる知らせを送らず、子が自分で終わるのを待つ（5.2）。2 度目からは同じ終わりを返す。
    ///
    /// # Panics
    ///
    /// 締切までに終わらないとき（子を止めて回収してから＝5.4）。
    pub fn wait_exit(&mut self) -> Ended {
        if let Some(ended) = &self.ended {
            return ended.clone();
        }
        let files = [self.stdout.as_path(), self.stderr.as_path()];
        let l = self.limits;
        let ended = close::wait_exit(&mut self.child, files, l.deadline, l.poll);
        self.ended.insert(ended).clone()
    }
}

impl Drop for Run {
    /// ⑴ 子が生きていれば静かに閉じる（締切で止める・panic しない＝5.5）。⑵ この本体の後に欄が
    /// 破棄され、`RunDirs` が残すか消すかを決める（子がファイルを掴んだまま複写・削除しない）。
    fn drop(&mut self) {
        if self.ended.is_some() {
            return;
        }
        let files = [self.stdout.as_path(), self.stderr.as_path()];
        let l = self.limits;
        // 既に終わっていれば、stop_quietly は送らずに回収して戻る。
        close::stop_quietly(&mut self.child, files, l.deadline, l.poll, l.close_retry);
    }
}

fn waits(l: &Limits) -> Waits {
    Waits {
        deadline: l.deadline,
        poll: l.poll,
        call_retry: l.call_retry,
        exit_grace: l.exit_grace,
    }
}

fn read_lossy(path: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned()
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
