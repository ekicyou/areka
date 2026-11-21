//! 閉じる知らせの送り直し・終わりの待ち・止め方・終わり方の読み取り。
//!
//! 閉じる知らせは `taskkill /PID <番号>`（`/F` なし・`/T` なし）＝利用者が窓を閉じるのと同じ
//! 閉じる要求。送る直前に取っ手の `try_wait` で子が生きていることを見るので、届くのは窓口が
//! 起こした子だけ（取っ手を持つ間、OS はその番号を使い回さない）。止めるのは取っ手への
//! `Child::kill` だけで、名前やほかの番号では止めない。時間は呼び手が渡す
//! （`close`・`wait_exit` は欄ごとの値・`stop_on_drop` は `Limits`）。既定の値はここでは作らない。

use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};

use crate::output::{self, WaitEnd};
use crate::run::Limits;

/// 閉じる知らせを受け取った印の行（kanade と結線した起動／未結線の起動の直ちの終了・ほかの出所の先着）。
const ACK: [&str; 2] = ["event=\"os_close_request\"", EXIT_LINE];

/// 終わり方の行（`quit_app` の `info`）。閉じる引用符まで含めて `app_exit_again` などと分ける。
const EXIT_LINE: &str = "event=\"app_exit\"";

/// 子の終わり方。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndedBy {
    /// 利用者と同じ閉じ方（`origin=KanadeStopped(…)` か `origin=OsClose`）。
    UserClose,
    /// 保険の自動終了（`origin=Smoke`）。
    SafetyExit,
    /// ほかの出所（`origin=` の値をそのまま。例: 記憶の書き込みの失敗）。
    Other(String),
    /// 終わり方の行が無い（窓を作る前に止まった起動など）。
    Unlogged,
}

/// 終わった走りの終了コードと終わり方。下の段の close で定め、run とクレートの入口が使う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    /// 子の終わり方（終了コードは `status.code()`）。
    pub status: ExitStatus,
    /// 利用者と同じ閉じ方か、保険の自動終了か、ほかか。
    pub by: EndedBy,
}

/// 出力の中の最初の終わり方の行の `origin=` の値で読み分ける（`quit_app` は最初の出所だけを `info` で出す）。
///
/// 値は本物の `?origin`（`Debug`・引用符なし）。中身に空白を含む値（`MemoryWriteFailed(PersistFailure { … })`）も
/// 括弧と文字列の内側を数えて丸ごと取る。行に `origin=` が無ければ `Other("")`。
pub(crate) fn ended_by(output: &str) -> EndedBy {
    let Some(line) = output.lines().find(|l| l.contains(EXIT_LINE)) else {
        return EndedBy::Unlogged;
    };
    let value = line
        .split_once("origin=")
        .map(|(_, rest)| debug_value(rest))
        .unwrap_or_default();
    match value {
        "OsClose" => EndedBy::UserClose,
        v if v.starts_with("KanadeStopped(") => EndedBy::UserClose,
        "Smoke" => EndedBy::SafetyExit,
        v => EndedBy::Other(v.to_string()),
    }
}

/// `Debug` の値を、括弧と文字列の外の最初の空白の手前まで取る。
fn debug_value(s: &str) -> &str {
    let (mut depth, mut in_str, mut escaped) = (0u32, false, false);
    for (i, c) in s.char_indices() {
        if in_str {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            c if c.is_whitespace() && depth == 0 => return &s[..i],
            _ => {}
        }
    }
    s
}

/// 閉じる知らせを送り直しながら終わりを待つ。締切で子を止めて回収し、panic する。
pub(crate) fn close(
    child: &mut Child,
    files: [&Path; 2],
    deadline: Duration,
    poll: Duration,
    close_retry: Duration,
) -> Ended {
    match close_or_kill(child, files, deadline, poll, close_retry) {
        Ok(status) => ended(status, files),
        Err(stop) => panic!("{}", stop.failure("close", deadline, files)),
    }
}

/// 破棄から呼ぶ形: `close` と同じ手順で閉じる知らせを送り直しつつ締切まで待つ。締切で子を止めて
/// 回収したら、panic の途中なら ASCII の 1 行を標準エラーへ書くだけ（二重の panic にしない）、違えば理由を
/// 書いて panic する（閉じ忘れて子を止めた回を緑にしない）。
pub(crate) fn stop_on_drop(child: &mut Child, files: [&Path; 2], limits: &Limits) {
    let Err(stop) = close_or_kill(
        child,
        files,
        limits.deadline,
        limits.poll,
        limits.close_retry,
    ) else {
        return;
    };
    if std::thread::panicking() {
        eprintln!(
            "real-run-kit: stopped child {} that did not close within {:?} (dropped while panicking)",
            stop.pid, limits.deadline,
        );
    } else {
        panic!(
            "{}\nclose() も wait_exit() も呼ばずに取っ手を捨てた（破棄の閉じる知らせで子が終わらなかった）",
            stop.failure("破棄", limits.deadline, files),
        );
    }
}

