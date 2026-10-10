//! 自前の HTTP の送り手（`http.rs`）のテスト。
//!
//! 相手は 2 つ: 本物の受け口（`areka_mcp::start` を空きポートに起こす）と、台本で答える偽の受け口
//! （`crate::fake_server`）。どちらも OS が割り当てる番号で、机の 9801〜9830 へはつながない。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use areka_mcp::{MCP_PATH, McpServer, SERVER_NAME, ToolRegistry};
use http::{HeaderName, HeaderValue};
use rmcp::model::ClientJsonRpcMessage;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::{HttpError, OwnHttp, read_reply, verify};
use crate::config::Timeouts;
use crate::fake_server::{FakeServer, Log, Reply, Script};

/// 受け口が黙ったときにテストを止めずに赤にする上限。
const DEADLINE: Duration = Duration::from_secs(10);

/// 本物の受け口を空きポートに起こす（自分のスレッドと自分の tokio で回る）。
fn real_receiver() -> (McpServer, u16) {
    let server = areka_mcp::start(&[0], ToolRegistry::default(), None);
    let port = server
        .local_addr()
        .expect("空きポートで待ち受けている")
        .port();
    (server, port)
}

/// 手書きの `POST`（送り手の実装を通さない）。`close` なら `Connection: close` を付ける。
fn raw_post(port: u16, body: &Value, close: bool) -> Vec<u8> {
    let body = body.to_string();
    let connection = if close { "Connection: close\r\n" } else { "" };
    format!(
        "POST {MCP_PATH} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\nContent-Length: {}\r\n{connection}\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// 手書きの読み取り: 頭の終わりまでと、`Content-Length` の長さの本文だけを読む
/// （その先は読まないので、接続が続いているか閉じたかを後から見られる）。
async fn raw_reply(stream: &mut TcpStream) -> (String, Value) {
    let read = async {
        let mut raw = Vec::new();
        let head_end = loop {
            if let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                break at + 4;
            }
            let n = stream.read_buf(&mut raw).await.expect("頭を読める");
            assert_ne!(
                n,
                0,
                "頭の途中で閉じた: {:?}",
                String::from_utf8_lossy(&raw)
            );
        };
        let head = String::from_utf8_lossy(&raw[..head_end]).to_ascii_lowercase();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .unwrap_or_else(|| panic!("Content-Length が無い: {head}"))
            .trim()
            .parse()
            .expect("Content-Length が数字");
        while raw.len() < head_end + length {
            let n = stream.read_buf(&mut raw).await.expect("本文を読める");
            assert_ne!(n, 0, "本文の途中で閉じた");
        }
        assert_eq!(raw.len(), head_end + length, "応答の後ろに余りがある");
        let body = serde_json::from_slice(&raw[head_end..]).expect("本文は JSON");
        (head, body)
    };
    tokio::time::timeout(DEADLINE, read)
        .await
        .expect("応答が上限のうちに届く")
}

/// 設計 D-4 の根拠の固定: 本物の受け口は、1 本の接続で「確かめの `initialize`（札なし）」と
/// 「本当の要求」の 2 つに順に答え、`Connection: close` の要求に答えた後に接続を閉じる。
/// これが崩れたら、送り手を別々の接続に分けずに止めて設計へ戻す。
#[tokio::test]
async fn real_receiver_answers_two_requests_on_one_connection_then_closes() {
    let (_server, port) = real_receiver();
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("本物の受け口へ接続できる");

    // 1 つ目: 確かめの旧式の `initialize`。版の札なし・接続は持ち越す。
    let initialize = json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "areka-mcp-bridge", "version": "0"},
        },
    });
    stream
        .write_all(&raw_post(port, &initialize, false))
        .await
        .expect("書ける");
    let (head, body) = raw_reply(&mut stream).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert_eq!(body["result"]["serverInfo"]["name"], SERVER_NAME, "{body}");
    assert_eq!(body["id"], 0, "{body}");

    // 2 つ目: 同じ接続で本当の要求。`Connection: close` を付ける。
    let ping = json!({"jsonrpc": "2.0", "id": "real-1", "method": "ping"});
    stream
        .write_all(&raw_post(port, &ping, true))
        .await
        .expect("同じ接続に書ける");
    let (head, body) = raw_reply(&mut stream).await;
    assert!(head.starts_with("http/1.1 200"), "{head}");
    assert_eq!(
        body,
        json!({"jsonrpc": "2.0", "id": "real-1", "result": {}})
    );

    // 受け口の側から閉じる（読みが 0 バイトで戻る）。
    let mut rest = [0u8; 16];
    let n = tokio::time::timeout(DEADLINE, stream.read(&mut rest))
        .await
        .expect("受け口が上限のうちに閉じる")
        .expect("閉じるまで読める");
    assert_eq!(n, 0, "Connection: close の後に受け口が閉じなかった");
}

