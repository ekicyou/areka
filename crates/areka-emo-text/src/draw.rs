//! # draw — DirectWrite/D2D 描画実行（COM 層・ファサード）
//!
//! `DrawExecutor`（可視窓の全域再描画・比較専用オラクル）とフォント解決・縦書きレシピの
//! lift・書式生成を担う。役割ごとに 2 つの子モジュール（兄弟ファイル）へ分かれる:
//!
//! | ファイル | 担当 |
//! |---|---|
//! | `draw.rs`（本ファイル） | 既定値・[`ResolvedFont`]・[`DirectionRecipe`]・書式生成・D2D ターゲット bitmap・比較専用オラクル |
//! | `draw_metrics.rs` | [`DWriteMetrics`]（計測専用 probe layout・行ボックス比の実測） |
//! | `draw_line_store.rs` | [`LineLayoutStore`]（行 TextLayout の生成・キャッシュ・インクはみ出しの実測） |
//!
//! 子は `super::` でファサードの定数・型・ヘルパを辿り、ファサードが再輸出するので
//! crate 内から見た入口（`crate::draw::DWriteMetrics` 等）は分割前と同一。
//!
//! **層規律**: COM 層——UI スレッド専有。`windows`（DirectWrite/D2D）を触るのは
//! 本モジュール群と surface のみ。失敗は log-first（`tracing::error!`＋`Err`）で扱い panic しない。
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
//! - 無効表示の層（`disable.font.*`）は [`ResolvedFont::looks`] で**実体化済み**
//!   （要件 4.4——`\f[disable]` の戻し先）。行単位の文字装飾（[`TextEffects`]）だけが
//!   **型シームのみ・実挙動なし**のまま残る（M2 予約）。
//! - 縦書きの寄せと下線の写像は確定済み（spec `areka-P0-balloon-vertical-canon`
//!   要件 5.1〜5.3・5.7：`align` は `left`＝上寄せ／`right`＝下寄せ／`center`＝縦中央、
//!   `valign` は `top`＝右寄せ／`bottom`＝左寄せ、下線は列の右側）。正典 2 ページで
//!   `valign` の写像が逆である事実（疑義 SC1）と areka が採る側の理由は
//!   `doc/COMPAT_ARCHITECTURE.md` §8 の該当行が正本で、寄せの追跡先は
//!   `areka-P0-text-align-shadow-canon`。
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
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FLOW_DIRECTION, DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
    DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_PARAGRAPH_ALIGNMENT, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_READING_DIRECTION,
    DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
    DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_LEADING, IDWriteFactory2, IDWriteFontCollection,
    IDWriteTextFormat,
};
// 行 TextLayout そのものを扱うのは子モジュール line_store と、比較専用オラクル
// [`DrawExecutor`]（`#[cfg(test)]`）だけ——非テストビルドの dead import を避けるべく
// cfg(test) へ隔離する。
#[cfg(test)]
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::core::{HSTRING, Interface};
use wintf::com::dwrite::DWriteFactoryExt;

use crate::TextLayerError;
use crate::canvas::TextEffects;
use crate::choice::ResolvedChoiceStyle;
use crate::look::LookLayers;
use crate::writing::WritingMode;

// 以下は比較専用オラクル [`DrawExecutor`]（`#[cfg(test)]`）だけが使う依存——本番経路
// （ViewboxExecutor）は viewbox_draw.rs 側で自前に持つため、非テストビルドの dead import を
// 避けるべく cfg(test) へ隔離する（オラクル隔離の一部・task 5）。
#[cfg(test)]
use crate::canvas::{ContentCanvas, ResidentContent};
#[cfg(test)]
use crate::layout::VisibleWindow;
#[cfg(test)]
use crate::region::ScaleContract;
#[cfg(test)]
use crate::surface::TextSurface;
#[cfg(test)]
use windows_numerics::{Matrix3x2, Vector2};
#[cfg(test)]
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
#[cfg(test)]
use wintf::ecs::GraphicsCore;

