//! 捕捉窓。呼出スレッドで同期的に発火したイベントを集め、窓が実際に生きていたことを
//! 番兵イベントで自己検査する。
//!
//! 窓の決定性は 3 つの仕掛けの合成で成り立つ（機序の詳細は [`crate::probe`] の doc）。
//!
//! 1. **常駐 probe**（[`ensure_interest_probes`]）— 他スレッドの先着で発行点の interest に
//!    `never` が焼き付く経路を、プロセス寿命で閉じる。
//! 2. **窓内での interest 再計算**（[`tracing::callsite::rebuild_interest_cache`]）— probe 常駐
//!    より前に焼かれてしまった `never` を、窓が開いた**後**の時点で確定的に解消する。
//! 3. **番兵イベント**— 窓の内側で対照イベントを 1 件発火して捕捉できることを確かめる。
//!    捕捉できなければ panic する（空の結果を静かに返して縮退しない）。番兵は返却前に
//!    取り除くので、呼出側の戻り値と主張は番兵の分だけ変わることが無い。
//!
//! [`capture`] が差す受け手（`WindowSubscriber`）は、窓が開いているあいだだけ目印
//! （`CaptureWindowMarker`）の問い合わせに答える。窓の中で起こしたアクター
//! （`areka-actor` の `spawn_actor`）はこの答えを見て窓の受け手を子のスレッドへ引き継ぐ。
//! 窓を閉じた後は答えず、届いた記録は捨てる（後から開いたほかの窓へは行かない）。

use std::any::TypeId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::event::CapturedEvent;
use crate::probe::ensure_interest_probes;

/// 番兵イベント専用の宛先。実コードがこの宛先へ発火することは無い。
pub(crate) const SENTINEL_TARGET: &str = "log_capture_kit::sentinel";

/// イベントを溜めるだけの最小 subscriber。
///
/// `enabled` は常に真＝**TRACE を含む全レベル**が捕捉対象（要件 3.5）。span は使わないので
/// `new_span` は固定 id を返す。
#[derive(Clone, Default)]
pub(crate) struct CaptureSubscriber(pub(crate) Arc<Mutex<Vec<CapturedEvent>>>);

impl tracing::Subscriber for CaptureSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        self.0
            .lock()
            .expect("捕捉バッファは毒化していない")
            .push(CapturedEvent::from_event(event));
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// 捕捉の窓の受け手だけが答える目印（`areka-actor` の `spawn.rs` に同じ形がある）。
pub(crate) type CaptureWindowMarker = [(); 0x6172_656B];

/// `capture` が差す窓専用の受け手。中の `CaptureSubscriber` へそのまま渡し、
/// 目印に答えるのと記録を渡すのは窓が開いているあいだだけ。
///
/// `enabled` は閉じた後も真のまま（偽にすると probe を切った実行で発行点に `never` が焼け、
/// 後の窓が記録を取りこぼすため）。
struct WindowSubscriber {
    inner: CaptureSubscriber,
    /// 窓が開いているか。`capture` を抜けるときに `CloseOnDrop` が偽にする。
    open: Arc<AtomicBool>,
}

impl WindowSubscriber {
    fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
}

impl tracing::Subscriber for WindowSubscriber {
    fn enabled(&self, meta: &tracing::Metadata<'_>) -> bool {
        self.inner.enabled(meta)
    }
    fn new_span(&self, attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        self.inner.new_span(attrs)
    }
    fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        self.inner.record(span, values)
    }
    fn record_follows_from(&self, span: &tracing::span::Id, follows: &tracing::span::Id) {
        self.inner.record_follows_from(span, follows)
    }
    fn event(&self, event: &tracing::Event<'_>) {
        // 閉じた後に届いた記録は捨てる（誰も読まない・ほかの窓へも行かない）。
        if self.is_open() {
            self.inner.event(event);
        }
    }
    fn enter(&self, span: &tracing::span::Id) {
        self.inner.enter(span)
    }
    fn exit(&self, span: &tracing::span::Id) {
        self.inner.exit(span)
    }
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        if id == TypeId::of::<Self>()
            || (id == TypeId::of::<CaptureWindowMarker>() && self.is_open())
        {
            // 目印は大きさ 0・整列 1 の型なので、null でない `self` の番地はその正しい値を指す。
            Some(self as *const Self as *const ())
        } else {
            None
        }
    }
}

