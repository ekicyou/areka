//! テスト用の偽の受け口（ループバックの空きポートで、方法ごとの台本で答える）。
//!
//! 部品のテスト（`src/lib.rs` が `cfg(test)` で読む）と通しのテスト（`tests/exe_test.rs`）が
//! この同じ 1 ファイルを `#[path]` で読む。根が違うので、中継の項目を `crate::` で引かない。
//! 自己確認（`fake_server_tests.rs`）は部品のテストの束（`src/lib.rs`）でだけ走る。
//!
//! 受け口は自分のスレッドと自分の tokio（`current_thread`）で回る。呼び手が同期のテストでも、
//! `current_thread` の `#[tokio::test]` でも、子プロセス（建てた exe）を待って止まっている間でも
//! 同じに答える。[`FakeServer::stop`] はそのスレッドの終わりを待つので、戻った時点で待受も
//! 開いていた接続も閉じている。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_mcp::{MCP_PATH, SERVER_NAME};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, watch};

/// 待つ上限（番号の束ね直し・[`FakeServer::wait`]）。過ぎたら panic でテストを赤にする。
const WAIT: Duration = Duration::from_secs(10);
const JSON: &str = "application/json";
/// 旧式の `initialize` を持つ版（新しい順）。本物の受け口が `initialize` にそのまま返す版。
const INITIALIZE_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// 1 つの方法への答え方。`id` は要求のものをそのまま返す。
#[derive(Debug, Clone)]
pub enum Reply {
    /// 200・`result` にこの値。
    Result(Value),
    /// 200・JSON-RPC の `error` にこの値（`code`・`message`・`data`）。
    Error(Value),
    /// HTTP の状態を変えた JSON-RPC のエラー（状態 400 の JSON）。
    ErrorStatus(u16, Value),
    /// `application/json` と名乗る、JSON として読めない本文（壊れた本文）。
    Broken,
    /// この HTTP の状態で、`text/plain` の本文。
    Text(u16, &'static str),
    /// 200 で本文なし（`Content-Length: 0`）。
    Empty,
    /// 202 で本文なし（通知への既定の答え）。
    Accepted,
    /// 頭で約束した長さの半分だけ本文を書いて閉じる。
    CloseMidway,
    /// 返事をしない（相手が閉じるまで、何も書かずに接続を持つ）。
    Never,
    /// `Transfer-Encoding: chunked` で、`result` にこの値（2 つの塊に分ける）。
    Chunked(Value),
    /// 長さの札なしで `result` にこの値を書いて閉じる（閉じるまでが本文）。
    UntilClose(Value),
    /// `text/event-stream` の頭と 1 件を書き、閉じずに接続を持つ。
    EventStream,
    /// この時間だけ遅れてから、中の答え方で答える（長い要求）。
    Delayed(Duration, Box<Reply>),
}

/// 台本: 名乗りと、方法ごとの答え方。台本に無い方法には既定（[`default_reply`]）で答える。
#[derive(Debug, Clone)]
pub struct Script {
    name: String,
    replies: HashMap<String, Reply>,
}

impl Script {
    /// areka と名乗る台本。
    pub fn areka() -> Script {
        Script::named(SERVER_NAME)
    }

    /// 別の名乗りの台本（`initialize` への既定の答えの `serverInfo.name` になる）。
    pub fn named(name: &str) -> Script {
        Script {
            name: name.to_owned(),
            replies: HashMap::new(),
        }
    }

    /// この方法（JSON-RPC の `method`。通知も同じ）への答え方を決める。
    pub fn on(mut self, method: &str, reply: Reply) -> Script {
        self.replies.insert(method.to_owned(), reply);
        self
    }
}

/// 受けた要求 1 つ。
#[derive(Debug, Clone)]
pub struct Received {
    /// JSON-RPC の `method`（本文に無ければ空）。
    pub method: String,
    /// 本文の全部（JSON として読めなければ `Null`）。
    pub body: Value,
    /// 何本目の接続で受けたか（[`Log::connections`] の添字）。
    pub connection: usize,
    /// 要求の札（届いた綴りのまま・届いた順）。
    pub headers: Vec<(String, String)>,
}

impl Received {
    /// 札の値（名前の大文字と小文字は区別しない）。
    pub fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }
}

/// 受け付けた接続 1 本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Conn {
    /// この接続で受けたバイトの数。接続だけして何も書かない相手は 0 のまま。
    pub bytes: usize,
    /// 接続が終わったか（相手が閉じた・受け口が答えた後に閉じた）。
    pub closed: bool,
}

