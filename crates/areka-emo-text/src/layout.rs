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
//! | 折返し判定 | 行内位置＋次グリフ幅 > 折返し基準／描画範囲の遠辺（3 方向共通・行内軸は常に正方向） | 同 | 同 |
//!
//! 折返し基準・描画範囲の遠辺・描画開始点は [`TextRegion`] が解決済みの絶対値（image px）。
//!
//! ## 行内軸の二段構え（折返し基準と描画範囲・R6.2/6.3）
//!
//! 行内軸には別々の意味を持つ 2 つの値が立つ——**折返し基準**
//! （[`TextRegion::wrap_threshold`]・`wordwrappoint`＝「ここを超えたら折り返す」）と
//! **描画範囲の行内軸の遠辺**（[`TextRegion::inline_limit`]・`validrect` の当該辺＝
//! 「ここを超えてはならない」絶対上限）である。折返しはどちらかを超えそうなら起き、
//! 遠辺の判定は折返し方式にも塊の途中かどうかにも依らず**配置の直前に必ず**通る。
//! 2 つの値は片方へ丸め込まない（絶対上限の意味論と、行末禁則文字が基準を超えて
//! ぶら下がる余地〔未実装〕を残すため）。折返し基準が遠辺の内にあるバルーン
//! （通常の定義）では遠辺の判定は決して発火せず、出力は本規則の導入前と一致する——
//! ただし **`\_l` による行内位置の跳躍を伴わない入力に限る**。跳躍先が描画範囲の遠辺の
//! 近くなら、折返し基準が内にあっても要件 6.2（描画範囲の外に置かない）が優先して
//! 折り返す。
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
//! 例外は **`\_l` が保留中のときだけ**である——改行の到着で、それより前に書かれた保留の
//! 実体化（現在行の確定 → **保留改行の適用** → カーソル位置の適用の 3 段）が先に走る
//! （書かれた順の適用・DD-11）。到着した改行そのものはやはり保留へ積まれるだけ（累算送りが
//! 効くのは次の可視グリフの直前）で、`\_l` を挟まない改行列は 1 ビットも変わらない
//! ＝上の規則は不変である。
//!
//! ## 行矩形の規約（R9.4 の再利用シーム）
//!
//! [`PositionedLine::rect`] は image px の絶対矩形。行内軸範囲＝行内開始〜最終グリフ
//! 送り終端（空行は零幅）・行送り軸範囲＝行位置から `font_height` 分（horizontal_tb
//! は下方向・vertical_rl は左方向・vertical_lr は右方向＝行送り方向と同符号）。
//! グリフ別の行内位置＋送り幅と併せ、choice-render のクリック可能範囲導出が
//! そのまま再利用できる（導出自体は実装しない・R9.4）。

use areka_sakura::contract::ActorKey;

