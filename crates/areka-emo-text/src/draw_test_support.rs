use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

use log_capture_kit::count_levels;

use super::{DWriteMetrics, FontCatalog, ResolvedFont};
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;
use windows::Win32::Graphics::DirectWrite::{DWRITE_FONT_STYLE, IDWriteTextFormat};

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

/// ぱすたのバルーンの `font.name` の候補列（本番の解決で先頭の「游ゴシック Medium」が GDI の名前として引ける）。
pub(crate) const PASTA_FONT_NAMES: &str =
    "游ゴシック Medium,Yu Gothic Medium,游ゴシック,Yu Gothic,BIZ UDPゴシック,メイリオ";

/// 游ゴシックがこの機械に無ければ赤で止める（中太の期待値が代替フォントで緑にならないように）。
pub(crate) fn require_yu_gothic(fonts: &FontCatalog) {
    assert!(
        fonts.family_for(&["Yu Gothic".to_owned()]).is_some(),
        "実フォント 游ゴシック（Yu Gothic）がこの機械に無い。本試験の期待値は当該フォントの\
         中太を前提にしているので、代替フォントへ縮退したまま緑にしない"
    );
}

/// TextFormat の家族名を読み戻す。
pub(crate) fn family_name(format: &IDWriteTextFormat) -> String {
    unsafe {
        let len = format.GetFontFamilyNameLength() as usize;
        let mut buf = vec![0u16; len + 1];
        format
            .GetFontFamilyName(&mut buf)
            .expect("GetFontFamilyName");
        String::from_utf16_lossy(&buf[..len])
    }
}

/// 書式の家族名・太さ・斜体。
pub(crate) fn face_of(format: &IDWriteTextFormat) -> (String, u32, DWRITE_FONT_STYLE) {
    unsafe {
        (
            family_name(format),
            format.GetFontWeight().0 as u32,
            format.GetFontStyle(),
        )
    }
}
