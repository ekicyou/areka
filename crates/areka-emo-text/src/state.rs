//! # state — cue 駆動の純粋状態機械（純粋層）
//!
//! cue 列（Text／NewLine／Clear）を actor 別の行/グリフ状態へ純粋に遷移させる
//! `TextLayerState`／`ActorTextState`／`RevealSchedule`（注入時刻駆動 typewriter）を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 時刻は常に注入（`talk_time`）で受け取り、内部で実時間を読まない。
//!
//! ## 遷移規則（design.md「TextLayerState / RevealSchedule」正本）
//!
//! - cue の `actor`（`ActorKey`・"0"=sakura／"1"=kero…）を鍵に、actor 別の
//!   独立した [`ActorTextState`] へ振り分ける（R1.6）。未知 actor の cue は状態を
//!   lazily 生成して蓄積する（無損失・描画は binding 解決後）。
//! - **後出し優先の即時適用**: `Text`＝追記（R2.1）・`NewLine`＝改行マーカー追記
//!   （R2.2）・`Clear`＝未リビール分を含む全消去（R2.3）。トーク上書きを抑止する
//!   ガードは持たない——中断可否は上流（kanade の中断ファンネル）で決着済みの
//!   前提で、届いた cue 列を忠実に適用するのみ（R10.5）。
//! - `Choice` は M1 では actor ごと初回のみ `warn!`＋無視（choice-render シーム・
//!   テキスト状態は汚さない）。
//! - **typewriter リビール（R3／R7 系）**: `Text` 追記時に per-glyph リビール時刻を
//!   `r_i = max(r_{i-1} + interval, at(chunk(i)))`（先頭は `r_0 = at`）で確定する。
//!   `interval` は**配送された cue の再生時間**から `interval = duration / glyph_count`
//!   で導出する（自前の文字送り定数は持たない・服従＝再生時間の真実源に従う・R7.1/R7.2）。
//!   可視数は `visible_glyphs(actor, t)`＝注入時刻 `t` で `r_i <= t` のグリフ数。
//!   実時間 sleep／`Instant` 不使用（注入時刻駆動・R3.3/R9.1）。
//! - グリフ単位は Rust の `char`（M1 正準。書記素クラスタ結合は M2 検討事項——
//!   emo2 fixture は結合文字を使用しない）。
//! - 同一 cue 列＋同一入力条件→同一状態（決定論・R2.4/R2.5）。

use std::collections::{BTreeMap, BTreeSet};

use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

/// テキスト層の調整値（design.md「TextLayerRuntime」の config 正本）。
///
/// 純粋層・結線層の双方が消費する共有設定。値は design.md の正準既定
/// （`line_pitch`＝`ceil(font.height × 1.25)` の係数 1.25）。
///
/// **文字送り間隔（旧 `char_wait`）は撤去済み**（R7.2）——reveal のペースは自前の定数で
/// なく**配送された cue の再生時間**（`TalkCue::duration`）から `interval = duration / N`
/// で導出する（服従＝再生時間の単一真実源に従う）。本 config は描画メトリクス由来の
/// `line_pitch_factor` のみを保持する。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayerConfig {
    /// 行送りピッチ係数（`line_pitch = ceil(font.height × 係数)`・既定 1.25）。
    pub line_pitch_factor: f32,
}

impl Default for TextLayerConfig {
    fn default() -> Self {
        Self {
            line_pitch_factor: 1.25,
        }
    }
}

/// 追記順の正本を構成する 1 要素（グリフ／改行マーカー）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextItem {
    /// 1 グリフ（Rust `char` 単位・M1 正準）。
    Glyph {
        /// グリフの文字。
        ch: char,
    },
    /// 改行マーカー（`CueCommand::NewLine { ratio }` の転写・`\n`=1.0／`\n[half]`=0.5）。
    LineBreak {
        /// 行送り量の比率（`行送り量 = line_pitch × ratio`）。
        ratio: f32,
    },
}

/// per-glyph リビール時刻列（注入時刻駆動 typewriter・R3.1–3.5／R7.1–7.3）。
///
/// 時刻式（design.md「typewriter リビール」正本・決定論の正準）:
/// `r_i = max(r_{i-1} + interval, at(chunk(i)))`（先頭グリフは `r_0 = at(chunk(0))`）。
/// `interval`（1 グリフあたりの送り間隔）は**配送された cue の再生時間**から
/// `interval = duration / glyph_count` で導出する（自前定数を持たない・R7.2）。
/// 可視数は `visible(t) = |{ i : r_i <= t }|`。`at` は下限（それより早く可視化しない・
/// R3.4）であり、リビールカーソルは配送 duration が定めるペースでバッファ末尾を追う
/// （長文時は遅延しうる・無損失）。時刻は常に注入（talk 起点相対秒）で、実時間
/// （`Instant`／sleep）には依存しない（R3.3）。`Clear` は schedule ごと初期化
/// （未リビール分を含む破棄・R2.3/R3.6）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RevealSchedule {
    /// グリフ i のリビール時刻 `r_i`（talk 起点相対秒・単調非減少）。
    times: Vec<f64>,
}

