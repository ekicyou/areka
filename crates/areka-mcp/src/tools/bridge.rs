//! アプリ本体へ届ける橋（送る・待つ・上限・終了の途中・記録）。

use std::sync::mpsc::Sender;
use std::time::Duration;

use areka_actor::{ReplyError, ReplyReceiver, ReplySender, reply_channel};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use super::{ToolCall, outcome};
use crate::{ToolContent, ToolOutcome};

/// アプリ本体へ届ける 1 件。
pub struct ToolRequest {
    pub call: ToolCall,
    pub reply: ReplyTo,
}

impl ToolRequest {
    /// 要求と、その返事を受ける側の対を作る（橋もテストもこの 1 つの関数で作る）。
    pub fn new(call: ToolCall) -> (ToolRequest, Pending) {
        let (sender, receiver) = reply_channel();
        let answered = CancellationToken::new();
        let abandoned = CancellationToken::new();
        let reply = ReplyTo {
            sender: Some(sender),
            answered: answered.clone(),
            abandoned: abandoned.clone(),
            ghost: String::new(),
        };
        let pending = Pending {
            receiver,
            answered,
            abandoned,
        };
        (ToolRequest { call, reply }, pending)
    }
}

/// 返事の送り手（Send。後から・別スレッドから送れる。送らずに落とすと「答えずに手放した」になる）。
pub struct ReplyTo {
    sender: Option<ReplySender<Answer>>,
    /// 送った（または落とした）合図。立てるのは `Drop` の 1 か所だけ。
    answered: CancellationToken,
    /// 逆向きの合図（`Pending` が落ちた）。
    abandoned: CancellationToken,
    ghost: String,
}

impl ReplyTo {
    /// 解決したゴーストの名前（一覧に出す値）を記録用に添える。
    pub fn for_ghost(mut self, label: impl Into<String>) -> Self {
        self.ghost = label.into();
        self
    }

    /// 返事を 1 回だけ送る。受け手がもう居なければ（上限の後）黙って捨てる。
    pub fn send(mut self, outcome: ToolOutcome) {
        if let Some(sender) = self.sender.take() {
            let ghost = std::mem::take(&mut self.ghost);
            // 受け手が居ない（上限を過ぎた）なら捨てる＝応答は 1 度だけ。
            let _ = sender.send(Answer { ghost, outcome });
        }
        // 合図はこの後の `Drop` が立てる。
    }

    /// 待つ側がもう居ない（上限を過ぎて `Pending` が落ちた）なら true。
    pub fn is_abandoned(&self) -> bool {
        self.abandoned.is_cancelled()
    }
}

impl Drop for ReplyTo {
    fn drop(&mut self) {
        // 送り手を先に落としてから合図を立てる。逆にすると、合図で起きた待つ側が
        // 「答えずに落ちた」と「未着」を見分けられない。
        drop(self.sender.take());
        self.answered.cancel();
    }
}

/// 返事 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub ghost: String,
    pub outcome: ToolOutcome,
}

/// 返事を受ける側。
pub struct Pending {
    receiver: ReplyReceiver<Answer>,
    answered: CancellationToken,
    abandoned: CancellationToken,
}

impl Pending {
    /// 待たずに覗く（アプリ本体側のテストが使う）。
    pub fn try_answer(&self) -> Result<Option<Answer>, ReplyError> {
        self.receiver.try_recv()
    }

    /// 送った（または落とした）合図の写し。待つ側は `Pending` への参照を `.await` を
    /// またいで持たず（中の受け手は `Sync` でない）、これを先に取り出して待つ。
    pub(crate) fn answered(&self) -> CancellationToken {
        self.answered.clone()
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.abandoned.cancel();
    }
}

/// 上限の文言（本番の 10 秒で "areka did not respond within 10 seconds"）。
pub(crate) fn timeout_text(limit: Duration) -> String {
    format!(
        "areka did not respond within {} seconds",
        limit.as_secs_f64()
    )
}

/// 送って待つ（登録表の処理の中身）。
///
/// 待つ間は `Pending` への参照を `.await` をまたいで持たない（中の受け手は `Sync` で
/// ないので、フューチャが `Send` でなくなる）。合図の写しを先に取り出して待つ。
pub(crate) async fn call(tx: Sender<ToolRequest>, call: ToolCall, limit: Duration) -> ToolOutcome {
    let tool = call.name();
    let (request, pending) = ToolRequest::new(call);
    let (ghost, result) = if tx.send(request).is_err() {
        // 受け口が落ちている（終了の途中）。返ってきた要求ごと落とし、待たない。
        (String::new(), shutting_down(tool))
    } else {
        let answered = pending.answered();
        // 上限を過ぎたかどうかは下の `try_answer` が見分ける。
        let _ = tokio::time::timeout(limit, answered.cancelled()).await;
        match pending.try_answer() {
            Ok(Some(answer)) => (answer.ghost, answer.outcome),
            Ok(None) => {
                warn!(
                    tool,
                    limit_ms = limit.as_millis() as u64,
                    "MCP: areka が上限のうちに答えなかった"
                );
                (String::new(), outcome::ng(timeout_text(limit)))
            }
            Err(_) => (String::new(), shutting_down(tool)),
        }
    };
    // ここで `Pending` が落ちる＝上限の後に届く返事は捨てられる。
    drop(pending);
    let text = match result.content.first() {
        Some(ToolContent::Text(text)) if result.is_error => text.as_str(),
        _ => "",
    };
    debug!(
        tool,
        ghost = ghost.as_str(),
        is_error = result.is_error,
        text,
        "MCP: ツールに答えた"
    );
    result
}

/// 答えずに手放された（終了の途中）。
fn shutting_down(tool: &str) -> ToolOutcome {
    warn!(tool, "MCP: areka が終了の途中で答えなかった");
    outcome::ng("areka is shutting down")
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
