//! # draw — DirectWrite/D2D 描画実行（COM 層）
//!
//! `DrawExecutor`（可視窓の全域再描画・フォント解決・縦書きレシピの lift）と
//! `DWriteMetrics`（測定専用 probe TextLayout 由来の `GlyphMetrics` 実装）を担う。
//!
//! **層規律**: COM 層——UI スレッド専有。`windows`（DirectWrite/D2D）を触るのは
//! 本モジュールと surface のみ。失敗は log-first（`tracing::error!`＋`Err`）で扱い panic しない。
//!
//! ## フォント解決＋方向レシピ（task 6.1・R4.1/R4.2/R10.3）
//!
//! - [`ResolvedFont`]: balloon 定義（[`BalloonModel`] の `Font`）から描画・レイアウト生成に
//!   必要な設定一式を解決する。欠落は ukadoc 既定へフォールバック
//!   （[`DEFAULT_FONT_NAME`]＝ＭＳ ゴシック／[`DEFAULT_FONT_HEIGHT`]＝12／色＝黒）。
//! - [`DirectionRecipe`]: `writing_mode` の解釈結果 [`WritingMode`] から DirectWrite の
//!   方向設定 4 点（Reading/Flow/Text/Paragraph）を一意に導出する。レシピは
//!   wintf `typewriter_layout.rs` 実証済みレシピの **lift（複製）**——wintf の
//!   テキスト widget system へは依存しない（steering 記憶 areka-emo-owns-drawing-wintf-lift）。
//! - [`create_text_format`]: 上記 2 つを実 `IDWriteTextFormat` へ焼き込む
//!   （生成失敗は warn→既定フォント再試行→なお失敗は `Device` エラー・R4.2）。
//! - 文字装飾（[`TextEffects`]）と `disable.font.*`（[`FontDisableSeam`]）は
//!   **型シームのみ・実挙動なし**（R10.3・M2 予約）。
//!
//! ## 計測専用 probe layout（task 6.2・R4.5・probe 規約）
//!
//! [`DWriteMetrics`]: **未折返しの測定専用 probe TextLayout** の cluster metrics から
//! [`GlyphMetrics`] を実装し、純粋層 `LayoutEngine` へ実測送り幅を注入する
//! （design discussion #1 裁定の probe 規約）:
//!
//! - probe は**折返し決定より前**に生成する（計測→折返しの一方向＝鶏卵の構造的切断）。
//! - probe の format は描画と**同一の [`create_text_format`] 経路**
//!   （フォント・サイズ・writing_mode 写像設定込み）で生成する。
//! - probe layout は折返し無効寸（[`PROBE_MAX_EXTENT`]）で組む＝未折返し。
//! - 計測結果はキャッシュ可（追記単調ゆえ確定内容の metrics は不変）——本実装は
//!   文字単位キャッシュ（format 固定につき 同一文字→同一送り幅の決定論）。
//!
//! probe と描画行 TextLayout の advance 一致 invariant の檻化は task 6.4、
//! 全域再描画（描画実行）は task 6.3 の領分。

use std::cell::RefCell;
use std::collections::HashMap;

use areka_parsers::balloon::BalloonModel;
use tracing::warn;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FLOW_DIRECTION, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
    DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_PARAGRAPH_ALIGNMENT, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_READING_DIRECTION,
    DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteFactory2, IDWriteFontCollection,
    IDWriteTextFormat,
};
use windows::core::HSTRING;
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};

use crate::TextLayerError;
use crate::canvas::TextEffects;
use crate::layout::GlyphMetrics;
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

/// SSP 既定フォント名（**全角表記** ＭＳ ゴシック・ukadoc 既定・R4.2）。
pub const DEFAULT_FONT_NAME: &str = "ＭＳ ゴシック";

/// 既定フォント高さ 12（**image px**・「単位はピクセル：ポイントではない」・ukadoc 既定・R4.1/R4.2）。
pub const DEFAULT_FONT_HEIGHT: f32 = 12.0;

/// TextFormat のロケール（wintf typewriter レシピからの lift・日本語正準）。
const LOCALE_JA_JP: &str = "ja-JP";

/// probe layout の折返し無効寸（image px）。行内軸・行送り軸の双方へ与え、
/// probe を**未折返し**にする（probe 規約——バルーン寸は image px で高々数百・
/// font.height も高々数十のため 1e6 は実用上無限）。
pub const PROBE_MAX_EXTENT: f32 = 1.0e6;

/// M2 予約キー接頭辞: `disable.font.*`（`\f[disable]` 用・SSP 2.5.51+）——
/// 予約名の記録のみ・実挙動なし（R10.3・fixture 未使用）。
pub const RESERVED_KEY_DISABLE_FONT_PREFIX: &str = "disable.font.";

/// `disable.font.*` 拡張の型シーム（実挙動なし・R10.3）。
///
/// `#[non_exhaustive]`＋フィールドなし＝crate 外から意味を持たせられない構造保証。
/// 実装（`\f[disable]` によるフォント変更禁止）は M2/後続ユニットの領分。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontDisableSeam {}