use crate::cursor_tag::{
    CursorAxis, CursorBasis, CursorWarnGuard, note_out_of_range, resolve_cursor_axis,
    warn_cursor_degrade,
};
use crate::region::TextRegion;
use crate::segment::SegmentPlan;
use crate::state::{CursorCoord, TextItem, TextLayerConfig};
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

    /// 行送りピッチ（image px）。正典式: `font_height + 行間`（切り上げなし）。
    /// 式も行間の既定値（2）も [`TextLayerConfig::line_pitch`] が正本で、
    /// 実装は自前で足し算をせずそこへ委譲する（design.md §4.1・R3.5）。
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
/// 行送りピッチは既定の調整値を読んで [`TextLayerConfig::line_pitch`] へ委譲する
/// （`font_height + 行間 2`・自前の仮想行間を持たない）。行ボックス丈は
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
        TextLayerConfig::default().line_pitch(font_height)
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
    ///     （`行内位置＋次グリフ幅 > 折返し基準`）。この引数を [`WrapPlan::CharByChar`] にした
    ///     出力は本機能導入前の layout と byte 等価（非回帰の構造保証・R4.1/4.3/8.3——
    ///     `SegmentPlan` を一切参照しないため境界値の算出自体が起きない・R4.2）。
    ///   - [`WrapPlan::Segmented`]: 塊先決——塊先頭で塊全体の advance 合計を全文 plan から
    ///     求め、残り行幅（`cap_rem`）に収まれば継続配置、行頭からの行幅（`cap_full`）まで
    ///     なら塊の前で行送りしてから配置、それも超える長大塊は当該塊のみ文字単位規則へ
    ///     縮退する（3.1/3.2）。塊内の残グリフは残数カウンタで追跡し追加判定なしで配置
    ///     （2.1/2.3・浮動丸めでの途中分割を構造排除）。plan に被覆されないグリフ（不整合）
    ///     は既存文字単位規則で配置される（優しい縮退・4.2）。
    /// - 折返し判定: `行内位置＋次グリフ幅 > 折返し基準` **または** `> 描画範囲の遠辺`
    ///   （3 方向共通・正準表・モジュール doc「行内軸の二段構え」）。遠辺の判定は
    ///   [`WrapPlan`] の分岐にも塊の途中かどうかにも依らず配置の直前に必ず通る。
    ///   行頭の 1 グリフはどちらを超えても配置する（無限折返しの構造排除・無損失）。
    /// - 行送り量: 自動折返し＝`line_pitch`・改行マーカー＝`line_pitch × Σratio`。
    /// - 改行は遅延（deferred newline・モジュール doc「改行の遅延」）: 到着即時に
    ///   行を送らず保留へ累算し、次の可視グリフ配置の直前に一括実体化する。保留のみ
    ///   では空行を出さず・末尾の保留改行は蒸発する。
    ///
    /// 塊先決は `visible_count` に依存しない（seg_sum は全 `items` から算出・INV-1/7.1）。
    /// ゲート①が④より先にあるため、塊途中で可視が切れても配置済み prefix の行は動かない
    /// （INV-2/7.2/7.3）。塊前行送りは行頭では `cap_rem == cap_full` ゆえ不発火＝空行を
    /// 作らない（INV-3）。縦書きは行内軸の `inline_pos`/`advance`/折返し基準/遠辺の演算のみゆえ
    /// 新規 mode 分岐なし（6.1/6.2・遠辺の軸解決は [`TextRegion`] が済ませている）。
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
        // pending-cursor の縮退 warn-once はキャラクター識別＋走査を跨いで持続する guard を要する
        // （per-frame 呼出でのスパム抑止＝ランタイム所有）。キャラクター文脈を持たない既存呼び口は
        // カーソルの解決・遅延実体化（2.1/2.3/2.5）を完全に行いつつ縮退 warn（R5.3）だけを抑止する
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

    /// [`layout`](Self::layout) の全挙動に加え、`\_l` の 2 縮退分岐（解釈不能／中央指定の
    /// 軸取り違え）を **キャラクターごと初回のみ** `warn!` する（R5.3・design 縮退表）。
    ///
    /// 負値絶対・`%`・`@` 相対は縮退ではなく**実導出**なので警告の対象ではない（R5.2）。
    ///
    /// warn guard は走査を跨いで持続する必要がある（per-frame layout 呼出での重複警告抑止）
    /// ため、呼び手（ランタイム＝`actor.rs` の `TextLayerRuntime`・既存 `unresolved_warned` と
    /// 同型の持続 guard）が所有し `&mut` で渡す。型の住処は解決層
    /// [`crate::cursor_tag::CursorWarnGuard`] で、本 API の署名は変わらない。行レイアウトの
    /// 純挙動は [`layout`](Self::layout) と完全同一——差は縮退ログの有無のみ（guard は決定的な
    /// 行出力に一切影響しない）。
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
        // 行内軸の二段構え（design.md §4.3・R6.2/6.3/6.8）: 折返し基準（soft・「超えたら
        // 折り返す」）と描画範囲の遠辺（hard・「超えてはならない」絶対上限）を**別の値**として
        // 持つ。`min` へ畳み込まない——畳むと絶対上限の意味論も、行末禁則文字が基準を超えて
        // ぶら下がる余地（本仕様では未実装）も表せなくなる。
        let soft = region.wrap_threshold();
        let hard = region.inline_limit();
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
        // 解決層（[`crate::cursor_tag::resolve_cursor_axis`]）が返した絶対 image px
        // （移動が成立しなかった軸は含めない＝当該軸不動・R1.6/5.5）。
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
                    //       据え置き）。
                    // ②' が (2) の改行送り/行内リセットに後勝ちするのは **`\n` → `\_l` の順で
                    // 書かれたときだけ**である（書かれた順の適用・DD-11）。逆順 `\_l` → `\n` では
                    // 改行の到着時点で 3 段が走り済みで、ここへ来る `pending_cursor` は空だから
                    // 改行が後勝ちする（`LineBreak` 腕を参照）。
                    if pending.is_some() || pending_cursor.is_some() {
                        finish_pending_line(
                            &mut lines,
                            &mut current,
                            mode,
                            inline_start,
                            inline_pos,
                            block_pos,
                            font_height,
                        );
                        apply_pending_newline(
                            &mut pending,
                            &mut inline_pos,
                            &mut block_pos,
                            inline_start,
                            block_dir,
                            pitch,
                        );
                        apply_pending_cursor(&mut pending_cursor, &mut inline_pos, &mut block_pos);
                    }
                    // ③ 折返し判定（WrapPlan で分岐・design System Flows「ゲート③」）。
                    // feed＝この可視グリフの配置前に行送りするか。ゲート①②④・行頭 1 グリフ
                    // 配置（無限折返し回避）・行矩形規約は分岐に依らず不変。
                    // 直前にフラッシュした場合 current は空ゆえ二重前進しない。
                    // 塊内（先決済み）かどうかは、分岐が `seg_remaining` を減らす前に読む。
                    let in_segment = seg_remaining > 0;
                    let feed = match wrap {
                        // CharByChar: 既存の文字単位規則そのまま（byte 等価の非回帰経路）。
                        WrapPlan::CharByChar => !current.is_empty() && inline_pos + advance > soft,
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
                                // 塊の収まり判定の基準は soft と hard の近い方（塊は
                                // どちらも超えられない）。2 つの値は畳まずに持ったまま、
                                // ここでの「行幅」の計算にだけ近い方を使う。
                                let limit = soft.min(hard);
                                let cap_rem = limit - inline_pos; // 残り行幅
                                let cap_full = limit - inline_start; // 行頭からの行幅
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
                                    !current.is_empty() && inline_pos + advance > soft
                                }
                            } else {
                                // plan 非被覆（不整合／長大塊の継続）: 既存 char 規則で配置
                                // （優しい縮退・4.2・design Error Handling「plan と items の不整合」）。
                                !current.is_empty() && inline_pos + advance > soft
                            }
                        }
                    };
                    // ③' 描画範囲の遠辺（hard）の判定。分岐（CharByChar／塊内／塊先頭／
                    // 非被覆）に依らず**配置の直前に必ず**通す——これが「描画範囲の外へ
                    // 文字を置かない」（R6.2）を構造で保つ最後の門である。行頭の 1 グリフ
                    // （`current` が空）は soft と同じく例外で、超えても置く（無限折返しの排除）。
                    let over_hard = !current.is_empty() && inline_pos + advance > hard;
                    if over_hard && in_segment {
                        // 先決済みの塊が途中で割れる＝「塊は分割されない」の例外。塊の容量は
                        // 塊先頭で `limit = soft.min(hard)` を基準に先決してあるので、送りを積む
                        // だけではここへ届かない——`\_l`（[`TextItem::CursorMove`]）が塊の途中で
                        // 行内位置を跳ばしたときにだけ発火する。ゆえに折返し基準が描画範囲の
                        // 内にある通常のバルーン（soft ≤ hard）でも起こりうる縮退である。
                        // 判断の理由が読める形で 1 件残す（design.md 縮退表・R6.6）。
                        tracing::debug!(
                            inline_pos,
                            advance,
                            hard,
                            "塊の途中で描画範囲の遠辺に達した——塊を分割して次行へ続ける"
                        );
                    }
                    if feed || over_hard {
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
                    // 書かれた順の適用（DD-11）: 到着時点でカーソルが保留中なら、**この改行を
                    // 保留へ積む前に、それより前に書かれた保留を完全に実体化する**——保留フラッシュ
                    // （ゲート②）と同じ (1) 現在行の確定 →(2) 保留改行 →(3) カーソル適用の **3 段**を、
                    // フラッシュ本体と**同じ実装**（[`finish_pending_line`]／[`apply_pending_newline`]／
                    // [`apply_pending_cursor`]）で走らせる。
                    //
                    // **(2) を省いて (1)(3) の 2 段にしてはならない。** `\_l` より前に書かれた保留改行の
                    // Σ が (3) を**追い越して**保留に残り、この直後に積む改行と合流して二重に効くからで
                    // ある——`[あ, \n, \_l[,100], \n, あ]` が 100 + 2×13 = 126 になり、書かれた順の 113
                    // にも旧正典の 100 にも一致しない（前に書かれた改行がカーソルの後に効いてしまう）。
                    //
                    // 分岐の門を `pending_cursor` の有無に置いているのは、カーソルが絡まない純粋な
                    // 改行列の意味論（連続改行は単一累算 Σratio・モジュール doc「改行の遅延」）を
                    // 1 ビットも動かさないため。門を `pending` にも広げると Σ が分割適用され、
                    // `pitch × Σ` と `Σ(pitch × ratio)` の丸めが分かれうる。
                    //
                    // これで `\_l` → `\n` の順では改行が後勝ちし（次行の先頭へ着地）、
                    // `\n` → `\_l` の順では従来どおりカーソルが後勝ちする（到着時に保留カーソルが
                    // 無いので本分岐は不発火）——順序で結果が分かれるのが正典の振舞いである。
                    // 末尾規則は不変: 実体化は位置の更新と現在行の確定だけで、内容の無い行は作らない。
                    if pending_cursor.is_some() {
                        finish_pending_line(
                            &mut lines,
                            &mut current,
                            mode,
                            inline_start,
                            inline_pos,
                            block_pos,
                            font_height,
                        );
                        apply_pending_newline(
                            &mut pending,
                            &mut inline_pos,
                            &mut block_pos,
                            inline_start,
                            block_dir,
                            pitch,
                        );
                        apply_pending_cursor(&mut pending_cursor, &mut inline_pos, &mut block_pos);
                    }
                    // 遅延（deferred newline・R1.1/1.3）: 行を閉じず・block も前進させず、
                    // 保留へ ratio を累算する（連続改行は単一累算 Σratio）。可視構造・
                    // 内容ビューボックスはここでは一切変化しない（R1.2/1.5）。
                    pending = Some(pending.map_or(ratio, |acc| acc + ratio));
                }
                TextItem::CursorMove { x, y } => {
                    // 到着時解決（保留のみ・行は閉じない・R2.1/2.3）。意味論そのもの
                    // （基点＋値×係数・縮退の分類・警告の一回化・範囲外の記録）は解決層
                    // [`crate::cursor_tag`] が持ち、本腕はその**配線**だけを担う——
                    // 実効位置を image 軸へ逆写像して軸ごとに解決を呼び、返った値を
                    // 行内／行送り軸へ写して保留する（design.md「配線 `LayoutEngine`」の 5 段手順）。
                    //
                    // 実効位置の image 軸への逆写像（`@` 相対の基点）。軸読み替え正準表の逆向き:
                    // `horizontal_tb` は行内＝x・行送り＝y、縦書き 2 方向は行内＝y・行送り＝x。
                    // `vertical_rl` の `block_pos` は列の**右端**なので、`\_l[@-1lh,0]` は
                    // 自動列送り（`block_pos += −1 × pitch`）と同じ値を与える＝正典「1 列ぶん左の列の先頭へ」。
                    //
                    // ここで渡すのは走査ローカルの現在位置ではなく**実効位置**——
                    // 「もし今フラッシュしたら次の文字が置かれる位置」である（R3.1「直前までに
                    // 置かれた文字の次に文字が置かれる位置」）。保留中の改行と保留中のカーソルを
                    // **保留フラッシュ（ゲート②）と同じ順**で仮適用して求める:
                    //   (1) 行の確定は inline_pos／block_pos を動かさない＝実効位置に寄与しない。
                    //   (2) 保留改行 Σratio を行送り軸へ適用し、行内軸を先頭へ戻す。
                    //   (3) 保留カーソルの指定軸で上書きする（不動軸は据え置き）。
                    // これは**フラッシュの複製ではなく、同じ規則に従う読み取り専用の計算**である
                    // ——`pending`／`pending_cursor` は `take()` せず、`inline_pos`／`block_pos`
                    // も書き換えない（走査ローカルは無変更・R3.5「基点は `\_l` 実行時点に固定」）。
                    let mut eff_inline = inline_pos;
                    let mut eff_block = block_pos;
                    if let Some(sum) = pending {
                        eff_block += block_dir * pitch * sum;
                        eff_inline = inline_start;
                    }
                    if let Some((inline_val, block_val)) = pending_cursor {
                        if let Some(iv) = inline_val {
                            eff_inline = iv;
                        }
                        if let Some(bv) = block_val {
                            eff_block = bv;
                        }
                    }
                    // 実効位置を image 軸へ逆写像する（変数名 `eff` は、同ファイルで 200 行に
                    // わたり「現在行のグリフ列」を意味する `current` との衝突を避けるため）。
                    let eff = match mode {
                        WritingMode::HorizontalTb => (eff_inline, eff_block),
                        WritingMode::VerticalRl | WritingMode::VerticalLr => {
                            (eff_block, eff_inline)
                        }
                    };
                    // 基点束。原点は**解決済みの文字描画開始点** `TextRegion::start()`（宣言された
                    // `origin` 成分は字義どおり・未宣言成分は書字開始角へ縮退）であって、validrect の
                    // 辺ではない（Requirement 2.1）。軸の向きは 3 書字方向共通（X 正＝右・Y 正＝下）で、
                    // 書字方向で変わるのは原点の位置だけ——`horizontal_tb`／`vertical_lr` は
                    // `(left, top)`・`vertical_rl` は `(right, top)`（design Data Models の原点表）。
                    // これにより `vertical_rl` の `\_l[0,0]` が 1 列目の先頭を指す（2.3）。
                    // `centerx`／`centery` の基準はバルーン画像の原寸（validrect でも原点でもない・4.3）。
                    let basis = CursorBasis {
                        origin: start,
                        current: eff,
                        image_size: region.image_size(),
                        font_height,
                        line_pitch: pitch,
                    };
                    let x_val = resolve_cursor_component(
                        x,
                        CursorAxis::X,
                        &basis,
                        region,
                        &mut cursor_warn,
                    );
                    let y_val = resolve_cursor_component(
                        y,
                        CursorAxis::Y,
                        &basis,
                        region,
                        &mut cursor_warn,
                    );
                    // 軸読み替え正準表: 水平/垂直軸値を行内/ブロック軸へ写像（horizontal_tb＝行内 x・
                    // ブロック y／縦書き rl・lr＝行内 y・ブロック x——layout の inline/block 割当と同一）。
                    let (inline_val, block_val) = match mode {
                        WritingMode::HorizontalTb => (x_val, y_val),
                        WritingMode::VerticalRl | WritingMode::VerticalLr => (y_val, x_val),
                    };
                    if inline_val.is_some() || block_val.is_some() {
                        // 有効軸が 1 つ以上＝保留（`\_l` は行区切り性を持つ・フラッシュで実体化）。
                        // 移動が成立しなかった軸は保留に含めない（省略・縮退＝状態不変・
                        // 当該軸不動・R1.6/5.5）。
                        //
                        // 保留は**軸ごとに合成**する（丸ごと上書きしない）——後の指定が動かさ
                        // なかった軸は先の指定が保留した値を保つ。正典「省略＝移動しない」は
                        // 「先に保留された値を捨てる」ことまでは意味しないからである（R1.2/1.6/
                        // 3.5・検証表 H2: `\_l[10,]\_l[,20]` → (10, 20)）。
                        let (old_inline, old_block) = pending_cursor.unwrap_or((None, None));
                        pending_cursor = Some((inline_val.or(old_inline), block_val.or(old_block)));
                    } else {
                        // 両軸 None（`\_l[,]` や両軸縮退）＝完全 no-op（行区切りもしない・正典
                        // 「両方省略で無効果」・R1.6/5.4/6.2・design 縮退表 両軸省略/両軸縮退 row）。
                        // **既存の保留も変えない**——`pending_cursor` への代入はこの腕には無い
                        // （合成は「成立した軸だけを重ねる」であって、不成立は保留の消去ではない）。
                        tracing::debug!(
                            "[layout_inner] 両軸縮退の \\_l を完全 no-op として素通しする（行区切りせず）"
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

/// 保留の実体化のうち **(1) 現在行の確定**（改行・`\_l` とも行区切り＝RN-3）。
///
/// 保留フラッシュ（ゲート②）と `LineBreak` 到着時の先行実体化（DD-11）が**共有する唯一の
/// 実装**である——複製すると 2 つの経路で「行区切り」の意味が黙って分かれうる。現在行が空なら
/// 何もしない（先頭フラッシュが空行を作らない・DD-2。末尾規則もこの 1 行で保たれる）。
fn finish_pending_line(
    lines: &mut Vec<PositionedLine>,
    current: &mut Vec<PositionedGlyph>,
    mode: WritingMode,
    inline_start: f32,
    inline_pos: f32,
    block_pos: f32,
    font_height: f32,
) {
    if current.is_empty() {
        return;
    }
    lines.push(finish_line(
        std::mem::take(current),
        mode,
        inline_start,
        inline_pos,
        block_pos,
        font_height,
    ));
}

/// 保留の実体化のうち **(2) 保留改行の適用**（累算送り `pitch × Σratio` を行送り軸へ載せ、
/// 行内軸を行頭へ戻す・newline-defer の既存規則）。適用した保留は消費する。
///
/// [`finish_pending_line`]／[`apply_pending_cursor`] と同じく、フラッシュ本体と
/// `LineBreak` 到着時の先行実体化（DD-11）が共有する唯一の実装である。
fn apply_pending_newline(
    pending: &mut Option<f32>,
    inline_pos: &mut f32,
    block_pos: &mut f32,
    inline_start: f32,
    block_dir: f32,
    pitch: f32,
) {
    if let Some(sum) = pending.take() {
        *block_pos += block_dir * pitch * sum;
        *inline_pos = inline_start;
    }
}

/// 保留の実体化のうち **(3) 保留カーソルの適用**（指定軸だけを絶対 image px で上書きし、
/// 不動軸は据え置く・R1.6/5.5）。適用した保留は消費する。
///
/// [`finish_pending_line`] と同じく、フラッシュ本体と先行実体化（DD-11）が共有する唯一の実装。
fn apply_pending_cursor(
    pending_cursor: &mut Option<(Option<f32>, Option<f32>)>,
    inline_pos: &mut f32,
    block_pos: &mut f32,
) {
    if let Some((inline_val, block_val)) = pending_cursor.take() {
        if let Some(iv) = inline_val {
            *inline_pos = iv;
        }
        if let Some(bv) = block_val {
            *block_pos = bv;
        }
    }
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

/// `\_l` の 1 軸ぶんを解決層へ委譲し、記録の 2 口へ配線する（配線層の責務そのもの）。
///
/// 意味論（基点＋値×係数・縮退の分類）は [`crate::cursor_tag::resolve_cursor_axis`] が持つ。
/// 本関数が足すのは戻り値の 3 形への振り分けだけで、**採る契約は次の 1 行に尽きる**:
///
/// - `Ok(Some(px))`＝移動が成立 → [`note_out_of_range`] で範囲外なら DEBUG を 1 件残し
///   （**位置は動かさない**＝内側へ寄せない・R2.6）、値をそのまま返す。
/// - `Ok(None)`＝軸省略 → 当該軸不動・**無音**（正典の正常形・R5.5）。
/// - `Err(degrade)`＝縮退 → guard があれば [`warn_cursor_degrade`]（キャラクター・分岐ごと
///   初回 1 回）。guard 不在（[`LayoutEngine::layout`] 経路）は警告を抑止するだけで、
///   当該軸不動という**純挙動は同一**である。
///
/// すなわち「`Err` のときだけ警告する」——`cursor_tag_resolve_tests.rs` の局所ヘルパ
/// `warn_if_degraded` が写しているのはこの契約である。
fn resolve_cursor_component(
    coord: CursorCoord,
    axis: CursorAxis,
    basis: &CursorBasis,
    region: &TextRegion,
    cursor_warn: &mut Option<(&ActorKey, &mut CursorWarnGuard)>,
) -> Option<f32> {
    match resolve_cursor_axis(coord, axis, basis) {
        Ok(Some(value)) => {
            // 範囲外は記録するだけ（値は素通し）。戻り値を使って寄せてはならない（R2.6）。
            note_out_of_range(axis, value, region);
            Some(value)
        }
        Ok(None) => None,
        Err(degrade) => {
            if let Some((actor, guard)) = cursor_warn.as_mut() {
                warn_cursor_degrade(actor, axis, coord, degrade, guard);
            }
            None
        }
    }
}

#[cfg(test)]
#[path = "layout_cursor_center_origin_tests.rs"]
mod cursor_center_origin_tests;
#[cfg(test)]
#[path = "layout_cursor_order_tests.rs"]
mod cursor_order_tests;
#[cfg(test)]
#[path = "layout_cursor_overflow_tests.rs"]
mod cursor_overflow_tests;
#[cfg(test)]
#[path = "layout_cursor_tests.rs"]
mod cursor_tests;
#[cfg(test)]
#[path = "layout_cursor_vertical_canon_tests.rs"]
mod cursor_vertical_canon_tests;
#[cfg(test)]
#[path = "layout_cursor_vertical_tests.rs"]
mod cursor_vertical_tests;
#[cfg(test)]
#[path = "layout_cursor_wiring_tests.rs"]
mod cursor_wiring_tests;
#[cfg(test)]
#[path = "layout_hard_limit_tests.rs"]
mod hard_limit_tests;
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
