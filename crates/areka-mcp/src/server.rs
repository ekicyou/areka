//! 待受の束ね・`mcp` スレッドの起こし・`Drop` で畳む取っ手。
//!
//! 束ねは呼び出し側のスレッドで同期に行う（起動の記録が呼び出し側で出る＝要件 9.3）。
//! tokio の `current_thread` ランタイムは `mcp` スレッドの中だけで作って回し、
//! 取っ手 [`McpServer`] の `Drop` で取り消して畳む（設計 B-3）。

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use areka_actor::{ActorHandle, spawn_actor};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::dispatch::{self, State};
use crate::handler::ArekaHandler;
use crate::registry::ToolRegistry;

/// 取っ手の `Drop` が `mcp` スレッドの終わりを待つ上限（`perf_thread_report` の `FINAL_WAIT` と同じ考え）。
pub const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);
/// `accept` が失敗したとき次を試すまでの間（`warn!` の嵐と CPU の空回りにしない）。
pub(crate) const ACCEPT_RETRY_WAIT: Duration = Duration::from_millis(100);

/// 待受を立てる。失敗しない（束ねの失敗は `error!` と「待ち受けない取っ手」で表す）。
///
/// `port`: None＝待ち受けない（`AREKA_MCP_PORT=0`）。Some(0)＝OS に空きポートを割り当てさせる（テスト用）。
pub fn start(port: Option<u16>, registry: ToolRegistry) -> McpServer {
    let Some(port) = port else {
        info!("MCP: 待ち受けない（AREKA_MCP_PORT が 0）");
        return McpServer { listening: None };
    };
    let bound = std::net::TcpListener::bind(("127.0.0.1", port))
        .and_then(|listener| listener.set_nonblocking(true).map(|()| listener))
        .and_then(|listener| listener.local_addr().map(|addr| (listener, addr)));
    let (listener, addr) = match bound {
        Ok(bound) => bound,
        Err(err) => {
            error!(port, error = %err, "MCP: ポートを束ねられなかった（待ち受けない）");
            return McpServer { listening: None };
        }
    };
    let token = CancellationToken::new();
    let state = Arc::new(State::new(
        addr.port(),
        ArekaHandler::new(registry),
        token.clone(),
    ));
    let (done_tx, done_rx) = mpsc::channel();
    let thread_token = token.clone();
    // inbox は使わない（取り消しは token で伝える）。送信端は取っ手が持って落とすだけ。
    let (inbox, thread) = spawn_actor::<(), _>("mcp", move |_inbox| {
        run(listener, thread_token, state, done_tx)
    });
    info!(url = %format!("http://{addr}/api/mcp/v1"), "MCP: 待受を始めた");
    McpServer {
        listening: Some(Listening {
            addr,
            token,
            done: done_rx,
            _inbox: inbox,
            _thread: thread,
        }),
    }
}

/// 待受の取っ手。落とすと待受が閉じる。
#[must_use = "落とすと待受が閉じる"]
pub struct McpServer {
    /// None＝待ち受けていない。
    listening: Option<Listening>,
}

struct Listening {
    addr: SocketAddr,
    token: CancellationToken,
    /// `mcp` スレッドが畳み終えた合図。
    done: mpsc::Receiver<()>,
    _inbox: Sender<()>,
    _thread: ActorHandle,
}

impl McpServer {
    /// 待受中なら束ねた実番号つきの番地（`Some(0)` で立てたときも実番号）。
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.listening.as_ref().map(|l| l.addr)
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        // 待ち受けていない取っ手は何もしない。
        let Some(listening) = self.listening.take() else {
            return;
        };
        listening.token.cancel();
        match listening.done.recv_timeout(SHUTDOWN_WAIT) {
            // Disconnected＝スレッドが既に終わっている（ランタイムを作れなかった後など）。
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                info!(addr = %listening.addr, "MCP: 待受を閉じた")
            }
            // 待ちきれなければ切り離す（終了の手順を止めない＝要件 1.8）。
            Err(RecvTimeoutError::Timeout) => warn!(
                addr = %listening.addr,
                wait_ms = SHUTDOWN_WAIT.as_millis() as u64,
                "MCP: 待受の終わりを待ちきれなかった（切り離して先へ進む）"
            ),
        }
    }
}

/// `mcp` スレッドの本体: ランタイムを作り、取り消されるまで受け、畳んで合図する。
fn run(
    listener: std::net::TcpListener,
    token: CancellationToken,
    state: Arc<State>,
    done: Sender<()>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            // 待受の口はここで閉じる。本体は止めない（done は落ちて Disconnected になる）。
            error!(error = %err, "MCP: ランタイムを作れなかった（待受を閉じる）");
            return;
        }
    };
    // 取り消されると受付のフューチャごと listener が落ちる＝受付の口を OS へ返す。
    runtime.block_on(token.run_until_cancelled(accept_loop(listener, state)));
    // 開いた接続のタスクは待たずに捨てる（要件 1.8）。
    runtime.shutdown_background();
    let _ = done.send(());
}

/// 接続を受けて、接続ごとに hyper の http1 で `dispatch::handle` を回す。
async fn accept_loop(listener: std::net::TcpListener, state: Arc<State>) {
    let listener = match tokio::net::TcpListener::from_std(listener) {
        Ok(listener) => listener,
        Err(err) => {
            error!(error = %err, "MCP: 受付の口をランタイムへ渡せなかった（待受を閉じる）");
            return;
        }
    };
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |req| dispatch::handle(state.clone(), req));
                    if let Err(err) = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(TokioIo::new(stream), service)
                        .await
                    {
                        // クライアントの切断・不正な HTTP。雑音にしないが記録は残す。
                        debug!(error = %err, "MCP: 接続が途中で終わった");
                    }
                });
            }
            Err(err) => {
                // ループは殺さない。間を置いて続ける。
                warn!(error = %err, "MCP: 接続を受けられなかった（間を置いて続ける）");
                tokio::time::sleep(ACCEPT_RETRY_WAIT).await;
            }
        }
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod server_tests;

#[cfg(test)]
#[path = "server_protocol_tests.rs"]
mod server_protocol_tests;

#[cfg(test)]
#[path = "server_gate_help_tests.rs"]
mod server_gate_help_tests;
