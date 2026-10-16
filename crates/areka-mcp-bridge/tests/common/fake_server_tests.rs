//! 偽の受け口の自己確認。部品のテストの束（`src/lib.rs`）でだけ走る。

use std::time::{Duration, Instant};

use rmcp::model::{ServerJsonRpcMessage, ServerResult};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use areka_mcp::{MCP_PATH, SERVER_NAME};

use crate::fake_server::*;

async fn connect(port: u16) -> TcpStream {
    TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("偽の受け口へ接続")
}

/// 手書きの POST を 1 つ書く。
async fn send(stream: &mut TcpStream, body: &Value, close: bool) {
    let body = body.to_string();
    let connection = if close { "Connection: close\r\n" } else { "" };
    let request = format!(
        "POST {MCP_PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         {connection}Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("要求を書く");
}

/// 応答の頭まで読む。戻りは状態・頭（小文字）・頭の後ろに既に届いていた分。
async fn receive_head(stream: &mut TcpStream) -> (u16, String, Vec<u8>) {
    let mut buf = Vec::new();
    let head_len = loop {
        if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        let n = stream.read_buf(&mut buf).await.expect("応答を読む");
        assert_ne!(n, 0, "応答の頭の途中で閉じた");
    };
    let rest = buf.split_off(head_len);
    let head = String::from_utf8(buf).expect("応答の頭").to_lowercase();
    (head[9..12].parse().expect("状態"), head, rest)
}

/// 応答を 1 つ読む。本文は長さの札があればその長さまで（先に閉じたらそこまで）、無ければ閉じるまで。
async fn receive(stream: &mut TcpStream) -> (u16, String, Vec<u8>) {
    let (status, head, mut body) = receive_head(stream).await;
    match promised_length(&head) {
        Some(length) => {
            while body.len() < length && stream.read_buf(&mut body).await.expect("本文を読む") > 0
            {
            }
        }
        None => {
            stream.read_to_end(&mut body).await.expect("閉じるまで読む");
        }
    }
    (status, head, body)
}

fn promised_length(head: &str) -> Option<usize> {
    let line = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length: "))?;
    Some(line.parse().expect("長さの札"))
}

/// 新しい接続で `id` 1 の要求を 1 つ送り（`Connection: close`）、応答を読む。
async fn ask(port: u16, method: &str) -> (u16, String, Vec<u8>) {
    let mut stream = connect(port).await;
    send(
        &mut stream,
        &json!({"jsonrpc": "2.0", "id": 1, "method": method}),
        true,
    )
    .await;
    receive(&mut stream).await
}

fn parsed(body: &[u8]) -> Value {
    serde_json::from_slice(body).expect("JSON の本文")
}

/// chunked の本文を解く。戻りは中身と塊の数。
fn dechunk(mut raw: &[u8]) -> (Vec<u8>, usize) {
    let (mut out, mut chunks) = (Vec::new(), 0);
    loop {
        let line = raw
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("塊の長さの行");
        let size = usize::from_str_radix(std::str::from_utf8(&raw[..line]).expect("16 進"), 16)
            .expect("16 進");
        raw = &raw[line + 2..];
        if size == 0 {
            assert_eq!(raw, b"\r\n", "終わりの塊の後ろ");
            return (out, chunks);
        }
        out.extend_from_slice(&raw[..size]);
        assert_eq!(&raw[size..size + 2], b"\r\n", "塊の終わり");
        raw = &raw[size + 2..];
        chunks += 1;
    }
}

/// 完了の姿: 起こして 1 要求を投げ、台本どおりの応答と受けた要求の記録が取れる。
/// 続けて、何も書かない接続の記録・止める・同じ番号に別の名乗りで立て直す、までを通す。
#[tokio::test]
async fn one_request_gets_the_scripted_reply_and_is_recorded() {
    let script = Script::areka().on("tools/call", Reply::Result(json!({"content": []})));
    let mut server = FakeServer::start(script);
    let port = server.port();
    let request =
        json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "t"}});

    let mut first = connect(port).await;
    send(&mut first, &request, true).await;
    let (status, head, body) = receive(&mut first).await;
    assert_eq!(status, 200);
    assert!(head.contains("content-type: application/json"), "{head}");
    assert_eq!(
        parsed(&body),
        json!({"jsonrpc": "2.0", "id": 7, "result": {"content": []}})
    );
    // `Connection: close` の要求に答えた後は、受け口の側が閉じる。
    assert_eq!(first.read(&mut [0u8; 1]).await.expect("閉じを読む"), 0);

    // 接続だけして 1 バイトも書かずに閉じた相手は、「受けたバイト 0」で残る。
    drop(connect(port).await);
    let log = server
        .wait(|log| log.connections.len() == 2 && log.connections.iter().all(|c| c.closed))
        .await;
    assert!(log.connections[0].bytes > 0);
    assert_eq!(
        log.connections[1],
        Conn {
            bytes: 0,
            closed: true
        }
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 1, "何も書かない接続は要求に数えない");
    assert_eq!(requests[0].method, "tools/call");
    assert_eq!(requests[0].body, request);
    assert_eq!(requests[0].connection, 0);
    assert_eq!(requests[0].header("content-type"), Some("application/json"));
    assert_eq!(requests[0].header("origin"), None);
    assert_eq!(server.requests_of("tools/call").len(), 1);
    assert!(server.requests_of("ping").is_empty());

    // 止めると、開いていた接続が閉じ、番号は接続を受けなくなる。記録は止めた後も読める。
    let mut idle = connect(port).await;
    server.wait(|log| log.connections.len() == 3).await;
    server.stop();
    assert!(
        matches!(idle.read(&mut [0u8; 1]).await, Ok(0) | Err(_)),
        "開いていた接続が閉じる"
    );
    let again = timeout(
        Duration::from_millis(300),
        TcpStream::connect(("127.0.0.1", port)),
    )
    .await;
    assert!(!matches!(again, Ok(Ok(_))), "止めた番号は接続を受けない");
    assert_eq!(server.requests().len(), 1);
    assert_eq!(server.connections().len(), 3);

    // 同じ番号に、別の名乗りで立て直す。
    let other = FakeServer::start_on(port, Script::named("ssp-mcp-server"));
    assert_eq!(other.port(), port);
    let (_, _, body) = ask(port, "initialize").await;
    assert_eq!(
        parsed(&body)["result"]["serverInfo"]["name"],
        "ssp-mcp-server"
    );
    assert_eq!(other.requests_of("initialize").len(), 1);

    // 動かしたまま台本を入れ替えると、次の要求から効く。
    other.rescript(Script::areka());
    let (_, _, body) = ask(port, "initialize").await;
    assert_eq!(parsed(&body)["result"]["serverInfo"]["name"], SERVER_NAME);
}

