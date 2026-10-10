//! # state_anchor — アンカー（`\_a`）の範囲の記録（純粋層）
//!
//! 開きの合図（`AnchorBegin`）から閉じの合図（`AnchorEnd`）までに、**開いた場所へ**追記された
//! 文字の並びを 1 つの範囲にする。範囲は選択肢と同じ列（[`ActorTextState::choices`]）に種類
//! [`SpanKind::Anchor`] で並び、通し番号は選択肢と共通（列の添字）。寿命も選択肢と同じで、
//! 本文の消去（`Clear`／`ClearAll`）で列と一緒に「開いている」印も消える。
//!
//! ## 開いている範囲は全体で高々 1 つ
//!
//! compile は台本の全体で開きと閉じを 1 対ずつに補ってから合図にするので、同時に開いている
//! 範囲は高々 1 つ。ただし合図の宛先は「その時点のスコープ」なので、開いたままスコープや
//! 文字の行き先が替わる台本（`\_a[x]あ\1い\_a\0う`）では、閉じが開いた場所と別の場所宛てに届く。
//! そこで閉じ（と、開きの前の防御の閉じ）は宛先に関わらず**全部の場所**を見て、開いている
//! 範囲を閉じる。伸ばすのは追記のあった場所の範囲だけなので、別の場所へ追記された文字
//! （上の例の「い」）は範囲に入らない。
//!
//! ## 記録
//!
//! 重なり（開いている間の開き）と迷子の閉じは compile が補って警告済みで、ここへは届かない。
//! 届いたときの扱いは防御で、`debug!` に留める。下見の空回しの写しは記録を 1 件も出さない
//! （本番の適用が出す・二重にしない）。

use super::{ActorTextState, ChoiceSpan, SpanKind, TextItem, TextLayerState};
use crate::place::PlaceKey;
use areka_sakura::contract::ActorKey;

impl ActorTextState {
    /// 今の文字の位置から始まる空の範囲を足し、「開いている」印を置く。
    ///
    /// 印は 1 つなので、既に開いていた範囲はここで伸びなくなる（＝そこで閉じる）。
    pub(super) fn anchor_begin(&mut self, id: &str, references: &[String]) {
        // 序数空間は items のグリフ（`Glyph` のみ）＝選択肢の範囲・リビールと同じ。
        let now = self
            .items
            .iter()
            .filter(|it| matches!(it, TextItem::Glyph { .. }))
            .count();
        let ordinal = self.choices.len();
        self.choices.push(ChoiceSpan {
            kind: SpanKind::Anchor,
            ordinal,
            id: id.to_owned(),
            label: String::new(),
            references: references.to_vec(),
            glyph_range: now..now,
        });
        self.anchor_open = Some(ordinal);
    }

    /// 「開いている」印を外す（範囲の記録は列に残る）。
    pub(super) fn anchor_end(&mut self) {
        self.anchor_open = None;
    }

    /// 文字（`Text`）と選択肢の文字（`Choice`）を追記した直後に呼ぶ。開いている範囲があれば、
    /// 終わりを追記の数だけ伸ばし、範囲の文字（Reference0 になる）に書記素クラスタを継ぐ。
    /// 改行や装飾の合図はここを通らないので、範囲の文字に入るのは文字だけ。
    pub(super) fn extend_open_anchor(&mut self, clusters: &[&str]) {
        let Some(span) = self.anchor_open.and_then(|i| self.choices.get_mut(i)) else {
            return;
        };
        span.glyph_range.end += clusters.len();
        span.label.extend(clusters.iter().copied());
    }

    /// アンカーの範囲が開いているか。
    pub(super) fn anchor_is_open(&self) -> bool {
        self.anchor_open.is_some()
    }
}

impl TextLayerState {
    /// `AnchorBegin` の適用: 合図の宛先の場所 `dest` に範囲を開く。
    ///
    /// どこかの場所で範囲が開いたままなら、先にそれを閉じる（到達しない防御）。
    pub(super) fn open_anchor(&mut self, dest: &PlaceKey, id: &str, references: &[String]) {
        let closed_previous = self.close_open_anchors();
        if !self.rehearsal {
            if closed_previous {
                tracing::debug!(actor = %dest.actor, id = %id, "前のアンカーが開いたまま——ここで閉じてから開く（到達しない防御）");
            }
            tracing::debug!(actor = %dest.actor, id = %id, "AnchorBegin cue 適用（アンカーの範囲を開く）");
        }
        self.place_entry(dest).anchor_begin(id, references);
    }

    /// `AnchorEnd` の適用: 合図の宛先に関わらず、開いている範囲を持つ場所を閉じる。
    ///
    /// どの場所にも開いている範囲が無ければ何もしない（迷子の閉じ・到達しない防御）。
    pub(super) fn close_anchor(&mut self, actor: &ActorKey) {
        let closed = self.close_open_anchors();
        if self.rehearsal {
            return;
        }
        if closed {
            tracing::debug!(actor = %actor, "AnchorEnd cue 適用（アンカーの範囲を閉じる）");
        } else {
            tracing::debug!(actor = %actor, "開いているアンカーが無い——閉じを無視する（到達しない防御）");
        }
    }

    /// 全部の場所の「開いている」印を外す。外した印があったかを返す。
    fn close_open_anchors(&mut self) -> bool {
        let mut closed = false;
        for state in self.actors.values_mut().filter(|s| s.anchor_is_open()) {
            state.anchor_end();
            closed = true;
        }
        closed
    }
}
