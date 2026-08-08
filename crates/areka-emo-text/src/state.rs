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
//!   **役割分担（newline-defer）**: 本層は `NewLine` を `items` へ**即時**追記する
//!   （追記正本の忠実記録＝後出し優先）に留まり、その改行を実際に**行送りとして
//!   可視化するか否か**（保留・累算・実体化・蒸発）は下流 layout の**遅延解釈**に委ねる。
//!   すなわち「追記は即時・行送りの可視化は layout が遅延解釈」（R4.3）。
//! - `Choice` は配送列に第一級で現れ（R8.6 仕様変更）、本層で実消費する（W4 choice-render）:
//!   `text` のグリフを items へ Text cue と同一の追記＋リビール時刻式で載せつつ、`ChoiceSpan`
//!   （配送順序数 `ordinal`・`\q` ID／表示 label／references の不透明転写・グリフ序数範囲）を
//!   `choices` へ記録する（R1.1/R1.2）。空 `text` は `warn!`（actor 付き）＋空範囲スパン記録・
//!   グリフ追記なし（R1.5 縮退）。`choices` は items と同一ライフサイクル（`Clear`/`ClearAll`
//!   で同時初期化・R5.1/R5.3/R9.5）。
//! - `Cursor`（`\_l` の不透明転写）も本層で実消費する: `parse_cursor_coord` で各軸を
//!   `CursorCoord` 語彙へ忠実転写し、`TextItem::CursorMove` を items へ追記する（改行マーカーと
//!   同格の非グリフアイテム＝reveal 対象外・グリフ／リビール状態は不変・R2.1）。単位換算・
//!   座標解決・原点解釈は下流 layout の責務。
//! - **typewriter リビール（R3／R7 系）**: `Text` 追記時に per-glyph リビール時刻を
//!   `r_i = max(r_{i-1} + interval, at(chunk(i)))`（先頭は `r_0 = at`）で確定する。
//!   `interval` は**配送された cue の再生時間**から `interval = duration / glyph_count`
//!   で導出する（自前の文字送り定数は持たない・服従＝再生時間の真実源に従う・R7.1/R7.2）。
//!   可視数は `visible_glyphs(actor, t)`＝注入時刻 `t` で `r_i <= t` のグリフ数。
//!   実時間 sleep／`Instant` 不使用（注入時刻駆動・R3.3/R9.1）。
//! - グリフ単位は Rust の `char`（M1 正準。書記素クラスタ結合は M2 検討事項——
//!   emo2 fixture は結合文字を使用しない）。
//! - 同一 cue 列＋同一入力条件→同一状態（決定論・R2.4/R2.5）。

use std::collections::BTreeMap;

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
    /// カーソル位置指定（`CueCommand::Cursor { x, y }`＝`\_l` の不透明転写）。
    ///
    /// 改行マーカーと同格の**非グリフ**アイテム（reveal 対象外・グリフ序数を消費しない）。
    /// 各軸は [`parse_cursor_coord`] で忠実転写した [`CursorCoord`] 語彙で、単位換算・
    /// 座標解決・原点解釈は下流 layout（`cursor_to_image_px`／pending-cursor 遅延実体化）の責務。
    CursorMove {
        /// x 軸の座標語彙。
        x: CursorCoord,
        /// y 軸の座標語彙。
        y: CursorCoord,
    },
}

/// `\_l` 座標 1 軸の語彙（不透明文字列の全語彙を保持・`Copy`・state.rs 所有）。
///
/// `Cursor` cue（`\_l[x,y]` の不透明転写）の各軸を、後段の換算層
/// （`layout.rs::cursor_to_image_px`）が縮退表どおりに分岐できる語彙へ忠実転写する
/// （design.md「純粋層 / StateIncrement」正本・R2.1/2.4/6.5）。本層は**語彙の保持**のみを
/// 責務とし、非負ゲート・単位換算・原点解釈は下流 layout の責務（面引数不透明転写規約）。
///
/// M1 の縮退（design.md 縮退表）:
/// - `Absolute { Px/Em/Lh, 非負 }` のみ layout が Some(image px) を返す実導出対象。
/// - `Absolute { Percent, .. }`・`Relative`（`@`）・負値・`Invalid`・`Omitted` は
///   layout が None（＝当該軸スキップ・warn-once 縮退）を返す。負の裸数値は本層では
///   Absolute へ忠実転写し（Invalid へ写像しない）、非負ゲートは layout 層に委ねる。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorCoord {
    /// 省略（空文字列）＝当該軸不動（正典・R2.4）。
    Omitted,
    /// 絶対座標（M1 実導出は Px/Em/Lh の非負値・Percent は縮退保持・負値は忠実転写）。
    Absolute { value: f32, unit: CursorUnit },
    /// 相対座標（`@` 接頭）。語彙保持のみ——M1 は layout が warn-once 縮退（None）。
    Relative { value: f32, unit: CursorUnit },
    /// パース不能（warn 縮退＝当該軸スキップ・状態不変・R6.5）。
    Invalid,
}

