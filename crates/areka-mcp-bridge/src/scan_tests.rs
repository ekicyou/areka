//! 探索（`scan.rs`）のテスト。
//!
//! 候補の列はテストが差し込む: 偽の受け口（`crate::fake_server`）・本物の受け口・束ねたまま
//! 待ち受けないソケット、の OS が割り当てた番号だけ。机の 9801〜9830 へはつながない。

use std::time::{Duration, Instant};

use areka_mcp::{SERVER_NAME, ToolRegistry};
use serde_json::{Value, json};
use tokio::net::TcpSocket;

use super::{Link, Scan, scan};
use crate::config::Timeouts;
use crate::fake_server::{Conn, FakeServer, Received, Reply, Script};

const SSP: &str = "ssp-mcp-server";

/// 確かめの上限を長く取った時間（負荷の下で偽の受け口の答えが遅れても、areka を飛ばさない）。
fn patient() -> Timeouts {
    Timeouts {
        verify: Duration::from_secs(10),
        ..Timeouts::default()
    }
}

/// 誰も居ない番号: 束ねたまま待ち受けないソケット（落とすまで、並んで走るテストに取られない）。
fn nobody() -> (TcpSocket, u16) {
    let socket = TcpSocket::new_v4().expect("ソケットを作れる");
    let any_port = "127.0.0.1:0".parse().expect("アドレスとして読める");
    socket.bind(any_port).expect("空きポートを束ねられる");
    let port = socket.local_addr().expect("番号が分かる").port();
    (socket, port)
}

fn found(scan: Scan) -> (Link, u16) {
    match scan {
        Scan::Found { link, port } => (link, port),
        Scan::NotFound { others } => panic!("見つからない（others: {others:?}）"),
    }
}

fn not_found(scan: Scan) -> Vec<(u16, String)> {
    match scan {
        Scan::NotFound { others } => others,
        Scan::Found { port, .. } => panic!("{port} に決まった"),
    }
}

/// つながった印が持つ、相手のサーバーの情報（JSON の形）。
fn peer_info(link: &Link) -> Value {
    let info = link.peer_info().expect("サーバーの情報を持っている");
    serde_json::to_value(&*info).expect("JSON にできる")
}

/// areka の今の情報を持つつながった印か（名乗りと、旧式の `initialize` で決まった版）。
fn assert_linked_to_areka(link: &Link) {
    let info = peer_info(link);
    assert_eq!(info["serverInfo"]["name"], SERVER_NAME, "{info}");
    assert_eq!(info["protocolVersion"], "2025-11-25", "{info}");
}

/// この受け口に届いたのは、探索の確かめの `initialize` の 1 つだけ（本当の要求も rmcp の手順も 0）。
async fn only_the_check_arrived(fake: &FakeServer) {
    let log = fake
        .wait(|log| log.connections.iter().any(|c| c.closed))
        .await;
    assert_eq!(log.connections.len(), 1, "{log:?}");
    let [check] = &log.requests[..] else {
        panic!("届くのは確かめの 1 つだけのはず: {log:?}");
    };
    assert_eq!(check.method, "initialize", "{log:?}");
    assert_eq!(check.body["id"], "identity-check", "{log:?}");
}

/// 先に居る SSP を飛ばして、後ろの areka に決める（要件 2.2・2.3）。SSP に届くのは確かめの 1 つ
/// だけ。決めた番号には rmcp のクライアントが旧式の `initialize` で立つ（設計 D-3）。
#[tokio::test]
async fn a_peer_with_another_name_is_passed_over_for_areka_behind_it() {
    let ssp = FakeServer::start(Script::named(SSP));
    let areka = FakeServer::start(Script::areka());

    let (link, port) = found(scan(&[ssp.port(), areka.port()], &patient()).await);
    assert_eq!(port, areka.port());
    only_the_check_arrived(&ssp).await;
    assert_linked_to_areka(&link);

    // rmcp の手順: `initialize`（名乗りは中継の名前と版）→ `notifications/initialized`。
    let log = areka
        .wait(|log| {
            let mut methods = log.requests.iter().map(|r| r.method.as_str());
            methods.any(|method| method == "notifications/initialized")
        })
        .await;
    let handshakes: Vec<_> = log
        .requests
        .iter()
        .filter(|r| r.method == "initialize" && r.body["id"] != "identity-check")
        .collect();
    let [handshake] = &handshakes[..] else {
        panic!("rmcp の手順の initialize は 1 つのはず: {log:?}");
    };
    assert_eq!(
        handshake.body["params"]["clientInfo"],
        json!({"name": "areka-mcp-bridge", "version": env!("CARGO_PKG_VERSION")}),
        "{log:?}"
    );
}

