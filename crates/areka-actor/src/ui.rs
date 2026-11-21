//! UI ブリッジ層: message pump 上で動く UI アクターへ、他スレッドから queue＋wakeup で
//! メッセージを届け、pump 内で drain させる配送ブリッジ。
//!
//! 純粋層（[`crate::spawn`]／[`crate::reply`]＝std のみ）には**コード依存しない**。queue＋wakeup は
//! [`async_channel`]（unbounded）が一体で担い、drain タスクの pump への投入は
//! [`wintf_winmsg_executor::spawn_local`] が担う。受信ループの耐障害規約は
//! [`crate::spawn::run_inbox`] を呼ばず、本モジュール内で**独立に同一規約**を実装する
//! （spawn モジュールへのコード依存を持たない）。
//!
//! # 起床機構（手組みしない）
//!
//! `UiSender::send` は unbounded の [`async_channel::Sender::try_send`]＝**Full が存在せず送信は
//! 決してブロックしない**（失敗経路は Closed のみ）。起床は async-channel 内部の waker
//! 機構（登録済み drain タスクの waker への wake）→ executor のクロススレッド waker（`PostMessageW`）
//! が pump を起こす、という既成の経路に委ねる。store→notify／listen-before-work の規律は
//! async-channel 内部が持つため、本クレートは手組みしない。
//!
//! # スレッド構成の不変
//!
//! 新スレッドを一切作らない。UI スレッドの MTA・render/window 固定・D2D 単一スレッド前提は不変。
//! 搬送する `M` は任意の `Send + 'static`＝emo-present の指令メッセージ・窓移動指令の
//! 型を下流がそのまま載せられる（器のみ提供）。

use std::ops::ControlFlow;

use tracing::Instrument;

/// UI スレッド上で UI アクター（pump 内 drain ループ）を起動する。
///
/// # 呼び出しスレッドの前提（重要）
///
/// **必ず message pump を回す UI スレッド**——このあと [`wintf_winmsg_executor::MessageLoop::run`]
/// （または wintf `WinApp::run` の `block_on`）を実行するスレッド——から呼ぶこと。`spawn_local` は
/// 呼び出しスレッドに紐づく executor 窓（thread-local・message-only）へ drain タスクをスケジュール
/// するため、そのスレッドで pump が回らなければ **drain タスクは永久に走らない**（executor 0.0.5
/// の構造上、この誤用は呼び出し時点では検出できない＝下記 Risks 参照）。
///
/// spawn 時に [`tracing::debug!`]（アクター名）を必ず記録する（誤用時の診断文脈を残す・ログ規律）。
///
/// # handler の戻り規約（[`run_inbox`](crate::spawn::run_inbox) と同一）
///
/// drain タスクは各メッセージを `handler` へ渡し、戻りで挙動を固定する:
///
/// - `Ok(ControlFlow::Continue(()))` → 次の受信へ。
/// - `Ok(ControlFlow::Break(()))` → 即時終了（消費者定義の Close variant 受領時）。
///   タスク所有の `Receiver` が drop され、queue の積み残しは破棄される。積み残しに
///   同梱された reply Sender も drop され、要求側は切断（`Err`）を観測する。
/// - `Err(e)` → [`tracing::error!`]（アクター名＋`%e`）を記録して drain を**継続**する。個別
///   メッセージの処理失敗はループを殺さない。
///
/// 終了経路はちょうど 2 つ: handler の `Ok(Break)` と、全 [`UiSender`] drop による `recv()` の
/// `Err(Closed)`。**handler の `Err` は終了経路ではない。**
///
/// handler は `!Send` 可（UI スレッドに束縛されたリソース——COM ポインタ・D2D デバイス等——を
/// 握れる。emo-present 適合）。handler は await を跨がず同期実行される（UI スレッド束縛リソースを
/// 安全に扱える）。
///
/// # 戻り値
///
/// `(`[`UiSender`]`, `[`JoinHandle`](wintf_winmsg_executor::JoinHandle)`)`。`UiSender` は
/// Clone・Send な送信端。`JoinHandle` は drain タスクの join ハンドルだが、**drop してもタスクは
/// 停止しない**（executor 仕様＝detach。停止は Close／全 `UiSender` drop の 2 経路のみ）。
///
/// # Risks（誤用時挙動・log-first / 安易な panic 禁止）
///
/// UI スレッド外から呼んだ場合の素の挙動: executor 0.0.5 の `spawn_local` は呼び出しスレッドの
/// thread-local executor 窓を**その場で生成**し、runnable をそこへ `PostMessageW` する。当該スレッド
/// で pump が回らなければメッセージは配送されず、**drain タスクは静かに走らないまま**になる
/// （panic もエラーも起きない silent failure）。これを `Err` へ引き上げる健全性確認は、
/// 呼び出し時点で「このスレッドで pump がこれから回るか」を知る術が無い（pump は本関数の**後**に
/// 呼び出し側が開始する）ため、本クレート単独では不能。よって既知リスクとして本 rustdoc に明記し、
/// 正用法は toy(b)（実走 pump）で担保する。silent failure に**ログが無い**状態だけは避けるため、
/// spawn 時 `debug!(actor)` を必ず残す（診断文脈）。
pub fn spawn_ui<M, E>(
    name: &str,
    mut handler: impl FnMut(M) -> Result<ControlFlow<()>, E> + 'static,
) -> (UiSender<M>, wintf_winmsg_executor::JoinHandle<()>)
where
    M: Send + 'static,
    E: std::fmt::Display + 'static,
{
    tracing::debug!(actor = %name, "spawn_ui: submitting UI drain task (must be the pump thread)");

    let (tx, rx) = async_channel::unbounded::<M>();
    // 区間は poll の間だけ入る（待ちの間に UI スレッドのほかの記録へ漏らさない）。
    let span = tracing::info_span!("actor", actor = %name);

    // drain タスク。受信ループの耐障害規約は run_inbox を呼ばず本タスク内で独立に同一実装する
    // （spawn モジュール非依存）。
    let handle = wintf_winmsg_executor::spawn_local(
        async move {
            loop {
                match rx.recv().await {
                    Err(async_channel::RecvError) => return,
                    Ok(msg) => match handler(msg) {
                        Ok(ControlFlow::Continue(())) => continue,
                        Ok(ControlFlow::Break(())) => return,
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "UI actor message handler returned error; continuing"
                            );
                            continue;
                        }
                    },
                }
            }
        }
        .instrument(span),
    );

    (UiSender { tx }, handle)
}