impl RevealSchedule {
    /// リビール時刻列（talk 起点相対秒）。
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// リビール時刻が 1 件も無いか。
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// 注入時刻 `t` での可視グリフ数 `visible(t) = |{ i : r_i <= t }|`（R3.5）。
    ///
    /// `times` は単調非減少（`extend_chunk` の max が保証）なので二分探索で数える。
    pub fn visible(&self, t: f64) -> usize {
        self.times.partition_point(|&r| r <= t)
    }

    /// chunk（Text cue 1 件分のグリフ列）のリビール時刻を時刻式で追記する。
    ///
    /// `r_i = max(r_{i-1} + interval, chunk_start)`・先頭（schedule が空）のみ
    /// `r_0 = chunk_start`。`interval` は当該 chunk の配送 duration から導出した 1 グリフ
    /// あたりの送り間隔（`duration / glyph_count`・呼び出し側で算出）。改行マーカーは
    /// グリフでないため本 schedule の対象外（interval を消費しない）。
    ///
    /// `interval` は dola ingress で有限・非負へ clamp 済みの duration 由来ゆえ非負であり
    /// （duration=0 なら interval=0＝全グリフが `chunk_start` で同時可視）、`times` は
    /// 単調非減少に保たれる（`visible` の二分探索前提）。
    fn extend_chunk(&mut self, glyph_count: usize, chunk_start: f64, interval: f64) {
        self.times.reserve(glyph_count);
        for _ in 0..glyph_count {
            let r = match self.times.last() {
                Some(&prev) => (prev + interval).max(chunk_start),
                None => chunk_start,
            };
            self.times.push(r);
        }
    }
}

/// actor 1 人分の表示テキスト状態（追記順の正本＋リビール時刻列）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActorTextState {
    /// 追記順の正本（グリフ／改行マーカー）。
    items: Vec<TextItem>,
    /// per-glyph リビール時刻列。
    reveal: RevealSchedule,
}

impl ActorTextState {
    /// 追記順の正本（グリフ／改行マーカー）。
    pub fn items(&self) -> &[TextItem] {
        &self.items
    }

    /// per-glyph リビール時刻列。
    pub fn reveal(&self) -> &RevealSchedule {
        &self.reveal
    }

    /// テキスト状態が初期状態（空）か。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.reveal.is_empty()
    }
}

/// actor 別テキスト状態の集約（cue→行/グリフ状態の純粋遷移・R1.6/R2.1–2.5/R10.5）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayerState {
    /// `ActorKey → ActorTextState`（決定論的順序のため BTreeMap・design.md 正本）。
    actors: BTreeMap<ActorKey, ActorTextState>,
    /// `Choice` cue の warn を actor ごと初回のみに抑える記録（テキスト状態の外）。
    choice_warned: BTreeSet<ActorKey>,
}

