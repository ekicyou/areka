//! 本物の受け口（`areka_mcp::start`）を相手にした通しのテスト。
//!
//! ツールを 1 本登録した受け口を空きポートに起こし、5 つの版それぞれで、同じ要求を「HTTP で直接
//! 送った答え」と「中継ごしの答え」で比べる（要件 1.2・1.10）。比べるのは応答の全体（`jsonrpc`・`id`・
//! `result`／`error`）を `serde_json::Value` にしたもので、欄の順と空白だけを問わない。
//!
//! 受け口のパスと名乗りは `areka_mcp` の定数を読む。受け口が名乗る値と中継が見なす値が食い違えば、
//! 中継が受け口を飛ばして「居ない」エラーを返すので、どの比較も赤になる（要件 8.2）。
//! 机の 9801〜9830 へはつながない（候補は起こした受け口の番号だけ）。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use areka_mcp::{McpServer, ToolContent, ToolOutcome, ToolRegistry, ToolSpec};
use areka_mcp_bridge::cache::Cache;
use areka_mcp_bridge::config::{Config, Timeouts};
use areka_mcp_bridge::service::run;
use serde_json::{Value, json};
use temp_path_kit::TempPath;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

/// `initialize` を持たない版（`server/discover` で始め、要求ごとに `_meta` と札を付ける）。
const STATELESS: &str = "2026-07-28";
/// 待つ上限。過ぎたら panic でテストを赤にする。
const WAIT: Duration = Duration::from_secs(10);
/// 登録するツールの名前。
const TOOL: &str = "probe";

/// ツールを 1 本登録した本物の受け口。`mode` が `fail` なら `isError: true` の結果を返す。
fn real_receiver() -> (McpServer, u16) {
    let schema = json!({"type": "object", "properties": {"mode": {"type": "string"}}});
    let spec = ToolSpec {
        name: TOOL.to_owned(),
        title: Some("Probe".to_owned()),
        description: Some("answers with the mode it was given".to_owned()),
        input_schema: schema.as_object().expect("オブジェクト").clone(),
    };
    let mut registry = ToolRegistry::default();
    registry.register(
        spec,
        Arc::new(|arguments| {
            Box::pin(async move {
                ToolOutcome {
                    content: vec![ToolContent::Text(format!("mode={}", arguments["mode"]))],
                    is_error: arguments["mode"] == "fail",
                }
            })
        }),
    );
    let server = areka_mcp::start(&[0], registry, None);
    let port = server.local_addr().expect("空きポートで待ち受けている");
    (server, port.port())
}

/// 直接つなぐクライアントの 1 往復: `message` を受け口へ POST して、状態と本文を返す。
/// 札はその版のクライアントが送るとおり: `initialize` には版の札を付けず、旧式の版のほかの要求には
/// `MCP-Protocol-Version`、`2026-07-28` にはさらに `Mcp-Method`（`tools/call` なら `Mcp-Name` も）。
fn post(port: u16, version: &str, message: &Value) -> (u16, String) {
    let method = message["method"].as_str().expect("メソッドがある");
    let mut extra = String::new();
    if method != "initialize" {
        extra.push_str(&format!("MCP-Protocol-Version: {version}\r\n"));
    }
    if version == STATELESS {
        extra.push_str(&format!("Mcp-Method: {method}\r\n"));
        if let Some(name) = message["params"]["name"].as_str() {
            extra.push_str(&format!("Mcp-Name: {name}\r\n"));
        }
    }
    let body = message.to_string();
    let path = areka_mcp::MCP_PATH;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         Accept: application/json, text/event-stream\r\n{extra}Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("受け口へつなげる");
    stream.set_read_timeout(Some(WAIT)).expect("上限を置ける");
    stream
        .write_all(format!("{head}{body}").as_bytes())
        .expect("要求を書ける");
    let mut raw = String::new();
    let read = stream.read_to_string(&mut raw);
    read.expect("受け口が閉じるまで読める");
    let (head, body) = raw.split_once("\r\n\r\n").expect("頭と本文に分かれる");
    // 受け口は長さ付きの単発の JSON で答える（分割の本文になったら、ここで気付く）。
    assert!(
        !head.to_ascii_lowercase().contains("transfer-encoding"),
        "{head}"
    );
    let status = head.split(' ').nth(1).and_then(|s| s.parse().ok());
    (status.expect("状態が読める"), body.to_owned())
}

