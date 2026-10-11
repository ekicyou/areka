//! 自前の HTTP/1.1 の送り手（接続ごとに相手を確かめてから送る）。
//!
//! rmcp の HTTP のクライアントの口（`StreamableHttpClient`）を、tokio の `TcpStream` と `httparse` で
//! 満たす。要求ごとに `127.0.0.1` の番号へ接続を開き、その接続の最初の要求として旧式の
//! `initialize` を送って名乗りを読み、areka と名乗った相手にだけ、同じ接続で本当の本文を送る
//! （設計 D-4）。接続は持ち越さない（本当の要求は `Connection: close`）。
//!
//! 失敗は「書く前」（本当の本文を 1 バイトも書いていない）と「書いた後」（届いたかもしれない）に
//! 分かれる（[`HttpError::written`]）。応答の上限の時間は持たない（呼び手の rmcp が要求ごとに持つ）。

use std::collections::HashMap;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use areka_mcp::{MCP_PATH, SERVER_NAME};
use futures::stream::BoxStream;
use http::{HeaderName, HeaderValue};
use rmcp::model::{ClientJsonRpcMessage, ClientRequest, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_client::{
    SseError, StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
};
use serde_json::{Value, json};
use sse_stream::Sse;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

use crate::config::Timeouts;

const JSON: &str = "application/json";
const EVENT_STREAM: &str = "text/event-stream";
/// `NotJson` に持つ本文の頭の長さ（バイト）。
const HEAD_BYTES: usize = 256;

/// 送り手。宛先は `127.0.0.1` の `port` だけ（変える設定は無い）。
#[derive(Clone)]
pub struct OwnHttp {
    pub port: u16,
    pub timeouts: Timeouts,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    // 書く前（要求は誰にも届いていない）
    #[error("connect failed: {0}")]
    Connect(std::io::Error),
    /// 名乗りが areka でない。名乗りを読めなかった相手（HTTP でない・JSON でない・途中で切れた）は
    /// `name` が空。
    #[error("peer on port {port} is {name:?}, not areka")]
    NotAreka { port: u16, name: String },
    #[error("no answer to the identity check within {0:?}")]
    VerifyTimeout(Duration),
    // 書いた後（届いたかもしれない）
    #[error("no response (empty body or connection closed)")]
    NoResponse,
    #[error("HTTP {status}, body is not JSON: {head}")]
    NotJson { status: u16, head: String },
}

impl HttpError {
    /// 本当の本文を書いた後の失敗か（`true` なら届いたかもしれないので、送り直さない）。
    pub fn written(&self) -> bool {
        matches!(self, HttpError::NoResponse | HttpError::NotJson { .. })
    }
}

impl StreamableHttpClient for OwnHttp {
    type Error = HttpError;

    /// `uri`・`session_id`・`auth_header` は使わない: 宛先は自分の `port` から組み（要件 2.7）、
    /// areka は無状態で認証も無い。`custom_headers`（rmcp が組んだ版などの札）はそのまま付ける。
    async fn post_message(
        &self,
        _uri: Arc<str>,
        message: ClientJsonRpcMessage,
        _session_id: Option<Arc<str>>,
        _auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<HttpError>> {
        let initialize = matches!(
            &message,
            ClientJsonRpcMessage::Request(r) if matches!(r.request, ClientRequest::InitializeRequest(_))
        );
        let body = serde_json::to_vec(&message)?;
        self.post(initialize, &body, &custom_headers)
            .await
            .map_err(StreamableHttpError::Client)
    }

    /// areka は無状態なので、rmcp はこれを呼ばない。
    async fn delete_session(
        &self,
        _uri: Arc<str>,
        _session_id: Arc<str>,
        _auth_header: Option<String>,
        _custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<HttpError>> {
        Ok(())
    }

    /// セッションが無ければ、rmcp はこれを呼ばない（SSE は読まない＝設計 D-16）。
    async fn get_stream(
        &self,
        _uri: Arc<str>,
        _session_id: Option<Arc<str>>,
        _last_event_id: Option<String>,
        _auth_header: Option<String>,
        _custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<HttpError>> {
        Err(StreamableHttpError::ServerDoesNotSupportSse)
    }
}

impl OwnHttp {
    /// 接続を開き、相手を確かめてから、同じ接続で本文 1 件を送って応答を読む。
    async fn post(
        &self,
        initialize: bool,
        body: &[u8],
        headers: &HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, HttpError> {
        let port = self.port;
        let connect = TcpStream::connect((Ipv4Addr::LOCALHOST, port));
        let mut stream = match tokio::time::timeout(self.timeouts.connect, connect).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(error)) => return Err(HttpError::Connect(error)),
            Err(_) => return Err(HttpError::Connect(io::ErrorKind::TimedOut.into())),
        };
        let mut buf = Vec::new();
        let real = request(port, body, headers, true);
        let wait = self.timeouts.verify;
        let reply = if initialize {
            // rmcp の手順の `initialize` は、その答えの名乗りで確かめる（2 度送らない）。
            let (name, reply) = ask_name(&mut stream, &mut buf, &real, port, wait).await?;
            expect_areka(port, name)?;
            reply
        } else {
            let check = check_request(port);
            let (name, _) = ask_name(&mut stream, &mut buf, &check, port, wait).await?;
            expect_areka(port, name)?;
            // ここから先は「書いた後」: 本当の本文が届いたかもしれない。
            match exchange(&mut stream, &mut buf, &real).await {
                Ok(reply) => reply,
                Err(error) => {
                    debug!(port, %error, "[bridge] the answer to the request did not arrive whole");
                    return Err(HttpError::NoResponse);
                }
            }
        };
        debug!(
            port,
            status = reply.status,
            bytes = reply.body.len(),
            "[bridge] got an answer"
        );
        classify(reply)
    }
}

/// 探索が使う素の確かめ: 開いてある接続で固定の `initialize` を送り、相手の名乗りを返す
/// （areka かどうかは呼び手が比べる）。
pub async fn verify(
    mut stream: TcpStream,
    port: u16,
    timeouts: &Timeouts,
) -> Result<String, HttpError> {
    let check = check_request(port);
    let (name, _) = ask_name(&mut stream, &mut Vec::new(), &check, port, timeouts.verify).await?;
    Ok(name)
}

/// 名乗りの確かめの 1 往復（固定の `initialize` でも、rmcp の手順の `initialize` でも同じ）:
/// 要求を書いて応答を読み、`result.serverInfo.name` を返す。待つ上限は `wait`。
/// 名乗りを読めなかった相手は、空の名乗りの `NotAreka`。
async fn ask_name(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    request: &[u8],
    port: u16,
    wait: Duration,
) -> Result<(String, Reply), HttpError> {
    let nameless = || HttpError::NotAreka {
        port,
        name: String::new(),
    };
    let reply = match tokio::time::timeout(wait, exchange(stream, buf, request)).await {
        Ok(Ok(reply)) => reply,
        Ok(Err(error)) => {
            debug!(port, %error, "[bridge] the peer did not answer the identity check with HTTP");
            return Err(nameless());
        }
        Err(_) => return Err(HttpError::VerifyTimeout(wait)),
    };
    let answer: Value = serde_json::from_slice(&reply.body).unwrap_or_default();
    match answer["result"]["serverInfo"]["name"].as_str() {
        Some(name) => Ok((name.to_owned(), reply)),
        None => {
            debug!(
                port,
                status = reply.status,
                "[bridge] the answer to the identity check has no serverInfo.name"
            );
            Err(nameless())
        }
    }
}

fn expect_areka(port: u16, name: String) -> Result<(), HttpError> {
    if name == SERVER_NAME {
        Ok(())
    } else {
        Err(HttpError::NotAreka { port, name })
    }
}

/// 相手を確かめる固定の旧式の `initialize`（札なし・接続は持ち越す）。
fn check_request(port: u16) -> Vec<u8> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": "identity-check",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION")},
        },
    });
    request(port, body.to_string().as_bytes(), &HashMap::new(), false)
}

