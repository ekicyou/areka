//! toy(a) 統合試験（8.1）: worker⇄worker request/reply を軸に、停止規約・積み残し破棄・
//! 全 Sender drop 終了・panic join 観測・handler Err 後の受信継続を単一の試験群として
//! 機械検証する。
//!
//! 本ファイルはクレートの**公開 API のみ**を用いる統合試験である（`use areka_actor::...`）。
//! 全ケースは `recv_timeout`／bounded join を用い、**どのケースも無限ブロックしない**
//! （8.3・設計「無限ブロックする試験を書かない」）。deadline は CI 余裕込み（応答は ms
//! 到達・deadline は秒単位）。
//!
//! ケース対応（design toy(a) の i–vi）:
//! - (i)   [`echo_round_trip_matches_payload`]         request/reply 応答一致（2.3）
//! - (ii)  [`close_then_join_completes_deterministically`] Close→join 決定的完走（3.4）
//! - (iii) [`backlog_after_close_is_discarded_and_reply_dropped`] 積み残し破棄→要求側 Err(Dropped)（3.3/3.6）
//! - (iv)  [`all_senders_dropped_joins_ok`]            全 Sender drop で join Ok（3.5）
//! - (v)   [`panicking_body_join_is_panicked`]         panic body の join が Err(Panicked)（1.3）
//! - (vi)  [`handler_err_does_not_stop_receive_loop`]  handler Err 後も後続処理継続（3.7）

use std::fmt;
use std::ops::ControlFlow;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use areka_actor::{ActorError, ReplyError, ReplySender, reply_channel, run_inbox, spawn_actor};

/// テスト全体の待機上限（CI 余裕込み）。実応答は ms 単位で到達する。
const DEADLINE: Duration = Duration::from_secs(5);

/// design の規範リファレンス形（envelope 規約の実例）。下流消費者はこの形の上に
/// 自分のメッセージ型を定義する。
enum EchoMsg {
    /// payload をそのまま reply へ返す request/reply variant。
    Echo {
        payload: String,
        reply: ReplySender<String>,
    },
    /// 横断制御の停止メッセージ（3.1・即時停止）。
    Close,
}

/// toy echo worker を `spawn_actor` + `run_inbox` で起動する（実ヘルパを行使する）。
///
/// handler は `Echo` を payload そのままで reply し、`Close` で `Break`。handler は
/// 失敗しないので `E = Infallible`。
fn spawn_echo_worker() -> (Sender<EchoMsg>, areka_actor::ActorHandle) {
    spawn_actor::<EchoMsg, _>("echo-worker", |rx| {
        run_inbox::<EchoMsg, std::convert::Infallible>(rx, |msg| match msg {
            EchoMsg::Echo { payload, reply } => {
                // 受信端が生存していれば必ず届く（2.3）。要求側が既に去っていれば
                // 未達値が Err で返るが、それは要求側キャンセルであり worker の失敗ではない。
                let _ = reply.send(payload);
                Ok(ControlFlow::Continue(()))
            }
            EchoMsg::Close => Ok(ControlFlow::Break(())),
        });
    })
}

/// (i) request/reply 応答一致（2.3）。
///
/// `Echo{payload, reply}` を送ると、要求側は `recv_timeout` で同一 payload を受け取る。
#[test]
fn echo_round_trip_matches_payload() {
    let (tx, handle) = spawn_echo_worker();

    let (reply_tx, reply_rx) = reply_channel::<String>();
    tx.send(EchoMsg::Echo {
        payload: "ping".to_owned(),
        reply: reply_tx,
    })
    .expect("worker inbox should accept Echo while alive");

    let got = reply_rx
        .recv_timeout(DEADLINE)
        .expect("echo reply should arrive before deadline");
    assert_eq!(got, "ping", "reply payload must match the request payload");

    // 後始末: Close→join で決定的に回収（ハングさせない）。
    tx.send(EchoMsg::Close).expect("send Close");
    handle.join().expect("worker joins cleanly after Close");
}

/// (ii) Close→join 決定的完走（3.4）。
///
/// `Close` 送信後の `join()` が `Ok(())` で速やかに完走する（有界＝ハングしない）。
#[test]
fn close_then_join_completes_deterministically() {
    let (tx, handle) = spawn_echo_worker();

    tx.send(EchoMsg::Close).expect("send Close");

    // join を別スレッドで走らせ、期限内に完了しなければ fail（ハング検出）。
    let joined = join_within(DEADLINE, handle);
    assert!(
        matches!(joined, Some(Ok(()))),
        "Close must drive a deterministic, bounded join returning Ok, got {joined:?}"
    );
}

/// (iii) Close 後続メッセージ破棄→要求側 Err(Dropped)（3.3/3.6）。
///
/// `Close` の**後ろ**に `Echo` を積む。worker は Close で即時停止し積み残しの Echo を
/// drain せず破棄する。破棄された Echo に同梱された `ReplySender` も drop されるため、
/// その `ReplyReceiver` は `Err(ReplyError::Dropped)` を観測する（永久ブロックしない）。
#[test]
fn backlog_after_close_is_discarded_and_reply_dropped() {
    let (tx, handle) = spawn_echo_worker();

    let (reply_tx, reply_rx) = reply_channel::<String>();
    // Close を先に積み、その後ろに Echo を積む＝Echo は Close 越しの積み残しになる。
    tx.send(EchoMsg::Close).expect("send Close");
    tx.send(EchoMsg::Echo {
        payload: "should-be-discarded".to_owned(),
        reply: reply_tx,
    })
    .expect("send trailing Echo (still enqueues; will be discarded on stop)");

    // worker は Close で止まり Echo を捨てる → 同梱 reply Sender drop → Dropped 観測。
    match reply_rx.recv_timeout(DEADLINE) {
        Err(ReplyError::Dropped) => {}
        other => panic!(
            "discarded backlog Echo must surface as Err(Dropped) (not Timeout/Ok), got {other:?}"
        ),
    }

    // 停止は決定的に完走する。
    let joined = join_within(DEADLINE, handle);
    assert!(
        matches!(joined, Some(Ok(()))),
        "worker joins Ok after discarding backlog, got {joined:?}"
    );
}

