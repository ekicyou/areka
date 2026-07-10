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
//! - **typewriter リビール（R3 系）**: `Text` 追記時に per-glyph リビール時刻を
//!   `r_i = max(r_{i-1} + char_wait, at(chunk(i)))`（先頭は `r_0 = at`）で確定し、
//!   可視数は `visible_glyphs(actor, t)`＝注入時刻 `t` で `r_i <= t` のグリフ数。
//!   実時間 sleep／`Instant` 不使用（注入時刻駆動・R3.3）。
//! - グリフ単位は Rust の `char`（M1 正準。書記素クラスタ結合は M2 検討事項——
//!   emo2 fixture は結合文字を使用しない）。
//! - 同一 cue 列＋同一入力条件→同一状態（決定論・R2.4/R2.5）。

use std::collections::{BTreeMap, BTreeSet};

use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

/// テキスト層の調整値（design.md「TextLayerRuntime」の config 正本）。
///
/// 純粋層・結線層の双方が消費する共有設定。値は design.md の正準既定
/// （`char_wait`＝0.05 s・`line_pitch`＝`ceil(font.height × 1.25)` の係数 1.25）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayerConfig {
    /// per-glyph の文字送り間隔（秒）。typewriter リビール時刻式
    /// `r_i = max(r_{i-1} + char_wait, at(chunk(i)))` の `char_wait`（既定 0.05）。
    pub char_wait: f64,
    /// 行送りピッチ係数（`line_pitch = ceil(font.height × 係数)`・既定 1.25）。
    pub line_pitch_factor: f32,
}

