use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

use super::{DWriteMetrics, ResolvedFont};
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

/// テスト用 BalloonModel 生成ヘルパ（font 以外は全成分未指定）。
pub(super) fn model_with_font(font: Font) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        font,
        None,
        None,
    )
}

/// 全成分未指定の Font（フォント定義欠落の balloon 定義）。
pub(super) fn empty_font() -> Font {
    Font::new(None, None, FontColor::new(None, None, None))
}

/// WARN/ERROR イベント数を数える最小 Subscriber（決定論的なログ檻・writing.rs の檻パターン踏襲）。
struct LevelCounter {
    warns: Arc<AtomicUsize>,
    errors: Arc<AtomicUsize>,
}

impl tracing::Subscriber for LevelCounter {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        match *event.metadata().level() {
            tracing::Level::WARN => {
                self.warns.fetch_add(1, Ordering::SeqCst);
            }
            tracing::Level::ERROR => {
                self.errors.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// クロージャをログ檻の中で実行し、（結果, WARN 件数, ERROR 件数）を返す。
pub(super) fn with_log_cage<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    let warns = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let subscriber = LevelCounter {
        warns: Arc::clone(&warns),
        errors: Arc::clone(&errors),
    };
    let out = tracing::subscriber::with_default(subscriber, f);
    (
        out,
        warns.load(Ordering::SeqCst),
        errors.load(Ordering::SeqCst),
    )
}

/// 既定フォント（ＭＳ ゴシック 12）の DWriteMetrics を組む。
pub(super) fn default_metrics(
    factory: &windows::Win32::Graphics::DirectWrite::IDWriteFactory2,
    mode: WritingMode,
) -> DWriteMetrics {
    let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
    DWriteMetrics::new(factory, &resolved, mode, &TextLayerConfig::default())
        .expect("既定フォントで DWriteMetrics 生成が成立する")
}
