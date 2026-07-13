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
//! ## 全域再描画オラクル（旧 task 6.3・R3.1/R7.3・task 5 で `#[cfg(test)]` 化）
//!
//! [`DrawExecutor`]: **比較専用の独立オラクル**（本番経路は `ViewboxExecutor` へ移行済み）。
//! 可視窓の行を毎更新オフスクリーン D2D ターゲット（TextSurface の front＝`front_tex`）へ
//! 透明 clear→全域再描画する（差分描画なし・SSP 忠実の確定裁定）:
//!
//! - **可視窓決定（純粋・layout.rs）と描画実行（本型）の分離**（R7.4 のシーム下半分）。
//!   [`VisibleWindow`] の `first_visible_line`＋`block_offset` を消費するだけで、
//!   スクロール判定は持たない。
//! - **行 TextLayout キャッシュ**: 行内容が不変（確定行＝リビール完了行）なら再利用し、
//!   リビール中の行のみ都度更新する。[`DrawExecutor::clear_cache`]（Clear cue の適用点）
//!   のみが全破棄する。
//! - **スケール一点適用**: 描画ターゲットへの `SetTransform(scale(k))` 一点のみ
//!   （k＝`ScaleContract::scale`）。レイアウト・行 TextLayout は image px（96 DPI 名目
//!   ＝DIP と同一視）のままで、DPI API との二重適用は構造的に存在しない。
//! - Image/Surface 住人（M1 型シーム）は `warn!`（executor ごと初回のみ）＋skip（R8.5）。
//!
//! ## probe/描画 一致 invariant（task 6.4・R4.5/R6.1–6.3/R7.5）
//!
//! probe（per-char 計測）と描画行 TextLayout の cluster advance の**同値** invariant は
//! 本モジュールの統合テスト（`probe_advances_match_drawn_line_cluster_advances`／
//! `advance_divergence_would_surface_as_wrap_position_drift`）が檻化する——乖離は
//! クリップに隠れず折返し位置のズレとして赤くなる（design Testing Strategy #5）。

use std::cell::RefCell;
use std::collections::HashMap;

use areka_parsers::balloon::BalloonModel;
use tracing::warn;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    ID2D1Bitmap1, ID2D1DeviceContext,
};
// 比較専用オラクル [`DrawExecutor`]（`#[cfg(test)]`）専用の描画 API——本番 create_d2d_target_bitmap
// が使う定義（上）と分けて cfg(test) に隔離する（非テストビルドの dead import を避ける）。
#[cfg(test)]
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
#[cfg(test)]
use windows::Win32::Graphics::Direct2D::{
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, ID2D1Image,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FLOW_DIRECTION, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
    DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_PARAGRAPH_ALIGNMENT, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_READING_DIRECTION,
    DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteFactory2, IDWriteFontCollection,
    IDWriteTextFormat, IDWriteTextLayout,
};
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::core::{HSTRING, Interface};
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};

use crate::TextLayerError;
use crate::canvas::TextEffects;
use crate::layout::GlyphMetrics;
use crate::state::TextLayerConfig;
use crate::viewbox::LineOverhang;
use crate::writing::WritingMode;

// 以下は比較専用オラクル [`DrawExecutor`]（`#[cfg(test)]`）だけが使う依存——本番経路
// （ViewboxExecutor）は viewbox_draw.rs 側で自前に持つため、非テストビルドの dead import を
// 避けるべく cfg(test) へ隔離する（オラクル隔離の一部・task 5）。
#[cfg(test)]
use windows_numerics::{Matrix3x2, Vector2};
#[cfg(test)]
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
#[cfg(test)]
use wintf::ecs::GraphicsCore;
#[cfg(test)]
use crate::canvas::{ContentCanvas, ResidentContent};
#[cfg(test)]
use crate::layout::VisibleWindow;
#[cfg(test)]
use crate::region::ScaleContract;
#[cfg(test)]
use crate::surface::TextSurface;

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

/// 行 TextLayout の format 前提（フォント名・高さ・writing_mode）——変わると
/// キャッシュ済み行レイアウトの前提が崩れるため format と行キャッシュを組み直す。
///
/// 比較専用オラクル [`DrawExecutor`] 専用（本番 `ViewboxExecutor` は自前のインライン
/// `FormatKey` を持つ・viewbox_draw.rs）——ゆえにオラクルと同じ `#[cfg(test)]` で保全する。
#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
struct FormatKey {
    font_name: String,
    /// f32 のビット表現（PartialEq の全順序比較を避ける・同値判定のみ）。
    font_height_bits: u32,
    mode: WritingMode,
}

/// キャッシュ済みの行 TextLayout（行内容の正本文字列＋実測インクはみ出しと対で保持・
/// 内容不変なら再利用）。`overhang` は生成時に一度だけ [`DWriteTextLayoutExt::get_overhang_metrics`]
/// で実測（確定行は再計測しない）——ViewboxExecutor のダーティ矩形が em ボックス下端はみ出しを
/// 取りこぼさないための実測値（D2）。
struct CachedLineLayout {
    text: String,
    layout: IDWriteTextLayout,
    overhang: LineOverhang,
}

/// 行 TextLayout の生成・キャッシュを担う共有ストア（複数の描画実行が**同一経路**で
/// 行レイアウトを得るための抽出型・design.md「draw.rs の再編（LineLayoutStore 抽出）」）。
///
/// 生成規則（行内軸＝[`PROBE_MAX_EXTENT`]・行送り軸＝`font_height`・同一 format）・キー
/// （canvas 行 index）・内容不変再利用・破棄規律（[`clear`](Self::clear) のみ全破棄）は
/// 抽出前の `DrawExecutor` 内実装と同一——TextLayout 生成経路の完全共有により両描画実行の
/// **byte 等価**を構造化する（RN5）。UI スレッド専有（COM 層規律）。
///
/// `pub(crate)`: [`DrawExecutor`]（front へ全域再描画）と `ViewboxExecutor`
/// （back へダーティ描画・viewbox_draw.rs）が**同一経路**で行レイアウトを得るため
/// crate 内へ公開する（生成規則・キー・破棄規律は不変）。
pub(crate) struct LineLayoutStore {
    /// 行 TextLayout 生成用 factory（probe/描画と同一の `IDWriteFactory2`）。
    factory: IDWriteFactory2,
    /// 行 TextLayout キャッシュ（key＝canvas 行 index。追記単調ゆえ確定行の index/内容は
    /// 不変——リビール中＝最終行のみ内容が変わり都度更新される）。
    cache: HashMap<usize, CachedLineLayout>,
    /// 行 TextLayout の累計生成回数（**常時コンパイル**・後続 task の `DrawStats` へ集計する
    /// ため `#[cfg(test)]` にしない・design「Modified Files」）。
    creations: u64,
}

