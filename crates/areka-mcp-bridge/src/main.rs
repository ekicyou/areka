#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! 中継の実行ファイルの入口（spec: areka-P0-mcp-stdio-bridge）。
//!
//! 引数は読まない（つなぐ先を変える手段は `AREKA_MCP_PORT` だけ＝要件 2.8）。標準出力へ書くのは
//! 中継が返す応答の行だけで、記録は全部標準エラーへ出す（要件 5.1・5.2）。リリースのビルドは
//! 窓を持たない（要件 5.3）。標準入出力が無い起こし方では、標準入力がすぐ終わりになって終わる。

use std::io;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use areka_mcp_bridge::config::Config;
use areka_mcp_bridge::service::run;
use futures::future::{Either, select};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::Notify;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

/// `RUST_LOG` が無い（読めない）ときの絞り。rmcp 自身の記録は `error` だけにする: `info` は
/// つないだ相手の情報の全部と届いた通知の本文を出し、`warn` はエラーで答えるたびに中身を出す
/// （areka が居ないときの答え・`AREKA_MCP_PORT=0` の答えのたびに 1 行。理由は中継が自分の行で残す）。
const DEFAULT_FILTER: &str = "info,rmcp=error";

/// 標準入力が閉じてから終わるまでを待つ上限。rmcp が書きかけの応答を書く 5 秒より少し長い。
const EXIT_WAIT: Duration = Duration::from_secs(6);

/// 標準入力。終わり（読めたのが 0 バイト・読みの失敗）を見たら `ended` へ知らせる。
struct Stdin {
    inner: tokio::io::Stdin,
    ended: Arc<Notify>,
}

impl AsyncRead for Stdin {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let polled = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(read) = &polled
            && (read.is_err() || buf.filled().len() == before)
        {
            self.ended.notify_one();
        }
        polled
    }
}

fn main() -> io::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    // 色の制御文字は付けない（Desktop は標準エラーをそのまま記録ファイルに書く）。
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_ansi(false)
        .with_env_filter(filter)
        .init();

    let config = Config::from_env();
    if config.candidates.is_empty() {
        info!("[bridge] AREKA_MCP_PORT=0: the bridge will not look for areka");
    } else {
        info!(ports = ?config.candidates, "[bridge] candidates on 127.0.0.1");
    }
    // 控えを持たないときの理由の 1 件は `Config::from_env`（の内側の `from_inputs`）が出している。
    if let Some(dir) = &config.cache_dir {
        info!(dir = %dir.display(), "[bridge] cache dir");
    }
    let waits = &config.timeouts;
    info!(connect = ?waits.connect, verify = ?waits.verify, reply = ?waits.reply, "[bridge] timeouts");

    // tokio はこの 1 本のスレッドだけで回す（控えの書きが同じプロセスの中で重ならない）。
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(serve(config));
    // 相手が読まない標準出力への書きが残っていても、待たずに終わる（終了コード 0）。
    runtime.shutdown_background();
    Ok(())
}

/// 標準入力が閉じるまで中継を回す。閉じると rmcp が書きかけの応答を 5 秒まで書いてから戻る
/// （要件 1.9）。相手が標準入力だけ閉じて標準出力を読まないと、詰まった書きを待って rmcp が
/// 戻らないので、閉じてから [`EXIT_WAIT`] で見切る。
async fn serve(config: Config) {
    let ended = Arc::new(Notify::new());
    let stdin = Stdin {
        inner: tokio::io::stdin(),
        ended: ended.clone(),
    };
    let overdue = async {
        ended.notified().await;
        tokio::time::sleep(EXIT_WAIT).await;
    };
    match select(pin!(run(config, stdin, tokio::io::stdout())), pin!(overdue)).await {
        Either::Left((quit, _)) => info!(?quit, "[bridge] finished"),
        Either::Right(_) => {
            warn!("[bridge] stdout is still not read {EXIT_WAIT:?} after stdin closed; quitting")
        }
    }
}
