//! 文字の配置の腕（`layout::scan` の子）: 可視の打ち切り→保留の実体化→折り返し判定→遠辺の判定→配置。
//! 足す予定の spec: `text-typesetting`（禁則・縦中横）・`text-ruby`（親文字の単位）。

use std::ops::ControlFlow;
use std::sync::Arc;

use super::super::{glyph_style_advance, segment_advance_sum};
use super::{
    PositionedGlyph, Scan, WrapPlan, apply_pending_cursor, apply_pending_newline, finish_line,
    finish_pending_line, line_pitch_of,
};

impl Scan<'_, '_> {
    /// 文字の腕（旧 `TextItem::Glyph { ref text }` の本文そのまま）。
    /// 可視の打ち切りで走査を止めるときだけ `ControlFlow::Break(())` を返す。
    /// 呼び手 `layout_inner` は親 `scan` にあるので `pub(super)`（メソッドの私有は
    /// `impl` を書いたモジュールとその子孫にしか見えない）。
    pub(super) fn glyph(&mut self, text: &Arc<str>) -> ControlFlow<()> {
        // ゲート順序の契約（DD-3）: ①可視 prefix 打切り → ②保留フラッシュ →
        // ③折返し判定 → ④配置。①を先頭に置くことで、リビールカーソルが
        // 改行を通過済みでも次の可視グリフが無い限り行送りは起きない（R4.2）。
        if self.placed == self.visible_count {
            return ControlFlow::Break(());
        }
        // 装飾番号と送り幅（R3.3／R7.10）。番号列が無い経路・既定の番号の文字は
        // 従来どおり `advance(text, font_height)`——既定の見た目の高さが
        // `font_height` と食い違う登録前の一瞬でも、装飾なしの出力を動かさない。
        let (style, advance, glyph_height) = glyph_style_advance(
            text,
            self.placed,
            self.font_height,
            self.metrics,
            self.styles.as_ref(),
        );
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
        if self.pending.is_some() || self.pending_cursor.is_some() {
            // 閉じる行の丈（要件 7.9）。文字の無い行（改行だけの行）はここで
            // 手元にある**次に置く文字**の大きさ＝そのとき効いている大きさになる。
            let closing = self.heights.close(Some(glyph_height));
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
        // ③ 折返し判定（WrapPlan で分岐・design System Flows「ゲート③」）。
        // feed＝この可視グリフの配置前に行送りするか。ゲート①②④・行頭 1 グリフ
        // 配置（無限折返し回避）・行矩形規約は分岐に依らず不変。
        // 直前にフラッシュした場合 current は空ゆえ二重前進しない。
        // 塊内（先決済み）かどうかは、分岐が `seg_remaining` を減らす前に読む。
        let in_segment = self.seg_remaining > 0;
        let feed = match self.wrap {
            // CharByChar: 既存の文字単位規則そのまま（byte 等価の非回帰経路）。
            WrapPlan::CharByChar => {
                !self.current.is_empty() && self.inline_pos + advance > self.soft
            }
            WrapPlan::Segmented(plan) => {
                if self.seg_remaining > 0 {
                    // 塊内: 先決済み＝追加判定なしで配置（浮動丸めでの途中分割排除・2.1/2.3）。
                    self.seg_remaining -= 1;
                    false
                } else if let Some(seg) = plan.segment_starting_at(self.placed) {
                    // 塊先頭: 塊全体の advance 合計を全文 plan から左畳み込みで先決
                    // （visible_count 非依存＝INV-1/7.1）。
                    let seg_sum = segment_advance_sum(
                        self.items,
                        self.placed,
                        seg.len,
                        self.font_height,
                        self.metrics,
                        self.styles.as_ref(),
                    );
                    // 塊の収まり判定の基準は soft と hard の近い方（塊は
                    // どちらも超えられない）。2 つの値は畳まずに持ったまま、
                    // ここでの「行幅」の計算にだけ近い方を使う。
                    let limit = self.soft.min(self.hard);
                    let cap_rem = limit - self.inline_pos; // 残り行幅
                    let cap_full = limit - self.inline_start; // 行頭からの行幅
                    if seg_sum <= cap_rem {
                        // 現在行に収まる → 分割せず継続配置（2.1/2.3）。
                        self.seg_remaining = seg.len - 1;
                        false
                    } else if seg_sum <= cap_full {
                        // 収まらないが行頭からなら収まる → 塊の前で行送り（2.2）。
                        // 行頭では cap_rem == cap_full ゆえ本分岐は構造的に不発火
                        // ＝ワードラップは空行を作らない（INV-3）。
                        self.seg_remaining = seg.len - 1;
                        true
                    } else {
                        // 長大塊（行頭からでも収まらない）: 当該塊のみ既存 char 規則へ
                        // 委譲（3.1/3.2）。seg_remaining は設定せず＝続くグリフは非被覆
                        // として char 規則で処理され、次の塊先頭で通常判定を再開する（3.3）。
                        !self.current.is_empty() && self.inline_pos + advance > self.soft
                    }
                } else {
                    // plan 非被覆（不整合／長大塊の継続）: 既存 char 規則で配置
                    // （優しい縮退・4.2・design Error Handling「plan と items の不整合」）。
                    !self.current.is_empty() && self.inline_pos + advance > self.soft
                }
            }
        };
        // ③' 描画範囲の遠辺（hard）の判定。分岐（CharByChar／塊内／塊先頭／
        // 非被覆）に依らず**配置の直前に必ず**通す——これが「描画範囲の外へ
        // 文字を置かない」（R6.2）を構造で保つ最後の門である。行頭の 1 グリフ
        // （`current` が空）は soft と同じく例外で、超えても置く（無限折返しの排除）。
        let over_hard = !self.current.is_empty() && self.inline_pos + advance > self.hard;
        if over_hard && in_segment {
            // 先決済みの塊が途中で割れる＝「塊は分割されない」の例外。塊の容量は
            // 塊先頭で `limit = soft.min(hard)` を基準に先決してあるので、送りを積む
            // だけではここへ届かない——`\_l`（[`TextItem::CursorMove`]）が塊の途中で
            // 行内位置を跳ばしたときにだけ発火する。ゆえに折返し基準が描画範囲の
            // 内にある通常のバルーン（soft ≤ hard）でも起こりうる縮退である。
            // 判断の理由が読める形で 1 件残す（design.md 縮退表・R6.6）。
            tracing::debug!(
                inline_pos = self.inline_pos,
                advance,
                hard = self.hard,
                "塊の途中で描画範囲の遠辺に達した——塊を分割して次行へ続ける"
            );
        }
        if feed || over_hard {
            let closing = self.heights.close(Some(glyph_height));
            self.lines.push(finish_line(
                std::mem::take(&mut self.current),
                self.mode,
                self.inline_start,
                self.inline_pos,
                self.block_pos,
                closing,
            ));
            self.block_pos += self.block_dir * line_pitch_of(self.metrics, closing);
            self.inline_pos = self.inline_start;
        }
        // ④ 配置。
        self.current.push(PositionedGlyph {
            text: text.clone(),
            inline_pos: self.inline_pos,
            advance,
            style,
        });
        self.inline_pos += advance;
        self.heights.place(glyph_height);
        self.placed += 1;
        ControlFlow::Continue(())
    }
}