/// 受けたものの記録（受けた順）。
#[derive(Debug, Clone, Default)]
pub struct Log {
    pub requests: Vec<Received>,
    pub connections: Vec<Conn>,
}

struct Shared {
    script: Mutex<Script>,
    log: watch::Sender<Log>,
}

/// 偽の受け口。落とすと止まる。
pub struct FakeServer {
    port: u16,
    shared: Arc<Shared>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FakeServer {
    /// OS が割り当てる空きポートで起こす。別の番号で起こし直すのも、これをもう 1 度呼ぶだけ。
    pub fn start(script: Script) -> FakeServer {
        FakeServer::start_on(0, script)
    }

    /// 番号を決めて起こす（止めた受け口と同じ番号に、別の台本で立て直す）。
    /// OS がまだ番号を放していなければ、期限（[`WAIT`]）まで束ね直す。
    pub fn start_on(port: u16, script: Script) -> FakeServer {
        let deadline = Instant::now() + WAIT;
        let listener = loop {
            match std::net::TcpListener::bind(("127.0.0.1", port)) {
                Ok(listener) => break listener,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(e) => panic!("fake server: cannot bind 127.0.0.1:{port} within {WAIT:?}: {e}"),
            }
        };
        listener
            .set_nonblocking(true)
            .expect("fake server: set_nonblocking");
        let port = listener
            .local_addr()
            .expect("fake server: local_addr")
            .port();
        let shared = Arc::new(Shared {
            script: Mutex::new(script),
            log: watch::Sender::new(Log::default()),
        });
        let (stop, stopped) = oneshot::channel::<()>();
        let thread = std::thread::spawn({
            let shared = shared.clone();
            move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("fake server: runtime");
                runtime.block_on(async move {
                    let listener = TcpListener::from_std(listener).expect("fake server: listener");
                    tokio::spawn(accept_loop(listener, shared));
                    // 止める合図（送り手が落ちても同じ）まで回す。
                    let _ = stopped.await;
                });
                // ここで runtime が落ち、待受と開いている接続の全部が閉じる。
            }
        });
        FakeServer {
            port,
            shared,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// 動かしたまま台本を入れ替える。効くのは次に届く要求から。もう受けた（記録に載った）要求は、
    /// 遅れて答えている途中でも元の台本のまま答える。
    pub fn rescript(&self, script: Script) {
        *self.shared.script.lock().expect("fake server: script") = script;
    }

    /// 止める。戻った時点で待受も開いていた接続も閉じている。記録は残る。2 度呼んでもよい。
    pub fn stop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take()
            && let Err(panic) = thread.join()
            && !std::thread::panicking()
        {
            std::panic::resume_unwind(panic);
        }
    }

    /// 受けた要求の全部（受けた順）。止めた後も読める。
    pub fn requests(&self) -> Vec<Received> {
        self.shared.log.borrow().requests.clone()
    }

    /// 受けた要求のうち、この方法のもの。
    pub fn requests_of(&self, method: &str) -> Vec<Received> {
        let mut requests = self.requests();
        requests.retain(|r| r.method == method);
        requests
    }

    /// 受け付けた接続の全部（受け付けた順）。止めた後も読める。
    pub fn connections(&self) -> Vec<Conn> {
        self.shared.log.borrow().connections.clone()
    }

    /// 記録が条件を満たすまで待って、そのときの記録を返す（応答の行が出ない通知が届いたこと・
    /// 何も書かない接続が閉じたこと、を眠らずに待つ）。期限（[`WAIT`]）を過ぎたら panic。
    pub async fn wait(&self, mut done: impl FnMut(&Log) -> bool) -> Log {
        let mut log = self.shared.log.subscribe();
        match tokio::time::timeout(WAIT, log.wait_for(|log| done(log))).await {
            Ok(Ok(log)) => log.clone(),
            _ => panic!(
                "fake server: the awaited record did not come within {WAIT:?}: {:?}",
                *self.shared.log.borrow()
            ),
        }
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn accept_loop(listener: TcpListener, shared: Arc<Shared>) {
    loop {
        // 受け付けの失敗（受け付ける前に相手が切った など）はその 1 本だけの話。次を待つ。
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let mut connection = 0;
        shared.log.send_modify(|log| {
            connection = log.connections.len();
            log.connections.push(Conn::default());
        });
        let shared = shared.clone();
        tokio::spawn(async move {
            // 入出力の失敗（相手が先に閉じた など）は「接続が終わった」と同じに扱う。
            let _ = serve(stream, connection, &shared).await;
            shared
                .log
                .send_modify(|log| log.connections[connection].closed = true);
        });
    }
}

/// 1 本の接続で、要求を読んでは答える。`Connection: close` の要求に答えたら閉じる。
async fn serve(mut stream: TcpStream, connection: usize, shared: &Shared) -> std::io::Result<()> {
    let mut buf = Vec::new();
    loop {
        let head = loop {
            match parse_head(&buf) {
                Ok(Some(head)) => break head,
                Ok(None) => {
                    if read_more(&mut stream, &mut buf, connection, shared).await? == 0 {
                        return Ok(());
                    }
                }
                Err(e) => return refuse(&mut stream, 400, &format!("request head: {e}")).await,
            }
        };
        let length = find_header(&head.headers, "content-length").and_then(|v| v.parse().ok());
        let end = head.len + length.unwrap_or(0);
        while buf.len() < end {
            if read_more(&mut stream, &mut buf, connection, shared).await? == 0 {
                return Ok(());
            }
        }
        let body: Value = serde_json::from_slice(&buf[head.len..end]).unwrap_or(Value::Null);
        buf.drain(..end);
        let method = body["method"].as_str().unwrap_or_default().to_owned();
        let close = find_header(&head.headers, "connection")
            .is_some_and(|v| v.eq_ignore_ascii_case("close"));
        // 答え方は記録に載せる前に決める（記録に載った要求は、後の `rescript` で変わらない）。
        let reply = {
            let script = shared.script.lock().expect("fake server: script");
            let scripted = script.replies.get(&method).cloned();
            scripted.unwrap_or_else(|| default_reply(&script.name, &method, &body))
        };
        shared.log.send_modify(|log| {
            log.requests.push(Received {
                method: method.clone(),
                body: body.clone(),
                connection,
                headers: head.headers,
            })
        });
        // 送り手の作りの間違いは、黙って答えずに平文で理由を返す（テストの失敗の文に出る）。
        if !head.post_to_mcp {
            return refuse(&mut stream, 404, &format!("expected POST {MCP_PATH}")).await;
        }
        if !body.is_object() {
            return refuse(&mut stream, 400, "the body is not a JSON object").await;
        }
        if !respond(&mut stream, &body["id"], reply, close, connection, shared).await? {
            return Ok(());
        }
    }
}

struct Head {
    /// 頭のバイトの数（本文はこの後ろ）。
    len: usize,
    /// 受け口のパスへの POST か。
    post_to_mcp: bool,
    headers: Vec<(String, String)>,
}

/// 要求の頭を解く。まだ揃っていなければ `None`。
fn parse_head(buf: &[u8]) -> Result<Option<Head>, httparse::Error> {
    let mut slots = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut slots);
    let httparse::Status::Complete(len) = request.parse(buf)? else {
        return Ok(None);
    };
    let headers = request.headers.iter();
    Ok(Some(Head {
        len,
        post_to_mcp: request.method == Some("POST") && request.path == Some(MCP_PATH),
        headers: headers
            .map(|h| {
                (
                    h.name.to_owned(),
                    String::from_utf8_lossy(h.value).into_owned(),
                )
            })
            .collect(),
    }))
}

fn find_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    let (_, value) = headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
    Some(value)
}

/// 接続から続きを読み、受けたバイトの数を記録に足す。0 は相手が閉じた。
async fn read_more(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    connection: usize,
    shared: &Shared,
) -> std::io::Result<usize> {
    let n = stream.read_buf(buf).await?;
    shared
        .log
        .send_modify(|log| log.connections[connection].bytes += n);
    Ok(n)
}

/// 台本に無い方法への答え。通知（`id` なし）は 202、rmcp のクライアントの手順
/// （`initialize`）と `tools/list`・`ping` は正しい結果、ほかは「方法が無い」のエラー。
fn default_reply(name: &str, method: &str, body: &Value) -> Reply {
    if body.get("id").is_none() {
        return Reply::Accepted;
    }
    match method {
        // 版は本物の受け口と同じに答える: `initialize` を持つ 4 版ならその版、ほか（rmcp の
        // クライアントが送る `2026-07-28`・知らない版・版なし）は `2025-11-25`。
        "initialize" => {
            let asked = body["params"]["protocolVersion"].as_str();
            let version = asked.filter(|v| INITIALIZE_VERSIONS.contains(v));
            Reply::Result(json!({
                "protocolVersion": version.unwrap_or(INITIALIZE_VERSIONS[0]),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": name, "version": "0.0.0"},
            }))
        }
        "tools/list" => Reply::Result(json!({"tools": []})),
        "ping" => Reply::Result(json!({})),
        _ => Reply::Error(json!({"code": -32601, "message": "Method not found"})),
    }
}

/// 台本どおりに答える。戻りは「この接続で次の要求を待つか」。
async fn respond(
    stream: &mut TcpStream,
    id: &Value,
    mut reply: Reply,
    close: bool,
    connection: usize,
    shared: &Shared,
) -> std::io::Result<bool> {
    while let Reply::Delayed(delay, inner) = reply {
        tokio::time::sleep(delay).await;
        reply = *inner;
    }
    let result = |value: Value| {
        json!({"jsonrpc": "2.0", "id": id, "result": value})
            .to_string()
            .into_bytes()
    };
    let error = |value: Value| {
        json!({"jsonrpc": "2.0", "id": id, "error": value})
            .to_string()
            .into_bytes()
    };
    let (status, content_type, body) = match reply {
        Reply::Result(value) => (200, JSON, result(value)),
        Reply::Error(value) => (200, JSON, error(value)),
        Reply::ErrorStatus(status, value) => (status, JSON, error(value)),
        Reply::Broken => (200, JSON, b"{\"jsonrpc\": \"2.0\", \"id\"".to_vec()),
        Reply::Text(status, text) => (status, "text/plain", text.into()),
        Reply::Empty => (200, JSON, Vec::new()),
        Reply::Accepted => (202, JSON, Vec::new()),
        Reply::CloseMidway => {
            let body = result(json!({"cut": "this answer is closed halfway"}));
            let framing = format!("Content-Length: {}\r\n", body.len());
            write(stream, 200, JSON, &framing, true, &body[..body.len() / 2]).await?;
            return Ok(false);
        }
        Reply::Never => {
            hold(stream, connection, shared).await;
            return Ok(false);
        }
        Reply::Chunked(value) => {
            let body = result(value);
            let mut chunks = Vec::new();
            for part in [&body[..body.len() / 2], &body[body.len() / 2..]] {
                chunks.extend_from_slice(format!("{:x}\r\n", part.len()).as_bytes());
                chunks.extend_from_slice(part);
                chunks.extend_from_slice(b"\r\n");
            }
            chunks.extend_from_slice(b"0\r\n\r\n");
            write(
                stream,
                200,
                JSON,
                "Transfer-Encoding: chunked\r\n",
                close,
                &chunks,
            )
            .await?;
            return Ok(!close);
        }
        Reply::UntilClose(value) => {
            write(stream, 200, JSON, "", true, &result(value)).await?;
            return Ok(false);
        }
        Reply::EventStream => {
            let event = [b"data: ".as_slice(), &result(json!({})), b"\n\n"].concat();
            write(stream, 200, "text/event-stream", "", false, &event).await?;
            hold(stream, connection, shared).await;
            return Ok(false);
        }
        Reply::Delayed(..) => unreachable!("the delays are unwrapped above"),
    };
    let framing = format!("Content-Length: {}\r\n", body.len());
    write(stream, status, content_type, &framing, close, &body).await?;
    Ok(!close)
}

/// 応答の頭と中身を書く。`framing` は長さの決め方の札の行（無ければ空）。
async fn write(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    framing: &str,
    close: bool,
    payload: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        _ => "Fake",
    };
    let connection = if close { "Connection: close\r\n" } else { "" };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\n{framing}{connection}\r\n"
    );
    stream.write_all(&[head.as_bytes(), payload].concat()).await
}

/// 平文で理由を返して、この接続を終える。
async fn refuse(stream: &mut TcpStream, status: u16, why: &str) -> std::io::Result<()> {
    let body = format!("fake server: {why}");
    let framing = format!("Content-Length: {}\r\n", body.len());
    write(
        stream,
        status,
        "text/plain",
        &framing,
        true,
        body.as_bytes(),
    )
    .await
}

/// 相手が閉じるまで、何も書かずに接続を持つ。その間に届いたバイトは、数だけ記録に足して捨てる。
async fn hold(stream: &mut TcpStream, connection: usize, shared: &Shared) {
    let mut sink = Vec::new();
    while matches!(read_more(stream, &mut sink, connection, shared).await, Ok(n) if n > 0) {
        sink.clear();
    }
}
