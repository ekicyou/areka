//! 判断（`service.rs`）のテスト: 偽の受け口（`crate::fake_server`）を相手に、入口 `run` を
//! メモリ上の管（`tokio::io::duplex`）で回す。
//!
//! 候補の列はテストが差し込む偽の受け口の番号だけ（机の 9801〜9830 へはつながない）。控えの置き場は
//! テストごとの一時フォルダ（`target` の下）。控えの書きは同期で、一時の名前がプロセス番号なので、
//! ここのテストは `current_thread` の `#[tokio::test]` だけで書く。

use std::time::Duration;

use areka_mcp::SERVER_NAME;
use rmcp::service::QuitReason;
use serde_json::{Value, json};
use temp_path_kit::TempPath;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, DuplexStream};
use tokio::net::TcpSocket;
use tokio::task::JoinHandle;

use super::run;
use crate::cache::Cache;
use crate::config::{Config, Timeouts};
use crate::fake_server::{FakeServer, Received, Reply, Script};

const PIPE: usize = 64 * 1024;
/// 待つ上限。過ぎたら panic でテストを赤にする。
const WAIT: Duration = Duration::from_secs(10);
const CACHE_DIR: &str = "mcp-bridge";

/// Desktop の役: 中継の標準入力へ行を書き、標準出力の行を読む。
struct Desk {
    stdin: DuplexStream,
    stdout: BufReader<DuplexStream>,
    run: JoinHandle<QuitReason>,
    /// 控えの置き場の根（落とすと消える）。
    root: TempPath,
}

impl Desk {
    /// この候補の列で中継を回す。確かめの上限は長く取る（負荷の下で偽の受け口の答えが遅れても
    /// areka を飛ばさない）。
    fn start(candidates: Vec<u16>) -> Desk {
        Desk::start_at(TempPath::under_target("bridge-service"), candidates)
    }

    /// 控えの置き場の根を受け取って回す（[`Desk::close`] が返した根を渡すと、起動し直した中継に
    /// なる＝前の中継が残した控えを読む）。
    fn start_at(root: TempPath, candidates: Vec<u16>) -> Desk {
        let timeouts = Timeouts {
            verify: WAIT,
            ..Timeouts::default()
        };
        let config = Config::from_parts(candidates, Some(root.child(CACHE_DIR)), timeouts);
        let (stdin, bridge_in) = tokio::io::duplex(PIPE);
        let (bridge_out, stdout) = tokio::io::duplex(PIPE);
        Desk {
            stdin,
            stdout: BufReader::new(stdout),
            run: tokio::spawn(run(config, bridge_in, bridge_out)),
            root,
        }
    }

    /// 置き場にある控え（別の `Cache` で読む＝起動し直した中継が読むのと同じ）。
    fn cache(&self) -> Cache {
        Cache::new(Some(self.root.child(CACHE_DIR)))
    }

    async fn send(&mut self, message: Value) {
        let line = format!("{message}\n");
        let written = self.stdin.write_all(line.as_bytes()).await;
        written.expect("標準入力へ書ける");
    }

    /// 標準出力の次の 1 行。
    async fn next(&mut self) -> Value {
        let mut line = String::new();
        let read = tokio::time::timeout(WAIT, self.stdout.read_line(&mut line)).await;
        read.expect("期限内に 1 行出る").expect("1 行読める");
        serde_json::from_str(&line).expect("標準出力の 1 行は JSON")
    }

    /// `id` 付きの要求を 1 つ送り、次の 1 行を返す。
    async fn ask(&mut self, id: Value, method: &str, params: Value) -> Value {
        self.send(request(id, method, params)).await;
        self.next().await
    }

