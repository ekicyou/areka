//! 要求ごとの検査と振り分けと記録（gate → `(method, path)` の振り分け → rmcp／help／404・405）。
//!
//! rmcp の `StreamableHttpService` を呼ぶのも、HTTP の状態（`StatusCode`）を綴るのも
//! このモジュールだけ。本文の上限は [`MAX_BODY_BYTES`] 1 つを、ここでの読み取りと
//! rmcp の `with_max_request_body_bytes` の両方へ渡す（設計 B-7）。

use std::convert::Infallible;
use std::sync::Arc;

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::header::{ALLOW, CONTENT_TYPE, HOST, HeaderName, HeaderValue, ORIGIN};
use hyper::{Method, Request, Response, StatusCode};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;
use tower_service::Service;
use tracing::{debug, warn};

use crate::gate::{self, Reject};
use crate::handler::ArekaHandler;
use crate::help::help_html;

/// 本文の上限（4 MiB）。超えた要求は rmcp へ渡す前に 413 で返す。
pub(crate) const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

/// MCP の受け口のパス（method を問わず rmcp へ）。
const MCP_PATH: &str = "/api/mcp/v1";
/// 登録案内のパス（GET だけ）。
const HELP_PATH: &str = "/api/mcp/help";

/// 応答の型（rmcp の `StreamableHttpService` の応答と同じ）。
type HttpResponse = Response<BoxBody<Bytes, Infallible>>;

/// 要求 1 件に共有する不変の状態（`start` が 1 度だけ組む）。
pub(crate) struct State {
    /// help に載せる実番号。
    pub port: u16,
    /// 複製して `Service::call` で呼ぶ（`poll_ready` は常に Ready）。
    pub mcp: StreamableHttpService<ArekaHandler, NeverSessionManager>,
}

impl State {
    /// rmcp の設定をここ 1 か所で組む: 無状態（`legacy_session_mode(false)`・
    /// `NeverSessionManager`）・JSON の単発応答・本文の上限・取り消しの合図。
    /// `allowed_hosts` は既定（`localhost`・`127.0.0.1`・`::1`）のまま、`allowed_origins` は
    /// 空のまま（`Origin` の検査は手前の `gate` が持つ＝設計 B-1）。
    pub(crate) fn new(port: u16, handler: ArekaHandler, token: CancellationToken) -> Self {
        let config = StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .with_max_request_body_bytes(MAX_BODY_BYTES)
            .with_cancellation_token(token);
        let mcp = StreamableHttpService::new(
            move || Ok(handler.clone()),
            Arc::new(NeverSessionManager::default()),
            config,
        );
        Self { port, mcp }
    }
}

/// 要求 1 件を「検査 → 振り分け → 記録」で処理する。失敗しない（拒否も応答で表す）。
pub(crate) async fn handle(
    state: Arc<State>,
    req: Request<Incoming>,
) -> Result<HttpResponse, Infallible> {
    let path = req.uri().path().to_owned();
    // 検査は path によらず最初に（要件 4.3・6.3）。値が文字列として読めなくても
    // 「無し」へ倒さず、置換文字入りの値として gate に拒ませる。
    let header = |name: HeaderName| {
        req.headers()
            .get(name)
            .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
    };
    let (origin, host) = (header(ORIGIN), header(HOST));
    let (response, rpc) = match gate::check(origin.as_deref(), host.as_deref()) {
        Err(reject) => {
            match &reject {
                Reject::Origin(value) => {
                    warn!(origin = %value, path = %path, "MCP: Origin を拒んだ")
                }
                Reject::Host(value) => {
                    warn!(host = value.as_deref().unwrap_or("-"), path = %path, "MCP: Host を拒んだ")
                }
            }
            (plain(StatusCode::FORBIDDEN), None)
        }
        Ok(()) => match (req.method(), path.as_str()) {
            (_, MCP_PATH) => forward(&state, req).await,
            (&Method::GET, HELP_PATH) => (help(state.port), None),
            (_, HELP_PATH) => {
                let mut response = plain(StatusCode::METHOD_NOT_ALLOWED);
                response
                    .headers_mut()
                    .insert(ALLOW, HeaderValue::from_static("GET"));
                (response, None)
            }
            _ => (plain(StatusCode::NOT_FOUND), None),
        },
    };
    let (method, id) = rpc.unwrap_or_else(|| ("-".to_owned(), "-".to_owned()));
    debug!(
        method = %method,
        id = %id,
        status = response.status().as_u16(),
        path = %path,
        "MCP: 要求に応えた"
    );
    Ok(response)
}

/// 本文を上限まで集めて rmcp へ渡す。戻りの 2 つ目は覗いた `(method, id)`（集められなければ None）。
async fn forward(
    state: &State,
    req: Request<Incoming>,
) -> (HttpResponse, Option<(String, String)>) {
    let (parts, body) = req.into_parts();
    let bytes = match Limited::new(body, MAX_BODY_BYTES).collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) if err.is::<LengthLimitError>() => {
            return (plain(StatusCode::PAYLOAD_TOO_LARGE), None);
        }
        // 本文の途中でクライアントが切れたなど（応答は届かないが記録は残る）。
        Err(_) => return (plain(StatusCode::BAD_REQUEST), None),
    };
    let rpc = peek(&bytes);
    let Ok(response) = state
        .mcp
        .clone()
        .call(Request::from_parts(parts, Full::new(bytes)))
        .await;
    (response, Some(rpc))
}

/// 本文から `method` と `id` を寛容に覗く（JSON の object でなければ・欄が無ければ `-`）。
/// 本文の解釈そのものは rmcp に任せる（ここは記録のためだけ）。
fn peek(body: &[u8]) -> (String, String) {
    let value: Option<serde_json::Value> = serde_json::from_slice(body).ok();
    let field = |name| value.as_ref().and_then(|v| v.get(name));
    let method = field("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-")
        .to_owned();
    let id = field("id").map_or_else(|| "-".to_owned(), ToString::to_string);
    (method, id)
}

/// 登録案内（200・`text/html; charset=utf-8`）。
fn help(port: u16) -> HttpResponse {
    with_body(
        StatusCode::OK,
        "text/html; charset=utf-8",
        help_html(port).into(),
    )
}

/// 状態の理由句だけを本文にした平文の応答（403／404／405／413／400）。
fn plain(status: StatusCode) -> HttpResponse {
    let reason = status.canonical_reason().unwrap_or("");
    with_body(
        status,
        "text/plain; charset=utf-8",
        Bytes::from_static(reason.as_bytes()),
    )
}

fn with_body(status: StatusCode, content_type: &'static str, body: Bytes) -> HttpResponse {
    let mut response = Response::new(Full::new(body).boxed());
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}