/// 解決済みフォント一式（`DrawExecutor::render` の `font` 引数・design.md「DrawExecutor（draw.rs）」）。
///
/// balloon 定義の欠落成分を ukadoc 既定で埋めた「レイアウト生成に必要な設定一式」。
/// [`resolve`](Self::resolve) だけが生成口（テストを除く）で、`height` は常に正値。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFont {
    /// 採用フォント名（`font.name` 先頭名 or [`DEFAULT_FONT_NAME`]・M1 は先頭のみ消費）。
    pub name: String,
    /// カンマ区切り複数指定（SSP 拡張）の残余名——フォールバック連鎖の**型シーム**
    /// （M1 未消費・design.md「フォールバック連鎖は型シーム」）。
    pub fallback_chain: Vec<String>,
    /// フォント高さ（image px・DirectWrite fontsize へ素通し・欠落/0 は [`DEFAULT_FONT_HEIGHT`]）。
    pub height: f32,
    /// フォント色 r/g/b（成分独立既定 0＝欠落は黒・ukadoc 既定）。
    pub color: (u8, u8, u8),
    /// 文字装飾の M2 予約シーム（実挙動なし・R10.3）。
    pub effects: TextEffects,
    /// `disable.font.*` の型シーム（実挙動なし・R10.3）。
    pub disable: FontDisableSeam,
}

impl ResolvedFont {
    /// balloon 定義からフォント設定一式を解決する（R4.1/R4.2）。
    ///
    /// - `font.name` 欠落 → [`DEFAULT_FONT_NAME`]（ukadoc 既定＝正常系・ログなし）。
    ///   カンマ区切り複数指定は先頭名を採用し残余を [`fallback_chain`](Self::fallback_chain)
    ///   に保持（M1 未消費）。宣言はあるが実質空（空文字/空白/カンマのみ）は縮退
    ///   （`warn!`＋既定フォント）。
    /// - `font.height` 欠落 → [`DEFAULT_FONT_HEIGHT`]（正常系・ログなし）。
    ///   `0` は DirectWrite fontsize の正値制約を満たせない縮退値（`warn!`＋既定値）。
    /// - `font.color.r/g/b` は成分独立既定 0（欠落＝黒・正常系・ログなし）。
    pub fn resolve(model: &BalloonModel) -> ResolvedFont {
        let font = model.font();

        let (name, fallback_chain) = match font.name() {
            None => (DEFAULT_FONT_NAME.to_owned(), Vec::new()),
            Some(raw) => {
                let mut names = raw.split(',').map(str::trim).filter(|s| !s.is_empty());
                match names.next() {
                    Some(first) => (first.to_owned(), names.map(str::to_owned).collect()),
                    None => {
                        warn!(
                            value = raw,
                            "font.name が実質空のため既定フォント ＭＳ ゴシック へフォールバックする"
                        );
                        (DEFAULT_FONT_NAME.to_owned(), Vec::new())
                    }
                }
            }
        };

        let height = match font.height() {
            None => DEFAULT_FONT_HEIGHT,
            Some(0) => {
                warn!(
                    "font.height が 0（DirectWrite fontsize は正値必須）のため既定値 12 へフォールバックする"
                );
                DEFAULT_FONT_HEIGHT
            }
            Some(h) => h as f32,
        };

        let color = font.color();
        let color = (
            color.r().unwrap_or(0),
            color.g().unwrap_or(0),
            color.b().unwrap_or(0),
        );

        ResolvedFont {
            name,
            fallback_chain,
            height,
            color,
            effects: TextEffects::default(),
            disable: FontDisableSeam::default(),
        }
    }
}

/// DirectWrite 方向設定レシピ——`writing_mode` の解釈結果から**一意に導出**される 4 点セット。
///
/// wintf `typewriter_layout.rs` 実証済みレシピの lift（複製・wintf 非依存）。
/// design.md 写像表:
///
/// | [`WritingMode`] | Reading | Flow |
/// |---|---|---|
/// | `HorizontalTb` | LEFT_TO_RIGHT | TOP_TO_BOTTOM |
/// | `VerticalRl` | TOP_TO_BOTTOM | RIGHT_TO_LEFT |
/// | `VerticalLr` | TOP_TO_BOTTOM | LEFT_TO_RIGHT |
///
/// いずれも Alignment LEADING＋Paragraph NEAR（書字開始角へ寄せる——軸読み替え正準表と整合）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionRecipe {
    /// `SetReadingDirection` へ渡す行内方向。
    pub reading: DWRITE_READING_DIRECTION,
    /// `SetFlowDirection` へ渡す行送り方向。
    pub flow: DWRITE_FLOW_DIRECTION,
    /// `SetTextAlignment` へ渡す行内寄せ（全方向 LEADING）。
    pub text_alignment: DWRITE_TEXT_ALIGNMENT,
    /// `SetParagraphAlignment` へ渡す行送り寄せ（全方向 NEAR）。
    pub paragraph_alignment: DWRITE_PARAGRAPH_ALIGNMENT,
}

