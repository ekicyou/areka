//! MCP の呼び口（待受の観測・ツールの呼び出し・答えの文の取り出し＝要件 4.2〜4.9・9.2・10.5）。
//!
//! 走りごとに 1 つの tokio `current_thread` の器を持ち、呼び手には同期の関数だけを見せる（10.5）。
//! つなぐのは中継の `scan` で名乗りを確かめた `Link`（rmcp のクライアント）で、使い回す（D-10）。
//! 子の取っ手と出力の 2 つのファイルは呼び出しごとに借りる（持ち主は run）。時間はすべて引数で受ける。

use std::fmt;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use areka_mcp_bridge::config::Timeouts;
use areka_mcp_bridge::scan::{Link, Scan, scan};
use rmcp::model::{CallToolRequestParams, CallToolResult, JsonObject};

use crate::output::{self, ChildExited, WaitEnd};

/// 待受を始めた行（先の候補が使用中で移った版も同じ頭で始まるので前方一致で拾う）。
const LISTEN_STARTED: &str = "MCP: 待受を始めた";
/// どの候補も束ねられなかった行（選んだ番号が先に取られていた＝9.2）。
const LISTEN_FAILED: &str = "MCP: どの候補のポートも束ねられなかった（待ち受けない）";
/// 待ち受けない設定の行。
const LISTEN_OFF: &str = "MCP: 待ち受けない（AREKA_MCP_PORT が 0）";

/// ツールの答え（4.6）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolReply {
    /// 文の片を順に "\n" で繋いだもの（絵などの文でない片は飛ばす）。
    pub text: String,
    /// ツールが失敗を答えた（isError）。成功の答えと見分ける（4.6）。
    pub is_error: bool,
}

/// MCP の呼び出しの失敗（4.5・4.7・4.8・9.2）。
#[derive(Debug)]
pub enum McpError {
    /// 子の記録が待受を始められなかったことを示した（4.5・9.2）。`line` はその行の全文。
    ListenFailed { port: u16, line: String },
    /// つながらない・答えが読めない・締切を過ぎた（4.7）。`target` は "127.0.0.1:<番号>"。
    Unreachable { target: String, reason: String },
    /// 呼び出しの前か途中で子が終わった（4.8）。
    ChildExited(ChildExited),
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpError::ListenFailed { port, line } => {
                write!(f, "MCP の待受を始められなかった（番号 {port}）: {line}")
            }
            McpError::Unreachable { target, reason } => {
                write!(f, "MCP につながらない（{target}）: {reason}")
            }
            McpError::ChildExited(e) => write!(
                f,
                "MCP の呼び出しの前か途中で子が終わった（終了コード {:?}）",
                e.status.code()
            ),
        }
    }
}

impl std::error::Error for McpError {}

/// 呼び出しの時間（既定の時間の型には頼らない・呼び手の run が既定の値を渡す）。
// run（4.1）が使い始めるまでは試験の外で使われない。
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Waits {
    /// 呼び出し 1 回（`call_until` は呼び直し全体）の締切。
    pub(crate) deadline: Duration,
    /// 出力と子の終わりの見回りの間隔。
    pub(crate) poll: Duration,
    /// `call_until` の呼び直しの間隔（`poll` とは別＝`get_log` の答えは大きい）。
    pub(crate) call_retry: Duration,
    /// 輸送の失敗の後に子の終わりを観測する猶予。
    pub(crate) exit_grace: Duration,
}

/// 走り 1 つ分の MCP の呼び口。`Link` は器より先に捨てる（欄の順）。
// run（4.1）が使い始めるまでは試験の外で使われない。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct McpChannel {
    link: Option<Link>,
    port: u16,
    listening: bool,
    rt: tokio::runtime::Runtime,
}