/// areka が 2 つ居れば先の 1 つで決め、後ろの候補には接続を開くだけで 1 バイトも書かない
/// （要件 2.2・設計 D-5）。
#[tokio::test]
async fn the_first_areka_decides_and_a_later_candidate_gets_no_byte() {
    let first = FakeServer::start(Script::areka());
    let second = FakeServer::start(Script::areka());

    let (_link, port) = found(scan(&[first.port(), second.port()], &patient()).await);
    assert_eq!(port, first.port());

    let log = second
        .wait(|log| log.connections.iter().any(|c| c.closed))
        .await;
    let untouched = Conn {
        bytes: 0,
        closed: true,
    };
    assert_eq!(log.connections, [untouched], "{log:?}");
    assert!(log.requests.is_empty(), "{log:?}");
}

/// areka がどこにも居なければ「見つからない」。areka でない名乗りが応えた番号と名乗りを、候補の順に
/// 集める（要件 3.6）。その相手にも、届くのは確かめの 1 つだけ。
#[tokio::test]
async fn without_areka_the_other_names_are_collected_in_candidate_order() {
    let (_socket_a, closed_a) = nobody();
    let (_socket_b, closed_b) = nobody();
    let ssp = FakeServer::start(Script::named(SSP));
    let other = FakeServer::start(Script::named("other"));

    // 候補の順は、受け口を起こした順（番号の小さい順になりやすい）と逆にする。
    let candidates = [closed_a, other.port(), closed_b, ssp.port()];
    let others = not_found(scan(&candidates, &patient()).await);
    assert_eq!(
        others,
        [
            (other.port(), "other".to_owned()),
            (ssp.port(), SSP.to_owned())
        ]
    );
    only_the_check_arrived(&ssp).await;
    only_the_check_arrived(&other).await;
}

/// 接続は受けるが返事をしない相手は、確かめの上限で見切って次の候補へ進む（要件 2.9）。
#[tokio::test]
async fn a_silent_peer_is_given_up_and_the_next_candidate_is_tried() {
    let silent = FakeServer::start(Script::areka().on("initialize", Reply::Never));
    let areka = FakeServer::start(Script::areka());
    let verify = Duration::from_secs(1);
    let timeouts = Timeouts {
        verify,
        ..Timeouts::default()
    };

    let started = Instant::now();
    let (_link, port) = found(scan(&[silent.port(), areka.port()], &timeouts).await);
    let elapsed = started.elapsed();
    assert_eq!(port, areka.port());
    // 上の限りはゆるく見る（負荷の下でも赤にしない）。
    assert!(
        verify <= elapsed && elapsed < Duration::from_secs(10),
        "{elapsed:?}"
    );
    only_the_check_arrived(&silent).await;
}

/// 番号 1 つの列（`AREKA_MCP_PORT=<番号>` の形）でも同じ確かめをする（要件 2.4）: 名乗りが違えば
/// 見つからず、areka ならその番号に決まる。
#[tokio::test]
async fn a_single_candidate_gets_the_same_check() {
    let ssp = FakeServer::start(Script::named(SSP));
    let others = not_found(scan(&[ssp.port()], &patient()).await);
    assert_eq!(others, [(ssp.port(), SSP.to_owned())]);
    only_the_check_arrived(&ssp).await;

    let areka = FakeServer::start(Script::areka());
    let (_link, port) = found(scan(&[areka.port()], &patient()).await);
    assert_eq!(port, areka.port());
}