    /// 標準入力を閉じる。中継は入力の終わりで終わり、標準出力に読んでいない行は残っていない
    /// （出るのは応答の行だけ＝通知には何も出ない）。控えの置き場の根を返す。
    async fn close(mut self) -> TempPath {
        drop(self.stdin);
        let quit = tokio::time::timeout(WAIT, self.run).await;
        let quit = quit.expect("期限内に終わる").expect("入口が panic しない");
        assert!(matches!(quit, QuitReason::Closed), "{quit:?}");
        let mut rest = String::new();
        let read = self.stdout.read_to_string(&mut rest).await;
        read.expect("標準出力を終わりまで読める");
        assert_eq!(rest, "", "読んでいない行が残っている");
        self.root
    }
}

fn request(id: Value, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

fn notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

fn result(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Desktop が送る `initialize` の中身（名乗りは `desktop`）。
fn initialize_params(version: &str) -> Value {
    json!({
        "protocolVersion": version,
        "capabilities": {},
        "clientInfo": {"name": "desktop", "version": "1"},
    })
}

/// サーバーの情報（`serverInfo` の版がこれ・説明の文なし）から組んだ `server/discover` の答え。
fn discovered(version: &str) -> Value {
    json!({
        "resultType": "complete",
        "supportedVersions": ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25", "2026-07-28"],
        "capabilities": {"tools": {}},
        "ttlMs": 0,
        "cacheScope": "private",
        "_meta": {"io.modelcontextprotocol/serverInfo": {"name": SERVER_NAME, "version": version}},
    })
}

/// `2026-07-28` で話す Desktop が要求ごとに `params` へ載せる `_meta`。
fn stateless_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "desktop", "version": "1"},
    })
}

/// 偽の受け口が返すツールの一覧。
fn tools() -> Value {
    json!({"tools": [{"name": "get_status", "description": "d", "inputSchema": {"type": "object"}}]})
}

/// この方法の要求が `count` 件届くまで待って、届いた全部を返す（応答の行が出ない通知を、眠らずに待つ）。
async fn arrived(fake: &FakeServer, method: &str, count: usize) -> Vec<Received> {
    let of = |r: &&Received| r.method == method;
    fake.wait(|log| log.requests.iter().filter(of).count() >= count)
        .await;
    fake.requests_of(method)
}

/// 受け口が受けた rmcp の手順の数（＝立てたつなぎ先の数）。
fn handshakes(fake: &FakeServer) -> usize {
    fake.requests_of("notifications/initialized").len()
}

/// 結果は areka の答えのまま返り、`id` は数でも文字列でも要求のまま（要件 1.2）。`isError: true` の
/// ツールの結果もエラーに作り替えない。先に居る別の相手に届くのは最初の探索の確かめ 1 つだけで、
/// 決めた後は候補を先頭から当たり直さない（要件 2.2）。
#[tokio::test]
async fn results_come_back_unchanged_under_the_request_id() {
    let failed = json!({"content": [{"type": "text", "text": "boom"}], "isError": true});
    let script = Script::areka()
        .on("tools/list", Reply::Result(tools()))
        .on("tools/call", Reply::Result(failed.clone()));
    let fake = FakeServer::start(script);
    let ssp = FakeServer::start(Script::named("ssp-mcp-server"));
    let mut desk = Desk::start(vec![ssp.port(), fake.port()]);

    let pong = desk.ask(json!(7), "ping", json!({})).await;
    assert_eq!(pong, result(json!(7), json!({})));
    let listed = desk.ask(json!("seven"), "tools/list", json!({})).await;
    assert_eq!(listed, result(json!("seven"), tools()));
    let call = json!({"name": "get_status", "arguments": {}});
    let called = desk.ask(json!(8), "tools/call", call).await;
    assert_eq!(called, result(json!(8), failed));

    desk.close().await;
    assert_eq!(handshakes(&fake), 1, "{:?}", fake.requests());
    assert_eq!(ssp.requests().len(), 1, "{:?}", ssp.requests());
}

