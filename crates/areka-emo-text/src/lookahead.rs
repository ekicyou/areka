//! # lookahead — 合図 1 つで状態を進める唯一の規則と、先渡しの空回し（純粋層）
//!
//! 本番の合図の適用（[`crate::actor::TextLayerRuntime::apply_cue`]）と、トークの先渡しを
//! 空回しで流す側（[`TalkLookahead::install`]）は、どちらも [`advance_state`] を呼んで状態を
//! 進める。行き先（`\s` の解決を含む）を求める規則を 2 つに分けて持たないためである（要件 2.2）。
//!
//! [`TalkLookahead`] は空回しの結果を「場所 × 区間の番号 → 区間の全文」として覚える。区間の
//! 番号は「先渡しを受け取ってからの全消去の回数（場所に状態があるかに関わらず）＋その場所の
//! `\c` の回数」で、状態の層が数える消去の回数（状態のある場所だけ進む）は使わない——
//! 先渡しと頭の全消去の間にバルーンの装着で場所ができると、そちらは 1 つずれるため。
//!
//! **層規律**: 純粋層。時計も窓も `World` も見ない（`PURE_SOURCES` に載せて走査する）。

use std::collections::{BTreeMap, BTreeSet};

use areka_sakura::contract::{CueCommand, TalkCue};
use tracing::{debug, warn};

use crate::place::{PlaceKey, TextPlace};
use crate::segment::{SegmentPlan, segment_plan};
use crate::state::{ActorTextState, SurfaceKeyOutcome, TextLayerState};

/// 合図 1 つで状態を進める。`\s` は `resolve` で番号に解いて行き先へ渡し（閉包が無ければ読まない）、
/// そのあと `state.apply_cue(cue)` を呼ぶ。
///
/// 同じ状態・同じ閉包・同じ合図なら同じ結果になる。描画の全消し要求や選択肢の写しの後始末は
/// 状態の外の話なので、ここでは扱わない（呼ぶ側の結線が持つ）。
pub(crate) fn advance_state(
    state: &mut TextLayerState,
    resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
    cue: &TalkCue,
) {
    // `\s` は解決の閉包で番号に解いて行き先へ渡す（要件 4.1）。閉包が無いあいだは
    // 読まない＝行き先は普通のバルーンのまま（要件 5.4）。
    if let CueCommand::Emote { key } = &cue.command {
        match resolve {
            Some(resolve) => state.route_surface(&cue.actor, resolve(key)),
            None => {
                debug!(actor = %cue.actor, key, "解決の閉包が無い——\\s を読まない（行き先は普通のバルーン）")
            }
        }
    }
    state.apply_cue(cue);
}

/// 先渡しを受け取ってからの消去の数え（区間の番号の元）。空回しと本番で同じ数え方をする。
#[derive(Clone, Default)]
struct ClearCounts {
    /// 全消去の回数（場所に状態があるかに関わらず進む）。
    all: u64,
    /// 場所ごとの `\c` の回数。
    by_place: BTreeMap<PlaceKey, u64>,
}

impl ClearCounts {
    /// 場所の今の区間の番号。
    fn number(&self, place: &PlaceKey) -> u64 {
        self.all + self.by_place.get(place).copied().unwrap_or(0)
    }

    /// 合図 1 つを数える（`dest` はその合図の行き先＝`\c` で消される場所）。
    fn count(&mut self, cue: &TalkCue, dest: &PlaceKey) {
        match cue.command {
            CueCommand::ClearAll => self.all += 1,
            CueCommand::Clear => *self.by_place.entry(dest.clone()).or_default() += 1,
            _ => {}
        }
    }
}

/// 区間の全文 1 つ: その区間が終わる時点の内容の写しと、その区切り（最初に要ったときに 1 度だけ計算）。
#[derive(Debug)]
struct Section {
    content: ActorTextState,
    plan: Option<SegmentPlan>,
}

/// 配置に使う字の列（[`TalkLookahead::basis`] の答え）。
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "タスク 4.2 で arrange_lines の文節の枝から読むまで"
    )
)]
pub(crate) enum Basis<'a> {
    /// 区間の全文で配置する（届いた内容は全文の先頭と一致している）。
    Full {
        content: &'a ActorTextState,
        plan: &'a SegmentPlan,
    },
    /// 届いた字だけで配置する（修正前の動き）。
    Arrived,
}

/// `full` の先頭が届いた内容 `arrived` と一致するか: 字の列（改行・カーソル移動を含む）・
/// 字ごとの装飾番号・装飾の表の 3 つとも、届いたぶんが全文の先頭と同じ。
fn begins_with(full: &ActorTextState, arrived: &ActorTextState) -> bool {
    full.items().starts_with(arrived.items())
        && full.glyph_styles().starts_with(arrived.glyph_styles())
        && full.styles().starts_with(arrived.styles())
}

