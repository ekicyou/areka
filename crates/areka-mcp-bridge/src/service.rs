//! 届いた要求への答えを決める判断と、中継を回す入口。
//!
//! Desktop の側は rmcp のサービスを直接実装し、最初の問い合わせを飛ばして回す（設計 D-2）。
//! `initialize`・`server/discover`・`ping` もここへ届き、どの要求が失敗しても次の行を読み続ける。

use std::sync::Arc;

use areka_mcp::SERVER_NAME;
use rmcp::model::{
    CacheScope, ClientCapabilities, ClientConfig, ClientNotification, ClientRequest,
    DiscoverResult, ErrorData, GetExtensions, Implementation, ListToolsResult, ProtocolVersion,
    RequestMetaObject, ResultType, ServerCapabilities, ServerConfig, ServerPeerInfo, ServerResult,
};
use rmcp::service::{
    NotificationContext, PeerRequestOptions, QuitReason, RequestContext, serve_directly,
};
use rmcp::{ClientHandler, RoleServer, Service, ServiceError};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::cache::Cache;
use crate::config::Config;
use crate::errors;
use crate::scan::{Link, Scan, scan};
use crate::stdio::LineTransport;

/// `AREKA_MCP_PORT=0`（待ち受けない設定）のときのエラーの理由（設計 D-9）。
const DISABLED: &str = "AREKA_MCP_PORT=0 (areka does not listen)";
/// 「居ない」エラーの文に載せる、areka でない相手の名乗りの長さの上限（字の数）。
const PEER_NAME_MAX: usize = 64;

/// areka の側へ立てる rmcp のクライアントの持ち主（設計 D-3）。名乗りは `areka-mcp-bridge` と
/// 中継の版。areka から届く要求・通知には rmcp の既定で応える（areka は送ってこない）。
pub struct BridgeClient;

impl ClientHandler for BridgeClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
    }
}

/// つないだ先: areka の側へ立てた rmcp のクライアントと、その番号。
pub struct Linked {
    pub link: Link,
    pub port: u16,
}

/// Desktop から届いた要求への答えを決める。
pub struct Bridge {
    config: Config,
    cache: Cache,
    /// つないだ先。`None` は、まだ探していないか、探して居なかった。要求は `Arc` を写して使い、
    /// 鍵は届ける前に放す（長いツールの間も、ほかの要求は待たない＝設計 D-13）。
    link: Mutex<Option<Arc<Linked>>>,
}

impl Bridge {
    pub fn new(config: Config) -> Bridge {
        let cache = Cache::new(config.cache_dir.clone());
        Bridge {
            config,
            cache,
            link: Mutex::new(None),
        }
    }

    /// つないだ先。無ければ候補を探し、見つかれば覚えて、areka のサーバーの情報を控えに書く
    /// （要件 3.1）。探す間は鍵を持つので、同時に届いた要求が探索を 2 度走らせない。
    /// 居なければ、areka でない名乗りが応えた番号と名乗りを返す。
    async fn ensure_link(&self) -> Result<Arc<Linked>, Vec<(u16, String)>> {
        let mut slot = self.link.lock().await;
        if let Some(linked) = &*slot {
            return Ok(linked.clone());
        }
        match scan(&self.config.candidates, &self.config.timeouts).await {
            Scan::Found { link, port } => {
                info!(port, "[bridge] connected to areka");
                if let Some(info) = link.peer_info() {
                    self.cache.save_server_info(&info);
                }
                let linked = Arc::new(Linked { link, port });
                *slot = Some(linked.clone());
                Ok(linked)
            }
            Scan::NotFound { others } => Err(others),
        }
    }

    /// つながっているときの答え。`initialize`・`server/discover` は areka へ送らず、つないだときに
    /// 得た areka の情報で答える（設計 D-8。Desktop の `initialize` を areka へ流さないことは、
    /// 送り手の確かめの前提でもある＝設計 D-4）。ほかは areka へ届ける。
    async fn answer(
        &self,
        linked: &Linked,
        request: ClientRequest,
        meta: RequestMetaObject,
    ) -> Result<ServerResult, ErrorData> {
        if !matches!(
            request,
            ClientRequest::InitializeRequest(_) | ClientRequest::DiscoverRequest(_)
        ) {
            return self.forward(linked, request, meta).await;
        }
        let info = linked.link.peer_info();
        let from_info = |info| first_answer(info, &request);
        let Some(answer) = info.as_deref().and_then(from_info) else {
            warn!(
                port = linked.port,
                "[bridge] the link holds no server info of areka"
            );
            return Err(errors::no_response("areka gave no server info"));
        };
        debug!(
            port = linked.port,
            "[bridge] answered the first inquiry from the server info of areka"
        );
        Ok(answer)
    }

