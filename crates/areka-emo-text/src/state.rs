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
//! - **typewriter リビール（R3 系・item 単位モデル）**: `Text`／`NewLine` 追記時に item
//!   （グリフ**と改行マーカー**）ごとの リビール時刻を `r_i = max(r_{i-1} + wait_i, at_i)`
//!   （先頭は `r_0 = at_0`）で確定する。グリフの `wait_i = char_wait`（clause 句読点直後は
//!   `+ punctuation_wait`・#3）、改行の `wait_i = 0`（＝`max(prev_reveal, cue.at)`・#4——先行
//!   `\w` を sakura が NewLine の `at` へ畳み込むため改行が `\w` の分だけ遅れる）。可視 item 数は
//!   `visible_items(actor, t)`＝`r_i <= t` の item prefix 長・可視グリフ数は `visible_glyphs`。
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
    /// `r_i = max(r_{i-1} + wait_i, at(chunk(i)))` の基本 `wait_i`（既定 0.05）。
    pub char_wait: f64,
    /// 句読点直後の追加ポーズ（秒）。clause 句読点（[`is_clause_punctuation`]・、。！？…等）に
    /// **続く**グリフの `wait_i` へ `char_wait` に加えて上乗せする（#3 実走欠陥修正）。
    /// SSP の 、。 自動ポーズ相当——emo2 の pasta script は明示 `\w` を打たない前提。
    /// 既定は char_wait の数倍で「、。 で目に見えて一拍おく」値（開発者が実走で微調整）。
    pub punctuation_wait: f64,
    /// 行送りピッチ係数（`line_pitch = ceil(font.height × 係数)`・既定 1.25）。
    pub line_pitch_factor: f32,
}

impl Default for TextLayerConfig {
    fn default() -> Self {
        Self {
            char_wait: 0.05,
            punctuation_wait: 0.3,
            line_pitch_factor: 1.25,
        }
    }
}

/// clause 句読点か（#3: 直後のグリフに追加ポーズ [`TextLayerConfig::punctuation_wait`] を課す集合）。
///
/// SSP の 、。 自動ポーズ相当の de-facto 集合（ukadoc は自動ウェイトを「SSP ユーザー設定で可変」と
/// するのみで文字集合を列挙しない＝典拠なし）。全角の読点/句点/感嘆/疑問/リーダ/中点/波ダッシュと
/// その ASCII 等価を含む。第一版の素朴集合——開発者が実走の手触りで調整する前提（config で可変）。
pub fn is_clause_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '、' | '。' | '，' | '．' | '！' | '？' | '…' | '‥' | '・' | '〜' | '～'
            | ',' | '.' | '!' | '?'
    )
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

/// per-item リビール時刻列（注入時刻駆動 typewriter・R3.1–3.5）。
///
/// **item 単位モデル**（実走欠陥 #3/#4 修正）: グリフ**と改行マーカー**の双方に reveal 時刻を
/// 割り、[`ActorTextState::items`] と index 整合させる（`times[k]` は `items[k]` のリビール時刻）。
///
/// 時刻式（design.md「typewriter リビール」正本・決定論の正準）:
/// `r_i = max(r_{i-1} + wait_i, at_i)`（先頭 item は `r_0 = at_0`）。ここで
/// - グリフの `wait_i = char_wait`（直前 item が clause 句読点グリフなら `+ punctuation_wait`・#3）、
/// - 改行マーカーの `wait_i = 0`（＝`r_lb = max(prev_reveal, cue.at)`・#4——sakura が先行 `\w` を
///   NewLine の `at` へ畳み込むため、改行は `\w` の分だけ遅れる）。
///
/// 可視 item 数は `visible(t) = |{ i : r_i <= t }|`。`at` は下限（それより早く可視化しない・
/// R3.4）であり、リビールカーソルは本層ペース（`char_wait`）でバッファ末尾を追う
/// （長文時は遅延しうる・無損失）。時刻は常に注入（talk 起点相対秒）で、実時間
/// （`Instant`／sleep）には依存しない（R3.3）。`Clear` は schedule ごと初期化
/// （未リビール分を含む破棄・R2.3/R3.6）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RevealSchedule {
    /// item i のリビール時刻 `r_i`（talk 起点相対秒・単調非減少・`items` と index 整合）。
    times: Vec<f64>,
}

