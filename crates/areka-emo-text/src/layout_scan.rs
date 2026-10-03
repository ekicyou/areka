//! 走査の本体（`layout` の子）: `layout_inner` の駆動・走査の状態 `Scan`・改行と `\_l` の腕・行を閉じる仕上げ。
//! 足す予定の spec: `text-ruby`（新しい項目の腕）・`text-align-shadow-canon`（`\_l` 直後の寄せの戻し・行矩形）。

use super::{
    ActorKey, CursorAxis, CursorBasis, CursorWarnGuard, GlyphMetrics, GlyphStyles, LayoutEngine,
    LineHeights, LineRect, PositionedGlyph, PositionedLine, TextItem, TextRegion, WrapPlan,
    WritingMode, line_pitch_of, resolve_cursor_component,
};
use crate::state::CursorCoord;

#[path = "layout_scan_glyph.rs"]
mod glyph;

/// 走査の状態。`layout_inner` の中で作り、`finish` で消える（フレームをまたがない）。
struct Scan<'a, 'w> {
    // 走査の外で決まる入力（layout_inner の引数）
    items: &'a [TextItem],
    visible_count: usize,
    region: &'a TextRegion,
    mode: WritingMode,
    font_height: f32,
    metrics: &'a dyn GlyphMetrics,
    wrap: WrapPlan<'a>,
    cursor_warn: Option<(&'a ActorKey, &'w mut CursorWarnGuard)>,
    styles: Option<GlyphStyles<'a>>,
    // 走査の前に 1 度だけ決まる値（旧 let 群）
    pitch: f32,
    soft: f32,
    hard: f32,
    start: (f32, f32),
    inline_start: f32,
    block_dir: f32,
    // 走査で動く変数（旧 layout_inner の可変の局所変数 9 つ・名前はそのまま）
    heights: LineHeights,
    lines: Vec<PositionedLine>,
    current: Vec<PositionedGlyph>,
    inline_pos: f32,
    block_pos: f32,
    placed: usize,
    pending: Option<f32>,
    pending_cursor: Option<(Option<f32>, Option<f32>)>,
    seg_remaining: usize,
}