/// `initialize`・`server/discover` は areka へ送らず、つないだときに得た areka の情報で答える
/// （要件 3.9）。版は、要求の版が `initialize` を持つ 4 版のどれかならその版、ほかは `2025-11-25`。
#[tokio::test]
async fn first_inquiries_are_answered_from_the_info_areka_gave() {
    let fake = FakeServer::start(Script::areka());
    let mut desk = Desk::start(vec![fake.port()]);

    let versions = [
        ("2025-06-18", "2025-06-18"),
        ("2024-11-05", "2024-11-05"),
        ("2026-07-28", "2025-11-25"),
        ("1999-01-01", "2025-11-25"),
    ];
    for (asked, answered) in versions {
        let reply = desk
            .ask(json!(asked), "initialize", initialize_params(asked))
            .await;
        let expected = json!({
            "protocolVersion": answered,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": "0.0.0"},
        });
        assert_eq!(reply, result(json!(asked), expected));
    }

    let reply = desk.ask(json!(5), "server/discover", json!({})).await;
    assert_eq!(reply, result(json!(5), discovered("0.0.0")));

    desk.close().await;
    // areka に届いた `initialize` は、中継の確かめと rmcp の手順のものだけ。
    let sent = fake.requests();
    assert!(
        sent.iter()
            .all(|r| r.body["params"]["clientInfo"]["name"] != "desktop"),
        "{sent:?}"
    );
    assert!(fake.requests_of("server/discover").is_empty(), "{sent:?}");
    assert_eq!(handshakes(&fake), 1, "{sent:?}");
}

/// 見つけた areka のサーバーの情報と、`tools/list` のエラーでない結果を控えに取る（要件 3.1・3.2）。
/// エラーの応答は控えに取らない（前の控えのまま）。
#[tokio::test]
async fn server_info_and_the_tool_list_are_cached_but_errors_are_not() {
    let fake = FakeServer::start(Script::areka().on("tools/list", Reply::Result(tools())));
    let mut desk = Desk::start(vec![fake.port()]);

    desk.ask(json!(1), "initialize", initialize_params("2025-11-25"))
        .await;
    let info = desk.cache().load_server_info();
    let info = serde_json::to_value(info.expect("サーバーの情報の控えがある")).expect("JSON");
    assert_eq!(info["serverInfo"]["name"], SERVER_NAME, "{info}");
    assert_eq!(desk.cache().load_tools(), None, "まだ一覧を返していない");

    desk.ask(json!(2), "tools/list", json!({})).await;
    let cached = desk.cache().load_tools().expect("一覧の控えがある");
    assert_eq!(serde_json::to_value(&cached).expect("JSON"), tools());

    let broken = json!({"code": -32603, "message": "broken"});
    fake.rescript(Script::areka().on("tools/list", Reply::Error(broken.clone())));
    let failed = desk.ask(json!(3), "tools/list", json!({})).await;
    assert_eq!(failed["error"], broken, "{failed}");
    assert_eq!(desk.cache().load_tools(), Some(cached));

    desk.close().await;
}

/// areka の JSON-RPC のエラーは、番号・文・データを同じにして返す（要件 4.3）。HTTP の状態が 400 でも、
/// 中継が型を知らない方法でも同じ（知らない方法も中身のまま届ける）。
#[tokio::test]
async fn areka_errors_come_back_with_the_same_code_message_and_data() {
    let refused = json!({"code": -32602, "message": "unknown tool", "data": {"name": "nope"}});
    let rejected = json!({"code": -32000, "message": "rejected", "data": [1, "two"]});
    let script = Script::areka()
        .on("tools/call", Reply::Error(refused.clone()))
        .on("x/rejected", Reply::ErrorStatus(400, rejected.clone()));
    let fake = FakeServer::start(script);
    let mut desk = Desk::start(vec![fake.port()]);

    let reply = desk
        .ask(json!(1), "tools/call", json!({"name": "nope"}))
        .await;
    assert_eq!(reply, json!({"jsonrpc": "2.0", "id": 1, "error": refused}));
    let reply = desk.ask(json!("b"), "x/rejected", json!({"k": [1]})).await;
    assert_eq!(reply["error"], rejected, "{reply}");
    // 台本に無い方法には、偽の受け口が「方法が無い」と答える。
    let reply = desk.ask(json!(3), "x/unknown", json!({})).await;
    let no_method = json!({"code": -32601, "message": "Method not found"});
    assert_eq!(reply["error"], no_method, "{reply}");

    desk.close().await;
    let [custom] = &fake.requests_of("x/rejected")[..] else {
        panic!("1 度だけ届くはず: {:?}", fake.requests());
    };
    assert_eq!(custom.body["params"]["k"], json!([1]), "{custom:?}");
}