// ---------------------------------------------------------------------------
// 子モジュール（役割ごとの兄弟ファイル）と再輸出
// ---------------------------------------------------------------------------
//
// 本ファイルはファサードで、計測（`draw_metrics.rs`）と行レイアウトの記憶
// （`draw_line_store.rs`）は子モジュールが持つ。子は `super::` で本ファイルの
// 定数・型・ヘルパを辿る。再輸出により crate 内から見た入口（`crate::draw::DWriteMetrics`
// ／`crate::draw::LineLayoutStore`）は分割前と同一のまま。

#[path = "draw_metrics.rs"]
mod metrics;

#[path = "draw_line_store.rs"]
mod line_store;

pub(crate) use line_store::LineLayoutStore;
pub use metrics::DWriteMetrics;

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

/// バルーン背景色の既定（白）——無効表示の混色の相手がまだ分からないときに使う
/// （要件 4.6）。実の背景（バルーン画像の原点画素）を知っている呼び手は
/// [`ResolvedFont::resolve_with_background`] へそれを渡す。
pub const DEFAULT_BALLOON_BACKGROUND: (u8, u8, u8) = (255, 255, 255);

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
    /// 行単位の文字装飾の M2 予約シーム（実挙動なし）。
    pub effects: TextEffects,
    /// 既定／無効表示の 2 層＋選択肢文字色（要件 4.1／4.4／4.5）。
    ///
    /// `\f[default]`／`\f[disable]` の戻し先で、本型が唯一の構築点。
    /// `looks.default` の `name`（＝[`name`](Self::name) ＋ [`fallback_chain`](Self::fallback_chain)）・
    /// `height`・`color` が同名のフィールドと一致することは、構築が
    /// [`resolve_with_background`](Self::resolve_with_background) の 1 か所だけであることが保証する。
    pub looks: LookLayers,
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
        ResolvedFont::resolve_with_background(model, DEFAULT_BALLOON_BACKGROUND)
    }

    /// [`resolve`](Self::resolve) にバルーンの**背景色**を与えた形（要件 4.4／4.5／4.6）。
    ///
    /// 背景色は無効表示の色を導くためだけに使う（混色の式は
    /// [`crate::color::mix_disabled`] が唯一の実装点）。背景を知らない呼び手は
    /// [`resolve`](Self::resolve) を使い、[`DEFAULT_BALLOON_BACKGROUND`]（白）が採られる。
    ///
    /// バルーン定義からまだ読めない 8 キー（`font.bold` ほか・要件 4.3／4.7）の口は
    /// [`LookLayers::from_balloon`] の引数列で、読めるようになったときはここで
    /// `model.font()` から読んで渡すだけで効く（読み取りの所有は
    /// `areka-P0-balloon-font-descript-keys`）。
    pub fn resolve_with_background(model: &BalloonModel, background: (u8, u8, u8)) -> ResolvedFont {
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

        // 選択肢の既定文字色は既存の選択肢表示の解決結果から取る——マーカー無し
        // （`cursor.style,none`）は文字色を定めないので既定の文字色。
        let cursor_text = ResolvedChoiceStyle::resolve(Some(model.cursor()), color)
            .paint(color)
            .map_or(color, |(_fill, text)| text);

        let mut candidates = Vec::with_capacity(1 + fallback_chain.len());
        candidates.push(name.clone());
        candidates.extend(fallback_chain.iter().cloned());

        ResolvedFont {
            name,
            fallback_chain,
            height,
            color,
            effects: TextEffects::default(),
            looks: LookLayers::from_balloon(candidates, height, color, background, cursor_text),
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
                // Choice 住人は内包 run を GlyphRun と同一の素描画で描く（比較オラクルは
                // ハイライトを持たない＝byte 等価 golden が不変に保たれる・R9.5）。
                ResidentContent::Choice(choice) => &choice.run,
                seam @ (ResidentContent::Image(_) | ResidentContent::Surface(_)) => {
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
    unsafe { dc.CreateBitmapFromDxgiSurface(&dxgi_surface, Some(&props as *const _)) }
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
    tracing::error!(
        context,
        "必須リソースが欠落（デバイス未初期化 または 前提不成立）"
    );
    TextLayerError::Device {
        hresult: 0,
        context,
    }
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
#[path = "draw_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "draw_format_metrics_tests.rs"]
mod format_metrics_tests;

#[cfg(test)]
#[path = "draw_oracle_tests.rs"]
mod oracle_tests;
