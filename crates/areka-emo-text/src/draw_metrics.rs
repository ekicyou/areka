//! # draw_metrics — 計測専用 probe layout（COM 層・draw ファサードの子）
//!
//! [`DWriteMetrics`]（測定専用 probe TextLayout 由来の [`GlyphMetrics`] 実装）と、
//! 実 font face metrics からの行ボックス比の実測を担う。親ファサード `draw.rs` からの
//! **純移動**——生成規則・キャッシュ規律・縮退の意図はいずれも移動前と同一。
//!
//! タスク 6.4 で計測は**見た目込み**になった。送り幅に効く見た目（候補列・大きさ・
//! 太字・斜体＝[`FontKey`]）ごとに試験用書式（probe format）を**鍵ごとに 1 度だけ**
//! 焼き、記憶の鍵も「文字と計測鍵」へ広げた。既定の見た目は従来どおり束縛書式の
//! 経路をそのまま通るので、装飾を使わない台本の計測値は 1 ビットも変わらない。
//!
//! **層規律**: COM 層——UI スレッド専有。失敗は log-first（`tracing::error!`＋`Err`）で
//! 扱い panic しない。probe 規約の本文は親ファサードのモジュール doc が正本。
//!
//! 親から `pub use` で再輸出されるため、crate 内から見た入口は
//! `crate::draw::DWriteMetrics` のまま変わらない。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use tracing::warn;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL, IDWriteFactory,
    IDWriteFactory2, IDWriteFontCollection, IDWriteTextFormat,
};
use windows::core::{BOOL, HSTRING, Interface};
use wintf::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};

use super::{
    DEFAULT_FONT_NAME, DirectionRecipe, FontCatalog, LOCALE_JA_JP, PROBE_MAX_EXTENT, ResolvedFont,
    create_text_format, device_err,
};
use crate::TextLayerError;
use crate::layout::GlyphMetrics;
use crate::look::{FontKey, TextLook};
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