/// 通知は `notifications/initialized` 以外を届け、標準出力には何も書かない（要件 1.3）。つなぐ先が
/// まだ無いときの通知は何もしない（探さない）。
#[tokio::test]
async fn notifications_reach_areka_without_any_output() {
    let fake = FakeServer::start(Script::areka());
    let mut desk = Desk::start(vec![fake.port()]);

    desk.send(notification("notifications/x-early", json!({})))
        .await;
    desk.ask(json!(1), "ping", json!({})).await;
    desk.send(notification("notifications/initialized", json!({})))
        .await;
    let params = json!({"k": "v", "_meta": {"x": 1}});
    desk.send(notification("notifications/x-note", params.clone()))
        .await;
    let notes = arrived(&fake, "notifications/x-note", 1).await;
    assert_eq!(notes[0].body["params"], params, "{notes:?}");
    // 次に出る行は、次の要求の応答（通知の分の行は無い）。
    let pong = desk.ask(json!(2), "ping", json!({})).await;
    assert_eq!(pong, result(json!(2), json!({})));

    desk.close().await;
    assert!(fake.requests_of("notifications/x-early").is_empty());
    // 届いた `notifications/initialized` は、rmcp の手順が自分で送った 1 つだけ。
    assert_eq!(handshakes(&fake), 1, "{:?}", fake.requests());
}

/// 取り消しの通知は areka へ届けない。通知の `requestId` は Desktop の番号で、areka が見るのは
/// 中継の側のクライアントが振り直した番号だけ（届けると、同じ番号の別の要求が巻き添えになる）。
/// 途中の要求の応答はそのまま返る。
#[tokio::test]
async fn a_cancel_notification_is_not_forwarded_and_fails_no_request_in_flight() {
    let slow = Reply::Delayed(
        Duration::from_millis(500),
        Box::new(Reply::Result(json!({"content": []}))),
    );
    let fake = FakeServer::start(Script::areka().on("tools/call", slow));
    let mut desk = Desk::start(vec![fake.port()]);

    desk.send(request(json!("slow"), "tools/call", json!({"name": "t"})))
        .await;
    arrived(&fake, "tools/call", 1).await;
    for id in 0..4 {
        let params = json!({"requestId": id, "reason": "user"});
        desk.send(notification("notifications/cancelled", params))
            .await;
    }
    // 取り消しの後に送った通知が届いた時点で、取り消しは 1 つも届いていない。
    desk.send(notification("notifications/x-after", json!({})))
        .await;
    arrived(&fake, "notifications/x-after", 1).await;

    let reply = desk.next().await;
    assert_eq!(reply, result(json!("slow"), json!({"content": []})));
    desk.close().await;
    let cancels = fake.requests_of("notifications/cancelled");
    assert!(cancels.is_empty(), "{cancels:?}");
}

/// 長い要求の間も次の要求は待たない（設計 D-13）。最初に同時に届いた 2 つの要求は、探索と
/// つなぎ先を 1 つだけ作る。
#[tokio::test]
async fn a_slow_request_does_not_hold_back_the_next_and_the_first_two_share_one_link() {
    let slow = Reply::Delayed(
        Duration::from_secs(2),
        Box::new(Reply::Result(json!({"content": []}))),
    );
    let fake = FakeServer::start(Script::areka().on("tools/call", slow));
    let mut desk = Desk::start(vec![fake.port()]);

    desk.send(request(json!("slow"), "tools/call", json!({"name": "t"})))
        .await;
    desk.send(request(json!("quick"), "ping", json!({}))).await;
    assert_eq!(desk.next().await, result(json!("quick"), json!({})));
    let reply = desk.next().await;
    assert_eq!(reply, result(json!("slow"), json!({"content": []})));

    desk.close().await;
    assert_eq!(handshakes(&fake), 1, "{:?}", fake.requests());
}

