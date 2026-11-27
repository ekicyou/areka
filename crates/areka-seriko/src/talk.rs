//! talk: 文字が現れる時刻の列。
//!
//! `talk,数値` の引き金は「数値分の文字がバルーンに現れた時刻」で鳴る。その時刻を、文字の層と同じ式で
//! 絶対の時刻として持つ。時計も起点も持たない。
//!
//! [`TalkFeed`]: スコープ 1 つの「時刻の順の i 字目が現れる時刻」の列。字の行き先（普通のバルーン・箱）は
//! 文字の層と同じ [`TextRoute`] で決め、塊（文字の cue 1 件）ごとに、合図の発火時刻を塊の頭、行き先の
//! 場所の前の字を前の字として、文字の層と同じ 1 本の式（[`chunk_reveal_offsets`]）で求める。列は場所を
//! またいで時刻の順（同じ時刻は届いた順）に並び、序数はその順の番号。消去は現れなかった字を捨てる。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use areka_emo_compose::{BoxName, SurfaceKeyOutcome, TextPlace, TextRoute};
use areka_sakura::cluster::cluster_count;
use areka_sakura::reveal::chunk_reveal_offsets;
use dola::{TimeDelta, TimePoint, TimeSpan};

/// seriko が受け取る箱の材料（シェル 1 つ分・既定は箱の無いシェル）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextRouting {
    /// サーフェス番号 → element番号の昇順の箱の名前（`areka_emo_compose::box_index`）。
    pub boxes: BTreeMap<u32, Vec<BoxName>>,
    /// 面の表に在るサーフェス番号（`\s` の答えを文字の層と同じに絞る）。
    pub surface_ids: BTreeSet<u32>,
}

/// スコープ 1 つの文字の時刻の列（時刻の順・序数は塊と場所をまたいで連続）。
#[derive(Default)]
pub(crate) struct TalkFeed {
    /// `glyphs` の先頭の文字の序数（刈り込んだ分だけ進む）。
    base: u64,
    /// 文字が現れる時刻と、その字の場所の `places` の番号（時刻について単調非減少）。
    glyphs: VecDeque<(TimePoint, usize)>,
    /// 字の場所と、その場所の前の字の時刻（消去で `None` に戻る＝次の塊は塊の頭から）。刈り込みでは
    /// 変わらない。
    places: Vec<(TextPlace, Option<TimePoint>)>,
    route: TextRoute,
}

impl TalkFeed {
    /// `\s` の答えで字の行き先を動かす。
    pub(crate) fn follow_surface(&mut self, routing: &TextRouting, outcome: SurfaceKeyOutcome) {
        self.route.follow_surface(&routing.boxes, outcome);
    }

    /// 名前の形の `\b[名前]` で行き先を今の面の箱へ切り替える（無い名前では変えない）。
    pub(crate) fn select_box(&mut self, routing: &TextRouting, name: &str) {
        self.route.select_box(&routing.boxes, name);
    }

    /// 行き先を、番号 `surface`（無ければ非表示）だけが届いた状態から決め直す。
    pub(crate) fn seed_surface(&mut self, routing: &TextRouting, surface: Option<u32>) {
        self.route = TextRoute::default();
        let outcome = surface.map_or(SurfaceKeyOutcome::Hide, SurfaceKeyOutcome::Show);
        self.route.follow_surface(&routing.boxes, outcome);
    }

    /// 印の合図: 行き先の場所の前の字を、長さ `duration` の 1 字の塊として進める（序数は作らない）。
    pub(crate) fn push_mark(&mut self, fire_time: TimePoint, duration: TimeSpan) {
        let place = self.dest_index();
        let prev = self.places[place].1.map(|last| last - fire_time);
        if let Some(offset) = chunk_reveal_offsets(prev, duration, 1).last() {
            self.places[place].1 = Some(fire_time + offset);
        }
    }

    /// 全消去の合図（発火時刻 `at`）: 全部の場所のその時刻より後に現れる字を捨て、全部の場所を塊の頭
    /// から始めさせ、行き先を今の番号の既定へ戻す。捨てた字の序数を昇順で返す。
    pub(crate) fn restart_all(&mut self, at: TimePoint, routing: &TextRouting) -> Vec<u64> {
        let dropped = self.drop_after(at, |_| true);
        self.route.reset(&routing.boxes);
        dropped
    }

    /// シェルの切替（効いた時刻 `at`）: 箱の場所のその時刻より後に現れる字を捨て、箱の場所を塊の頭から
    /// 始めさせ、新しい表で行き先を決め直す（今の番号が新しいシェルに無ければ `candidates` の最初に在る
    /// 番号へ）。捨てた字の序数を昇順で返す。
    pub(crate) fn rebase_boxes(
        &mut self,
        at: TimePoint,
        routing: &TextRouting,
        candidates: &[u32],
    ) -> Vec<u64> {
        let dropped = self.drop_after(at, |place| matches!(place, TextPlace::Box(_)));
        self.route.reset(&routing.boxes);
        if let Some(outcome) = self
            .route
            .fallback_outcome(&routing.surface_ids, candidates)
        {
            self.route.follow_surface(&routing.boxes, outcome);
        }
        dropped
    }
}

