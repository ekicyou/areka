//! 判断（`service.rs`）のテストのうち、届ける前の失敗の扱い（要件 2.10・2.11・設計 D-6）:
//! 覚えた番号に areka が居なくなったら、覚えを捨てて探し直し、見つかれば同じ要求を 1 回だけ送る。
//!
//! 道具（`Desk` など）は `service_tests.rs` のもの。ここも `current_thread` の `#[tokio::test]` だけで書く。

use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use areka_mcp::SERVER_NAME;
use rmcp::model::{ClientRequest, RequestMetaObject};
use serde_json::{Value, json};
use tokio::net::TcpSocket;

use super::Bridge;
use super::failure_tests::patient;
use super::tests::{Desk, WAIT, arrived, handshakes, request, result, stateless_meta, unavailable};
use crate::config::Config;
use crate::fake_server::{FakeServer, Received, Reply, Script};

/// 別の名乗り（SSP の MCP サーバー）。
const OTHER: &str = "ssp-mcp-server";

/// ツールの結果（文で見分ける）。
fn said(text: &str) -> Value {
    json!({"content": [{"type": "text", "text": text}]})
}

/// 文で見分けられるツールの結果を返す areka の台本。
fn areka_saying(text: &str) -> Script {
    Script::areka().on("tools/call", Reply::Result(said(text)))
}

/// 止めた受け口の番号を、束ねたまま待ち受けないソケットで取っておく（落とすまで、並んで走るテストに
/// 取られない）。OS がまだ番号を放していなければ、期限まで束ね直す。
fn keep_closed(port: u16) -> TcpSocket {
    let deadline = Instant::now() + WAIT;
    loop {
        let socket = TcpSocket::new_v4().expect("ソケットを作れる");
        match socket.bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))) {
            Ok(()) => return socket,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => panic!("止めた受け口の番号 {port} を束ねられない: {error}"),
        }
    }
}

/// 受け口が `from` 件目より後に受けた要求。
fn since(fake: &FakeServer, from: usize) -> Vec<Received> {
    fake.requests().split_off(from)
}

/// 全部が、中継の確かめの `initialize`（Desktop の中身を持たない）か。
fn only_checks(requests: &[Received]) -> bool {
    let check = |r: &Received| {
        r.method == "initialize" && r.body["params"]["clientInfo"]["name"] == "areka-mcp-bridge"
    };
    requests.iter().all(check)
}

/// areka が別の番号で起動し直した（要件 2.10）: 覚えた番号に接続できないので、探し直して、同じ要求を
/// 新しいつなぎ先へ 1 回だけ送る。要求の `_meta` も同じまま届く（`2026-07-28` の要求はその版のまま）。
/// 新しい areka のサーバーの情報が控えに入り、次の要求も届く（要件 4.6）。
#[tokio::test]
async fn a_request_reaches_areka_restarted_on_another_port_exactly_once() {
    let mut gone = FakeServer::start(areka_saying("old"));
    let info = json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": "9.9.9"},
    });
    let script = areka_saying("new").on("initialize", Reply::Result(info));
    let back = FakeServer::start(script);
    let mut desk = Desk::start(vec![gone.port(), back.port()]);

    let call = json!({"name": "get_status"});
    let called = desk.ask(json!(1), "tools/call", call).await;
    assert_eq!(called, result(json!(1), said("old")));
    assert!(back.requests().is_empty(), "{:?}", back.requests());

    gone.stop();
    let _closed = keep_closed(gone.port());
    let meta = stateless_meta();
    let call = json!({"name": "get_status", "_meta": meta});
    let called = desk.ask(json!(2), "tools/call", call).await;
    assert_eq!(called, result(json!(2), said("new")));
    let cached = desk.cache().load_server_info();
    let cached = serde_json::to_value(cached.expect("サーバーの情報の控え")).expect("JSON");
    assert_eq!(cached["serverInfo"]["version"], "9.9.9", "{cached}");
    let pong = desk.ask(json!(3), "ping", json!({})).await;
    assert_eq!(pong["id"], 3, "{pong}");

    desk.close().await;
    assert_eq!(gone.requests_of("tools/call").len(), 1, "止める前の 1 つ");
    let [sent] = &back.requests_of("tools/call")[..] else {
        panic!("1 度だけ届くはず: {:?}", back.requests());
    };
    assert_eq!(sent.header("MCP-Protocol-Version"), Some("2026-07-28"));
    for (key, value) in meta.as_object().expect("オブジェクト") {
        assert_eq!(&sent.body["params"]["_meta"][key], value, "{sent:?}");
    }
    assert_eq!(handshakes(&back), 1, "{:?}", back.requests());
}

