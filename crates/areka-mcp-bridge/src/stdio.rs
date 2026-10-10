//! 標準入出力の輸送（1 行 1 要求）。
//!
//! rmcp の stdio の輸送（`AsyncRwTransport`）は JSON として読めない行を黙って捨てるので、
//! 行の形のエラー 2 つ（`-32700`・`-32600`）を自分で 1 行書く輸送を持つ（要件 1.6・1.7）。
//! 標準出力へ書くのは、応答の行とこの 2 つのエラーの行だけ（要件 5.1）。

use std::future::Future;
use std::io;
use std::sync::Arc;

use rmcp::RoleServer;
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::Transport;
use serde_json::error::Category;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::errors;

/// 最初の行の頭に付くことがある UTF-8 の BOM。
const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// 標準入出力を rmcp の輸送にする。
pub struct LineTransport<R, W> {
    reader: BufReader<R>,
    /// rmcp は `send` の未来を並べて走らせるので、1 行を書く間は鍵を持つ。
    writer: Arc<Mutex<W>>,
    /// 読みかけの行。rmcp は `receive` を `select!` の中で回し、途中で捨てて呼び直すので、
    /// 行が揃うまで持ち越す。上限は設けない（要件 1.8）。
    line: Vec<u8>,
    /// 次に揃う行が最初の行か（BOM を落とすのは最初の行だけ）。
    first: bool,
    /// 書いている途中の行の形のエラー。`receive` が途中で捨てられても行が欠けないように、
    /// 書く仕事は `receive` の外で回し、次の行を読む前に終わりを待つ。
    replying: Option<JoinHandle<()>>,
}

impl<R: AsyncRead, W> LineTransport<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer: Arc::new(Mutex::new(writer)),
            line: Vec::new(),
            first: true,
            replying: None,
        }
    }
}

impl<R, W> LineTransport<R, W> {
    /// 書いている途中の行の形のエラーが書き終わるのを待つ。
    async fn settle_reply(&mut self) {
        if let Some(task) = &mut self.replying {
            if let Err(error) = task.await {
                tracing::error!(%error, "[bridge] the task writing a line-shape error did not finish");
            }
            self.replying = None;
        }
    }
}

impl<R, W> Transport<RoleServer> for LineTransport<R, W>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Error = io::Error;

    fn send(
        &mut self,
        item: ServerJsonRpcMessage,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        // `serde_json::to_vec` は改行を含まない 1 行にする（要件 1.4）。
        let line = serde_json::to_vec(&item);
        let writer = self.writer.clone();
        async move { write_line(&writer, line?).await }
    }

    async fn receive(&mut self) -> Option<ClientJsonRpcMessage> {
        loop {
            self.settle_reply().await;
            match self.reader.read_until(b'\n', &mut self.line).await {
                // 入力の終わり。行末の無い読みかけが残っていれば、それが最後の 1 行。
                Ok(0) if self.line.is_empty() => return None,
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(%error, "[bridge] failed to read a line from stdin");
                    return None;
                }
            }
            // ここから次の `await` までは途切れないので、行の持ち越しと最初の行の印は食い違わない。
            let parsed = parse(&self.line, self.first);
            self.line.clear();
            self.first = false;
            let error = match parsed {
                None => continue,
                Some(Ok(message)) => return Some(message),
                Some(Err(error)) => error,
            };
            let reply = match error.classify() {
                Category::Syntax | Category::Eof => {
                    tracing::debug!(%error, "[bridge] a line is not JSON; answering -32700");
                    errors::parse_error()
                }
                Category::Data | Category::Io => {
                    tracing::debug!(%error, "[bridge] a line is not a JSON-RPC message; answering -32600");
                    errors::invalid_request()
                }
            };
            let writer = self.writer.clone();
            self.replying = Some(tokio::spawn(async move {
                if let Err(error) = write_line(&writer, reply.to_string().into_bytes()).await {
                    tracing::warn!(%error, "[bridge] failed to write a line-shape error to stdout");
                }
            }));
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        self.settle_reply().await;
        self.writer.lock().await.shutdown().await
    }
}

/// 1 行を要求として読む。`None` は空の行（空白だけの行を含む＝要件 1.5）。
/// 行末の CR・LF と前後の空白を落とし、最初の行だけ頭の BOM も落とす。
fn parse(line: &[u8], first: bool) -> Option<serde_json::Result<ClientJsonRpcMessage>> {
    let line = match line.strip_prefix(UTF8_BOM) {
        Some(rest) if first => rest,
        _ => line,
    };
    let line = line.trim_ascii();
    (!line.is_empty()).then(|| serde_json::from_slice(line))
}

/// 1 行＋LF を、書き手の鍵を持ったまま 1 度で書いて掃き出す（行の途中にほかの行が混ざらない）。
async fn write_line<W: AsyncWrite + Unpin>(writer: &Mutex<W>, mut line: Vec<u8>) -> io::Result<()> {
    line.push(b'\n');
    let mut writer = writer.lock().await;
    writer.write_all(&line).await?;
    writer.flush().await
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod tests;