/// UI アクター宛の送信端（Clone・Send・非ブロック）。
///
/// unbounded な [`async_channel::Sender`] の newtype。任意スレッドから複製・送信でき
/// （`M: Send`）、[`send`](UiSender::send) は決してブロックしない（unbounded ゆえ Full 無し）。
/// 全 `UiSender` が drop されると drain タスクの `recv()` が `Err(Closed)` となり
/// 受信ループは正常終了する。
pub struct UiSender<M> {
    tx: async_channel::Sender<M>,
}

impl<M> Clone for UiSender<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<M: Send> UiSender<M> {
    /// メッセージを queue へ積む（[`try_send`](async_channel::Sender::try_send)・unbounded ゆえ
    /// 常に即時・ブロックしない）。起床は channel 内部の waker 経由で pump へ伝わる。
    ///
    /// # Errors
    ///
    /// UI アクター停止後（drain タスク終了＝`Receiver` drop で channel closed）は
    /// [`UiSendError`] を返す。エラーは未達メッセージを同梱し、送信側は「相手が停止済み」を
    /// 同期観測できる（[`std::sync::mpsc::SendError`] と同型の切断観測）。unbounded ゆえ Full は
    /// 存在せず、失敗経路は Closed のみである。
    pub fn send(&self, msg: M) -> Result<(), UiSendError<M>> {
        self.tx.try_send(msg).map_err(|e| match e {
            async_channel::TrySendError::Closed(msg) => UiSendError(msg),
            // unbounded ゆえ Full は構造上発生しない。到達した場合も未達値を切断として返す。
            async_channel::TrySendError::Full(msg) => UiSendError(msg),
        })
    }
}

/// [`UiSender::send`] の失敗（UI アクター停止後の送信）。未達メッセージを同梱して返す。
///
/// [`std::sync::mpsc::SendError`] と同型（切断＝相手停止済みの同期観測）。
#[derive(Debug, thiserror::Error)]
#[error("UI actor inbox closed (drain task stopped)")]
pub struct UiSendError<M>(pub M);

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト専用: 走行中の pump（`spawn_local`）を要さずに `UiSender` を構築するための
    /// 内部シーム。同期部分（queue 格納・Closed 変換）だけを pump 非依存で検証する。
    /// 実走 pump 上の drain 挙動は toy(b)（integration test）で検証する。
    fn ui_sender_for_test<M>() -> (UiSender<M>, async_channel::Receiver<M>) {
        let (tx, rx) = async_channel::unbounded::<M>();
        (UiSender { tx }, rx)
    }

    #[test]
    fn send_stores_message_in_queue() {
        let (sender, rx) = ui_sender_for_test::<&'static str>();
        sender
            .send("cmd-1")
            .expect("send to a live inbox must succeed");
        sender
            .send("cmd-2")
            .expect("send to a live inbox must succeed");

        assert_eq!(rx.try_recv().expect("first message queued"), "cmd-1");
        assert_eq!(rx.try_recv().expect("second message queued"), "cmd-2");
        assert!(rx.try_recv().is_err(), "queue should be drained now");
    }

    #[test]
    fn send_after_receiver_dropped_returns_undelivered_message() {
        let (sender, rx) = ui_sender_for_test::<String>();
        drop(rx);

        match sender.send("late-cmd".to_owned()) {
            Err(UiSendError(msg)) => {
                assert_eq!(msg, "late-cmd", "未達メッセージが回収できること");
            }
            Ok(()) => panic!("send to a closed inbox must fail with the undelivered message"),
        }
    }

    #[test]
    fn cloned_sender_targets_same_inbox() {
        let (sender, rx) = ui_sender_for_test::<u32>();
        let clone = sender.clone();
        sender.send(1).expect("original sender send");
        clone.send(2).expect("cloned sender send");

        assert_eq!(rx.try_recv().expect("from original"), 1);
        assert_eq!(rx.try_recv().expect("from clone"), 2);
    }
}
