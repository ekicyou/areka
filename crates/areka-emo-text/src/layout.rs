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
//! 折返し・改行後の行は、描画開始点（範囲内の origin 宣言は宣言どおり・範囲外と未宣言は書字開始角）の**行内軸成分**へ戻る
//! （全行で同一の行内開始＝単一規則。行送り軸成分だけが行ごとに進む）。
//!
//! ## 可視 prefix 規則（typewriter との接続）
//!
//! `layout` は追記順 items の先頭から「`visible_count`+1 個目のグリフ」直前までを
//! 配置対象とする。リビール時刻の解決（`visible_glyphs(actor, t)`）は state 層の
//! 領分で、本層は個数だけを受け取る。
//!
//! 渡される items は届いた字の列とは限らない。字は台本のタグとタグの間のひと続きごとに
//! 分かれて届くため、分かち書きの折返しでは呼び手（`actor_present.rs` の `arrange_lines`）が、
//! 再生の前に知らされた台本の全部から作った区間の全文（届いた字の列はその先頭）を渡し、
//! 見える数で切らせる。全文が無い・届いた字の列が全文の先頭と食い違うときは届いた字の列を
//! 渡す。どちらでも本層の規則は同じである。`visible_count` より後ろの items が効くのは、
//! 分かち書きの塊の幅の合計（見えている字が塊の途中で切れても、塊全体の幅で塊の前の行送りを
//! 決める）と、見えている最後の字の後ろの改行・カーソル移動を先に読むこと（次の字が置かれる
//! まで保留されるだけで、行の割り当てには出ない）の 2 つだけである。
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

use crate::cursor_tag::{CursorAxis, CursorBasis, CursorWarnGuard};
use crate::look::{GlyphStyles, StyleId, TextLook};
use crate::region::TextRegion;
use crate::segment::SegmentPlan;
use crate::state::{TextItem, TextLayerConfig};
use crate::writing::WritingMode;

// 本ファイルは行配置の本体で、自己完結した補助 2 つ（塊の advance 合計・`\_l` の 1 軸
// 解決の配線）は子モジュール `layout_line_ops.rs` が持つ。純移動ゆえ本体からの
// 呼び出し方は分割前と同一。
#[path = "layout_line_ops.rs"]
mod line_ops;

#[path = "layout_styled.rs"]
mod styled;

#[path = "layout_scan.rs"]
mod scan;

use line_ops::{resolve_cursor_component, segment_advance_sum};
use styled::{LineHeights, glyph_style_advance, line_pitch_of};

/// グリフ送りの注入点（metrics 依存の唯一の口・R4.5）。
///
/// 「グリフ送り幅・行送りピッチ」だけを注入し、折返し位置・行送りの決定
/// アルゴリズム自体は純粋に保つ分離線の正準。構造テストは [`FixedMetrics`]、
/// 実行時は COM 層の DWriteMetrics（測定専用 probe TextLayout 由来）を注入する。
/// 両者で折返し位置は異なってよいが、アルゴリズム分岐は存在しない。
pub trait GlyphMetrics {
    /// グリフ（書記素クラスタ 1 つ）の行内送り幅（image px）。writing_mode の行内軸方向の寸。
    fn advance(&self, text: &str, font_height: f32) -> f32;

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

    /// **見た目込み**のグリフ行内送り幅（image px・R7.10／R11.1）。
    ///
    /// 既定実装は見た目の**大きさだけ**を見て [`GlyphMetrics::advance`] へ委譲する——
    /// 既存の 2 実装（[`FixedMetrics`]・COM 層 `DWriteMetrics`）と既存の呼び手は
    /// これで無変更のまま済む。フォント名・太字・斜体まで含めて測るのは
    /// 実測 metrics（`DWriteMetrics`）の領分で、そちらが本メソッドを上書きする。
    fn advance_styled(&self, text: &str, look: &TextLook) -> f32 {
        self.advance(text, look.height)
    }
}

