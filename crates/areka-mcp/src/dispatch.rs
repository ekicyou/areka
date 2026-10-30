//! 要求ごとの検査と振り分けと記録（gate → `(method, path)` の振り分け → rmcp／help／404・405）。
//!
//! rmcp の `StreamableHttpService` を呼ぶのも、HTTP の状態（`StatusCode`）を綴るのも
//! このモジュールだけ。本文の上限は [`MAX_BODY_BYTES`] 1 つを、ここでの読み取りと
//! rmcp の `with_max_request_body_bytes` の両方へ渡す（設計 B-7）。

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::header::{ALLOW, CONTENT_TYPE, HOST, HeaderName, HeaderValue, ORIGIN};
use hyper::{Method, Request, Response, StatusCode};
use rmcp::model::{ErrorCode, ErrorData, JsonRpcError, ProtocolVersion, RequestId};
use rmcp::transport::common::http_header::HEADER_MCP_PROTOCOL_VERSION;
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
pub const MCP_PATH: &str = "/api/mcp/v1";
/// 登録案内のパス（GET だけ）。
const HELP_PATH: &str = "/api/mcp/help";

/// 応答の型（rmcp の `StreamableHttpService` の応答と同じ）。
type HttpResponse = Response<BoxBody<Bytes, Infallible>>;

/// 要求 1 件に共有する不変の状態（`start` が 1 度だけ組む）。
pub(crate) struct State {
    /// help に載せる実番号。
    pub port: u16,
    /// help の Claude Desktop の設定例に載せる中継の置き場（None＝場所が取れなかった）。
    pub bridge_exe: Option<PathBuf>,
    /// 複製して `Service::call` で呼ぶ（`poll_ready` は常に Ready）。
    pub mcp: StreamableHttpService<ArekaHandler, NeverSessionManager>,
}

impl State {
    /// rmcp の設定をここ 1 か所で組む: 無状態（`legacy_session_mode(false)`・
    /// `NeverSessionManager`）・JSON の単発応答・本文の上限・取り消しの合図。
    /// `allowed_hosts` は既定（`localhost`・`127.0.0.1`・`::1`）のまま、`allowed_origins` は
    /// 空のまま（`Origin` の検査は手前の `gate` が持つ＝設計 B-1）。
    pub(crate) fn new(
        port: u16,
        bridge_exe: Option<PathBuf>,
        handler: ArekaHandler,
        token: CancellationToken,
    ) -> Self {
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
        Self {
            port,
            bridge_exe,
            mcp,
        }
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
            (&Method::GET, HELP_PATH) => (help(&state), None),
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
    let header = |name| parts.headers.get(name).and_then(|v| v.to_str().ok());
    let protocol_version = header(HEADER_MCP_PROTOCOL_VERSION).map(str::to_owned);
    // rmcp と同じ見方。JSON でない `Content-Type` の 415 は今のまま（要件 6.7）。
    let json_body =
        header(CONTENT_TYPE.as_str()).is_some_and(|v| v.starts_with("application/json"));
    let Ok(response) = state
        .mcp
        .clone()
        .call(Request::from_parts(parts, Full::new(bytes.clone())))
        .await;
    // rmcp が読めた要求は rmcp の答えのまま。415 のときだけ本文を読み直す（要件 6.3・6.7）。
    if !json_body || response.status() != StatusCode::UNSUPPORTED_MEDIA_TYPE {
        return (response, Some(rpc));
    }
    let shape_error = serde_json::from_slice(&bytes)
        .ok()
        .and_then(|raw| request_shape_error(&raw));
    let Some(err) = shape_error else {
        return (response, Some(rpc));
    };
    let status = shape_error_status(protocol_version.as_deref(), err.error.code);
    let body = serde_json::to_vec(&err).expect("JsonRpcError は JSON に書ける");
    (
        with_body(status, "application/json", body.into()),
        Some(rpc),
    )
}

/// rmcp が 415 で断る本文のうち、理由が `params` の形だけのものへの答え（中継も使う）。
///
/// JSON-RPC 2.0 の要求（`jsonrpc` が `"2.0"`・`method` が文字列・`id` が数か文字列）で、
/// `params` が在り `null` でもオブジェクトでもないときだけ `Some`。数・文字列・真偽値は
/// `-32600`、配列は `-32602`。メソッドの名前は見ない。答えは rmcp の公開の型で、`id` は要求のまま。
pub fn request_shape_error(raw: &serde_json::Value) -> Option<JsonRpcError> {
    use serde_json::Value;
    if raw.get("jsonrpc")?.as_str()? != "2.0" || !raw.get("method")?.is_string() {
        return None;
    }
    let id: RequestId = serde_json::from_value(raw.get("id")?.clone()).ok()?;
    let error = match raw.get("params")? {
        Value::Null | Value::Object(_) => return None,
        Value::Array(_) => {
            ErrorData::invalid_params("Invalid params: params must be an object", None)
        }
        _ => ErrorData::invalid_request(
            "Invalid Request: params must be an array or an object",
            None,
        ),
    };
    Some(JsonRpcError::new(Some(id), error))
}

/// rmcp の `jsonrpc_http_status`（非公開）の写しのうち、`request_shape_error` の 2 つの番号の分。
/// `2026-07-28` 以降（rmcp と同じ文字列の比較）の `-32602` だけ 400、ほかは 200（旧式の版は rmcp も 200）。
/// rmcp の版を上げたら、rmcp 自身の `-32602` の状態と突き合わせるテストで確かめる（要件 6.4）。
fn shape_error_status(protocol_version: Option<&str>, code: ErrorCode) -> StatusCode {
    if protocol_version.is_some_and(|v| v >= ProtocolVersion::V_2026_07_28.as_str())
        && code == ErrorCode::INVALID_PARAMS
    {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::OK
    }
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
fn help(state: &State) -> HttpResponse {
    with_body(
        StatusCode::OK,
        "text/html; charset=utf-8",
        help_html(state.port, state.bridge_exe.as_deref()).into(),
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

#[cfg(test)]
#[path = "dispatch_shape_tests.rs"]
mod dispatch_shape_tests;
