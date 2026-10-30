//! # draw_metrics — 計測専用 probe layout（COM 層・draw ファサードの子）
//!
//! [`DWriteMetrics`]（測定専用 probe TextLayout 由来の [`GlyphMetrics`] 実装）と、
//! 実 font face metrics からの行ボックス比の実測を担う。親ファサード `draw.rs` からの
//! **純移動**——生成規則・キャッシュ規律・縮退の意図はいずれも移動前と同一。
//!
//! タスク 6.4 で計測は**見た目込み**になった。送り幅に効く見た目（候補列・大きさ・
//! 太字・斜体＝[`FontKey`]）ごとに試験用書式（probe format）を**鍵ごとに 1 度だけ**
//! 焼き、記憶の鍵も「計測鍵と文字」へ広げた。既定の見た目は従来どおり束縛書式の
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
use std::sync::Arc;

use tracing::warn;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_METRICS, DWRITE_LINE_METRICS, DWRITE_TEXT_METRICS, IDWriteFactory, IDWriteFactory2,
    IDWriteFontCollection, IDWriteTextFormat,
};
use windows::core::{BOOL, HSTRING, Interface};
use wintf::com::dwrite::DWriteTextLayoutExt;

use super::{
    DirectionRecipe, FontCatalog, FontFace, ResolvedFont, create_line_layout, create_text_format,
    device_err, try_create_format,
};
use crate::TextLayerError;
use crate::layout::{GlyphMetrics, column_centred_em_box_left};
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
///   （顔は鍵の候補列の顔か、全滅なら束縛の顔のみ・R11.3）。
/// - `advance` は対象文字（書記素クラスタ 1 つ）全体の**未折返し probe layout**
///   （行と同じ生成口 [`create_line_layout`]・行内軸 [`PROBE_MAX_EXTENT`](super::PROBE_MAX_EXTENT) 寸）を生成し
///   cluster metrics の width 合計を返す（折返し決定より
///   前の計測＝鶏卵なし。DirectWrite が 2 つに整形するクラスタ＝国旗などは合計が行の中で
///   占める幅になる）。
/// - 計測値は「計測鍵 → 文字（クラスタ文字列）」の 2 段でキャッシュする（書式が鍵で定まる＝
///   同じ組は同じ送り幅の決定論・probe 規約「確定内容の metrics は不変ゆえキャッシュ可」・
///   R11.1）。引くときは `&str` のまま引いて割り当てない。
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
    /// 縦書きの em の箱の左の比（`(left + baseline − h/2) ÷ h`・生成時に一度だけ実測）。
    /// 横書きに束縛した物は `None`（既定の実装）。縦書きで測れなかった物は 0（行の矩形の真ん中）。
    vertical_em_box_left_ratio: Option<f32>,
    /// 「計測鍵 → 文字（クラスタ文字列）→ 送り幅」の 2 段の計測キャッシュ（probe 成功値のみ・
    /// 失敗は縮退値を返しキャッシュしない）。既定の見た目は [`default_key`](Self::default_key)
    /// の組で入る。内側の鍵を `Arc<str>` にして `&str` で引く（引くときに割り当てない）。
    cache: RefCell<HashMap<FontKey, HashMap<Arc<str>, f32>>>,
    /// 候補列の解決台帳（共有・警告の源を 1 つに保つ）。
    fonts: Rc<FontCatalog>,
    /// 束縛書式の顔（`fonts.pick(font)` の解決）——全滅の計測鍵はこの顔で測る（要件 4.10）。
    bound_face: FontFace,
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
    ///
    /// **硬い経路が 1 つ増えている**（タスク 9.4 の記録・6.2 由来）——候補列の解決台帳を
    /// 作るために `GetSystemFontCollection` を通すので、同 API の失敗が
    /// **計測の縮退ではなく装着そのものの失敗**（`Err(TextLayerError::Device)`）になる。
    /// 以前は書式生成の失敗が行ボックス比の縮退で済んでいた。失敗は
    /// `FontCatalog::new` が `error!` で記録するので記録なしの失敗経路ではない。
    ///
    /// **本番の呼び手は [`new_shared`](Self::new_shared) へ移った**（`actor_decoration.rs::build_actor_render`
    /// が台帳を 1 つだけ作って描画器と共有する・要件 9.8）。本関数の本番の呼び手は 0 で、
    /// 残しているのは自前の台帳で足りる決定論テストが呼ぶ委譲の入口としてである
    /// （`draw_test_support.rs`／`draw_oracle_tests.rs`／`draw_format_metrics_tests.rs`／
    /// `actor_choice_contract_tests.rs`／`viewbox_draw_live_diff_tests.rs`／
    /// `viewbox_draw_oracle_regression_tests.rs`／`viewbox_draw_png_dump_tests.rs`、および
    /// 統合試験 `tests/kero_menu_capacity_test.rs`／`tests/line_pitch_readback_test.rs`）。
    /// crate 外では
    /// `crates/areka-emo-text/examples/emo-text-layer/drive.rs` が 1 か所で呼ぶ（本番ではない）。
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
        // 縦書きの em の箱の置き場も、行ボックス比と同じく生成時に 1 度だけ測る（書体の寸法は不変）。
        // 測れないときは 0（行の矩形の真ん中＝列のレシピが em の箱を置く場所）へ縮退する。
        let vertical_em_box_left_ratio = match mode {
            WritingMode::HorizontalTb => None,
            WritingMode::VerticalRl | WritingMode::VerticalLr => Some(
                measure_vertical_em_box_left_ratio(factory, &format, font.height, mode)
                    .unwrap_or_else(|step| {
                        warn!(
                            font = %picked.name,
                            ?mode,
                            step,
                            "縦書きの列の基線を測れない——em の箱を行の矩形の真ん中へ縮退する"
                        );
                        0.0
                    }),
            ),
        };
        Ok(DWriteMetrics {
            factory: factory.clone(),
            format,
            font_height: font.height,
            config: *config,
            line_box_ratio,
            vertical_em_box_left_ratio,
            cache: RefCell::new(HashMap::new()),
            fonts,
            bound_face: picked.face,
            default_key: font.looks.default.font_key(),
            mode,
            probe_formats: RefCell::new(HashMap::new()),
            probe_format_creations: Cell::new(0),
        })
    }

    /// 受け取った解決台帳の**観測口**（テスト専用・本番の共有経路ではない）。
    ///
    /// 描画側との共有はこの読み口を経由しない——`actor_decoration.rs::build_actor_render` が
    /// [`FontCatalog`] を 1 つだけ作り、[`DWriteMetrics::new_shared`] と
    /// [`ViewboxExecutor::new_shared`](crate::viewbox_draw::ViewboxExecutor::new_shared) の
    /// **両方へ引数で渡す**形で共有が成立する（要件 9.8）。ゆえに本関数の本番の呼び手は 0 で、
    /// 唯一の呼び手は `draw_metrics_styled_tests.rs::the_shared_catalog_constructor_measures_the_same_as_the_owning_one`
    /// ——「渡した台帳がそのまま使われている」（`Rc` の同一性）を確かめるための口である。
    /// 本番の入口ではないことを構造で示すため [`probe_format_creations`](Self::probe_format_creations)
    /// と同じく `#[cfg(test)]` で塞いである。
    #[cfg(test)]
    pub(super) fn fonts(&self) -> &FontCatalog {
        &self.fonts
    }

    /// 計測鍵ごとの試験用書式（**鍵ごとに 1 度だけ**生成して持ち回す）。
    ///
    /// 顔は「鍵の候補列の顔、全滅なら束縛の顔」（[`FontCatalog::face_or`]）——描画の範囲指定
    /// （`apply_font_ranges`）と同じ規則なので、全滅の区間も描画と同じ顔で測る（要件 4.10・
    /// 完了 spec の「全滅は ＭＳ ゴシックで測る」を上書き）。書式は束縛書式と共用の
    /// `draw.rs::try_create_format` に顔と鍵の太字／斜体／大きさを渡して作り、最後に同じ
    /// [`DirectionRecipe`] を焼く（要件 4.1「計測と描画は同じ書式経路」・設定の写しを持たない）。
    /// 計測鍵が顔を一意に決めるので、記憶の鍵は計測鍵のまま。
    fn probe_format_for(&self, key: &FontKey) -> Result<IDWriteTextFormat, TextLayerError> {
        if let Some(format) = self.probe_formats.borrow().get(key) {
            return Ok(format.clone());
        }
        let face = self.fonts.face_or(&key.name, &self.bound_face);
        let format = try_create_format(
            &self.factory,
            &face,
            key.bold,
            key.italic,
            f32::from_bits(key.height_bits),
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

    /// 「計測鍵と文字」の組で記憶しながら、指定の書式で 1 文字（クラスタ 1 つ）を測る。
    ///
    /// 記憶に無ければ probe layout を作って測り、成功値だけを入れる（失敗は
    /// [`probe_advance`](Self::probe_advance) 内で `error!` 済み——縮退値は呼び手が
    /// 決めて記憶しない＝次回再試行）。引くときは割り当てず、入れるときだけ鍵を作る。
    fn measure(&self, text: &str, key: &FontKey, format: &IDWriteTextFormat) -> Option<f32> {
        let cached = self
            .cache
            .borrow()
            .get(key)
            .and_then(|by_text| by_text.get(text).copied());
        if let Some(cached) = cached {
            return Some(cached);
        }
        let advance = self.probe_advance(text, format).ok()?;
        self.cache
            .borrow_mut()
            .entry(key.clone())
            .or_default()
            .insert(Arc::from(text), advance);
        Some(advance)
    }

    /// 1 文字（クラスタ 1 つ）全体の未折返し probe layout を指定の書式で、行と同じ生成口
    /// （[`create_line_layout`]・縦書きは列のレシピ）で生成し、cluster metrics の width 合計を返す
    /// （要件 2.4: 計測と行は同じ設定）。行送り軸の箱は書式の大きさ。
    fn probe_advance(&self, text: &str, format: &IDWriteTextFormat) -> Result<f32, TextLayerError> {
        let block_extent = unsafe { format.GetFontSize() };
        let layout = create_line_layout(&self.factory, text, format, block_extent, self.mode)?;
        let clusters = layout
            .get_cluster_metrics()
            .map_err(device_err("GetClusterMetrics(probe)"))?;
        Ok(clusters.iter().map(|c| c.width).sum())
    }

    /// キャッシュ済み計測数（内側の表の要素数の合計。テスト観測用: 同じ「計測鍵と文字」の
    /// 再計測が probe を増やさない檻・鍵が文字だけへ戻れば見た目違いが同じ組に潰れて赤くなる）。
    ///
    /// `pub(super)`: ファサード配下の兄弟テスト（`draw_format_metrics_tests.rs`＝
    /// `crate::draw` の子）から見える最小の可視性。crate 外へは出さない。
    #[cfg(test)]
    pub(super) fn cached_probe_count(&self) -> usize {
        self.cache.borrow().values().map(HashMap::len).sum()
    }
}

impl GlyphMetrics for DWriteMetrics {
    /// 実測送り幅（image px＝format の DIP そのまま・writing_mode の行内軸方向の寸）。
    ///
    /// `font_height` は束縛フォント（format へ焼き込み済み）と一致していることが契約。
    /// 不一致は `warn!`＋縮退継続（値は束縛 format の実測のまま——probe は描画と同一
    /// format が正準のため引数側へ寄せない）。probe 失敗は `error!`（[`device_err`]）
    /// 済みで、決定論の縮退値（`FixedMetrics` と同式: ASCII だけのクラスタ＝height/2・
    /// それ以外＝height）を返して継続する（trait は失敗経路を持たない・log-first でログ無し
    /// 失敗にしない）。
    fn advance(&self, text: &str, font_height: f32) -> f32 {
        if font_height != self.font_height {
            warn!(
                requested = font_height,
                bound = self.font_height,
                "advance へ束縛フォントと異なる font_height が渡された——束縛 format の実測を返す"
            );
        }
        // 失敗は probe_advance 内で error! 済み。縮退値はキャッシュしない（次回再試行）。
        self.measure(text, &self.default_key, &self.format)
            .unwrap_or_else(|| degraded_advance(text, self.font_height))
    }

    /// 見た目込みの実測送り幅（R7.10／R11.1）。
    ///
    /// 既定の見た目（束縛書式に焼いた鍵と同値）は [`advance`](Self::advance) の
    /// **同じ経路**をそのまま通る——装飾を使わない台本の計測値が従来と変わらない。
    /// それ以外は鍵ごとの試験用書式で測り「計測鍵と文字」の組で記憶する。
    /// 書式の生成に失敗したときは `error!` 済みの縮退値（`FixedMetrics` と同式・
    /// 基準は**その見た目の**大きさ）を返して継続する。
    fn advance_styled(&self, text: &str, look: &TextLook) -> f32 {
        let key = look.font_key();
        if key == self.default_key {
            return self.advance(text, look.height);
        }
        self.probe_format_for(&key)
            .ok()
            .and_then(|format| self.measure(text, &key, &format))
            .unwrap_or_else(|| degraded_advance(text, look.height))
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

    /// 縦書きの em の箱の左＝生成時に実測した比 × `font_height`（問われた向きが束縛した縦書きと
    /// 同じとき）。縦書きに束縛した物へ別の向き（横書きを含む）を問うたときは 0（列のレシピは
    /// どちらの向きでも em の箱を行の矩形の真ん中に置く）。横書きに束縛した物は既定の実装。
    ///
    /// 列のレシピ（行間 UNIFORM・基線は箱の幅の半分）の下の実測は Yu Gothic UI・ＭＳ ゴシック・
    /// 游ゴシックの 2 方向とも 0。列のレシピの前の値（Yu Gothic UI ∓0.1650・游ゴシック −0.3013）は
    /// spec `emo-text-dwrite-fidelity` research.md §10.3。
    fn vertical_em_box_left(&self, font_height: f32, mode: WritingMode) -> f32 {
        match self.vertical_em_box_left_ratio {
            Some(ratio) if mode == self.mode => ratio * font_height,
            Some(_) => 0.0,
            None => {
                column_centred_em_box_left(self.line_box_height(font_height), font_height, mode)
            }
        }
    }
}

/// 実測できないときの決定論の縮退送り幅（`FixedMetrics` と同式: ASCII だけのクラスタ＝
/// 高さ半分・それ以外＝高さ 1 つ分）。
fn degraded_advance(text: &str, height: f32) -> f32 {
    if text.is_ascii() {
        height / 2.0
    } else {
        height
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

/// 束縛した書式で、縦書きの em の箱の左の比 `(x − h/2) ÷ h` を測る。
///
/// x は列の基線の行矩形の左辺からの位置で、本番の行と同じ書式・同じ生成口（[`create_line_layout`]・
/// 列のレシピ）・同じ幅 h の仮名 1 字（「あ」）の TextLayout の
/// `GetMetrics().left + GetLineMetrics()[0].baseline`（式 A）。列のレシピは基線を箱の幅の半分に
/// 置くので、実測は書体によらず 0（research.md §10.3）。
/// 失敗は失敗した所の名前を `Err` で返す（生成口の失敗は `error!` 済み・呼び手が `warn!` に載せて縮退）。
fn measure_vertical_em_box_left_ratio(
    factory: &IDWriteFactory2,
    format: &IDWriteTextFormat,
    font_height: f32,
    mode: WritingMode,
) -> Result<f32, &'static str> {
    if font_height <= 0.0 {
        return Err("font_height <= 0");
    }
    let layout = create_line_layout(factory, "あ", format, font_height, mode)
        .map_err(|_| "create_line_layout")?;
    let mut text = DWRITE_TEXT_METRICS::default();
    unsafe { layout.GetMetrics(&mut text) }.map_err(|_| "GetMetrics")?;
    let mut lines = [DWRITE_LINE_METRICS::default(); 4];
    let mut count = 0;
    unsafe { layout.GetLineMetrics(Some(&mut lines), &mut count) }.map_err(|_| "GetLineMetrics")?;
    if count == 0 {
        return Err("GetLineMetrics: 行が 0 本");
    }
    let ratio = (text.left + lines[0].baseline - font_height / 2.0) / font_height;
    if ratio.is_finite() {
        Ok(ratio)
    } else {
        Err("値が有限でない")
    }
}

#[cfg(test)]
#[path = "draw_metrics_styled_tests.rs"]
mod styled_tests;