impl Default for TextLayerConfig {
    fn default() -> Self {
        Self {
            char_wait: 0.05,
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

/// per-glyph リビール時刻列（注入時刻駆動 typewriter・R3.1–3.5）。
///
/// 時刻式（design.md「typewriter リビール」正本・決定論の正準）:
/// `r_i = max(r_{i-1} + char_wait, at(chunk(i)))`（先頭グリフは `r_0 = at(chunk(0))`）。
/// 可視数は `visible(t) = |{ i : r_i <= t }|`。`at` は下限（それより早く可視化しない・
/// R3.4）であり、リビールカーソルは本層ペース（`char_wait`）でバッファ末尾を追う
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
    /// `r_i = max(r_{i-1} + char_wait, chunk_start)`・先頭（schedule が空）のみ
    /// `r_0 = chunk_start`。改行マーカーはグリフでないため本 schedule の対象外
    /// （`char_wait` を消費しない）。
    fn extend_chunk(&mut self, glyph_count: usize, chunk_start: f64, char_wait: f64) {
        self.times.reserve(glyph_count);
        for _ in 0..glyph_count {
            let r = match self.times.last() {
                Some(&prev) => (prev + char_wait).max(chunk_start),
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
    /// リビール開始の下限・R3.4）と `config.char_wait` の時刻式で確定する（R3.1–3.3）。
    /// リビール中の後続 cue も即時適用される（R3.6）——本層は cue の `at` を読む
    /// だけで書き換えず、pacing が cue 時刻へ影響することはない（R10.2）。
    pub fn apply_cue(&mut self, cue: &TalkCue, config: &TextLayerConfig) {
        match &cue.command {
            CueCommand::Text(text) => {
                let glyph_count = text.chars().count();
                tracing::debug!(actor = %cue.actor, len = glyph_count, at = cue.at, "Text cue 適用（追記＋リビール時刻確定）");
                let state = self.actors.entry(cue.actor.clone()).or_default();
                state
                    .items
                    .extend(text.chars().map(|ch| TextItem::Glyph { ch }));
                state.reveal.extend_chunk(glyph_count, cue.at, config.char_wait);
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
            CueCommand::Choice { .. } => {
                // M1 対象外（choice-render シーム）: actor ごと初回のみ warn!＋無視・状態は汚さない。
                if self.choice_warned.insert(cue.actor.clone()) {
                    tracing::warn!(actor = %cue.actor, "Choice cue は M1 未対応のため無視する（choice-render シーム）");
                }
            }
            // Balloon 向けでない command（cue_target_of が Shell/None に分類）は本状態機械の
            // 消費対象外——上流 routing の責務。防御的に無視する（catch-all を置かず、dola の
            // variant 追加時にコンパイラが再検討を強制する）。
            CueCommand::Emote { .. } | CueCommand::EntityRef(..) | CueCommand::Custom { .. } => {
                tracing::debug!(actor = %cue.actor, command = ?cue.command, "Balloon 向けでない cue を無視（上流 routing の対象外流入）");
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

    /// テスト用 cue 生成ヘルパ。
    fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(actor),
            command,
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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())), &config);
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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒ".into())), &config);
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("ルや".into())), &config);
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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("aあ🦆".into())), &config);
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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())), &config);
        state.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }), &config);
        state.apply_cue(&cue("0", 0.2, CueCommand::NewLine { ratio: 0.5 }), &config);
        state.apply_cue(&cue("0", 0.3, CueCommand::Text("B".into())), &config);
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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒルや".into())), &config);
        state.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }), &config);
        state.apply_cue(&cue("0", 0.2, CueCommand::Text("アヒル！".into())), &config);
        state.apply_cue(&cue("0", 0.3, CueCommand::Clear), &config);

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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("さくら".into())), &config);
        state.apply_cue(&cue("1", 0.0, CueCommand::Text("けろ".into())), &config);
        state.apply_cue(&cue("0", 0.5, CueCommand::Clear), &config);

        assert!(items_of(&state, "0").is_empty());
        assert_eq!(
            items_of(&state, "1"),
            &[TextItem::Glyph { ch: 'け' }, TextItem::Glyph { ch: 'ろ' }]
        );
    }

    // ── R1.6: actor 別振り分け・独立状態・lazily 生成 ──

    #[test]
    fn cues_route_to_independent_actor_states() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())), &config);
        state.apply_cue(&cue("1", 0.1, CueCommand::Text("B".into())), &config);
        state.apply_cue(&cue("0", 0.2, CueCommand::Text("C".into())), &config);

        assert_eq!(
            items_of(&state, "0"),
            &[TextItem::Glyph { ch: 'A' }, TextItem::Glyph { ch: 'C' }]
        );
        assert_eq!(items_of(&state, "1"), &[TextItem::Glyph { ch: 'B' }]);
    }

    #[test]
    fn unknown_actor_state_lazily_created_and_accumulates() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default();
        assert!(state.actor_state(&ActorKey::from("7")).is_none());

        state.apply_cue(&cue("7", 0.0, CueCommand::Text("x".into())), &config);
        assert_eq!(items_of(&state, "7"), &[TextItem::Glyph { ch: 'x' }]);
    }

    #[test]
    fn actors_iterate_in_deterministic_key_order() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default();
        // 逆順に生成しても走査は ActorKey 昇順（決定論的順序）。
        state.apply_cue(&cue("1", 0.0, CueCommand::Text("b".into())), &config);
        state.apply_cue(&cue("0", 0.1, CueCommand::Text("a".into())), &config);

        let keys: Vec<&ActorKey> = state.actors().map(|(k, _)| k).collect();
        assert_eq!(keys, vec![&ActorKey::from("0"), &ActorKey::from("1")]);
    }

    // ── R10.5: 上書きガードなし・後出し優先の忠実適用 ──

    #[test]
    fn later_cues_apply_immediately_without_overwrite_guard() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default();
        // talk 1 の途中に talk 2 の cue 列（Clear→Text）が届いても、そのまま忠実に適用される。
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("talk1".into())), &config);
        state.apply_cue(&cue("0", 0.1, CueCommand::Clear), &config);
        state.apply_cue(&cue("0", 0.1, CueCommand::Text("talk2".into())), &config);

        let expected: Vec<TextItem> = "talk2".chars().map(|ch| TextItem::Glyph { ch }).collect();
        assert_eq!(items_of(&state, "0"), expected.as_slice());
    }

    // ── R2.4/R2.5: 純粋・決定論（同一 cue 列→同一状態） ──

    #[test]
    fn same_cue_sequence_yields_identical_state() {
        let config = TextLayerConfig::default();
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
            a.apply_cue(c, &config);
        }
        for c in &sequence {
            b.apply_cue(c, &config);
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
            let config = TextLayerConfig::default();
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
            state.apply_cue(&choice("0", 0.0), &config);
            state.apply_cue(&choice("0", 0.1), &config); // 同一 actor 2 回目は warn しない
            state.apply_cue(&choice("1", 0.2), &config); // 別 actor は初回 warn
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
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())), &config);

        let before = state.clone();
        state.apply_cue(
            &cue("0", 0.1, CueCommand::Emote { key: "smile".into() }),
            &config,
        );
        state.apply_cue(&cue("0", 0.2, CueCommand::EntityRef(42)), &config);
        assert_eq!(state, before);
    }

    // ── TextLayerConfig 既定値（design.md 正準: char_wait=0.05 / line_pitch 係数=1.25） ──

    #[test]
    fn config_defaults_match_design_canon() {
        let config = TextLayerConfig::default();
        assert_eq!(config.char_wait, 0.05);
        assert_eq!(config.line_pitch_factor, 1.25);
    }

    // ══ typewriter リビール進行（注入時刻駆動・R3 系） ══
    //
    // 以下のテストは FP 誤差を排するため、2 の冪で正確に表現できる char_wait
    // （0.25／0.5）を主に使う（既定 0.05 の検証は安全マージン付き時刻で行う）。

    /// FP 丸めに依存しない検証用 config（char_wait=0.25 は 2 の冪＝正確表現）。
    fn exact_config() -> TextLayerConfig {
        TextLayerConfig {
            char_wait: 0.25,
            ..TextLayerConfig::default()
        }
    }

    fn reveal_times_of(state: &TextLayerState, actor: &str) -> Vec<f64> {
        state
            .actor_state(&ActorKey::from(actor))
            .expect("actor state should exist")
            .reveal()
            .times()
            .to_vec()
    }

    // ── R3.1/R3.2/R3.3: r_i 式（先頭 r_0 = at・以降 prev + char_wait）・注入時刻駆動 ──

    #[test]
    fn reveal_times_follow_char_wait_formula_from_chunk_start() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())), &config);
        // r_0 = at(chunk(0)) = 1.0・以降 prev + 0.25。
        assert_eq!(reveal_times_of(&state, "0"), vec![1.0, 1.25, 1.5]);
    }

    #[test]
    fn visible_glyphs_progress_one_by_one_with_injected_time() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())), &config);

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 0.0), 0);
        assert_eq!(state.visible_glyphs(&actor, 1.0), 1); // r_0 <= t で可視
        assert_eq!(state.visible_glyphs(&actor, 1.24), 1);
        assert_eq!(state.visible_glyphs(&actor, 1.25), 2);
        assert_eq!(state.visible_glyphs(&actor, 1.5), 3);
        assert_eq!(state.visible_glyphs(&actor, 100.0), 3); // 末尾到達後は飽和
    }

    /// 既定 config（char_wait=0.05）でも進行する（丸め安全マージン付き時刻で観測）。
    #[test]
    fn visible_glyphs_progress_with_default_char_wait() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())), &config);

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
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())), &config);
        state.apply_cue(&cue("0", 10.0, CueCommand::Text("cd".into())), &config);

        // chunk 2 の r は max(0.25+0.25, 10.0)=10.0 起点——前 chunk 完了済みでも 10.0 まで待つ。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 10.0, 10.25]);

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 5.0), 2);
        assert_eq!(state.visible_glyphs(&actor, 9.99), 2);
        assert_eq!(state.visible_glyphs(&actor, 10.0), 3);
        assert_eq!(state.visible_glyphs(&actor, 10.25), 4);
    }

    /// R3.4 後段: 直前 chunk が未リビールでも、リビールカーソルは本層ペース
    /// （char_wait）でバッファ末尾を追う（at が過去でも max が prev+char_wait を選ぶ）。
    #[test]
    fn reveal_cursor_chases_tail_when_next_chunk_start_is_earlier() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())), &config);
        // 前 chunk の末尾 r_3=0.75 が未来のうちに次 chunk（at=0.1）が届く。
        state.apply_cue(&cue("0", 0.1, CueCommand::Text("ef".into())), &config);

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
        let config = exact_config();
        state.apply_cue(&cue("0", 2.0, CueCommand::Text("abc".into())), &config);
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("de".into())), &config); // at が過去
        state.apply_cue(&cue("0", 9.0, CueCommand::Text("f".into())), &config); // at が未来

        let times = reveal_times_of(&state, "0");
        assert!(times.windows(2).all(|w| w[0] <= w[1]), "times: {times:?}");
    }

    // ── R3.6: リビール中の後続 cue も後出し優先で即時反映 ──

    /// 追記: リビール中の Text 追記は items へ即時反映され、schedule は末尾を追う。
    #[test]
    fn text_append_during_reveal_applies_immediately() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())), &config);
        state.apply_cue(&cue("0", 0.3, CueCommand::Text("ef".into())), &config);

        // items は即時に 6 グリフ（未リビール分も保持＝無損失）。
        assert_eq!(items_of(&state, "0").len(), 6);
        assert_eq!(reveal_times_of(&state, "0").len(), 6);
    }

    /// 改行: LineBreak はリビール枠（時刻）を消費しない——schedule はグリフのみ対象。
    #[test]
    fn line_break_takes_no_reveal_slot() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())), &config);
        state.apply_cue(&cue("0", 0.25, CueCommand::NewLine { ratio: 1.0 }), &config);
        state.apply_cue(&cue("0", 0.25, CueCommand::Text("cd".into())), &config);

        // items 5 件（グリフ 4＋改行 1）・times はグリフ 4 件分のみ。
        assert_eq!(items_of(&state, "0").len(), 5);
        // 改行マーカーは char_wait を消費しない: c は max(0.25+0.25, 0.25)=0.5。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.5, 0.75]);
    }

    /// 全消去: リビール中の Clear は未リビール分を含め schedule ごと破棄し、
    /// 以後の可視数は 0。次 chunk のリビールは旧 tail に影響されず at 起点で再開。
    #[test]
    fn clear_during_reveal_discards_unrevealed_and_resets_pacing() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())), &config);
        state.apply_cue(&cue("0", 0.3, CueCommand::Clear), &config); // r_2/r_3 未リビールのまま消去

        let actor = ActorKey::from("0");
        assert_eq!(state.visible_glyphs(&actor, 0.3), 0);
        assert_eq!(state.visible_glyphs(&actor, 100.0), 0);

        // 新 chunk は旧 tail（0.75）でなく自身の at=0.5 から: r = [0.5, 0.75]。
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("xy".into())), &config);
        assert_eq!(reveal_times_of(&state, "0"), vec![0.5, 0.75]);
        assert_eq!(state.visible_glyphs(&actor, 0.4), 0);
        assert_eq!(state.visible_glyphs(&actor, 0.5), 1);
    }

    // ── R3.5/10.2: 決定論（同一 cue 列＋同一注入時刻列→各時刻の可視数が常に一致） ──

    #[test]
    fn same_cues_and_times_yield_identical_visible_counts() {
        let config = TextLayerConfig::default();
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
            a.apply_cue(c, &config);
            b.apply_cue(c, &config);
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
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())), &config);
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("".into())), &config);

        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25]);
        // 空 chunk は tail も動かさない: 次 chunk は通常式のまま。
        state.apply_cue(&cue("0", 0.5, CueCommand::Text("c".into())), &config);
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
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())), &config);
        state.apply_cue(&cue("1", 1.0, CueCommand::Text("xy".into())), &config);

        assert_eq!(state.visible_glyphs(&ActorKey::from("0"), 0.5), 3);
        assert_eq!(state.visible_glyphs(&ActorKey::from("1"), 0.5), 0);
        assert_eq!(state.visible_glyphs(&ActorKey::from("1"), 1.0), 1);
    }
}