// run（4.1）が使い始めるまでは試験の外で使われない。
#[cfg_attr(not(test), allow(dead_code))]
impl McpChannel {
    /// 選んだ番号への呼び口（まだつながない）。器を作れないのは試験の机の故障なので panic。
    pub(crate) fn new(port: u16) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("MCP の呼び口の非同期の器を作れなかった: {e}"));
        McpChannel {
            link: None,
            port,
            listening: false,
            rt,
        }
    }

    /// ツールを 1 回呼ぶ（4.2・4.6〜4.8）。`args` は JSON の object の文（読めなければ panic）。
    pub(crate) fn call(
        &mut self,
        child: &mut Child,
        files: [&Path; 2],
        name: &str,
        args: &str,
        waits: Waits,
    ) -> Result<ToolReply, McpError> {
        let args = parse_args(args);
        let end = Instant::now() + waits.deadline;
        self.call_by(child, files, name, &args, end, waits)
            .map_err(|stop| match stop {
                Stop::Err(e) => e,
                Stop::Deadline(reason) => McpError::Unreachable {
                    target: self.target(),
                    reason,
                },
            })
    }

    /// 答えの文に `needle` が現れるまで呼び直す（4.9）。失敗の答えと含まない答えは `call_retry` だけ
    /// 待って呼び直す。`McpError` はそのまま返す。締切（呼び出しの途中で来たものも）は
    /// 3.4 と同じ形の文に最後の答え（無ければ「答え無し」）を添えて panic。
    pub(crate) fn call_until(
        &mut self,
        child: &mut Child,
        files: [&Path; 2],
        name: &str,
        args: &str,
        needle: &str,
        waits: Waits,
    ) -> Result<ToolReply, McpError> {
        let args = parse_args(args);
        let started = Instant::now();
        let end = started + waits.deadline;
        let mut last: Option<ToolReply> = None;
        loop {
            if Instant::now() < end {
                match self.call_by(child, files, name, &args, end, waits) {
                    Ok(reply) if !reply.is_error && reply.text.contains(needle) => {
                        return Ok(reply);
                    }
                    Ok(reply) => last = Some(reply),
                    Err(Stop::Err(e)) => return Err(e),
                    Err(Stop::Deadline(_)) => {}
                }
            }
            let now = Instant::now();
            if now >= end {
                let last = last.map_or_else(|| "答え無し".to_string(), |r| format!("{r:?}"));
                panic!(
                    "{}--- 最後の `{name}` の答え ---\n{last}",
                    output::failure_text(&[needle], now - started, files)
                );
            }
            std::thread::sleep(waits.call_retry.min(end - now));
        }
    }

    fn target(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn call_by(
        &mut self,
        child: &mut Child,
        files: [&Path; 2],
        name: &str,
        args: &JsonObject,
        end: Instant,
        waits: Waits,
    ) -> Result<ToolReply, Stop> {
        self.ensure_listening(child, files, end, waits.poll)?;
        let link = match self.link.take() {
            Some(link) => link,
            None => self.connect(child, end)?,
        };
        let params = CallToolRequestParams::new(name.to_string()).with_arguments(args.clone());
        let outcome = self.rt.block_on(async {
            tokio::select! {
                r = tokio::time::timeout_at(end.into(), link.call_tool(params)) => match r {
                    Ok(Ok(result)) => Outcome::Reply(result),
                    Ok(Err(e)) => Outcome::Transport(e.to_string()),
                    Err(_) => Outcome::Deadline,
                },
                status = watch_exit(child, waits.poll) => Outcome::Exited(status),
            }
        });
        // 答えが来たときだけつなぎを使い回す（ほかは捨て、次の呼び出しがつなぎ直す）。
        match outcome {
            Outcome::Reply(result) => {
                self.link = Some(link);
                Ok(reply_of(result))
            }
            Outcome::Exited(e) => Err(McpError::ChildExited(e).into()),
            Outcome::Deadline => Err(Stop::Deadline(format!("`{name}` の答えを締切まで待った"))),
            Outcome::Transport(reason) => {
                // 子が受け口を閉じてから終わるまでの間の失敗を、子の終わりと見分ける（4.8）。
                // 猶予は締切を越えない。
                let grace = (Instant::now() + waits.exit_grace).min(end);
                loop {
                    if let Ok(Some(status)) = child.try_wait() {
                        return Err(McpError::ChildExited(ChildExited { status }).into());
                    }
                    if Instant::now() >= grace {
                        return Err(McpError::Unreachable {
                            target: self.target(),
                            reason,
                        }
                        .into());
                    }
                    std::thread::sleep(waits.poll);
                }
            }
        }
    }

    /// まだなら待受の 3 行のどれかが出るのを待つ（4.3〜4.5）。
    fn ensure_listening(
        &mut self,
        child: &mut Child,
        files: [&Path; 2],
        end: Instant,
        poll: Duration,
    ) -> Result<(), Stop> {
        if self.listening {
            return Ok(());
        }
        let needles = [LISTEN_STARTED, LISTEN_FAILED, LISTEN_OFF];
        match output::wait_any(files, &needles, child, end, poll) {
            Ok(0) => {
                self.listening = true;
                Ok(())
            }
            Ok(i) => Err(McpError::ListenFailed {
                port: self.port,
                line: line_with(files, needles[i]),
            }
            .into()),
            Err(WaitEnd::ChildExited(e)) => Err(McpError::ChildExited(e).into()),
            Err(WaitEnd::Deadline) => {
                Err(Stop::Deadline("締切まで待受の行が出なかった".to_string()))
            }
        }
    }

    /// 中継の `scan` で名乗りを確かめてつなぐ（4.2・4.7）。答えの上限などは中継の既定。
    fn connect(&mut self, child: &mut Child, end: Instant) -> Result<Link, Stop> {
        let port = self.port;
        let found = self.rt.block_on(async {
            tokio::time::timeout_at(end.into(), scan(&[port], &Timeouts::default())).await
        });
        let unreachable = |reason: String| {
            Stop::Err(McpError::Unreachable {
                target: format!("127.0.0.1:{port}"),
                reason,
            })
        };
        let stop = match found {
            Ok(Scan::Found { link, .. }) => return Ok(link),
            Ok(Scan::NotFound { others }) if others.is_empty() => {
                unreachable("areka と名乗る相手が居ない".to_string())
            }
            Ok(Scan::NotFound { others }) => unreachable(format!("別の相手が名乗った: {others:?}")),
            Err(_) => Stop::Deadline("名乗りの確かめが締切まで終わらなかった".to_string()),
        };
        // 確かめの間に子が終わっていれば、そちらが本当の訳（4.8）。
        if let Ok(Some(status)) = child.try_wait() {
            return Err(McpError::ChildExited(ChildExited { status }).into());
        }
        Err(stop)
    }
}