/// 覚えた番号に別の名乗りの相手が立った（要件 2.11）: 要求の本文はその相手に届かず（届くのは確かめの
/// `initialize` だけ）、探し直して、別の番号の areka へ 1 回だけ送る。
#[tokio::test]
async fn a_request_never_reaches_another_server_that_took_over_the_port() {
    let taken = FakeServer::start(Script::areka());
    let moved = FakeServer::start(areka_saying("moved"));
    let mut desk = Desk::start(vec![taken.port(), moved.port()]);
    desk.ask(json!(1), "ping", json!({})).await;
    let before = taken.requests().len();
    taken.rescript(Script::named(OTHER).on("tools/call", Reply::Result(said("wrong peer"))));

    let call = json!({"name": "get_status"});
    let called = desk.ask(json!(2), "tools/call", call).await;
    assert_eq!(called, result(json!(2), said("moved")));
    let pong = desk.ask(json!(3), "ping", json!({})).await;
    assert_eq!(pong, result(json!(3), json!({})));

    desk.close().await;
    // 入れ替わった相手に届いたのは、要求の接続の確かめと、探し直しの確かめの 2 つだけ。
    let after = since(&taken, before);
    assert!(after.len() == 2 && only_checks(&after), "{after:?}");
    assert_eq!(moved.requests_of("tools/call").len(), 1);
    assert_eq!(moved.requests_of("ping").len(), 1);
    assert_eq!(handshakes(&moved), 1, "{:?}", moved.requests());
}

/// 覚えた番号に別の名乗りの相手が立ち、areka はどこにも居ない: 探し直して、居ないときの答え（その番号に
/// 応えた名乗りを載せたエラー・`ping` は自分で答える）。areka が戻れば、次の要求は届く（要件 4.6）。
#[tokio::test]
async fn without_areka_anywhere_a_takeover_gets_the_not_found_answers() {
    let fake = FakeServer::start(Script::areka());
    let port = fake.port();
    let mut desk = Desk::start(vec![port]);
    desk.ask(json!(1), "ping", json!({})).await;
    let before = fake.requests().len();
    fake.rescript(Script::named(OTHER).on("tools/call", Reply::Result(said("wrong peer"))));

    let call = json!({"name": "get_status"});
    let reply = desk.ask(json!(2), "tools/call", call.clone()).await;
    let reason = format!(
        "{SERVER_NAME} not found on 127.0.0.1 ports {port}; port {port} answered as \"{OTHER}\""
    );
    assert_eq!(reply, unavailable(json!(2), &reason));
    // 要求の接続の確かめと、探し直しの確かめの 2 つだけ（探し直しは 1 回）。
    let after = since(&fake, before);
    assert!(after.len() == 2 && only_checks(&after), "{after:?}");
    let pong = desk.ask(json!(3), "ping", json!({})).await;
    assert_eq!(pong, result(json!(3), json!({})));
    assert!(only_checks(&since(&fake, before)), "{:?}", fake.requests());

    fake.rescript(areka_saying("back"));
    let called = desk.ask(json!(4), "tools/call", call).await;
    assert_eq!(called, result(json!(4), said("back")));
    desk.close().await;
    assert_eq!(handshakes(&fake), 2, "{:?}", fake.requests());
}

/// 長い要求の途中で、別の要求が書く前の失敗を見て覚えを捨てても、長い要求の応答は返る（古いつなぎ先は、
/// 使っている要求が終わるまで落ちない）。
#[tokio::test]
async fn a_slow_request_survives_another_request_forgetting_the_link() {
    let slow = Reply::Delayed(
        Duration::from_secs(2),
        Box::new(Reply::Result(said("slow"))),
    );
    let old = FakeServer::start(Script::areka().on("tools/call", slow));
    let new = FakeServer::start(Script::areka());
    let mut desk = Desk::start(vec![old.port(), new.port()]);

    desk.send(request(json!("slow"), "tools/call", json!({"name": "t"})))
        .await;
    arrived(&old, "tools/call", 1).await;
    // もう受けた要求は元の台本のまま答える。次の接続からは別の名乗り。
    old.rescript(Script::named(OTHER));
    let pong = desk.ask(json!("quick"), "ping", json!({})).await;
    assert_eq!(pong, result(json!("quick"), json!({})));
    assert_eq!(new.requests_of("ping").len(), 1, "{:?}", new.requests());
    assert!(old.requests_of("ping").is_empty(), "{:?}", old.requests());

    let reply = desk.next().await;
    assert_eq!(reply, result(json!("slow"), said("slow")));
    desk.close().await;
    assert_eq!(old.requests_of("tools/call").len(), 1);
    assert!(new.requests_of("tools/call").is_empty());
}