    /// areka が居ないときの答え（設計「areka が居ないときの答え」の表）。最初の問い合わせと
    /// `tools/list` は、当たる控えがあれば控えで答える（要件 3.4）。`ping` は自分で空の結果
    /// （要件 3.7）。ほかの要求と、当たる控えが無い問い合わせには「居ない」エラー（要件 3.5・3.6）。
    fn absent(
        &self,
        request: &ClientRequest,
        meta: &RequestMetaObject,
        others: &[(u16, String)],
    ) -> Result<ServerResult, ErrorData> {
        let cached = match request {
            ClientRequest::PingRequest(_) => return Ok(ServerResult::empty(())),
            ClientRequest::InitializeRequest(_) | ClientRequest::DiscoverRequest(_) => {
                let info = self.cache.load_server_info();
                info.and_then(|info| first_answer(&info, request))
            }
            ClientRequest::ListToolsRequest(_) => {
                let tools = self.cache.load_tools();
                tools.map(|tools| ServerResult::ListToolsResult(in_dialect_of(tools, meta)))
            }
            _ => None,
        };
        let method = request.method();
        if let Some(answer) = cached {
            info!(method, "[bridge] areka not found; answered from the cache");
            return Ok(answer);
        }
        let reason = not_found(&self.config.candidates, others);
        info!(method, "[bridge] {reason}; answered with an error");
        Err(errors::server_not_available(&reason))
    }

    /// 要求を areka へ届け、応答の上限つきで待って、結果をそのまま返す。要求の `_meta` は rmcp が
    /// 外して渡してくるので、付け直して届ける（`2026-07-28` で話す Desktop の要求は、rmcp の
    /// クライアントがその版の札を付けて、areka にもその版の要求として届く＝要件 1.10）。
    /// `tools/list` のエラーでない結果は、返す前に控えに書く（要件 3.2）。
    async fn forward(
        &self,
        linked: &Linked,
        request: ClientRequest,
        meta: RequestMetaObject,
    ) -> Result<ServerResult, ErrorData> {
        let options = PeerRequestOptions::with_timeout(self.config.timeouts.reply).with_meta(meta);
        let answer = match linked.link.send_request_with_option(request, options).await {
            Ok(sent) => sent.await_response().await,
            Err(error) => Err(error),
        };
        match answer {
            Ok(result) => {
                if let ServerResult::ListToolsResult(tools) = &result {
                    self.cache.save_tools(tools);
                }
                Ok(result)
            }
            // areka の JSON-RPC のエラーは、番号・文・データを変えずに返す（要件 4.3）。
            Err(ServiceError::McpError(error)) => {
                debug!(
                    port = linked.port,
                    code = error.code.0,
                    "[bridge] areka answered with an error"
                );
                Err(error)
            }
            Err(error) => {
                warn!(port = linked.port, %error, "[bridge] the request to areka failed");
                Err(errors::no_response(&error.to_string()))
            }
        }
    }
}

impl Service<RoleServer> for Bridge {
    async fn handle_request(
        &self,
        request: ClientRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, ErrorData> {
        // ⑴ 待ち受けない設定（候補が空）: 探さず、控えも使わず、`ping` にもエラー（要件 2.6）。
        if self.config.candidates.is_empty() {
            return Err(errors::server_not_available(DISABLED));
        }
        // ⑵ つなぐ先を決める（無ければ探す。居ないと答えた後も、要求のたびに探す＝要件 3.9）。
        match self.ensure_link().await {
            // ⑶ つながっている。
            Ok(linked) => self.answer(&linked, request, context.meta).await,
            // ⑷ 居ない。
            Err(others) => self.absent(&request, &context.meta, &others),
        }
    }

    /// つながっていれば、`notifications/initialized` と取り消しの通知のほかを届ける（設計 D-10）。
    /// 失敗は記録だけ。つなぐ先が無ければ何もしない（通知のためには探さない）。標準出力には
    /// 何も出ない。
    async fn handle_notification(
        &self,
        mut notification: ClientNotification,
        context: NotificationContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        // `notifications/initialized` は、rmcp の手順が areka へ自分の分を送っている。
        // 取り消しの通知の `requestId` は Desktop の番号で、areka が見るのは中継の側のクライアントが
        // 振り直した番号だけ。届けると、rmcp のクライアントが同じ番号で待っている自分の別の要求を
        // 取り消しで終わらせる。
        if matches!(
            notification,
            ClientNotification::InitializedNotification(_)
                | ClientNotification::CancelledNotification(_)
        ) {
            return Ok(());
        }
        let Some(linked) = self.link.lock().await.clone() else {
            return Ok(());
        };
        // rmcp が外して渡してきた付きもの（`_meta` はこの中）を付け直す。
        *notification.extensions_mut() = context.extensions;
        if let Err(error) = linked.link.send_notification(notification).await {
            warn!(port = linked.port, %error, "[bridge] the notification to areka failed");
        }
        Ok(())
    }

    /// rmcp が自分の側の設定に使うだけの値（Desktop へは出ない）。
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
    }
}