impl LineLayoutStore {
    /// factory を束ねて空ストアを生成する（factory は clone 保持）。
    pub(crate) fn new(factory: &IDWriteFactory2) -> LineLayoutStore {
        LineLayoutStore {
            factory: factory.clone(),
            cache: HashMap::new(),
            creations: 0,
        }
    }

    /// 行 TextLayout の取得（内容不変なら再利用・変化時のみ生成して置換）。
    ///
    /// 行の箱寸は「行内軸＝折返し無効寸（[`PROBE_MAX_EXTENT`]・折返しは純粋層で決定済み
    /// ＝再折返しさせない）・行送り軸＝`font_height`」。方向レシピ（LEADING/NEAR）により
    /// 行は箱の書字開始角に付くため、描画原点＝行矩形原点で位置が定まる。
    pub(crate) fn line_layout(
        &mut self,
        index: usize,
        text: &str,
        format: &IDWriteTextFormat,
        font_height: f32,
        mode: WritingMode,
    ) -> Result<IDWriteTextLayout, TextLayerError> {
        if let Some(cached) = self.cache.get(&index) {
            if cached.text == text {
                return Ok(cached.layout.clone());
            }
        }
        let (max_width, max_height) = match mode {
            WritingMode::HorizontalTb => (PROBE_MAX_EXTENT, font_height),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (font_height, PROBE_MAX_EXTENT),
        };
        let layout = self
            .factory
            .create_text_layout(&HSTRING::from(text), format, max_width, max_height)
            .map_err(device_err("CreateTextLayout(line)"))?;
        self.creations += 1;
        // 実測インクはみ出し（生成時 1 回・確定行は再計測しない）。行ボックスのブロック軸寸は
        // font_height（横＝max_height／縦＝max_width）ゆえ、その軸の overhang が em ボックスからの
        // はみ出しを直接与える。行内軸は巨大 PROBE_MAX_EXTENT 箱ゆえ overhang は巨大負値＝`max(0.0)`
        // で 0 に丸まる（resident_rect はブロック軸の overhang のみ使う）。
        let overhang = measure_line_overhang(&layout)?;
        self.cache.insert(
            index,
            CachedLineLayout {
                text: text.to_owned(),
                layout: layout.clone(),
                overhang,
            },
        );
        Ok(layout)
    }

    /// キャッシュ済み行の実測インクはみ出し（[`LineOverhang`]）——`ViewboxExecutor` が plan へ渡す。
    /// 未生成 index は `None`（呼び手は既定 0＝em ボックス丈として扱う）。
    pub(crate) fn overhang(&self, index: usize) -> Option<LineOverhang> {
        self.cache.get(&index).map(|c| c.overhang)
    }

    /// キャッシュを全破棄する（Clear cue の適用点・破棄はこの口だけ）。
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    /// 行 TextLayout の累計生成回数（常時コンパイル・`DrawStats` 集計とテスト観測の共通読み口）。
    /// `ViewboxExecutor::render`（viewbox_draw.rs）が本フレームの生成増分を `DrawStats`
    /// （`line_layout_creations`）へ集計するために非テストビルドでも読む。
    pub(crate) fn creations(&self) -> u64 {
        self.creations
    }
}

/// 行 TextLayout の実測インクはみ出し（[`LineOverhang`]・image px・全成分 ≥ 0）を返す。
///
/// [`DWriteTextLayoutExt::get_overhang_metrics`]（`GetOverhangMetrics`）はレイアウトボックス各辺
/// からのはみ出し（正＝外側・DIP）を返す。行ボックスのブロック軸寸が `font_height`（横＝`max_height`
/// ／縦＝`max_width`）に設定済みゆえ、その軸の値が em ボックス下端/上端（縦は左右）からのはみ出しを
/// 直接与える。行内軸は巨大 `PROBE_MAX_EXTENT` 箱ゆえ値は巨大負値＝`max(0.0)` で 0 に丸まる
/// （`resident_rect` はブロック軸の overhang のみ使うため、これで正しくブロック軸だけが効く）。
fn measure_line_overhang(layout: &IDWriteTextLayout) -> Result<LineOverhang, TextLayerError> {
    let o = layout
        .get_overhang_metrics()
        .map_err(device_err("GetOverhangMetrics(line)"))?;
    Ok(LineOverhang {
        top: o.top.max(0.0),
        bottom: o.bottom.max(0.0),
        left: o.left.max(0.0),
        right: o.right.max(0.0),
    })
}