// ---- ここから下は送り手（`OwnHttp`）を通す ----

type Posted = Result<StreamableHttpPostResponse, StreamableHttpError<HttpError>>;

fn no_headers() -> HashMap<HeaderName, HeaderValue> {
    HashMap::new()
}

/// rmcp が呼ぶのと同じ口で 1 件送る。`uri` には別の機械を、セッションと認証には値を渡す:
/// 送り手はどれも使わず、自分の番号の `127.0.0.1` へつなぐ（要件 2.7）。
async fn post(port: u16, message: &Value, headers: HashMap<HeaderName, HeaderValue>) -> Posted {
    let http = OwnHttp {
        port,
        timeouts: Timeouts::default(),
    };
    let message: ClientJsonRpcMessage =
        serde_json::from_value(message.clone()).expect("要求の形として読める");
    http.post_message(
        "http://192.0.2.1:9/elsewhere".into(),
        message,
        Some("session".into()),
        Some("token".into()),
        headers,
    )
    .await
}

/// JSON の応答 1 件の中身（セッションの番号は付かない）。
fn json_of(posted: Posted) -> Value {
    match posted {
        Ok(StreamableHttpPostResponse::Json(message, None)) => {
            serde_json::to_value(&message).expect("応答は JSON にできる")
        }
        other => panic!("JSON の応答でない: {other:?}"),
    }
}

/// rmcp の手順が最初に送る形の `initialize`（名乗りは確かめの固定の本文と違うものにする）。
fn initialize(version: &str) -> Value {
    json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": version,
            "capabilities": {},
            "clientInfo": {"name": "handshake-of-the-test", "version": "0"},
        },
    })
}

fn call(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": "dump", "arguments": {}},
    })
}

/// 文を 1 つ持つツールの結果。
fn text_result(text: &str) -> Value {
    json!({"content": [{"type": "text", "text": text}]})
}

/// 接続が終わるまで待ってから、偽の受け口の記録を返す（送り手は要求ごとに 1 本だけ開く）。
async fn finished(fake: &FakeServer) -> Log {
    fake.wait(|log| log.connections.iter().any(|c| c.closed))
        .await
}

/// 本物の受け口へ、送り手ごしに届く: `id` 付きの要求は JSON の結果、通知は「受け取った」、
/// rmcp の手順の `initialize` は areka の名乗りの結果。
#[tokio::test]
async fn own_http_reaches_the_real_receiver() {
    let (_server, port) = real_receiver();

    let ping = json!({"jsonrpc": "2.0", "id": "p-1", "method": "ping"});
    assert_eq!(
        json_of(post(port, &ping, no_headers()).await),
        json!({"jsonrpc": "2.0", "id": "p-1", "result": {}})
    );

    let note = json!({"jsonrpc": "2.0", "method": "notifications/roots/list_changed"});
    let noted = post(port, &note, no_headers()).await;
    assert!(
        matches!(noted, Ok(StreamableHttpPostResponse::Accepted)),
        "{noted:?}"
    );

    let answer = json_of(post(port, &initialize("2025-06-18"), no_headers()).await);
    assert_eq!(answer["id"], 1, "{answer}");
    assert_eq!(
        answer["result"]["serverInfo"]["name"], SERVER_NAME,
        "{answer}"
    );
    assert_eq!(
        answer["result"]["protocolVersion"], "2025-06-18",
        "{answer}"
    );
}