/// 要求の `_meta` は areka へそのまま届く（要件 1.10）。`2026-07-28` で話す Desktop の要求は、
/// areka にもその版の要求として届く（版の札と本文の `_meta`）。
#[tokio::test]
async fn request_meta_reaches_areka_so_a_stateless_request_stays_stateless() {
    let fake = FakeServer::start(Script::areka().on("tools/list", Reply::Result(tools())));
    let mut desk = Desk::start(vec![fake.port()]);

    let meta = stateless_meta();
    let listed = desk
        .ask(json!(1), "tools/list", json!({"_meta": meta}))
        .await;
    assert_eq!(listed, result(json!(1), tools()));

    desk.close().await;
    let [sent] = &fake.requests_of("tools/list")[..] else {
        panic!("1 度だけ届くはず: {:?}", fake.requests());
    };
    assert_eq!(sent.header("MCP-Protocol-Version"), Some("2026-07-28"));
    let sent_meta = &sent.body["params"]["_meta"];
    for (key, value) in meta.as_object().expect("オブジェクト") {
        assert_eq!(&sent_meta[key], value, "{sent:?}");
    }
}

// ---- areka が居ないとき・待ち受けない設定のとき（要件 2.6・3.4〜3.9） ----

/// 誰も居ない番号: 束ねたまま待ち受けないソケット（落とすまで、並んで走るテストに取られない）。
fn nobody() -> (TcpSocket, u16) {
    let socket = TcpSocket::new_v4().expect("ソケットを作れる");
    let any_port = "127.0.0.1:0".parse().expect("アドレスとして読める");
    socket.bind(any_port).expect("空きポートを束ねられる");
    let port = socket.local_addr().expect("番号が分かる").port();
    (socket, port)
}

/// 中継が自分で作る「居ない」エラーの行（番号 `-1`・`Server not available` で始まる文）。
fn unavailable(id: Value, reason: &str) -> Value {
    let message = format!("Server not available: {reason}");
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -1, "message": message}})
}

/// 前に動いた中継が残したサーバーの情報の控え（偽の受け口の今の答えとは、版も説明の文も違う）。
fn cached_info() -> Value {
    json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": "1.2.3"},
        "instructions": "cached words",
    })
}

/// 控えを直に置く（前に動いた中継が areka の答えを書いたのと同じ口で）。
fn seed(cache: &Cache, info: Option<Value>, tools: Option<Value>) {
    if let Some(info) = info {
        cache.save_server_info(&serde_json::from_value(info).expect("サーバーの情報の形"));
    }
    if let Some(tools) = tools {
        cache.save_tools(&serde_json::from_value(tools).expect("一覧の形"));
    }
}

/// `2026-07-28` の要求の `params`。
fn stateless() -> Value {
    json!({"_meta": stateless_meta()})
}

/// 控えの一覧で `2026-07-28` の要求に答える形: areka の受け口がその版で付けるのと同じ 3 欄を補う
/// （`ttlMs` が 0 なので、クライアントは次も聞き直す）。
fn stateless_tools_from_the_cache() -> Value {
    let mut listed = tools();
    listed["resultType"] = json!("complete");
    listed["ttlMs"] = json!(0);
    listed["cacheScope"] = json!("private");
    listed
}