/// **比較専用の独立オラクル**（本番経路は `ViewboxExecutor` へ移行済み・除去は本ユニットの
/// 範囲外——別決断）。live-diff で viewbox とバイト比較するために全域再描画方式を
/// `#[cfg(test)]` で保全する（task 5・design.md「draw.rs の再編（LineLayoutStore 抽出＋
/// オラクル化）」）。render のロジック・origin 式は viewbox 都合で一切変えない——変えれば
/// 比較の意味を失う（オラクルの独立性）。
///
/// ContentCanvas の可視窓を DirectWrite/D2D で全域再描画する実行部
/// （旧 task 6.3・R3.1/R7.3・design.md「DrawExecutor（draw.rs）」）。
///
/// 毎更新、TextSurface の front（`front_tex`・オフスクリーン D2D ターゲット）を透明 clear→
/// 可視窓の行を描画する（差分描画なし）。スクロールも同経路（可視窓決定は純粋層の
/// [`VisibleWindow`] が済ませている——R7.4 分離シームの描画実行側）。
///
/// - **行 TextLayout キャッシュ**: 確定行（内容不変）は再生成しない・リビール中の行のみ
///   都度更新・[`clear_cache`](Self::clear_cache)（Clear cue 適用点）のみ全破棄。
/// - **スケール一点適用**: `SetTransform(scale(k))` を描画ターゲットへ一度だけ適用
///   （k＝[`ScaleContract::scale`]・フォントサイズ/レイアウトは image px のまま）。
/// - 失敗は log-first（`error!`＋`Err`・当該フレーム skip・次フレーム再試行）・panic 禁止。
///
/// UI スレッド専有（COM 層規律）。
#[cfg(test)]
pub struct DrawExecutor {
    /// 行 TextLayout 生成用 factory（probe/描画と同一の `IDWriteFactory2`）。
    dwrite: IDWriteFactory2,
    /// 専用 D2D DC（wintf の共有 DC の描画状態を汚さない・ターゲットは render 中のみ設定）。
    dc: ID2D1DeviceContext,
    /// 描画/計測共用 format（[`create_text_format`] 経路・FormatKey 不変なら再利用）。
    format: Option<(FormatKey, IDWriteTextFormat)>,
    /// 行 TextLayout の生成・キャッシュストア（[`LineLayoutStore`]・両描画実行が同一経路で
    /// 行レイアウトを得るための共有資産＝抽出前の内蔵キャッシュと byte 等価・RN5）。
    line_store: LineLayoutStore,
    /// Image/Surface 住人シームの warn 抑制フラグ（executor ごと初回のみ・R8.5）。
    seam_warned: bool,
}

#[cfg(test)]
impl DrawExecutor {
    /// `GraphicsCore` から描画実行部を生成する（DWrite factory＋専用 D2D DC）。
    ///
    /// デバイス未初期化（`GraphicsCore` 無効化後）は log-first で `Device` エラー。
    pub fn new(core: &GraphicsCore) -> Result<DrawExecutor, TextLayerError> {
        let dwrite = core
            .dwrite_factory()
            .ok_or_else(|| none_err("GraphicsCore::dwrite_factory"))?
            .clone();
        let d2d = core
            .d2d_device()
            .ok_or_else(|| none_err("GraphicsCore::d2d_device"))?;
        let dc = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .map_err(device_err("CreateDeviceContext(DrawExecutor)"))?;
        // 行キャッシュは共有ストアへ抽出済み。store の factory は `dwrite` の別 clone
        // （`ensure_format` が `dwrite` を使い続けるため本体にも保持する・最小変更）。
        let line_store = LineLayoutStore::new(&dwrite);
        Ok(DrawExecutor {
            dwrite,
            dc,
            format: None,
            line_store,
            seam_warned: false,
        })
    }

    /// Clear cue の適用点: 行 TextLayout キャッシュを全破棄する
    /// （design「確定行は再生成しない・**Clear で全破棄**」——破棄はこの口だけ・
    /// 共有ストア [`LineLayoutStore::clear`] へ委譲）。
    pub fn clear_cache(&mut self) {
        self.line_store.clear();
    }

