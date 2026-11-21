//! `spawn_relay` — 相互依存の循環を解く汎用転送コンポーネント。
//!
//! 上流からのメッセージを型変換しつつ下流へ中継する。上流が全て手放されたら
//! 明示停止なしで自然に終了する。

use std::sync::mpsc::{Receiver, Sender};

use areka_actor::ActorHandle;

/// `rx` から受けたメッセージを `From` 変換して `tx` へ流す。上流全 `Sender` drop で自然終了。
/// 下流切断（送出 `Err`）は `warn!` の上で終了する（宙吊りなし）。
///
/// `rx` は呼び出し側が所有する受信端で、送信端は返さない。スレッド名・tracing span・
/// panic 安全性は `spawn_actor` のものを使い、`spawn_actor` が作る inbox は捨てる。
pub fn spawn_relay<A, B>(name: &str, rx: Receiver<A>, tx: Sender<B>) -> ActorHandle
where
    A: Send + 'static,
    B: From<A> + Send + 'static,
{
    let relay_name: String = name.to_string();
    let (_unused_tx, handle) = areka_actor::spawn_actor::<A, _>(name, move |_unused_rx| {
        loop {
            match rx.recv() {
                Ok(msg) => {
                    let converted: B = B::from(msg);
                    if tx.send(converted).is_err() {
                        tracing::warn!(event = "relay_downstream_gone", relay = %relay_name, "downstream disconnected; relay stopping");
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::run_bounded;
    use std::sync::mpsc;
    use std::time::Duration;

    #[derive(Debug, PartialEq, Eq)]
    struct A(u32);

    #[derive(Debug, PartialEq, Eq)]
    struct B(u32);

    impl From<A> for B {
        fn from(a: A) -> Self {
            B(a.0)
        }
    }

    // 送った順に変換済みの値が下流へ届く。
    #[test]
    fn forwards_with_type_conversion_in_order() {
        let (a_tx, a_rx) = mpsc::channel::<A>();
        let (b_tx, b_rx) = mpsc::channel::<B>();

        let handle = spawn_relay::<A, B>("test-relay-forward", a_rx, b_tx);

        a_tx.send(A(1)).expect("send A(1)");
        a_tx.send(A(2)).expect("send A(2)");
        a_tx.send(A(3)).expect("send A(3)");

        assert_eq!(
            b_rx.recv_timeout(Duration::from_secs(5))
                .expect("recv B(1)"),
            B(1)
        );
        assert_eq!(
            b_rx.recv_timeout(Duration::from_secs(5))
                .expect("recv B(2)"),
            B(2)
        );
        assert_eq!(
            b_rx.recv_timeout(Duration::from_secs(5))
                .expect("recv B(3)"),
            B(3)
        );

        drop(a_tx);
        run_bounded(
            "relay join after upstream drop",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("relay terminates normally after upstream drop");
            },
        );
    }

    // 上流全 Sender drop で自然終了し、join が有界時間内に Ok を返す。
    #[test]
    fn natural_termination_on_upstream_drop() {
        let (a_tx, a_rx) = mpsc::channel::<A>();
        let (b_tx, _b_rx) = mpsc::channel::<B>();

        let handle = spawn_relay::<A, B>("test-relay-upstream-drop", a_rx, b_tx);

        drop(a_tx);

        run_bounded(
            "relay join after upstream drop",
            Duration::from_secs(5),
            move || {
                handle.join().expect(
                    "relay must terminate normally (Ok) on upstream drop, not hang or panic",
                );
            },
        );
    }

    // 下流の受信端が drop されると、送出 Err で relay が終了する。
    #[test]
    fn downstream_disconnect_stops_relay_without_hanging() {
        let (a_tx, a_rx) = mpsc::channel::<A>();
        let (b_tx, b_rx) = mpsc::channel::<B>();

        let handle = spawn_relay::<A, B>("test-relay-downstream-disconnect", a_rx, b_tx);

        drop(b_rx);

        a_tx.send(A(42)).expect("send A(42)");

        run_bounded(
            "relay join after downstream disconnect",
            Duration::from_secs(5),
            move || {
                handle.join().expect(
                    "relay must terminate normally (Ok) on downstream disconnect, not hang or panic",
                );
            },
        );
    }
}
