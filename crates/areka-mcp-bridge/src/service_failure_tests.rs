//! 判断（`service.rs`）のテストのうち、areka へ届ける途中の失敗（要件 4）。
//!
//! 道具（`Desk` など）は `service_tests.rs` のもの。ここも `current_thread` の `#[tokio::test]` だけで書く。

use std::any::TypeId;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rmcp::ServiceError;
use rmcp::model::{ClientRequest, RequestMetaObject};
use rmcp::transport::DynamicTransportError;
use rmcp::transport::streamable_http_client::StreamableHttpError;
use serde_json::{Value, json};

use super::tests::{Desk, WAIT, handshakes, result};
use super::{Bridge, Failure, classify};
use crate::config::{Config, REPLY_WAIT, Timeouts};
use crate::fake_server::{FakeServer, Reply, Script};
use crate::http::HttpError;

/// 中継が自分で作るエラーの行（番号 `-1`）。
fn failed(id: Value, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -1, "message": message}})
}

/// 確かめの上限を長く取った時間（負荷の下で偽の受け口の答えが遅れても areka を飛ばさない）。
pub(super) fn patient() -> Timeouts {
    Timeouts {
        verify: WAIT,
        ..Timeouts::default()
    }
}

/// 書いた後の失敗は中継のエラーで答え、送り直さない（要件 4.5）: 壊れた本文・状態付きの平文・SSE は
/// `Invalid JSON response from server` に HTTP の状態と本文の頭（要件 4.2）、本文なし・途中で閉じる接続は
/// `No response from server`（要件 4.1）。エラーは控えに取らず、エラーの後も同じつなぎ先で次の行に答える
/// （要件 4.6）。SSE の本文は終わらないが、頭だけで決めるので待たない。
#[tokio::test]
async fn a_failure_after_writing_is_answered_with_an_error_and_never_sent_again() {
    let unanswered = "No response from server: empty body or connection closed";
    let cases = [
        (
            Reply::Broken,
            "Invalid JSON response from server: HTTP 200 {\"jsonrpc\": \"2.0\", \"id\"",
        ),
        (
            Reply::Text(500, "boom"),
            "Invalid JSON response from server: HTTP 500 boom",
        ),
        (
            Reply::EventStream,
            "Invalid JSON response from server: HTTP 200 content-type text/event-stream",
        ),
        (Reply::Empty, unanswered),
        (Reply::CloseMidway, unanswered),
    ];
    for (reply, message) in cases {
        let script = Script::areka()
            .on("tools/call", reply.clone())
            .on("tools/list", reply);
        let fake = FakeServer::start(script);
        let mut desk = Desk::start(vec![fake.port()]);

        let call = json!({"name": "get_status"});
        let answer = desk.ask(json!(1), "tools/call", call).await;
        assert_eq!(answer, failed(json!(1), message));
        assert_eq!(fake.requests_of("tools/call").len(), 1, "{message}");
        // 一覧の失敗も同じエラーで、控えに取らない。
        let answer = desk.ask(json!("l"), "tools/list", json!({})).await;
        assert_eq!(answer, failed(json!("l"), message));
        assert_eq!(desk.cache().load_tools(), None, "{message}");
        let pong = desk.ask(json!(2), "ping", json!({})).await;
        assert_eq!(pong, result(json!(2), json!({})), "{message}");

        desk.close().await;
        // 次の行の往復の後も、本当の要求が届いたのは 1 回ずつ（遅れた送り直しも無い）。
        // つなぎ先は作り直していない。
        let sent = fake.requests();
        for method in ["tools/call", "tools/list"] {
            let count = fake.requests_of(method).len();
            assert_eq!(count, 1, "{message}: {method}: {sent:?}");
        }
        assert_eq!(handshakes(&fake), 1, "{message}: {sent:?}");
    }
}

/// areka が応答の上限まで応えない（要件 4.4）: `No response from server: timed out after <上限>` で答え、
/// 送り直さず、同じつなぎ先で次の行に答える。
#[tokio::test]
async fn a_request_not_answered_within_the_bound_gets_the_timeout_error_once() {
    let reply = Duration::from_millis(300);
    let fake = FakeServer::start(Script::areka().on("tools/call", Reply::Never));
    let timeouts = Timeouts { reply, ..patient() };
    let mut desk = Desk::start_with(vec![fake.port()], timeouts);
    // 先につないでおく（探索の時間を、上限の計りに入れない）。
    desk.ask(json!(0), "ping", json!({})).await;

    let asked = Instant::now();
    let call = json!({"name": "get_status"});
    let answer = desk.ask(json!(1), "tools/call", call).await;
    let waited = asked.elapsed();
    let message = "No response from server: timed out after 300ms";
    assert_eq!(answer, failed(json!(1), message));
    assert!(reply <= waited && waited < WAIT, "{waited:?}");
    let pong = desk.ask(json!(2), "ping", json!({})).await;
    assert_eq!(pong, result(json!(2), json!({})));

    desk.close().await;
    let sent = fake.requests();
    assert_eq!(fake.requests_of("tools/call").len(), 1, "{sent:?}");
    assert_eq!(handshakes(&fake), 1, "{sent:?}");
}