/// 名乗りを読めない相手・名乗りが空の相手・黙る相手は飛ばすが、集めた名乗りには入れない
/// （エラーの文に載せる名乗りが無い）。
#[tokio::test]
async fn peers_without_a_readable_name_are_skipped_but_not_collected() {
    let unreadable = FakeServer::start(Script::areka().on("initialize", Reply::Text(200, "hello")));
    let nameless = FakeServer::start(Script::named(""));
    let ssp = FakeServer::start(Script::named(SSP));

    let candidates = [unreadable.port(), nameless.port(), ssp.port()];
    let others = not_found(scan(&candidates, &patient()).await);
    assert_eq!(others, [(ssp.port(), SSP.to_owned())]);

    let silent = FakeServer::start(Script::areka().on("initialize", Reply::Never));
    let timeouts = Timeouts {
        verify: Duration::from_millis(250),
        ..Timeouts::default()
    };
    let others = not_found(scan(&[silent.port()], &timeouts).await);
    assert_eq!(others, []);
}

/// 接続は候補の全部へ並べて開く（設計 D-5）: 誰も居ない番号がいくつ並んでも、待つのは接続の上限の
/// 1 回分ほど。順に開くと、この机では 10 × 500 ms = 5 秒かかる（居ない番号への接続は、断られる
/// より先に上限で切れる）。
#[tokio::test]
async fn connections_to_all_candidates_are_opened_side_by_side() {
    let closed: Vec<(TcpSocket, u16)> = (0..10).map(|_| nobody()).collect();
    let areka = FakeServer::start(Script::areka());
    let mut candidates: Vec<u16> = closed.iter().map(|(_, port)| *port).collect();
    candidates.push(areka.port());
    let connect = Duration::from_millis(500);
    let timeouts = Timeouts {
        connect,
        ..patient()
    };

    let started = Instant::now();
    let (_link, port) = found(scan(&candidates, &timeouts).await);
    let elapsed = started.elapsed();
    assert_eq!(port, areka.port());
    // 上の限りは負荷の分だけゆるく見る（並べて開けば 500 ms ほど）。
    assert!(elapsed < connect * 6, "{elapsed:?}");
}

/// 確かめを通った番号で rmcp のクライアントを立てる手順が失敗したら（確かめの後で相手が替わった・
/// 居なくなったときの道）、探索は「見つからない」で返る。
#[tokio::test]
async fn a_failed_handshake_after_the_check_is_not_found() {
    // 名乗りは areka だが、`initialize` の結果の形になっていない答え: 確かめは通り、rmcp の手順が
    // 失敗する。
    let odd_result = json!({"serverInfo": {"name": SERVER_NAME}});
    let odd = FakeServer::start(Script::areka().on("initialize", Reply::Result(odd_result)));
    let others = not_found(scan(&[odd.port()], &patient()).await);
    assert_eq!(others, []);

    // 確かめで飛ばしたのではなく、rmcp の手順の `initialize` まで進んでいる。
    let handshake = |r: &Received| r.method == "initialize" && r.body["id"] != "identity-check";
    odd.wait(|log| log.requests.iter().any(handshake)).await;
}

/// 本物の受け口（`areka_mcp::start` を空きポートに起こす）を探索が見つけ、立てた rmcp の
/// クライアントで要求が通る（rmcp のクライアント → 自前の送り手 → 本物の受け口、の通し）。
#[tokio::test]
async fn scan_finds_the_real_receiver_and_the_link_carries_a_request() {
    let server = areka_mcp::start(&[0], ToolRegistry::default(), None);
    let port = server
        .local_addr()
        .expect("空きポートで待ち受けている")
        .port();

    let (link, found_port) = found(scan(&[port], &patient()).await);
    assert_eq!(found_port, port);
    assert_linked_to_areka(&link);
    link.list_all_tools().await.expect("tools/list が通る");
    link.cancel().await.expect("クライアントを畳める");
}