/// 構造テスト用の決定論 metrics（R4.5/R11.6）。
///
/// 決定論仮想値: ASCII だけのクラスタ＝半角 `font_height / 2`・それ以外＝全角 `font_height`
/// 1 つ分（ZWJ 列などの部品の数で増やさない）。
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
    fn advance(&self, text: &str, font_height: f32) -> f32 {
        if text.is_ascii() {
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
#[derive(Clone, Debug, PartialEq)]
pub struct PositionedGlyph {
    /// グリフの文字列（書記素クラスタ 1 つ・アイテムの写し＝参照数の増減だけで割り当てない）。
    pub text: std::sync::Arc<str>,
    /// 行内軸の配置位置（image px 絶対座標。horizontal_tb＝x・縦書き＝y）。
    pub inline_pos: f32,
    /// 行内送り幅（image px・注入 metrics 由来）。
    pub advance: f32,
    /// この文字に効く装飾の番号（[`StyleId::DEFAULT`]＝そのスコープの既定の見た目・R3.3）。
    ///
    /// 番号の意味を与えるのは装飾の表（[`crate::look::StyleTable`]）で、本型は写しを運ぶだけ。
    /// 番号列を渡さない配置（[`LayoutEngine::layout`]・[`LayoutEngine::layout_with_cursor_warn`]）
    /// では全グリフが [`StyleId::DEFAULT`] になる。
    pub style: StyleId,
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
    /// 値は「**先頭可視行の開始側と描画範囲の開始側の差**」である（送りの後、先頭可視行の
    /// 開始側は描画範囲の開始側にぴたりと重なる）。最初の行の前に空き（台詞冒頭の保留改行）が
    /// 無い入力では `lines[0]` の開始側が描画範囲の開始側に等しいため、値は旧規則
    /// 「スキップした行のブロック軸位置差」と一致する（D17・R17.1）。
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
    /// - `items`: 追記順の字の列（state 層の `ActorTextState::items`）。届いた字の列か、
    ///   届いた字の列を先頭に持つ区間の全文（モジュール doc「可視 prefix 規則」）。
    /// - `visible_count`: 可視グリフ数（state 層 `visible_glyphs` の出力）。
    ///   可視 prefix 規則（モジュール doc）で配置対象を切る。
    /// - `wrap`: 折返し計画（[`WrapPlan`]）。**ゲート③（折返し判定）だけ**をこの引数で
    ///   分岐させ、ゲート①（可視打切り）・②（保留フラッシュ）・④（配置）の意味論は
    ///   分岐に依らず不変（design System Flows「ゲート③」）:
    ///   - [`WrapPlan::CharByChar`]（既定・OFF 経路）: 既存の文字単位規則
    ///     （`行内位置＋次グリフ幅 > 折返し基準`）。この引数を [`WrapPlan::CharByChar`] にした
    ///     出力は本機能導入前の layout と byte 等価（非回帰の構造保証・R4.1/4.3/8.3——
    ///     `SegmentPlan` を一切参照しないため境界値の算出自体が起きない・R4.2）。
    ///   - [`WrapPlan::Segmented`]: 塊先決——塊先頭で塊全体の advance 合計を、plan の塊の
    ///     範囲と渡された `items` から求め、残り行幅（`cap_rem`）に収まれば継続配置、行頭からの行幅（`cap_full`）まで
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
    /// 塊先決は `visible_count` に依存しない（seg_sum は渡された `items` の全部から算出・INV-1/7.1）。
    /// 渡された字の列と plan が同じなら、見える数を増やしても前の字の行は動かない。字が届くたびに
    /// 字の列そのものが伸びる呼び方（届いた字の列で区切って渡す）ではこの性質は成り立たないので、
    /// 分かち書きの折返しの呼び手は区間の全文を渡す（モジュール doc「可視 prefix 規則」）。
    /// ゲート①が④より先にあるため、塊途中で可視が切れても配置済み prefix の行は動かない
    /// （INV-2/7.2/7.3）。塊前行送りは行頭では `cap_rem == cap_full` ゆえ不発火＝空行を
    /// 作らない（INV-3）。縦書きは行内軸の `inline_pos`/`advance`/折返し基準/遠辺の演算のみゆえ
    /// 新規 mode 分岐なし（6.1/6.2・遠辺の軸解決は [`TextRegion`] が済ませている）。
    ///
    /// 同一入力→同一出力（R2.5 系）。失敗経路なし（全入力で値を返す純関数）。
    ///
    /// **本番の呼び手は [`layout_styled`](Self::layout_styled) へ移った**（`actor_present.rs` の
    /// `arrange_lines` が唯一の本番呼出点で、`present_actor` はそれを呼ぶ）。本関数の本番の呼び手は 0 で、残しているのは
    /// 非回帰の檻（`layout_styled_tests.rs` が「装飾なしの出力が装飾導入前と 1 ビットも
    /// 変わらない」を本関数の出力と突き合わせる）をはじめとする多数の決定論テスト
    /// （`canvas.rs`／`layout_*_tests.rs`／`viewbox_draw_*_tests.rs`／`draw_oracle_tests.rs`／
    /// `tests/` の統合試験）が呼ぶ委譲の入口としてである。crate 外では
    /// `crates/areka-emo-text/examples/emo-text-layer/drive.rs` が 2 か所で呼ぶ（本番ではない）。
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
            None,
        )
    }

    /// [`layout`](Self::layout) の全挙動に加え、`\_l` の 2 縮退分岐（解釈不能／中央指定の
    /// 軸取り違え）を **キャラクターごと初回のみ** `warn!` する（R5.3・design 縮退表）。
    ///
    /// 負値絶対・`%`・`@` 相対は縮退ではなく**実導出**なので警告の対象ではない（R5.2）。
    ///
    /// warn guard は走査を跨いで持続する必要がある（per-frame layout 呼出での重複警告抑止）
    /// ため、ランタイム（`actor.rs` の `TextLayerRuntime::cursor_warn`・既存 `unresolved_warned` と
    /// 同型の持続 guard）が所有し `&mut` で渡す。型の住処は解決層
    /// [`crate::cursor_tag::CursorWarnGuard`] で、本 API の署名は変わらない。行レイアウトの
    /// 純挙動は [`layout`](Self::layout) と完全同一——差は縮退ログの有無のみ（guard は決定的な
    /// 行出力に一切影響しない）。
    ///
    /// **本番の呼び手は [`layout_styled`](Self::layout_styled) へ移った**。ランタイムが持つ
    /// guard は `present_actor` から `arrange_lines` を経て `layout_styled` の引数として渡っており、本関数を経由
    /// **しない**（guard の所有者は変わらないが、受け取る関数は 1 段先である）。本関数の
    /// 本番の呼び手は 0 で、残しているのは非回帰の檻（`layout_cursor_tests.rs`／
    /// `layout_cursor_wiring_tests.rs`）が呼ぶ委譲の入口としてである。
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
            None,
        )
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
    /// **先頭可視行の開始側と描画範囲の開始側の差**（送りの原点は描画範囲の開始側であって、
    /// 最初の行の開始側ではない——D17／R17.1）。原点を最初の行に取ると、台詞冒頭の保留改行
    /// （`\n[150]` 等）で空いた分が候補に入らず永久に送られないため、収まるはずの行まで
    /// 送り出されて最新の 1 行だけが残る（症状 F）。冒頭に空きの無い入力では
    /// `near(&lines[0])` が描画範囲の開始側に等しく、値は旧規則（スキップした行の位置差）と
    /// 一致する。全行超過でも最新行へ飽和する（最新行は常に可視・行を失わない）。
    /// 失敗経路なし（全入力で値を返す純関数）。
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
        // 送り量の原点は**描画範囲の開始側**（`layout` と同じ軸読み替え正準表で正規化ブロック
        // 座標へ写す）。最初の行の開始側ではない——冒頭の保留改行で空いた分を送りの候補に
        // 含めるためである（D17／R17.1・症状 F）。空きの無い入力では両者は同値。
        let start = region.start();
        let origin = match mode {
            WritingMode::HorizontalTb => start.1,
            WritingMode::VerticalRl => -start.0,
            WritingMode::VerticalLr => start.0,
        };
        // 行単位スクロール: 最新行が収まる最小スキップ数を探す（全行超過は最新行へ飽和）。
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

#[cfg(test)]
#[path = "layout_cluster_tests.rs"]
mod cluster_tests;
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

#[cfg(test)]
#[path = "layout_styled_tests.rs"]
mod styled_tests;
