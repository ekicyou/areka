//! 純粋層: request/reply（oneshot 相当）の器。
//!
//! per-request に [`std::sync::mpsc::channel`] を 1 本生成し、薄い newtype 対
//! （[`ReplySender`]／[`ReplyReceiver`]）で包んで返す。自前 oneshot（Mutex/Condvar/
//! unsafe）は持たず、std mpsc の配送保証と drop 意味論をそのまま利用する（DD-4）。
//!
//! - [`ReplySender::send`] は `self` を consume し「応答は高々 1 回」を型で強制する
//!   （2.2 の oneshot 相当）。受信端が既に drop 済みなら未達値を `Err(value)` で返す。
//! - [`ReplyReceiver::recv`]／[`recv_timeout`](ReplyReceiver::recv_timeout) は、応答が
//!   送られる前に `ReplySender` が drop されると [`ReplyError::Dropped`] を返す（3.6・切断の
//!   自己シグナル・永久ブロックしない）。`recv_timeout` は期限超過を [`ReplyError::Timeout`]
//!   として `Dropped` と区別する。

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// request/reply 用の返信チャンネル対を生成する（oneshot 相当・per-request）。
///
/// 内部で [`mpsc::channel`] を 1 本生成し、送信端を [`ReplySender`]・受信端を
/// [`ReplyReceiver`] で包んで返す。任意スレッド間で使用可（`T: Send`）。
pub fn reply_channel<T: Send>() -> (ReplySender<T>, ReplyReceiver<T>) {
    let (tx, rx) = mpsc::channel::<T>();
    (ReplySender(tx), ReplyReceiver(rx))
}

/// request/reply の返信用送信端（oneshot 相当・応答は高々 1 回）。
///
/// メッセージ enum の variant にフィールドとして同梱し、受信側アクターがこれ 1 本へ
/// 応答を返す（envelope 規約）。応答未送信のまま drop されると、対応する
/// [`ReplyReceiver`] は [`ReplyError::Dropped`] を観測する（3.6）。
pub struct ReplySender<T>(Sender<T>);

impl<T: Send> ReplySender<T> {
    /// 応答を 1 回だけ送る（`self` consume で 2 回目を型が禁止する）。
    ///
    /// 受信側（[`ReplyReceiver`]）が既に drop 済みなら、送れなかった値を `Err(value)` で
    /// 返す（[`mpsc::SendError`] を剥がして値のみ返す・要求側キャンセルの観測）。
    pub fn send(self, value: T) -> Result<(), T> {
        self.0.send(value).map_err(|mpsc::SendError(value)| value)
    }
}

/// request/reply の返信用受信端（oneshot 相当・応答は高々 1 回受け取る）。
///
/// 対応する [`ReplySender`] が応答を送れば必ず受け取れる（2.3・mpsc の配送保証）。
/// 応答が送られず `ReplySender` が drop されれば [`ReplyError::Dropped`] を返す（3.6）。
pub struct ReplyReceiver<T>(Receiver<T>);

impl<T: Send> ReplyReceiver<T> {
    /// 応答を待つ（`self` consume）。
    ///
    /// [`ReplySender`] が応答せず drop された場合は [`ReplyError::Dropped`] を返す
    /// （std mpsc の [`RecvError`](std::sync::mpsc::RecvError)＝全 Sender drop を写像・
    /// 永久ブロックしない・3.6）。
    pub fn recv(self) -> Result<T, ReplyError> {
        self.0.recv().map_err(|_| ReplyError::Dropped)
    }

    /// 上限時間付きで応答を待つ（`self` consume）。
    ///
    /// 期限内に応答が来なければ [`ReplyError::Timeout`]、期限前に [`ReplySender`] が応答せず
    /// drop されれば [`ReplyError::Dropped`] を返す（両者を区別する・3.6）。
    pub fn recv_timeout(self, timeout: Duration) -> Result<T, ReplyError> {
        self.0.recv_timeout(timeout).map_err(|e| match e {
            RecvTimeoutError::Timeout => ReplyError::Timeout,
            RecvTimeoutError::Disconnected => ReplyError::Dropped,
        })
    }
}

/// request/reply の応答受信で観測し得る失敗。
#[derive(Debug, thiserror::Error)]
pub enum ReplyError {
    /// [`ReplySender`] が応答せず drop された（アクター停止・要求キャンセル）。
    #[error("reply sender dropped (actor stopped or request cancelled)")]
    Dropped,
    /// 上限時間内に応答が受信されなかった（[`ReplyReceiver::recv_timeout`] のみ）。
    #[error("reply not received within timeout")]
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // ケース1: 送受信往復。応答を送れば受信側は同じ値を受け取る（2.3）。
    #[test]
    fn round_trip_delivers_value() {
        let (tx, rx) = reply_channel::<u32>();
        tx.send(42).expect("send should succeed while receiver is alive");
        assert_eq!(rx.recv().expect("value should be delivered"), 42);
    }

    // ケース2: 送信端 drop → Dropped（3.6・永久ブロックしない）。
    #[test]
    fn sender_dropped_without_reply_yields_dropped() {
        let (tx, rx) = reply_channel::<u32>();
        drop(tx); // 応答せず drop。
        // Sender は既に drop 済みゆえ recv は即時 Err で復帰する（ハングしない）。
        match rx.recv() {
            Err(ReplyError::Dropped) => {}
            other => panic!("expected Err(Dropped), got {other:?}"),
        }
    }

    // ケース3: timeout → Timeout（Sender は生存させたまま応答しない）。
    #[test]
    fn timeout_is_distinct_from_dropped() {
        let (tx, rx) = reply_channel::<u32>();
        let start = std::time::Instant::now();
        let result = rx.recv_timeout(Duration::from_millis(50));
        // Sender を応答後まで生かしておく（drop を timeout より後にする）。
        assert!(start.elapsed() >= Duration::from_millis(50), "must wait the timeout");
        match result {
            Err(ReplyError::Timeout) => {}
            other => panic!("expected Err(Timeout), got {other:?}"),
        }
        drop(tx);
    }

    // 追加: recv_timeout 経由の往復成功。
    #[test]
    fn recv_timeout_returns_value_when_replied() {
        let (tx, rx) = reply_channel::<&'static str>();
        thread::spawn(move || {
            tx.send("pong").expect("reply while receiver waits");
        });
        let got = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("value should arrive before timeout");
        assert_eq!(got, "pong");
    }

    // 追加: 受信端 drop 後の send は未達値を Err(value) で返す。
    #[test]
    fn send_after_receiver_dropped_returns_value() {
        let (tx, rx) = reply_channel::<String>();
        drop(rx);
        match tx.send("orphan".to_owned()) {
            Err(value) => assert_eq!(value, "orphan"),
            Ok(()) => panic!("send to a dropped receiver must fail with the value"),
        }
    }

    // 追加: recv_timeout でも Sender drop は Dropped（Timeout と区別）。
    #[test]
    fn recv_timeout_sees_dropped_when_sender_gone() {
        let (tx, rx) = reply_channel::<u32>();
        drop(tx);
        match rx.recv_timeout(Duration::from_secs(5)) {
            Err(ReplyError::Dropped) => {}
            other => panic!("expected Err(Dropped), got {other:?}"),
        }
    }
}
