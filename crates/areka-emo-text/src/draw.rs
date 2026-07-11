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
use wintf::com::dwrite::DWriteFactoryExt;

use crate::TextLayerError;
use crate::canvas::TextEffects;
use crate::writing::WritingMode;

/// SSP 既定フォント名（**全角表記** ＭＳ ゴシック・ukadoc 既定・R4.2）。
pub const DEFAULT_FONT_NAME: &str = "ＭＳ ゴシック";

/// 既定フォント高さ 12（**image px**・「単位はピクセル：ポイントではない」・ukadoc 既定・R4.1/R4.2）。
pub const DEFAULT_FONT_HEIGHT: f32 = 12.0;

/// TextFormat のロケール（wintf typewriter レシピからの lift・日本語正準）。
const LOCALE_JA_JP: &str = "ja-JP";

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
        DEFAULT_FONT_HEIGHT, DEFAULT_FONT_NAME, DirectionRecipe, FontDisableSeam,
        RESERVED_KEY_DISABLE_FONT_PREFIX, ResolvedFont, create_text_format,
    };
    use crate::TextLayerError;
    use crate::canvas::TextEffects;
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
}