/// `capture` を抜けるとき（panic で抜けるときも）窓を閉じた印を立てる番。
struct CloseOnDrop(Arc<AtomicBool>);

impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// 捕捉窓の本体。`subscriber` を現在のスレッドの既定へ差し、`sink` から結果を取り出す。
///
/// `subscriber` と `sink` を別引数に分けてあるのは、**番兵を落とす捕捉先**を差して
/// 「番兵検査が本当に赤にできる」ことを自己テストで較正するためである
/// （検査が赤にできないなら、その検査は何も証明していない）。
///
/// 手順は Flow 1 の図のとおり: probe 常駐（冪等）→ 窓を開く → 窓内で interest 再計算 →
/// 番兵発火 → `f` 実行 → **窓を抜けてから**結果を取り出す → 番兵の存在を検査して除去。
pub(crate) fn run_with_subscriber<S, R>(
    subscriber: S,
    sink: Arc<Mutex<Vec<CapturedEvent>>>,
    f: impl FnOnce() -> R,
) -> (R, Vec<CapturedEvent>)
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    ensure_interest_probes();

    let out = tracing::subscriber::with_default(subscriber, || {
        // probe 常駐前（プロセス起動〜初回の窓）に焼かれた `never` の掃き残しを、
        // 窓が開いた**後**の時点で確定的に潰す。
        tracing::callsite::rebuild_interest_cache();
        // 対照イベント（番兵）。窓が本当に生きているかを、同じ窓の内側で示す。
        tracing::trace!(target: SENTINEL_TARGET, "capture window is live");
        f()
    });

    // 取り出しは窓を抜けた**後**に行う。`Arc` の強参照が 1 本になること
    // （`Arc::try_unwrap` の成否）に依存しない形にしてある——依存させると、捕捉先の
    // 複製を持つ呼出側で結果が黙って空になる（`areka-kanade` で実際に間欠失敗した形）。
    let mut events = std::mem::take(&mut *sink.lock().expect("捕捉バッファは毒化していない"));

    let before = events.len();
    events.retain(|e| e.target != SENTINEL_TARGET);
    assert!(
        events.len() < before,
        "捕捉窓の対照イベント（{SENTINEL_TARGET}）を捕捉できなかった。\
         差し込んだ捕捉先がイベントを受け取っていないため、この窓の捕捉結果は \
         「出なかった」ことの証拠にならない"
    );

    (out, events)
}

/// 既定の捕捉 API。`f` の実行中に**現在のスレッド**で同期的に発火したイベントを、
/// `f` の戻り値と共に返す。
///
/// - TRACE を含む全レベルが対象（要件 3.5）。
/// - 窓の外・他スレッドで発火したイベントは混入しない（要件 3.6）。例外は窓の中で起こした
///   アクター（`areka-actor` の `spawn_actor`）のスレッドとその子孫で、窓の受け手を引き継ぐので
///   その記録も入る。入れたいなら、アクターの記録が済んだことを確かめてから窓を閉じる。
///   窓を閉じた後にそれらが出した記録は捨てられ、後から開いたほかの窓には入らない。
///   それ以外の別スレッドで発火するログを捕える必要がある場合は、別名の全スレッド捕捉 API を使う。
/// - 戻り値に番兵イベントは含まれない（要件 3.3・6.3）。捕捉が働いていなければ panic する。
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<CapturedEvent>) {
    let open = Arc::new(AtomicBool::new(true));
    // 最初の束縛＝関数の終わり（結果を取り出した後）に最後に落ちて窓を閉じる。
    let _close = CloseOnDrop(Arc::clone(&open));
    let subscriber = WindowSubscriber {
        inner: CaptureSubscriber::default(),
        open,
    };
    let sink = Arc::clone(&subscriber.inner.0);
    run_with_subscriber(subscriber, sink, f)
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
