//! 捕捉窓そのものの決定性を固定する自己テスト。
//!
//! 「他スレッドが先に同じ発行点を踏む」場面は待ち時間ではなく [`std::thread::JoinHandle::join`]
//! で順序を確定させる（時間に色が依存するテストを作らない、という本ワークスペースの規律）。
//! 各テストは**自分専用の発行点**（専用の宛先を持つ module 直下の関数）を使う。発行点の
//! interest はプロセス大域に焼き付くため、発行点を共有すると他テストの実行順で条件が変わる。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::event::{CapturedEvent, FieldCollector, FieldValue};

// ---- テスト専用の発行点（1 テスト 1 宛先） --------------------------------

const TARGET_BEFORE: &str = "log_capture_kit::selftest::before_window";
const TARGET_INSIDE: &str = "log_capture_kit::selftest::inside_window";
const TARGET_TRACE: &str = "log_capture_kit::selftest::trace_level";
const TARGET_OTHER_THREAD: &str = "log_capture_kit::selftest::other_thread";
const TARGET_RETAINED: &str = "log_capture_kit::selftest::retained_sink";

fn emit_before_window() {
    tracing::info!(target: TARGET_BEFORE, mark = "before", "窓の外で先に登録される発行点");
}

fn emit_inside_window() {
    tracing::info!(target: TARGET_INSIDE, mark = "inside", "窓の内側で先に登録される発行点");
}

fn emit_trace_level() {
    tracing::trace!(target: TARGET_TRACE, mark = "trace", "最下位レベルの発行点");
}

fn emit_other_thread() {
    tracing::warn!(target: TARGET_OTHER_THREAD, mark = "other", "他スレッド／窓外の発行点");
}

fn emit_retained() {
    tracing::info!(target: TARGET_RETAINED, mark = "retained", "共有参照を握られたままの窓");
}

fn count_target(events: &[CapturedEvent], target: &str) -> usize {
    events.iter().filter(|e| e.target == target).count()
}

// ---- (a) 他スレッドの先着で取りこぼさない --------------------------------

/// 要件 3.2／3.4-a: **窓の外で**別スレッドが先に同じ発行点を登録しても捕捉できる。
///
/// `join()` で「別スレッドの登録 → 窓を開く」の順序を確定させるので、色は時間に依存しない。
#[test]
fn captures_event_whose_callsite_another_thread_registered_before_the_window() {
    std::thread::spawn(emit_before_window)
        .join()
        .expect("先着スレッドは panic しない");

    let ((), events) = capture(emit_before_window);

    assert_eq!(
        count_target(&events, TARGET_BEFORE),
        1,
        "先着スレッドが登録した発行点を窓内で取りこぼした: {events:?}"
    );
}

/// 要件 3.1／3.2／3.4-a: **窓の内側で**別スレッドが先に同じ発行点を登録しても捕捉できる。
///
/// これが Flow 1 の図がそのまま描いている場面（`O->>C: 同じ発行点を初回登録`）。
/// 別スレッドの発火自体は窓へ混入しない（スレッド局所＝要件 3.6）ので、期待件数は 1。
#[test]
fn captures_event_whose_callsite_another_thread_registers_inside_the_window() {
    let ((), events) = capture(|| {
        std::thread::spawn(emit_inside_window)
            .join()
            .expect("先着スレッドは panic しない");
        emit_inside_window();
    });

    assert_eq!(
        count_target(&events, TARGET_INSIDE),
        1,
        "窓内で先着登録された発行点の捕捉件数が 1 ではない: {events:?}"
    );
}

// ---- (b) 対照イベントを捕まえない捕捉先は失敗を宣告される ----------------

/// 対照イベント（番兵）だけを落とす捕捉先。register_callsite を `sometimes` に固定するので、
/// この捕捉先が大域の interest キャッシュへ `never` を焼くことはない（他テストへ無害）。
#[derive(Clone)]
struct SentinelDroppingSubscriber(Arc<Mutex<Vec<CapturedEvent>>>);