/// 先渡しの空回しで求めた区間の全文の持ち主（design.md「`TalkLookahead`」）。
///
/// 正本ではない（届いた字の正本は `TextLayerState`）。本番の状態を書き換えない。
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "タスク 4.1 で実行時（TextLayerRuntime）から呼ぶまで"
    )
)]
#[derive(Default)]
pub(crate) struct TalkLookahead {
    /// 場所 × 区間の番号 → その区間の全文。
    sections: BTreeMap<(PlaceKey, u64), Section>,
    /// 先渡しを受け取ってからの消去の数え（本番の合図で進む）。
    counts: ClearCounts,
    /// warn を出し済みの（場所, 区間の番号）。
    warned: BTreeSet<(PlaceKey, u64)>,
    /// 先渡しの列の写し（届いた合図が列と食い違えば捨てる）。
    upcoming: Option<Vec<TalkCue>>,
    /// 先渡しの列のうち、本番で届いた数。
    delivered: usize,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "タスク 4.1 で実行時（TextLayerRuntime）から呼ぶまで"
    )
)]
impl TalkLookahead {
    /// 先渡しを受け取る: 今の状態の写しで空回しし、区間の全文を入れ替える（design.md の手順 1〜8）。
    pub(crate) fn install(
        &mut self,
        state: &TextLayerState,
        resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
        upcoming: &[TalkCue],
    ) {
        // 1. 持ち越し: 場所ごとに今の区間の全文と warn 済みの印だけを取り置く。
        let counts = std::mem::take(&mut self.counts);
        let carried: Vec<(PlaceKey, Section)> = std::mem::take(&mut self.sections)
            .into_iter()
            .filter(|((place, n), _)| *n == counts.number(place))
            .map(|((place, _), content)| (place, content))
            .collect();
        let warned: Vec<PlaceKey> = std::mem::take(&mut self.warned)
            .into_iter()
            .filter(|(place, n)| *n == counts.number(place))
            .map(|(place, _)| place)
            .collect();
        // 2〜5. 数えは 0 から（上の take で戻した）、印つきの写しで空回しする。
        self.rehearse(
            state.rehearsal_copy(),
            ClearCounts::default(),
            BTreeSet::new(),
            resolve,
            upcoming,
        );
        // 6. 本番の数えはここから（1 の take で 0 に戻したまま。空回しは自前の数えで番号を振った）。
        // 7. 持ち越しを番号 0 へ（空回しが番号 0 に控えた場所は空回しの側を採る）。warn 済みの
        //    印も持ち越しの一部なので、空回しが全文を求め直した場所では空回しの側（印なし）を採る
        //    ＝新しい全文との食い違いは改めて 1 度記録する（`reinstall` が印を消すのと同じ考え）。
        self.warned = warned
            .into_iter()
            .map(|place| (place, 0))
            .filter(|key| !self.sections.contains_key(key))
            .collect();
        for (place, section) in carried {
            self.sections.entry((place, 0)).or_insert(section);
        }
        // 8. やり直しのために列を覚える。
        self.upcoming = Some(upcoming.to_vec());
        self.delivered = 0;
    }

    /// 本番の合図 1 つを見て、消去と届いた数を数える（状態を進める前に呼ぶ）。`dest` はその合図の行き先。
    /// 合図が先渡しの列の次の合図と同じでなければ、列を捨てる（以後やり直さない）。
    pub(crate) fn note_cue(&mut self, cue: &TalkCue, dest: &PlaceKey) {
        self.counts.count(cue, dest);
        let Some(upcoming) = &self.upcoming else {
            return;
        };
        if upcoming.get(self.delivered) == Some(cue) {
            self.delivered += 1;
        } else {
            debug!(actor = %cue.actor, delivered = self.delivered, "先渡しの列に無い合図——列を捨てる（以後やり直さない）");
            self.upcoming = None;
        }
    }

    /// 空回しをやり直す: 今の状態の写しから、先渡しの列のまだ届いていない残りを流し、
    /// 今の区間とそれより後の区間の全文を入れ替える（design.md の手順 1〜5）。
    /// 先渡しの列が無ければ何もしない。
    pub(crate) fn reinstall(
        &mut self,
        state: &TextLayerState,
        resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
    ) {
        // 1. 列が無い（受け取っていない・捨てた）なら今ある全文のまま。
        let Some(upcoming) = self.upcoming.take() else {
            return;
        };
        // 2. 印は全部消し、全文は今の番号より前のものだけ残す（場所ごと）。
        self.warned.clear();
        let counts = &self.counts;
        self.sections
            .retain(|(place, n), _| *n < counts.number(place));
        // 3〜4. 数えは今の値の写しから（本番の数えには触れない）。内容のある場所は全部「足した」
        //       ことにして始める＝残りが何も足さない場所でも、今の内容がそのまま今の区間の全文になる。
        let copy = state.rehearsal_copy();
        let added = copy
            .places()
            .filter(|(_, content)| !content.is_empty())
            .map(|(key, _)| key.clone())
            .collect();
        // 5. まだ届いていない残りを流して控える。
        let rest = upcoming.get(self.delivered..).unwrap_or_default();
        debug!(
            delivered = self.delivered,
            rest = rest.len(),
            "空回しをやり直す（出発点が合図と無関係に変わった）"
        );
        self.rehearse(copy, self.counts.clone(), added, resolve, rest);
        self.upcoming = Some(upcoming);
    }

