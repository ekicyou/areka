//! # layout — 折返し・行送り・スクロール可視窓の決定（純粋層）
//!
//! `GlyphMetrics` trait（グリフ送り幅・行送りピッチの唯一の注入口・R4.5）を通じて
//! metrics 依存を外部化し、折返し位置・行送り・あふれ判定・可視窓決定の
//! アルゴリズム自体は描画方式に依存しない純粋な形に保つ `LayoutEngine`／
//! `FixedMetrics`／`PositionedLine`／`VisibleWindow` を担う。
//! [`LayoutEngine::visible_window`] は「可視窓の決定（純粋）」だけを返し、
//! 描画実行（全域再描画・R7.3）は COM 層（draw）の領分——R7.4 の分離シーム。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 実測 metrics（DWriteMetrics・probe TextLayout 由来）は COM 層（draw）が
//! [`GlyphMetrics`] を実装して注入する。
//!
//! ## 軸読み替え（design.md「軸読み替え正準表」R6.1–6.3）
//!
//! 3 方向は**単一の読み替え規則**で扱う——回るのは軸の役割だけで、
//! アルゴリズム分岐は存在しない:
//!
//! | 項目 | horizontal_tb | vertical_rl | vertical_lr |
//! |---|---|---|---|
//! | 行内軸（文字が進む） | +x | +y | +y |
//! | 行送り軸（行が進む） | +y | −x | +x |
//! | 折返し判定 | 行内位置＋次グリフ幅 > 閾値（3 方向共通・行内軸は常に正方向） | 同 | 同 |
//!
//! 折返し閾値・描画開始点は [`TextRegion`] が解決済みの絶対値（image px）。
//!
//! ## 行内開始位置の規則（design 無言域の実装正準）
//!
//! 折返し・改行後の行は、描画開始点（宣言 origin は字義・未宣言は書字開始角）の**行内軸成分**へ戻る
//! （全行で同一の行内開始＝単一規則。行送り軸成分だけが行ごとに進む）。
//!
//! ## 可視 prefix 規則（typewriter との接続）
//!
//! `layout` は追記順 items の先頭から「`visible_count`+1 個目のグリフ」直前までを
//! 配置対象とする。リビール時刻の解決（`visible_glyphs(actor, t)`）は state 層の
//! 領分で、本層は個数だけを受け取る。
//!
//! ## 改行の遅延（deferred newline・SSP 準拠・areka-P0-newline-defer）
//!
//! 改行マーカー（`NewLine{ratio}`）は「文字書き込み位置を次行先頭へ動かす予約
//! （reservation）」であり、**到着即時には行を送らない**。走査ローカルの保留
//! （`pending: Option<f32>`＝Σratio）へ ratio を累算し（連続改行は単一累算）、
//! **次の可視グリフが実際に配置される直前にのみ一括実体化**する（累算送り
//! `pitch × Σratio` を block 位置へ適用）。保留のみでは行を開かず・空行を
//! [`PositionedLine`] として出さず・内容ビューボックスを変えない（ビューボックスは
//! 実際に置いた可視コンテンツだけが決める・content 種別非依存）。可視 prefix 末尾より
//! 後ろ・後続可視グリフを持たない末尾改行は**保留のまま蒸発**する（走査終了・打切りで
//! 単に捨てられる＝R5.2/5.3）。この規則は 3 方向（横書き／縦書き rl・lr）で同一
//! （前進量が軸読み替え式に乗るだけ・アルゴリズム分岐なし）。
//!
//! ## 行矩形の規約（R9.4 の再利用シーム）
//!
//! [`PositionedLine::rect`] は image px の絶対矩形。行内軸範囲＝行内開始〜最終グリフ
//! 送り終端（空行は零幅）・行送り軸範囲＝行位置から `font_height` 分（horizontal_tb
//! は下方向・vertical_rl は左方向・vertical_lr は右方向＝行送り方向と同符号）。
//! グリフ別の行内位置＋送り幅と併せ、choice-render のクリック可能範囲導出が
//! そのまま再利用できる（導出自体は実装しない・R9.4）。

use std::collections::BTreeSet;

use areka_sakura::contract::ActorKey;

use crate::region::TextRegion;
use crate::segment::SegmentPlan;
use crate::state::{CursorCoord, CursorUnit, TextItem, TextLayerConfig};
use crate::writing::WritingMode;

/// グリフ送りの注入点（metrics 依存の唯一の口・R4.5）。
///
/// 「グリフ送り幅・行送りピッチ」だけを注入し、折返し位置・行送りの決定
/// アルゴリズム自体は純粋に保つ分離線の正準。構造テストは [`FixedMetrics`]、
/// 実行時は COM 層の DWriteMetrics（測定専用 probe TextLayout 由来）を注入する。
/// 両者で折返し位置は異なってよいが、アルゴリズム分岐は存在しない。
pub trait GlyphMetrics {
    /// グリフの行内送り幅（image px）。writing_mode の行内軸方向の寸。
    fn advance(&self, ch: char, font_height: f32) -> f32;