impl DirectionRecipe {
    /// [`WritingMode`] から方向レシピを一意に導出する（3 方向→3 レシピの単射・R5.5 消費側）。
    pub fn for_mode(mode: WritingMode) -> DirectionRecipe {
        let (reading, flow) = match mode {
            WritingMode::HorizontalTb => (
                DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
                DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
            ),
            WritingMode::VerticalRl => (
                DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
                DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT,
            ),
            WritingMode::VerticalLr => (
                DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
                DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
            ),
        };
        DirectionRecipe {
            reading,
            flow,
            text_alignment: DWRITE_TEXT_ALIGNMENT_LEADING,
            paragraph_alignment: DWRITE_PARAGRAPH_ALIGNMENT_NEAR,
        }
    }

    /// レシピを `IDWriteTextFormat` へ焼き込む（失敗は log-first: `error!`＋`Device` Err）。
    pub fn apply(&self, format: &IDWriteTextFormat) -> Result<(), TextLayerError> {
        unsafe {
            format
                .SetReadingDirection(self.reading)
                .map_err(device_err("SetReadingDirection"))?;
            format
                .SetFlowDirection(self.flow)
                .map_err(device_err("SetFlowDirection"))?;
            format
                .SetTextAlignment(self.text_alignment)
                .map_err(device_err("SetTextAlignment"))?;
            format
                .SetParagraphAlignment(self.paragraph_alignment)
                .map_err(device_err("SetParagraphAlignment"))?;
        }
        Ok(())
    }
}

/// 解決済みフォント＋方向レシピから描画/計測共用の `IDWriteTextFormat` を生成する。
///
/// フォント生成失敗（Error Categories・R4.2）: `warn!`→[`DEFAULT_FONT_NAME`] で再試行→
/// なお失敗は `error!`＋[`TextLayerError::Device`]（panic しない）。
/// task 6.2 の計測専用 probe layout も**同一のこの format 生成経路**を用いる前提
/// （probe 規約: 描画と同一 TextFormat）。
pub fn create_text_format(
    factory: &IDWriteFactory2,
    font: &ResolvedFont,
    mode: WritingMode,
) -> Result<IDWriteTextFormat, TextLayerError> {
    let format = match try_create_format(factory, &font.name, font.height) {
        Ok(format) => format,
        Err(e) => {
            warn!(
                font_name = font.name.as_str(),
                hresult = e.code().0,
                "TextFormat 生成に失敗——既定フォント ＭＳ ゴシック で再試行する"
            );
            try_create_format(factory, DEFAULT_FONT_NAME, font.height).map_err(|e| {
                let hresult = e.code().0;
                tracing::error!(
                    hresult,
                    font_height = font.height,
                    "既定フォントでも TextFormat 生成に失敗"
                );
                TextLayerError::Device {
                    hresult,
                    context: "CreateTextFormat",
                }
            })?
        }
    };
    DirectionRecipe::for_mode(mode).apply(&format)?;
    Ok(format)
}

/// `CreateTextFormat` の 1 回試行（weight/style/stretch は NORMAL・locale ja-JP＝wintf lift）。
fn try_create_format(
    factory: &IDWriteFactory2,
    family_name: &str,
    font_size: f32,
) -> windows::core::Result<IDWriteTextFormat> {
    factory.create_text_format(
        &HSTRING::from(family_name),
        None::<&IDWriteFontCollection>,
        DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL,
        font_size,
        &HSTRING::from(LOCALE_JA_JP),
    )
}

/// 計測専用 probe TextLayout 由来の実測 [`GlyphMetrics`]（task 6.2・R4.5・probe 規約）。
///
/// 純粋層 `LayoutEngine` の外部注入点（`&dyn GlyphMetrics`）へ、DirectWrite の実測
/// 送り幅を提供する。probe 規約（モジュール doc）:
///
/// - format は描画と同一の [`create_text_format`] 経路（解決済みフォント＋
///   writing_mode 方向レシピ込み）で**生成時に一度だけ**焼く。
/// - `advance` は対象文字の**未折返し probe layout**（[`PROBE_MAX_EXTENT`] 寸）を
///   生成し cluster metrics の width 合計を返す（折返し決定より前の計測＝鶏卵なし）。
/// - 計測値は文字単位でキャッシュする（format 固定＝同一文字は同一送り幅の決定論・
///   probe 規約「確定内容の metrics は不変ゆえキャッシュ可」）。
///
/// UI スレッド専有（COM 層規律）。`line_pitch` は M1 正準式
/// `ceil(font_height × line_pitch_factor)`（正本 [`TextLayerConfig::line_pitch_factor`]・
/// `FixedMetrics` と同一式）に従う。
pub struct DWriteMetrics {
    /// probe layout 生成用 factory（描画と同じ `IDWriteFactory2`）。
    factory: IDWriteFactory2,
    /// 描画と同一経路で生成済みの計測用 format（フォント・サイズ・方向レシピ込み）。
    format: IDWriteTextFormat,
    /// 束縛フォント高さ（`ResolvedFont::height`・format へ焼き込み済みの正本）。
    font_height: f32,
    /// 行送りピッチ係数（正本 [`TextLayerConfig::line_pitch_factor`]）。
    line_pitch_factor: f32,
    /// 文字単位の計測キャッシュ（probe 成功値のみ・失敗は縮退値を返しキャッシュしない）。
    cache: RefCell<HashMap<char, f32>>,
}