/// 閉じる知らせを送らずに子が自分で終わるのを待つ。締切で子を止めて回収し、panic する。
pub(crate) fn wait_exit(
    child: &mut Child,
    files: [&Path; 2],
    deadline: Duration,
    poll: Duration,
) -> Ended {
    let end = Instant::now() + deadline;
    match wait_until(child, end, poll) {
        Some(status) => ended(status, files),
        None => {
            let stop = kill(child, "送っていない（自分で終わるのを待つ形）".to_string());
            panic!("{}", stop.failure("wait_exit", deadline, files))
        }
    }
}

/// 締切で止めた子の記録。
struct Stopped {
    pid: u32,
    /// 最後の `taskkill` の結果（終了コード）。
    last_send: String,
    /// 回収の結果。
    reaped: Result<ExitStatus, String>,
}

impl Stopped {
    /// 締切切れの文: 止めたこと・最後の送り・出力の場所と末尾（末尾の組み立ては output と共用）。
    fn failure(&self, what: &str, deadline: Duration, files: [&Path; 2]) -> String {
        let reaped = match &self.reaped {
            Ok(status) => format!("回収した終わり方 {status}"),
            Err(e) => format!("回収できなかった: {e}"),
        };
        let tails = output::failure_text(&ACK, deadline, files);
        // 頭の 1 行（「…を待ったが現れなかった」）はこちらの文で言い直すので落とす。
        let tails = tails.split_once('\n').map_or("", |(_, rest)| rest);
        format!(
            "締切切れ: {what} が締切 {deadline:?} の内に子の終わりを観測できず、窓口が起こした子（PID {}）を止めた（{reaped}）\n最後の閉じる知らせ: {}\n{tails}",
            self.pid, self.last_send,
        )
    }
}

fn ended(status: ExitStatus, files: [&Path; 2]) -> Ended {
    let text = files.map(output::read_text).join("\n");
    Ended {
        status,
        by: ended_by(&text),
    }
}

/// Sending（`close_retry` ごとに送り直す）→ Acked（送り直さない）
/// → Exited。締切なら Killed（`Err`）。
fn close_or_kill(
    child: &mut Child,
    files: [&Path; 2],
    deadline: Duration,
    poll: Duration,
    close_retry: Duration,
) -> Result<ExitStatus, Stopped> {
    let end = Instant::now() + deadline;
    let mut last_send = "まだ送っていない".to_string();
    // 送り直しをまたいで読んだ位置を持つ（送り直すたびに頭から読み直さない）。
    let mut tails = output::tails(files);
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Ok(status);
        }
        if Instant::now() >= end {
            return Err(kill(child, last_send));
        }
        last_send = send_close(child.id());
        let next = (Instant::now() + close_retry).min(end);
        match output::wait_any(&mut tails, &ACK, child, next, poll) {
            Ok(_) => return wait_until(child, end, poll).ok_or_else(|| kill(child, last_send)),
            Err(WaitEnd::ChildExited(exited)) => return Ok(exited.status),
            Err(WaitEnd::Deadline) => {}
        }
    }
}

/// 強制なしの閉じる知らせを 1 度送り、終了コードを返す。taskkill の出力は OEM のコードページ（日本語の
/// 環境では cp932）で、UTF-8 として読むと化けるので捨てる。
fn send_close(pid: u32) -> String {
    match Command::new("taskkill")
        .args(["/PID", &pid.to_string()])
        .output()
    {
        Ok(out) => format!("taskkill /PID {pid} の終了コード {:?}", out.status.code()),
        Err(e) => format!("taskkill /PID {pid} を起こせなかった: {e}"),
    }
}

/// 締切まで `poll` ごとに子の終わりを見る。終わらなければ `None`。
fn wait_until(child: &mut Child, end: Instant, poll: Duration) -> Option<ExitStatus> {
    loop {
        // 終わりを見られないのは取っ手の故障。終わったとは言えないので締切まで見続ける。
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status);
        }
        if Instant::now() >= end {
            return None;
        }
        std::thread::sleep(poll);
    }
}

/// 窓口の取っ手の子だけを止めて回収する。
fn kill(child: &mut Child, last_send: String) -> Stopped {
    // 既に終わっていれば kill は失敗するが、回収はできる。
    let _ = child.kill();
    Stopped {
        pid: child.id(),
        last_send,
        reaped: child.wait().map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
#[path = "close_tests.rs"]
mod tests;