    /// 行送りピッチ（image px）。M1 正準: `ceil(font_height × 1.25)`
    /// （係数は [`TextLayerConfig::line_pitch_factor`] が正本・既定 1.25）。
    fn line_pitch(&self, font_height: f32) -> f32;

    /// **実レンダリング行ボックス丈**（image px・descent 込み＝`ascent + descent`）。
    ///
    /// em ボックス丈（`font_height`）ではなく、フォントが実際にインクを置く行ボックスの
    /// ブロック軸寸。DirectWrite は行を `ascent + descent`（design metrics）で組むため、
    /// 和文フォントでは `font_height` を大きく超える（実測: Yu Gothic UI ＝ `1.3301em`
    /// ゆえ 28px で 37.24px・ＭＳ ゴシック ＝ ちょうど `1.0em`）。**行矩形（em ボックス）を
    /// そのまま帯として使うと descent 側のインクが帯の外へ出る**——選択肢 hover ハイライト
    /// 矩形／ヒット矩形のブロック軸帯はこの実測丈を源にする（design.md R3.3 の座標整合を
    /// 保ったまま「文字の下が切れる」を構造的に排除する・[`crate::choice::highlight_band_extent`]）。
    ///
    /// 実装: COM 層 `DWriteMetrics` は**実 font face metrics**
    /// （`GetMetrics` の `ascent`/`descent`/`designUnitsPerEm`）から算出し、
    /// [`FixedMetrics`] は決定論仮想値を返す。文字列非依存（フォント固有の設計値）。
    fn line_box_height(&self, font_height: f32) -> f32;
}

/// 構造テスト用の決定論 metrics（R4.5/R11.6）。
///
/// 決定論仮想値: 全角（非 ASCII）＝`font_height`・半角（ASCII）＝`font_height / 2`。
/// 行送りピッチは M1 正準式 `ceil(font_height × 既定係数 1.25)`。行ボックス丈は
/// [`FIXED_LINE_BOX_RATIO`]×`font_height`。
/// タイポグラフィ的正確さは目的でない——折返し・行送りアルゴリズムの檻のための値。
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedMetrics;

/// [`FixedMetrics`] の仮想行ボックス比（`ascent + descent` ÷ em）。
///
/// 和文フォントの実測（Yu Gothic UI ＝ 1.3301em）に倣った**決定論仮想値**——
/// 「行ボックス丈 > em ボックス丈」という実フォントの性質を構造テストへ持ち込むための値で、
/// 特定フォントの再現が目的ではない（`FixedMetrics` の advance 仮想値と同格）。
pub const FIXED_LINE_BOX_RATIO: f32 = 1.33;

impl GlyphMetrics for FixedMetrics {
    fn advance(&self, ch: char, font_height: f32) -> f32 {
        if ch.is_ascii() {
            font_height / 2.0
        } else {
            font_height
        }
    }

    fn line_pitch(&self, font_height: f32) -> f32 {
        (font_height * TextLayerConfig::default().line_pitch_factor).ceil()
    }

    fn line_box_height(&self, font_height: f32) -> f32 {
        font_height * FIXED_LINE_BOX_RATIO
    }
}

/// 行の画像空間矩形（image px 絶対座標・R9.4 の再利用シーム）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineRect {
    /// 左辺（image px）。
    pub left: f32,
    /// 上辺（image px）。
    pub top: f32,
    /// 右辺（image px）。
    pub right: f32,
    /// 下辺（image px）。
    pub bottom: f32,
}

/// 配置済みグリフ（行内軸の絶対位置＋送り幅・クリック可能範囲導出の入力形・R9.4）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    /// グリフの文字。
    pub ch: char,
    /// 行内軸の配置位置（image px 絶対座標。horizontal_tb＝x・縦書き＝y）。
    pub inline_pos: f32,
    /// 行内送り幅（image px・注入 metrics 由来）。
    pub advance: f32,
}

/// 配置済みの 1 行（行矩形＋グリフ列・choice-render 再利用シーム・R9.4）。
#[derive(Clone, Debug, PartialEq)]
pub struct PositionedLine {
    /// 行の画像空間矩形（規約はモジュール doc「行矩形の規約」）。
    pub rect: LineRect,
    /// 行内のグリフ列（行内軸位置の昇順・空行は空列）。
    pub glyphs: Vec<PositionedGlyph>,
}

/// スクロール可視窓（先頭可視行＋ブロック軸オフセット・R7.4 分離シームの上半分）。
///
/// 「可視窓の決定（純粋な計算）」だけを表す値——描画実行は持たない（R7.4）。
/// emo-text-viewbox はこの出力を「クリップ視窓＋内容オフセット」へ写像して
/// 描画実行だけを差し替える。非スクロール時は `first_visible_line = 0`・
/// `block_offset = 0.0`。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleWindow {
    /// 先頭可視行の index（[`LayoutEngine::layout`] 出力の行列に対する添字）。
    pub first_visible_line: usize,
    /// ブロック軸（行送り軸）の内容オフセット（image px・符号付き）。
    ///
    /// 描画時に各行のブロック軸位置へ**加算**する平行移動量
    /// （horizontal_tb＝y・縦書き＝x——軸読み替え正準表のスクロール方向:
    /// 横書き＝内容が上（負）・vertical_rl＝内容が右（正）・vertical_lr＝内容が左（負））。
    /// 行単位スクロールゆえ値は「スキップした行のブロック軸位置差」そのもの。
    pub block_offset: f32,
}