    /// 可視窓を全域再描画して TextSurface の front（`front_tex`）へ焼く
    /// （失敗は `error!`＋`Err`・panic 禁止。提示（swapchain Present）は
    /// [`TextSurface::present`] の領分——呼び手が本 render の後に呼ぶ）。
    ///
    /// 全域再描画の構造: 資源確定（可謬・BeginDraw 前）→ 透明 clear →
    /// `SetTransform(scale(k))` 一点 → 可視窓の行を `DrawTextLayout` → EndDraw。
    pub fn render(
        &mut self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        font: &ResolvedFont,
        mode: WritingMode,
        contract: &ScaleContract,
        surface: &mut TextSurface,
    ) -> Result<(), TextLayerError> {
        let format = self.ensure_format(font, mode)?;

        // ── Phase 1（可謬）: 描画資源を BeginDraw の前に確定する ──
        // 可視窓の行（first_visible_line 以降）の TextLayout と描画原点を組む。
        // 原点＝住人の平行移動（validrect-local・image px）＋ブロック軸の可視窓オフセット
        // （横書き＝y・縦書き＝x——軸読み替え正準表のスクロール方向）。
        let mut draws: Vec<(Vector2, IDWriteTextLayout)> = Vec::new();
        for (index, resident) in canvas
            .residents
            .iter()
            .enumerate()
            .skip(window.first_visible_line)
        {
            let run = match &resident.content {
                ResidentContent::GlyphRun(run) => run,
                seam => {
                    // M1 型シーム（Image/Surface）: warn（executor ごと初回のみ）＋skip（R8.5）。
                    if !self.seam_warned {
                        self.seam_warned = true;
                        warn!(
                            resident = ?seam,
                            "Image/Surface 住人は M1 型シームのため描画を skip する（実挙動なし）"
                        );
                    }
                    continue;
                }
            };
            if run.glyphs.is_empty() {
                continue;
            }
            let text: String = run.glyphs.iter().map(|g| g.ch).collect();
            let layout = self.line_layout(index, &text, &format, font.height, mode)?;
            let (dx, dy) = resident.transform.offset();
            let origin = match mode {
                WritingMode::HorizontalTb => Vector2 {
                    X: dx,
                    Y: dy + window.block_offset,
                },
                WritingMode::VerticalRl | WritingMode::VerticalLr => Vector2 {
                    X: dx + window.block_offset,
                    Y: dy,
                },
            };
            draws.push((origin, layout));
        }

        let (r, g, b) = font.color;
        let brush = self
            .dc
            .create_solid_color_brush(
                &D2D1_COLOR_F {
                    r: r as f32 / 255.0,
                    g: g as f32 / 255.0,
                    b: b as f32 / 255.0,
                    a: 1.0,
                },
                None,
            )
            .map_err(device_err("CreateSolidColorBrush"))?;
        let target = create_target_bitmap(&self.dc, surface)?;

        // ── Phase 2（描画・全域再描画）: この区間の D2D 呼び出しは不可謬（戻り値なし）で、
        // 失敗は EndDraw に集約される。SetTarget は成否によらず必ず解除する。 ──
        unsafe { self.dc.SetTarget(&target) };
        unsafe { self.dc.BeginDraw() };
        // 透明 clear（premultiplied 全 0）＝差分描画なしの構造（R7.3）。
        self.dc.clear(None);
        // スケール一点適用: k はここ（描画ターゲットの変換）だけ。レイアウト座標は image px。
        self.dc.set_transform(&Matrix3x2 {
            M11: contract.scale,
            M12: 0.0,
            M21: 0.0,
            M22: contract.scale,
            M31: 0.0,
            M32: 0.0,
        });
        for (origin, layout) in &draws {
            self.dc
                .draw_text_layout(*origin, layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
        }
        let end = unsafe { self.dc.EndDraw(None, None) };
        unsafe { self.dc.SetTarget(None::<&ID2D1Image>) };
        end.map_err(device_err("EndDraw"))?;
        Ok(())
    }

    /// 描画/計測共用 format の確保（[`create_text_format`] 経路・FormatKey 不変なら再利用）。
    ///
    /// フォント/方向の変更は行キャッシュの前提（同一 format で組んだ TextLayout）を崩す
    /// ため組み直す——Clear とは別口の**正当性上の必然**（実運用は actor ごと固定のため
    /// 通常経路では発火しない・`debug!` 記録）。
    fn ensure_format(
        &mut self,
        font: &ResolvedFont,
        mode: WritingMode,
    ) -> Result<IDWriteTextFormat, TextLayerError> {
        let key = FormatKey {
            font_name: font.name.clone(),
            font_height_bits: font.height.to_bits(),
            mode,
        };
        if let Some((cached_key, format)) = &self.format {
            if *cached_key == key {
                return Ok(format.clone());
            }
            tracing::debug!(
                ?key,
                "フォント/方向が変わったため format と行レイアウトキャッシュを組み直す"
            );
            self.line_store.clear();
        }
        let format = create_text_format(&self.dwrite, font, mode)?;
        self.format = Some((key, format.clone()));
        Ok(format)
    }

    /// 行 TextLayout の取得（共有ストア [`LineLayoutStore::line_layout`] へ委譲）。
    ///
    /// 内容不変なら再利用・変化時のみ生成して置換——生成規則・キー・再利用規律は
    /// すべて共有ストア側に一元化されている（両描画実行の byte 等価前提・RN5）。
    fn line_layout(
        &mut self,
        index: usize,
        text: &str,
        format: &IDWriteTextFormat,
        font_height: f32,
        mode: WritingMode,
    ) -> Result<IDWriteTextLayout, TextLayerError> {
        self.line_store
            .line_layout(index, text, format, font_height, mode)
    }

    /// テスト観測用: 行 TextLayout の累計生成回数（確定行キャッシュの檻・共有ストアの
    /// 常時コンパイルカウンタを既存テストの usize 比較へ写す）。
    #[cfg(test)]
    fn line_layout_creations(&self) -> usize {
        self.line_store.creations() as usize
    }
}

/// 描画面テクスチャ（front/back のいずれか）を D2D ターゲット bitmap として巻く共有ヘルパ
/// （B8G8R8A8 premultiplied・96 DPI 名目——スケールは `SetTransform` の一点のみ。描画面
/// テクスチャは SHADER_RESOURCE bind を持たないため CANNOT_DRAW を併記する）。
///
/// [`DrawExecutor`]（front を巻く）と `ViewboxExecutor`（back を巻く・viewbox_draw.rs）が
/// **同一 props** でターゲット bitmap を得ることで byte 等価の構造前提を共有する（RN5）。
pub(crate) fn create_d2d_target_bitmap(
    dc: &ID2D1DeviceContext,
    tex: &ID3D11Texture2D,
) -> Result<ID2D1Bitmap1, TextLayerError> {
    let dxgi_surface: IDXGISurface = tex
        .cast()
        .map_err(device_err("target tex->IDXGISurface cast"))?;
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: core::mem::ManuallyDrop::new(None),
    };
    unsafe {
        dc.CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&props as *const _))
    }
    .map_err(device_err("CreateBitmapFromDxgiSurface"))
}

/// TextSurface の front（`front_tex`）を D2D ターゲット bitmap として巻く（比較専用オラクル
/// [`DrawExecutor`] 専用・共有ヘルパ [`create_d2d_target_bitmap`] へ委譲——props・挙動は不変）。
/// オラクルと同じ `#[cfg(test)]` で保全する（本番 `ViewboxExecutor` は back を巻く）。
#[cfg(test)]
fn create_target_bitmap(
    dc: &ID2D1DeviceContext,
    surface: &TextSurface,
) -> Result<ID2D1Bitmap1, TextLayerError> {
    create_d2d_target_bitmap(dc, surface.front_tex())
}

/// `Option` が `None`（デバイス未初期化など本来到達しない欠落）を
/// [`TextLayerError::Device`] にする（surface.rs と同型の log-first ヘルパ）。
///
/// 比較専用オラクル [`DrawExecutor::new`]（`#[cfg(test)]`）専用ゆえ同じく cfg(test)
/// で保全する（本番の欠落写像は各所の [`device_err`] が担う）。
#[cfg(test)]
fn none_err(context: &'static str) -> TextLayerError {
    tracing::error!(context, "必須リソースが欠落（デバイス未初期化 または 前提不成立）");
    TextLayerError::Device { hresult: 0, context }
}

