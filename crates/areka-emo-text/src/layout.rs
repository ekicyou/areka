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
//! 折返し・改行後の行は、描画開始点（origin クランプ正準）の**行内軸成分**へ戻る
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

use crate::region::TextRegion;
use crate::segment::SegmentPlan;
use crate::state::{TextItem, TextLayerConfig};
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
}

/// 構造テスト用の決定論 metrics（R4.5/R11.6）。
///
/// 決定論仮想値: 全角（非 ASCII）＝`font_height`・半角（ASCII）＝`font_height / 2`。
/// 行送りピッチは M1 正準式 `ceil(font_height × 既定係数 1.25)`。
/// タイポグラフィ的正確さは目的でない——折返し・行送りアルゴリズムの檻のための値。
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedMetrics;

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
                    // ② 保留改行の一括実体化（次の可視コンテンツ配置の直前・R2.1）。
                    // current 非空なら現在行を確定してから block を Σratio ぶん前進させる。
                    // 先頭改行（current 空）は空行を作らず前進のみ（DD-2）。
                    if let Some(sum) = pending.take() {
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
                        block_pos += block_dir * pitch * sum;
                        inline_pos = inline_start;
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
                TextItem::CursorMove { .. } => {
                    // カーソル指定は非グリフ（reveal 対象外）。実配置（pending-cursor 遅延実体化＝
                    // 現在行確定→保留改行 Σratio→カーソル指定軸上書き）は LayoutCursor タスク
                    // （layout.rs 増分・design.md「純粋層 / LayoutCursor」）の領分。本 P0 増分では
                    // 行レイアウトへ影響させない no-op プレースホルダとして走査を素通しする。
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
    let rect = match mode {
        WritingMode::HorizontalTb => LineRect {
            left: inline_start,
            top: block_pos,
            right: inline_end,
            bottom: block_pos + font_height,
        },
        WritingMode::VerticalRl => LineRect {
            left: block_pos - font_height,
            top: inline_start,
            right: block_pos,
            bottom: inline_end,
        },
        WritingMode::VerticalLr => LineRect {
            left: block_pos,
            top: inline_start,
            right: block_pos + font_height,
            bottom: inline_end,
        },
    };
    PositionedLine { rect, glyphs }
}

#[cfg(test)]
mod tests {
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    use super::{
        FixedMetrics, GlyphMetrics, LayoutEngine, LineRect, PositionedLine, VisibleWindow, WrapPlan,
    };
    use crate::region::TextRegion;
    use crate::segment::{Segment, SegmentPlan};
    use crate::state::TextItem;
    use crate::writing::WritingMode;

    /// テスト画像原寸（image px・region.rs の檻と同一値）。
    const IMAGE: (u32, u32) = (400, 224);

    /// テスト用 BalloonModel 生成ヘルパ（幾何成分だけ指定）。
    fn model(
        origin: (Option<i32>, Option<i32>),
        wordwrap: (Option<i32>, Option<i32>),
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(origin.0, origin.1),
            WordWrapPoint::new(wordwrap.0, wordwrap.1),
            ValidRect::new(None, None, None, None),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
            None,
        )
    }

    /// n 個の全角グリフ（'あ'）item 列。
    fn glyphs(n: usize) -> Vec<TextItem> {
        std::iter::repeat_n(TextItem::Glyph { ch: 'あ' }, n).collect()
    }

    /// 行のグリフ行内位置列を抜き出す。
    fn inline_positions(line: &PositionedLine) -> Vec<f32> {
        line.glyphs.iter().map(|g| g.inline_pos).collect()
    }

    // ── R4.5: FixedMetrics の決定論仮想値（全角=height・半角=height/2・pitch=ceil(×1.25)） ──

    /// 全角（非 ASCII）は font_height・半角（ASCII）は font_height/2。
    #[test]
    fn fixed_metrics_advance_full_width_for_nonascii_half_for_ascii() {
        let m = FixedMetrics;
        assert_eq!(m.advance('あ', 12.0), 12.0);
        assert_eq!(m.advance('a', 12.0), 6.0);
        assert_eq!(m.advance(' ', 10.0), 5.0);
        assert_eq!(m.advance('漢', 10.0), 10.0);
    }

    /// 行送りピッチは ceil(font_height × 1.25)——端数ケースで檻化
    /// （tasks.md Implementation Notes: floor/丸めなし変異を殺す値を選ぶ）。
    #[test]
    fn fixed_metrics_line_pitch_ceils_fractional_values() {
        let m = FixedMetrics;
        assert_eq!(m.line_pitch(12.0), 15.0); // 15.0（割り切れ）
        assert_eq!(m.line_pitch(10.0), 13.0); // 12.5 → 13（ceil でなければ fail）
    }

    // ── R6.1: 横書き——行内 +x・行送り +y・折返し閾値 wordwrappoint.x ──

    /// 横書きの折返し: 行内位置＋次グリフ幅 > 閾値で折返し、
    /// ちょうど閾値で終わるグリフは折返さない（> 判定・境界檻）。
    #[test]
    fn horizontal_wraps_before_glyph_exceeding_threshold() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // 全角 6 グリフ・font 10: 5 個目は送り終端 50＝閾値ちょうど→残留、
        // 6 個目は 50+10=60 > 50 → 折返し。
        let lines = LayoutEngine::layout(
            &glyphs(6),
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(
            inline_positions(&lines[0]),
            vec![0.0, 10.0, 20.0, 30.0, 40.0]
        );
        assert_eq!(inline_positions(&lines[1]), vec![0.0]);
        // 行矩形: 行内範囲＝開始〜送り終端・行送り軸は +y へ pitch(13) 進む。
        assert_eq!(
            lines[0].rect,
            LineRect {
                left: 0.0,
                top: 0.0,
                right: 50.0,
                bottom: 10.0
            }
        );
        assert_eq!(
            lines[1].rect,
            LineRect {
                left: 0.0,
                top: 13.0,
                right: 10.0,
                bottom: 23.0
            }
        );
    }

    /// 半角/全角混在の送り幅が累積する（グリフ別 advance の出力整合・R9.4）。
    #[test]
    fn horizontal_mixed_width_advances_accumulate() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'a' },
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'b' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 6.0, 18.0]);
        let advances: Vec<f32> = lines[0].glyphs.iter().map(|g| g.advance).collect();
        assert_eq!(advances, vec![6.0, 12.0, 6.0]);
        assert_eq!(lines[0].rect.right, 24.0);
    }

    // ── R6.2: 日本語縦書き（vertical_rl）——行内 +y・行送り −x・閾値 wordwrappoint.y ──

    /// vertical_rl: 折返しは y 閾値・列は右→左へ pitch 分進む・列矩形は左方向へ厚み。
    #[test]
    fn vertical_rl_wraps_on_y_threshold_and_feeds_leftward() {
        let region = TextRegion::resolve(
            &model((None, None), (None, Some(30))),
            IMAGE,
            WritingMode::VerticalRl,
        );
        // 書字開始角＝validrect 右上 (400, 0)。全角 5 グリフ・font 10:
        // 3 個目は送り終端 30＝閾値ちょうど→残留、4 個目で折返し。
        assert_eq!(region.start(), (400.0, 0.0));
        let lines = LayoutEngine::layout(
            &glyphs(5),
            5,
            &region,
            WritingMode::VerticalRl,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0, 10.0]);
        // 列 0: x 帯 [390,400]・y 0..30。列 1: block 400−13=387 → x 帯 [377,387]。
        assert_eq!(
            lines[0].rect,
            LineRect {
                left: 390.0,
                top: 0.0,
                right: 400.0,
                bottom: 30.0
            }
        );
        assert_eq!(
            lines[1].rect,
            LineRect {
                left: 377.0,
                top: 0.0,
                right: 387.0,
                bottom: 20.0
            }
        );
    }

    // ── R6.3: vertical_lr——行内軸は rl と同一・行送りだけ +x（単一読み替え規則） ──

    /// vertical_lr: 折返し・行内位置は vertical_rl と完全一致し、
    /// 列送りの向きだけが +x へ反転する（軸読み替えの単一規則の檻）。
    #[test]
    fn vertical_lr_mirrors_rl_inline_layout_and_feeds_rightward() {
        let model = model((None, None), (None, Some(30)));
        let region_lr = TextRegion::resolve(&model, IMAGE, WritingMode::VerticalLr);
        // 書字開始角＝validrect 左上 (0, 0)。
        assert_eq!(region_lr.start(), (0.0, 0.0));
        let lines_lr = LayoutEngine::layout(
            &glyphs(5),
            5,
            &region_lr,
            WritingMode::VerticalLr,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines_lr.len(), 2);
        assert_eq!(
            lines_lr[0].rect,
            LineRect {
                left: 0.0,
                top: 0.0,
                right: 10.0,
                bottom: 30.0
            }
        );
        assert_eq!(
            lines_lr[1].rect,
            LineRect {
                left: 13.0,
                top: 0.0,
                right: 23.0,
                bottom: 20.0
            }
        );
        // 行内軸の配置は vertical_rl と同一（読み替え規則は 1 つ・分岐なし）。
        let region_rl = TextRegion::resolve(&model, IMAGE, WritingMode::VerticalRl);
        let lines_rl = LayoutEngine::layout(
            &glyphs(5),
            5,
            &region_rl,
            WritingMode::VerticalRl,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        for (lr, rl) in lines_lr.iter().zip(&lines_rl) {
            assert_eq!(inline_positions(lr), inline_positions(rl));
        }
    }

    // ── 改行マーカー（NewLine{ratio}）: 行送り量 = line_pitch × ratio（正準表） ──

    /// 明示改行の ratio（1.0／0.5）が行送り量へ反映される
    /// （font 12 → pitch 15: 行送り軸位置 0 → 15 → 22.5・端数檻）。
    #[test]
    fn explicit_line_break_ratio_scales_line_feed() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 3);
        let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
        assert_eq!(tops, vec![0.0, 15.0, 22.5]);
        assert!(lines.iter().all(|l| l.glyphs.len() == 1));
    }

    /// 縦書きでも改行マーカーは列送り（−x）として同じ規則で効く（正準表「同（列送り）」）。
    #[test]
    fn vertical_line_break_feeds_column_axis() {
        let region = TextRegion::resolve(
            &model((None, None), (None, None)),
            IMAGE,
            WritingMode::VerticalRl,
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        let lines = LayoutEngine::layout(
            &items,
            2,
            &region,
            WritingMode::VerticalRl,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2);
        // pitch 15 × 0.5 = 7.5 だけ左へ: 列 1 の右辺 = 400 − 7.5。
        assert_eq!(lines[0].rect.right, 400.0);
        assert_eq!(lines[1].rect.right, 392.5);
    }

    // ── 縮退・境界 ──

    /// 行頭の 1 グリフが閾値超過でも配置される（無限折返しなし・グリフを落とさない）。
    #[test]
    fn single_glyph_exceeding_threshold_is_placed_per_line() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(3), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let lines = LayoutEngine::layout(
            &glyphs(2),
            2,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(
            lines.len(),
            2,
            "1 行 1 グリフで前進する（無限ループしない）"
        );
        assert_eq!(inline_positions(&lines[0]), vec![0.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0]);
    }

    /// 空入力は空の行列（行なし）。可視 0 のグリフ列も行を生まない。
    #[test]
    fn empty_input_and_zero_visible_yield_no_lines() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let empty = LayoutEngine::layout(
            &[],
            0,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert!(empty.is_empty());
        let unrevealed = LayoutEngine::layout(
            &glyphs(3),
            0,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert!(unrevealed.is_empty());
    }

    /// 可視 prefix: visible_count 個のグリフだけが配置され、超過指定は全量へ飽和する。
    #[test]
    fn visible_count_gates_placed_glyphs() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items: Vec<TextItem> = "abcde".chars().map(|ch| TextItem::Glyph { ch }).collect();
        let partial = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(partial.len(), 1);
        assert_eq!(inline_positions(&partial[0]), vec![0.0, 6.0, 12.0]);
        let saturated = LayoutEngine::layout(
            &items,
            99,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(saturated[0].glyphs.len(), 5);
    }

    /// 可視 prefix 内の改行マーカーは遅延（deferred newline）: その後ろに可視グリフが
    /// 現れるまで行を開かず保留する（R4.2）。次の可視グリフが reveal された時点でのみ
    /// 一括実体化する（R4.1）——保留中は空行を出さない（R1.2）。
    #[test]
    fn line_break_defers_until_next_visible_glyph() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'b' },
        ];
        // visible=1: b が未リビール——改行は保留のまま行を開かない（R4.2）。
        let held = LayoutEngine::layout(
            &items,
            1,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(held.len(), 1, "保留改行は行を開かない（空行を出さない）");
        assert_eq!(held[0].glyphs.len(), 1);
        assert_eq!(held[0].glyphs[0].ch, 'a');
        // visible=2: b がリビール——保留改行が実体化して 2 行になる（R4.1）。
        let materialized = LayoutEngine::layout(
            &items,
            2,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(materialized.len(), 2, "次可視グリフ配置で保留改行が実体化");
        assert_eq!(materialized[0].glyphs[0].ch, 'a');
        assert_eq!(materialized[1].glyphs[0].ch, 'b');
        // 実体化後の 2 行目: 行内 0 起点（b は ASCII で advance 6）・行送り軸位置は
        // pitch(15) 分進む・中間空行は生じない。
        assert_eq!(
            materialized[1].rect,
            LineRect {
                left: 0.0,
                top: 15.0,
                right: 6.0,
                bottom: 27.0
            }
        );
    }

    /// 末尾改行（後続の可視グリフを持たない）は保留のまま蒸発する——空行を開かない
    /// （R1.1/1.2/5.2）。A→B 切替で A の末尾段落区切りが痕跡を残さない核心。
    #[test]
    fn trailing_line_break_defers_and_evaporates() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
        ];
        let lines = LayoutEngine::layout(
            &items,
            1,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 1, "末尾保留改行は蒸発＝空行を開かない");
        assert_eq!(lines[0].glyphs.len(), 1);
    }

    // ── 遅延意味論（deferred newline・newline-defer）の判断分岐（FixedMetrics 全網羅） ──

    /// 3.1: 連続する複数の改行マーカーは単一の累算保留として実体化され、中間に空行が生じず
    /// 行間が ratio 合計（`pitch × Σratio`）になる。実体化後の後続グリフは通常配置（pending
    /// 消費済み）。`[a, \n, \n(0.5), b, c]` → 2 行（a／b c・間隔 pitch×1.5）。
    #[test]
    fn consecutive_newlines_accumulate_into_single_flush() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'b' },
            TextItem::Glyph { ch: 'c' },
        ];
        // font 12 → pitch 15。累算 Σratio=1.5 → 行送り 22.5。
        let lines = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2, "連続改行は単一累算＝中間空行なし");
        let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
        assert_eq!(tops, vec![0.0, 22.5], "行間 = pitch(15) × Σratio(1.5)");
        assert_eq!(lines[0].glyphs.len(), 1, "行 0 は a のみ");
        assert_eq!(
            lines[1].glyphs.len(),
            2,
            "行 1 は b c（pending 消費済みで通常配置）"
        );
    }

    /// 3.2: 先頭改行（可視グリフ配置前の保留）は空行を作らず block 前進のみで実体化する
    /// （DD-2）・`ratio 0` の改行は「送りゼロで行を替える」縮退挙動を保つ（DD-5）・改行
    /// マーカーのみの入力（可視グリフなし）は 0 行を返し内容ビューボックスを変化させない（1.5）。
    #[test]
    fn leading_newline_zero_ratio_and_newline_only_input() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // (a) 先頭改行 `[\n, a]` → 1 行・block 位置 start + pitch(15)・空行なし。
        let leading = [
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'a' },
        ];
        let lines = LayoutEngine::layout(
            &leading,
            1,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 1, "先頭改行は空行を作らない");
        assert_eq!(lines[0].rect.top, 15.0, "block 位置は start + pitch へ前進");
        assert_eq!(lines[0].glyphs.len(), 1);

        // (b) ratio 0 `[a, \n(0), b]` → 2 行・同一 block 位置（行替えのみ・送りゼロ）。
        let zero = [
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 0.0 },
            TextItem::Glyph { ch: 'b' },
        ];
        let zlines = LayoutEngine::layout(
            &zero,
            2,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(zlines.len(), 2, "ratio 0 でも行を替える");
        let ztops: Vec<f32> = zlines.iter().map(|l| l.rect.top).collect();
        assert_eq!(ztops, vec![0.0, 0.0], "送りゼロ＝同一 block 位置");

        // (c) 改行のみ `[\n, \n]`（可視グリフなし）→ 0 行（ビューボックス不変の構造的証明）。
        let only = [
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::LineBreak { ratio: 1.0 },
        ];
        let empty = LayoutEngine::layout(
            &only,
            0,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert!(
            empty.is_empty(),
            "改行のみの入力は 0 行（占有範囲を作らない）"
        );
    }

    /// 3.3: typewriter リビール進行との整合。同一 items で可視グリフ数を段階的に増やすと、
    /// 改行マーカーの実体化はその改行より後ろの可視グリフが現れた時点でのみ起きる（R4.1/4.2）。
    /// `[a, \n, b, \n, c]` の可視 1→2→3 で行数 1→2→3。
    #[test]
    fn reveal_progression_materializes_only_when_next_glyph_appears() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = [
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'b' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'c' },
        ];
        // 可視 v のとき行数 v（改行はその後ろのグリフが reveal された時点でのみ実体化）。
        for visible in 1..=3 {
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &region,
                WritingMode::HorizontalTb,
                12.0,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            assert_eq!(
                lines.len(),
                visible,
                "可視 {visible}: 改行の実体化は後続グリフの reveal 時のみ（保留は行を開かない）"
            );
            // 末尾行は必ず可視グリフを含む（空行を出さない Postcondition 1）。
            assert!(lines.iter().all(|l| !l.glyphs.is_empty()));
        }
    }

    /// 3.4: 満杯付近で保留改行が実体化された直後にあふれ判定が従来どおり発火する（3.2/7.3 後段）。
    /// 満杯 3 行＋`\n`＋**次の可視グリフ**（実体化トリガあり）→ 4 行目が実体化しあふれ発火。
    /// 保留のみ（次グリフなし）で不発火である対の片側は `trailing_pending_newline_does_not_
    /// trigger_overflow`（既存更新檻）が担保する。
    #[test]
    fn materialized_newline_near_full_triggers_overflow() {
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // 満杯 3 行（下端 10/23/36）＋改行＋次グリフ → 改行が実体化し 4 行目（下端 49 > 36）。
        let mut items = broken_lines(3);
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.push(TextItem::Glyph { ch: 'あ' });
        let window = window_for(&items, &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: -13.0
            },
            "実体化後は従来どおりあふれ発火（次グリフが保留改行を実体化）"
        );
    }

    /// 3.5: 縦書き（vertical_rl／vertical_lr）でも横書きと同一の遅延・累算・実体化・蒸発規則が
    /// 成立し、行送りが軸読み替え正準表（`block_dir × pitch × Σratio`）に従う（R6.1/6.2）。
    #[test]
    fn deferred_rules_hold_in_vertical_modes() {
        // (a) vertical_rl 累算実体化: `[あ, \n, \n(0.5), あ]` → 2 列・列送り −x で Σratio 1.5。
        let region_rl = TextRegion::resolve(
            &model((None, None), (None, None)),
            IMAGE,
            WritingMode::VerticalRl,
        );
        let acc = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        // font 12 → pitch 15。書字開始角 x=400。列 1 の右辺 = 400 − 15×1.5 = 377.5。
        let lines = LayoutEngine::layout(
            &acc,
            2,
            &region_rl,
            WritingMode::VerticalRl,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2, "縦書きでも連続改行は単一累算（中間列なし）");
        assert_eq!(lines[0].rect.right, 400.0);
        assert_eq!(
            lines[1].rect.right, 377.5,
            "列送り = block_dir × pitch × Σratio"
        );

        // (b) 縦書き 2 方向で trailing 改行は蒸発する（`[あ, \n]` → 1 列）。
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            let region = TextRegion::resolve(&model((None, None), (None, None)), IMAGE, mode);
            let trailing = [
                TextItem::Glyph { ch: 'あ' },
                TextItem::LineBreak { ratio: 1.0 },
            ];
            let t = LayoutEngine::layout(
                &trailing,
                1,
                &region,
                mode,
                12.0,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            assert_eq!(t.len(), 1, "{mode:?}: 末尾保留改行は蒸発（空列なし）");
        }
    }

    /// 3.6: 決定論（同一入力→同一出力）に、連続改行・末尾改行を含む新しい入力パターンを
    /// 追加し、遅延意味論の全分岐が 3 方向で同一入力→同一出力を返すことを確認する（7.1）。
    #[test]
    fn deferred_semantics_same_input_yields_identical_output() {
        let model = model((Some(0), Some(0)), (Some(50), None));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let region = TextRegion::resolve(&model, IMAGE, mode);
            // 連続改行＋末尾改行を含む列（遅延・累算・実体化・蒸発の全分岐を通す）。
            let items = [
                TextItem::LineBreak { ratio: 1.0 },
                TextItem::Glyph { ch: 'あ' },
                TextItem::LineBreak { ratio: 1.0 },
                TextItem::LineBreak { ratio: 0.5 },
                TextItem::Glyph { ch: 'a' },
                TextItem::LineBreak { ratio: 1.0 },
            ];
            let first = LayoutEngine::layout(
                &items,
                2,
                &region,
                mode,
                10.0,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            let second = LayoutEngine::layout(
                &items,
                2,
                &region,
                mode,
                10.0,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            assert_eq!(
                first, second,
                "mode {mode:?} で遅延意味論の決定論が崩れている"
            );
        }
    }

    /// 折返し・改行後の行は描画開始点の行内成分へ戻る（origin が validrect 内部の場合も
    /// 同一規則・行送り軸だけが進む）。
    #[test]
    fn wrapped_lines_return_to_start_inline_component() {
        let region = TextRegion::resolve(
            &model((Some(100), Some(50)), (Some(150), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        assert_eq!(region.start(), (100.0, 50.0));
        // font 10: 100,110,120,130,140（5 個目終端 150＝閾値ちょうど）→ 6 個目で折返し。
        let lines = LayoutEngine::layout(
            &glyphs(6),
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(inline_positions(&lines[1]), vec![100.0]);
        assert_eq!(lines[1].rect.top, 63.0); // 50 + pitch 13
    }

    // ── R2.5 系/R11.6: 決定論（同一入力→同一出力・DirectWrite 非依存の構造テスト） ──

    // ── 3.2 R7.1/7.2/7.4/7.5（+R6.4）: あふれ判定とスクロール可視窓（visible_window） ──
    //
    // 幾何の共通前提: FixedMetrics・font 10 → pitch 13（ceil(12.5)）・全角 1 グリフ/行。
    // あふれ判定は軸読み替え正準表の行（横書き=最新行の下端 > validrect.bottom・
    // vertical_rl=最新列の左端 < validrect.left・vertical_lr=最新列の右端 > validrect.right）。

    /// テスト用 BalloonModel 生成ヘルパ（validrect 込み・順序は top,bottom,left,right）。
    fn model_rect(
        origin: (Option<i32>, Option<i32>),
        validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(origin.0, origin.1),
            WordWrapPoint::new(None, None),
            ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
            None,
        )
    }

    /// n 行（各行 全角 1 グリフ・明示改行 ratio 1.0 区切り）の item 列。
    fn broken_lines(n: usize) -> Vec<TextItem> {
        let mut items = Vec::new();
        for i in 0..n {
            if i > 0 {
                items.push(TextItem::LineBreak { ratio: 1.0 });
            }
            items.push(TextItem::Glyph { ch: 'あ' });
        }
        items
    }

    /// layout→visible_window の通し（テスト用最短経路・visible は全量）。
    fn window_for(
        items: &[TextItem],
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
    ) -> VisibleWindow {
        let visible = items
            .iter()
            .filter(|i| matches!(i, TextItem::Glyph { .. }))
            .count();
        let lines = LayoutEngine::layout(
            items,
            visible,
            region,
            mode,
            font_height,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        LayoutEngine::visible_window(&lines, region, mode)
    }

    /// 領域内に収まる間はあふれ非発火（先頭可視行 0・オフセット 0）。
    /// 最新行の下端が validrect.bottom とちょうど一致する境界は「超えていない」
    /// （判定は `>`・境界檻）。
    #[test]
    fn horizontal_within_region_does_not_scroll() {
        // validrect top0/bottom36: 3 行の下端 10/23/36——3 行目はちょうど 36。
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let window = window_for(&broken_lines(3), &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 0,
                block_offset: 0.0
            }
        );
    }

    /// 横書きのあふれは縦スクロール（R7.2）: 1 行超過で先頭可視行が 1 行進み、
    /// 内容は上（−y）へ pitch 分オフセットする。行単位＝オフセットは行位置差そのもの。
    #[test]
    fn horizontal_overflow_scrolls_vertically_by_whole_lines() {
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // 4 行目の下端 49 > 36 → 1 行スキップで 49-13=36 ≤ 36（最小スキップ檻）。
        let one_over = window_for(&broken_lines(4), &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(
            one_over,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: -13.0
            }
        );
        // 6 行（最新行下端 75）→ 3 行スキップ（75-39=36）・オフセット −39。
        let three_over = window_for(&broken_lines(6), &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(
            three_over,
            VisibleWindow {
                first_visible_line: 3,
                block_offset: -39.0
            }
        );
    }

    /// vertical_rl のあふれは横スクロール（R7.2）: 最新列の左端 < validrect.left で発火し、
    /// 内容は右（+x）へオフセットする（古い列が右端から消える——正準表）。
    #[test]
    fn vertical_rl_overflow_scrolls_content_rightward() {
        // validrect left360/right400。列の左端 390/377/364/351——4 列目 351 < 360。
        let region = TextRegion::resolve(
            &model_rect((None, None), (Some(0), Some(224), Some(360), Some(400))),
            IMAGE,
            WritingMode::VerticalRl,
        );
        assert_eq!(region.start(), (400.0, 0.0));
        let fits = window_for(&broken_lines(3), &region, WritingMode::VerticalRl, 10.0);
        assert_eq!(fits.first_visible_line, 0);
        assert_eq!(fits.block_offset, 0.0);
        let over = window_for(&broken_lines(4), &region, WritingMode::VerticalRl, 10.0);
        assert_eq!(
            over,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: 13.0
            }
        );
    }

    /// vertical_lr のあふれは横スクロール: 最新列の右端 > validrect.right で発火し、
    /// 内容は左（−x）へオフセットする（正準表）。
    #[test]
    fn vertical_lr_overflow_scrolls_content_leftward() {
        // validrect left0/right40。列の右端 10/23/36/49——4 列目 49 > 40。
        let region = TextRegion::resolve(
            &model_rect((None, None), (Some(0), Some(224), Some(0), Some(40))),
            IMAGE,
            WritingMode::VerticalLr,
        );
        let fits = window_for(&broken_lines(3), &region, WritingMode::VerticalLr, 10.0);
        assert_eq!(fits.first_visible_line, 0);
        assert_eq!(fits.block_offset, 0.0);
        let over = window_for(&broken_lines(4), &region, WritingMode::VerticalLr, 10.0);
        assert_eq!(
            over,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: -13.0
            }
        );
    }

    /// 空の行列は既定窓（先頭 0・オフセット 0）——失敗経路なしの純関数。
    #[test]
    fn empty_lines_yield_default_window() {
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let window = LayoutEngine::visible_window(&[], &region, WritingMode::HorizontalTb);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 0,
                block_offset: 0.0
            }
        );
    }

    /// 全行超過（どこまでスキップしても最新行が収まらない）は最新行へ飽和する
    /// （最新行は常に可視・行を失わない縮退規則）。
    #[test]
    fn all_lines_overflowing_saturates_to_newest_line() {
        // font 50 → pitch 63・行下端 50/113/176 は全て validrect.bottom 40 超過。
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(40), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let window = window_for(&broken_lines(3), &region, WritingMode::HorizontalTb, 50.0);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 2,
                block_offset: -126.0
            }
        );
        // 1 行だけで領域より厚い場合も先頭 0・オフセット 0（それ以上戻せない）。
        let single = window_for(&broken_lines(1), &region, WritingMode::HorizontalTb, 50.0);
        assert_eq!(
            single,
            VisibleWindow {
                first_visible_line: 0,
                block_offset: 0.0
            }
        );
    }

    /// ratio 付き改行の端数行送り（pitch 15 × 0.5 = 7.5）でもオフセットは
    /// 実際の行位置差＝端数そのもの（整数量子化しない・端数檻）。
    #[test]
    fn fractional_ratio_feed_scrolls_by_fractional_line_distance() {
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(30), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // font 12 → pitch 15。ratio 0.5 区切り 4 行: 上端 0/7.5/15/22.5・下端 …/34.5 > 30。
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'あ' },
        ];
        let window = window_for(&items, &region, WritingMode::HorizontalTb, 12.0);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: -7.5
            }
        );
    }

    /// 末尾の保留改行はあふれ判定に参加しない（内容ビューボックスを増やさない・
    /// R3.1/5.3/7.3 前段）。満杯 3 行（ちょうど収まる）＋trailing `\n` → 保留のまま
    /// 蒸発しあふれ不発火（`first_visible_line=0`）。新規檻 5（実体化後発火）と対を成す。
    #[test]
    fn trailing_pending_newline_does_not_trigger_overflow() {
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // 3 行（下端 10/23/36——ちょうど収まる）＋末尾改行は保留のまま蒸発＝空 4 行目を
        // 開かないためあふれ入力に現れない。
        let mut items = broken_lines(3);
        items.push(TextItem::LineBreak { ratio: 1.0 });
        let window = window_for(&items, &region, WritingMode::HorizontalTb, 10.0);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 0,
                block_offset: 0.0
            },
            "保留改行はあふれ判定に不参加（スクロール不発火）"
        );
    }

    /// 同一入力に対する visible_window 出力は完全一致する（純関数・決定論檻・R7.5）。
    #[test]
    fn visible_window_same_input_yields_identical_output() {
        let region = TextRegion::resolve(
            &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let lines = LayoutEngine::layout(
            &broken_lines(5),
            5,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let first = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
        let second = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
        assert_eq!(first, second);
    }

    /// 同一入力に対する layout 出力は完全一致する（純関数・決定論檻）。
    #[test]
    fn same_input_yields_identical_output() {
        let model = model((Some(0), Some(0)), (Some(50), None));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let region = TextRegion::resolve(&model, IMAGE, mode);
            let items = [
                TextItem::Glyph { ch: 'あ' },
                TextItem::LineBreak { ratio: 0.5 },
                TextItem::Glyph { ch: 'a' },
            ];
            let first = LayoutEngine::layout(
                &items,
                2,
                &region,
                mode,
                10.0,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            let second = LayoutEngine::layout(
                &items,
                2,
                &region,
                mode,
                10.0,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            assert_eq!(first, second, "mode {mode:?} で決定論が崩れている");
        }
    }

    // ── Task 4.1: 塊先決による折返し（WrapPlan::Segmented・手組み SegmentPlan 注入） ──
    //
    // budouy 非依存でゲート③の判断分岐を全網羅するため、plan は手組み（from_segments）で
    // 注入する（design「テスト形: 手組み SegmentPlan」）。共通前提: FixedMetrics・font 10 →
    // 全角 'あ' の advance 10・pitch 13。閾値は wordwrappoint x（横書き）／y（縦書き）。

    /// (start, len) 列から手組み SegmentPlan を作る。
    fn plan(segs: &[(usize, usize)]) -> SegmentPlan {
        SegmentPlan::from_segments(
            segs.iter()
                .map(|&(start, len)| Segment { start, len })
                .collect(),
        )
    }

    /// 各グリフの (行 index, 行内位置, 文字) を配置順に平坦化する（prefix 一致比較用）。
    fn flat_glyphs(lines: &[PositionedLine]) -> Vec<(usize, f32, char)> {
        lines
            .iter()
            .enumerate()
            .flat_map(|(li, l)| l.glyphs.iter().map(move |g| (li, g.inline_pos, g.ch)))
            .collect()
    }

    /// 2.1/2.3: 残り行幅に収まる塊は分割せず現在行へ継続配置する（塊内は追加判定なし）。
    /// 行頭の塊 {0,2}（fit）→ 続く塊 {2,2} も残り行幅 30 に収まる → 全 4 グリフが 1 行。
    #[test]
    fn segmented_fits_places_on_current_line_without_split() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(4);
        let p = plan(&[(0, 2), (2, 2)]);
        let lines = LayoutEngine::layout(
            &items,
            4,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(lines.len(), 1, "両塊とも残り行幅に収まる → 1 行に継続配置");
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0, 30.0]);
    }

    /// 2.2: 残り行幅に収まらないが行頭幅には収まる塊は、塊の前で行送りして塊全体を次行頭へ
    /// 移す（途中分割しない）。塊 {4,2}（seg_sum 20）は残り行幅 10 に入らず行頭幅 50 に入る。
    /// CharByChar（5+1 割り）との対比で ON が効いていることを示す。
    #[test]
    fn segmented_not_fit_breaks_before_whole_segment() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(6);
        let p = plan(&[(0, 4), (4, 2)]);
        let seg_lines = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(seg_lines.len(), 2);
        assert_eq!(inline_positions(&seg_lines[0]), vec![0.0, 10.0, 20.0, 30.0]);
        assert_eq!(
            inline_positions(&seg_lines[1]),
            vec![0.0, 10.0],
            "塊 {{4,2}} は分割されず塊ごと次行へ"
        );
        // 対比: CharByChar は 5 グリフ目まで行 0（塊を割る割り方）。
        let char_lines = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(char_lines[0].glyphs.len(), 5);
        assert_ne!(seg_lines, char_lines, "ON（塊維持）が char 割りと異なる");
    }

    /// 2.2/2.3: 境界値の檻（`<=` vs `<`）。塊の advance 合計がちょうど残り行幅に一致すれば
    /// 残留、1 グリフ増えて超えれば塊前で行送り。prefix {0,2}（inline 20・残り行幅 30）に対し、
    /// seg_sum 30（={2,3}）は残留・seg_sum 40（={2,4}）は行送り。
    #[test]
    fn segmented_boundary_exactly_fits_stays_else_breaks() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // ちょうど: seg_sum 30 == cap_rem 30 → 残留（1 行）。
        let items5 = glyphs(5);
        let p_fit = plan(&[(0, 2), (2, 3)]);
        let fit = LayoutEngine::layout(
            &items5,
            5,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p_fit),
        );
        assert_eq!(fit.len(), 1, "seg_sum == cap_rem は残留（<= 判定の境界檻）");
        assert_eq!(inline_positions(&fit[0]), vec![0.0, 10.0, 20.0, 30.0, 40.0]);
        // 1 グリフ増: seg_sum 40 > cap_rem 30 → 塊前で行送り。
        let items6 = glyphs(6);
        let p_over = plan(&[(0, 2), (2, 4)]);
        let over = LayoutEngine::layout(
            &items6,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p_over),
        );
        assert_eq!(over.len(), 2);
        assert_eq!(over[0].glyphs.len(), 2, "行 0 は prefix 塊のみ");
        assert_eq!(over[1].glyphs.len(), 4, "塊 {{2,4}} は分割されず次行へ");
    }

    /// 2.1/2.3（counter）: 先決済み塊の内部では追加判定を通さず配置する（残グリフ数カウンタ）。
    /// threshold 30 で塊 {2,3}（seg_sum 30）が break-before で行 1 頭へ移り、3 グリフが分割
    /// されずに行 1 へ連続配置される。CharByChar は g2 まで行 0（塊を割る）——counter による
    /// 塊維持が char 割りと異なることを示す。
    #[test]
    fn segmented_predecided_segment_is_not_split_inside() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(30), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(5);
        let p = plan(&[(0, 2), (2, 3)]);
        let seg = LayoutEngine::layout(
            &items,
            5,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(seg.len(), 2);
        assert_eq!(seg[0].glyphs.len(), 2);
        assert_eq!(
            inline_positions(&seg[1]),
            vec![0.0, 10.0, 20.0],
            "塊内は追加判定なしで連続配置（3 グリフが行 1 に一体で載る）"
        );
        // 対比: char 規則は g2 まで行 0（3 グリフ）→ 塊が割れる割り方。
        let ch = LayoutEngine::layout(
            &items,
            5,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(ch[0].glyphs.len(), 3);
        assert_ne!(seg, ch, "counter による塊維持が char 割りと異なる");
    }

    /// 4.2: plan に被覆されないグリフ（不整合・長大塊継続）は既存の文字単位規則で配置される
    /// （優しい縮退）。塊 {0,2} のみ被覆・g2..g5 は非被覆 → 出力は純 CharByChar と完全一致。
    #[test]
    fn plan_non_covered_glyphs_fall_back_to_char_rule() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(6);
        let p = plan(&[(0, 2)]); // glyph 2..5 は非被覆
        let seg = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        let ch = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(
            seg, ch,
            "非被覆グリフは既存文字単位規則で配置（plan 不整合の優しい縮退）"
        );
        assert_eq!(seg[0].glyphs.len(), 5, "char 割り（5+1）どおり");
        assert_eq!(seg[1].glyphs.len(), 1);
    }

    /// 7.1/8.3（INV-1/INV-2）: 塊先決は visible_count に依存しない。同一 items+plan で visible を
    /// 段階的に増やすと、各段階の配置は全量出力の先頭 v グリフ（行所属・行内位置とも）に一致する
    /// （リフロー跳び不発生）。核心: g4（塊 {4,2} 先頭）は g5 が不可視でも行 1 へ落ちる
    /// （seg_sum が全文 items から算出されるため——可視部分列で計算する実装なら行 0 に留まり失敗）。
    #[test]
    fn predecision_is_independent_of_visible_count() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(6);
        let p = plan(&[(0, 4), (4, 2)]);
        let full = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        let full_flat = flat_glyphs(&full);
        for v in 0..=6 {
            let partial = LayoutEngine::layout(
                &items,
                v,
                &region,
                WritingMode::HorizontalTb,
                10.0,
                &FixedMetrics,
                WrapPlan::Segmented(&p),
            );
            assert_eq!(
                flat_glyphs(&partial).as_slice(),
                &full_flat[..v],
                "visible {v}: 配置が全量出力の prefix と不一致（リフロー跳び）"
            );
        }
        // 核心: visible 5（g5 不可視）でも g4 は行 1（塊先決は全文由来・INV-1）。
        let v5 = LayoutEngine::layout(
            &items,
            5,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(v5.len(), 2);
        assert_eq!(v5[1].glyphs.len(), 1, "行 1 は g4 のみ（g5 未リビール）");
        assert_eq!(v5[1].glyphs[0].inline_pos, 0.0);
    }

    /// 6.1/6.2: 縦書き（vertical_rl/vertical_lr）でも軸読み替えのみで同一の塊先決式が成立する。
    /// 同一 items+plan で塊 {4,2} が 2 列目へ移り（4+2）、行内軸の先決（塊割れ点・行内位置）が
    /// rl/lr で完全一致する（新規 mode 分岐なし・単一読み替え規則）。
    #[test]
    fn segmented_same_rule_in_vertical_modes() {
        let m = model((None, None), (None, Some(50)));
        let items = glyphs(6);
        let p = plan(&[(0, 4), (4, 2)]);
        let mut inline_by_mode: Vec<Vec<Vec<f32>>> = Vec::new();
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            let region = TextRegion::resolve(&m, IMAGE, mode);
            let lines = LayoutEngine::layout(
                &items,
                6,
                &region,
                mode,
                10.0,
                &FixedMetrics,
                WrapPlan::Segmented(&p),
            );
            assert_eq!(lines.len(), 2, "{mode:?}: 塊 {{4,2}} が 2 列目へ");
            assert_eq!(lines[0].glyphs.len(), 4);
            assert_eq!(lines[1].glyphs.len(), 2, "{mode:?}: 塊は分割されない");
            inline_by_mode.push(lines.iter().map(inline_positions).collect());
        }
        assert_eq!(
            inline_by_mode[0], inline_by_mode[1],
            "縦書き 2 方向で行内軸の塊先決（割れ点・行内位置）が不一致"
        );
    }

    /// 4.1/4.3/8.3: OFF 経路（`WrapPlan::CharByChar`）の非回帰アンカー。既定の折返し
    /// モード（文字単位）は本機能導入前の layout と byte 等価であり、`SegmentPlan` を
    /// 一切参照しない（境界値の算出自体が起きない・R4.2）。閾値 50・font 10 の 6 グリフは
    /// char 割り（5+1）で、行内位置・行矩形まで確定値へ pin する（この確定出力は
    /// `horizontal_wraps_before_glyph_exceeding_threshold` と一致——OFF 出力が全 layout
    /// 呼出で不変であることの明示アンカー）。
    #[test]
    fn char_by_char_is_off_path_non_regression_anchor() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(6);
        let lines = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(lines.len(), 2, "char 割り（5+1）どおり");
        assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0, 30.0, 40.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0]);
        assert_eq!(
            lines[0].rect,
            LineRect {
                left: 0.0,
                top: 0.0,
                right: 50.0,
                bottom: 10.0
            }
        );
        assert_eq!(
            lines[1].rect,
            LineRect {
                left: 0.0,
                top: 13.0,
                right: 10.0,
                bottom: 23.0
            }
        );
    }

    // ── Task 4.2: 長大塊の文字単位縮退（WrapPlan::Segmented・design System Flows H-No→D→C） ──
    //
    // 縮退は `seg_sum > cap_full`（行頭からでも 1 行に収まらない塊）に限って発火し、当該塊のみ
    // 既存の文字単位規則へ委譲する（design「縮退は塊に閉じる」3.3）。共通前提は 4.1 と同じ
    // FixedMetrics・font 10（全角 'あ' advance 10）。

    /// 3.1: 縮退は `seg_sum > cap_full` の塊に限る。長大塊 {0,5}（seg_sum 50 > cap_full 30）は
    /// 文字単位で複数行へ割られ（塊維持でも欠落でもない）、直後の収まる塊 {5,2} は縮退せず塊ごと
    /// 1 行へ載る。threshold 30・font 10。
    #[test]
    fn segmented_degrades_only_when_exceeding_cap_full() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(30), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(7);
        let p = plan(&[(0, 5), (5, 2)]);
        let lines = LayoutEngine::layout(
            &items,
            7,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        // 長大塊 {0,5} は char 規則で 3+2 に割れ（1 行 3 グリフ＝閾値 30／30）・全 5 グリフ配置。
        assert_eq!(lines.len(), 3);
        assert_eq!(
            inline_positions(&lines[0]),
            vec![0.0, 10.0, 20.0],
            "長大塊は char 粒度で複数行へ（塊ごと 1 行に潰さない）"
        );
        assert_eq!(inline_positions(&lines[1]), vec![0.0, 10.0]);
        // 直後の塊 {5,2}（seg_sum 20 ≤ cap_full 30）は縮退せず塊ごと 1 行へ（通常判定）。
        assert_eq!(
            inline_positions(&lines[2]),
            vec![0.0, 10.0],
            "収まる塊は縮退せず塊単位で維持（縮退は > cap_full に限る）"
        );
        // 全 7 グリフが過不足なく配置される（欠落なし）。
        assert_eq!(flat_glyphs(&lines).len(), 7);
    }

    /// 3.2: 縮退中も行頭 1 グリフは閾値超過でも配置し（無限折返し回避）、呼出は必ず停止して
    /// 全グリフを配置する。極小閾値 3・font 10（'あ' advance 10 > threshold）で 1 グリフ/行。
    /// char モードの `single_glyph_exceeding_threshold_is_placed_per_line` を Segmented で鏡写す。
    #[test]
    fn segmented_degrade_places_line_head_glyph_and_terminates() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(3), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(3);
        // 単一長大塊 {0,3}（seg_sum 30 > cap_full 3）→ 当該塊のみ char 規則へ縮退。
        let p = plan(&[(0, 3)]);
        let lines = LayoutEngine::layout(
            &items,
            3,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(
            lines.len(),
            3,
            "閾値超過グリフは 1 行 1 グリフで前進（無限ループしない・停止する）"
        );
        assert_eq!(inline_positions(&lines[0]), vec![0.0]);
        assert_eq!(inline_positions(&lines[1]), vec![0.0]);
        assert_eq!(inline_positions(&lines[2]), vec![0.0]);
        // 全グリフが配置される（行頭 1 グリフ配置規則で 1 個も落ちない）。
        assert_eq!(flat_glyphs(&lines).len(), 3);
    }

    /// 3.3: 縮退は当該塊に閉じ、直後の塊で通常の塊単位判定が再開する。長大塊 {0,4}（seg_sum 40 >
    /// cap_full 30）は char 割り、続く塊 {4,3}（seg_sum 30 == cap_full 30）は縮退を引き継がず塊前
    /// 行送りで塊ごと次行へ載る。char モードなら g4 は行 1 に残る（塊維持でない）——差分が再開の証左。
    #[test]
    fn segmented_degrade_scoped_to_segment_resumes_next() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(30), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let items = glyphs(7);
        let p = plan(&[(0, 4), (4, 3)]);
        let seg = LayoutEngine::layout(
            &items,
            7,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(seg.len(), 3);
        // 長大塊 {0,4} は char 縮退（3+1）。
        assert_eq!(inline_positions(&seg[0]), vec![0.0, 10.0, 20.0]);
        assert_eq!(inline_positions(&seg[1]), vec![0.0], "縮退塊の残り g3");
        // 後続塊 {4,3} は通常判定を再開＝塊ごと行 2 へ（塊単位 break-before・塊維持）。
        assert_eq!(
            inline_positions(&seg[2]),
            vec![0.0, 10.0, 20.0],
            "後続塊は塊単位で維持（縮退が漏れ出していない）"
        );
        // 縮退が漏れれば g4 は行 1（inline 10）に char 継続で載る——そうでないことを対比で固定。
        let ch = LayoutEngine::layout(
            &items,
            7,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(ch[1].glyphs.len(), 3, "char モードは行 1 が [g3,g4,g5]");
        assert_ne!(seg, ch, "後続塊で塊単位判定が再開＝char 全割りと異なる");
    }

    /// 8.3: 極端に長い塊でも全グリフが配置され表示が破綻しない（panic せず停止し無損失）。
    /// 50 グリフ単一塊・threshold 30 → char 縮退で複数行に割れ、全グリフがちょうど一度ずつ現れる。
    #[test]
    fn segmented_extremely_long_segment_places_all_glyphs() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(30), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let n = 50;
        let items = glyphs(n);
        let p = plan(&[(0, n)]);
        let lines = LayoutEngine::layout(
            &items,
            n,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        // 全 50 グリフがちょうど一度ずつ配置される（欠落・重複なし）。
        assert_eq!(flat_glyphs(&lines).len(), n, "全グリフが配置される（無損失）");
        // 各行は行頭 1 グリフ以外は閾値内（3 グリフ/行＝30／30）——はみ出しが構造的に起きない。
        for line in &lines {
            assert!(
                line.glyphs.len() <= 3,
                "1 行に閾値超のグリフが積まれている（縮退が効いていない）: {}",
                line.glyphs.len()
            );
            assert!(!line.glyphs.is_empty(), "空行が生じている");
        }
        // 全グリフが同一文字 'あ'（内容が壊れていない）。
        assert!(lines.iter().all(|l| l.glyphs.iter().all(|g| g.ch == 'あ')));
    }

    // ── Task 4.4: 保留改行との整合とリフロー跳び不発生（WrapPlan::Segmented） ──
    //
    // ゲート順序（①可視打切り→②保留フラッシュ→③折返し判定→④配置）は 4.1 で確立済み。
    // ここではその順序契約の帰結を檻化する: 保留改行の実体化直後は行頭（inline_pos ==
    // inline_start）ゆえ塊先決が「塊先頭かつ残り行幅最大（cap_rem == cap_full）」で走り
    // （design System Flows「保留フラッシュとの順序」5.3）、deferred newline の意味論
    // （遅延・累算・蒸発）は ON でも一切変わらず（5.1/5.2）、typewriter リビール進行の
    // 全段階で配置済みグリフの行が動かない（INV-2・7.2/7.3）。共通前提は 4.1/4.2 と同じ
    // FixedMetrics・font 10（全角 'あ' advance 10・pitch 13）。

    /// 5.3: 保留改行の実体化直後の行頭で塊先決が走る。`[塊A, \n, 塊B, 塊C]`（run2 = 塊B+塊C）で、
    /// 保留改行が run2 を 2 行目行頭へ送り、そこで塊 C が塊ごと 3 行目へワードラップされる
    /// （フラッシュ後の行で塊単位判定が効いている証左）。block 前進は `pitch × Σratio` で OFF と
    /// 同一（char 経路の 2 行目 top と一致）。CharByChar は run2 を char 割りして塊 C を割る。
    #[test]
    fn segmented_predecision_runs_at_line_head_after_pending_flush() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // 塊A(2) / \n / 塊B(3) 塊C(3)。glyph 通し番号は LineBreak を数えない → 塊B は 2、塊C は 5。
        let mut items = glyphs(2);
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.extend(glyphs(6));
        let p = plan(&[(0, 2), (2, 3), (5, 3)]);
        let seg = LayoutEngine::layout(
            &items,
            8,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(seg.len(), 3, "塊A / 塊B / 塊C が 3 行に分かれる");
        // 行 0: 塊A（保留改行の前）。
        assert_eq!(inline_positions(&seg[0]), vec![0.0, 10.0]);
        assert_eq!(seg[0].rect.top, 0.0);
        // 行 1: 塊B が実体化後の行頭に塊ごと載る（塊先決が cap_rem == cap_full で走る）。
        assert_eq!(
            inline_positions(&seg[1]),
            vec![0.0, 10.0, 20.0],
            "塊B は実体化後の行頭に塊ごと配置（塊先決が行頭で先決）"
        );
        assert_eq!(
            seg[1].rect.top, 13.0,
            "block 前進 = pitch(13) × Σratio(1.0)（保留改行の送りは OFF と同一）"
        );
        // 行 2: 塊C は残り行幅（20）に収まらず塊ごと次行へ（フラッシュ後の行で塊単位判定が再開）。
        assert_eq!(
            inline_positions(&seg[2]),
            vec![0.0, 10.0, 20.0],
            "塊C は分割されず塊ごと 3 行目へ（ワードラップが 2 行目以降でも効く）"
        );
        // 対比: CharByChar は同一入力で run2 を char 割り（塊C を割る）＝実体化後の送りは同じでも
        // 折返し粒度が異なる。2 行目 top は両経路で 13（block 前進 = pitch × Σratio は不変）。
        let ch = LayoutEngine::layout(
            &items,
            8,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(
            ch[1].rect.top, 13.0,
            "OFF でも実体化後の block 前進は pitch × Σratio（deferred newline の送りは分岐不変）"
        );
        assert_eq!(ch[1].glyphs.len(), 5, "OFF は run2 を char 割り（5+…）");
        assert_ne!(
            seg, ch,
            "ON はフラッシュ後の行で塊単位ワードラップ＝char 割りと異なる"
        );
    }

    /// 5.1/5.2: deferred newline の意味論（遅延・累算・蒸発）は ON でも不変。ワードラップが
    /// 発火しない（全塊が収まる）入力では Segmented 出力は CharByChar 出力と完全一致する
    /// ——segmentation は改行意味論を変えず、行内の折返し粒度だけを担う。
    /// (a) 末尾保留改行の蒸発（空行なし）・(b) 連続 `\n\n` の単一累算フラッシュ。
    #[test]
    fn deferred_newline_semantics_unchanged_under_segmented() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (None, None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        // (a) 末尾保留改行の蒸発: `[塊A(2), \n]` → 1 行・末尾改行は保留のまま蒸発。
        let mut trailing = glyphs(2);
        trailing.push(TextItem::LineBreak { ratio: 1.0 });
        let p_a = plan(&[(0, 2)]);
        let seg_a = LayoutEngine::layout(
            &trailing,
            2,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p_a),
        );
        let ch_a = LayoutEngine::layout(
            &trailing,
            2,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(seg_a.len(), 1, "ON でも末尾保留改行は蒸発（空行なし）");
        assert_eq!(
            seg_a, ch_a,
            "ワードラップ非発火では ON 出力 = OFF 出力（改行意味論は不変）"
        );

        // (b) 連続 `\n\n(0.5)` の単一累算フラッシュ: `[a, \n, \n(0.5), b, c]` → 2 行・Σratio 1.5。
        // run1="a"(glyph0)・run2="bc"(glyph1,2)。塊は全て収まる → ON = OFF。
        let acc = [
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'b' },
            TextItem::Glyph { ch: 'c' },
        ];
        let p_b = plan(&[(0, 1), (1, 2)]);
        let seg_b = LayoutEngine::layout(
            &acc,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p_b),
        );
        let ch_b = LayoutEngine::layout(
            &acc,
            3,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(seg_b.len(), 2, "ON でも連続改行は単一累算＝中間空行なし");
        let tops: Vec<f32> = seg_b.iter().map(|l| l.rect.top).collect();
        assert_eq!(tops, vec![0.0, 22.5], "行間 = pitch(15) × Σratio(1.5)（ON でも不変）");
        assert_eq!(
            seg_b, ch_b,
            "ワードラップ非発火では連続改行の累算意味論が ON = OFF"
        );
    }

    /// 7.2/7.3（INV-2 の核心）: 保留改行 + ワードラップが共存する入力で、可視グリフ数を 0 から
    /// 段階的に増やしても、各段階の配置は全量出力の先頭 v グリフ（行所属・行内位置とも）に
    /// 常に一致する（配置済みグリフの行が後から動かない＝リフロー跳び不発生）。
    /// `[塊A(2), \n, 塊B(3), 塊C(3)]`・threshold 50 で塊C は残り行幅に入らず塊ごと 3 行目へ。
    /// 核心: visible 6（塊C 先頭 g5 のみ可視・g6/g7 不可視）でも g5 は 3 行目行頭に居る
    /// ——seg_sum が全文 plan から算出されるため。可視部分列で seg_sum を計算する実装なら
    /// g5 は 2 行目末（inline 30）に留まり、後で g6/g7 の出現で 3 行目へ跳んで失敗する。
    #[test]
    fn segmented_prefix_stable_across_pending_flush_and_wrap() {
        let region = TextRegion::resolve(
            &model((Some(0), Some(0)), (Some(50), None)),
            IMAGE,
            WritingMode::HorizontalTb,
        );
        let mut items = glyphs(2);
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.extend(glyphs(6));
        let p = plan(&[(0, 2), (2, 3), (5, 3)]);
        let full = LayoutEngine::layout(
            &items,
            8,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        let full_flat = flat_glyphs(&full);
        assert_eq!(full_flat.len(), 8);
        for v in 0..=8 {
            let partial = LayoutEngine::layout(
                &items,
                v,
                &region,
                WritingMode::HorizontalTb,
                10.0,
                &FixedMetrics,
                WrapPlan::Segmented(&p),
            );
            assert_eq!(
                flat_glyphs(&partial).as_slice(),
                &full_flat[..v],
                "visible {v}: 配置が全量出力の prefix と不一致（リフロー跳び発生）"
            );
        }
        // 核心の明示: visible 6（g5 のみ可視）で g5 は 3 行目行頭（塊C 先決は全文由来・INV-1/INV-2）。
        let v6 = LayoutEngine::layout(
            &items,
            6,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        assert_eq!(v6.len(), 3, "行 0=塊A・行 1=塊B・行 2=塊C 先頭");
        assert_eq!(v6[2].glyphs.len(), 1, "行 2 は g5 のみ（g6/g7 未リビール）");
        assert_eq!(
            v6[2].glyphs[0].inline_pos, 0.0,
            "g5 は 3 行目行頭に先決済み（可視非依存＝リフロー跳びなし）"
        );
    }

    /// 6.1/6.2 × 7.2/7.3: 縦書き（vertical_rl）でも保留改行 + ワードラップ共存下で prefix 安定性が
    /// 成立する（軸読み替えのみ・新規 mode 分岐なし）。横書きと同一 items+plan を vertical_rl で
    /// 段階リビールし、各段階の配置（列所属・行内軸位置）が全量出力の prefix に一致する。
    #[test]
    fn segmented_prefix_stable_in_vertical_mode() {
        let region = TextRegion::resolve(
            &model((None, None), (None, Some(50))),
            IMAGE,
            WritingMode::VerticalRl,
        );
        let mut items = glyphs(2);
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.extend(glyphs(6));
        let p = plan(&[(0, 2), (2, 3), (5, 3)]);
        let full = LayoutEngine::layout(
            &items,
            8,
            &region,
            WritingMode::VerticalRl,
            10.0,
            &FixedMetrics,
            WrapPlan::Segmented(&p),
        );
        let full_flat = flat_glyphs(&full);
        assert_eq!(full_flat.len(), 8);
        assert_eq!(full.len(), 3, "縦書きでも 塊A / 塊B / 塊C の 3 列");
        for v in 0..=8 {
            let partial = LayoutEngine::layout(
                &items,
                v,
                &region,
                WritingMode::VerticalRl,
                10.0,
                &FixedMetrics,
                WrapPlan::Segmented(&p),
            );
            assert_eq!(
                flat_glyphs(&partial).as_slice(),
                &full_flat[..v],
                "vertical_rl visible {v}: 配置が全量出力の prefix と不一致（リフロー跳び）"
            );
        }
    }
}