/// layout への折返し計画の受け渡し（OFF は境界値を一切持たない——R4 の構造保証）。
///
/// ゲート③（折返し判定）の分割点選択だけを分岐させるシーム。`CharByChar` は
/// 既存の文字単位折返し（byte 等価の非回帰経路）、`Segmented` は事前計算済みの
/// [`SegmentPlan`] を参照した分かち書きワードラップ（塊先決＋長大塊縮退）。
/// ゲート①（可視打切り）・②（保留フラッシュ）・④（配置）は分岐に依らず不変。
#[derive(Clone, Copy, Debug)]
pub enum WrapPlan<'a> {
    /// 従来の文字単位折返し（既存コードパス・byte 等価）。
    CharByChar,
    /// 分かち書きワードラップ（塊境界は事前計算済みの [`SegmentPlan`] を参照）。
    Segmented(&'a SegmentPlan),
}

/// 折返し・行送りの決定エンジン（純粋・R4.5/R6.1–6.3）。
pub struct LayoutEngine;

impl LayoutEngine {
    /// 折返し・行送りを解決して行列（[`PositionedLine`] 列）を得る（純粋・決定論）。
    ///
    /// - `items`: 追記順の正本（state 層の `ActorTextState::items`）。
    /// - `visible_count`: 可視グリフ数（state 層 `visible_glyphs` の出力）。
    ///   可視 prefix 規則（モジュール doc）で配置対象を切る。
    /// - `wrap`: 折返し計画（[`WrapPlan`]）。**ゲート③（折返し判定）だけ**をこの引数で
    ///   分岐させ、ゲート①（可視打切り）・②（保留フラッシュ）・④（配置）の意味論は
    ///   分岐に依らず不変（design System Flows「ゲート③」）:
    ///   - [`WrapPlan::CharByChar`]（既定・OFF 経路）: 既存の文字単位規則
    ///     （`行内位置＋次グリフ幅 > 閾値`）。この引数を [`WrapPlan::CharByChar`] にした
    ///     出力は本機能導入前の layout と byte 等価（非回帰の構造保証・R4.1/4.3/8.3——
    ///     `SegmentPlan` を一切参照しないため境界値の算出自体が起きない・R4.2）。
    ///   - [`WrapPlan::Segmented`]: 塊先決——塊先頭で塊全体の advance 合計を全文 plan から
    ///     求め、残り行幅（`cap_rem`）に収まれば継続配置、行頭からの行幅（`cap_full`）まで
    ///     なら塊の前で行送りしてから配置、それも超える長大塊は当該塊のみ文字単位規則へ
    ///     縮退する（3.1/3.2）。塊内の残グリフは残数カウンタで追跡し追加判定なしで配置
    ///     （2.1/2.3・浮動丸めでの途中分割を構造排除）。plan に被覆されないグリフ（不整合）
    ///     は既存文字単位規則で配置される（優しい縮退・4.2）。
    /// - 折返し判定: `行内位置＋次グリフ幅 > 閾値`（3 方向共通・正準表）。
    ///   行頭の 1 グリフは閾値超過でも配置する（無限折返しの構造排除・無損失）。
    /// - 行送り量: 自動折返し＝`line_pitch`・改行マーカー＝`line_pitch × Σratio`。
    /// - 改行は遅延（deferred newline・モジュール doc「改行の遅延」）: 到着即時に
    ///   行を送らず保留へ累算し、次の可視グリフ配置の直前に一括実体化する。保留のみ
    ///   では空行を出さず・末尾の保留改行は蒸発する。
    ///
    /// 塊先決は `visible_count` に依存しない（seg_sum は全 `items` から算出・INV-1/7.1）。
    /// ゲート①が④より先にあるため、塊途中で可視が切れても配置済み prefix の行は動かない
    /// （INV-2/7.2/7.3）。塊前行送りは行頭では `cap_rem == cap_full` ゆえ不発火＝空行を
    /// 作らない（INV-3）。縦書きは行内軸の `inline_pos`/`advance`/`threshold` 演算のみゆえ
    /// 新規 mode 分岐なし（6.1/6.2）。
    ///
    /// 同一入力→同一出力（R2.5 系）。失敗経路なし（全入力で値を返す純関数）。
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        items: &[TextItem],
        visible_count: usize,
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
        metrics: &dyn GlyphMetrics,
        wrap: WrapPlan<'_>,
    ) -> Vec<PositionedLine> {
        // pending-cursor の縮退 warn-once は actor 識別＋走査を跨いで持続する guard を要する
        // （per-frame 呼出でのスパム抑止＝ランタイム所有）。actor 文脈を持たない既存呼び口は
        // カーソル換算・遅延実体化（2.1/2.3/2.5）を完全に行いつつ縮退 warn（6.5）だけを抑止する
        // （`None` 経路）。純挙動は [`layout_with_cursor_warn`] と完全同一。
        Self::layout_inner(
            items,
            visible_count,
            region,
            mode,
            font_height,
            metrics,
            wrap,
            None,
        )
    }

    /// [`layout`](Self::layout) の全挙動に加え、`\_l` 換算の 4 縮退分岐（負値絶対／`%`／
    /// `@` 相対／パース不能）を **actor ごと初回のみ** `warn!` する（6.5・design 縮退表）。
    ///
    /// warn guard は走査を跨いで持続する必要がある（per-frame layout 呼出での重複警告抑止）
    /// ため、呼び手（ランタイム＝`actor.rs` の `TextLayerRuntime`・既存 `unresolved_warned` と
    /// 同型の持続 guard）が所有し `&mut` で渡す。行レイアウトの純挙動は [`layout`](Self::layout)
    /// と完全同一——差は縮退ログの有無のみ（guard は決定的な行出力に一切影響しない）。
    #[allow(clippy::too_many_arguments)]
    pub fn layout_with_cursor_warn(
        items: &[TextItem],
        visible_count: usize,
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
        metrics: &dyn GlyphMetrics,
        wrap: WrapPlan<'_>,
        actor: &ActorKey,
        warn: &mut CursorWarnGuard,
    ) -> Vec<PositionedLine> {
        Self::layout_inner(
            items,
            visible_count,
            region,
            mode,
            font_height,
            metrics,
            wrap,
            Some((actor, warn)),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn layout_inner(
        items: &[TextItem],
        visible_count: usize,
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
        metrics: &dyn GlyphMetrics,
        wrap: WrapPlan<'_>,
        mut cursor_warn: Option<(&ActorKey, &mut CursorWarnGuard)>,
    ) -> Vec<PositionedLine> {
        let pitch = metrics.line_pitch(font_height);
        let threshold = region.wrap_threshold();
        let start = region.start();
        // 軸読み替え正準表: 行内軸開始・行送り軸開始・行送り方向（±1）。
        // 行内軸は 3 方向とも正方向（+x／+y）＝折返し判定は共通式で回る。
        let (inline_start, block_start, block_dir) = match mode {
            WritingMode::HorizontalTb => (start.0, start.1, 1.0f32),
            WritingMode::VerticalRl => (start.1, start.0, -1.0f32),
            WritingMode::VerticalLr => (start.1, start.0, 1.0f32),
        };

        let mut lines: Vec<PositionedLine> = Vec::new();
        let mut current: Vec<PositionedGlyph> = Vec::new();
        let mut inline_pos = inline_start;
        let mut block_pos = block_start;
        let mut placed = 0usize;
        // 改行の保留（deferred newline）: None＝保留なし・Some(Σratio)＝累算済み予約。
        // `f32` 単独でなく Option なのは `\n[0]`（ratio 0＝行替え・送りゼロ）を「保留なし」
        // と区別して保存するため（DD-5）。走査ローカル＝フレームを跨ぐ状態を持たない。
        let mut pending: Option<f32> = None;
        // pending-cursor（`\_l` 遅延実体化）: None＝保留なし・Some((inline, block))＝
        // `cursor_to_image_px` 済みの絶対 image px（換算 None の軸は含めない＝当該軸不動・R2.4）。
        // 走査ローカル＝フレームを跨がない（newline-defer の `pending` と同型・同一フラッシュで合成）。
        let mut pending_cursor: Option<(Option<f32>, Option<f32>)> = None;
        // 先決済み塊の残グリフ数（Segmented 経路のみ使用）。正＝塊内（追加判定なし配置）・
        // 0 かつ塊先頭でない＝plan 非被覆（既存 CharByChar 式で判定）——この 2 状態の区別が
        // 「塊は途中分割されない」の型保証（design System Flows「塊内は追加判定なし」）。
        let mut seg_remaining: usize = 0;

        for item in items {
            match *item {
                TextItem::Glyph { ch } => {
                    // ゲート順序の契約（DD-3）: ①可視 prefix 打切り → ②保留フラッシュ →
                    // ③折返し判定 → ④配置。①を先頭に置くことで、リビールカーソルが
                    // 改行を通過済みでも次の可視グリフが無い限り行送りは起きない（R4.2）。
                    if placed == visible_count {
                        break;
                    }
                    let advance = metrics.advance(ch, font_height);
                    // ② 保留フラッシュ（次の可視コンテンツ配置の直前・R2.1/2.3）。保留改行と
                    // pending-cursor は同一フラッシュに混在しうるため順序が意味を持つ（design
                    // 「ゲート②の直後に②'として挿入」）。厳密順序:
                    //   (1) 現在行が非空なら確定（改行・`\_l` とも行区切り＝RN-3・先頭フラッシュは
                    //       current 空ゆえ空行を作らない・DD-2）。
                    //   (2) 保留改行 Σratio を block へ適用し行内を先頭へ戻す（newline-defer 既存規則）。
                    //   (3) pending-cursor の指定軸で inline/block を上書き（絶対 image px・不動軸は
                    //       据え置き）。②' が (2) の改行送り/行内リセットに後勝ち＝カーソル明示位置が最終値。
                    if pending.is_some() || pending_cursor.is_some() {
                        if !current.is_empty() {
                            lines.push(finish_line(
                                std::mem::take(&mut current),
                                mode,
                                inline_start,
                                inline_pos,
                                block_pos,
                                font_height,
                            ));
                        }
                        if let Some(sum) = pending.take() {
                            block_pos += block_dir * pitch * sum;
                            inline_pos = inline_start;
                        }
                        if let Some((inline_val, block_val)) = pending_cursor.take() {
                            if let Some(iv) = inline_val {
                                inline_pos = iv;
                            }
                            if let Some(bv) = block_val {
                                block_pos = bv;
                            }
                        }
                    }
                    // ③ 折返し判定（WrapPlan で分岐・design System Flows「ゲート③」）。
                    // feed＝この可視グリフの配置前に行送りするか。ゲート①②④・行頭 1 グリフ
                    // 配置（無限折返し回避）・行矩形規約は分岐に依らず不変。
                    // 直前にフラッシュした場合 current は空ゆえ二重前進しない。
                    let feed = match wrap {
                        // CharByChar: 既存の文字単位規則そのまま（byte 等価の非回帰経路）。
                        WrapPlan::CharByChar => {
                            !current.is_empty() && inline_pos + advance > threshold
                        }
                        WrapPlan::Segmented(plan) => {
                            if seg_remaining > 0 {
                                // 塊内: 先決済み＝追加判定なしで配置（浮動丸めでの途中分割排除・2.1/2.3）。
                                seg_remaining -= 1;
                                false
                            } else if let Some(seg) = plan.segment_starting_at(placed) {
                                // 塊先頭: 塊全体の advance 合計を全文 plan から左畳み込みで先決
                                // （visible_count 非依存＝INV-1/7.1）。
                                let seg_sum = segment_advance_sum(
                                    items,
                                    placed,
                                    seg.len,
                                    font_height,
                                    metrics,
                                );
                                let cap_rem = threshold - inline_pos; // 残り行幅
                                let cap_full = threshold - inline_start; // 行頭からの行幅
                                if seg_sum <= cap_rem {
                                    // 現在行に収まる → 分割せず継続配置（2.1/2.3）。
                                    seg_remaining = seg.len - 1;
                                    false
                                } else if seg_sum <= cap_full {
                                    // 収まらないが行頭からなら収まる → 塊の前で行送り（2.2）。
                                    // 行頭では cap_rem == cap_full ゆえ本分岐は構造的に不発火
                                    // ＝ワードラップは空行を作らない（INV-3）。
                                    seg_remaining = seg.len - 1;
                                    true
                                } else {
                                    // 長大塊（行頭からでも収まらない）: 当該塊のみ既存 char 規則へ
                                    // 委譲（3.1/3.2）。seg_remaining は設定せず＝続くグリフは非被覆
                                    // として char 規則で処理され、次の塊先頭で通常判定を再開する（3.3）。
                                    !current.is_empty() && inline_pos + advance > threshold
                                }
                            } else {
                                // plan 非被覆（不整合／長大塊の継続）: 既存 char 規則で配置
                                // （優しい縮退・4.2・design Error Handling「plan と items の不整合」）。
                                !current.is_empty() && inline_pos + advance > threshold
                            }
                        }
                    };
                    if feed {
                        lines.push(finish_line(
                            std::mem::take(&mut current),
                            mode,
                            inline_start,
                            inline_pos,
                            block_pos,
                            font_height,
                        ));
                        block_pos += block_dir * pitch;
                        inline_pos = inline_start;
                    }
                    // ④ 配置。
                    current.push(PositionedGlyph {
                        ch,
                        inline_pos,
                        advance,
                    });
                    inline_pos += advance;
                    placed += 1;
                }
                TextItem::LineBreak { ratio } => {
                    // 遅延（deferred newline・R1.1/1.3）: 行を閉じず・block も前進させず、
                    // 保留へ ratio を累算する（連続改行は単一累算 Σratio）。可視構造・
                    // 内容ビューボックスはここでは一切変化しない（R1.2/1.5）。
                    pending = Some(pending.map_or(ratio, |acc| acc + ratio));
                }
                TextItem::CursorMove { x, y } => {
                    // 到着時換算（保留のみ・行は閉じない・R2.1/2.3）。x→水平軸（origin＝validrect
                    // 左辺）／y→垂直軸（origin＝validrect 上辺）を `cursor_to_image_px` で絶対 image px
                    // 化する（em＝font_height・lh＝pitch・原点加算——design §`\_l 換算式`）。
                    let x_val = cursor_to_image_px(x, region.left(), font_height, pitch);
                    let y_val = cursor_to_image_px(y, region.top(), font_height, pitch);
                    // 縮退 4 分岐（負値絶対／`%`／`@`／パース不能）の actor ごと warn-once（6.5）。
                    // Omitted（軸省略）・実導出成功（Some）は無音。guard 不在（`layout` 経路）は抑止。
                    if let Some((actor, guard)) = cursor_warn.as_mut() {
                        warn_cursor_degrade(x, x_val, *actor, &mut **guard);
                        warn_cursor_degrade(y, y_val, *actor, &mut **guard);
                    }
                    // 軸読み替え正準表: 水平/垂直軸値を行内/ブロック軸へ写像（horizontal_tb＝行内 x・
                    // ブロック y／縦書き rl・lr＝行内 y・ブロック x——layout の inline/block 割当と同一）。
                    let (inline_val, block_val) = match mode {
                        WritingMode::HorizontalTb => (x_val, y_val),
                        WritingMode::VerticalRl | WritingMode::VerticalLr => (y_val, x_val),
                    };
                    if inline_val.is_some() || block_val.is_some() {
                        // 有効軸が 1 つ以上＝保留（`\_l` は行区切り性を持つ・フラッシュで実体化）。
                        // 換算 None の軸は保留に含めない（縮退＝状態不変・当該軸不動・R2.4）。
                        pending_cursor = Some((inline_val, block_val));
                    } else {
                        // 両軸 None（`\_l[,]` や全縮退）＝完全 no-op（行区切りもしない・正典
                        // 「両方省略で無効果」・R2.4・design 縮退表 両軸省略/全縮退 row）。
                        tracing::debug!(
                            "両軸縮退の \\_l を完全 no-op として素通しする（行区切りせず）"
                        );
                    }
                }
            }
        }
        // 最終行の確定: グリフを含む現在行のみ確定する（行の確定は常にグリフ配置に
        // 隣接するため、旧 `opened` フラグは `!current.is_empty()` と等価・DD-4）。
        // 残存する保留（末尾改行）は実体化せず蒸発する（R5.2/5.3）。
        if !current.is_empty() {
            lines.push(finish_line(
                current,
                mode,
                inline_start,
                inline_pos,
                block_pos,
                font_height,
            ));
        }
        lines
    }

    /// スクロール可視窓の決定（純粋・R7.1/7.2/7.4/7.5——分離シームの上半分）。
    ///
    /// あふれ判定は軸読み替え正準表の行をそのまま実装する:
    ///
    /// | mode | あふれ判定 | スクロール方向 |
    /// |---|---|---|
    /// | horizontal_tb | 最新行の下端 > validrect.bottom | 縦（内容が上へ） |
    /// | vertical_rl | 最新列の左端 < validrect.left | 横（内容が右へ） |
    /// | vertical_lr | 最新列の右端 > validrect.right | 横（内容が左へ） |
    ///
    /// 3 方向は「行送り方向を正とする正規化ブロック座標」への読み替えで単一式に
    /// 畳む（アルゴリズム分岐なし——layout と同じ規律）。**行単位・即時**（M1 正準・
    /// アニメなし）: 最新行が境界内へ収まる**最小の**先頭可視行を選び、オフセットは
    /// スキップした行のブロック軸位置差そのもの。全行超過でも最新行へ飽和する
    /// （最新行は常に可視・行を失わない）。失敗経路なし（全入力で値を返す純関数）。
    pub fn visible_window(
        lines: &[PositionedLine],
        region: &TextRegion,
        mode: WritingMode,
    ) -> VisibleWindow {
        let Some(last) = lines.last() else {
            return VisibleWindow {
                first_visible_line: 0,
                block_offset: 0.0,
            };
        };
        // 正規化ブロック座標（行送り方向が正）: near＝行の開始側・far＝行の遠端・
        // boundary＝validrect の行送り側境界。正準表のあふれ判定行と 1:1。
        type Edge = fn(&PositionedLine) -> f32;
        let (near, far, boundary, block_dir): (Edge, Edge, f32, f32) = match mode {
            WritingMode::HorizontalTb => (|l| l.rect.top, |l| l.rect.bottom, region.bottom(), 1.0),
            WritingMode::VerticalRl => (|l| -l.rect.right, |l| -l.rect.left, -region.left(), -1.0),
            WritingMode::VerticalLr => (|l| l.rect.left, |l| l.rect.right, region.right(), 1.0),
        };
        let last_far = far(last);
        if last_far <= boundary {
            // あふれ非発火（境界ちょうどは「超えていない」——正準表は > 判定）。
            return VisibleWindow {
                first_visible_line: 0,
                block_offset: 0.0,
            };
        }
        // 行単位スクロール: 最新行が収まる最小スキップ数を探す（全行超過は最新行へ飽和）。
        let origin = near(&lines[0]);
        let first_visible_line = lines
            .iter()
            .position(|line| last_far - (near(line) - origin) <= boundary)
            .unwrap_or(lines.len() - 1);
        // 実軸の平行移動量: 正規化座標のスキップ距離を行送り方向の符号で戻す。
        let block_offset = -block_dir * (near(&lines[first_visible_line]) - origin);
        tracing::debug!(
            ?mode,
            first_visible_line,
            block_offset,
            total_lines = lines.len(),
            "あふれ発火——スクロール可視窓を決定した（行単位・即時）"
        );
        VisibleWindow {
            first_visible_line,
            block_offset,
        }
    }
}

/// 塊の advance 合計（塊先決の判定式の左辺 `seg_sum`）。
///
/// glyph 通し番号 `[start_serial, start_serial + len)`（`items` 中の `Glyph` のみを
/// 0 起点で数えた範囲）のグリフ送り幅を、通し番号昇順＝**左畳み込み順**で合計する
/// （配置も同順ゆえ浮動小数の順序依存を実装と一致させる・design Service Interface）。
/// 全 `items` を走るため合計は `visible_count` に依存しない（INV-1/7.1）。
fn segment_advance_sum(
    items: &[TextItem],
    start_serial: usize,
    len: usize,
    font_height: f32,
    metrics: &dyn GlyphMetrics,
) -> f32 {
    let end = start_serial + len;
    let mut sum = 0.0f32;
    let mut serial = 0usize;
    for item in items {
        if let TextItem::Glyph { ch } = *item {
            if serial >= end {
                break;
            }
            if serial >= start_serial {
                sum += metrics.advance(ch, font_height);
            }
            serial += 1;
        }
    }
    sum
}

/// 行の確定: 行内範囲（開始〜送り終端）と行送り軸位置から行矩形を組む
/// （行送り軸の厚み方向は行送り方向と同符号——モジュール doc「行矩形の規約」）。
fn finish_line(
    glyphs: Vec<PositionedGlyph>,
    mode: WritingMode,
    inline_start: f32,
    inline_end: f32,
    block_pos: f32,
    font_height: f32,
) -> PositionedLine {
    // 行内開始エッジ（rect の行内軸近端）は「実際に置かれた先頭グリフの inline_pos」から取る。
    // グリフは行内軸で単調増加ゆえ先頭が近端。`\_l` カーソル字下げは pending-cursor 実体化で
    // 各グリフの `inline_pos` に載る一方、行頭定数 `inline_start` には載らない。描画（draw.rs）は
    // 行矩形原点（rect.left／縦書きは rect.top）を平行移動原点にして bare 文字列を DWrite で
    // 再レイアウトする——per-glyph `inline_pos` は描画では捨てられる——ため、字下げを rect の
    // 近端エッジへ反映しないと draw が hit（inline_pos 由来）とずれる（R3.3・字下げ描画欠落）。
    // 非カーソル行では先頭グリフ＝`inline_start` ゆえ従来値と一致し回帰なし。3 方向とも先頭グリフ
    // が近端＝式は writing mode に依存しない。空行は呼び手が `!current.is_empty()` で弾くため
    // 構造上出ないが、防御的に `inline_start` へ退避する。
    let inline_lo = glyphs.first().map_or(inline_start, |g| g.inline_pos);
    let rect = match mode {
        WritingMode::HorizontalTb => LineRect {
            left: inline_lo,
            top: block_pos,
            right: inline_end,
            bottom: block_pos + font_height,
        },
        WritingMode::VerticalRl => LineRect {
            left: block_pos - font_height,
            top: inline_lo,
            right: block_pos,
            bottom: inline_end,
        },
        WritingMode::VerticalLr => LineRect {
            left: block_pos,
            top: inline_lo,
            right: block_pos + font_height,
            bottom: inline_end,
        },
    };
    PositionedLine { rect, glyphs }
}

/// `\_l` カーソル座標 → image px 絶対座標の M1 実導出換算（純粋・全域・layout.rs 所有＝
/// レイアウトカーソル意味論）。
///
/// 絶対 Px/Em/Lh の**非負値**のみ `Some(image px 絶対座標)` を返す。Percent／Relative（`@`）／
/// 負値絶対／[`CursorCoord::Invalid`]／[`CursorCoord::Omitted`] は `None`（呼び手が状態不変
/// スキップ＋warn-once・R2.4/6.5）。換算式（design Supporting References §`\_l 換算式`）:
///
/// - `Px`: `image_px = value`（裸数値＝バルーン画像 px 恒等）
/// - `Em`: `image_px = value × font_height`（1em＝タグ時点の文字高さ＝`ResolvedFont::height`）
/// - `Lh`: `image_px = value × line_pitch`（1lh＝行送りピッチ＝`ceil(font_height × 1.25)`）
/// - 最終座標 ＝ `origin`（当該軸の validrect 原点＝`\_l` 原点・文字描画範囲左上・RN-3）＋ `image_px`
///
/// `origin`／`font_height`／`line_pitch` は呼び手が軸読み替え・metrics 解決済みで渡す
/// （本関数は係数乗算と原点加算のみ——`line_pitch` は引数として受け取り内部算出しない）。
/// パニックせず全入力で `Option<f32>` を返す（`Result` なし・R2.4 決定論）。物理化（`×k`）は
/// 呼び手の領分で、本換算は image px で完結する（2 空間モデルの規律・2.2）。
pub fn cursor_to_image_px(
    coord: CursorCoord,
    origin: f32,
    font_height: f32,
    line_pitch: f32,
) -> Option<f32> {
    match coord {
        // 絶対 Px/Em/Lh の非負値のみ実導出（負値絶対はここで弾かず下の match ガードで None）。
        CursorCoord::Absolute { value, unit } if value >= 0.0 => {
            let factor = match unit {
                CursorUnit::Px => 1.0,
                CursorUnit::Em => font_height,
                CursorUnit::Lh => line_pitch,
                // Percent は M1 縮退保持（実導出せず None＝当該軸スキップ）。
                CursorUnit::Percent => return None,
            };
            Some(origin + value * factor)
        }
        // 負値絶対・Relative（@）・Invalid・Omitted は縮退（None＝状態不変スキップ・warn-once）。
        _ => None,
    }
}

/// `\_l` 換算縮退の分岐種別（actor ごと warn-once の鍵・4 分岐）。
///
/// 各分岐を actor ごとに厳密 1 回だけ警告するための識別子（design 縮退表 2.4/6.5）。
/// Omitted（軸省略）は正典の正常形ゆえ本種別に含めない（warn しない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CursorDegrade {
    /// 負値絶対（`\_l[-1,…]`＝非負ゲート外）。
    NegativeAbsolute,
    /// `%`（M1 縮退保持・実導出せず）。
    Percent,
    /// `@` 相対（M1 縮退保持・実導出せず）。
    Relative,
    /// パース不能（`CursorCoord::Invalid`）。
    Invalid,
}