/// つなぎ先の働き手が終わっていた（rmcp の `TransportClosed`）: 送り直さずエラーで答え、覚えを捨てるので、
/// 次の要求は探し直して届く（要件 4.6）。捨てるのは、覚えが自分の使ったつなぎ先のままのときだけ。
#[tokio::test]
async fn a_dead_link_is_forgotten_so_the_next_request_scans_again() {
    let fake = FakeServer::start(Script::areka());
    let bridge = Bridge::new(Config::from_parts(vec![fake.port()], None, patient()));
    let ping = || ClientRequest::PingRequest(Default::default());
    let meta = RequestMetaObject::default;
    let Ok(dead) = bridge.ensure_link().await else {
        panic!("areka が見つかるはず");
    };
    dead.link.cancellation_token().cancel();
    let ended = async {
        while !dead.link.is_transport_closed() {
            tokio::task::yield_now().await;
        }
    };
    let ended = tokio::time::timeout(WAIT, ended).await;
    ended.expect("つなぎ先の働き手が終わる");

    let error = bridge.forward(&dead, ping(), meta()).await;
    let error = error.expect_err("終わったつなぎ先では届かない");
    assert_eq!(error.code.0, -1, "{error:?}");
    let message = "No response from server: ";
    assert!(error.message.starts_with(message), "{error:?}");
    assert!(bridge.link.lock().await.is_none(), "覚えを捨てている");

    let Ok(fresh) = bridge.ensure_link().await else {
        panic!("探し直して見つかるはず");
    };
    assert!(!Arc::ptr_eq(&dead, &fresh));
    // 古いつなぎ先の失敗は、別の要求が入れ替えた後の覚えを捨てない。
    let error = bridge.forward(&dead, ping(), meta()).await;
    error.expect_err("終わったつなぎ先では届かない");
    let kept = bridge.link.lock().await.clone();
    assert!(kept.is_some_and(|kept| Arc::ptr_eq(&kept, &fresh)));
    let pong = bridge.forward(&fresh, ping(), meta()).await;
    pong.expect("新しいつなぎ先で届く");

    assert_eq!(fake.requests_of("ping").len(), 1, "{:?}", fake.requests());
    assert_eq!(handshakes(&fake), 2, "{:?}", fake.requests());
}

/// 失敗の読み分け（設計 D-6）: 書く前と読むのは、送り手の「書く前」の失敗だけ。送り手の「書いた後」の
/// 失敗・応答の上限・送り手の失敗に戻せない形（rmcp 自身の失敗）は書いた後（送り直さない側）で、番号 `-1` の
/// エラーになる。本番の上限（30 秒）の文は `timed out after 30s`。
#[test]
fn only_the_senders_failures_before_writing_are_read_as_before_write() {
    let sent = |error: StreamableHttpError<HttpError>| {
        let error = DynamicTransportError::from_parts("t", TypeId::of::<()>(), Box::new(error));
        ServiceError::TransportSend(error)
    };
    let sender = |error| sent(StreamableHttpError::Client(error));
    let read = |error: ServiceError| match classify(0, &error) {
        Failure::BeforeWrite(reason) => format!("before: {reason}"),
        Failure::AfterWrite(answer) => {
            assert_eq!(answer.code.0, -1, "{answer:?}");
            format!("after: {}", answer.message)
        }
    };
    let other = HttpError::NotAreka {
        port: 1,
        name: "ssp".to_owned(),
    };
    let not_json = HttpError::NotJson {
        status: 415,
        head: "nope".to_owned(),
    };
    let unanswered = "after: No response from server: empty body or connection closed";
    let cases = [
        (
            sender(HttpError::Connect(io::ErrorKind::TimedOut.into())),
            "before: connect failed: ",
        ),
        (
            sender(other),
            "before: peer on port 1 is \"ssp\", not areka",
        ),
        (
            sender(HttpError::VerifyTimeout(Duration::from_secs(3))),
            "before: no answer to the identity check within 3s",
        ),
        (sender(HttpError::NoResponse), unanswered),
        (
            sender(not_json),
            "after: Invalid JSON response from server: HTTP 415 nope",
        ),
        // ここから下は rmcp 自身の失敗。輸送の失敗は中の理由だけを載せる（輸送の型の名前は載せない）。
        (
            sent(StreamableHttpError::TransportChannelClosed),
            "after: No response from server: Transport channel closed",
        ),
        // 理由の続きが rmcp の文なので、書き出しだけを見る。
        (
            ServiceError::TransportClosed,
            "after: No response from server: ",
        ),
        (
            ServiceError::UnexpectedResponse,
            "after: No response from server: ",
        ),
    ];
    for (error, expected) in cases {
        let shown = format!("{error:?}");
        let got = read(error);
        // 理由の続きが OS・rmcp の文のもの（`: ` で終わる側）は、書き出しまでを比べる。
        let begins = expected.ends_with(": ") && got.starts_with(expected);
        assert!(got == expected || begins, "{shown}: {got}");
    }
    let timed_out = ServiceError::Timeout {
        timeout: REPLY_WAIT,
    };
    let expected = "after: No response from server: timed out after 30s";
    assert_eq!(read(timed_out), expected);
}
