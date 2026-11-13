//! テスト専用の部品（手書きの HTTP/1.1 クライアント・JSON-RPC の組み立て・サーバの起こし口）。
//!
//! クライアントは `areka-update` の `winhttp_real_tests::answer` の逆向き: 要求 1 本を
//! `Connection: close` で送り、サーバが閉じるまで読んでから状態・ヘッダ・本文に分ける。
//! ネットへは出ない（`127.0.0.1` の空きポートだけ＝要件 9.1）。

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::tools::ToolRequest;
use crate::{McpServer, ToolRegistry, start};

/// MCP の受け口のパス（テストの側で契約を綴る）。
pub(crate) const MCP_V1: &str = "/api/mcp/v1";
/// help のパス。
pub(crate) const HELP: &str = "/api/mcp/help";
/// rmcp が `POST` に求める `Accept`（両方が無いと 406＝設計 B-8）。
pub(crate) const RPC_ACCEPT: &str = "application/json, text/event-stream";
/// 無状態版の版。
pub(crate) const STATELESS: &str = "2026-07-28";
/// 保存した SSP の定義（テストだけが読む。本番はクレートの外のファイルを読まない＝要件 7.6）。
const SAVED_LIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../doc/ssp-mcp/tools-list-ssp-2.9.05.json"
);
/// 受け手の要求を待つ上限。届かなければテストを止めずに失敗させる（正しさは待ち時間に頼らない）。
pub(crate) const RECV_WAIT: Duration = Duration::from_secs(10);
/// 返事の上限。待ちの間の `ping` のテストが上限で終わらないよう長く取る（受け手は必ず答える）。
pub(crate) const LONG_WAIT: Duration = Duration::from_secs(30);
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

    /// 200 で JSON 1 件の応答を読み、`result` を返す（`error` があれば失敗）。
    pub(crate) fn result(&self) -> Value {
        assert_eq!(self.status, 200, "{self:?}");
        let value = self.json();
        assert!(value.get("error").is_none(), "{value}");
        value["result"].clone()
    }

    /// 200 で JSON 1 件の応答を読み、`error` を返す（`result` があれば失敗）。
    pub(crate) fn error(&self) -> Value {
        assert_eq!(self.status, 200, "{self:?}");
        let value = self.json();
        assert!(value.get("result").is_none(), "{value}");
        value["error"].clone()
    }
}

/// JSON のオブジェクトの中身を取り出す（オブジェクトでなければ失敗）。
pub(crate) fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        other => panic!("オブジェクトでない: {other}"),
    }
}

/// 空きポート（候補 `&[0]`）でサーバを起こし、取っ手と実番号つきの番地を返す。
pub(crate) fn serve(registry: ToolRegistry) -> (McpServer, SocketAddr) {
    let server = start(&[0], registry, None);
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

/// 保存した SSP の定義（[`SAVED_LIST`]）を JSON 1 件として読む。
pub(crate) fn saved_list() -> Value {
    let text = std::fs::read_to_string(SAVED_LIST).expect("保存した JSON が読めない");
    serde_json::from_str(&text).expect("保存した JSON が JSON でない")
}

/// 旧式の `initialize` の `params`（`protocolVersion` だけを変える）。
pub(crate) fn initialize_params(version: &str) -> Value {
    json!({
        "protocolVersion": version,
        "capabilities": {},
        "clientInfo": { "name": "areka-test", "version": "0" }
    })
}

/// 無状態版の要求に付ける本文の `_meta`。
pub(crate) fn stateless_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": STATELESS,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "areka-test", "version": "0" }
    })
}

/// 旧式の経路（ヘッダも `_meta` も無い）で 1 件送る。
pub(crate) fn post_legacy(
    addr: SocketAddr,
    method: &str,
    id: u64,
    params: Option<Value>,
) -> Response {
    post_rpc(addr, &[], &rpc(method, Some(id), params))
}