/// 4 種類の要求（`initialize`・`server/discover`・`tools/list`・`tools/call`）を送り、それぞれが結果で
/// 答えられたかを返す。結果でない答えは、この理由の「居ない」エラーであること。
async fn answered(desk: &mut Desk, reason: &str) -> [bool; 4] {
    let asks = [
        ("initialize", initialize_params("2025-11-25")),
        ("server/discover", stateless()),
        ("tools/list", json!({})),
        ("tools/call", json!({"name": "get_status"})),
    ];
    let mut got = [false; 4];
    for (slot, (method, params)) in got.iter_mut().zip(asks) {
        let reply = desk.ask(json!(method), method, params).await;
        *slot = reply.get("result").is_some();
        if !*slot {
            assert_eq!(reply, unavailable(json!(method), reason), "{method}");
        }
    }
    got
}

/// areka が居ない（候補に居るのは別の名乗りの相手だけ）ときの、控えがある場合の答え（設計「areka が
/// 居ないときの答え」の表・要件 3.4・3.6〜3.8）。最初の問い合わせと `tools/list` は控えで、`ping` は
/// 自分で答え、ほかはエラー。一覧の形は、控えを書いたときの話し方でなく、要求の話し方に合わせる。
#[tokio::test]
async fn without_areka_the_cache_answers_the_first_inquiries_and_the_tool_list() {
    let ssp = FakeServer::start(Script::named("ssp-mcp-server"));
    let mut desk = Desk::start(vec![ssp.port()]);
    // 控えの一覧は、`2026-07-28` の話し方で書かれた形（areka が付けた 3 欄入り）。
    let mut written = tools();
    written["resultType"] = json!("complete");
    written["ttlMs"] = json!(60_000);
    written["cacheScope"] = json!("public");
    seed(&desk.cache(), Some(cached_info()), Some(written));

    for (asked, answered) in [("2025-06-18", "2025-06-18"), ("2026-07-28", "2025-11-25")] {
        let reply = desk
            .ask(json!(asked), "initialize", initialize_params(asked))
            .await;
        let mut expected = cached_info();
        expected["protocolVersion"] = json!(answered);
        assert_eq!(reply, result(json!(asked), expected));
    }
    let reply = desk.ask(json!(1), "server/discover", stateless()).await;
    let mut expected = discovered("1.2.3");
    expected["instructions"] = json!("cached words");
    assert_eq!(reply, result(json!(1), expected));

    // 旧式の話し方の要求には 3 欄を付けない。`2026-07-28` の要求には、控えの値でなく決まった 3 欄。
    let reply = desk.ask(json!(2), "tools/list", json!({})).await;
    assert_eq!(reply, result(json!(2), tools()));
    let reply = desk.ask(json!(3), "tools/list", stateless()).await;
    assert_eq!(reply, result(json!(3), stateless_tools_from_the_cache()));

    let pong = desk.ask(json!("p"), "ping", json!({})).await;
    assert_eq!(pong, result(json!("p"), json!({})));
    let port = ssp.port();
    let reason = format!(
        "{SERVER_NAME} not found on 127.0.0.1 ports {port}; port {port} answered as \"ssp-mcp-server\""
    );
    let call = json!({"name": "get_status"});
    let reply = desk.ask(json!(4), "tools/call", call).await;
    assert_eq!(reply, unavailable(json!(4), &reason));
    let reply = desk.ask(json!("u"), "x/unknown", json!({})).await;
    assert_eq!(reply, unavailable(json!("u"), &reason));
    // 通知には何も出さない（探しもしない）。エラーの後も、次の行に答える。
    desk.send(notification("notifications/x-note", json!({})))
        .await;
    let pong = desk.ask(json!(5), "ping", json!({})).await;
    assert_eq!(pong, result(json!(5), json!({})));

    desk.close().await;
    // 別の名乗りの相手に届いたのは、`id` を持つ要求 9 つのための確かめだけ（通知の分は無い）。
    let sent = ssp.requests();
    assert_eq!(sent.len(), 9, "{sent:?}");
    let checks = |r: &Received| {
        r.method == "initialize" && r.body["params"]["clientInfo"]["name"] != "desktop"
    };
    assert!(sent.iter().all(checks), "{sent:?}");
}

