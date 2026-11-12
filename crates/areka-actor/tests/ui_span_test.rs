//! UI アクターの記録の区間が poll の間だけ入ることを縛る（audit-sylphya-actor #2）。
//!
//! 区間の出入りを数える自前の受け手 `EnterDepth` をポンプのスレッドの既定に据えて UI アクターを
//! 起こし、handler の中の深さ（1＝区間が付いている較正）と、ポンプが戻った直後＝タスクの poll の
//! 外の深さ（0）を見る。区間をタスクの頭で `enter` したままだと、待ちの間も入ったままで 1 になる。
//! 共有の捕捉の受け手は区間を数えないので素の `set_default` を使う（テストはこの 1 本だけ）。

mod ui_pump_support;

use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::time::Duration;

use areka_actor::{ReplySender, reply_channel, spawn_ui};
use ui_pump_support::run_pump_until;

const TEST_DEADLINE: Duration = Duration::from_secs(5);

/// 区間に入るたび +1・出るたび −1 を数えるだけの受け手（依存 0）。
struct EnterDepth(Arc<AtomicIsize>);

impl tracing::Subscriber for EnterDepth {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, _: &tracing::Event<'_>) {}
    fn enter(&self, _: &tracing::span::Id) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn exit(&self, _: &tracing::span::Id) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[test]
fn the_ui_actor_span_is_entered_only_while_polled() {
    let depth = Arc::new(AtomicIsize::new(0));
    let _default = tracing::subscriber::set_default(EnterDepth(Arc::clone(&depth)));

    let in_handler = Arc::new(AtomicIsize::new(-1));
    let handler_depth = Arc::clone(&depth);
    let handler_seen = Arc::clone(&in_handler);
    let (ui_tx, _drain) = spawn_ui("ui-span", move |reply: ReplySender<()>| {
        handler_seen.store(handler_depth.load(Ordering::SeqCst), Ordering::SeqCst);
        let _ = reply.send(());
        Ok::<_, &'static str>(ControlFlow::Continue(()))
    })
    .expect("spawn_ui on the pump (test) thread must succeed");
    // 全送り手が落ちるとタスクが終わって区間を出てしまうので、測るまで 1 つ握っておく。
    let keep_alive = ui_tx.clone();

    let done = Arc::new(AtomicBool::new(false));
    let worker_done = Arc::clone(&done);
    let worker = std::thread::spawn(move || {
        let (reply_tx, reply_rx) = reply_channel::<()>();
        if ui_tx.send(reply_tx).is_err() {
            panic!("UiSender::send failed (inbox closed)");
        }
        reply_rx
            .recv_timeout(TEST_DEADLINE)
            .expect("reply within the deadline");
        worker_done.store(true, Ordering::Release);
    });

    run_pump_until(&done, TEST_DEADLINE);
    // ポンプが戻った直後＝タスクの poll の外・ポンプのスレッドの上。
    let after_pump = depth.load(Ordering::SeqCst);

    worker.join().expect("worker thread must not panic");
    drop(keep_alive);
    assert_eq!(
        in_handler.load(Ordering::SeqCst),
        1,
        "handler の中は UI アクターの区間に入っているはず"
    );
    assert_eq!(
        after_pump, 0,
        "待ちの間は UI アクターの区間に入っていないはず"
    );
}