/// 名乗りが合えば、確かめの `initialize` と本当の要求の 2 つが同じ接続で届き、応答が返る。
/// 本当の本文は意味を変えずに届く（要件 1.1）。
#[tokio::test]
async fn a_request_follows_the_identity_check_on_the_same_connection() {
    let fake = FakeServer::start(Script::areka());
    // `params` を書いておく: rmcp の型は `params` の無い `tools/list` を `"params": {}` 付きで
    // 書き出す（送り手に渡る前の話）ので、届いた本文と字面で比べられる形にする。
    let sent = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/list", "params": {}});

    let answer = json_of(post(fake.port(), &sent, no_headers()).await);
    assert_eq!(
        answer,
        json!({"jsonrpc": "2.0", "id": 7, "result": {"tools": []}})
    );

    let log = finished(&fake).await;
    assert_eq!(log.connections.len(), 1, "{log:?}");
    let received: Vec<_> = log
        .requests
        .iter()
        .map(|r| (r.method.as_str(), r.connection))
        .collect();
    assert_eq!(received, [("initialize", 0), ("tools/list", 0)], "{log:?}");
    let check = &log.requests[0].body;
    assert_eq!(check["params"]["clientInfo"]["name"], "areka-mcp-bridge");
    assert_eq!(check["params"]["protocolVersion"], "2025-11-25");
    assert_eq!(log.requests[1].body, sent);
}

/// 送る本文そのものが `initialize` なら、その答えの名乗りで確かめる（2 度送らない）。
#[tokio::test]
async fn an_initialize_message_is_its_own_identity_check() {
    let fake = FakeServer::start(Script::areka());
    let sent = initialize("2025-11-25");

    let answer = json_of(post(fake.port(), &sent, no_headers()).await);
    assert_eq!(
        answer["result"]["serverInfo"]["name"], SERVER_NAME,
        "{answer}"
    );

    let log = finished(&fake).await;
    assert_eq!(log.connections.len(), 1, "{log:?}");
    assert_eq!(log.requests.len(), 1, "{log:?}");
    assert_eq!(log.requests[0].body, sent);
    assert_eq!(log.requests[0].header("connection"), Some("close"));
}

/// 本文の長さの決まり方の 3 通り（`Content-Length`・chunked・閉じるまで）のどれでも、
/// 数 MiB の応答を欠けずに読む（要件 1.8）。
#[tokio::test]
async fn a_large_body_is_read_whole_by_each_of_the_three_framings() {
    let text = "areka の応答 ".repeat(3 * 1024 * 1024 / 16);
    assert!(text.len() >= 3 * 1024 * 1024);
    let result = text_result(&text);
    for reply in [
        Reply::Result(result.clone()),
        Reply::Chunked(result.clone()),
        Reply::UntilClose(result.clone()),
    ] {
        let fake = FakeServer::start(Script::areka().on("tools/call", reply));
        let answer = json_of(post(fake.port(), &call(json!(1)), no_headers()).await);
        assert_eq!(answer["id"], 1);
        let got = answer["result"]["content"][0]["text"].as_str();
        assert!(got == Some(text.as_str()), "the body is not whole");
    }
}

/// HTTP の状態が 400 でも、本文が JSON なら中身（areka の JSON-RPC のエラー）が返る（要件 4.3）。
#[tokio::test]
async fn a_json_body_comes_back_whatever_the_http_status() {
    let error = json!({"code": -32602, "message": "bad params", "data": {"why": "test"}});
    let script = Script::areka().on("tools/call", Reply::ErrorStatus(400, error.clone()));
    let fake = FakeServer::start(script);

    let answer = json_of(post(fake.port(), &call(json!("c-1")), no_headers()).await);
    assert_eq!(
        answer,
        json!({"jsonrpc": "2.0", "id": "c-1", "error": error})
    );
}

/// 通知に 202 が返れば「受け取った」。通知も、確かめの後に同じ接続で届く。
#[tokio::test]
async fn a_notification_answered_with_202_is_accepted() {
    let fake = FakeServer::start(Script::areka());
    let note = json!({"jsonrpc": "2.0", "method": "notifications/roots/list_changed"});

    let noted = post(fake.port(), &note, no_headers()).await;
    assert!(
        matches!(noted, Ok(StreamableHttpPostResponse::Accepted)),
        "{noted:?}"
    );

    let log = finished(&fake).await;
    let received: Vec<_> = log
        .requests
        .iter()
        .map(|r| (r.method.as_str(), r.connection))
        .collect();
    assert_eq!(
        received,
        [("initialize", 0), ("notifications/roots/list_changed", 0)],
        "{log:?}"
    );
}