/// 当たる控えが無い問い合わせにはエラー（要件 3.5）。控えは種類ごとに見る: サーバーの情報だけ・一覧だけが
/// あるときは、ある側だけ控えで答える。`ping` は控えが無くても自分で答える（要件 3.7）。
#[tokio::test]
async fn without_areka_an_inquiry_with_no_cache_of_its_kind_gets_the_error() {
    let other = FakeServer::start(Script::named("other"));
    let port = other.port();
    let mut desk = Desk::start(vec![port]);
    let reason = format!(
        "{SERVER_NAME} not found on 127.0.0.1 ports {port}; port {port} answered as \"other\""
    );

    assert_eq!(answered(&mut desk, &reason).await, [false; 4]);
    let pong = desk.ask(json!(0), "ping", json!({})).await;
    assert_eq!(pong, result(json!(0), json!({})));

    seed(&desk.cache(), Some(cached_info()), None);
    let got = answered(&mut desk, &reason).await;
    assert_eq!(got, [true, true, false, false], "サーバーの情報だけ");

    let info_file = desk.root.child(CACHE_DIR).join("server-info.json");
    std::fs::remove_file(info_file).expect("サーバーの情報の控えを消せる");
    seed(&desk.cache(), None, Some(tools()));
    let got = answered(&mut desk, &reason).await;
    assert_eq!(got, [false, false, true, false], "一覧だけ");

    desk.close().await;
}

/// 「居ない」エラーの文（設計 D-9）: 候補の番号を候補の順に並べ、areka でない名乗りが応えた番号と名乗りを
/// 続ける（要件 3.6）。誰も応えなければ続きは無い。名乗りは相手が書いた字なので、64 字で切って `...` を
/// 付け、制御文字を `?` に替える（長い文・行の分かれた文を Desktop と記録に流し込ませない）。
#[tokio::test]
async fn the_not_found_error_names_the_candidates_and_who_answered_instead() {
    let (_held, closed) = nobody();
    let mut desk = Desk::start(vec![closed]);
    let reply = desk.ask(json!(1), "tools/call", json!({})).await;
    let reason = format!("{SERVER_NAME} not found on 127.0.0.1 ports {closed}");
    assert_eq!(reply, unavailable(json!(1), &reason));
    desk.close().await;

    let ssp = FakeServer::start(Script::named("ssp-mcp-server"));
    let rude = FakeServer::start(Script::named(&format!("a\nb\t{}", "x".repeat(100))));
    let (first, last) = (ssp.port(), rude.port());
    let mut desk = Desk::start(vec![first, closed, last]);
    let reply = desk.ask(json!(2), "tools/call", json!({})).await;
    let reason = format!(
        "{SERVER_NAME} not found on 127.0.0.1 ports {first}, {closed}, {last}; \
         port {first} answered as \"ssp-mcp-server\"; port {last} answered as \"a?b?{}...\"",
        "x".repeat(60)
    );
    assert_eq!(reply, unavailable(json!(2), &reason));
    desk.close().await;
}

/// `AREKA_MCP_PORT=0`（候補が空）: 探さず、置き場に控えがあっても使わず、`id` を持つ全部（`ping` も）に
/// 「待ち受けない設定」のエラー（要件 2.6）。通知には何も出さず、エラーの後も次の行に答える（要件 4.6）。
#[tokio::test]
async fn port_zero_answers_every_request_with_the_disabled_error() {
    let mut desk = Desk::start(Vec::new());
    seed(&desk.cache(), Some(cached_info()), Some(tools()));

    let asks = [
        ("ping", json!({})),
        ("initialize", initialize_params("2025-11-25")),
        ("server/discover", stateless()),
        ("tools/list", json!({})),
        ("tools/call", json!({"name": "get_status"})),
    ];
    for (method, params) in asks {
        desk.send(notification("notifications/x-note", json!({})))
            .await;
        let reply = desk.ask(json!(method), method, params).await;
        let reason = "AREKA_MCP_PORT=0 (areka does not listen)";
        assert_eq!(reply, unavailable(json!(method), reason), "{method}");
    }
    desk.close().await;
}