impl RevealSchedule {
    /// リビール時刻列（talk 起点相対秒・`items` と index 整合）。
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// リビール時刻が 1 件も無いか。
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// 注入時刻 `t` での可視 item 数 `visible(t) = |{ i : r_i <= t }|`（R3.5）。
    ///
    /// 返り値はグリフ＋改行を通した item prefix 長。`times` は単調非減少
    /// （[`Self::push_item`] の max が保証）なので二分探索で数える。
    pub fn visible(&self, t: f64) -> usize {
        self.times.partition_point(|&r| r <= t)
    }

    /// item 1 件分のリビール時刻を時刻式 `r = max(prev + min_wait, at)` で追記する。
    ///
    /// 先頭（schedule が空）のみ `r_0 = at`（`min_wait` は先行 item が無いため不使用）。
    /// `min_wait` は「直前 item から本 item までの最小間隔」——グリフは
    /// `char_wait`（句読点直後は `+ punctuation_wait`）、改行マーカーは `0`。
    fn push_item(&mut self, min_wait: f64, at: f64) {
        let r = match self.times.last() {
            Some(&prev) => (prev + min_wait).max(at),
            None => at,
        };
        self.times.push(r);
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
    /// `Text`／`NewLine` は追記と同時に item 単位のリビール時刻を `cue.at`（リビール開始の
    /// 下限・R3.4）と時刻式 `r_i = max(r_{i-1} + wait_i, at_i)` で確定する（R3.1–3.3）。改行も
    /// 自身の reveal 枠を持ち（`wait=0`＝`max(prev_reveal, cue.at)`・#4）、句読点直後のグリフは
    /// `punctuation_wait` の追加ポーズを負う（#3）。リビール中の後続 cue も即時適用される（R3.6）
    /// ——本層は cue の `at` を読むだけで書き換えず、pacing が cue 時刻へ影響しない（R10.2）。
    pub fn apply_cue(&mut self, cue: &TalkCue, config: &TextLayerConfig) {
        match &cue.command {
            CueCommand::Text(text) => {
                let glyph_count = text.chars().count();
                tracing::debug!(actor = %cue.actor, len = glyph_count, at = cue.at, "Text cue 適用（追記＋item リビール時刻確定）");
                let state = self.actors.entry(cue.actor.clone()).or_default();
                state.items.reserve(glyph_count);
                for ch in text.chars() {
                    // #3: 直前 item が clause 句読点グリフなら、本グリフの wait に punctuation_wait を上乗せ
                    //（SSP の 、。 自動ポーズ相当）。直前が改行/非句読点/先頭なら素の char_wait のみ。
                    let prev_is_punct = matches!(
                        state.items.last(),
                        Some(TextItem::Glyph { ch: prev }) if is_clause_punctuation(*prev)
                    );
                    let min_wait = config.char_wait
                        + if prev_is_punct { config.punctuation_wait } else { 0.0 };
                    state.reveal.push_item(min_wait, cue.at);
                    state.items.push(TextItem::Glyph { ch });
                }
            }
            CueCommand::NewLine { ratio } => {
                tracing::debug!(actor = %cue.actor, ratio, at = cue.at, "NewLine cue 適用（改行マーカー追記＋item リビール時刻確定）");
                let state = self.actors.entry(cue.actor.clone()).or_default();
                // #4: 改行は自身の reveal 枠 `r = max(prev_reveal, cue.at)`（min_wait=0）を持つ。
                // sakura が先行 `\w` を NewLine の at へ畳み込むため、改行は `\w` の分だけ遅れる。
                state.reveal.push_item(0.0, cue.at);
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
            // 文字状態機械が消費しない command（cue_target_of が Shell/None に分類）は本状態機械の
            // 対象外——上流 routing の責務。防御的に無視する（catch-all を置かず、dola の
            // variant 追加時にコンパイラが再検討を強制する）。`BalloonSurface` は表示系
            // （SurfaceSink/seriko）の消費対象＝文字状態機械へは配送しない防御面（R3.2）。
            CueCommand::Emote { .. }
            | CueCommand::EntityRef(..)
            | CueCommand::Custom { .. }
            | CueCommand::BalloonSurface { .. } => {
                tracing::debug!(actor = %cue.actor, command = ?cue.command, "文字状態機械が消費しない cue を無視（上流 routing の対象外流入）");
            }
        }
    }

    /// 注入時刻 `t` での actor 別可視 **item** 数（グリフ＋改行の通し prefix 長・決定論・R3.5）。
    ///
    /// `visible_items(t) = |{ i : r_i <= t }|`。レイアウト（[`crate::layout::LayoutEngine::layout`]）へ
    /// 渡す**可視 item prefix 長**——グリフだけでなくリビール済み改行も含めて先頭から数える
    /// （item 単位モデル・#4）。未生成の actor は 0。同一 cue 列＋同一注入時刻列→同一値（決定論檻）。
    pub fn visible_items(&self, actor: &ActorKey, t: f64) -> usize {
        self.actors
            .get(actor)
            .map_or(0, |state| state.reveal.visible(t))
    }

    /// 注入時刻 `t` での actor 別可視グリフ数（改行を除いたグリフのみ・決定論・R3.5）。
    ///
    /// 可視 item prefix（[`Self::visible_items`]）のうち [`TextItem::Glyph`] だけを数える
    /// （改行マーカーは含めない）。未生成の actor は 0。「実際に何文字表示されたか」の観測口
    /// （レイアウトへは [`Self::visible_items`] を渡す——本口はグリフ数の檻/診断向け）。
    pub fn visible_glyphs(&self, actor: &ActorKey, t: f64) -> usize {
        self.actors.get(actor).map_or(0, |state| {
            let visible = state.reveal.visible(t);
            state.items[..visible]
                .iter()
                .filter(|item| matches!(item, TextItem::Glyph { .. }))
                .count()
        })
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
        // BalloonSurface（バルーン面切替）は表示系の消費対象＝文字状態機械へは配送しない（R3.2）。
        // 適用しても文字状態（items／reveal／visible_glyphs）は完全に不変。
        state.apply_cue(
            &cue("0", 0.3, CueCommand::BalloonSurface { key: "2".into() }),
            &config,
        );
        assert_eq!(state, before);
    }

    // ── TextLayerConfig 既定値（design.md 正準: char_wait=0.05 / line_pitch 係数=1.25） ──

    #[test]
    fn config_defaults_match_design_canon() {
        let config = TextLayerConfig::default();
        assert_eq!(config.char_wait, 0.05);
        assert_eq!(config.punctuation_wait, 0.3);
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

    /// 改行: LineBreak も item 単位で reveal 枠を持つ（#4）——schedule は items と index 整合し、
    /// 改行の reveal は `max(prev_reveal, cue.at)`（`min_wait=0`）。可視 item には数えるが
    /// 可視グリフには数えない。
    #[test]
    fn line_break_takes_a_reveal_slot() {
        let mut state = TextLayerState::default();
        let config = exact_config();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())), &config);
        state.apply_cue(&cue("0", 0.25, CueCommand::NewLine { ratio: 1.0 }), &config);
        state.apply_cue(&cue("0", 0.25, CueCommand::Text("cd".into())), &config);

        // items 5 件（グリフ 4＋改行 1）・reveal も 5 件（item と index 整合）。
        assert_eq!(items_of(&state, "0").len(), 5);
        // a=0.0, b=0.25, 改行=max(0.25,0.25)=0.25, c=max(0.25+0.25,0.25)=0.5, d=0.75。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.25, 0.5, 0.75]);
        // 改行は item prefix には数えるがグリフには数えない。
        let actor = ActorKey::from("0");
        assert_eq!(state.visible_items(&actor, 0.25), 3, "a, b, 改行");
        assert_eq!(state.visible_glyphs(&actor, 0.25), 2, "a, b（改行はグリフでない）");
    }