/// 札: rmcp から渡る札は本当の要求にそのまま付き、確かめの `initialize` には付かない。
/// `Origin`・セッション・認証の札はどちらにも付かない。本当の要求だけが `Connection: close`。
#[tokio::test]
async fn rmcp_headers_go_on_the_real_request_only_and_origin_is_never_sent() {
    let fake = FakeServer::start(Script::areka());
    let from_rmcp = [
        ("mcp-protocol-version", "2025-11-25"),
        ("mcp-method", "tools/call"),
        ("mcp-name", "dump"),
    ];
    let headers = from_rmcp
        .iter()
        .map(|(n, v)| (HeaderName::from_static(n), HeaderValue::from_static(v)))
        .collect();

    json_of(post(fake.port(), &call(json!(1)), headers).await);

    let log = finished(&fake).await;
    let [check, real] = &log.requests[..] else {
        panic!("2 つの要求が届くはず: {log:?}");
    };
    let host = format!("127.0.0.1:{}", fake.port());
    for request in [check, real] {
        assert_eq!(request.header("host"), Some(host.as_str()));
        assert_eq!(request.header("content-type"), Some("application/json"));
        assert_eq!(
            request.header("accept"),
            Some("application/json, text/event-stream")
        );
        for absent in ["origin", "mcp-session-id", "authorization"] {
            assert_eq!(request.header(absent), None, "{absent}: {request:?}");
        }
    }
    for (name, value) in from_rmcp {
        assert_eq!(real.header(name), Some(value), "{name}: {real:?}");
        assert_eq!(check.header(name), None, "{name}: {check:?}");
    }
    assert_eq!(check.header("connection"), None, "{check:?}");
    assert_eq!(real.header("connection"), Some("close"), "{real:?}");
}

/// 探索が使う素の確かめは、相手の名乗りをそのまま返す（areka かどうかは呼び手が比べる）。
/// 届くのは確かめの `initialize` の 1 つだけ。
#[tokio::test]
async fn verify_returns_the_name_the_peer_gives() {
    for name in [SERVER_NAME, "ssp-mcp-server"] {
        let fake = FakeServer::start(Script::named(name));
        let stream = TcpStream::connect(("127.0.0.1", fake.port()))
            .await
            .expect("偽の受け口へ接続できる");

        let got = verify(stream, fake.port(), &Timeouts::default()).await;
        assert_eq!(got.expect("名乗りが返る"), name);

        let log = finished(&fake).await;
        assert_eq!(log.requests.len(), 1, "{log:?}");
        assert_eq!(log.requests[0].method, "initialize");
    }
}

/// セッションと SSE の口は、呼ばれない前提の決まった答え（areka は無状態・JSON の単発応答）。
#[tokio::test]
async fn session_and_sse_entries_answer_without_connecting() {
    // 番号 0 へはつなげない: つなぎに行けば、ここの答えが変わる。
    let http = OwnHttp {
        port: 0,
        timeouts: Timeouts::default(),
    };
    let uri: Arc<str> = "http://127.0.0.1:0/".into();
    let deleted = http
        .delete_session(uri.clone(), "session".into(), None, no_headers())
        .await;
    assert!(deleted.is_ok(), "{deleted:?}");
    let stream = http.get_stream(uri, None, None, None, no_headers()).await;
    assert!(matches!(
        stream,
        Err(StreamableHttpError::ServerDoesNotSupportSse)
    ));
}

/// 応答の読み取り（接続なし・バイト列から）: chunked の塊の拡張（`;…`）と後置の札を読み捨て、
/// 同じ接続の次の応答を、読みかけ（`buf`）を持ち越して続けて読める。
#[tokio::test]
async fn chunk_extensions_and_trailers_are_skipped_and_the_next_reply_follows() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Type: Application/JSON; charset=utf-8\r\n\
        Transfer-Encoding: chunked\r\n\r\n\
        4;name=value\r\n{\"a\"\r\nA\r\n:\"chunked\"\r\n1\r\n}\r\n0\r\nX-Trailer: 1\r\n\r\n\
        HTTP/1.1 400 Bad Request\r\nContent-Length: 4\r\n\r\nnext";
    let mut stream = &raw[..];
    let mut buf = Vec::new();

    let first = read_reply(&mut stream, &mut buf).await.expect("1 つ目");
    assert_eq!(first.status, 200);
    assert_eq!(first.content_type, "application/json");
    assert_eq!(first.body, br#"{"a":"chunked"}"#);

    let second = read_reply(&mut stream, &mut buf).await.expect("2 つ目");
    assert_eq!(second.status, 400);
    assert_eq!(second.content_type, "");
    assert_eq!(second.body, b"next");
    assert!(buf.is_empty() && stream.is_empty(), "余りが残った");
}