impl TextLayerState {
    /// cue の純粋適用（DirectWrite 非依存・決定論）。
    ///
    /// 後出し優先の即時適用: `Text`＝追記・`NewLine`＝改行マーカー追記・
    /// `Clear`＝未リビール分を含む全消去。トーク上書きガードは持たず、届いた
    /// cue 列を到着順に忠実へ適用する（R10.5）。未知 actor は lazily 生成する。
    ///
    /// `Text` は追記と同時に per-glyph リビール時刻を `cue.at`（chunk 開始時刻＝
    /// リビール開始の下限・R3.4）と**配送された cue の再生時間から導出した
    /// `interval = duration / glyph_count`**の時刻式で確定する（R7.1/R7.3・服従＝
    /// 自前定数を持たず再生時間の真実源に従う）。`glyph_count = 0`（空テキスト）は
    /// 追記も除算も行わない（0 割り回避・R1.8）。`duration = 0`（瞬時／後方互換 cue）は
    /// `interval = 0` ゆえ全グリフが `cue.at` で同時可視になる（R1.2/R7.3 の縮退）。
    /// duration は dola ingress で有限・非負へ clamp 済みゆえ本層で再 clamp しない。
    /// リビール中の後続 cue も即時適用される（R3.6）——本層は cue の `at`／`duration` を
    /// 読むだけで書き換えず、pacing が cue 時刻へ影響することはない（R10.2）。
    pub fn apply_cue(&mut self, cue: &TalkCue) {
        match &cue.command {
            CueCommand::Text(text) => {
                let glyph_count = text.chars().count();
                // 服従（R7.1/R7.3）: reveal ペースは自前定数でなく配送 duration 由来。
                // N=0 は除算しない（0 割り回避・R1.8）。N>0 かつ duration=0 は interval=0
                // ＝全グリフが cue.at で同時可視（縮退・R1.2）。
                let interval = if glyph_count > 0 {
                    cue.duration / glyph_count as f64
                } else {
                    0.0
                };
                tracing::debug!(actor = %cue.actor, len = glyph_count, at = cue.at, duration = cue.duration, interval, "Text cue 適用（追記＋配送 duration 由来のリビール時刻確定）");
                let state = self.actors.entry(cue.actor.clone()).or_default();
                state
                    .items
                    .extend(text.chars().map(|ch| TextItem::Glyph { ch }));
                state.reveal.extend_chunk(glyph_count, cue.at, interval);
            }
            CueCommand::NewLine { ratio } => {
                tracing::debug!(actor = %cue.actor, ratio, "NewLine cue 適用（改行マーカー追記）");
                let state = self.actors.entry(cue.actor.clone()).or_default();
                state.items.push(TextItem::LineBreak { ratio: *ratio });
            }
            CueCommand::Clear => {
                tracing::debug!(actor = %cue.actor, "Clear cue 適用（未リビール分含む全消去）");
                // schedule ごと初期状態へ戻す（未リビールの文字も含めて破棄＝後出し優先・R2.3）。
                *self.actors.entry(cue.actor.clone()).or_default() = ActorTextState::default();
            }
            CueCommand::ClearAll => {
                tracing::debug!(actor = %cue.actor, "ClearAll cue 適用（全スコープを未リビール分含め消去）");
                // 対象スコープのみの `Clear` と峻別: 保持する**全** actor スコープの表示
                // テキスト（未リビール分を含む）を消去する。上流は残存スコープを列挙
                // できないため、全消しは本状態機械が自己完結して行う。
                //
                // 各スコープを初期状態へ**戻す**（`actors` から entry を除去しない）。
                // 提示層（`present_frame`）は `actors()` の走査で再描画対象を決めるため、
                // entry ごと消すと当該スコープが走査から外れ、既描画のテキストが画面に
                // 残留する（＝消えない）。`Clear` と同じ「entry は残し中身を空にする」
                // 流儀を守ることで、次フレームで空描画され実際に消える。
                for state in self.actors.values_mut() {
                    *state = ActorTextState::default();
                }
            }
            CueCommand::Choice { .. } => {
                // M1 対象外（choice-render シーム）: actor ごと初回のみ warn!＋無視・状態は汚さない。
                if self.choice_warned.insert(cue.actor.clone()) {
                    tracing::warn!(actor = %cue.actor, "Choice cue は M1 未対応のため無視する（choice-render シーム）");
                }
            }
            // 文字状態機械が消費しない command（cue_target_of が Shell/None に分類）は本状態機械の
            // 対象外——上流 routing の責務。防御的に無視する（catch-all を置かず、dola の
            // variant 追加時にコンパイラが再検討を強制する）。`BalloonSurface` は表示系
            // （SurfaceSink/seriko）の消費対象＝文字状態機械へは配送しない防御面（R3.2）。
            // `Wait` は action を持たない純粋な待ち（cue_target_of が `None` 分類）＝
            // どの表現者の担当でもなく、本状態機械は状態を変えない。時間は envelope
            // duration が担い、上流が後続 cue の時刻へ焼き込み済みゆえ、ここで新たな
            // ローカル遅延を生じさせてはならない（二重待ち禁止）。
            CueCommand::Emote { .. }
            | CueCommand::EntityRef(..)
            | CueCommand::Custom { .. }
            | CueCommand::BalloonSurface { .. }
            | CueCommand::Wait => {
                tracing::debug!(actor = %cue.actor, command = ?cue.command, "文字状態機械が消費しない cue を無視（上流 routing の対象外流入）");
            }
        }
    }

    /// 注入時刻 `t` での actor 別可視グリフ数（決定論・R3.5）。
    ///
    /// `visible(t) = |{ i : r_i <= t }|`（改行マーカーは数えない・グリフのみ）。
    /// 未生成の actor は 0。同一 cue 列＋同一注入時刻列→同一可視数（決定論檻）。
    pub fn visible_glyphs(&self, actor: &ActorKey, t: f64) -> usize {
        self.actors
            .get(actor)
            .map_or(0, |state| state.reveal.visible(t))
    }

    /// actor のテキスト状態（未生成の actor は `None`）。
    pub fn actor_state(&self, actor: &ActorKey) -> Option<&ActorTextState> {
        self.actors.get(actor)
    }

    /// 全 actor のテキスト状態（`ActorKey` 昇順・決定論的順序）。
    pub fn actors(&self) -> impl Iterator<Item = (&ActorKey, &ActorTextState)> {
        self.actors.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;

    /// FP 丸めに依存しない reveal 間隔（0.25 は 2 の冪＝正確表現）。duration 駆動 reveal では
    /// `interval = duration / N` ゆえ、Text cue へ `N × REVEAL_INTERVAL` を焼き込むことで
    /// interval=0.25 の決定論的リビール時刻列を得る（旧 char_wait=0.25 檻と機能等価・
    /// 期待リビール時刻は実装と同一の `D/N` 算術で成立し、旧 0.05 リテラル由来値を使わない）。
    const REVEAL_INTERVAL: f64 = 0.25;

    /// テスト用 cue 生成ヘルパ。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込み
    /// （reveal interval=0.25）、他コマンドは瞬時（duration=0）とする。明示的な duration を
    /// 与えたい縮退（D=0／空テキスト）・honor no-op 檻は [`cue_dur`] を使う。
    fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
        let duration = match &command {
            CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
            _ => 0.0,
        };
        TalkCue {
            at,
            actor: ActorKey::from(actor),
            command,
            duration,
        }
    }