impl LayoutEngine {
    /// 配置の本体。`styles` は「グリフ序数→装飾番号／見た目」の読み口で、
    /// `None`（[`layout`](Self::layout)・[`layout_with_cursor_warn`](Self::layout_with_cursor_warn)）
    /// の経路は装飾を入れる前と 1 ビットも変わらない——送り幅は `metrics.advance` のまま、
    /// [`PositionedGlyph::style`] は全グリフ [`StyleId::DEFAULT`]（`layout_styled_tests.rs`
    /// が装飾導入前の実測値で固定している）。`Some` のときだけ、既定でない番号の文字を
    /// [`GlyphMetrics::advance_styled`] で測り、番号を配置済みグリフへ写す（R3.3／R7.10／R11.2）。
    ///
    /// 行の丈と行送りは [`LineHeights`] が供給する——閉じる行に置かれた文字の em の最大値
    /// （文字が無い行は次に置く文字の大きさ・それも無ければスコープの現在の見た目）を
    /// [`line_pitch_of`] へ渡すだけで、装飾のための別の式は持ち込まない（R7.8／R7.9）。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn layout_inner(
        items: &[TextItem],
        visible_count: usize,
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
        metrics: &dyn GlyphMetrics,
        wrap: WrapPlan<'_>,
        cursor_warn: Option<(&ActorKey, &mut CursorWarnGuard)>,
        styles: Option<GlyphStyles<'_>>,
    ) -> Vec<PositionedLine> {
        // 行送りの式は [`line_pitch_of`] の 1 点だけを通る（要件 7.8）。この `pitch` は
        // `\_l` の**単位 `lh` の係数**（`CursorBasis::line_pitch`）専用で、design の宣言
        // どおり既定の大きさのまま装飾では動かない。行を閉じる各点と `\_l` の実効位置の
        // 先読みは、そこで閉じる行の丈から引き直す（[`LineHeights`]）。
        let pitch = line_pitch_of(metrics, font_height);
        // 行の丈（行内最大 em・要件 7.9）。番号列が無い経路では常に `font_height` を返すので
        // 従来の出力と 1 ビットも変わらない。
        let heights = LineHeights::new(styles.as_ref(), font_height);
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

        let lines: Vec<PositionedLine> = Vec::new();
        let current: Vec<PositionedGlyph> = Vec::new();
        let inline_pos = inline_start;
        let block_pos = block_start;
        let placed = 0usize;
        // 改行の保留（deferred newline）: None＝保留なし・Some(Σratio)＝累算済み予約。
        // `f32` 単独でなく Option なのは `\n[0]`（ratio 0＝行替え・送りゼロ）を「保留なし」
        // と区別して保存するため（DD-5）。走査ローカル＝フレームを跨ぐ状態を持たない。
        let pending: Option<f32> = None;
        // pending-cursor（`\_l` 遅延実体化）: None＝保留なし・Some((inline, block))＝
        // 解決層（[`crate::cursor_tag::resolve_cursor_axis`]）が返した絶対 image px
        // （移動が成立しなかった軸は含めない＝当該軸不動・R1.6/5.5）。
        // 走査ローカル＝フレームを跨がない（newline-defer の `pending` と同型・同一フラッシュで合成）。
        let pending_cursor: Option<(Option<f32>, Option<f32>)> = None;
        // 先決済み塊の残グリフ数（Segmented 経路のみ使用）。正＝塊内（追加判定なし配置）・
        // 0 かつ塊先頭でない＝plan 非被覆（既存 CharByChar 式で判定）——この 2 状態の区別が
        // 「塊は途中分割されない」の型保証（design System Flows「塊内は追加判定なし」）。
        let seg_remaining: usize = 0;

        let mut scan = Scan {
            items,
            visible_count,
            region,
            mode,
            font_height,
            metrics,
            wrap,
            cursor_warn,
            styles,
            pitch,
            soft,
            hard,
            start,
            inline_start,
            block_dir,
            heights,
            lines,
            current,
            inline_pos,
            block_pos,
            placed,
            pending,
            pending_cursor,
            seg_remaining,
        };
        for item in items {
            match *item {
                TextItem::Glyph { ref text } => {
                    if scan.glyph(text).is_break() {
                        break;
                    }
                }
                TextItem::LineBreak { ratio } => scan.line_break(ratio),
                TextItem::CursorMove { x, y } => scan.cursor_move(x, y),
            }
        }
        scan.finish()
    }
}

impl Scan<'_, '_> {
    /// 改行の腕（旧 `TextItem::LineBreak { ratio }` の本文そのまま）。
    fn line_break(&mut self, ratio: f32) {
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
        if self.pending_cursor.is_some() {
            // 先行実体化の時点では**次に置く文字が無い**——文字の置かれていない
            // 行はスコープの現在の見た目の大きさで送る（要件 7.9）。
            let closing = self.heights.close(None);
            finish_pending_line(
                &mut self.lines,
                &mut self.current,
                self.mode,
                self.inline_start,
                self.inline_pos,
                self.block_pos,
                closing,
            );
            apply_pending_newline(
                &mut self.pending,
                &mut self.inline_pos,
                &mut self.block_pos,
                self.inline_start,
                self.block_dir,
                line_pitch_of(self.metrics, closing),
            );
            apply_pending_cursor(
                &mut self.pending_cursor,
                &mut self.inline_pos,
                &mut self.block_pos,
            );
        }
        // 遅延（deferred newline・R1.1/1.3）: 行を閉じず・block も前進させず、
        // 保留へ ratio を累算する（連続改行は単一累算 Σratio）。可視構造・
        // 内容ビューボックスはここでは一切変化しない（R1.2/1.5）。
        self.pending = Some(self.pending.map_or(ratio, |acc| acc + ratio));
    }