/// (iv) 全 Sender drop で join Ok（3.5）。
///
/// `Close` を送らず、`Sender<EchoMsg>` の全クローンを drop する。inbox が Disconnected に
/// なり受信ループは正常終了、`join()` は `Ok(())`（有界）。
#[test]
fn all_senders_dropped_joins_ok() {
    let (tx, handle) = spawn_echo_worker();

    // クローンを作り、Close を一切送らず全て drop（唯一の停止駆動が Disconnected になる）。
    let tx2 = tx.clone();
    drop(tx);
    drop(tx2);

    let joined = join_within(DEADLINE, handle);
    assert!(
        matches!(joined, Some(Ok(()))),
        "dropping all senders (no Close) must end the loop via Disconnected and join Ok, got {joined:?}"
    );
}

/// (v) panic body の join が Err(Panicked)（1.3）。
///
/// body が panic するアクターを起動し、`join()` が `Err(ActorError::Panicked{..})` を返す。
/// アクター名と panic payload（メッセージ）が観測可能であること。
#[test]
fn panicking_body_join_is_panicked() {
    // panic の stderr 出力を静める（テスト失敗ではなく想定挙動）。復元は不要。
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let (_tx, handle) = spawn_actor::<EchoMsg, _>("panicky-echo", |_rx| {
        panic!("boom in echo worker");
    });

    let joined = join_within(DEADLINE, handle);
    std::panic::set_hook(prev_hook);

    match joined {
        Some(Err(ActorError::Panicked { actor, message })) => {
            assert_eq!(actor, "panicky-echo", "panic must carry the actor name");
            assert!(
                message.contains("boom in echo worker"),
                "panic must carry the payload message, got: {message}"
            );
        }
        other => panic!("panicking body must join to Err(Panicked), got {other:?}"),
    }
}

/// (vi) handler Err 後も後続処理継続（3.7・受信ループ耐障害）。
///
/// handler が「毒 payload」に対し `Err` を返す。基盤は error! 記録のうえ受信を継続し、
/// 後続の `Echo` は正常に処理され reply が届く（後続 reply を観測して継続を確認）。
/// handler の形は `Result<ControlFlow<()>, E>`（`E: Display`）。
#[test]
fn handler_err_does_not_stop_receive_loop() {
    /// handler の失敗を表す表示可能なエラー（`E: Display` 要件を満たす）。
    #[derive(Debug)]
    struct PoisonedPayload(String);
    impl fmt::Display for PoisonedPayload {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "poisoned payload rejected: {}", self.0)
        }
    }

    let (tx, handle) = spawn_actor::<EchoMsg, _>("resilient-echo", |rx| {
        run_inbox::<EchoMsg, PoisonedPayload>(rx, |msg| match msg {
            // 毒 payload は Err（＝この 1 件は処理失敗）。基盤はログして継続するはず。
            EchoMsg::Echo { payload, reply } if payload == "poison" => {
                // reply は送らず drop（要求側は Dropped を観測し得るが本ケースの主眼ではない）。
                drop(reply);
                Err(PoisonedPayload(payload))
            }
            EchoMsg::Echo { payload, reply } => {
                let _ = reply.send(payload);
                Ok(ControlFlow::Continue(()))
            }
            EchoMsg::Close => Ok(ControlFlow::Break(())),
        });
    });

    // 1件目: 毒 payload（handler が Err を返す）。その reply は破棄され Dropped。
    let (poison_reply_tx, poison_reply_rx) = reply_channel::<String>();
    tx.send(EchoMsg::Echo {
        payload: "poison".to_owned(),
        reply: poison_reply_tx,
    })
    .expect("send poison Echo");
    // 毒メッセージ自体の reply は返らない（Dropped）。ここで先にそれを消費しておくことで、
    // 2件目の観測が「後続が処理された」ことのみに依存するようにする。
    match poison_reply_rx.recv_timeout(DEADLINE) {
        Err(ReplyError::Dropped) => {}
        other => panic!("poison message reply should be Dropped, got {other:?}"),
    }

    // 2件目: 正常 payload。handler Err の後でもループが生きていれば処理され reply が届く。
    let (ok_reply_tx, ok_reply_rx) = reply_channel::<String>();
    tx.send(EchoMsg::Echo {
        payload: "after-error".to_owned(),
        reply: ok_reply_tx,
    })
    .expect("send follow-up Echo");
    let got = ok_reply_rx
        .recv_timeout(DEADLINE)
        .expect("message AFTER a handler Err must still be processed and replied (3.7)");
    assert_eq!(got, "after-error", "follow-up reply payload must match");

    // 後始末。
    tx.send(EchoMsg::Close).expect("send Close");
    handle.join().expect("resilient worker joins cleanly");
}

/// `handle.join()` を別スレッドで走らせ、`timeout` 内に完了すればその結果を `Some` で返す。
/// 期限超過（ハング）なら `None`＝呼び出し側が fail させる。どのケースもブロックしっぱなしに
/// しないための有界 join ヘルパ。
fn join_within(
    timeout: Duration,
    handle: areka_actor::ActorHandle,
) -> Option<Result<(), ActorError>> {
    let (done_tx, done_rx) = reply_channel::<Result<(), ActorError>>();
    thread::spawn(move || {
        let _ = done_tx.send(handle.join());
    });
    done_rx.recv_timeout(timeout).ok()
}
