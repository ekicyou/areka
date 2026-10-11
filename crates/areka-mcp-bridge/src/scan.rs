//! 候補の列から areka を 1 つ選ぶ探索。
//!
//! 候補の全部へ TCP の接続を並べて開き、開いた接続を候補の順に確かめ（`http::verify`）にかけて、
//! 最初に areka と名乗った 1 つで決める。あとの候補の接続は 1 バイトも書かずに閉じる（設計 D-5）。
//! 決めた番号には、自前の送り手（`OwnHttp`）の上に rmcp のクライアントを立てる（設計 D-3）。

use std::io;
use std::net::Ipv4Addr;
use std::time::Duration;

use areka_mcp::{MCP_PATH, SERVER_NAME};
use futures::future::join_all;
use rmcp::service::{RoleClient, RunningService, serve_client};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

use crate::config::Timeouts;
use crate::http::{HttpError, OwnHttp, verify};
use crate::service::BridgeClient;

/// areka とつながった印: areka の側へ立てた rmcp のクライアント 1 本。`peer_info()` が areka の
/// サーバーの情報（名乗り・できること・説明の文）を持つ。
pub type Link = RunningService<RoleClient, BridgeClient>;

pub enum Scan {
    Found {
        link: Link,
        port: u16,
    },
    /// `others` は、areka でない名乗りが応えた番号と名乗り（候補の順）。名乗りを読めなかった相手・
    /// 返事をしない相手・開かなかった番号は入らない（理由は標準エラーにだけ残る）。
    NotFound {
        others: Vec<(u16, String)>,
    },
}

/// 候補の列（空でない）から areka を探す。`Found` のとき、`port` より前の候補には確かめの
/// `initialize` しか届いておらず、後ろの候補には接続を開いただけで 1 バイトも書いていない。
/// 決めた番号で rmcp の手順が失敗したら（確かめの直後に相手が居なくなった など）、見つからなかった
/// ものとして返す（次の要求がもう 1 度探す）。
pub async fn scan(candidates: &[u16], timeouts: &Timeouts) -> Scan {
    let mut others = Vec::new();
    let Some(port) = pick(candidates, timeouts, &mut others).await else {
        return Scan::NotFound { others };
    };
    match stand(port, timeouts).await {
        Some(link) => Scan::Found { link, port },
        None => Scan::NotFound { others },
    }
}

/// areka と名乗った最初の候補の番号。飛ばした候補は、理由を標準エラーに残す（誰かが応えた番号は
/// `info`、開かなかった番号は `debug`＝areka が居ない間は要求のたびに 20 個近く出るため）。
/// 戻るときに、残りの接続は何も書かないまま閉じる。
async fn pick(
    candidates: &[u16],
    timeouts: &Timeouts,
    others: &mut Vec<(u16, String)>,
) -> Option<u16> {
    // 並べて開く: 居ない番号への接続は断られるまで約 2 秒かかる机があるので、順には待たない。
    // 全部の結果が出てから確かめるので、areka が先頭に居ても、開かない番号があれば上限 1 回分は待つ。
    let opening = candidates.iter().map(|&port| open(port, timeouts.connect));
    let opened = join_all(opening).await;
    for (&port, stream) in candidates.iter().zip(opened) {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                debug!(port, %error, "[bridge] skipped a candidate: not open");
                continue;
            }
        };
        match verify(stream, port, timeouts).await {
            Ok(name) if name == SERVER_NAME => return Some(port),
            Ok(name) if !name.is_empty() => {
                // 名乗りは相手の書いた字。制御文字が記録の行を割らない形で出す。
                info!(
                    port,
                    ?name,
                    "[bridge] skipped a candidate: it answered as another server"
                );
                others.push((port, name));
            }
            Ok(_) | Err(HttpError::NotAreka { .. }) => {
                info!(
                    port,
                    "[bridge] skipped a candidate: no readable server name in its answer"
                );
            }
            Err(error) => info!(port, %error, "[bridge] skipped a candidate"),
        }
    }
    None
}

/// `127.0.0.1` の番号へ接続を開く。上限 `wait` を過ぎたら `TimedOut`。
async fn open(port: u16, wait: Duration) -> io::Result<TcpStream> {
    let connect = TcpStream::connect((Ipv4Addr::LOCALHOST, port));
    match tokio::time::timeout(wait, connect).await {
        Ok(result) => result,
        Err(_) => Err(io::ErrorKind::TimedOut.into()),
    }
}

/// 番号に rmcp のクライアントを立てる（旧式の `initialize` → `notifications/initialized`）。
/// 手順の `initialize` も送り手を通るので、ここでも名乗りが確かめられる（設計 D-4）。
/// 失敗は、中身を標準エラーに残して `None`。
async fn stand(port: u16, timeouts: &Timeouts) -> Option<Link> {
    let http = OwnHttp {
        port,
        timeouts: timeouts.clone(),
    };
    // 送り手は宛先を自分の番号から組むので、この URI は rmcp の設定を埋めるだけ。
    let uri = format!("http://127.0.0.1:{port}{MCP_PATH}");
    let config = StreamableHttpClientTransportConfig::with_uri(uri);
    let transport = StreamableHttpClientTransport::with_client(http, config);
    match serve_client(BridgeClient, transport).await {
        Ok(link) => Some(link),
        Err(error) => {
            // 失敗の中身には相手の書いた字が入りうるので、制御文字が記録の行を割らない形で出す。
            warn!(
                port,
                ?error,
                "[bridge] areka answered the identity check, but the client could not start"
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
