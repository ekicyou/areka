//! toy(b) 統合試験（独立プロセス・bounded pump）: worker→UI pump 実走 echo 往復。
//!
//! # 目的（要件 8.2 / 8.3・design.md「toy tests」toy(b)・System Flows「UI 配送ブリッジ echo」）
//! task 3 で実装した `ui` モジュール（`spawn_ui`／`UiSender`）と `reply` モジュール
//! （`reply_channel`／`ReplyReceiver`）の**公開 API**を、`spawn_ui`＝内部で `spawn_local`／
//! `MessageLoop::run`／`PostThreadMessageW` heartbeat の**実走 pump**上で駆動し、
//! worker→UI アクターへの echo 往復が成立することを単一の機械 pass/fail（8.2/8.3）で観測する。
//!
//! # 縛ること
//! 実際の `spawn_ui`／`UiSender` ブリッジと `reply_channel` の envelope パターンを**そのまま**
//! 通して echo を往復させる（＝公開 API の正用法担保・design「正用法は toy(b) で担保」）。
//! あわせて受信のループの 2 つの枝を縛る: handler の `Err` の後も続く・全 `UiSender` を落とすと
//! 終わる（audit-sylphya-actor #5）。pump は `ui_pump_support::run_pump_until` が回す。
//!
//! # スレッド構成（重要・tasks.md Implementation Notes / ui.rs Risks）
//! pump を回すスレッドが **UI（pump）スレッド**を演じる。`spawn_ui` は呼び出しスレッドの
//! thread-local executor 窓へ drain タスクをスケジュールするため、`spawn_ui` と
//! `MessageLoop::run` は**必ず同一スレッド**で実行する。別スレッドで `spawn_ui`
//! すると drain タスクが走らず deadline 失敗（silent = case (c)）になる。worker と heartbeat は
//! 別スレッド。
//!
//! # bounded 保証（ハングしない）
//! filter は「done フラグ or deadline 超過」で `msg_loop.quit()` する。heartbeat が
//! 無入力でも ~25ms 間隔で `GetMessageW` を起こすため、echo が届かなくても deadline（5 秒）で
//! 必ず pump を抜ける。deadline 超過・echo 内容不一致・応答不達はいずれも `assert!` 失敗として
//! 観測される（8.3）。

mod ui_pump_support;

use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use areka_actor::{ReplyError, ReplySender, reply_channel, spawn_ui};
use ui_pump_support::run_pump_until;

/// toy(b) の上限時間。echo 不達でもこの期限で必ず pump を抜ける（CI 余裕込み・design 例 5 秒）。
const TEST_DEADLINE: Duration = Duration::from_secs(5);

/// UI アクター宛メッセージ（消費者定義の enum・envelope 規約）。
///
/// `Echo` は request/reply variant として返信端 [`ReplySender`] を同梱する（2.2 の oneshot 相当）。
/// `Fail` は handler に `Err` を返させる（受信のループが止まらないことの確かめ）。
/// `Close` は横断制御の停止メッセージ（3.1）で、drain タスクを即時終了させる。
enum UiMsg {
    /// echo 要求: `payload` をそのまま返信端へ返す。
    Echo {
        payload: String,
        reply: ReplySender<String>,
    },
    /// handler が `Err` を返す。
    Fail,
    /// 停止（即時終了・handler は `Ok(Break)` を返す）。
    Close,
}

/// UI アクターの handler（await を跨がず同期実行される）。
/// Echo → 返信端へ payload を返して継続。Fail → `Err`。Close → 即時終了（Break）。
fn handle(msg: UiMsg) -> Result<ControlFlow<()>, &'static str> {
    match msg {
        UiMsg::Echo { payload, reply } => {
            // 要求側キャンセル（Receiver drop）は Err(value) だが handler はそれで死なない。
            let _ = reply.send(payload);
            Ok(ControlFlow::Continue(()))
        }
        UiMsg::Fail => Err("deliberate handler failure"),
        UiMsg::Close => Ok(ControlFlow::Break(())),
    }
}