/// `POST` 1 件のバイト列。`Origin` は付けない（areka の検査を通る）。`close` なら、答えた後に
/// 受け口が接続を閉じる。
fn request(
    port: u16,
    body: &[u8],
    headers: &HashMap<HeaderName, HeaderValue>,
    close: bool,
) -> Vec<u8> {
    let mut out = format!(
        "POST {MCP_PATH} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: {JSON}\r\n\
         Accept: {JSON}, {EVENT_STREAM}\r\nContent-Length: {}\r\n",
        body.len()
    )
    .into_bytes();
    if close {
        out.extend_from_slice(b"Connection: close\r\n");
    }
    for (name, value) in headers {
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    out
}

/// 応答 1 件。`content_type` は小文字の種類だけ（`;` より後ろは落とす・無ければ空）。
struct Reply {
    status: u16,
    content_type: String,
    body: Vec<u8>,
}

/// 応答を rmcp の口の答えにする。HTTP の状態は、202（受け取った）のほかは見ない（要件 4.3）。
fn classify(reply: Reply) -> Result<StreamableHttpPostResponse, HttpError> {
    let Reply {
        status,
        content_type,
        body,
    } = reply;
    if status == 202 {
        return Ok(StreamableHttpPostResponse::Accepted);
    }
    if content_type == EVENT_STREAM {
        let head = format!("content-type {EVENT_STREAM}");
        return Err(HttpError::NotJson { status, head });
    }
    if body.is_empty() {
        return Err(HttpError::NoResponse);
    }
    let head = || String::from_utf8_lossy(&body[..body.len().min(HEAD_BYTES)]).into_owned();
    if content_type != JSON {
        let head = head();
        return Err(HttpError::NotJson { status, head });
    }
    match serde_json::from_slice::<ServerJsonRpcMessage>(&body) {
        Ok(message) => Ok(StreamableHttpPostResponse::Json(message, None)),
        Err(error) => {
            debug!(status, %error, "[bridge] the JSON body is not a JSON-RPC message");
            let head = head();
            Err(HttpError::NotJson { status, head })
        }
    }
}

/// 要求 1 件を書き、応答 1 件を読む。`buf` はこの接続の読みかけ。
async fn exchange(stream: &mut TcpStream, buf: &mut Vec<u8>, request: &[u8]) -> io::Result<Reply> {
    stream.write_all(request).await?;
    read_reply(stream, buf).await
}

/// 本文の長さの決まり方。
enum Framing {
    Length(usize),
    Chunked,
    UntilClose,
}

struct Head {
    /// 頭のバイトの数（本文はこの後ろ）。
    len: usize,
    status: u16,
    content_type: String,
    framing: Framing,
}

/// 応答 1 件を読む。本文は `Content-Length`・chunked・閉じるまで、の 3 通りで欠けずに読む
/// （大きさの上限なし＝要件 1.8）。202 と `text/event-stream` は本文を読まない（SSE の本文は
/// 終わらないことがある）。途中で閉じたら `UnexpectedEof`、HTTP として読めなければ `InvalidData`。
async fn read_reply<R: AsyncRead + Unpin>(stream: &mut R, buf: &mut Vec<u8>) -> io::Result<Reply> {
    let head = loop {
        if let Some(head) = parse_head(buf)? {
            break head;
        }
        fill(stream, buf, buf.len() + 1).await?;
    };
    buf.drain(..head.len);
    let body = if head.status == 202 || head.content_type == EVENT_STREAM {
        Vec::new()
    } else {
        match head.framing {
            Framing::Length(length) => {
                fill(stream, buf, length).await?;
                let rest = buf.split_off(length);
                std::mem::replace(buf, rest)
            }
            Framing::Chunked => read_chunked(stream, buf).await?,
            Framing::UntilClose => {
                while stream.read_buf(buf).await? != 0 {}
                std::mem::take(buf)
            }
        }
    };
    Ok(Reply {
        status: head.status,
        content_type: head.content_type,
        body,
    })
}

/// 応答の頭を解く。まだ揃っていなければ `None`。
fn parse_head(buf: &[u8]) -> io::Result<Option<Head>> {
    let mut slots = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut slots);
    let httparse::Status::Complete(len) = response.parse(buf).map_err(invalid)? else {
        return Ok(None);
    };
    let header = |name: &str| {
        let found = response
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))?;
        Some(
            String::from_utf8_lossy(found.value)
                .trim()
                .to_ascii_lowercase(),
        )
    };
    let framing = if header("transfer-encoding").is_some_and(|v| v.contains("chunked")) {
        Framing::Chunked
    } else if let Some(length) = header("content-length") {
        Framing::Length(length.parse().map_err(invalid)?)
    } else {
        Framing::UntilClose
    };
    let content_type = header("content-type").unwrap_or_default();
    let content_type = content_type.split(';').next().unwrap_or_default().trim();
    Ok(Some(Head {
        len,
        status: response.code.unwrap_or_default(),
        content_type: content_type.to_owned(),
        framing,
    }))
}