/// 控えは中継を起動し直しても残る（要件 3.3）: 前の中継が areka から受け取った答えで、後から起こした
/// 中継が、`initialize` で始める Desktop にも `server/discover` で始める Desktop にも答える（要件 3.4）。
#[tokio::test]
async fn a_bridge_started_later_answers_from_the_cache_an_earlier_one_left() {
    let fake = FakeServer::start(Script::areka().on("tools/list", Reply::Result(tools())));
    let mut first = Desk::start(vec![fake.port()]);
    first.ask(json!(1), "tools/list", json!({})).await;
    let root = first.close().await;
    // areka が去り、同じ番号に居るのは別の名乗りの相手。
    fake.rescript(Script::named("ssp-mcp-server"));

    let mut legacy = Desk::start_at(root, vec![fake.port()]);
    let reply = legacy
        .ask(json!(1), "initialize", initialize_params("2025-06-18"))
        .await;
    let expected = json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": "0.0.0"},
    });
    assert_eq!(reply, result(json!(1), expected));
    let reply = legacy.ask(json!(2), "tools/list", json!({})).await;
    assert_eq!(reply, result(json!(2), tools()));
    let root = legacy.close().await;

    let mut modern = Desk::start_at(root, vec![fake.port()]);
    let reply = modern.ask(json!(1), "server/discover", stateless()).await;
    assert_eq!(reply, result(json!(1), discovered("0.0.0")));
    // 旧式の話し方で書かれた控えの一覧に、`2026-07-28` の 3 欄を補う。
    let reply = modern.ask(json!(2), "tools/list", stateless()).await;
    assert_eq!(reply, result(json!(2), stateless_tools_from_the_cache()));
    modern.close().await;

    // areka に届いた一覧の要求は最初の中継の 1 つだけで、立てたつなぎ先も 1 つ。
    let listed = fake.requests_of("tools/list");
    assert_eq!(listed.len(), 1, "{:?}", fake.requests());
    assert_eq!(handshakes(&fake), 1, "{:?}", fake.requests());
}

/// 居ないと答えた後で areka が見つかれば、以降は areka の今の答えで応え、控えが新しい中身に置き換わる
/// （要件 3.9・3.1・3.2）。
#[tokio::test]
async fn once_areka_is_found_its_answers_replace_the_cache() {
    let fake = FakeServer::start(Script::named("ssp-mcp-server"));
    let mut desk = Desk::start(vec![fake.port()]);
    let old = json!({"tools": []});
    seed(&desk.cache(), Some(cached_info()), Some(old.clone()));
    let reply = desk.ask(json!(1), "tools/list", json!({})).await;
    assert_eq!(reply, result(json!(1), old));

    // 同じ番号に areka が現れる（サーバーの情報もツールの一覧も、控えとは違う）。
    fake.rescript(Script::areka().on("tools/list", Reply::Result(tools())));
    let reply = desk.ask(json!(2), "tools/list", json!({})).await;
    assert_eq!(reply, result(json!(2), tools()));
    let reply = desk
        .ask(json!(3), "initialize", initialize_params("2025-11-25"))
        .await;
    let expected = json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": "0.0.0"},
    });
    assert_eq!(reply, result(json!(3), expected.clone()));

    let cache = desk.cache();
    let info = cache.load_server_info().expect("サーバーの情報の控え");
    assert_eq!(serde_json::to_value(info).expect("JSON"), expected);
    let listed = cache.load_tools().expect("一覧の控え");
    assert_eq!(serde_json::to_value(listed).expect("JSON"), tools());
    desk.close().await;
    assert_eq!(handshakes(&fake), 1, "{:?}", fake.requests());
}