/// rmcp のクライアントの手順への既定の答えと、台本の答え方の全部が、名前どおりの応答になる。
#[tokio::test]
async fn the_handshake_defaults_and_every_reply_mode_answer_as_named() {
    let script = Script::areka()
        .on(
            "m/error",
            Reply::Error(json!({"code": -32000, "message": "boom", "data": {"k": 1}})),
        )
        .on(
            "m/status",
            Reply::ErrorStatus(400, json!({"code": -32600, "message": "bad"})),
        )
        .on("m/broken", Reply::Broken)
        .on("m/text", Reply::Text(415, "plain words"))
        .on("m/empty", Reply::Empty)
        .on("m/midway", Reply::CloseMidway)
        .on(
            "m/chunked",
            Reply::Chunked(json!({"long": "x".repeat(300)})),
        )
        .on(
            "m/delayed",
            Reply::Delayed(
                Duration::from_millis(50),
                Box::new(Reply::Result(json!({"late": true}))),
            ),
        )
        .on("m/until-close", Reply::UntilClose(json!({"n": 1})))
        .on("m/sse", Reply::EventStream)
        .on("m/never", Reply::Never);
    let server = FakeServer::start(script);
    let port = server.port();

    // 1 本の接続で initialize → notifications/initialized（202）→ tools/list → ping。
    let mut stream = connect(port).await;
    let initialize = json!({"jsonrpc": "2.0", "id": "a", "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "self-check", "version": "0"}}});
    send(&mut stream, &initialize, false).await;
    let (status, _, body) = receive(&mut stream).await;
    assert_eq!(status, 200);
    // rmcp が応答を読むのと同じ型で読めて、initialize の結果と判る。
    let ServerJsonRpcMessage::Response(response) =
        serde_json::from_slice(&body).expect("rmcp の型で読める")
    else {
        panic!("応答でない: {}", String::from_utf8_lossy(&body));
    };
    assert_eq!(serde_json::to_value(&response.id).expect("id"), json!("a"));
    let ServerResult::InitializeResult(result) = response.result else {
        panic!("initialize の結果でない: {:?}", response.result);
    };
    assert_eq!(result.server_info.name, SERVER_NAME);
    assert_eq!(result.protocol_version.as_str(), "2025-06-18");
    assert!(result.capabilities.tools.is_some());

    send(
        &mut stream,
        &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        false,
    )
    .await;
    let (status, _, body) = receive(&mut stream).await;
    assert_eq!((status, body.len()), (202, 0));
    send(
        &mut stream,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        false,
    )
    .await;
    assert_eq!(
        parsed(&receive(&mut stream).await.2),
        json!({"jsonrpc": "2.0", "id": 2, "result": {"tools": []}})
    );
    send(
        &mut stream,
        &json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}),
        false,
    )
    .await;
    assert_eq!(
        parsed(&receive(&mut stream).await.2),
        json!({"jsonrpc": "2.0", "id": 3, "result": {}})
    );
    let seen: Vec<_> = server
        .requests()
        .into_iter()
        .map(|r| (r.method, r.connection))
        .collect();
    let expected = [
        "initialize",
        "notifications/initialized",
        "tools/list",
        "ping",
    ]
    .map(|m| (m.to_owned(), 0));
    assert_eq!(seen, expected, "4 つとも同じ 1 本の接続で受ける");

    // 版の答え方は本物の受け口と同じ: `initialize` を持つ 4 版はその版、ほか（rmcp のクライアントが
    // 送る 2026-07-28・知らない版・版なし）は 2025-11-25。
    for (asked, answered) in [
        (json!("2026-07-28"), "2025-11-25"),
        (json!("2025-11-25"), "2025-11-25"),
        (json!("2025-03-26"), "2025-03-26"),
        (json!("2024-11-05"), "2024-11-05"),
        (json!("1999-01-01"), "2025-11-25"),
        (Value::Null, "2025-11-25"),
    ] {
        let mut stream = connect(port).await;
        let initialize = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": asked}});
        send(&mut stream, &initialize, true).await;
        let version = parsed(&receive(&mut stream).await.2)["result"]["protocolVersion"].clone();
        assert_eq!(version, answered, "{asked} で始めたときの版");
    }

    // 台本に無い方法は、JSON-RPC のエラーで答える。
    assert_eq!(
        parsed(&ask(port, "no/such").await.2)["error"]["code"],
        -32601
    );

    let (status, _, body) = ask(port, "m/error").await;
    let error = json!({"code": -32000, "message": "boom", "data": {"k": 1}});
    assert_eq!(
        (status, parsed(&body)),
        (200, json!({"jsonrpc": "2.0", "id": 1, "error": error}))
    );

    let (status, head, body) = ask(port, "m/status").await;
    assert_eq!(
        (status, parsed(&body)["error"]["code"].clone()),
        (400, json!(-32600))
    );
    assert!(head.contains("content-type: application/json"), "{head}");

    let (status, head, body) = ask(port, "m/broken").await;
    assert!(
        status == 200 && head.contains("content-type: application/json"),
        "{head}"
    );
    assert!(!body.is_empty() && serde_json::from_slice::<Value>(&body).is_err());

    let (status, head, body) = ask(port, "m/text").await;
    assert!(
        status == 415 && head.contains("content-type: text/plain"),
        "{head}"
    );
    assert_eq!(body, b"plain words");

    let (status, head, body) = ask(port, "m/empty").await;
    assert_eq!(
        (status, promised_length(&head), body.len()),
        (200, Some(0), 0)
    );

    let (status, head, body) = ask(port, "m/midway").await;
    let promised = promised_length(&head).expect("長さの札");
    assert!(
        status == 200 && !body.is_empty() && body.len() < promised,
        "{} / {promised}",
        body.len()
    );

    let (status, head, body) = ask(port, "m/chunked").await;
    assert!(
        status == 200 && head.contains("transfer-encoding: chunked"),
        "{head}"
    );
    assert_eq!(promised_length(&head), None);
    let (body, chunks) = dechunk(&body);
    assert_eq!(
        parsed(&body),
        json!({"jsonrpc": "2.0", "id": 1, "result": {"long": "x".repeat(300)}})
    );
    assert!(
        chunks >= 2,
        "塊が 1 つだけだと、つなぎ目の読み方が確かめられない"
    );

    // 長さの札も chunked も無い応答は、閉じるまでが本文。
    let (status, head, body) = ask(port, "m/until-close").await;
    assert!(
        !head.contains("content-length") && !head.contains("transfer-encoding"),
        "{head}"
    );
    assert_eq!(
        (status, parsed(&body)["result"].clone()),
        (200, json!({"n": 1}))
    );

    let started = Instant::now();
    let (_, _, body) = ask(port, "m/delayed").await;
    assert!(started.elapsed() >= Duration::from_millis(50));
    assert_eq!(parsed(&body)["result"], json!({"late": true}));

    // SSE: 頭は text/event-stream で長さの札が無く、1 件を書いた後も閉じない。
    let mut sse = connect(port).await;
    send(
        &mut sse,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "m/sse"}),
        true,
    )
    .await;
    let (status, head, mut event) = receive_head(&mut sse).await;
    assert!(
        status == 200 && head.contains("content-type: text/event-stream"),
        "{head}"
    );
    assert_eq!(promised_length(&head), None);
    while !event.ends_with(b"\n\n") {
        assert_ne!(
            sse.read_buf(&mut event).await.expect("SSE の 1 件"),
            0,
            "SSE の途中で閉じた"
        );
    }
    assert!(event.starts_with(b"data: "));

    // 返事をしない: 待っても 1 バイトも来ず、閉じもしない。こちらが閉じると接続が終わる。
    let mut silent = connect(port).await;
    send(
        &mut silent,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "m/never"}),
        true,
    )
    .await;
    let nothing = timeout(Duration::from_millis(100), silent.read(&mut [0u8; 1])).await;
    assert!(nothing.is_err(), "返事をしないはずが {nothing:?}");
    let log = server
        .wait(|log| log.requests.iter().any(|r| r.method == "m/never"))
        .await;
    let held = log
        .requests
        .iter()
        .find(|r| r.method == "m/never")
        .expect("待った要求")
        .connection;
    assert!(!server.connections()[held].closed);
    drop((stream, sse, silent));
    server
        .wait(|log| log.connections.iter().all(|c| c.closed))
        .await;

    // 動かしたまま台本を入れ替えても、もう受けた（記録に載った）要求は元の台本のまま答える。
    // 変わるのは次に届く要求から。
    let mut long = connect(port).await;
    send(
        &mut long,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "m/delayed"}),
        true,
    )
    .await;
    server
        .wait(|log| {
            log.requests
                .iter()
                .filter(|r| r.method == "m/delayed")
                .count()
                == 2
        })
        .await;
    server.rescript(Script::named("replaced"));
    assert_eq!(
        parsed(&receive(&mut long).await.2)["result"],
        json!({"late": true})
    );
    assert_eq!(
        parsed(&ask(port, "m/delayed").await.2)["error"]["code"],
        -32601
    );
}