impl DWriteMetrics {
    /// 解決済みフォント＋writing_mode から計測用 metrics を生成する。
    ///
    /// format は描画と同一の [`create_text_format`] 経路（既定フォント再試行込み・
    /// R4.2）——probe 規約「描画に使うのと同一のフォント設定・writing_mode 設定」の
    /// 構造的保証。生成失敗は当該経路の log-first（`warn!`/`error!`＋`Err`）に従う。
    pub fn new(
        factory: &IDWriteFactory2,
        font: &ResolvedFont,
        mode: WritingMode,
        config: &TextLayerConfig,
    ) -> Result<DWriteMetrics, TextLayerError> {
        let format = create_text_format(factory, font, mode)?;
        Ok(DWriteMetrics {
            factory: factory.clone(),
            format,
            font_height: font.height,
            line_pitch_factor: config.line_pitch_factor,
            cache: RefCell::new(HashMap::new()),
        })
    }

    /// 1 文字の未折返し probe layout を生成し、cluster metrics の width 合計を返す。
    fn probe_advance(&self, ch: char) -> Result<f32, TextLayerError> {
        let text = HSTRING::from(ch.to_string());
        let layout = self
            .factory
            .create_text_layout(&text, &self.format, PROBE_MAX_EXTENT, PROBE_MAX_EXTENT)
            .map_err(device_err("CreateTextLayout(probe)"))?;
        let clusters = layout
            .get_cluster_metrics()
            .map_err(device_err("GetClusterMetrics(probe)"))?;
        Ok(clusters.iter().map(|c| c.width).sum())
    }

    /// キャッシュ済み計測数（テスト観測用: 同一文字の再計測が probe を増やさない檻）。
    #[cfg(test)]
    fn cached_probe_count(&self) -> usize {
        self.cache.borrow().len()
    }
}

impl GlyphMetrics for DWriteMetrics {
    /// 実測送り幅（image px＝format の DIP そのまま・writing_mode の行内軸方向の寸）。
    ///
    /// `font_height` は束縛フォント（format へ焼き込み済み）と一致していることが契約。
    /// 不一致は `warn!`＋縮退継続（値は束縛 format の実測のまま——probe は描画と同一
    /// format が正準のため引数側へ寄せない）。probe 失敗は `error!`（[`device_err`]）
    /// 済みで、決定論の縮退値（`FixedMetrics` と同式: 全角＝height・半角＝height/2）
    /// を返して継続する（trait は失敗経路を持たない・log-first でログ無し失敗にしない）。
    fn advance(&self, ch: char, font_height: f32) -> f32 {
        if font_height != self.font_height {
            warn!(
                requested = font_height,
                bound = self.font_height,
                "advance へ束縛フォントと異なる font_height が渡された——束縛 format の実測を返す"
            );
        }
        if let Some(&cached) = self.cache.borrow().get(&ch) {
            return cached;
        }
        match self.probe_advance(ch) {
            Ok(advance) => {
                self.cache.borrow_mut().insert(ch, advance);
                advance
            }
            // 失敗は probe_advance 内で error! 済み。縮退値はキャッシュしない（次回再試行）。
            Err(_) => {
                if ch.is_ascii() {
                    self.font_height / 2.0
                } else {
                    self.font_height
                }
            }
        }
    }

    /// 行送りピッチ＝M1 正準式 `ceil(font_height × line_pitch_factor)`。
    fn line_pitch(&self, font_height: f32) -> f32 {
        (font_height * self.line_pitch_factor).ceil()
    }
}

