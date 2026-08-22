//! Task 2.3 実現可能性スパイク（検証専用・独立バイナリ integration test）。
//!
//! # 目的（要件 8.2 / 8.3・design.md「toy tests」Risks / validation Issue 1）
//! design は「`spawn_local` ＋ `MessageLoop::run` ＋ `PostThreadMessageW`(heartbeat) の
//! 三点同時使用は in-repo 前例がない」ことを最大リスク（Issue 1）とし、本ユニットの
//! 実装系タスクの先頭にこの組合せ確認スパイクを置くことを求めた。特に「`MessageLoop::run`
//! **単独**で `spawn_local` タスクが poll されるか」が最初の確認事項である。
//!
//! 本スパイクは spawn.rs / reply.rs から**独立**に（それらのコードへ一切依存せず・reply の
//! 返信路にはローカルの素の `std::sync::mpsc` を用いて）、UI スレッド役の 1 本の echo 往復で
//! 三点組合せが成立することを機械 pass/fail（8.2/8.3）で観測する。
//!
//! # 確定した方式: PRIMARY（`spawn_local` ＋ `MessageLoop::run` ＋ `PostThreadMessageW` heartbeat）
//! スパイクは **PRIMARY 組合せで成立**する。フォールバック（block_on / message-only 窓＋
//! PostMessageW）は不要。根拠は executor 0.0.5 のソース実測:
//!
//! - `MessageLoop::run` は `spawn_local` タスクを poll する。`spawn_local` は runnable を
//!   `PostMessageW(EXECUTOR_WINDOW, MSG_ID_WAKE, .., runnable_ptr)` でスケジュールし、
//!   executor 窓の WndProc が dispatch 時に runnable を実行する。`MessageLoop::run` の
//!   `run_loop` は `GetMessageW`→`DispatchMessageW` で全メッセージ（wake 含む・wake は
//!   filter から保護）を配送するため、`block_on` を経由せずとも同一スレッドの spawn_local
//!   タスクが駆動される（executor rustdoc「Like block_on(), this function runs any tasks
//!   spawned … using spawn_local()」・crate 内 `disallow_wake_message_filtering` テストが
//!   `spawn_local`→`msg_loop.quit()` の駆動を実証）。
//! - heartbeat（別スレッドから `PostThreadMessageW(ui_tid, WM_NULL)`）は `GetMessageW` を
//!   無入力でも起こし、filter が deadline / done フラグを再評価できるようにする。WM_NULL は
//!   hwnd なしスレッドメッセージゆえ `DispatchMessageW` で無害に消費される。
//!   （`parent_window.rs::pump_until_hello_or` の bounded-pump 規律の写経。ただし本スパイクは
//!   HWND を一切生成しない＝executor 内部の message-only 窓のみで完結する。）
//!
//! この結論により後続タスク 3（`spawn_ui`）・4.2（toy(b)）は PRIMARY 組合せを実装方針として
//! 確定してよい（公開 API 不変のフォールバック局所差し替えは不要）。
//!
//! # bounded 保証
//! filter は「done フラグ or deadline 超過」で `msg_loop.quit()` する。heartbeat が無入力でも
//! ~25ms 間隔で `GetMessageW` を起こすため、echo が届かなくても deadline（5 秒）で必ず抜ける
//! （ハングしない）。echo 受領で done が立ち、次の heartbeat 起床で即 quit する。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_NULL};

use wintf_winmsg_executor::{FilterResult, MessageLoop, spawn_local};

/// heartbeat（別スレッドからの `PostThreadMessageW(WM_NULL)`）の送信間隔。
/// 無入力でも `GetMessageW` をブロックさせ続けず、filter が done / deadline を再評価できる
/// ようにする（`parent_window.rs` の `HEARTBEAT_INTERVAL` 写経）。
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(25);

/// スパイクの上限時間。echo 不達でもこの期限で必ず pump を抜ける（bounded 保証）。
const SPIKE_DEADLINE: Duration = Duration::from_secs(5);

/// worker → UI drain タスクへ送る echo 要求。返信路はローカルの素の `std::sync::mpsc`
/// （spike の独立性のため reply.rs を使わない）を同梱する。
struct EchoRequest {
    payload: String,
    reply: std::sync::mpsc::Sender<String>,
}

