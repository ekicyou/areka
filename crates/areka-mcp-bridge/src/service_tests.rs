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
        let root = TempPath::under_target("bridge-service");
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
    /// （出るのは応答の行だけ＝通知には何も出ない）。
    async fn close(mut self) {
        drop(self.stdin);
        let quit = tokio::time::timeout(WAIT, self.run).await;
        let quit = quit.expect("期限内に終わる").expect("入口が panic しない");
        assert!(matches!(quit, QuitReason::Closed), "{quit:?}");
        let mut rest = String::new();
        let read = self.stdout.read_to_string(&mut rest).await;
        read.expect("標準出力を終わりまで読める");
        assert_eq!(rest, "", "読んでいない行が残っている");
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
    let expected = json!({
        "resultType": "complete",
        "supportedVersions": ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25", "2026-07-28"],
        "capabilities": {"tools": {}},
        "ttlMs": 0,
        "cacheScope": "private",
        "_meta": {"io.modelcontextprotocol/serverInfo": {"name": SERVER_NAME, "version": "0.0.0"}},
    });
    assert_eq!(reply, result(json!(5), expected));

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

    let meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "desktop", "version": "1"},
    });
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
