//! `spawn_relay` — 相互依存の循環を解く汎用転送コンポーネント。
//!
//! 上流からのメッセージを型変換しつつ下流へ中継する最小機構。上流が全て
//! 手放されたら明示停止を持たず自然に終了する（start-relay／down-relay の
//! 2 箇所で使用・design.md「ghost::relay」）。task 2.1 で実装する。

use std::sync::mpsc::{Receiver, Sender};

use areka_actor::ActorHandle;

/// `rx` から受けたメッセージを `From` 変換して `tx` へ流す。上流全 `Sender` drop で自然終了。
/// 下流切断（送出 `Err`）は `warn!` の上で終了する（宙吊りなし）。
///
/// `rx` は呼び出し側が所有する外部チャンネルの受信端であり、`spawn_relay` 自身は
/// 送信端を返さない（「inbox」を内部生成する `areka_actor::spawn_actor` とは異なる）。
/// 命名付きスレッド・tracing span・panic 安全性は `spawn_actor` の機構をそのまま流用する
/// （`spawn_actor` 自身が内部生成する channel は未使用のまま捨て、body は closure が
/// キャプチャした外部 `rx` を駆動する）。
pub fn spawn_relay<A, B>(name: &str, rx: Receiver<A>, tx: Sender<B>) -> ActorHandle
where
    A: Send + 'static,
    B: From<A> + Send + 'static,
{
    let relay_name: String = name.to_string();
    let (_unused_tx, handle) = areka_actor::spawn_actor::<A, _>(name, move |_unused_rx| loop {
        match rx.recv() {
            Ok(msg) => {
                let converted: B = B::from(msg);
                if tx.send(converted).is_err() {
                    tracing::warn!(relay = %relay_name, "downstream disconnected; relay stopping");
                    return;
                }
            }
            Err(_) => return, // 上流全 Sender drop: 自然終了（ログ不要）。
        }
    });
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{self, sync_channel};
    use std::thread;
    use std::time::Duration;

    /// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
    /// テストを失敗させる（どのテストもハングしないことを保証する・areka-actor 流儀）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
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

    #[derive(Debug, PartialEq, Eq)]
    struct A(u32);

    #[derive(Debug, PartialEq, Eq)]
    struct B(u32);

    impl From<A> for B {
        fn from(a: A) -> Self {
            B(a.0)
        }
    }

    // ケース1: 転送＋型変換。送った順に変換済みの値が下流へ届く。
    #[test]
    fn forwards_with_type_conversion_in_order() {
        let (a_tx, a_rx) = mpsc::channel::<A>();
        let (b_tx, b_rx) = mpsc::channel::<B>();

        let handle = spawn_relay::<A, B>("test-relay-forward", a_rx, b_tx);

        a_tx.send(A(1)).expect("send A(1)");
        a_tx.send(A(2)).expect("send A(2)");
        a_tx.send(A(3)).expect("send A(3)");

        assert_eq!(
            b_rx.recv_timeout(Duration::from_secs(5)).expect("recv B(1)"),
            B(1)
        );
        assert_eq!(
            b_rx.recv_timeout(Duration::from_secs(5)).expect("recv B(2)"),
            B(2)
        );
        assert_eq!(
            b_rx.recv_timeout(Duration::from_secs(5)).expect("recv B(3)"),
            B(3)
        );

        // 後始末: 上流を手放して自然終了させる。
        drop(a_tx);
        handle.join().expect("relay terminates normally after upstream drop");
    }

    // ケース2: 上流全 Sender drop → 自然終了（join が Ok(()) を有界時間内に返す）。
    #[test]
    fn natural_termination_on_upstream_drop() {
        let (a_tx, a_rx) = mpsc::channel::<A>();
        let (b_tx, _b_rx) = mpsc::channel::<B>();

        let handle = spawn_relay::<A, B>("test-relay-upstream-drop", a_rx, b_tx);

        // 上流を手放す＝自然終了経路。
        drop(a_tx);

        run_bounded(
            "relay join after upstream drop",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("relay must terminate normally (Ok) on upstream drop, not hang or panic");
            },
        );
    }

    // ケース3: 下流切断（受信端 drop）→ 送出 Err で relay が宙吊りなく終了する。
    #[test]
    fn downstream_disconnect_stops_relay_without_hanging() {
        let (a_tx, a_rx) = mpsc::channel::<A>();
        let (b_tx, b_rx) = mpsc::channel::<B>();

        let handle = spawn_relay::<A, B>("test-relay-downstream-disconnect", a_rx, b_tx);

        // 下流受信端を先に手放す。
        drop(b_rx);

        // 上流へ1件送る: relay は tx.send が Err になり終了するはず。
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
