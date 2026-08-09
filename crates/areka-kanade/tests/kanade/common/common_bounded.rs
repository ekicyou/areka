//! 期限付き待機ヘルパ（どのテストもハングしない・Req 7.3）。
//!
//! `common/mod.rs`（1,657 行）から責務単位で切り出した子モジュール（タスク 8.2）。
//! 項目は親のファサードから再輸出されるため、消費側の `super::common::X` は不変である。

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use areka_actor::{ActorError, ActorHandle};
use areka_kanade::{KanadeMsg, MonotonicMs};

/// 期限付き待機の既定上限（mock は即応ゆえ十分に余裕を持たせた保険値）。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

// ============================================================================
// ハング検出ヘルパ
// ============================================================================

/// 期限付き待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければテストを
/// 失敗させる（どのテストもハングしない・Req 7.3）。areka-actor のテスト慣行に倣う。
pub fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    run_join_bounded(what, timeout, f);
}

/// 内部: クロージャを別スレッドで走らせ、`recv_timeout` の期限で完了を判定する。
pub(super) fn run_join_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    use std::sync::mpsc::sync_channel;
    let (done_tx, done_rx) = sync_channel::<()>(0);
    thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// [`ActorHandle`] を期限付きで join する（ハングせず結果を返す）。
///
/// 停止駆動（Close 送信／全 Sender drop）を先に済ませてから呼ぶこと。期限内に join が
/// 完了しなければテストを失敗させる。
pub fn join_bounded(what: &str, timeout: Duration, handle: ActorHandle) -> Result<(), ActorError> {
    use std::sync::mpsc::sync_channel;
    let (res_tx, res_rx) = sync_channel::<Result<(), ActorError>>(0);
    thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
    }
}

/// Tick を 1 秒刻みで送り続け、kanade の終了（inbox 切断＝send Err）で戻る。
/// quit:true talk の帰結として終了が必然の台本でのみ使うこと。
/// kanade が終了しない（欠陥）場合は DEFAULT_TIMEOUT の壁時計 deadline で
/// ハングでなく panic（失敗）として検出する。
///
/// 復帰駆動の完了バリア（R7' 新構造・7.1/7.3/8.5）。`first_tick_second` から 1 秒刻みで
/// 単調増加する `now`（＝`MonotonicMs(second * 1_000)`）を持つ [`KanadeMsg::Tick`] を、
/// `sender.send` が `Err`（Receiver drop＝kanade スレッド終了＝inbox 切断）を返すまで
/// 反復送出する（反復回数の上限は持たない・上限非依存の完了バリア）。
///
/// 供給ペーシングは送出ごとの [`std::thread::yield_now`] 1 回で足る（kanade へ処理を譲る）。
/// 滞留した Tick は切断時に破棄され意味論に影響しない（設計 Implementation Notes）。
///
/// # 非空虚性（ハング→失敗変換・7.3）
/// kanade が終了しない欠陥時は send が成功し続けるが、`DEFAULT_TIMEOUT` の
/// [`std::time::Instant`] deadline をループ内で毎回判定し、超過したら `what` を含む
/// 説明的メッセージで [`panic!`] する（silent hang を作らない）。
pub fn drive_ticks_until_disconnect(
    sender: &Sender<KanadeMsg>,
    first_tick_second: u64,
    what: &str,
) {
    let deadline = std::time::Instant::now() + DEFAULT_TIMEOUT;
    let mut second = first_tick_second;
    loop {
        // 切断（Receiver drop＝kanade 終了）で戻る＝完了バリア。上限回数は持たない。
        if sender
            .send(KanadeMsg::Tick {
                now: MonotonicMs(second * 1_000),
            })
            .is_err()
        {
            return;
        }
        // 供給ペーシング: 送出ごとに短い backoff sleep で kanade ワーカースレッドへ CPU を
        // 明け渡す。`yield_now()`（Windows: `SwitchToThread`＝同一プロセッサの ready スレッドのみに
        // 譲る）は、`cargo test --workspace` の並列実行でコア数を超えるスレッド（多数の協調ループ檻
        // ＋各 kanade ワーカー）が走る飽和下では kanade ワーカーへ確実に譲れず、producer の busy-spin が
        // worker を CPU 飢餓させて `DEFAULT_TIMEOUT` を偽陽性で踏む（実ハングではなく競合飢餓＝単独/
        // 直列では緑・並列で赤・失敗数も負荷依存で変動する非決定 flake）。短い sleep はスレッドを実際に
        // deschedule するため飽和下でも worker が確実に前進でき、本ループの終了は少数の論理 tick で必然
        // ゆえ総遅延は無視できる（deadline には到達しない）。
        std::thread::sleep(Duration::from_micros(200));
        // ハング検出: deadline 超過は必ず panic（kanade 非終了の欠陥を失敗へ変換）。
        if std::time::Instant::now() >= deadline {
            panic!(
                "'{what}' did not disconnect within {DEFAULT_TIMEOUT:?} \
                 (kanade failed to terminate; possible hang)"
            );
        }
        second += 1;
    }
}