/// `\_l` 座標の単位（design.md「純粋層 / StateIncrement」正本）。
///
/// `Px`＝image px（裸数値）・`Em`＝`em`（font 高基準）・`Lh`＝`lh`（行送り基準）・
/// `Percent`＝`%`（縮退保持・M1 は layout が None）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorUnit {
    /// image px（裸数値・M1 実導出）。
    Px,
    /// `em`（font 高基準・M1 実導出）。
    Em,
    /// `lh`（行送り基準・M1 実導出）。
    Lh,
    /// `%`（縮退保持・M1 は layout が None）。
    Percent,
}

/// 不透明転写文字列 → `CursorCoord` 語彙（純粋・全入力で値を返す全域関数・state.rs 所有）。
///
/// パニックせず `Result` も返さない（不透明文字列の全入力に対し値を返す・R2.4 決定論）。
/// 文法:
/// - 空文字列 `""` → [`CursorCoord::Omitted`]（当該軸省略）。
/// - `@` 接頭 → [`CursorCoord::Relative`]（残りを本体としてパース）。無ければ
///   [`CursorCoord::Absolute`]。
/// - 本体末尾のサフィックス `%`／`em`／`lh` で単位を決め（無ければ [`CursorUnit::Px`]）、
///   残りを `f32` としてパースする。
/// - 数値本体が有限 `f32` へパースできれば当該 variant、できない／非有限なら
///   [`CursorCoord::Invalid`]。
///
/// 負値は Absolute/Relative へ**忠実転写**する（非負ゲートは下流 `cursor_to_image_px`）。
pub fn parse_cursor_coord(raw: &str) -> CursorCoord {
    // 空文字列＝当該軸省略（正典 R2.4）。
    if raw.is_empty() {
        return CursorCoord::Omitted;
    }

    // `@` 接頭は相対（残りを本体としてパースし、成功時に Relative へ包む）。
    let (is_relative, body) = match raw.strip_prefix('@') {
        Some(rest) => (true, rest),
        None => (false, raw),
    };

    // 末尾サフィックスで単位を決める（`%`／`em`／`lh`・無ければ裸数値＝Px）。
    // `%` を先に剥がすことで "5%" 等を確実に Percent へ分類する。
    let (num_str, unit) = if let Some(n) = body.strip_suffix('%') {
        (n, CursorUnit::Percent)
    } else if let Some(n) = body.strip_suffix("em") {
        (n, CursorUnit::Em)
    } else if let Some(n) = body.strip_suffix("lh") {
        (n, CursorUnit::Lh)
    } else {
        (body, CursorUnit::Px)
    };

    match num_str.parse::<f32>() {
        // 非有限（NaN／inf）は換算を汚さぬよう Invalid へ縮退（防御・R6.5）。
        Ok(value) if value.is_finite() => {
            if is_relative {
                CursorCoord::Relative { value, unit }
            } else {
                CursorCoord::Absolute { value, unit }
            }
        }
        // 数値本体が空／非数値／未知サフィックス残り→パース不能（R6.5 状態不変スキップの源）。
        _ => CursorCoord::Invalid,
    }
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

/// 選択肢スパン（`ActorTextState` の一部・state.rs 所有・design.md「StateIncrement」正本）。
///
/// `Choice` cue 消費時に、追記したグリフ範囲＋不透明転写した `\q` 属性を 1 スパンとして記録する
/// （R1.1/R1.2）。`ordinal` は配送順序数（hover／照会の主キー）で `choices.len()` を焼き込む。
/// `glyph_range` は items のグリフ序数空間（`Glyph` のみを数える序数・改行／カーソル等の非グリフ
/// アイテムを含めない・`visible_glyphs`／reveal と同一序数空間）で、空 `text` は空範囲（`start..start`）。
///
/// 不変条件（design.md「Data Models §不変条件」1）: `glyph_range` は items のグリフ序数空間で
/// **互いに素**かつ**追記順に単調**（各 Choice cue は自身が追記したグリフのみを範囲化するため
/// 構造的に成立）。
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceSpan {
    /// 配送順序数（hover／照会の主キー・記録時の `choices.len()`）。
    pub ordinal: usize,
    /// `\q` ID（不透明転写）。
    pub id: String,
    /// 表示文字列（不透明転写・`text` の複製）。
    pub label: String,
    /// `\q` 第 3 引数以降（参照列・不透明転写）。
    pub references: Vec<String>,
    /// グリフ序数範囲（items のグリフ序数空間・空 `text` は空範囲）。
    pub glyph_range: core::ops::Range<usize>,
}

/// actor 1 人分の表示テキスト状態（追記順の正本＋リビール時刻列＋選択肢スパン）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActorTextState {
    /// 追記順の正本（グリフ／改行マーカー／カーソル指定）。
    items: Vec<TextItem>,
    /// per-glyph リビール時刻列。
    reveal: RevealSchedule,
    /// 選択肢スパン（items と同一ライフサイクル＝`Clear`/`ClearAll` で同時初期化・R5.1/R5.3）。
    choices: Vec<ChoiceSpan>,
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

    /// 選択肢スパン列（配送順・R1.2／R1.3 照会の源）。
    pub fn choices(&self) -> &[ChoiceSpan] {
        &self.choices
    }

    /// テキスト状態が初期状態（空）か。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.reveal.is_empty() && self.choices.is_empty()
    }
}