/// pending-cursor 換算縮退の **actor ごと warn-once** 檻（走査を跨いで持続＝ランタイム所有）。
///
/// `\_l` の 4 縮退分岐（負値絶対／`%`／`@`／パース不能）を actor ごと初回のみ `warn!` する
/// （per-frame layout 呼出での重複警告抑止・design 縮退表 2.4/6.5）。既存
/// `TextLayerRuntime::unresolved_warned`（`BTreeSet<ActorKey>`）と同型の持続 guard で、
/// [`LayoutEngine::layout_with_cursor_warn`] に `&mut` で渡す。行レイアウトの純挙動は guard に
/// 依存しない——guard は縮退ログの重複抑止のみを担い、決定的な行出力へ影響しない。
#[derive(Clone, Debug, Default)]
pub struct CursorWarnGuard {
    /// 既に警告済みの `(actor, 縮退分岐)`（決定論的順序のため `BTreeSet`）。
    warned: BTreeSet<(ActorKey, CursorDegrade)>,
}

impl CursorWarnGuard {
    /// `(actor, degrade)` が初回なら記録して `true`（＝今回警告する）、既出なら `false`。
    fn should_warn(&mut self, actor: &ActorKey, degrade: CursorDegrade) -> bool {
        self.warned.insert((actor.clone(), degrade))
    }
}