/// 三点組合せ（spawn_local + MessageLoop::run + PostThreadMessageW heartbeat）の最小 echo 往復。
///
/// 観測:
/// - drain タスク（`spawn_local`）が `MessageLoop::run` **単独**（block_on 非経由）で poll され、
///   `recv().await` で echo 要求を受領して返信する（Issue 1 の主眼）。
/// - worker が返信を受領する → done フラグが立ち、bounded pump が deadline 前に quit する。
///
/// deadline 超過（echo 不達）は `assert!` 失敗として観測される（8.3）。
#[test]
fn spike_primary_spawn_local_message_loop_heartbeat_echo() {
    // UI スレッド役（＝本テストスレッド）のスレッド ID。heartbeat 先。
    // SAFETY: GetCurrentThreadId は引数なしで現スレッド ID を返す純粋な取得。
    let ui_thread_id = unsafe { GetCurrentThreadId() };

    // UI 側 inbox（unbounded async-channel）。worker → drain タスクの搬送路。
    let (ui_tx, ui_rx) = async_channel::unbounded::<EchoRequest>();

    // worker が echo を受領したかの観測フラグ（bounded pump の quit 条件）。
    let done = Arc::new(AtomicBool::new(false));

    // --- drain タスクを UI スレッド（＝本スレッド）へ spawn_local で投入 ---
    // recv().await で 1 件受領 → payload を echo（"<payload>" をそのまま返す）→ 次の recv へ。
    // ここでは 1 往復のみ観測すれば十分ゆえ、echo 後もループを続けるが Close は使わない
    // （done フラグ＋deadline で pump を抜けるため drain タスクの明示終了は不要＝
    //  MessageLoop::run 復帰時に suspend される。executor 仕様）。
    let drain = spawn_local(async move {
        while let Ok(req) = ui_rx.recv().await {
            // handler は await を跨がず即応答（UI スレッド束縛リソースを安全に扱える形の実証）。
            let echoed = format!("echo:{}", req.payload);
            // 返信路はローカル std mpsc。受信側が生きていれば届く。
            let _ = req.reply.send(echoed);
        }
    });

    // --- worker スレッド: UiSender 相当（async-channel Sender）で 1 件送信し返信を待つ ---
    let worker_done = Arc::clone(&done);
    let worker = std::thread::Builder::new()
        .name("spike-echo-worker".to_owned())
        .spawn(move || {
            // ローカル返信路（reply.rs 非依存・素の std mpsc）。
            let (reply_tx, reply_rx) = std::sync::mpsc::channel::<String>();
            // try_send（unbounded ゆえ Full なし・失敗は Closed のみ）。
            ui_tx
                .try_send(EchoRequest {
                    payload: "ping".to_owned(),
                    reply: reply_tx,
                })
                .expect("spike: UI inbox への try_send が Closed で失敗（drain 未起動？）");

            // 返信を上限時間付きで待つ（永久ブロックしない）。届けば echo 一致を検証。
            let got = reply_rx
                .recv_timeout(SPIKE_DEADLINE)
                .expect("spike: 上限時間内に echo 返信を受領できなかった（三点組合せ不成立）");
            assert_eq!(got, "echo:ping", "echo 内容が一致すべき");

            // 受領観測 → done を立てて bounded pump に quit させる。
            worker_done.store(true, Ordering::Release);

            // drain タスクを quit 条件成立後に安全に抜けさせるため sender を drop する
            // （ui_tx はこのクロージャが所有＝スコープ終了で drop → recv() が Err(Closed)）。
        })
        .expect("spike: worker スレッド起動に失敗");

    // --- heartbeat: 別スレッドから UI スレッドへ WM_NULL を撃ち、GetMessageW を起こす ---
    // done が立つ or deadline 超過まで ~25ms 間隔で送出。送出失敗（キュー未生成の一瞬）は無視。
    let hb_stop = Arc::new(AtomicBool::new(false));
    let heartbeat = {
        let hb_stop = Arc::clone(&hb_stop);
        std::thread::Builder::new()
            .name("spike-heartbeat".to_owned())
            .spawn(move || {
                while !hb_stop.load(Ordering::Acquire) {
                    // SAFETY: WM_NULL(0) は無害な起こし用スレッドメッセージ。ui_thread_id は
                    // 本テストスレッド（生存中・pump 実行中）。送出失敗はキュー未生成の一瞬のみで無害。
                    unsafe {
                        let _ = PostThreadMessageW(ui_thread_id, WM_NULL, WPARAM(0), LPARAM(0));
                    }
                    std::thread::sleep(HEARTBEAT_INTERVAL);
                }
            })
            .expect("spike: heartbeat スレッド起動に失敗")
    };

    // --- bounded pump: MessageLoop::run 単独で drain タスクを駆動しつつ done/deadline で quit ---
    let deadline = Instant::now() + SPIKE_DEADLINE;
    MessageLoop::run(|msg_loop, _msg| {
        // done（echo 受領）または deadline 超過で即 quit。heartbeat が無入力でもこの
        // filter を定期的に起こすため bounded に抜けられる。
        if done.load(Ordering::Acquire) || Instant::now() >= deadline {
            msg_loop.quit();
        }
        FilterResult::Forward
    });

    // pump 復帰後の後始末（bounded であることの確認込み）。
    hb_stop.store(true, Ordering::Release);
    let _ = heartbeat.join();
    let _ = worker.join();
    // drain タスクは MessageLoop::run 復帰で suspend 済み。detach（drop）で背景放置しない
    // よう明示 drop（JoinHandle drop は detach＝タスク継続だが、ここでは pump が止まっており
    // 再 poll されないため安全に破棄できる）。
    drop(drain);

    // 単一の pass/fail 観測（8.2/8.3）: echo が deadline 前に往復したなら done が立つ。
    assert!(
        done.load(Ordering::Acquire),
        "spike: 三点組合せ（spawn_local + MessageLoop::run + PostThreadMessageW）で \
         echo 往復が成立せず deadline を超過した（PRIMARY 不成立＝フォールバック検討が必要）"
    );
}
