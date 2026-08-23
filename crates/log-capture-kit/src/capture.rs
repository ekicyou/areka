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
/// - 窓の外・他スレッドで発火したイベントは混入しない（要件 3.6）。別スレッドで発火する
///   ログを捕える必要がある場合は、別名の全スレッド捕捉 API を使う。
/// - 戻り値に番兵イベントは含まれない（要件 3.3・6.3）。捕捉が働いていなければ panic する。
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<CapturedEvent>) {
    let subscriber = CaptureSubscriber::default();
    let sink = Arc::clone(&subscriber.0);
    run_with_subscriber(subscriber, sink, f)
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