/// `\_l` 換算が `None`（当該軸スキップ）へ縮退したとき、縮退分岐を分類し actor ごと初回のみ
/// `warn!` する（design 縮退表 2.4/6.5）。
///
/// `converted.is_some()`（実導出成功）・`CursorCoord::Omitted`（軸省略＝正典の正常形）は
/// 何もしない。負値絶対は unit を問わず非負ゲート外ゆえ `NegativeAbsolute` として分類する
/// （`value < 0.0` を `Percent` より先に判定）。
fn warn_cursor_degrade(
    coord: CursorCoord,
    converted: Option<f32>,
    actor: &ActorKey,
    guard: &mut CursorWarnGuard,
) {
    if converted.is_some() {
        // 実導出成功＝縮退なし（非負 Px/Em/Lh）。
        return;
    }
    let degrade = match coord {
        // 軸省略は正典の正常形（縮退表「\_l 軸省略」＝ログなし・R2.4）。
        CursorCoord::Omitted => return,
        // 負値絶対（unit を問わず value<0 は非負ゲート外＝None）。
        CursorCoord::Absolute { value, .. } if value < 0.0 => CursorDegrade::NegativeAbsolute,
        // 非負 `%` は M1 縮退保持。
        CursorCoord::Absolute {
            unit: CursorUnit::Percent,
            ..
        } => CursorDegrade::Percent,
        // 非負 Px/Em/Lh は `converted` が Some ゆえ到達しない——防御的に無音。
        CursorCoord::Absolute { .. } => return,
        CursorCoord::Relative { .. } => CursorDegrade::Relative,
        CursorCoord::Invalid => CursorDegrade::Invalid,
    };
    if guard.should_warn(actor, degrade) {
        tracing::warn!(
            actor = %actor,
            ?coord,
            ?degrade,
            "\\_l 座標が縮退した（当該軸スキップ・状態不変）——actor ごと初回のみ警告する（6.5）"
        );
    }
}

#[cfg(test)]
#[path = "layout_cursor_tests.rs"]
mod cursor_tests;
#[cfg(test)]
#[path = "layout_segmented_tests.rs"]
mod segmented_tests;
#[cfg(test)]
#[path = "layout_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "layout_visible_window_tests.rs"]
mod visible_window_tests;
#[cfg(test)]
#[path = "layout_wrap_tests.rs"]
mod wrap_tests;