/// 直接送った答え（JSON 1 件。JSON-RPC のエラーは HTTP の状態に関わらず本文で返る）。
fn direct(port: u16, version: &str, message: &Value) -> Value {
    let (status, body) = post(port, version, message);
    serde_json::from_str(&body)
        .unwrap_or_else(|error| panic!("{version} {message}: HTTP {status} {body:?} ({error})"))
}

/// Desktop の役: 中継の標準入力へ行を書き、標準出力の行を読む。
struct Desk {
    stdin: DuplexStream,
    stdout: BufReader<DuplexStream>,
    /// 控えの置き場（落とすと消える）。
    cache_dir: TempPath,
}

impl Desk {
    /// 候補がこの番号だけの中継を回す。確かめの上限は長く取る（負荷の下で受け口の答えが遅れても
    /// areka を飛ばさない）。
    fn start(port: u16) -> Desk {
        let cache_dir = TempPath::under_target("bridge-real-receiver");
        let dir = Some(cache_dir.path().to_owned());
        let timeouts = Timeouts {
            verify: WAIT,
            ..Timeouts::default()
        };
        let config = Config::from_parts(vec![port], dir, timeouts);
        let (stdin, bridge_in) = tokio::io::duplex(64 * 1024);
        let (bridge_out, stdout) = tokio::io::duplex(64 * 1024);
        tokio::spawn(run(config, bridge_in, bridge_out));
        Desk {
            stdin,
            stdout: BufReader::new(stdout),
            cache_dir,
        }
    }

    async fn send(&mut self, message: &Value) {
        let line = format!("{message}\n");
        let written = self.stdin.write_all(line.as_bytes()).await;
        written.expect("標準入力へ書ける");
    }

    /// 要求を 1 行送り、標準出力の次の 1 行を返す（中継ごしの答え）。
    async fn ask(&mut self, message: &Value) -> Value {
        self.send(message).await;
        let mut line = String::new();
        let read = tokio::time::timeout(WAIT, self.stdout.read_line(&mut line)).await;
        read.expect("期限内に 1 行出る").expect("1 行読める");
        serde_json::from_str(&line).expect("標準出力の 1 行は JSON")
    }
}

/// その版のクライアントが送る要求 1 件。`2026-07-28` は `params._meta` に版・名乗り・できることを
/// 載せる。旧式の版は `params` をそのまま（無ければ欄ごと省く）。
fn request(version: &str, case: &Value) -> Value {
    let mut message = json!({"jsonrpc": "2.0", "id": case["id"], "method": case["method"]});
    if let Some(params) = case.get("params") {
        message["params"] = params.clone();
    }
    if version == STATELESS {
        message["params"]["_meta"] = json!({
            "io.modelcontextprotocol/protocolVersion": version,
            "io.modelcontextprotocol/clientCapabilities": {},
            "io.modelcontextprotocol/clientInfo": {"name": "desktop", "version": "1"},
        });
    }
    message
}

