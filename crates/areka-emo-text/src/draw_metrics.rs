//! # draw_metrics — 計測専用 probe layout（COM 層・draw ファサードの子）
//!
//! [`DWriteMetrics`]（測定専用 probe TextLayout 由来の [`GlyphMetrics`] 実装）と、
//! 実 font face metrics からの行ボックス比の実測を担う。親ファサード `draw.rs` からの
//! **純移動**——生成規則・キャッシュ規律・縮退の意図はいずれも移動前と同一。
//!
//! **層規律**: COM 層——UI スレッド専有。失敗は log-first（`tracing::error!`＋`Err`）で
//! 扱い panic しない。probe 規約の本文は親ファサードのモジュール doc が正本。
//!
//! 親から `pub use` で再輸出されるため、crate 内から見た入口は
//! `crate::draw::DWriteMetrics` のまま変わらない。

use std::cell::RefCell;
use std::collections::HashMap;

use tracing::warn;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_METRICS, IDWriteFactory, IDWriteFactory2, IDWriteFontCollection, IDWriteTextFormat,
};
use windows::core::{BOOL, HSTRING, Interface};
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};

use super::{PROBE_MAX_EXTENT, ResolvedFont, create_text_format, device_err};
use crate::TextLayerError;
use crate::layout::GlyphMetrics;
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

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
/// `font_height + 行間`（正本 [`TextLayerConfig::line_pitch`]・`FixedMetrics` と同一式）
/// に従う——足し算は自分で持たず config へ委譲する。
pub struct DWriteMetrics {
    /// probe layout 生成用 factory（描画と同じ `IDWriteFactory2`）。
    factory: IDWriteFactory2,
    /// 描画と同一経路で生成済みの計測用 format（フォント・サイズ・方向レシピ込み）。
    format: IDWriteTextFormat,
    /// 束縛フォント高さ（`ResolvedFont::height`・format へ焼き込み済みの正本）。
    font_height: f32,
    /// 行送りの調整値（行間の正本 [`TextLayerConfig`]・`line_pitch` の委譲先）。
    config: TextLayerConfig,
    /// 実 font face metrics 由来の行ボックス比 `(ascent + descent) ÷ designUnitsPerEm`
    /// （生成時に一度だけ実測・文字列非依存＝フォント固有の設計値）。
    line_box_ratio: f32,
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
        // 行ボックス比は **format が実際に束縛したフォント**の face metrics から実測する
        // （既定フォント再試行後でも format 側から辿るため取り違えが起きない）。取得失敗は
        // warn＋行送りピッチと同丈の比（`line_pitch(h) / h`）へ縮退する（帯はピッチで
        // 頭打ちゆえ縮退値でも隣接行を侵さない・R3.10・現行の縮退の意図を新式のまま保つ）。
        let line_box_ratio = measure_line_box_ratio(factory, &format).unwrap_or_else(|| {
            let fallback_ratio = if font.height > 0.0 {
                config.line_pitch(font.height) / font.height
            } else {
                1.0
            };
            warn!(
                font = %font.name,
                line_gap = config.line_gap,
                fallback = fallback_ratio,
                "font face metrics を取得できない——行ボックス比を行送りピッチ相当へ縮退する"
            );
            fallback_ratio
        });
        Ok(DWriteMetrics {
            factory: factory.clone(),
            format,
            font_height: font.height,
            config: *config,
            line_box_ratio,
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
    ///
    /// `pub(super)`: ファサード配下の兄弟テスト（`draw_format_metrics_tests.rs`＝
    /// `crate::draw` の子）から見える最小の可視性。crate 外へは出さない。
    #[cfg(test)]
    pub(super) fn cached_probe_count(&self) -> usize {
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

    /// 行送りピッチ＝正典式 `font_height + 行間`。式は [`TextLayerConfig::line_pitch`]
    /// が唯一の定義点で、ここは委譲するだけ（自前の足し算を持たない・R3.5）。
    fn line_pitch(&self, font_height: f32) -> f32 {
        self.config.line_pitch(font_height)
    }

    /// 実レンダリング行ボックス丈＝`font_height × (ascent + descent) ÷ designUnitsPerEm`
    /// （生成時に実測した [`line_box_ratio`](Self::line_box_ratio) を掛けるだけ・文字列非依存）。
    ///
    /// 実測例: Yu Gothic UI ＝ upem 2048・ascent 2210・descent 514 → 比 1.3301
    /// （28px で 37.24px＝em ボックス 28px より 9.24px 高い）／ＭＳ ゴシック ＝ upem 256・
    /// ascent 220・descent 36 → 比ちょうど 1.0（既定フォントでは em ボックスと一致するため
    /// **既定フォントだけを見ていると descent はみ出しが観測されない**——記憶
    /// emo-text-byte-equiv-default-font-blindspot の系）。
    fn line_box_height(&self, font_height: f32) -> f32 {
        font_height * self.line_box_ratio
    }
}

/// format が束縛したフォントの face metrics から行ボックス比 `(ascent + descent) ÷ upem` を実測する。
///
/// format 自身が持つ family 名・フォント コレクション・weight/style/stretch を辿るため、
/// [`create_text_format`] の既定フォント再試行（R4.2）後でも**実際に描画されるフォント**を測る。
/// 取得経路のいずれかが失敗・不在（family 未発見・upem 0 等）なら `None`（呼び手が縮退）。
fn measure_line_box_ratio(factory: &IDWriteFactory2, format: &IDWriteTextFormat) -> Option<f32> {
    // family 名（format 焼込値）。
    let len = unsafe { format.GetFontFamilyNameLength() } as usize;
    let mut name = vec![0u16; len + 1];
    unsafe { format.GetFontFamilyName(&mut name) }.ok()?;
    let name = HSTRING::from_wide(&name[..len]);
    // フォント コレクション（format が持たなければシステム コレクション）。
    let collection: IDWriteFontCollection = match unsafe { format.GetFontCollection() } {
        Ok(c) => c,
        Err(_) => {
            let base: IDWriteFactory = factory.cast().ok()?;
            let mut system: Option<IDWriteFontCollection> = None;
            unsafe { base.GetSystemFontCollection(&mut system, false) }.ok()?;
            system?
        }
    };
    let mut index = 0u32;
    let mut exists = BOOL(0);
    unsafe { collection.FindFamilyName(&name, &mut index, &mut exists) }.ok()?;
    if !exists.as_bool() {
        return None;
    }
    let family = unsafe { collection.GetFontFamily(index) }.ok()?;
    let font = unsafe {
        family.GetFirstMatchingFont(
            format.GetFontWeight(),
            format.GetFontStretch(),
            format.GetFontStyle(),
        )
    }
    .ok()?;
    let face = unsafe { font.CreateFontFace() }.ok()?;
    let mut metrics = DWRITE_FONT_METRICS::default();
    unsafe { face.GetMetrics(&mut metrics) };
    let upem = metrics.designUnitsPerEm as f32;
    if upem <= 0.0 {
        return None;
    }
    Some((metrics.ascent as f32 + metrics.descent as f32) / upem)
}