/// 無状態版の経路で 1 件送る（`MCP-Protocol-Version`・`Mcp-Method`・`tools/call` なら `Mcp-Name`・本文の `_meta`）。
pub(crate) fn post_stateless(addr: SocketAddr, method: &str, id: u64, params: Value) -> Response {
    let mut params = params;
    params["_meta"] = stateless_meta();
    let name = params["name"].as_str().map(str::to_owned);
    let mut headers = vec![("MCP-Protocol-Version", STATELESS), ("Mcp-Method", method)];
    if let Some(name) = name.as_deref() {
        headers.push(("Mcp-Name", name));
    }
    post_rpc(addr, &headers, &rpc(method, Some(id), Some(params)))
}

/// 両方の経路で同じ要求を送る（経路の名前, 応答）。旧式は id 1、無状態版は id 2。
/// `params` が None なら旧式は欄ごと省き、無状態版は `{}`。
pub(crate) fn post_both(
    addr: SocketAddr,
    method: &str,
    params: Option<Value>,
) -> [(&'static str, Response); 2] {
    [
        ("旧式", post_legacy(addr, method, 1, params.clone())),
        (
            "無状態版",
            post_stateless(addr, method, 2, params.unwrap_or_else(|| json!({}))),
        ),
    ]
}

/// `tools/call` の `params`。
pub(crate) fn call_params(name: &str, arguments: Value) -> Value {
    json!({ "name": name, "arguments": arguments })
}

/// 受け手に要求が 1 件も届いていない（検査で弾いた要求はアプリ本体へ届かない）。
/// 検査は handler を呼ぶ前に同期で済み、応答はその後に返るので、応答を受けた後に覗けば十分。
pub(crate) fn assert_no_request(rx: &Receiver<ToolRequest>) {
    match rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        Ok(request) => panic!("受け手に要求が届いた: {:?}", request.call),
        Err(TryRecvError::Disconnected) => panic!("受け口が閉じている"),
    }
}

/// 登録表と受け口の組 `entrances`（`all_entrances(..)` の戻り値）のサーバへ、旧式と無状態版の
/// 両方の経路で同じ `tools/call` を送り、`-32602` の `error` と HTTP の状態を固定する。
/// どちらも `result` は無く、受け手への要求は 0 件。
pub(crate) fn assert_invalid_params(
    entrances: (ToolRegistry, Receiver<ToolRequest>),
    name: &str,
    arguments: Value,
    message: &str,
) {
    let (registry, rx) = entrances;
    let (_server, addr) = serve(registry);
    let params = call_params(name, arguments);
    // 旧式の経路も無状態版の経路も 200（rmcp 3.9.0 で測った値。3.5.0 の無状態版は 400 だった＝差の一覧へ）。
    for (path, response) in post_both(addr, "tools/call", Some(params)) {
        assert_eq!(response.status, 200, "{path}: {response:?}");
        let value = response.json();
        assert!(value.get("result").is_none(), "{path}: {value}");
        assert_eq!(
            value["error"],
            json!({ "code": -32602, "message": message }),
            "{path}: {value}"
        );
    }
    assert_no_request(&rx);
}

/// 要求 1 本を送って応答を受ける。
///
/// 足すのは `Host`（`headers` に無ければ `127.0.0.1:<port>`）・`Content-Length`・
/// `Connection: close` だけ。送りと読みは [`send_raw`] に任せる。
pub(crate) fn request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Response {
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
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    send_raw(addr, &bytes, false)
}

/// 組み上がった要求のバイト列をそのまま送り、close_write なら書きの側を閉じてから、
/// サーバが閉じるまで読んで分ける（ヘッダの値に UTF-8 でないバイトを入れる・本文を約束より短く切る、のため）。
///
/// 書き込みの失敗は無視して読みに進む（本文の上限を超えた要求に、サーバが読み切る前に
/// 413 を返して閉じることがある）。
pub(crate) fn send_raw(addr: SocketAddr, raw: &[u8], close_write: bool) -> Response {
    let mut stream = TcpStream::connect(addr).expect("ループバックへ接続できる");
    stream.set_read_timeout(Some(READ_TIMEOUT)).unwrap();
    stream.set_write_timeout(Some(READ_TIMEOUT)).unwrap();
    let _ = stream.write_all(raw);
    if close_write {
        let _ = stream.shutdown(Shutdown::Write);
    }
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