/// 本スレッドで UI アクターを起動して pump を回し、worker から `before` を送ったあと Echo を
/// 往復させる。往復が期限の中に済んだかを返す（worker の panic は join で伝える）。
fn echo_round_trip_after(before: Vec<UiMsg>) -> bool {
    let done = Arc::new(AtomicBool::new(false));

    let (ui_tx, _drain) =
        spawn_ui("toy-ui-echo", handle).expect("spawn_ui on the pump (test) thread must succeed");

    let worker_done = Arc::clone(&done);
    let worker = std::thread::Builder::new()
        .name("toy-ui-echo-worker".to_owned())
        .spawn(move || {
            let send = |msg: UiMsg| {
                // UiMsg は ReplySender 同梱ゆえ Debug 非実装につき `.expect` は使えない。
                if ui_tx.send(msg).is_err() {
                    panic!("UiSender::send failed (inbox closed — UI drain not live)");
                }
            };
            for msg in before {
                send(msg);
            }

            let (reply_tx, reply_rx) = reply_channel::<String>();
            send(UiMsg::Echo {
                payload: "ping".to_owned(),
                reply: reply_tx,
            });

            // 返信を上限時間付きで待つ（永久ブロックしない）。不達は Timeout/Dropped として観測。
            match reply_rx.recv_timeout(TEST_DEADLINE) {
                Ok(echo) => assert_eq!(echo, "ping", "echoed payload must match the request"),
                Err(ReplyError::Timeout) => {
                    panic!("echo not received within the deadline (worker→UI pump echo failed)")
                }
                Err(ReplyError::Dropped) => {
                    panic!("reply sender dropped before echo (UI drain never handled the request)")
                }
            }

            worker_done.store(true, Ordering::Release);
            // drain タスクを止める（Close＝即時終了・3.2）。停止不達でも done＋deadline で pump は抜ける。
            let _ = ui_tx.send(UiMsg::Close);
        })
        .expect("worker thread must start");

    run_pump_until(&done, TEST_DEADLINE);

    worker.join().expect("worker thread must not panic");
    done.load(Ordering::Acquire)
}

/// 実走 pump 上での worker→UI echo 往復（公開 API 経由）。
#[test]
fn toy_b_worker_to_ui_pump_echo_round_trip() {
    assert!(
        echo_round_trip_after(Vec::new()),
        "worker→UI pump echo round-trip did not complete before the deadline \
         (echo undelivered / mismatched / drain not driven on MessageLoop::run)"
    );
}

/// handler が `Err` を返した後も受信のループは続き、2 件目の Echo の返信が届く（#5）。
/// `Err` で止める直し間違いは返信が `Dropped`（または送信が Closed）になって worker が panic する。
#[test]
fn a_handler_error_does_not_stop_the_ui_actor() {
    assert!(
        echo_round_trip_after(vec![UiMsg::Fail]),
        "the echo after a handler error did not come back before the deadline"
    );
}

/// 全 `UiSender` を落とすと UI アクターが終わる（#5・要件 5.3 の正規の終わり）。
///
/// pump は子のスレッドで回す（`spawn_ui` もそのスレッド）。drain の終わりを `spawn_local` の
/// 見張りで `ended` に映し、pump が止まったら子のスレッドが知らせを送る。テストのスレッドは
/// 知らせを 5 秒の期限で待つので、`Err(Closed)` で回り続ける直し間違いでも止まらずに赤で終わる。
#[test]
fn dropping_every_sender_ends_the_ui_actor() {
    let (notice_tx, notice_rx) = std::sync::mpsc::channel::<bool>();
    let _pump = std::thread::Builder::new()
        .name("toy-ui-drop-pump".to_owned())
        .spawn(move || {
            let (ui_tx, drain) = spawn_ui("toy-ui-drop", handle)
                .expect("spawn_ui on the pump (child) thread must succeed");
            let ended = Arc::new(AtomicBool::new(false));
            let watch_ended = Arc::clone(&ended);
            let _watch = wintf_winmsg_executor::spawn_local(async move {
                drain.await;
                watch_ended.store(true, Ordering::Release);
            });
            let extra = ui_tx.clone();
            drop(ui_tx);
            drop(extra);

            run_pump_until(&ended, TEST_DEADLINE);
            let _ = notice_tx.send(ended.load(Ordering::Acquire));
        })
        .expect("pump thread must start");

    // 直し間違いでは子のスレッドが回り続けるので join せず、知らせだけを期限つきで待つ。
    match notice_rx.recv_timeout(TEST_DEADLINE) {
        Ok(ended) => assert!(
            ended,
            "the UI actor did not end after every sender was dropped"
        ),
        Err(e) => panic!("no end notice from the pump thread within the deadline: {e:?}"),
    }
}
