//! toy(b) 統合試験（独立プロセス・bounded pump）: worker→UI pump 実走 echo 往復。
//!
//! # 目的（要件 8.2 / 8.3・design.md「toy tests」toy(b)・System Flows「UI 配送ブリッジ echo」）
//! task 3 で実装した `ui` モジュール（`spawn_ui`／`UiSender`）と `reply` モジュール
//! （`reply_channel`／`ReplyReceiver`）の**公開 API**を、task 2.3 スパイクで確定した
//! PRIMARY 組合せ（`spawn_ui`＝内部で `spawn_local`／`MessageLoop::run`／`PostThreadMessageW`
//! heartbeat）の**実走 pump**上で駆動し、worker→UI アクターへの echo 往復が成立することを
//! 単一の機械 pass/fail（8.2/8.3）で観測する。
//!
//! # スパイクとの差分
//! `spike_ui_pump.rs`（task 2.3）は spawn.rs／reply.rs から**独立**に、生の async-channel と
//! 素の std mpsc で三点組合せの成立可否だけを確認した。本 toy(b) はその成立を前提に、
//! 実際の `spawn_ui`／`UiSender` ブリッジと `reply_channel` の envelope パターンを**そのまま**
//! 通して echo を往復させる（＝task 3 の公開 API の正用法担保・design「正用法は toy(b) で担保」）。
//!
//! # スレッド構成（重要・tasks.md Implementation Notes / ui.rs Risks）
//! 本テストスレッドが **UI（pump）スレッド**を演じる。`spawn_ui` は呼び出しスレッドの
//! thread-local executor 窓へ drain タスクをスケジュールするため、`spawn_ui` と
//! `MessageLoop::run` は**必ず同一（本テスト）スレッド**で実行する。別スレッドで `spawn_ui`
//! すると drain タスクが走らず deadline 失敗（silent = case (c)）になる。worker と heartbeat は
//! 別スレッド。
//!
//! # bounded 保証（ハングしない）
//! filter は「done フラグ（echo 受領）or deadline 超過」で `msg_loop.quit()` する。heartbeat が
//! 無入力でも ~25ms 間隔で `GetMessageW` を起こすため、echo が届かなくても deadline（5 秒）で
//! 必ず pump を抜ける。echo 受領で done が立ち、次の heartbeat 起床で即 quit する。
//! deadline 超過・echo 内容不一致・応答不達はいずれも `assert!` 失敗として観測される（8.3）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::ops::ControlFlow;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_NULL};

use wintf_winmsg_executor::{FilterResult, MessageLoop};

use areka_actor::{reply_channel, spawn_ui, ReplyError, ReplySender};

/// heartbeat（別スレッドからの `PostThreadMessageW(WM_NULL)`）の送信間隔。
/// 無入力でも `GetMessageW` をブロックさせ続けず、filter が done / deadline を再評価できる
/// ようにする（bounded 保証の要・spike_ui_pump.rs の写経）。
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(25);

/// toy(b) の上限時間。echo 不達でもこの期限で必ず pump を抜ける（CI 余裕込み・design 例 5 秒）。
const TEST_DEADLINE: Duration = Duration::from_secs(5);

/// UI アクター宛メッセージ（消費者定義の enum・envelope 規約）。
///
/// `Echo` は request/reply variant として返信端 [`ReplySender`] を同梱する（2.2 の oneshot 相当）。
/// `Close` は横断制御の停止メッセージ（3.1）で、drain タスクを即時終了させる。
enum UiMsg {
    /// echo 要求: `payload` をそのまま返信端へ返す。
    Echo {
        payload: String,
        reply: ReplySender<String>,
    },
    /// 停止（即時終了・handler は `Ok(Break)` を返す）。
    Close,
}