/// `windows_core::Error` を [`TextLayerError::Device`] へ写像する（surface.rs と同型の
/// log-first ヘルパ: `error!`＋`Err` 戻り値・panic 禁止）。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> TextLayerError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "DirectWrite/D2D 呼び出しが失敗");
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

    // ── task 6.3 R3.1/R7.3: DrawExecutor——可視窓の全域再描画を自前供給面へ焼き込む ──
    //
    // 観測可能な完了状態: 可視グリフ数が増えるほど自前供給面の読み戻し結果で非透明
    // ピクセルが単調に増加し、Clear 後は全域が透明に戻る。併せて構造契約を檻化する:
    // 差分描画でなく全域再描画（残渣なし）・確定行 TextLayout キャッシュは Clear のみ
    // 全破棄・合成スケールは描画ターゲットへの一点適用（二重適用/未適用の排除）。

    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::name::Name;
    use bevy_ecs::prelude::World;
    use windows::UI::Composition::Compositor;
    use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
    use wintf::com::wuc::create_dispatcher_queue_controller;
    use wintf::ecs::{GraphicsCore, Visual};

    use super::DrawExecutor;
    use crate::actor::TextSlotBinding;
    use crate::canvas::{
        ContentCanvas, ImageSeam, RegionTransform, Resident, ResidentContent, SurfaceSeam,
    };
    use crate::layout::VisibleWindow;
    use crate::region::ScaleContract;
    use crate::surface::TextSurface;

    /// テスト用 WUC apartment / dispatcher（surface.rs テストと同一方針:
    /// COM 未初期化のテストスレッドでは ASTA 第一候補・NONE 保険）。
    fn make_dispatcher_and_compositor()
    -> (windows::System::DispatcherQueueController, Compositor) {
        let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e_asta| {
                create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
            })
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        let compositor = Compositor::new().expect("Compositor::new 失敗");
        (dq, compositor)
    }

    /// 描画テスト一式（dispatcher/compositor/core/World の寿命を束ねる・headless）。
    struct DrawRig {
        _dq: windows::System::DispatcherQueueController,
        compositor: Compositor,
        core: GraphicsCore,
        world: World,
    }

    impl DrawRig {
        fn new() -> DrawRig {
            let (_dq, compositor) = make_dispatcher_and_compositor();
            let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
            DrawRig {
                _dq,
                compositor,
                core,
                world: World::new(),
            }
        }

        /// 予約スロット（emo-present VisualMount 同型）を組み、image px 原寸と k から
        /// TextSurface を装着する（物理寸＝ceil(image × k)・offset (0,0)）。
        fn attach(&mut self, image_size: (u32, u32), k: f32) -> TextSurface {
            let window = self.world.spawn_empty().id();
            let slot = self
                .world
                .spawn((
                    Name::new("emo-text-layer-slot"),
                    Visual::default(),
                    ChildOf(window),
                ))
                .id();
            self.world.flush();
            let physical = (
                (image_size.0 as f32 * k).ceil() as u32,
                (image_size.1 as f32 * k).ceil() as u32,
            );
            let binding = TextSlotBinding::new(slot, window, k, physical);
            TextSurface::attach(
                &mut self.world,
                &binding,
                &self.compositor,
                &self.core,
                physical,
                (0.0, 0.0),
            )
            .expect("TextSurface::attach 失敗")
        }
    }

    /// テスト用 BalloonModel（幾何のみ・font 未指定＝既定 ＭＳ ゴシック）。
    fn geo_model(
        origin: (Option<i32>, Option<i32>),
        font_height: Option<u32>,
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(origin.0, origin.1),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(None, font_height, FontColor::new(None, None, None)),
            None,
        )
    }

    /// 文字列→グリフ item 列。
    fn glyph_items(s: &str) -> Vec<TextItem> {
        s.chars().map(|ch| TextItem::Glyph { ch }).collect()
    }

    /// layout→canvas→visible_window→render→read_back の通し（テスト用最短経路）。
    #[allow(clippy::too_many_arguments)]
    fn render_items(
        executor: &mut DrawExecutor,
        surface: &mut TextSurface,
        items: &[TextItem],
        visible: usize,
        region: &TextRegion,
        mode: WritingMode,
        font: &ResolvedFont,
        metrics: &DWriteMetrics,
        contract: &ScaleContract,
    ) -> Vec<u8> {
        let lines = LayoutEngine::layout(items, visible, region, mode, font.height, metrics);
        let canvas = crate::canvas::ContentCanvas::from_layout(&lines, region, mode);
        let window = LayoutEngine::visible_window(&lines, region, mode);
        executor
            .render(&canvas, &window, font, mode, contract, surface)
            .expect("DrawExecutor::render 失敗");
        surface.read_back().expect("read_back 失敗")
    }

    /// 非透明ピクセル数（BGRA 密配列の α ≠ 0）。
    fn opaque_count(bytes: &[u8]) -> usize {
        bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
    }

    /// 非透明ピクセルの外接範囲 (min_x, min_y)（インクなしは None）。
    fn ink_min(bytes: &[u8], width: u32) -> Option<(u32, u32)> {
        let mut min: Option<(u32, u32)> = None;
        for (i, px) in bytes.chunks_exact(4).enumerate() {
            if px[3] != 0 {
                let (x, y) = (i as u32 % width, i as u32 / width);
                min = Some(match min {
                    None => (x, y),
                    Some((mx, my)) => (mx.min(x), my.min(y)),
                });
            }
        }
        min
    }

    /// 観測可能な完了状態（task 6.3）: 可視グリフ数が増えるほど自前供給面の読み戻しで
    /// 非透明ピクセルが単調に増加し（R3.1 typewriter 進行の描画側）、Clear（状態全消去＋
    /// 確定行キャッシュ全破棄）後は全域が透明に戻る。
    #[test]
    fn render_grows_opaque_pixels_monotonically_and_clear_restores_transparency() {
        let mut rig = DrawRig::new();
        let image = (120u32, 60u32);
        let mut surface = rig.attach(image, 1.0);
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
        let mode = WritingMode::HorizontalTb;
        let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
            .expect("DWriteMetrics 生成失敗");
        let region = TextRegion::resolve(&geo_model((Some(0), Some(0)), None), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

        let items = glyph_items("あいうえお");
        let mut counts = Vec::new();
        for visible in 0..=items.len() {
            let bytes = render_items(
                &mut executor, &mut surface, &items, visible, &region, mode, &font, &metrics,
                &contract,
            );
            counts.push(opaque_count(&bytes));
        }
        assert_eq!(counts[0], 0, "可視 0 グリフ＝全透明");
        for i in 1..counts.len() {
            assert!(
                counts[i] > counts[i - 1],
                "非透明ピクセルは可視グリフ数とともに単調増加する: {counts:?}"
            );
        }

        // Clear: 未リビール分含む全消去（空 canvas）＋確定行キャッシュの全破棄。
        executor.clear_cache();
        let bytes = render_items(
            &mut executor, &mut surface, &[], 0, &region, mode, &font, &metrics, &contract,
        );
        assert!(
            bytes.iter().all(|&b| b == 0),
            "Clear 後は全域が透明（premultiplied 全 0）へ戻る"
        );
    }

    /// 全域再描画の構造檻（R7.3）: 可視内容を減らした再描画で以前のインクが残らない
    /// （差分描画なら 5 グリフ分の残渣が出る）。同一入力の再描画はバイト一致（決定論）。
    #[test]
    fn render_is_full_redraw_without_residue() {
        let mut rig = DrawRig::new();
        let image = (120u32, 60u32);
        let mut surface = rig.attach(image, 1.0);
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
        let mode = WritingMode::HorizontalTb;
        let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
            .expect("DWriteMetrics 生成失敗");
        let region = TextRegion::resolve(&geo_model((Some(0), Some(0)), None), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

        let items = glyph_items("あいうえお");
        let two_first = render_items(
            &mut executor, &mut surface, &items, 2, &region, mode, &font, &metrics, &contract,
        );
        let five = render_items(
            &mut executor, &mut surface, &items, 5, &region, mode, &font, &metrics, &contract,
        );
        let two_again = render_items(
            &mut executor, &mut surface, &items, 2, &region, mode, &font, &metrics, &contract,
        );
        assert!(opaque_count(&five) > opaque_count(&two_first));
        assert_eq!(
            two_first, two_again,
            "全域再描画: 5 グリフ描画後に 2 グリフへ戻すと以前の描画とバイト一致（残渣なし）"
        );
    }

    /// 確定行キャッシュの檻（task 6.3・design「確定行は再生成しない・Clear で全破棄」）:
    /// 内容不変の行（確定行）は TextLayout を再生成せず、リビール中の行のみ都度更新。
    /// clear_cache（Clear 適用点）だけが全破棄する。
    #[test]
    fn confirmed_line_layouts_regenerate_only_on_clear() {
        let mut rig = DrawRig::new();
        let image = (120u32, 60u32);
        let mut surface = rig.attach(image, 1.0);
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
        let mode = WritingMode::HorizontalTb;
        let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
            .expect("DWriteMetrics 生成失敗");
        let region = TextRegion::resolve(&geo_model((Some(0), Some(0)), None), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

        // 行 0 確定（"あい"）＋行 1 リビール中（"う"）。
        let mut items = glyph_items("あい");
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.push(TextItem::Glyph { ch: 'う' });

        render_items(
            &mut executor, &mut surface, &items, 3, &region, mode, &font, &metrics, &contract,
        );
        assert_eq!(executor.line_layout_creations(), 2, "初回は 2 行分を生成");

        render_items(
            &mut executor, &mut surface, &items, 3, &region, mode, &font, &metrics, &contract,
        );
        assert_eq!(
            executor.line_layout_creations(),
            2,
            "内容不変の再描画は確定行・現行行とも再生成しない（キャッシュ再利用）"
        );

        // リビール進行: 行 1 が "う"→"うえ" へ——行 1 のみ都度更新（行 0 は確定キャッシュ）。
        items.push(TextItem::Glyph { ch: 'え' });
        render_items(
            &mut executor, &mut surface, &items, 4, &region, mode, &font, &metrics, &contract,
        );
        assert_eq!(
            executor.line_layout_creations(),
            3,
            "リビール中の行のみ再生成（確定行 0 はキャッシュ維持）"
        );

        // Clear 適用点: 全破棄→次描画は全行を再生成する。
        executor.clear_cache();
        render_items(
            &mut executor, &mut surface, &items, 4, &region, mode, &font, &metrics, &contract,
        );
        assert_eq!(
            executor.line_layout_creations(),
            5,
            "Clear のみが確定行キャッシュを全破棄する（再描画で 2 行分を再生成）"
        );
    }

    /// スケール一点適用の構造檻（task 6.3・DPI/スケール契約）: k=2 のインク開始位置は
    /// 画像座標 ×2 の近傍に現れる。二重適用（×4）でも未適用（×1）でもないことを
    /// 範囲判定で排除する（origin (20,20)・正解 ≈40〜・二重 80〜・未適用 ≈20）。
    #[test]
    fn scale_applies_exactly_once_at_draw_target() {
        let mut rig = DrawRig::new();
        let image = (120u32, 60u32);
        let k = 2.0f32;
        let mut surface = rig.attach(image, k);
        assert_eq!(surface.size(), (240, 120), "物理寸 = ceil(image × k)");
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let model = geo_model((Some(20), Some(20)), None);
        let font = ResolvedFont::resolve(&model);
        let mode = WritingMode::HorizontalTb;
        let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
            .expect("DWriteMetrics 生成失敗");
        let region = TextRegion::resolve(&model, image, mode);
        let contract = ScaleContract::new(k, None);
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

        // '■'（全角・インクがほぼ em ボックスを満たす）1 グリフを (20,20) へ。
        let items = glyph_items("■");
        let bytes = render_items(
            &mut executor, &mut surface, &items, 1, &region, mode, &font, &metrics, &contract,
        );
        let (min_x, min_y) = ink_min(&bytes, 240).expect("インクが描かれる");
        // 正解: 画像座標 (20 + ベアリング数 px) × 2 ≈ [40, 40+2×font]。
        // 二重適用なら x ≥ 80・未適用なら x ≈ 20——いずれも範囲外。
        assert!(
            (38..=70).contains(&min_x),
            "インク開始 x={min_x} は一点適用の範囲 [38,70]（未適用≈20・二重適用≥80 を排除）"
        );
        assert!(
            (38..=70).contains(&min_y),
            "インク開始 y={min_y} は一点適用の範囲 [38,70]（未適用≈20・二重適用≥80 を排除）"
        );
    }

    /// スクロール可視窓の描画: あふれ発火後は先頭行が供給面から消え（全域再描画）、
    /// 可視窓の行が領域先頭へ詰めて描かれる（同一入力の再描画はバイト一致・決定論）。
    #[test]
    fn scroll_overflow_drops_oldest_line_via_full_redraw() {
        let mut rig = DrawRig::new();
        let image = (60u32, 40u32);
        let mut surface = rig.attach(image, 1.0);
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        // font 10 → pitch 13・行下端 10/23/36/49——validrect.bottom 40 で 4 行目があふれる。
        let model = geo_model((Some(0), Some(0)), Some(10));
        let font = ResolvedFont::resolve(&model);
        let mode = WritingMode::HorizontalTb;
        let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
            .expect("DWriteMetrics 生成失敗");
        let region = TextRegion::resolve(&model, image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

        // 行 0 = ■■■（3 グリフ）・行 1〜3 = ■ 各 1 グリフ。
        let mut items3 = glyph_items("■■■");
        for _ in 0..2 {
            items3.push(TextItem::LineBreak { ratio: 1.0 });
            items3.push(TextItem::Glyph { ch: '■' });
        }
        let mut items4 = items3.clone();
        items4.push(TextItem::LineBreak { ratio: 1.0 });
        items4.push(TextItem::Glyph { ch: '■' });

        // 3 行（収まる・可視 ■×5）→ 4 行（あふれ・可視窓は行 1〜3 ＝ ■×3）。
        let before = render_items(
            &mut executor, &mut surface, &items3, 5, &region, mode, &font, &metrics, &contract,
        );
        let after = render_items(
            &mut executor, &mut surface, &items4, 6, &region, mode, &font, &metrics, &contract,
        );
        assert!(
            opaque_count(&after) < opaque_count(&before),
            "スクロール後は行 0（■×3）が全域再描画で消える: before={} after={}",
            opaque_count(&before),
            opaque_count(&after)
        );
        let (_, min_y) = ink_min(&after, 60).expect("スクロール後もインクが描かれる");
        assert!(
            min_y < 10,
            "可視窓先頭行（行 1）は block_offset で領域先頭へ詰めて描かれる（min_y={min_y}）"
        );
        // 決定論: 同一入力の再描画はバイト一致（差分累積なし）。
        let again = render_items(
            &mut executor, &mut surface, &items4, 6, &region, mode, &font, &metrics, &contract,
        );
        assert_eq!(after, again, "同一入力→同一ピクセル（全域再描画の決定論）");
    }

    /// R8.5: Image/Surface 住人は M1 型シーム——描画は warn!＋skip（インクなし・panic なし）。
    /// warn は executor ごと初回のみ（ログスパム抑制）。
    #[test]
    fn image_and_surface_seam_residents_warn_and_skip() {
        let mut rig = DrawRig::new();
        let image = (120u32, 60u32);
        let mut surface = rig.attach(image, 1.0);
        let font = ResolvedFont::resolve(&geo_model((Some(0), Some(0)), None));
        let mode = WritingMode::HorizontalTb;
        let contract = ScaleContract::new(1.0, None);
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");

        let canvas = ContentCanvas {
            residents: vec![
                Resident {
                    content: ResidentContent::Image(ImageSeam::default()),
                    transform: RegionTransform::translation(5.0, 5.0),
                    effects: TextEffects::default(),
                },
                Resident {
                    content: ResidentContent::Surface(SurfaceSeam::default()),
                    transform: RegionTransform::identity(),
                    effects: TextEffects::default(),
                },
            ],
            size: (120.0, 60.0),
        };
        let window = VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0,
        };

        let (result, warns, errors) = with_log_cage(|| {
            executor.render(&canvas, &window, &font, mode, &contract, &mut surface)
        });
        result.expect("シーム住人があっても render は成功する（skip 継続）");
        assert!(warns >= 1, "シーム住人の描画要求は warn を記録する");
        assert_eq!(errors, 0);
        let bytes = surface.read_back().expect("read_back 失敗");
        assert!(
            bytes.iter().all(|&b| b == 0),
            "シーム住人は実挙動なし＝インクを一切描かない（R8.5）"
        );

        let (result, warns2, _) = with_log_cage(|| {
            executor.render(&canvas, &window, &font, mode, &contract, &mut surface)
        });
        result.expect("2 回目の render も成功する");
        assert_eq!(warns2, 0, "シーム warn は executor ごと初回のみ（スパム抑制）");
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

    // ── task 6.4 R4.5/R6.1–6.3/R7.5: probe/描画行 TextLayout の送り幅一致 invariant ──
    //
    // design Testing Strategy Integration #5（正典）: 同一 TextFormat・同一テキストで
    // probe layout（DWriteMetrics＝per-char 計測）と描画行 TextLayout
    // （DrawExecutor::line_layout＝render が DrawTextLayout へ渡す実物）の cluster
    // advance が**同値**であることを、等幅（ＭＳ ゴシック）＋プロポーショナル欧文混在
    // （ＭＳ Ｐゴシック）の両方で檻化する。
    //
    // 許容誤差: なし（f32 完全一致）。同一 factory・同一 create_text_format 経路・
    // 同一テキストに対する DirectWrite 計測は決定論であり、design の「同値」が正準
    // ——epsilon を挟むと per-char probe と行文脈（カーニング等）の乖離
    // （6.2 申し送りの検出責務）を握りつぶすため導入しない。

    /// テキストごとの invariant 検証ケース（フォント名 None＝既定 ＭＳ ゴシック）。
    const INVARIANT_CASES: [(Option<&str>, &str); 2] = [
        // 等幅: 既定 ＭＳ ゴシック・全角/半角混在。
        (None, "あiWa。漢！x"),
        // プロポーショナル欧文混在: ＭＳ Ｐゴシック・'i'/'W' 等の可変幅＋全角同居。
        (Some("ＭＳ Ｐゴシック"), "iWMjlあ。W"),
    ];

    /// ケースの ResolvedFont（height 12 固定・color 既定）。
    fn invariant_font(name: Option<&str>) -> ResolvedFont {
        let font = Font::new(
            name.map(str::to_owned),
            Some(12),
            FontColor::new(None, None, None),
        );
        ResolvedFont::resolve(&model_with_font(font))
    }

    /// DrawExecutor の実描画経路（ensure_format→line_layout）で行 TextLayout を組み、
    /// cluster metrics の幅列を返す——render が DrawTextLayout へ渡すのと同一の layout
    /// （検証用の別組みではない）から実描画送り幅を読む。
    fn drawn_line_cluster_widths(
        executor: &mut DrawExecutor,
        text: &str,
        font: &ResolvedFont,
        mode: WritingMode,
    ) -> Vec<f32> {
        let format = executor
            .ensure_format(font, mode)
            .expect("ensure_format 失敗");
        let layout = executor
            .line_layout(0, text, &format, font.height, mode)
            .expect("line_layout 失敗");
        layout
            .get_cluster_metrics()
            .expect("GetClusterMetrics(描画行) 失敗")
            .iter()
            .map(|c| c.width)
            .collect()
    }

    /// 観測可能な完了状態（task 6.4）: 同一フォント設定・同一テキストで、計測専用
    /// probe の per-char advance と描画行 TextLayout の per-cluster advance が完全一致
    /// する（等幅＋プロポーショナル欧文混在 × 3 方向）。プロポーショナルの行文脈調整
    /// （カーニング等）で per-char probe と行計測が乖離するなら、本檻がその文字で
    /// 赤くなる（6.2 申し送りの検出責務・クリップでは隠れない metrics 述語）。
    #[test]
    fn probe_advances_match_drawn_line_cluster_advances() {
        let rig = DrawRig::new();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
        for (name, text) in INVARIANT_CASES {
            let font = invariant_font(name);
            for mode in [
                WritingMode::HorizontalTb,
                WritingMode::VerticalRl,
                WritingMode::VerticalLr,
            ] {
                let metrics =
                    DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
                        .expect("DWriteMetrics 生成失敗");
                let widths = drawn_line_cluster_widths(&mut executor, text, &font, mode);
                assert_eq!(
                    widths.len(),
                    text.chars().count(),
                    "{} {mode:?}: 検証テキストは 1 文字=1 cluster の前提",
                    font.name
                );
                for (ch, width) in text.chars().zip(&widths) {
                    let probe = metrics.advance(ch, font.height);
                    assert!(probe > 0.0, "{} {mode:?} {ch:?}: probe は正値", font.name);
                    assert_eq!(
                        probe, *width,
                        "{} {mode:?} {ch:?}: probe advance（計測専用）と描画行 cluster \
                         advance（実描画）の同値 invariant",
                        font.name
                    );
                }
            }
        }
    }

    /// 乖離の検出形式（task 6.4「クリップで隠さない」）: probe 駆動で折返した各行に
    /// ついて、描画行 TextLayout の送り終端（行内開始＋cluster advance の同順逐次加算
    /// ——LayoutEngine の inline_pos 累積と同じ f32 結合順）が行矩形の行内終端と完全
    /// 一致し、かつ折返し閾値を超えない。probe と実描画の送り幅が乖離すれば、行終端の
    /// 不一致＝**折返し位置のズレ**としてここで赤くなる（供給面クリップに依存しない
    /// metrics 述語——ピクセル述語は 9.2/10.2 の領分）。
    #[test]
    fn advance_divergence_would_surface_as_wrap_position_drift() {
        let rig = DrawRig::new();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let mut executor = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
        for (name, text) in INVARIANT_CASES {
            let font = invariant_font(name);
            for mode in [
                WritingMode::HorizontalTb,
                WritingMode::VerticalRl,
                WritingMode::VerticalLr,
            ] {
                let metrics =
                    DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
                        .expect("DWriteMetrics 生成失敗");
                // 折返し閾値＝先頭 4 文字の probe 送り終端の floor——実測値が閾値と
                // 折返し位置の両方を駆動する形（metrics が違えば折返しもズレる）。
                let cum4: f32 = text
                    .chars()
                    .take(4)
                    .map(|ch| metrics.advance(ch, font.height))
                    .sum();
                let threshold = cum4.floor() as i32;
                assert!(threshold >= 1, "{} {mode:?}: 閾値は正値", font.name);
                // 幾何モデル: 横書き＝origin(0,0)＋wordwrappoint.x・縦書き＝origin 既定
                // （書字開始角）＋wordwrappoint.y（軸読み替え正準表）。
                let (origin, wordwrap) = match mode {
                    WritingMode::HorizontalTb => (
                        Origin::new(Some(0), Some(0)),
                        WordWrapPoint::new(Some(threshold), None),
                    ),
                    WritingMode::VerticalRl | WritingMode::VerticalLr => (
                        Origin::new(None, None),
                        WordWrapPoint::new(None, Some(threshold)),
                    ),
                };
                let model = BalloonModel::new(
                    WindowPosition::new(None, None),
                    origin,
                    wordwrap,
                    ValidRect::new(None, None, None, None),
                    empty_font(),
                    None,
                );
                let region = TextRegion::resolve(&model, (400, 224), mode);
                let items = glyph_items(text);
                let lines =
                    LayoutEngine::layout(&items, items.len(), &region, mode, font.height, &metrics);
                assert!(
                    lines.len() >= 2,
                    "{} {mode:?}: 実測駆動の折返しが実際に発生する構成",
                    font.name
                );
                let placed: usize = lines.iter().map(|l| l.glyphs.len()).sum();
                assert_eq!(placed, text.chars().count(), "折返しでグリフを失わない");
                for (i, line) in lines.iter().enumerate() {
                    let line_text: String = line.glyphs.iter().map(|g| g.ch).collect();
                    let widths = drawn_line_cluster_widths(&mut executor, &line_text, &font, mode);
                    let (inline_start, inline_end) = match mode {
                        WritingMode::HorizontalTb => (line.rect.left, line.rect.right),
                        WritingMode::VerticalRl | WritingMode::VerticalLr => {
                            (line.rect.top, line.rect.bottom)
                        }
                    };
                    // LayoutEngine の inline_pos 累積と同じ逐次加算（f32 結合順まで一致）。
                    let mut drawn_end = inline_start;
                    for w in &widths {
                        drawn_end += *w;
                    }
                    assert_eq!(
                        drawn_end, inline_end,
                        "{} {mode:?} 行 {i} ({line_text:?}): 実描画の送り終端＝計測 \
                         レイアウトの行内終端（不一致＝折返し位置のズレとして検出）",
                        font.name
                    );
                    assert!(
                        inline_end <= threshold as f32 || line.glyphs.len() == 1,
                        "{} {mode:?} 行 {i}: 行終端 {inline_end} は折返し閾値 {threshold} 内 \
                         （行頭 1 グリフ縮退を除く）",
                        font.name
                    );
                }
            }
        }
    }
}