impl TalkFeed {
    /// 文字の cue 1 件（台詞の文字・選択肢の文字）を、書記素クラスタの数だけ行き先の場所へ積む。
    pub(crate) fn push_text(&mut self, fire_time: TimePoint, duration: TimeSpan, text: &str) {
        self.push_chunk(fire_time, duration, cluster_count(text));
    }

    /// 塊 1 つ（頭＝合図の発火時刻 `fire_time`・再生時間 `duration`・`count` 文字）を行き先の場所へ積む。
    ///
    /// 文字の時刻は文字の層と同じ [`chunk_reveal_offsets`] で求める（前の文字は行き先の場所の直前の字）。
    /// 字は列の時刻の順の位置（同じ時刻の字の後ろ）へ入る。`count == 0` は何も積まない。
    pub(crate) fn push_chunk(&mut self, fire_time: TimePoint, duration: TimeSpan, count: usize) {
        let place = self.dest_index();
        let prev = self.places[place].1.map(|last| last - fire_time);
        for offset in chunk_reveal_offsets(prev, duration, count) {
            let r = fire_time + offset;
            let index = self.glyphs.partition_point(|&(t, _)| t <= r);
            self.glyphs.insert(index, (r, place));
            self.places[place].1 = Some(r);
        }
    }

    /// 消去の合図（発火時刻 `at`）: 行き先の場所のその時刻より後に現れる文字を捨て、その場所の次の塊を
    /// 塊の頭から始めさせる。捨てた字の序数を昇順で返す。
    ///
    /// 文字の層は消去で未表示の文字ごと場所の列を空にするので、現れなかった文字で口は動かない（中断
    /// された前の台詞の残りは、新しい台詞の頭の全消去でここで捨てる）。
    pub(crate) fn restart_chain(&mut self, at: TimePoint) -> Vec<u64> {
        let dest = self.route.dest().clone();
        self.drop_after(at, |place| *place == dest)
    }

    /// 置き直しの知らせ: 効く時刻 `effective_at` より後の時刻（全部の場所の字と前の字）を `delta` だけ
    /// 動かし、効く時刻より前になる物は効く時刻にする。効く時刻以前の時刻は書き換えない（単調な写しなので
    /// 列の順は崩れない・刈り込んだ文字の序数はそのまま）。
    pub(crate) fn shift_after(&mut self, effective_at: TimePoint, delta: TimeDelta) {
        let shift = |t: TimePoint| {
            if t > effective_at {
                (t + delta).max(effective_at)
            } else {
                t
            }
        };
        for (t, _) in &mut self.glyphs {
            *t = shift(*t);
        }
        for (_, last) in &mut self.places {
            *last = last.map(shift);
        }
    }

    /// 時刻 `t` までに現れた文字の数（序数で数える・ちょうどの時刻の文字は現れている）。
    pub(crate) fn revealed_until(&self, t: TimePoint) -> u64 {
        self.base + self.glyphs.partition_point(|&(r, _)| r <= t) as u64
    }

    /// 序数 `g` の文字が現れる時刻（刈り込み済み・まだ無い序数は `None`）。
    pub(crate) fn time_of(&self, g: u64) -> Option<TimePoint> {
        let index = usize::try_from(g.checked_sub(self.base)?).ok()?;
        self.glyphs.get(index).map(|&(t, _)| t)
    }

    /// 序数 `keep_from` より前の文字を捨てる。残る文字・後から届く文字の序数は変わらない
    /// （在る文字の数より先へは進めない）。
    pub(crate) fn prune_before(&mut self, keep_from: u64) {
        let count = usize::try_from(keep_from.saturating_sub(self.base))
            .map_or(self.glyphs.len(), |n| n.min(self.glyphs.len()));
        self.glyphs.drain(..count);
        self.base += count as u64;
    }

    fn dest_index(&mut self) -> usize {
        let dest = self.route.dest();
        match self.places.iter().position(|(place, _)| place == dest) {
            Some(index) => index,
            None => {
                self.places.push((dest.clone(), None));
                self.places.len() - 1
            }
        }
    }

    /// `hit` の場所の `at` より後の字を捨て、その場所の前の字を無しにする。捨てた字の序数を昇順で返す。
    fn drop_after(&mut self, at: TimePoint, hit: impl Fn(&TextPlace) -> bool) -> Vec<u64> {
        let hits: Vec<bool> = self.places.iter().map(|(place, _)| hit(place)).collect();
        let mut dropped = Vec::new();
        let mut ordinal = self.base;
        self.glyphs.retain(|&(t, place)| {
            let drop = hits[place] && t > at;
            if drop {
                dropped.push(ordinal);
            }
            ordinal += 1;
            !drop
        });
        for ((_, last), _) in self.places.iter_mut().zip(&hits).filter(|(_, hit)| **hit) {
            *last = None;
        }
        dropped
    }
}

#[cfg(test)]
#[path = "talk_origin_tests.rs"]
mod origin_tests;

#[cfg(test)]
#[path = "talk_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "talk_place_tests.rs"]
mod place_tests;