/// 実走 pump 上での worker→UI echo 往復（PRIMARY 組合せ・task 3 公開 API 経由）。
///
/// 観測:
/// - worker が `UiSender::send(Echo{..})` で要求を積む → `spawn_ui` の drain タスクが
///   `MessageLoop::run` 実走 pump 上で受領し、handler が `reply.send(payload)` で返信する。
/// - worker が `ReplyReceiver::recv_timeout` で echo を受領し内容一致（"ping"）を検証 → done を立て、
///   `UiSender::send(Close)` で drain を止める。
/// - filter が done で `quit()` → bounded pump が deadline 前に復帰する。
///
/// deadline 超過（echo 不達）・echo 内容不一致は `assert!`／`assert_eq!` 失敗として観測される（8.3）。
#[test]
fn toy_b_worker_to_ui_pump_echo_round_trip() {
    // UI（pump）スレッド役＝本テストスレッドの ID を worker 起動**前**に取得する（heartbeat 先）。
    // SAFETY: GetCurrentThreadId は引数なしで現スレッド ID を返す純粋な取得。
    let ui_thread_id = unsafe { GetCurrentThreadId() };

    // worker が echo を受領し検証まで終えたかの観測フラグ（bounded pump の quit 条件）。
    let done = Arc::new(AtomicBool::new(false));

    // --- UI アクターを本（＝UI/pump）スレッドへ spawn_ui で起動 ---
    // handler は await を跨がず同期実行される（UI スレッド束縛リソースを安全に扱える形の実証）。
    // Echo → 返信端へ payload をそのまま返して受信継続（Continue）。Close → 即時終了（Break）。
    // handler は失敗しない（返信の送信不能＝要求側キャンセルは握り潰す）ため、エラー型は
    // `Infallible` を明示する（`spawn_ui` の `E: Display` を満たしつつ never を型で表す）。
    let (ui_tx, _drain) = spawn_ui("toy-ui-echo", |msg: UiMsg| -> Result<ControlFlow<()>, std::convert::Infallible> {
        match msg {
            UiMsg::Echo { payload, reply } => {
                // 返信端が生きていれば届く。要求側キャンセル（Receiver drop）は Err(value) だが
                // handler はそれで死なない（値を捨てて継続・器の責務外）。
                let _ = reply.send(payload);
                Ok(ControlFlow::Continue(()))
            }
            UiMsg::Close => Ok(ControlFlow::Break(())),
        }
    })
    .expect("spawn_ui on the pump (test) thread must succeed");

    // --- worker スレッド: UiSender 経由で Echo を送り、返信を上限時間付きで待つ ---
    let worker_ui_tx = ui_tx.clone();
    let worker_done = Arc::clone(&done);
    let worker = std::thread::Builder::new()
        .name("toy-ui-echo-worker".to_owned())
        .spawn(move || {
            // per-request の返信路（reply.rs の公開 API・envelope パターンの正用）。
            let (reply_tx, reply_rx) = reply_channel::<String>();

            // Echo 要求を UI アクターの inbox へ積む（unbounded ゆえ Full なし・非ブロック・4.2）。
            // 失敗経路は Closed のみ（drain 未起動＝UI アクター停止）。UiMsg は ReplySender 同梱ゆえ
            // Debug 非実装につき `.expect` は使えない。明示 match で切断を試験失敗として観測する。
            if worker_ui_tx
                .send(UiMsg::Echo {
                    payload: "ping".to_owned(),
                    reply: reply_tx,
                })
                .is_err()
            {
                panic!("UiSender::send(Echo) failed (inbox closed — UI drain not live)");
            }

            // 返信を上限時間付きで待つ（永久ブロックしない）。不達は Timeout/Dropped として観測。
            let got = reply_rx.recv_timeout(TEST_DEADLINE);
            match got {
                Ok(echo) => {
                    // 応答不一致は試験失敗（8.3）。
                    assert_eq!(echo, "ping", "echoed payload must match the request");
                }
                Err(ReplyError::Timeout) => {
                    panic!("echo not received within the deadline (worker→UI pump echo failed)");
                }
                Err(ReplyError::Dropped) => {
                    panic!("reply sender dropped before echo (UI drain never handled the request)");
                }
            }

            // 受領＋一致を確認 → done を立てて bounded pump に quit させる。
            worker_done.store(true, Ordering::Release);

            // drain タスクを止める（Close＝即時終了・3.2）。停止不達でも done＋deadline で pump は抜ける。
            let _ = worker_ui_tx.send(UiMsg::Close);
        })
        .expect("worker thread must start");

    // --- heartbeat: 別スレッドから UI スレッドへ WM_NULL を撃ち、GetMessageW を起こす ---
    // done が立つ or deadline 超過まで ~25ms 間隔で送出。送出失敗（キュー未生成の一瞬）は無視。
    let hb_stop = Arc::new(AtomicBool::new(false));
    let heartbeat = {
        let hb_stop = Arc::clone(&hb_stop);
        std::thread::Builder::new()
            .name("toy-ui-echo-heartbeat".to_owned())
            .spawn(move || {
                while !hb_stop.load(Ordering::Acquire) {
                    // SAFETY: WM_NULL(0) は無害な起こし用スレッドメッセージ。ui_thread_id は
                    // 本テストスレッド（pump 実行中・生存）。送出失敗はキュー未生成の一瞬のみで無害。
                    unsafe {
                        let _ = PostThreadMessageW(ui_thread_id, WM_NULL, WPARAM(0), LPARAM(0));
                    }
                    std::thread::sleep(HEARTBEAT_INTERVAL);
                }
            })
            .expect("heartbeat thread must start")
    };

    // --- bounded pump: MessageLoop::run 単独で drain タスクを駆動しつつ done/deadline で quit ---
    // filter は wake / その他メッセージを Forward（DispatchMessageW へ流す）。wake は drain タスクの
    // 再開に必須ゆえ filter で握り潰さない。
    let deadline = Instant::now() + TEST_DEADLINE;
    MessageLoop::run(|msg_loop, _msg| {
        // done（echo 往復完了）または deadline 超過で即 quit。heartbeat が無入力でもこの
        // filter を定期的に起こすため bounded に抜けられる（ハングしない）。
        if done.load(Ordering::Acquire) || Instant::now() >= deadline {
            msg_loop.quit();
        }
        FilterResult::Forward
    });

    // pump 復帰後の後始末（bounded であることの確認込み）。
    hb_stop.store(true, Ordering::Release);
    let _ = heartbeat.join();
    worker.join().expect("worker thread must not panic");
    // drain タスクは Close 受領で終了済み。JoinHandle drop は detach（executor 仕様）だが、
    // pump は既に止まっており再 poll されないため安全に破棄できる。
    drop(_drain);
    // 全 UiSender を drop（本スレッド保持分）。worker 保持分は worker スコープ終了で drop 済み。
    drop(ui_tx);

    // 単一の pass/fail 観測（8.2/8.3）: echo が deadline 前に往復したなら done が立つ。
    assert!(
        done.load(Ordering::Acquire),
        "worker→UI pump echo round-trip did not complete before the deadline \
         (echo undelivered / mismatched / drain not driven on MessageLoop::run)"
    );
}