    /// 箱の場所の区間の全文と数えを捨てる（箱の束の差し替え）。warn 済みの印も区間と一緒に捨てる。
    pub(crate) fn forget_boxes(&mut self) {
        let is_box = |key: &PlaceKey| matches!(key.place, TextPlace::Box(_));
        self.sections.retain(|(place, _), _| !is_box(place));
        self.warned.retain(|(place, _)| !is_box(place));
        self.counts.by_place.retain(|place, _| !is_box(place));
    }

    /// 場所の今の区間の番号。
    pub(crate) fn number(&self, place: &PlaceKey) -> u64 {
        self.counts.number(place)
    }

    /// 写し `copy` で合図の列を空回しし、区間の全文を控える（`install` の手順 4〜5）。番号は
    /// `counts` で振る。`added` は始めから「この区間に足した」ことにする場所。
    fn rehearse(
        &mut self,
        mut copy: TextLayerState,
        mut counts: ClearCounts,
        mut added: BTreeSet<PlaceKey>,
        resolve: Option<&dyn Fn(&str) -> SurfaceKeyOutcome>,
        cues: &[TalkCue],
    ) {
        let _span = tracing::debug_span!("rehearsal").entered();
        for cue in cues {
            let dest = PlaceKey {
                actor: cue.actor.clone(),
                place: copy.destination(&cue.actor),
            };
            // 消去の前に、消される場所のうち「この区間に足した」ものを今の番号で控える。
            let cleared: Vec<PlaceKey> = match cue.command {
                CueCommand::ClearAll => std::mem::take(&mut added).into_iter().collect(),
                CueCommand::Clear => added.take(&dest).into_iter().collect(),
                _ => Vec::new(),
            };
            for place in cleared {
                self.keep(&copy, &counts, place);
            }
            counts.count(cue, &dest);
            advance_state(&mut copy, resolve, cue);
            if matches!(
                cue.command,
                CueCommand::Text(_)
                    | CueCommand::Choice { .. }
                    | CueCommand::NewLine { .. }
                    | CueCommand::Cursor { .. }
            ) {
                added.insert(dest);
            }
        }
        for place in added {
            self.keep(&copy, &counts, place);
        }
    }

    /// 写しの場所の内容を、`counts` での今の番号の区間の全文として控える（区切りは `basis` で要ったときに）。
    fn keep(&mut self, copy: &TextLayerState, counts: &ClearCounts, place: PlaceKey) {
        if let Some(content) = copy.place_state(&place) {
            let n = counts.number(&place);
            let section = Section {
                content: content.clone(),
                plan: None,
            };
            self.sections.insert((place, n), section);
        }
    }

    /// 配置に使う字の列を答える。全文が無い・食い違うときは [`Basis::Arrived`] を返し、
    /// （場所, 区間）ごとに初回だけ warn を残す。届いた内容に字が 1 つも無ければ warn しない。
    pub(crate) fn basis(&mut self, place: &PlaceKey, arrived: &ActorTextState) -> Basis<'_> {
        let key = (place.clone(), self.counts.number(place));
        let reason = match self.sections.get(&key) {
            None => "先渡しが無い",
            Some(section) if !begins_with(&section.content, arrived) => {
                "届いた字が先渡しと食い違う"
            }
            Some(_) => {
                return self
                    .sections
                    .get_mut(&key)
                    .map_or(Basis::Arrived, |section| {
                        let plan = section
                            .plan
                            .get_or_insert_with(|| segment_plan(section.content.items()));
                        Basis::Full {
                            content: &section.content,
                            plan,
                        }
                    });
            }
        };
        // 字の数は装飾番号の列の長さ（字ごとに 1 つ・改行とカーソル移動は数えない）。
        let arrived_glyphs = arrived.glyph_styles().len();
        if arrived_glyphs > 0 && !self.warned.contains(&key) {
            let full_glyphs = self
                .sections
                .get(&key)
                .map_or(0, |section| section.content.glyph_styles().len());
            warn!(
                actor = %place.actor,
                place = ?place.place,
                reason,
                arrived_glyphs,
                full_glyphs,
                "文節の折り返しで区間の全文を使えない——届いた字だけで区切って配置する（修正前の動き）"
            );
            self.warned.insert(key);
        }
        Basis::Arrived
    }
}

#[cfg(test)]
#[path = "lookahead_tests.rs"]
mod tests;