    /// `\_l` の腕（旧 `TextItem::CursorMove { x, y }` の本文そのまま）。
    fn cursor_move(&mut self, x: CursorCoord, y: CursorCoord) {
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
        let mut eff_inline = self.inline_pos;
        let mut eff_block = self.block_pos;
        if let Some(sum) = self.pending {
            // 行送りはフラッシュ側と同じ規則——閉じる行の丈から引く
            // （[`LineHeights::peek`] の doc に理由・要件 7.9）。
            eff_block += self.block_dir * line_pitch_of(self.metrics, self.heights.peek()) * sum;
            eff_inline = self.inline_start;
        }
        if let Some((inline_val, block_val)) = self.pending_cursor {
            if let Some(iv) = inline_val {
                eff_inline = iv;
            }
            if let Some(bv) = block_val {
                eff_block = bv;
            }
        }
        // 実効位置を image 軸へ逆写像する（変数名 `eff` は、同ファイルで 200 行に
        // わたり「現在行のグリフ列」を意味する `current` との衝突を避けるため）。
        let eff = match self.mode {
            WritingMode::HorizontalTb => (eff_inline, eff_block),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (eff_block, eff_inline),
        };
        // 基点束。原点は**解決済みの文字描画開始点** `TextRegion::start()`（範囲内の
        // `origin` 宣言は宣言どおり・範囲外の宣言と未宣言成分は書字開始角）であって、validrect の
        // 辺ではない（Requirement 2.1）。軸の向きは 3 書字方向共通（X 正＝右・Y 正＝下）で、
        // 書字方向で変わるのは原点の位置だけ——`horizontal_tb`／`vertical_lr` は
        // `(left, top)`・`vertical_rl` は `(right, top)`（design Data Models の原点表）。
        // これにより `vertical_rl` の `\_l[0,0]` が 1 列目の先頭を指す（2.3）。
        // `centerx`／`centery` の基準はバルーン画像の原寸（validrect でも原点でもない・4.3）。
        let basis = CursorBasis {
            origin: self.start,
            current: eff,
            image_size: self.region.image_size(),
            font_height: self.font_height,
            line_pitch: self.pitch,
        };
        let x_val =
            resolve_cursor_component(x, CursorAxis::X, &basis, self.region, &mut self.cursor_warn);
        let y_val =
            resolve_cursor_component(y, CursorAxis::Y, &basis, self.region, &mut self.cursor_warn);
        // 軸読み替え正準表: 水平/垂直軸値を行内/ブロック軸へ写像（horizontal_tb＝行内 x・
        // ブロック y／縦書き rl・lr＝行内 y・ブロック x——layout の inline/block 割当と同一）。
        let (inline_val, block_val) = match self.mode {
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
            let (old_inline, old_block) = self.pending_cursor.unwrap_or((None, None));
            self.pending_cursor = Some((inline_val.or(old_inline), block_val.or(old_block)));
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

    /// 最終行の確定（旧 走査の後の `if !current.is_empty()` と `lines` の返却）。
    fn finish(mut self) -> Vec<PositionedLine> {
        // 最終行の確定: グリフを含む現在行のみ確定する（行の確定は常にグリフ配置に
        // 隣接するため、旧 `opened` フラグは `!current.is_empty()` と等価・DD-4）。
        // 残存する保留（末尾改行）は実体化せず蒸発する（R5.2/5.3）。
        if !self.current.is_empty() {
            let closing = self.heights.close(None);
            self.lines.push(finish_line(
                self.current,
                self.mode,
                self.inline_start,
                self.inline_pos,
                self.block_pos,
                closing,
            ));
        }
        self.lines
    }
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
    line_height: f32,
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
        line_height,
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
///
/// `line_height` は**その行の丈**であって既定の大きさとは限らない——装飾のある行では
/// 行内に置かれた文字の em の最大値が入る（R7.9・[`LineHeights::close`] が決める）。
/// 装飾の無い経路では従来どおり `font_height` がそのまま届く。
fn finish_line(
    glyphs: Vec<PositionedGlyph>,
    mode: WritingMode,
    inline_start: f32,
    inline_end: f32,
    block_pos: f32,
    line_height: f32,
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
            bottom: block_pos + line_height,
        },
        WritingMode::VerticalRl => LineRect {
            left: block_pos - line_height,
            top: inline_lo,
            right: block_pos,
            bottom: inline_end,
        },
        WritingMode::VerticalLr => LineRect {
            left: block_pos,
            top: inline_lo,
            right: block_pos + line_height,
            bottom: inline_end,
        },
    };
    PositionedLine { rect, glyphs }
}
