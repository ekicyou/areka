//! テスト専用の部品（手書きの HTTP/1.1 クライアント・JSON-RPC の組み立て・サーバの起こし口）。
//!
//! クライアントは `areka-update` の `winhttp_real_tests::answer` の逆向き: 要求 1 本を
//! `Connection: close` で送り、サーバが閉じるまで読んでから状態・ヘッダ・本文に分ける。
//! ネットへは出ない（`127.0.0.1` の空きポートだけ＝要件 9.1）。

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::{McpServer, ToolRegistry, start};

/// MCP の受け口のパス（テストの側で契約を綴る）。
pub(crate) const MCP_V1: &str = "/api/mcp/v1";
/// rmcp が `POST` に求める `Accept`（両方が無いと 406＝設計 B-8）。
pub(crate) const RPC_ACCEPT: &str = "application/json, text/event-stream";
/// 読み取りの上限。サーバが黙ったらテストを止めずに失敗させる。
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// 応答 1 件（`headers` は受けた順・名前は受けたままの綴り）。
#[derive(Debug)]
pub(crate) struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// ヘッダの値（名前は大文字小文字を問わない・最初の 1 つ）。
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// 本文を JSON 1 件として読む（読めなければ本文を添えて失敗）。
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or_else(|err| {
            panic!(
                "本文が JSON でない（{err}）: {}",
                String::from_utf8_lossy(&self.body)
            )
        })
    }
}

/// 空きポート（`Some(0)`）でサーバを起こし、取っ手と実番号つきの番地を返す。
pub(crate) fn serve(registry: ToolRegistry) -> (McpServer, SocketAddr) {
    let server = start(Some(0), registry);
    let addr = server.local_addr().expect("空きポートで待ち受けている");
    (server, addr)
}

/// JSON-RPC の要求 1 件を組む。`id` が None なら通知、`params` が None なら欄ごと省く。
pub(crate) fn rpc(method: &str, id: Option<u64>, params: Option<serde_json::Value>) -> String {
    let mut value = serde_json::json!({ "jsonrpc": "2.0", "method": method });
    if let Some(id) = id {
        value["id"] = id.into();
    }
    if let Some(params) = params {
        value["params"] = params;
    }
    value.to_string()
}

/// rmcp を呼ぶ要求: `POST /api/mcp/v1` に `Content-Type: application/json` と
/// [`RPC_ACCEPT`] を既定で付ける。`extra` はその後ろに足す（`Origin` など）。
/// `Accept` を外す・`Content-Type` を変えるテストは [`request`] を直に使う。
pub(crate) fn post_rpc(addr: SocketAddr, extra: &[(&str, &str)], body: &str) -> Response {
    let mut headers = vec![("Content-Type", "application/json"), ("Accept", RPC_ACCEPT)];
    headers.extend_from_slice(extra);
    request(addr, "POST", MCP_V1, &headers, body.as_bytes())
}

/// 要求 1 本を送って応答を受ける。
///
/// 足すのは `Host`（`headers` に無ければ `127.0.0.1:<port>`）・`Content-Length`・
/// `Connection: close` だけ。書き込みの失敗は無視して読みに進む（本文の上限を超えた
/// 要求に、サーバが読み切る前に 413 を返して閉じることがある）。
pub(crate) fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Response {
    let mut stream = TcpStream::connect(addr).expect("ループバックへ接続できる");
    stream.set_read_timeout(Some(READ_TIMEOUT)).unwrap();
    stream.set_write_timeout(Some(READ_TIMEOUT)).unwrap();
    let mut head = format!("{method} {path} HTTP/1.1\r\n");
    if !headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("host")) {
        head.push_str(&format!("Host: {addr}\r\n"));
    }
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body));
    parse(&read_until_closed(&mut stream))
}

/// サーバが閉じるまで読む。途中で切られても（接続のリセット）、受けた分があればそれを使う。
fn read_until_closed(stream: &mut TcpStream) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                panic!(
                    "応答が {READ_TIMEOUT:?} のうちに終わらなかった（受けた分 {} バイト）",
                    raw.len()
                )
            }
            Err(err) if raw.is_empty() => panic!("応答を 1 バイトも受けられなかった: {err}"),
            Err(_) => break,
        }
    }
    raw
}

/// 状態行・ヘッダ・本文に分ける（本文は `Transfer-Encoding: chunked` を解き、
/// `Content-Length` があればその長さで切る）。
fn parse(raw: &[u8]) -> Response {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("ヘッダの終わりが無い: {}", String::from_utf8_lossy(raw)));
    let head = std::str::from_utf8(&raw[..split]).expect("ヘッダが UTF-8");
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("状態行が読めない: {status_line}"));
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(n, v)| (n.trim().to_owned(), v.trim().to_owned()))
        .collect();
    let mut response = Response {
        status,
        headers,
        body: Vec::new(),
    };
    response.body = if response
        .header("transfer-encoding")
        .is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
    {
        dechunk(rest)
    } else if let Some(len) = response.header("content-length") {
        let len: usize = len.parse().expect("Content-Length が数字");
        assert!(rest.len() >= len, "本文が短い: {} < {len}", rest.len());
        rest[..len].to_vec()
    } else {
        rest.to_vec()
    };
    response
}

/// `Transfer-Encoding: chunked` の本文を解く（拡張 `;…` と後置ヘッダは読み捨てる）。
fn dechunk(mut rest: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    loop {
        let eol = rest
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("チャンクの長さの行が終わっている");
        let size_line = std::str::from_utf8(&rest[..eol]).expect("チャンクの長さが ASCII");
        let size_hex = size_line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_hex, 16).expect("チャンクの長さが 16 進");
        rest = &rest[eol + 2..];
        if size == 0 {
            return body;
        }
        body.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

/// 自己点検: 空きポートで起こしたサーバへ `ping` を送り、200 と `result: {}` を受ける。
#[test]
fn self_check_ping_on_ephemeral_port() {
    let (_server, addr) = serve(ToolRegistry::default());
    let response = post_rpc(addr, &[], &rpc("ping", Some(1), None));
    assert_eq!(response.status, 200, "{response:?}");
    let value = response.json();
    assert_eq!(value["id"], 1);
    assert_eq!(value["result"], serde_json::json!({}));
}