    /// 明示 duration 版の cue ヘルパ（D=0 の縮退・空テキストの 0 割り回避・honor no-op 檻用）。
    fn cue_dur(actor: &str, at: f64, duration: f64, command: CueCommand) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(actor),
            command,
            duration,
        }
    }

    /// actor の items を取得する（未生成なら panic ＝テスト失敗として扱う）。
    fn items_of<'a>(state: &'a TextLayerState, actor: &str) -> &'a [TextItem] {
        state
            .actor_state(&ActorKey::from(actor))
            .expect("actor state should exist")
            .items()
    }

    // ── R2.1: Text 追記 ──

    #[test]
    fn text_cue_appends_glyphs_in_order() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
        assert_eq!(
            items_of(&state, "0"),
            &[
                TextItem::Glyph { ch: 'ア' },
                TextItem::Glyph { ch: 'ヒ' },
                TextItem::Glyph { ch: 'ル' },
            ]
        );
    }

    #[test]
    fn consecutive_text_cues_append() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒ".into())));
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("ルや".into())));
        assert_eq!(
            items_of(&state, "0"),
            &[
                TextItem::Glyph { ch: 'ア' },
                TextItem::Glyph { ch: 'ヒ' },
                TextItem::Glyph { ch: 'ル' },
                TextItem::Glyph { ch: 'や' },
            ]
        );
    }

    /// グリフ単位は Rust `char`（M1 正準）——多バイト文字も 1 char = 1 グリフ。
    #[test]
    fn glyph_unit_is_rust_char() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("aあ🦆".into())));
        assert_eq!(
            items_of(&state, "0"),
            &[
                TextItem::Glyph { ch: 'a' },
                TextItem::Glyph { ch: 'あ' },
                TextItem::Glyph { ch: '🦆' },
            ]
        );
    }

    // ── R2.2: NewLine 改行（ratio 転写） ──

    #[test]
    fn newline_cue_appends_line_break_marker_with_ratio() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())));
        state.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue("0", 0.2, CueCommand::NewLine { ratio: 0.5 }));
        state.apply_cue(&cue("0", 0.3, CueCommand::Text("B".into())));
        assert_eq!(
            items_of(&state, "0"),
            &[
                TextItem::Glyph { ch: 'A' },
                TextItem::LineBreak { ratio: 1.0 },
                TextItem::LineBreak { ratio: 0.5 },
                TextItem::Glyph { ch: 'B' },
            ]
        );
    }

    // ── R2.3: Clear 全消去（未リビール分含む・schedule ごと初期化） ──

    #[test]
    fn clear_resets_actor_state_to_initial() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒルや".into())));
        state.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue("0", 0.2, CueCommand::Text("アヒル！".into())));
        state.apply_cue(&cue("0", 0.3, CueCommand::Clear));

        let actor = state
            .actor_state(&ActorKey::from("0"))
            .expect("actor state should exist");
        assert_eq!(actor, &ActorTextState::default());
        assert!(actor.is_empty());
        assert!(actor.items().is_empty());
        assert!(actor.reveal().is_empty());
        assert!(actor.reveal().times().is_empty());
    }

    #[test]
    fn clear_only_affects_target_actor() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("さくら".into())));
        state.apply_cue(&cue("1", 0.0, CueCommand::Text("けろ".into())));
        state.apply_cue(&cue("0", 0.5, CueCommand::Clear));

        assert!(items_of(&state, "0").is_empty());
        assert_eq!(
            items_of(&state, "1"),
            &[TextItem::Glyph { ch: 'け' }, TextItem::Glyph { ch: 'ろ' }]
        );
    }

    /// `ClearAll` は保持する**全**スコープを消去し、対象スコープのみの `Clear` と
    /// 峻別される。cue の actor（ここでは "0"）に関わらず、当該 talk が書き込んで
    /// いないスコープ（"1"）も消える点が要点。
    #[test]
    fn clear_all_erases_every_actor_scope_unlike_clear() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("さくら".into())));
        state.apply_cue(&cue("1", 0.0, CueCommand::Text("けろ".into())));

        // 対象スコープのみの Clear では他スコープが残る（対比）。
        state.apply_cue(&cue("0", 0.5, CueCommand::Clear));
        assert!(!items_of(&state, "1").is_empty());

        // ClearAll は cue の actor に関わらず全スコープを消す。
        state.apply_cue(&cue("0", 1.0, CueCommand::ClearAll));
        assert!(items_of(&state, "0").is_empty());
        assert!(
            items_of(&state, "1").is_empty(),
            "ClearAll は当該 cue が名指ししていないスコープも消去する"
        );
    }

    /// `Wait`（action を持たない純粋な待ち）は文字状態機械の担当外——受け取っても
    /// テキスト状態を一切変えない（葉の否定的 no-op・二重待ちを生まない）。
    #[test]
    fn wait_cue_leaves_text_state_untouched() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
        let before = state
            .actor_state(&ActorKey::from("0"))
            .expect("actor state should exist")
            .clone();

        state.apply_cue(&cue("0", 0.5, CueCommand::Wait));

        let after = state
            .actor_state(&ActorKey::from("0"))
            .expect("actor state should exist");
        assert_eq!(&before, after, "Wait は状態を変えない（action なし）");
    }

    // ── R1.6: actor 別振り分け・独立状態・lazily 生成 ──

    #[test]
    fn cues_route_to_independent_actor_states() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())));
        state.apply_cue(&cue("1", 0.1, CueCommand::Text("B".into())));
        state.apply_cue(&cue("0", 0.2, CueCommand::Text("C".into())));

        assert_eq!(
            items_of(&state, "0"),
            &[TextItem::Glyph { ch: 'A' }, TextItem::Glyph { ch: 'C' }]
        );
        assert_eq!(items_of(&state, "1"), &[TextItem::Glyph { ch: 'B' }]);
    }

    #[test]
    fn unknown_actor_state_lazily_created_and_accumulates() {
        let mut state = TextLayerState::default();
        assert!(state.actor_state(&ActorKey::from("7")).is_none());

        state.apply_cue(&cue("7", 0.0, CueCommand::Text("x".into())));
        assert_eq!(items_of(&state, "7"), &[TextItem::Glyph { ch: 'x' }]);
    }

    #[test]
    fn actors_iterate_in_deterministic_key_order() {
        let mut state = TextLayerState::default();
        // 逆順に生成しても走査は ActorKey 昇順（決定論的順序）。
        state.apply_cue(&cue("1", 0.0, CueCommand::Text("b".into())));
        state.apply_cue(&cue("0", 0.1, CueCommand::Text("a".into())));

        let keys: Vec<&ActorKey> = state.actors().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&ActorKey::from("0"), &ActorKey::from("1")]);
    }

    // ── R10.5: 上書きガードなし・後出し優先の忠実適用 ──

    #[test]
    fn later_cues_apply_immediately_without_overwrite_guard() {
        let mut state = TextLayerState::default();
        // talk 1 の途中に talk 2 の cue 列（Clear→Text）が届いても、そのまま忠実に適用される。
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("talk1".into())));
        state.apply_cue(&cue("0", 0.1, CueCommand::Clear));
        state.apply_cue(&cue("0", 0.1, CueCommand::Text("talk2".into())));

        let expected: Vec<TextItem> = "talk2".chars().map(|ch| TextItem::Glyph { ch }).collect();
        assert_eq!(items_of(&state, "0"), expected.as_slice());
    }

    // ── R2.4/R2.5: 純粋・決定論（同一 cue 列→同一状態） ──

    #[test]
    fn same_cue_sequence_yields_identical_state() {
        let sequence = vec![
            cue("0", 0.0, CueCommand::Text("アヒルやアヒル！".into())),
            cue("0", 0.4, CueCommand::NewLine { ratio: 1.0 }),
            cue("1", 0.5, CueCommand::Text("なんやそれ".into())),
            cue("0", 0.8, CueCommand::Text("ガーガー".into())),
            cue("1", 0.9, CueCommand::Clear),
            cue(
                "0",
                1.0,
                CueCommand::Choice {
                    id: "yes".into(),
                    text: "はい".into(),
                },
            ),
            cue("1", 1.1, CueCommand::Text("……".into())),
        ];

        let mut a = TextLayerState::default();
        let mut b = TextLayerState::default();
        for c in &sequence {
            a.apply_cue(c);
        }
        for c in &sequence {
            b.apply_cue(c);
        }
        assert_eq!(a, b);
    }

    // ── Choice（M1 対象外・actor ごと初回のみ warn＋無視） ──

    /// WARN イベント数を数える最小 Subscriber（決定論的なログ檻・実時間非依存）。
    struct WarnCounter {
        warns: Arc<AtomicUsize>,
    }

    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.warns.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[test]
    fn choice_cue_is_ignored_and_warns_once_per_actor() {
        let warns = Arc::new(AtomicUsize::new(0));
        let subscriber = WarnCounter {
            warns: Arc::clone(&warns),
        };

        let state = tracing::subscriber::with_default(subscriber, || {
            let mut state = TextLayerState::default();
            let choice = |actor: &str, at: f64| {
                cue(
                    actor,
                    at,
                    CueCommand::Choice {
                        id: "yes".into(),
                        text: "はい".into(),
                    },
                )
            };
            state.apply_cue(&choice("0", 0.0));
            state.apply_cue(&choice("0", 0.1)); // 同一 actor 2 回目は warn しない
            state.apply_cue(&choice("1", 0.2)); // 別 actor は初回 warn
            state
        });

        // warn は actor ごと初回のみ（"0" で 1 回・"1" で 1 回）。
        assert_eq!(warns.load(Ordering::SeqCst), 2);
        // テキスト状態は汚さない（actor エントリも作らない）。
        assert!(state.actor_state(&ActorKey::from("0")).is_none());
        assert!(state.actor_state(&ActorKey::from("1")).is_none());
    }

    // ── Balloon 向けでない command の防御的無視 ──

    #[test]
    fn non_balloon_commands_do_not_disturb_state() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())));

        let before = state.clone();
        state.apply_cue(&cue(
            "0",
            0.1,
            CueCommand::Emote {
                key: "smile".into(),
            },
        ));
        state.apply_cue(&cue("0", 0.2, CueCommand::EntityRef(42)));
        // BalloonSurface（バルーン面切替）は表示系の消費対象＝文字状態機械へは配送しない（R3.2）。
        // 適用しても文字状態（items／reveal／visible_glyphs）は完全に不変。
        state.apply_cue(&cue(
            "0",
            0.3,
            CueCommand::BalloonSurface { key: "2".into() },
        ));
        assert_eq!(state, before);
    }

    // ── TextLayerConfig 既定値（design.md 正準: line_pitch 係数=1.25・char_wait は撤去済み） ──

    #[test]
    fn config_defaults_match_design_canon() {
        let config = TextLayerConfig::default();
        // char_wait は撤去済み（reveal は配送 duration 由来）——config は line_pitch_factor のみ。
        assert_eq!(config.line_pitch_factor, 1.25);
    }

    // ══ typewriter リビール進行（注入時刻駆動・R3／R7 系） ══
    //
    // reveal ペースは配送 duration 由来（`interval = duration / N`）。FP 誤差を排するため、
    // 2 の冪で正確に表現できる間隔（0.25）を主に使い、Text cue へ `N × 0.25` の duration を
    // 焼き込む（[`cue`] ヘルパが自動で行う）。期待リビール時刻は実装と同一の `D/N` 算術で
    // 成立し、非 2 冪の間隔（≈0.05）は安全マージン付き時刻で観測する。

    fn reveal_times_of(state: &TextLayerState, actor: &str) -> Vec<f64> {
        state
            .actor_state(&ActorKey::from(actor))
            .expect("actor state should exist")
            .reveal()
            .times()
            .to_vec()
    }

    // ── R3.1/R3.2/R3.3/R7.1: r_i 式（先頭 r_0 = at・以降 prev + interval）・注入時刻駆動 ──

    /// reveal interval は配送 duration 由来（`interval = duration / N`）。Text("アヒル") へ
    /// `cue` ヘルパが焼き込む duration = 3 × 0.25 = 0.75 ゆえ interval = 0.75/3 = 0.25。
    #[test]
    fn reveal_times_follow_duration_derived_interval_from_chunk_start() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())));
        // r_0 = at(chunk(0)) = 1.0・以降 prev + interval(=duration/N=0.25)。
        assert_eq!(reveal_times_of(&state, "0"), vec![1.0, 1.25, 1.5]);
    }

    #[test]
    fn visible_glyphs_progress_one_by_one_with_injected_time() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())));

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 0.0), 0);
        assert_eq!(state.visible_glyphs(&actor, 1.0), 1); // r_0 <= t で可視
        assert_eq!(state.visible_glyphs(&actor, 1.24), 1);
        assert_eq!(state.visible_glyphs(&actor, 1.25), 2);
        assert_eq!(state.visible_glyphs(&actor, 1.5), 3);
        assert_eq!(state.visible_glyphs(&actor, 100.0), 3); // 末尾到達後は飽和
    }

    /// 非 2 冪の配送 duration でも進行する（丸め安全マージン付き時刻で観測）。
    /// duration=0.15・N=3 → interval = 0.15/3 ≈ 0.05（f64 除算・厳密ビット等価は主張しない）。
    /// リビール時刻 r ≈ [1.0, 1.05, 1.10] を、±0.01 マージン付き注入時刻で観測する
    /// （旧 0.05 リテラル由来の期待値でなく `D/N` 由来の近似時刻＋マージンで固定・FP flaky 回避）。
    #[test]
    fn visible_glyphs_progress_with_duration_derived_interval() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue_dur("0", 1.0, 0.15, CueCommand::Text("アヒル".into())));

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 0.99), 0);
        assert_eq!(state.visible_glyphs(&actor, 1.0), 1);
        assert_eq!(state.visible_glyphs(&actor, 1.06), 2); // r_1 ≈ 1.05
        assert_eq!(state.visible_glyphs(&actor, 1.11), 3); // r_2 ≈ 1.10
    }

    // ── R3.4: at は下限（それより早く可視化しない）・後続 chunk が未来なら待つ ──

    #[test]
    fn glyphs_never_visible_before_chunk_start() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
        state.apply_cue(&cue("0", 10.0, CueCommand::Text("cd".into())));

        // chunk 2 の r は max(0.25+0.25, 10.0)=10.0 起点——前 chunk 完了済みでも 10.0 まで待つ。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 10.0, 10.25]);

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 5.0), 2);
        assert_eq!(state.visible_glyphs(&actor, 9.99), 2);
        assert_eq!(state.visible_glyphs(&actor, 10.0), 3);
        assert_eq!(state.visible_glyphs(&actor, 10.25), 4);
    }

    /// R3.4 後段: 直前 chunk が未リビールでも、リビールカーソルは配送 duration が定める
    /// ペース（interval）でバッファ末尾を追う（at が過去でも max が prev+interval を選ぶ）。
    #[test]
    fn reveal_cursor_chases_tail_when_next_chunk_start_is_earlier() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
        // 前 chunk の末尾 r_3=0.75 が未来のうちに次 chunk（at=0.1）が届く。
        state.apply_cue(&cue("0", 0.1, CueCommand::Text("ef".into())));

        assert_eq!(
            reveal_times_of(&state, "0"),
            vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.25]
        );

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 0.9), 4); // chunk 境界で加速しない
        assert_eq!(state.visible_glyphs(&actor, 1.0), 5);
    }

    /// リビール時刻列は常に単調非減少（RevealSchedule の不変条件）。
    #[test]
    fn reveal_times_are_monotonic_non_decreasing() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 2.0, CueCommand::Text("abc".into())));
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("de".into()))); // at が過去
        state.apply_cue(&cue("0", 9.0, CueCommand::Text("f".into()))); // at が未来

        let times = reveal_times_of(&state, "0");
        assert!(times.windows(2).all(|w| w[0] <= w[1]), "times: {times:?}");
    }

    // ── R3.6: リビール中の後続 cue も後出し優先で即時反映 ──

    /// 追記: リビール中の Text 追記は items へ即時反映され、schedule は末尾を追う。
    #[test]
    fn text_append_during_reveal_applies_immediately() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
        state.apply_cue(&cue("0", 0.3, CueCommand::Text("ef".into())));

        // items は即時に 6 グリフ（未リビール分も保持＝無損失）。
        assert_eq!(items_of(&state, "0").len(), 6);
        assert_eq!(reveal_times_of(&state, "0").len(), 6);
    }

    /// 改行: LineBreak はリビール枠（時刻）を消費しない——schedule はグリフのみ対象。
    #[test]
    fn line_break_takes_no_reveal_slot() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
        state.apply_cue(&cue("0", 0.25, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue("0", 0.25, CueCommand::Text("cd".into())));

        // items 5 件（グリフ 4＋改行 1）・times はグリフ 4 件分のみ。
        assert_eq!(items_of(&state, "0").len(), 5);
        // 改行マーカーは reveal 枠（interval）を消費しない: c は max(0.25+0.25, 0.25)=0.5。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.5, 0.75]);
    }

    /// 全消去: リビール中の Clear は未リビール分を含め schedule ごと破棄し、
    /// 以後の可視数は 0。次 chunk のリビールは旧 tail に影響されず at 起点で再開。
    #[test]
    fn clear_during_reveal_discards_unrevealed_and_resets_pacing() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
        state.apply_cue(&cue("0", 0.3, CueCommand::Clear)); // r_2/r_3 未リビールのまま消去

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 0.3), 0);
        assert_eq!(state.visible_glyphs(&actor, 100.0), 0);

        // 新 chunk は旧 tail（0.75）でなく自身の at=0.5 から: r = [0.5, 0.75]。
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("xy".into())));
        assert_eq!(reveal_times_of(&state, "0"), vec![0.5, 0.75]);
        assert_eq!(state.visible_glyphs(&actor, 0.4), 0);
        assert_eq!(state.visible_glyphs(&actor, 0.5), 1);
    }

    // ── R3.5/10.2: 決定論（同一 cue 列＋同一注入時刻列→各時刻の可視数が常に一致） ──

    #[test]
    fn same_cues_and_times_yield_identical_visible_counts() {
        let sequence = vec![
            cue("0", 0.0, CueCommand::Text("アヒルやアヒル！".into())),
            cue("0", 0.4, CueCommand::NewLine { ratio: 1.0 }),
            cue("1", 0.5, CueCommand::Text("なんやそれ".into())),
            cue("0", 0.8, CueCommand::Text("ガーガー".into())),
            cue("1", 0.9, CueCommand::Clear),
            cue("1", 1.1, CueCommand::Text("……".into())),
        ];
        // 注入時刻列（フレーム時刻のつもり・cue 境界を跨ぐサンプル点）。
        let probe_times: Vec<f64> = (0..40).map(|i| i as f64 * 0.05).collect();

        let mut a = TextLayerState::default();
        let mut b = TextLayerState::default();
        for c in &sequence {
            a.apply_cue(c);
            b.apply_cue(c);
        }

        for actor in [ActorKey::from("0"), ActorKey::from("1")] {
            for &t in &probe_times {
                assert_eq!(
                    a.visible_glyphs(&actor, t),
                    b.visible_glyphs(&actor, t),
                    "actor {actor} at t={t}"
                );
            }
        }
        assert_eq!(a, b);
    }

    // ── 境界: 空テキスト・未知 actor・複数 chunk 連結 ──

    #[test]
    fn empty_text_cue_adds_no_reveal_times() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("".into())));

        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25]);
        // 空 chunk は tail も動かさない: 次 chunk は通常式のまま。
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("c".into())));
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.5]);
    }

    #[test]
    fn visible_glyphs_of_unknown_actor_is_zero() {
        let state = TextLayerState::default();
        assert_eq!(state.visible_glyphs(&ActorKey::from("9"), 42.0), 0);
    }

    /// 可視数は actor 独立（他 actor のリビール進行に影響されない）。
    #[test]
    fn visible_glyphs_are_independent_per_actor() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
        state.apply_cue(&cue("1", 1.0, CueCommand::Text("xy".into())));

        assert_eq!(state.visible_glyphs(&ActorKey::from("0"), 0.5), 3);
        assert_eq!(state.visible_glyphs(&ActorKey::from("1"), 0.5), 0);
        assert_eq!(state.visible_glyphs(&ActorKey::from("1"), 1.0), 1);
    }

    // ══ 服従契約の縮退（1.2/7.3）と honor no-op（2.2/7.5）══

    /// 縮退（1.2/7.3）: 配送 duration=0（瞬時／後方互換 cue）かつ N≥1 は interval=0 ゆえ
    /// 全グリフが `cue.at` で**同時**可視になる（旧 char_wait 実装は 0.05 刻みで 1 グリフずつ
    /// 出すため、この同時可視は duration 服従後にのみ成立する）。
    #[test]
    fn zero_duration_reveals_all_glyphs_simultaneously_at_cue_at() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue_dur("0", 1.0, 0.0, CueCommand::Text("アヒル".into())));
        let actor = ActorKey::from("0");

        // reveal 時刻は全て cue.at（interval=0 ゆえ max(prev+0, at)=at）。
        assert_eq!(reveal_times_of(&state, "0"), vec![1.0, 1.0, 1.0]);
        assert_eq!(state.visible_glyphs(&actor, 0.99), 0);
        assert_eq!(
            state.visible_glyphs(&actor, 1.0),
            3,
            "D=0＋N=3 は全グリフが cue.at で同時可視"
        );
    }

    /// 縮退（1.8/7.3）: N=0（空テキスト）は duration が非零でも追記せず、除算（duration/0）を
    /// 行わない。`cue` ヘルパは空テキストへ duration=0 を焼くため、ここでは敵対的な
    /// 「空テキスト＋非零 duration」を [`cue_dur`] で直接与え、0 割り・追記なしを固定する。
    #[test]
    fn empty_text_with_nonzero_duration_adds_nothing_and_never_divides() {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
        // 空テキスト＋非零 duration（dola ingress を経ずに来た敵対的 cue を想定）。
        state.apply_cue(&cue_dur("0", 0.5, 5.0, CueCommand::Text("".into())));

        // 追記なし・reveal 時刻も増えない（0 割りせず panic もしない）。
        assert_eq!(items_of(&state, "0").len(), 2, "空テキストは追記しない");
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25]);
        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 100.0), 2);
    }

    /// honor no-op（2.2/2.3/7.5）: 担当外の cue（Emote／Wait）は action を無視するのみで、
    /// その duration から**新たなローカル遅延を生じさせない**——後続の担当 Text cue の
    /// reveal は、担当外 cue を挟まない対照実行と**完全に一致**する（葉の否定的 no-op）。
    #[test]
    fn non_relevant_cue_adds_no_local_delay_to_following_text_reveal() {
        // 対照（担当外 cue なし）。
        let mut control = TextLayerState::default();
        control.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
        control.apply_cue(&cue("0", 0.5, CueCommand::Text("い".into())));

        // 実験（間に Emote／Wait を巨大 duration で挿入）。担当外ゆえ reveal に効いてはならない。
        let mut experiment = TextLayerState::default();
        experiment.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
        experiment.apply_cue(&cue_dur(
            "0",
            0.25,
            100.0,
            CueCommand::Emote {
                key: "smile".into(),
            },
        ));
        experiment.apply_cue(&cue_dur("0", 0.4, 100.0, CueCommand::Wait));
        experiment.apply_cue(&cue("0", 0.5, CueCommand::Text("い".into())));

        assert_eq!(
            reveal_times_of(&experiment, "0"),
            reveal_times_of(&control, "0"),
            "担当外 cue（Emote/Wait）の duration は後続 Text の reveal を一切遅らせない"
        );

        // 直接値でも固定: 2 つ目 Text の r_0 は自身の at=0.5（担当外 duration 100 が乗らない）。
        // 1 つ目 "あ"(N=1,dur0.25) の r_0=0.0・tail=0.0 → "い"(N=1,dur0.25) r = max(0.0+0.25, 0.5)=0.5。
        assert_eq!(reveal_times_of(&experiment, "0"), vec![0.0, 0.5]);
        assert_eq!(experiment.visible_glyphs(&ActorKey::from("0"), 0.5), 2);
    }
}
