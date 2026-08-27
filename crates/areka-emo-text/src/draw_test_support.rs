use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

use log_capture_kit::count_levels;

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

/// クロージャを共有のログ捕捉窓の中で実行し、（結果, WARN 件数, ERROR 件数）を返す。
///
/// 件数の集計は硬化機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ねる。
/// 戻り値の組は移行前と同一で、呼出側の判定内容は変わらない。
pub(super) fn with_log_cage<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    let (out, counts) = count_levels(f);
    (out, counts.warn, counts.error)
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
