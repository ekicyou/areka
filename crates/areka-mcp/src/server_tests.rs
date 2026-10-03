//! 実ソケットのテスト: 待受・束ねの失敗・終了（要件 1・2.4・9.3・9.4）。
//!
//! ポートは毎回 OS に割り当てさせる（固定の番号は使わない＝要件 9.1）。
//! 記録は `capture` で数える（`start` と `Drop` の記録は呼び出し側のスレッドで出る）。

use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::{SHUTDOWN_WAIT, start};
use crate::ToolRegistry;
use crate::testkit::{post_rpc, rpc, serve};

fn of_level(events: &[CapturedEvent], level: Level) -> Vec<&CapturedEvent> {
    events.iter().filter(|e| e.level == level).collect()
}

/// `ping` が通ることを確かめる（接続が受け付けられ、要求に答えている）。
fn assert_ping(addr: std::net::SocketAddr) {
    let response = post_rpc(addr, &[], &rpc("ping", Some(1), None));
    assert_eq!(response.status, 200, "{response:?}");
    assert_eq!(response.json()["result"], serde_json::json!({}));
}

/// (要件 1.1, 1.2) 待受を始めると info が 1 件・URL は `127.0.0.1` の実番号の `/api/mcp/v1`。
#[test]
fn listen_logs_info_with_url() {
    let (server, events) = capture(|| start(Some(0), ToolRegistry::default()));
    let addr = server.local_addr().expect("空きポートで待ち受けている");
    assert!(addr.ip().is_loopback() && addr.is_ipv4(), "{addr}");
    let infos = of_level(&events, Level::INFO);
    assert_eq!(infos.len(), 1, "{events:?}");
    assert_eq!(
        infos[0].field("url"),
        Some(format!("http://127.0.0.1:{}/api/mcp/v1", addr.port()).as_str()),
        "{events:?}"
    );
    assert!(of_level(&events, Level::ERROR).is_empty(), "{events:?}");
    assert!(of_level(&events, Level::WARN).is_empty(), "{events:?}");
}

/// (要件 1.3, 1.4, 9.3) 使用中の番号では error 1 件（番号と理由）・待ち受けない取っ手・処理は続く。
#[test]
fn bind_failure_logs_error_and_returns_off() {
    // 先に std の受け口で空きポートを占める（固定の番号は使わない）。
    let occupied = TcpListener::bind("127.0.0.1:0").expect("空きポートを占める");
    let port = occupied.local_addr().unwrap().port();

    let (server, events) = capture(|| start(Some(port), ToolRegistry::default()));
    let errors = of_level(&events, Level::ERROR);
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(errors[0].field("port"), Some(port.to_string().as_str()));
    assert!(
        errors[0].field("error").is_some_and(|e| !e.is_empty()),
        "OS の理由が載っていない: {events:?}"
    );
    assert!(of_level(&events, Level::INFO).is_empty(), "{events:?}");
    assert!(server.local_addr().is_none());

    // 以後の処理が続く: 待ち受けない取っ手は黙って落ち、別の待受は普通に立つ。
    let ((), drop_events) = capture(|| drop(server));
    assert!(drop_events.is_empty(), "{drop_events:?}");
    let (_other, addr) = serve(ToolRegistry::default());
    assert_ping(addr);
    drop(occupied);
}

/// (要件 2.4) `0`（None）では info 1 件・待ち受けない（落としても閉じる記録は出ない）。
#[test]
fn disabled_port_logs_info_and_no_thread() {
    let (server, events) = capture(|| start(None, ToolRegistry::default()));
    assert_eq!(of_level(&events, Level::INFO).len(), 1, "{events:?}");
    assert_eq!(events.len(), 1, "{events:?}");
    // 理由（環境変数の名前）も載せる（要件 2.4）。
    assert!(events[0].message().contains(crate::PORT_ENV), "{events:?}");
    assert!(server.local_addr().is_none());
    // スレッドも待受も無いので、畳む記録（info「閉じた」）も出ない。
    let ((), drop_events) = capture(|| drop(server));
    assert!(drop_events.is_empty(), "{drop_events:?}");
}

/// (要件 1.5) 接続 A を開いたまま（何も送らない）でも、接続 B の `ping` が答えを得る。
#[test]
fn two_connections_interleave() {
    let (_server, addr) = serve(ToolRegistry::default());
    let idle = TcpStream::connect(addr).expect("接続 A を開く");
    // B が A に塞がれれば testkit の読み取りの上限で失敗する。
    assert_ping(addr);
    drop(idle);
}

/// (要件 1.7, 9.4) 畳んだ後は同じ番地へ接続できない。
#[test]
fn drop_closes_port() {
    let (server, addr) = serve(ToolRegistry::default());
    assert_ping(addr);
    drop(server);
    let result = TcpStream::connect_timeout(&addr, Duration::from_millis(500));
    assert!(result.is_err(), "畳んだ後に接続できた: {result:?}");
}

/// (要件 1.8, 1.9, 9.4) 接続を開いたまま畳んでも `SHUTDOWN_WAIT` 以内に戻り、info 1 件（warn 0 件）。
#[test]
fn drop_with_open_connection_returns_within_bound() {
    let (server, addr) = serve(ToolRegistry::default());
    let idle = TcpStream::connect(addr).expect("接続を開く");
    // 先に開いた A は受付の順で先に受け取られる。B の答えが届いた時点で A も受付済み。
    assert_ping(addr);

    let started = Instant::now();
    let ((), events) = capture(|| drop(server));
    let elapsed = started.elapsed();
    assert!(elapsed < SHUTDOWN_WAIT, "畳むのに {elapsed:?} かかった");
    let infos = of_level(&events, Level::INFO);
    assert_eq!(infos.len(), 1, "{events:?}");
    assert_eq!(infos[0].field("addr"), Some(addr.to_string().as_str()));
    assert!(of_level(&events, Level::WARN).is_empty(), "{events:?}");
    drop(idle);
}