/// `windows_core::Error` を [`TextLayerError::Device`] へ写像する（surface.rs と同型の
/// log-first ヘルパ: `error!`＋`Err` 戻り値・panic 禁止）。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> TextLayerError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "DirectWrite 呼び出しが失敗");
        TextLayerError::Device { hresult, context }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };
    use windows::Win32::Graphics::DirectWrite::{
        DWRITE_FACTORY_TYPE_SHARED, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
        DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
        DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
        DWRITE_READING_DIRECTION_TOP_TO_BOTTOM, DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteTextFormat,
    };
    use wintf::com::dwrite::dwrite_create_factory;

    use super::{
        DEFAULT_FONT_HEIGHT, DEFAULT_FONT_NAME, DWriteMetrics, DirectionRecipe, FontDisableSeam,
        PROBE_MAX_EXTENT, RESERVED_KEY_DISABLE_FONT_PREFIX, ResolvedFont, create_text_format,
    };
    use crate::TextLayerError;
    use crate::canvas::TextEffects;
    use crate::layout::{GlyphMetrics, LayoutEngine};
    use crate::region::TextRegion;
    use crate::state::{TextItem, TextLayerConfig};
    use crate::writing::WritingMode;

    /// テスト用 BalloonModel 生成ヘルパ（font 以外は全成分未指定）。
    fn model_with_font(font: Font) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            font,
            None,
        )
    }

    /// 全成分未指定の Font（フォント定義欠落の balloon 定義）。
    fn empty_font() -> Font {
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
    fn with_log_cage<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
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

    // ── R4.1/R4.2: フォント解決とフォールバック（純粋部・COM 不要） ──

    /// 観測可能な完了状態: フォント名/高さが欠落した balloon 定義に対しても
    /// 既定値でレイアウト生成に必要な設定一式が得られる（ukadoc 既定・正常系につき警告なし）。
    #[test]
    fn missing_font_definition_resolves_to_ukadoc_defaults() {
        let (font, warns, errors) =
            with_log_cage(|| ResolvedFont::resolve(&model_with_font(empty_font())));
        assert_eq!(font.name, DEFAULT_FONT_NAME);
        assert_eq!(font.name, "ＭＳ ゴシック", "既定フォント名は全角 ＭＳ ゴシック");
        assert_eq!(font.height, DEFAULT_FONT_HEIGHT);
        assert_eq!(font.height, 12.0, "既定フォント高さは 12（image px・ukadoc 既定）");
        assert_eq!(font.color, (0, 0, 0), "FontColor 欠落→黒");
        assert!(font.fallback_chain.is_empty());
        assert_eq!((warns, errors), (0, 0), "ukadoc 既定の適用は正常系＝ログなし");
    }

    #[test]
    fn full_font_definition_passes_through() {
        let font = Font::new(
            Some("Meiryo".to_owned()),
            Some(20),
            FontColor::new(Some(10), Some(20), Some(30)),
        );
        let (resolved, warns, _) =
            with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
        assert_eq!(resolved.name, "Meiryo");
        assert_eq!(resolved.height, 20.0);
        assert_eq!(resolved.color, (10, 20, 30));
        assert!(resolved.fallback_chain.is_empty());
        assert_eq!(warns, 0);
    }

    /// カンマ区切り複数指定（SSP 拡張）は M1 では先頭のみ採用・残余は型シームに保持。
    #[test]
    fn comma_separated_names_adopt_first_and_keep_rest_as_seam() {
        let font = Font::new(
            Some("Meiryo, ＭＳ Ｐゴシック ,ＭＳ ゴシック".to_owned()),
            None,
            FontColor::new(None, None, None),
        );
        let resolved = ResolvedFont::resolve(&model_with_font(font));
        assert_eq!(resolved.name, "Meiryo", "先頭名のみ採用（M1）");
        assert_eq!(
            resolved.fallback_chain,
            vec!["ＭＳ Ｐゴシック".to_owned(), "ＭＳ ゴシック".to_owned()],
            "残余は trim 済みでフォールバック連鎖シームに保持（M1 未消費）"
        );
    }

    /// 宣言はあるが実質空の font.name は縮退（warn＋既定フォント・R4.2 log-first）。
    #[test]
    fn empty_font_name_falls_back_to_default_with_warn() {
        for raw in ["", "  ", " , "] {
            let font = Font::new(
                Some(raw.to_owned()),
                None,
                FontColor::new(None, None, None),
            );
            let (resolved, warns, _) =
                with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
            assert_eq!(resolved.name, DEFAULT_FONT_NAME, "raw {raw:?} は既定フォントへ");
            assert!(resolved.fallback_chain.is_empty());
            assert_eq!(warns, 1, "raw {raw:?} はちょうど 1 回 warn を記録する");
        }
    }

    /// font.height,0 は DirectWrite fontsize の正値制約を満たせない縮退値（warn＋既定 12）。
    #[test]
    fn zero_height_falls_back_to_default_with_warn() {
        let font = Font::new(None, Some(0), FontColor::new(None, None, None));
        let (resolved, warns, _) =
            with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
        assert_eq!(resolved.height, DEFAULT_FONT_HEIGHT);
        assert_eq!(warns, 1);
    }

    /// font.color は成分独立既定 0（部分欠落→欠落成分のみ 0・ukadoc 既定 0＝正常系）。
    #[test]
    fn partial_color_channels_default_to_zero() {
        let font = Font::new(None, None, FontColor::new(Some(255), None, Some(7)));
        let (resolved, warns, _) =
            with_log_cage(|| ResolvedFont::resolve(&model_with_font(font)));
        assert_eq!(resolved.color, (255, 0, 7));
        assert_eq!(warns, 0);
    }

    // ── 方向レシピ: writing_mode 解釈結果→DirectWrite 設定の一意導出（design 写像表） ──

    /// design.md 写像表どおりの一意導出:
    /// HorizontalTb→Reading LTR＋Flow TTB／VerticalRl→Reading TTB＋Flow RTL／
    /// VerticalLr→Reading TTB＋Flow LTR。
    #[test]
    fn direction_recipe_maps_three_modes_per_design_table() {
        let h = DirectionRecipe::for_mode(WritingMode::HorizontalTb);
        assert_eq!(h.reading, DWRITE_READING_DIRECTION_LEFT_TO_RIGHT);
        assert_eq!(h.flow, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM);

        let vrl = DirectionRecipe::for_mode(WritingMode::VerticalRl);
        assert_eq!(vrl.reading, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM);
        assert_eq!(vrl.flow, DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT);

        let vlr = DirectionRecipe::for_mode(WritingMode::VerticalLr);
        assert_eq!(vlr.reading, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM);
        assert_eq!(vlr.flow, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT);

        // 3 方向は互いに異なるレシピへ写る（一意導出＝単射）。
        assert_ne!(h, vrl);
        assert_ne!(h, vlr);
        assert_ne!(vrl, vlr);
    }

    /// いずれの方向も Alignment LEADING＋Paragraph NEAR（design 写像表の共通部）。
    #[test]
    fn all_direction_recipes_share_leading_near_alignment() {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let recipe = DirectionRecipe::for_mode(mode);
            assert_eq!(recipe.text_alignment, DWRITE_TEXT_ALIGNMENT_LEADING);
            assert_eq!(recipe.paragraph_alignment, DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
        }
    }

    // ── R10.3: 文字装飾／disable.font.* は型シームのみ（実挙動なし） ──

    /// 装飾（TextEffects）と disable.font.* シームは型のみ＝データを一切持たない
    /// （zero-sized・M1 で描画へ影響し得ない構造保証）。
    #[test]
    fn decoration_and_disable_seams_are_type_only() {
        assert_eq!(RESERVED_KEY_DISABLE_FONT_PREFIX, "disable.font.");
        assert_eq!(std::mem::size_of::<TextEffects>(), 0);
        assert_eq!(std::mem::size_of::<FontDisableSeam>(), 0);
        // ResolvedFont はシームを保持するが Default 生成のみ（実挙動なし）。
        let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        assert_eq!(resolved.effects, TextEffects::default());
        assert_eq!(resolved.disable, FontDisableSeam::default());
    }

    // ── COM 検証（headless DWrite・デバイス非依存・窓不要） ──

    /// TextFormat の実設定を読み戻す（family 名・fontsize・4 方向設定）。
    fn read_family_name(format: &IDWriteTextFormat) -> String {
        unsafe {
            let len = format.GetFontFamilyNameLength() as usize;
            let mut buf = vec![0u16; len + 1];
            format
                .GetFontFamilyName(&mut buf)
                .expect("GetFontFamilyName");
            String::from_utf16_lossy(&buf[..len])
        }
    }

    /// 観測可能な完了状態（COM 側）: 欠落 balloon 定義→既定値一式で実 TextFormat が
    /// 生成でき、3 方向それぞれでレシピどおりの設定が焼き込まれている。
    #[test]
    fn text_format_from_missing_definition_carries_defaults_and_recipe() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
            .expect("DWriteCreateFactory（デバイス非依存・headless 可）");
        let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let recipe = DirectionRecipe::for_mode(mode);
            let format = create_text_format(&factory, &resolved, mode)
                .expect("既定値一式で TextFormat 生成が成立する");
            assert_eq!(read_family_name(&format), "ＭＳ ゴシック");
            unsafe {
                assert_eq!(format.GetFontSize(), 12.0);
                assert_eq!(format.GetReadingDirection(), recipe.reading, "{mode:?}");
                assert_eq!(format.GetFlowDirection(), recipe.flow, "{mode:?}");
                assert_eq!(format.GetTextAlignment(), recipe.text_alignment);
                assert_eq!(
                    format.GetParagraphAlignment(),
                    recipe.paragraph_alignment
                );
            }
        }
    }

    /// 明示定義（名前・高さ）はそのまま TextFormat へ写る（fontsize＝font.height 素通し）。
    #[test]
    fn text_format_honors_explicit_name_and_height() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let font = Font::new(
            Some("ＭＳ Ｐゴシック".to_owned()),
            Some(20),
            FontColor::new(None, None, None),
        );
        let resolved = ResolvedFont::resolve(&model_with_font(font));
        let format = create_text_format(&factory, &resolved, WritingMode::HorizontalTb)
            .expect("明示定義で TextFormat 生成が成立する");
        assert_eq!(read_family_name(&format), "ＭＳ Ｐゴシック");
        unsafe {
            assert_eq!(format.GetFontSize(), 20.0);
        }
    }

    /// フォント生成失敗経路（R4.2・Error Categories）: 生成失敗→warn＋既定フォント再試行→
    /// なお失敗は error!＋Device Err（panic しない）。fontsize 非正値は DirectWrite が
    /// 決定論的に拒否するため、再試行でも失敗する入力として経路全体を檻化する。
    #[test]
    fn unusable_format_surfaces_device_error_after_default_retry() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        // resolve は正値を保証するため、失敗経路は手組みの縮退値で叩く（crate 内テスト特権）。
        let mut resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        resolved.height = 0.0;
        let (result, warns, errors) = with_log_cage(|| {
            create_text_format(&factory, &resolved, WritingMode::HorizontalTb)
        });
        match result {
            Err(TextLayerError::Device { context, .. }) => {
                assert_eq!(context, "CreateTextFormat");
            }
            other => panic!("Device エラーを期待したが {other:?}"),
        }
        assert_eq!(warns, 1, "初回失敗→既定フォント再試行の warn がちょうど 1 回");
        assert_eq!(errors, 1, "再試行失敗→error! がちょうど 1 回");
    }

    // ── task 6.2 R4.5: DWriteMetrics——計測専用 probe TextLayout（probe 規約） ──
    //
    // probe 規約（design discussion #1 裁定）: 未折返し（折返し無効寸）の測定専用
    // TextLayout を、描画と同一の create_text_format 経路（フォント・サイズ・
    // writing_mode 写像設定込み）で折返し決定の前に生成し、cluster metrics から
    // advance を得る（鶏卵の構造的切断）。probe はキャッシュ可（確定内容の metrics 不変）。

    use wintf::com::dwrite::DWriteTextLayoutExt;

    /// テスト側の手組み probe（実装と独立に同一規約で測る参照値）。
    fn manual_probe_advance(
        factory: &windows::Win32::Graphics::DirectWrite::IDWriteFactory2,
        font: &ResolvedFont,
        mode: WritingMode,
        ch: char,
    ) -> f32 {
        let format = create_text_format(factory, font, mode).expect("参照 format");
        let layout = wintf::com::dwrite::DWriteFactoryExt::create_text_layout(
            factory,
            &windows::core::HSTRING::from(ch.to_string()),
            &format,
            PROBE_MAX_EXTENT,
            PROBE_MAX_EXTENT,
        )
        .expect("参照 probe layout");
        layout
            .get_cluster_metrics()
            .expect("参照 cluster metrics")
            .iter()
            .map(|c| c.width)
            .sum()
    }

    /// 既定フォント（ＭＳ ゴシック 12）の DWriteMetrics を組む。
    fn default_metrics(
        factory: &windows::Win32::Graphics::DirectWrite::IDWriteFactory2,
        mode: WritingMode,
    ) -> DWriteMetrics {
        let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        DWriteMetrics::new(factory, &resolved, mode, &TextLayerConfig::default())
            .expect("既定フォントで DWriteMetrics 生成が成立する")
    }

    /// 観測可能な完了状態（task 6.2）: 実測送り幅は、描画と同一の format 経路で
    /// 生成した未折返し probe layout の cluster metrics と一致する。
    #[test]
    fn dwrite_metrics_advance_matches_manual_probe_layout() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
        for ch in ['あ', 'a', '漢', 'W', '。'] {
            let expected = manual_probe_advance(&factory, &resolved, WritingMode::HorizontalTb, ch);
            assert_eq!(
                metrics.advance(ch, DEFAULT_FONT_HEIGHT),
                expected,
                "{ch:?} の実測 advance が probe 参照値と一致する"
            );
            assert!(expected > 0.0, "{ch:?} の advance は正値");
        }
    }

    /// probe は writing_mode 写像設定込みの同一 format で生成される——縦書き
    /// （vertical_rl）の実測は縦書き format の probe 参照値と一致する（横書き format
    /// の値ではない）。
    #[test]
    fn dwrite_metrics_probe_carries_writing_mode_recipe() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let metrics = default_metrics(&factory, mode);
            for ch in ['あ', 'a', '、'] {
                assert_eq!(
                    metrics.advance(ch, DEFAULT_FONT_HEIGHT),
                    manual_probe_advance(&factory, &resolved, mode, ch),
                    "{mode:?} {ch:?}: probe は当該 writing_mode の format で測られる"
                );
            }
        }
    }

    /// 等幅（ＭＳ ゴシック）: 全角＝半角×2 の実測。プロポーショナル
    /// （ＭＳ Ｐゴシック）: 'i' と 'W' の送り幅が異なる実測——FixedMetrics の
    /// 仮想値では出ない差が実測で得られることの檻。
    #[test]
    fn dwrite_metrics_measures_fixed_pitch_and_proportional_distinctly() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        // 等幅: 既定 ＭＳ ゴシック——全角は半角のちょうど 2 倍。
        let gothic = default_metrics(&factory, WritingMode::HorizontalTb);
        let full = gothic.advance('あ', DEFAULT_FONT_HEIGHT);
        let half = gothic.advance('a', DEFAULT_FONT_HEIGHT);
        assert!(full > 0.0 && half > 0.0);
        assert_eq!(full, half * 2.0, "等幅フォントの全角＝半角×2");
        // プロポーショナル: ＭＳ Ｐゴシック——'i' は 'W' より狭い。
        let p_font = Font::new(
            Some("ＭＳ Ｐゴシック".to_owned()),
            Some(12),
            FontColor::new(None, None, None),
        );
        let p_resolved = ResolvedFont::resolve(&model_with_font(p_font));
        let p_metrics = DWriteMetrics::new(
            &factory,
            &p_resolved,
            WritingMode::HorizontalTb,
            &TextLayerConfig::default(),
        )
        .expect("プロポーショナルで DWriteMetrics 生成が成立する");
        let narrow = p_metrics.advance('i', 12.0);
        let wide = p_metrics.advance('W', 12.0);
        assert!(
            narrow < wide,
            "プロポーショナルの実測: 'i'({narrow}) < 'W'({wide})"
        );
    }

    /// 決定論: 同一フォント・同一文字→同一送り幅（同一インスタンスの再計測も
    /// 別インスタンスも完全一致・R2.5 系/R11.6 の COM 側檻）。
    #[test]
    fn dwrite_metrics_is_deterministic_across_calls_and_instances() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let first = default_metrics(&factory, WritingMode::VerticalRl);
        let second = default_metrics(&factory, WritingMode::VerticalRl);
        for ch in ['あ', 'x', '！'] {
            let a = first.advance(ch, DEFAULT_FONT_HEIGHT);
            let b = first.advance(ch, DEFAULT_FONT_HEIGHT);
            let c = second.advance(ch, DEFAULT_FONT_HEIGHT);
            assert_eq!(a, b, "{ch:?}: 再計測（キャッシュ経路）も同値");
            assert_eq!(a, c, "{ch:?}: 別インスタンスも同値");
        }
    }

    /// キャッシュ規約: 同一文字の probe は 1 回だけ生成され、以後はキャッシュから
    /// 返る（値は同一・probe 規約「確定内容の metrics は不変ゆえキャッシュ可」）。
    #[test]
    fn dwrite_metrics_caches_probed_advances() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
        assert_eq!(metrics.cached_probe_count(), 0);
        let first = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
        assert_eq!(metrics.cached_probe_count(), 1);
        let again = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
        assert_eq!(metrics.cached_probe_count(), 1, "同一文字の再計測は probe を増やさない");
        assert_eq!(first, again);
        metrics.advance('a', DEFAULT_FONT_HEIGHT);
        assert_eq!(metrics.cached_probe_count(), 2);
    }

    /// line_pitch は M1 正準式 ceil(font_height × TextLayerConfig::line_pitch_factor)
    /// ——FixedMetrics と同じ正本（trait doc）に従う。
    #[test]
    fn dwrite_metrics_line_pitch_follows_config_canon() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
        assert_eq!(metrics.line_pitch(12.0), 15.0);
        assert_eq!(metrics.line_pitch(10.0), 13.0, "12.5 → ceil 13");
        // 係数は config が正本——非既定係数も反映される。
        let resolved = ResolvedFont::resolve(&model_with_font(empty_font()));
        let config = TextLayerConfig {
            line_pitch_factor: 2.0,
            ..TextLayerConfig::default()
        };
        let doubled =
            DWriteMetrics::new(&factory, &resolved, WritingMode::HorizontalTb, &config)
                .expect("非既定係数でも生成が成立する");
        assert_eq!(doubled.line_pitch(10.0), 20.0);
    }

    /// 契約檻: advance へ束縛フォントと異なる font_height が渡されたら warn（縮退継続・
    /// 値は束縛 format の実測のまま——probe は描画と同一 format が正準ゆえ）。
    #[test]
    fn dwrite_metrics_warns_on_font_height_mismatch() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
        let bound = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
        let (mismatched, warns, errors) = with_log_cage(|| metrics.advance('あ', 99.0));
        assert_eq!(warns, 1, "束縛高さと異なる font_height はちょうど 1 回 warn");
        assert_eq!(errors, 0);
        assert_eq!(mismatched, bound, "値は束縛 format の実測のまま（縮退継続）");
    }

    /// 観測可能な完了状態（task 6.2）: LayoutEngine の外部注入点（&dyn GlyphMetrics）へ
    /// 実測値ベースの送り幅を提供でき、折返しが実測 advance で決まる——probe（計測）が
    /// 折返し決定より前＝鶏卵にならない順序で機能する単体確認。
    #[test]
    fn layout_engine_wraps_using_measured_advances() {
        let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("factory");
        let metrics = default_metrics(&factory, WritingMode::HorizontalTb);
        let full = metrics.advance('あ', DEFAULT_FONT_HEIGHT);
        // 折返し閾値＝実測全角 1.5 個分: 2 文字目で「行内位置＋次グリフ幅 > 閾値」が成立。
        let threshold = (full * 1.5).round() as i32;
        let model = BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(Some(0), Some(0)),
            WordWrapPoint::new(Some(threshold), None),
            ValidRect::new(None, None, None, None),
            empty_font(),
            None,
        );
        let region = TextRegion::resolve(&model, (400, 224), WritingMode::HorizontalTb);
        let items = [TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }];
        let lines = LayoutEngine::layout(
            &items,
            2,
            &region,
            WritingMode::HorizontalTb,
            DEFAULT_FONT_HEIGHT,
            &metrics,
        );
        assert_eq!(lines.len(), 2, "実測 advance が折返し判定を駆動する");
        assert_eq!(lines[0].glyphs.len(), 1);
        assert_eq!(lines[0].glyphs[0].advance, full, "配置グリフの advance は実測値");
        assert_eq!(lines[1].glyphs[0].inline_pos, 0.0, "折返し行は行内開始へ戻る");
    }
}