/// 計測専用 probe TextLayout 由来の実測 [`GlyphMetrics`]（task 6.2・R4.5・probe 規約）。
///
/// 純粋層 `LayoutEngine` の外部注入点（`&dyn GlyphMetrics`）へ、DirectWrite の実測
/// 送り幅を提供する。probe 規約（モジュール doc）:
///
/// - format は描画と同一の [`create_text_format`] 経路（解決済みフォント＋
///   writing_mode 方向レシピ込み）で**生成時に一度だけ**焼く。
/// - 既定でない見た目は、その計測鍵（[`FontKey`]）ごとの試験用書式を
///   [`probe_format_for`](Self::probe_format_for) が**鍵ごとに 1 度だけ**焼く
///   （家族名は [`FontCatalog::family_for`] が解決した名前か [`DEFAULT_FONT_NAME`]
///   のみ・R11.3）。
/// - `advance` は対象文字の**未折返し probe layout**（[`PROBE_MAX_EXTENT`] 寸）を
///   生成し cluster metrics の width 合計を返す（折返し決定より前の計測＝鶏卵なし）。
/// - 計測値は「文字と計測鍵」の組でキャッシュする（書式が鍵で定まる＝同じ組は同じ
///   送り幅の決定論・probe 規約「確定内容の metrics は不変ゆえキャッシュ可」・R11.1）。
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
    /// 「文字と計測鍵」単位の計測キャッシュ（probe 成功値のみ・失敗は縮退値を返し
    /// キャッシュしない）。既定の見た目は [`default_key`](Self::default_key) の組で入る。
    cache: RefCell<HashMap<(char, FontKey), f32>>,
    /// 候補列の解決台帳（共有・警告の源を 1 つに保つ）。
    fonts: Rc<FontCatalog>,
    /// 束縛書式に焼いた見た目の計測鍵——この鍵の計測は束縛書式の経路をそのまま通る
    /// （装飾を使わない台本の計測値が従来と 1 ビットも変わらない構造的保証）。
    default_key: FontKey,
    /// 書字方向（鍵ごとの試験用書式へ束縛書式と同じ方向レシピを焼くため）。
    mode: WritingMode,
    /// 計測鍵ごとの試験用書式（鍵ごとに 1 度だけ生成して持ち回す）。
    probe_formats: RefCell<HashMap<FontKey, IDWriteTextFormat>>,
    /// 試験用書式を実際に**生成した回数**（記憶に無くて作った回数）。保持庫の要素数ではなく
    /// 生成回数を数える——同じ鍵で作り直しても要素数は増えないので、要素数では
    /// 「鍵ごとに 1 度だけ」を見張れない（実測 2026-09-12）。
    probe_format_creations: Cell<usize>,
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
        let fonts = Rc::new(FontCatalog::new(factory)?);
        DWriteMetrics::new_shared(factory, font, mode, config, fonts)
    }

    /// 候補列の解決台帳を**共有して**計測用 metrics を生成する（要件 9.8）。
    ///
    /// 束縛書式は `fonts.pick(font)` の複製——バルーン定義のカンマ区切り候補列も
    /// 「記述順に最初の実在フォント」で解決される。台帳を共有するのは、同じ候補列の
    /// 全滅の記録が計測側と描画側で二重に出ないようにするため（警告の源が 1 つ）。
    pub fn new_shared(
        factory: &IDWriteFactory2,
        font: &ResolvedFont,
        mode: WritingMode,
        config: &TextLayerConfig,
        fonts: Rc<FontCatalog>,
    ) -> Result<DWriteMetrics, TextLayerError> {
        let picked = fonts.pick(font);
        let format = create_text_format(factory, &picked, mode)?;
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
            fonts,
            default_key: font.looks.default.font_key(),
            mode,
            probe_formats: RefCell::new(HashMap::new()),
            probe_format_creations: Cell::new(0),
        })
    }

    /// 候補列の解決台帳（共有の読み口——描画側が同じ台帳を使うための入口）。
    pub fn fonts(&self) -> &FontCatalog {
        &self.fonts
    }

    /// 計測鍵ごとの試験用書式（**鍵ごとに 1 度だけ**生成して持ち回す）。
    ///
    /// 家族名の**第 2 の入口**——DirectWrite へ渡る名前は
    /// [`FontCatalog::family_for`] が解決した名前か [`DEFAULT_FONT_NAME`] のどちらかだけで、
    /// 別名への差し替え（縦書き異体名など）は存在しない。設定は束縛書式の生成経路
    /// （`create_text_format`／`try_create_format`）と同じ並び——コレクション既定・
    /// stretch NORMAL・locale ja-JP——に、鍵の太字／斜体／大きさを載せた形で、
    /// 最後に同じ [`DirectionRecipe`] を焼く（R11.3「計測と描画は同じ書式経路」）。
    fn probe_format_for(&self, key: &FontKey) -> Result<IDWriteTextFormat, TextLayerError> {
        if let Some(format) = self.probe_formats.borrow().get(key) {
            return Ok(format.clone());
        }
        let family = self
            .fonts
            .family_for(&key.name)
            .unwrap_or_else(|| DEFAULT_FONT_NAME.to_owned());
        let format = self
            .factory
            .create_text_format(
                &HSTRING::from(family.as_str()),
                None::<&IDWriteFontCollection>,
                if key.bold {
                    DWRITE_FONT_WEIGHT_BOLD
                } else {
                    DWRITE_FONT_WEIGHT_NORMAL
                },
                if key.italic {
                    DWRITE_FONT_STYLE_ITALIC
                } else {
                    DWRITE_FONT_STYLE_NORMAL
                },
                DWRITE_FONT_STRETCH_NORMAL,
                f32::from_bits(key.height_bits),
                &HSTRING::from(LOCALE_JA_JP),
            )
            .map_err(device_err("CreateTextFormat(probe)"))?;
        self.probe_format_creations
            .set(self.probe_format_creations.get() + 1);
        DirectionRecipe::for_mode(self.mode).apply(&format)?;
        self.probe_formats
            .borrow_mut()
            .insert(key.clone(), format.clone());
        Ok(format)
    }

    /// 試験用書式を実際に生成した回数（テスト観測用: 同じ鍵の 2 度目が生成を起こさない檻）。
    #[cfg(test)]
    pub(super) fn probe_format_creations(&self) -> usize {
        self.probe_format_creations.get()
    }

    /// 「文字と計測鍵」の組で記憶しながら、指定の書式で 1 文字を測る。
    ///
    /// 記憶に無ければ probe layout を作って測り、成功値だけを入れる（失敗は
    /// [`probe_advance`](Self::probe_advance) 内で `error!` 済み——縮退値は呼び手が
    /// 決めて記憶しない＝次回再試行）。
    fn measure(&self, ch: char, key: &FontKey, format: &IDWriteTextFormat) -> Option<f32> {
        if let Some(&cached) = self.cache.borrow().get(&(ch, key.clone())) {
            return Some(cached);
        }
        let advance = self.probe_advance(ch, format).ok()?;
        self.cache.borrow_mut().insert((ch, key.clone()), advance);
        Some(advance)
    }

    /// 1 文字の未折返し probe layout を指定の書式で生成し、cluster metrics の width 合計を返す。
    fn probe_advance(&self, ch: char, format: &IDWriteTextFormat) -> Result<f32, TextLayerError> {
        let text = HSTRING::from(ch.to_string());
        let layout = self
            .factory
            .create_text_layout(&text, format, PROBE_MAX_EXTENT, PROBE_MAX_EXTENT)
            .map_err(device_err("CreateTextLayout(probe)"))?;
        let clusters = layout
            .get_cluster_metrics()
            .map_err(device_err("GetClusterMetrics(probe)"))?;
        Ok(clusters.iter().map(|c| c.width).sum())
    }

    /// キャッシュ済み計測数（テスト観測用: 同じ「文字と計測鍵」の再計測が probe を
    /// 増やさない檻・鍵が文字だけへ戻れば見た目違いが同じ組に潰れて赤くなる）。
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
        // 失敗は probe_advance 内で error! 済み。縮退値はキャッシュしない（次回再試行）。
        self.measure(ch, &self.default_key, &self.format)
            .unwrap_or_else(|| degraded_advance(ch, self.font_height))
    }

    /// 見た目込みの実測送り幅（R7.10／R11.1）。
    ///
    /// 既定の見た目（束縛書式に焼いた鍵と同値）は [`advance`](Self::advance) の
    /// **同じ経路**をそのまま通る——装飾を使わない台本の計測値が従来と変わらない。
    /// それ以外は鍵ごとの試験用書式で測り「文字と計測鍵」の組で記憶する。
    /// 書式の生成に失敗したときは `error!` 済みの縮退値（`FixedMetrics` と同式・
    /// 基準は**その見た目の**大きさ）を返して継続する。
    fn advance_styled(&self, ch: char, look: &TextLook) -> f32 {
        let key = look.font_key();
        if key == self.default_key {
            return self.advance(ch, look.height);
        }
        self.probe_format_for(&key)
            .ok()
            .and_then(|format| self.measure(ch, &key, &format))
            .unwrap_or_else(|| degraded_advance(ch, look.height))
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

/// 実測できないときの決定論の縮退送り幅（`FixedMetrics` と同式: 全角＝高さ・半角＝高さ半分）。
fn degraded_advance(ch: char, height: f32) -> f32 {
    if ch.is_ascii() { height / 2.0 } else { height }
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

#[cfg(test)]
#[path = "draw_metrics_styled_tests.rs"]
mod styled_tests;