/// 最初の問い合わせ（`initialize`・`server/discover`）の答えを、areka のサーバーの情報から組む。
/// つながっているときは areka の今の情報から、居ないときは控えの情報から、同じに組む（設計 D-8）。
/// 名乗りの無い情報・最初の問い合わせでない要求からは組まない。
fn first_answer(info: &ServerPeerInfo, request: &ClientRequest) -> Option<ServerResult> {
    let mut config =
        ServerConfig::new(info.capabilities.clone()).with_server_info(info.server_info.clone()?);
    config.instructions = info.instructions.clone();
    config.meta = info.meta.clone();
    match request {
        ClientRequest::InitializeRequest(request) => Some(ServerResult::InitializeResult(
            config.with_protocol_version(negotiated(request.params.protocol_version.clone())),
        )),
        ClientRequest::DiscoverRequest(_) => Some(ServerResult::DiscoverResult(
            DiscoverResult::from_server_info(ProtocolVersion::KNOWN_VERSIONS.to_vec(), config),
        )),
        _ => None,
    }
}

/// 控えの一覧を、要求の話し方に合わせる。控えは最後に書いたときの話し方の形のまま（`2026-07-28` なら
/// `resultType`・`ttlMs`・`cacheScope` 付き、旧式なら無し）なので、areka の受け口が版ごとに返す形に
/// 揃える: `2026-07-28` 以降の要求（`_meta` に版がある）には 3 欄を付け（無いとクライアントが拒む）、
/// 旧式の要求からは外す。`ttlMs` は 0 なので、クライアントは次も聞き直す（areka が見つかれば今の一覧に
/// 替わる）。
fn in_dialect_of(mut tools: ListToolsResult, meta: &RequestMetaObject) -> ListToolsResult {
    let version = meta.protocol_version();
    let stateless = version.is_some_and(|v| v >= ProtocolVersion::V_2026_07_28);
    tools.result_type = stateless.then_some(ResultType::COMPLETE);
    tools.ttl_ms = stateless.then_some(0);
    tools.cache_scope = stateless.then_some(CacheScope::Private);
    tools
}

/// 「居ない」エラーの理由の文（設計 D-9）: 候補の番号を候補の順に並べ、areka でない名乗りが応えた番号と
/// 名乗りを続ける（要件 3.6）。
fn not_found(candidates: &[u16], others: &[(u16, String)]) -> String {
    let ports: Vec<String> = candidates.iter().map(u16::to_string).collect();
    let mut reason = format!(
        "{SERVER_NAME} not found on 127.0.0.1 ports {}",
        ports.join(", ")
    );
    for (port, name) in others {
        // 名乗りは相手が書いた字。長い文・行の分かれた文を Desktop と記録へ流し込ませない。
        let plain = |c: char| if c.is_control() { '?' } else { c };
        let shown: String = name.chars().take(PEER_NAME_MAX).map(plain).collect();
        let cut = if name.chars().nth(PEER_NAME_MAX).is_some() {
            "..."
        } else {
            ""
        };
        reason.push_str(&format!("; port {port} answered as \"{shown}{cut}\""));
    }
    reason
}

/// `initialize` に返す版（設計 D-2・areka の受け口と同じ決まり）: 要求の版が `initialize` を持つ
/// 既知の版ならその版、ほか（`2026-07-28`・知らない版）は `2025-11-25`。
fn negotiated(requested: ProtocolVersion) -> ProtocolVersion {
    if requested.has_initialize() && ProtocolVersion::KNOWN_VERSIONS.contains(&requested) {
        requested
    } else {
        ProtocolVersion::LATEST_WITH_INITIALIZE
    }
}

/// 中継を回す入口: 標準入出力（テストはメモリ上の管）の上で、入力が終わるまで要求に答える。
pub async fn run<R, W>(config: Config, stdin: R, stdout: W) -> QuitReason
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let transport = LineTransport::new(stdin, stdout);
    let running = serve_directly(Bridge::new(config), transport, None);
    running
        .waiting()
        .await
        .unwrap_or_else(QuitReason::JoinError)
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