/// 1 つの版で、最初の問い合わせから順に、直接の答えと中継ごしの答えを比べる。中継は版ごとに
/// 新しく起こす（Desktop は 1 つのプロセスで話し方を替えない）。`at`・`is` は、比較が「どちらも同じ
/// 的外れの答え」で通らないための、直接の答えの中の 1 か所（JSON ポインタと値）。
async fn compare(version: &str) {
    let (_server, port) = real_receiver();
    let mut desk = Desk::start(port);
    let stateless = version == STATELESS;
    let name = areka_mcp::SERVER_NAME;

    // 最初の問い合わせ: 旧式の版は `initialize`（続けて通知）、`2026-07-28` は `server/discover`
    // （旧式の話し方の `server/discover` は比べない＝設計 D-8）。
    // `ping` は、旧式の版では空の結果、`2026-07-28` では areka の `-32601`（その版に `ping` は無い）。
    let (first, ping_at, ping_is) = if stateless {
        let at = "/result/_meta/io.modelcontextprotocol~1serverInfo/name";
        let first = json!({"id": 1, "method": "server/discover", "at": at, "is": name});
        (first, "/error/code", json!(-32601))
    } else {
        let client = json!({"name": "desktop", "version": "1"});
        let params = json!({"protocolVersion": version, "capabilities": {}, "clientInfo": client});
        let first = json!({"id": 1, "method": "initialize", "params": params,
            "at": "/result/protocolVersion", "is": version});
        (first, "/result", json!({}))
    };
    let (ok, fail) = (json!({"mode": "ok"}), json!({"mode": "fail"}));
    // 最後の行: 形の違う `params`（`name` が文字列でない）の `tools/call` は、受け口の rmcp が知らない
    // メソッドとして読んで `-32601` で答える。中継も型のない要求として届けるので、同じ答えになる。
    let cases = json!([
        first,
        {"id": 2, "method": "tools/list", "at": "/result/tools/0/name", "is": TOOL},
        {"id": "call-ok", "method": "tools/call", "params": {"name": TOOL, "arguments": ok},
            "at": "/result/content/0/text", "is": "mode=\"ok\""},
        {"id": 4, "method": "tools/call", "params": {"name": TOOL, "arguments": fail},
            "at": "/result/isError", "is": true},
        {"id": 5, "method": "ping", "at": ping_at, "is": ping_is},
        {"id": 6, "method": "nope/unknown", "at": "/error/code", "is": -32601},
        {"id": 7, "method": "tools/call", "params": {"name": "no_such_tool", "arguments": {}},
            "at": "/error/code", "is": -32602},
        {"id": 8, "method": "tools/call", "params": {"name": 7}, "at": "/error/code", "is": -32601},
    ]);

    for case in cases.as_array().expect("配列") {
        let message = request(version, case);
        let direct = direct(port, version, &message);
        let bridged = desk.ask(&message).await;
        assert_eq!(
            bridged, direct,
            "{version} {case}: 中継ごしの答え（左）が直接の答え（右）と違う"
        );
        assert_eq!(direct["id"], case["id"], "{version} {case}: {direct}");
        let at = case["at"].as_str().expect("ポインタ");
        assert_eq!(direct.pointer(at), Some(&case["is"]), "{version}: {direct}");
        if case["method"] == "initialize" {
            // 旧式の版のクライアントは `initialize` の後に通知を送る。受け口は本文なしの 202 で受け、
            // 中継は何も出さない（出せば、次の比較が `id` の違う行を読んで赤になる）。
            let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
            assert_eq!(post(port, version, &initialized), (202, String::new()));
            desk.send(&initialized).await;
        }
    }

    // 中継は本当にこの受け口と話した: 控えに、受け口の名乗りと登録したツールが残っている。
    let cache = Cache::new(Some(desk.cache_dir.path().to_owned()));
    let info = cache.load_server_info().and_then(|info| info.server_info);
    assert_eq!(info.map(|info| info.name), Some(name.to_owned()));
    let tools = cache.load_tools().expect("ツールの一覧の控えがある").tools;
    assert_eq!(tools.len(), 1, "{tools:?}");
    assert_eq!(tools[0].name, TOOL);
}

#[tokio::test]
async fn v2026_07_28_bridged_answers_equal_direct_answers() {
    compare("2026-07-28").await;
}

#[tokio::test]
async fn v2025_11_25_bridged_answers_equal_direct_answers() {
    compare("2025-11-25").await;
}

#[tokio::test]
async fn v2025_06_18_bridged_answers_equal_direct_answers() {
    compare("2025-06-18").await;
}

#[tokio::test]
async fn v2025_03_26_bridged_answers_equal_direct_answers() {
    compare("2025-03-26").await;
}

#[tokio::test]
async fn v2024_11_05_bridged_answers_equal_direct_answers() {
    compare("2024-11-05").await;
}