impl tracing::Subscriber for SentinelDroppingSubscriber {
    fn register_callsite(
        &self,
        _meta: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        tracing::subscriber::Interest::sometimes()
    }
    fn enabled(&self, meta: &tracing::Metadata<'_>) -> bool {
        meta.target() != SENTINEL_TARGET
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields: Vec<(String, FieldValue)> = Vec::new();
        event.record(&mut FieldCollector(&mut fields));
        self.0.lock().expect("捕捉バッファは毒化していない").push(CapturedEvent {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            fields,
        });
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// 要件 3.3／6.3 の較正: 対照イベントが捕まらない捕捉先を差すと**失敗が宣告**される
/// （空の結果を静かに返して縮退しない）。この検査が赤にできることが、番兵検査が
/// 何かを証明していることの根拠になる。
#[test]
#[should_panic(expected = "対照イベント")]
fn declares_failure_when_the_sentinel_is_not_captured() {
    let sink: Arc<Mutex<Vec<CapturedEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let subscriber = SentinelDroppingSubscriber(Arc::clone(&sink));
    let _ = run_with_subscriber(subscriber, Arc::clone(&sink), || {
        tracing::info!(target: "log_capture_kit::selftest::dropped", "番兵は落ちる");
    });
}

// ---- (c) 最下位レベル --------------------------------------------------

/// 要件 3.5: TRACE を含む全レベルが捕捉対象。
#[test]
fn captures_trace_level_events() {
    let ((), events) = capture(emit_trace_level);

    let hit = events
        .iter()
        .find(|e| e.target == TARGET_TRACE)
        .unwrap_or_else(|| panic!("TRACE イベントを捕捉できていない: {events:?}"));
    assert_eq!(hit.level, tracing::Level::TRACE);
}

// ---- (d) 窓の外・他スレッドの混入なし ------------------------------------

/// 要件 3.6: 窓の外で発火したイベントも、窓の内側で**他スレッド**が発火したイベントも
/// 混入しない（既定 API はスレッド局所の捕捉意味論）。
#[test]
fn does_not_capture_events_from_outside_the_window_or_other_threads() {
    emit_other_thread(); // 窓の外

    let ((), events) = capture(|| {
        std::thread::spawn(emit_other_thread)
            .join()
            .expect("他スレッドは panic しない");
    });

    assert_eq!(
        count_target(&events, TARGET_OTHER_THREAD),
        0,
        "窓外／他スレッドのイベントが混入した: {events:?}"
    );
}

// ---- 対照イベントは返却前に取り除かれる ----------------------------------

/// 要件 3.3／6.3: 戻り値と捕捉結果は対照イベントの分だけ増えない。
#[test]
fn sentinel_is_removed_before_returning() {
    let (value, events) = capture(|| 42u32);

    assert_eq!(value, 42);
    assert!(
        events.iter().all(|e| e.target != SENTINEL_TARGET),
        "対照イベントが呼出側へ漏れている: {events:?}"
    );
    assert!(events.is_empty(), "何も発火していない窓が空でない: {events:?}");
}

// ---- 捕捉結果の取り出しは共有参照の解放に依存しない ----------------------

/// 設計の Invariants: 捕捉結果は `with_default` を抜けた**後**に取り出し、共有参照が
/// 1 本だけになること（`Arc::try_unwrap` の成否）に依存しない。呼出側が共有参照を
/// 握ったままでもイベントが返ることで示す。
#[test]
fn extracts_events_even_while_the_shared_sink_is_still_held() {
    let sink: Arc<Mutex<Vec<CapturedEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let subscriber = CaptureSubscriber(Arc::clone(&sink));

    // `sink`（テストが握る）＋ subscriber の中の 1 本＝強参照は 2 本以上ある。
    let ((), events) = run_with_subscriber(subscriber, Arc::clone(&sink), emit_retained);

    assert_eq!(
        count_target(&events, TARGET_RETAINED),
        1,
        "共有参照が残っている状態で捕捉結果を取り出せていない: {events:?}"
    );
}

// ---- 常駐の仕掛けは冪等 --------------------------------------------------

/// 設計の Invariants: 常駐 probe はプロセス寿命で 1 度だけ確立され、多スレッドから
/// 同時に呼んでも安全（冪等）。
#[test]
fn ensure_interest_probes_is_idempotent_across_threads() {
    static DONE: AtomicUsize = AtomicUsize::new(0);

    ensure_interest_probes();
    ensure_interest_probes();

    let hands: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                ensure_interest_probes();
                DONE.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();
    for h in hands {
        h.join().expect("probe 確立スレッドは panic しない");
    }

    assert_eq!(DONE.load(Ordering::SeqCst), 8);

    // 冪等呼出のあとでも窓は生きている（番兵検査が通る）。
    let ((), _events) = capture(|| {});
}