/// 呼び出しの終わり方。締切だけは呼び手ごとに扱いが違う（`call` は Unreachable・`call_until` は panic）。
enum Stop {
    Err(McpError),
    Deadline(String),
}

impl From<McpError> for Stop {
    fn from(e: McpError) -> Self {
        Stop::Err(e)
    }
}

enum Outcome {
    Reply(CallToolResult),
    Exited(ChildExited),
    Deadline,
    Transport(String),
}

/// 子が終わるまで `poll` ごとに見回る（呼び出しの未来と並べる）。
async fn watch_exit(child: &mut Child, poll: Duration) -> ChildExited {
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return ChildExited { status };
        }
        tokio::time::sleep(poll).await;
    }
}

/// 2 つのファイルから `needle` を含む最初の行の全文（読めない字は置き換え）。
fn line_with(files: [&Path; 2], needle: &str) -> String {
    files
        .iter()
        .filter_map(|p| std::fs::read(p).ok())
        .find_map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .lines()
                .find(|l| l.contains(needle))
                .map(str::to_string)
        })
        .unwrap_or_else(|| needle.to_string())
}

/// 答えの写し（4.6）: 文の片を "\n" で繋ぎ、文でない片は飛ばし、失敗の印が無ければ成功。
fn reply_of(result: CallToolResult) -> ToolReply {
    let text = result
        .content
        .iter()
        .filter_map(|c| c.as_text())
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    ToolReply {
        text,
        is_error: result.is_error.unwrap_or(false),
    }
}

/// 引数の文を JSON の object として読む。読めない文は呼び手の誤りなので panic（D-12）。
fn parse_args(args: &str) -> JsonObject {
    match serde_json::from_str(args) {
        Ok(serde_json::Value::Object(map)) => map,
        Ok(other) => panic!("MCP の引数が JSON の object でない: {other}"),
        Err(e) => panic!("MCP の引数が JSON として読めない（{e}）: {args}"),
    }
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