/// chunked の本文を解く（塊の拡張 `;…` と後置の札は読み捨てる）。
async fn read_chunked<R: AsyncRead + Unpin>(
    stream: &mut R,
    buf: &mut Vec<u8>,
) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line = read_line(stream, buf).await?;
        let size = line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size, 16).map_err(invalid)?;
        if size == 0 {
            // 後置の札（あれば）を、空の行まで読み捨てる。
            while !read_line(stream, buf).await?.is_empty() {}
            return Ok(body);
        }
        // 塊の後ろの CRLF まで。
        let end = size
            .checked_add(2)
            .ok_or_else(|| invalid("chunk size overflow"))?;
        fill(stream, buf, end).await?;
        body.extend_from_slice(&buf[..size]);
        buf.drain(..end);
    }
}

/// 行末までの 1 行（行末は含めない）を `buf` から取り出す。
async fn read_line<R: AsyncRead + Unpin>(stream: &mut R, buf: &mut Vec<u8>) -> io::Result<String> {
    loop {
        if let Some(at) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=at).collect();
            return Ok(String::from_utf8_lossy(&line).trim_end().to_owned());
        }
        fill(stream, buf, buf.len() + 1).await?;
    }
}

/// `buf` が `len` バイト以上になるまで読む。その前に相手が閉じたら `UnexpectedEof`。
async fn fill<R: AsyncRead + Unpin>(
    stream: &mut R,
    buf: &mut Vec<u8>,
    len: usize,
) -> io::Result<()> {
    while buf.len() < len {
        if stream.read_buf(buf).await? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
    }
    Ok(())
}

fn invalid(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