/// 2 つの要求が同時に書く前の失敗を見ても、探し直しとつなぎ直しは 1 回: 後から来た側は、先の側が
/// 入れ替えた新しいつなぎ先をそのまま使う。
#[tokio::test]
async fn two_requests_failing_before_writing_at_once_link_again_only_once() {
    let taken = FakeServer::start(Script::areka());
    let moved = FakeServer::start(areka_saying("moved"));
    let mut desk = Desk::start(vec![taken.port(), moved.port()]);
    desk.ask(json!(0), "ping", json!({})).await;
    let before = taken.requests().len();
    taken.rescript(Script::named(OTHER));

    for id in ["a", "b"] {
        desk.send(request(json!(id), "tools/call", json!({"name": id})))
            .await;
    }
    let mut ids = Vec::new();
    for _ in 0..2 {
        let reply = desk.next().await;
        assert_eq!(reply["result"], said("moved"), "{reply}");
        ids.push(reply["id"].clone());
    }
    ids.sort_by_key(Value::to_string);
    assert_eq!(ids, [json!("a"), json!("b")]);

    desk.close().await;
    // 2 つとも古い番号で確かめに落ちていて（2 つ）、探し直しの確かめは 1 つだけ。
    let after = since(&taken, before);
    assert!(after.len() == 3 && only_checks(&after), "{after:?}");
    assert_eq!(moved.requests_of("tools/call").len(), 2);
    assert_eq!(handshakes(&moved), 1, "{:?}", moved.requests());
}

/// 探し直して見つけた先でも書く前の失敗になったら、もう探さない（送るのは 1 回だけ）: そのつなぎ先も
/// 捨てて、居ないときの答え。次の要求が探し直す。
#[tokio::test]
async fn a_second_failure_before_writing_is_answered_without_another_scan() {
    let gone = FakeServer::start(Script::areka());
    let brief = FakeServer::start(Script::areka());
    let ports = vec![gone.port(), brief.port()];
    let bridge = Bridge::new(Config::from_parts(ports.clone(), None, patient()));
    let Ok(stale) = bridge.ensure_link().await else {
        panic!("areka が見つかるはず");
    };
    gone.rescript(Script::named(OTHER));
    // 探し直しのつなぎの手順（rmcp の `notifications/initialized`）が済んだところで、見つけた先も
    // 別の名乗りに替わる。
    let leaves = async {
        let joined = |r: &Received| r.method == "notifications/initialized";
        brief.wait(|log| log.requests.iter().any(joined)).await;
        brief.rescript(Script::named("other"));
    };
    let call = json!({"method": "tools/call", "params": {"name": "get_status"}});
    let call: ClientRequest = serde_json::from_value(call).expect("要求の形");
    let meta = RequestMetaObject::default;
    let (answer, ()) = tokio::join!(bridge.forward(&stale, call, meta()), leaves);

    let error = answer.expect_err("どこにも届いていない");
    let reason = format!(
        "Server not available: {SERVER_NAME} not found on 127.0.0.1 ports {}, {}",
        ports[0], ports[1]
    );
    assert_eq!((error.code.0, &*error.message), (-1, &*reason));
    assert!(bridge.link.lock().await.is_none(), "覚えを捨てている");
    // 見つけた先に届いたのは、探索の確かめ・rmcp の手順（`initialize`・通知とその確かめ）・送る前の
    // 確かめ、まで。3 度目の確かめ（もう 1 度の探索）は無い。
    let sent = brief.requests();
    assert_eq!(brief.requests_of("initialize").len(), 4, "{sent:?}");
    assert_eq!(handshakes(&brief), 1, "{sent:?}");
    for fake in [&gone, &brief] {
        assert!(fake.requests_of("tools/call").is_empty(), "{sent:?}");
    }

    brief.rescript(Script::areka());
    let Ok(fresh) = bridge.ensure_link().await else {
        panic!("探し直して見つかるはず");
    };
    let ping = ClientRequest::PingRequest(Default::default());
    bridge.forward(&fresh, ping, meta()).await.expect("届く");
}