    /// #3 精密檻: 句読点直後のグリフだけが `punctuation_wait` を負い、非句読点直後は素の char_wait。
    /// config で可変（開発者が実走で手触りを調整する）。
    #[test]
    fn punctuation_wait_is_configurable_and_delays_following_glyph() {
        let mut state = TextLayerState::default();
        // 2 の冪で正確表現できる値（char_wait 0.25・punctuation_wait 1.0）。
        let config = TextLayerConfig {
            char_wait: 0.25,
            punctuation_wait: 1.0,
            line_pitch_factor: 1.25,
        };
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ、い。う".into())), &config);
        // あ=0.0／、=0.0+0.25=0.25（あは非句読点）／い=0.25+0.25+1.0=1.5（、直後）／
        // 。=1.5+0.25=1.75（いは非句読点）／う=1.75+0.25+1.0=3.0（。直後）。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 1.5, 1.75, 3.0]);
    }

    /// 設計判断（#4）: 改行の reveal は `max(prev_reveal, cue.at)` のみで、直前が句読点でも
    /// `punctuation_wait` を上乗せしない（改行は自身の `at` が唯一の下限）。
    #[test]
    fn line_break_after_punctuation_uses_only_cue_at() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig {
            char_wait: 0.25,
            punctuation_wait: 1.0,
            line_pitch_factor: 1.25,
        };
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ。".into())), &config);
        state.apply_cue(&cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }), &config);
        // あ=0.0／。=0.25／改行=max(0.25, cue.at=0.0)=0.25（句読点 wait なし）。
        assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.25]);
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

    // ══ 実走欠陥 #3（句読点ウェイト）／#4（改行ウェイト）: item 単位リビール ══
    //
    // 真因: 従来 schedule はグリフのみに時刻を割り、改行マーカーは reveal 枠を持たず
    // （NewLine は cue.at を読まない）レイアウトで即時反映され、`\n` 直前の `\w`（sakura が
    // NewLine の at へ畳み込む）が無視されて早発した（#4）。同時に per-glyph の wait が
    // 一様で句読点で追加ポーズが入らなかった（#3）。修正は「item（グリフ＋改行）ごとに
    // reveal 時刻を割る」item 単位モデル。

    /// #4: 改行マーカーは自身の reveal 枠を `r_lb = max(prev_reveal, cue.at)` で持つ。
    /// sakura が先行 `\w` を NewLine の `at` へ畳み込むため、改行は `\w` の分だけ遅れる。
    #[test]
    fn line_break_reveal_time_honors_preceding_wait() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default(); // char_wait = 0.05
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あい".into())), &config);
        // NewLine の at=0.5 は「先行 \w（500ms）を畳み込んだ post-wait offset」の転写。
        state.apply_cue(&cue("0", 0.5, CueCommand::NewLine { ratio: 1.0 }), &config);

        let times = reveal_times_of(&state, "0");
        // item ごとに 1 reveal 時刻: グリフ 2＋改行 1 = 3（従来はグリフ 2 のみ）。
        assert_eq!(times.len(), 3, "改行マーカーも reveal 枠を占める（got {times:?}）");
        // グリフは 0.0／0.05・改行は自身の at=0.5（畳み込まれた \w）まで待つ。
        assert_eq!(times[2], 0.5, "改行 reveal = max(prev_reveal, cue.at) が \\w を尊重する");
    }

    /// #3: 句読点（、）直後のグリフは通常の char_wait より長いポーズで遅れる
    /// （SSP の 、。 自動ポーズ相当・emo2 は明示 `\w` を打たない）。
    #[test]
    fn punctuation_glyph_adds_extra_reveal_pause() {
        let mut state = TextLayerState::default();
        let config = TextLayerConfig::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ、い".into())), &config);

        let times = reveal_times_of(&state, "0");
        assert_eq!(times.len(), 3);
        let normal_gap = times[1] - times[0]; // あ→、: 素の char_wait 間隔
        let after_punct_gap = times[2] - times[1]; // 、→い: 句読点ポーズを含む間隔
        assert!(
            after_punct_gap > normal_gap + 1e-9,
            "、直後のグリフは素の間隔より長く待つ（normal={normal_gap}, after={after_punct_gap}）"
        );
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