/// actor 別テキスト状態の集約（cue→行/グリフ状態の純粋遷移・R1.6/R2.1–2.5/R10.5）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayerState {
    /// `ActorKey → ActorTextState`（決定論的順序のため BTreeMap・design.md 正本）。
    actors: BTreeMap<ActorKey, ActorTextState>,
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
            CueCommand::Choice {
                id,
                text,
                references,
            } => {
                // Choice cue の実消費（R1.1/R1.2）: `text` のグリフを Text cue と同一の追記＋
                // リビール時刻式で載せ、追記したグリフ序数範囲＋不透明転写した `\q` 属性を
                // 1 スパンとして `choices` へ記録する。空 `text` は warn!（actor 付き）＋空範囲
                // スパン記録・グリフ追記なし（R1.5 縮退・design.md 縮退表 Choice text 空 row）。
                let glyph_count = text.chars().count();
                let interval = if glyph_count > 0 {
                    cue.duration / glyph_count as f64
                } else {
                    0.0
                };
                let state = self.actors.entry(cue.actor.clone()).or_default();
                // 序数空間は items のグリフ（`Glyph` のみ）＝`visible_glyphs`／reveal と同一。
                let start = state
                    .items
                    .iter()
                    .filter(|it| matches!(it, TextItem::Glyph { .. }))
                    .count();
                if glyph_count == 0 {
                    // R1.5 縮退: 空範囲スパンのみ記録（グリフ追記・リビール拡張なし・行を生まない）。
                    tracing::warn!(actor = %cue.actor, id = %id, "Choice cue の text が空——空範囲スパンを記録（グリフ追記なし・R1.5 縮退）");
                } else {
                    tracing::debug!(actor = %cue.actor, id = %id, len = glyph_count, at = cue.at, duration = cue.duration, interval, "Choice cue 適用（グリフ追記＋配送 duration 由来のリビール時刻確定＋スパン記録）");
                    state
                        .items
                        .extend(text.chars().map(|ch| TextItem::Glyph { ch }));
                    state.reveal.extend_chunk(glyph_count, cue.at, interval);
                }
                let ordinal = state.choices.len();
                state.choices.push(ChoiceSpan {
                    ordinal,
                    id: id.clone(),
                    label: text.clone(),
                    references: references.clone(),
                    glyph_range: start..(start + glyph_count),
                });
            }
            CueCommand::Cursor { x, y } => {
                // カーソル位置指定（`\_l` の不透明転写）の実消費（R2.1）: 各軸を parse_cursor_coord で
                // `CursorCoord` 語彙へ忠実転写し、非グリフアイテム `CursorMove` を items へ追記する。
                // グリフ／リビール状態は不変（reveal 対象外）。単位換算・座標解決・原点解釈は下流 layout。
                let x = parse_cursor_coord(x);
                let y = parse_cursor_coord(y);
                tracing::debug!(actor = %cue.actor, ?x, ?y, "Cursor cue 適用（CursorMove 追記・グリフ/リビール不変）");
                let state = self.actors.entry(cue.actor.clone()).or_default();
                state.items.push(TextItem::CursorMove { x, y });
            }
            // 文字状態機械が消費しない command（cue_target_of が Shell/None に分類）は本状態機械の
            // 対象外——演者側 relevance の責務。防御的に無視する（catch-all を置かず、dola の
            // variant 追加時にコンパイラが再検討を強制する）。`BalloonSurface` は表示系
            // （seriko＝`cue_target_of` が Shell 分類）の消費対象＝文字状態機械へは動作させない防御面（R3.2）。
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
#[path = "state_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "state_cue_apply_tests.rs"]
mod cue_apply_tests;

#[cfg(test)]
#[path = "state_reveal_tests.rs"]
mod reveal_tests;

#[cfg(test)]
#[path = "state_cursor_coord_parse_tests.rs"]
mod cursor_coord_parse_tests;
