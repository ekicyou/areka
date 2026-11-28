//! # state — cue 駆動の純粋状態機械（純粋層）
//!
//! cue 列（Text／NewLine／Clear）を actor 別の行/グリフ状態へ純粋に遷移させる
//! `TextLayerState`／`ActorTextState`／`RevealSchedule`（注入時刻駆動 typewriter）を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 時刻は常に注入（フレームの時刻 `frame_now`）で受け取り、内部で実時間を読まない。
//!
//! ## 遷移規則
//!
//! - cue の `actor`（`ActorKey`）を鍵に、actor 別の独立した [`ActorTextState`] へ振り分ける。
//!   未知 actor の cue は状態を lazily 生成して蓄積する（無損失・描画は binding 解決後）。
//! - **後出し優先の即時適用**: `Text`＝追記・`NewLine`＝改行マーカー追記・`Clear`＝未リビール分を含む全消去。
//!   トーク上書きのガードは持たない（中断可否は上流の中断ファンネルで決着済み）。
//!   `NewLine` は `items` へ即時追記するだけで、改行を行送りとして可視化するか（保留・累算・実体化・蒸発）は
//!   下流 layout の遅延解釈に委ねる。
//! - `Choice` は本層で実消費する: `text` のグリフを Text cue と同一の追記＋リビール時刻式で `items` へ載せ、
//!   `ChoiceSpan`（配送順序数 `ordinal`・`\q` ID／表示 label／references の不透明転写・グリフ序数範囲）を
//!   `choices` へ記録する。空 `text` は `warn!`（actor 付き）＋空範囲スパン記録・グリフ追記なし。
//!   `choices` は items と同一ライフサイクル（`Clear`/`ClearAll` で同時初期化）。
//! - `AnchorBegin`／`AnchorEnd`（`\_a`）も本層で実消費する: 開きから閉じまでに開いた場所へ追記された文字を、
//!   選択肢と同じ列 `choices` の 1 件（種類＝アンカー）として記録する。中身は子の `anchor`（`state_anchor.rs`）が持つ。
//! - `Cursor`（`\_l` の不透明転写）も本層で実消費する: `parse_cursor_coord` で各軸を `CursorCoord` 語彙へ
//!   忠実転写し、`TextItem::CursorMove` を items へ追記する（非グリフアイテム＝reveal 対象外）。
//!   単位の係数・基点・原点の解釈・軸取り違えの判定は解決層 [`crate::cursor_tag`] の責務で、
//!   その解決を軸ごとに呼ぶ配線が `layout` である。
//! - **typewriter リビール**: `Text` 追記時に per-glyph リビール時刻を、seriko と同じ 1 本の式
//!   [`areka_sakura::reveal::chunk_reveal_offsets`]（配送された cue の再生時間から導く）で確定する。
//!   字の現れる時刻は「合図の発火時刻＋塊の頭からの隔たり」の絶対の時刻。自前の文字送り定数は持たない。
//!   可視数は `visible_glyphs(actor, frame_now)`＝`r_i <= frame_now` のグリフ数で、見せた字の数は別に覚えない。
//! - グリフ単位は書記素クラスタ（UAX #29）。切り方は [`areka_sakura::cluster::clusters`] だけを通す
//!   （再生時間の文字数と同じ切り方・ZWJ 列・国旗・肌色・異体字セレクタ・キーキャップ・結合文字が 1 アイテム）。
//! - 同一 cue 列＋同一入力条件→同一状態（決定論）。

use std::collections::BTreeMap;
use std::sync::Arc;

use areka_sakura::cluster::clusters;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue, TimeDelta, TimePoint, TimeSpan};
use areka_sakura::reveal::chunk_reveal_offsets;
use dola::cue::OriginNotice;

use crate::inline_marker::{MARKER_GLYPH, is_marker_carrier};
use crate::look::{StyleId, StyleTable};
use crate::place::{PlaceKey, TextPlace};

use decoration::font_tag_tokens;
pub use decoration::{BoxTraits, Decoration};
pub use route::{SurfaceFallback, SurfaceKeyOutcome};

/// テキスト層の調整値。
///
/// 純粋層・結線層の双方が消費する共有設定であり、**行送りの式の唯一の定義点**
/// （`line_pitch = font.height + 行間`）。純粋層の
/// [`crate::layout::FixedMetrics`] も COM 層の `DWriteMetrics` も自前で足し算をせず
/// [`TextLayerConfig::line_pitch`] を呼ぶ。
///
/// reveal のペースは自前の定数でなく配送された cue の再生時間（`TalkCue::duration`）から
/// 導出する。本 config は行間 `line_gap` のみを保持する。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLayerConfig {
    /// 行間（image px・0 以上の有限の値で小数も通る）。`1lh = 1em + 行間` の「行間」にあたる量で、既定は 2
    /// （裁定 2026-09-05・根拠は `doc/COMPAT_ARCHITECTURE.md`）。
    pub line_gap: f32,
}

impl Default for TextLayerConfig {
    fn default() -> Self {
        Self { line_gap: 2.0 }
    }
}

impl TextLayerConfig {
    /// 行送りピッチ＝`font_height + line_gap`（切り上げなし・両項とも 0 以上の有限の px で小数も通る）。
    ///
    /// 行送り・`\_l` の `lh` 係数・比率つき改行の送り量・選択肢の帯の上限は、いずれも
    /// この 1 本の式を源にする（別々の係数を持たない）。書字方向が縦書きのときは
    /// 同じ値を列送りとして読み替えるだけで、縦書き専用の係数は持たない。
    pub fn line_pitch(&self, font_height: f32) -> f32 {
        font_height + self.line_gap
    }

    /// 非有限・負の `line_gap` を警告つきで 0 へ縮退した調整値を返す（log-first）。
    ///
    /// 呼び手は構築直後に 1 度だけ通す規約（[`crate::actor::TextLayerRuntime::new`]）——
    /// 以降は正規化済みの値を配るため、縮退の警告は構築時の 1 件だけになる。
    /// `line_gap` が有限かつ 0 以上なら値も警告も変えずそのまま返す。
    pub fn normalized(self) -> TextLayerConfig {
        if self.line_gap.is_finite() && self.line_gap >= 0.0 {
            return self;
        }
        tracing::warn!(
            line_gap = self.line_gap,
            fallback = 0,
            "line_gap が非有限または負——行間を 0 へ縮退する"
        );
        TextLayerConfig { line_gap: 0.0 }
    }
}

/// 追記順の正本を構成する 1 要素（グリフ／改行マーカー）。
#[derive(Clone, Debug, PartialEq)]
pub enum TextItem {
    /// 1 グリフ＝書記素クラスタ 1 つ（UAX #29 の拡張書記素クラスタ・非空）。
    ///
    /// 構築は [`TextItem::glyph`] を通す。中身を共有の不変文字列（`Arc<str>`）にするのは、
    /// 毎フレームの配置（[`crate::layout::PositionedGlyph`] への写し）で割り当てを起こさず、
    /// 状態をスレッドの間で受け渡せる（`Send`）ようにするため。
    Glyph {
        /// グリフの文字列（クラスタ 1 つ）。
        text: Arc<str>,
    },
    /// 改行マーカー（`CueCommand::NewLine { ratio }` の転写・`\n`=1.0／`\n[half]`=0.5）。
    LineBreak {
        /// 行送り量の比率（`行送り量 = line_pitch × ratio`）。
        ratio: f32,
    },
    /// カーソル位置指定（`CueCommand::Cursor { x, y }`＝`\_l` の不透明転写）。
    ///
    /// 改行マーカーと同格の**非グリフ**アイテム（reveal 対象外・グリフ序数を消費しない）。
    /// 各軸は [`parse_cursor_coord`] で忠実転写した [`CursorCoord`] 語彙で、単位換算・基点の
    /// 解釈・軸の判定は下流の解決層（`cursor_tag.rs`）、実体化は配線層（pending-cursor 遅延
    /// 実体化）の責務。
    CursorMove {
        /// x 軸の座標語彙。
        x: CursorCoord,
        /// y 軸の座標語彙。
        y: CursorCoord,
    },
}

impl TextItem {
    /// クラスタ 1 つからグリフのアイテムを作る（本番の cue の適用もテストの直書きもこれを通す）。
    ///
    /// 前提「非空・ちょうど 1 クラスタ」はデバッグビルドの検査で確かめる（テストの直書きで
    /// 2 クラスタを 1 アイテムに詰める誤りを捕まえる）。本番は [`areka_sakura::cluster::clusters`] の
    /// 各要素を渡すので前提が崩れない。
    pub fn glyph(text: &str) -> TextItem {
        debug_assert!(
            areka_sakura::cluster::cluster_count(text) == 1,
            "TextItem::glyph はクラスタちょうど 1 つを受ける: {text:?}"
        );
        TextItem::Glyph {
            text: Arc::from(text),
        }
    }
}

/// `\_l` 座標 1 軸の語彙（不透明文字列の全語彙を保持・`Copy`・state.rs 所有）。
///
/// `Cursor` cue（`\_l[x,y]` の不透明転写）の各軸を、後段の解決層（`cursor_tag.rs`）が
/// 解決表どおりに分岐できる語彙へ忠実転写する。本層は**語彙の保持**のみを責務とし、
/// 単位換算・基点の解釈・軸の判定は下流 `cursor_tag` の責務。
///
/// 本層は軸の情報を持たない: `centerx` が Y 軸に書かれた（軸取り違えの）事実は解決層が
/// 判定し、語彙としては X 軸の `centerx` と同じ [`CursorCoord::CenterX`] を返す。
///
/// 縮退は 2 分岐だけである: 解釈不能（[`CursorCoord::Invalid`]）と、
/// 中央指定の軸取り違え。負値絶対・`%`・`@` 相対はいずれも解決層が「基点＋値×係数」の
/// 式 1 本で**実導出**する形であって、縮退ではない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorCoord {
    /// 省略（空文字列）＝当該軸不動（正典）。
    Omitted,
    /// 絶対座標（負値・小数を含む忠実転写）。基点は解決後の文字描画開始点。
    Absolute { value: f32, unit: CursorUnit },
    /// 相対座標（`@` 接頭・負値・小数を含む）。基点は現在の文字描画位置。
    Relative { value: f32, unit: CursorUnit },
    /// `centerx`＝バルーン画像の幅の半分。基準は文字描画開始点でも文字描画範囲でも
    /// なくバルーン画像そのもの。X 軸に書かれたときだけ解決層が実導出する。
    CenterX,
    /// `centery`＝バルーン画像の高さの半分。基準はバルーン画像そのもの。
    /// Y 軸に書かれたときだけ解決層が実導出する。
    CenterY,
    /// パース不能・非有限（解決層が当該軸不動へ縮退）。
    Invalid,
}

/// `\_l` 座標の単位。
///
/// 解決層が用いる係数は `Px`＝1（image px 恒等）・`Em`＝文字高さ・`Lh`＝行送り・
/// `Percent`＝文字高さ / 100。単位は**軸に依らないスカラー**であり、書字方向や適用軸に
/// 応じた再写像・拒否は行わない。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorUnit {
    /// image px（裸数値・係数 1）。
    Px,
    /// `em`（係数＝タグを書いた時点での文字高さ）。
    Em,
    /// `lh`（係数＝行送り＝1em＋行間）。
    Lh,
    /// `%`（係数＝文字高さ / 100。100%＝文字高さ 1 個ぶん）。
    Percent,
}

/// 不透明転写文字列 → `CursorCoord` 語彙（純粋・全入力で値を返す全域関数・state.rs 所有）。
///
/// パニックせず `Result` も返さない（不透明文字列の全入力に対し値を返す・決定論）。
/// 受理形式は `"" | centerx | centery | @?数値(em|lh|%)?` の 1 形式。文法:
/// - 空文字列 `""` → [`CursorCoord::Omitted`]（当該軸省略）。
/// - 生文字列が `centerx`／`centery` に**小文字で完全一致**するとき →
///   [`CursorCoord::CenterX`]／[`CursorCoord::CenterY`]。判定は `@` 剥離の**前**に置くので
///   `@centerx` は該当せず、`@` を剥がした本体が数値でないため [`CursorCoord::Invalid`] に
///   落ちる。大小文字の扱いは正典沈黙ゆえ `doc/COMPAT_ARCHITECTURE.md` の
///   「小文字の完全一致のみ」の先例に揃える（`CENTERX` は [`CursorCoord::Invalid`]）。
/// - `@` 接頭 → [`CursorCoord::Relative`]（残りを本体としてパース）。無ければ
///   [`CursorCoord::Absolute`]。
/// - 本体末尾のサフィックス `%`／`em`／`lh` で単位を決め（無ければ [`CursorUnit::Px`]）、
///   残りを `f32` としてパースする。
/// - 数値本体が有限 `f32` へパースできれば当該 variant、できない／非有限なら
///   [`CursorCoord::Invalid`]。
///
/// 負値は Absolute/Relative へ**忠実転写**する（負値は解決層が原点から負方向へ実導出する
/// 形であって、語彙層は Invalid へ写像しない）。
pub fn parse_cursor_coord(raw: &str) -> CursorCoord {
    if raw.is_empty() {
        return CursorCoord::Omitted;
    }

    // `@` 剥離の前に判定する: 後だと `@centerx` が CenterX になり、`Invalid` の事後条件を破る。
    match raw {
        "centerx" => return CursorCoord::CenterX,
        "centery" => return CursorCoord::CenterY,
        _ => {}
    }

    let (is_relative, body) = match raw.strip_prefix('@') {
        Some(rest) => (true, rest),
        None => (false, raw),
    };

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
        Ok(value) if value.is_finite() => {
            if is_relative {
                CursorCoord::Relative { value, unit }
            } else {
                CursorCoord::Absolute { value, unit }
            }
        }
        _ => CursorCoord::Invalid,
    }
}

/// per-glyph リビール時刻列（注入時刻駆動 typewriter）。
///
/// 時刻式は [`chunk_reveal_offsets`]（`max(前の字 + duration ÷ count, 塊の頭)` を字ごとに 1 回だけ丸める）。
/// 可視数は `visible(frame_now) = |{ i : r_i <= frame_now }|`。発火時刻は下限（それより早く可視化しない）で、
/// リビールは配送 duration が定めるペースでバッファ末尾を追う（長文時は遅延しうる・無損失）。
/// 時刻は常に注入（絶対の時刻）で、実時間には依存しない。`Clear` は schedule ごと初期化する。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RevealSchedule {
    /// グリフ i のリビール時刻 `r_i`（絶対の時刻・単調非減少）。
    times: Vec<TimePoint>,
}

impl RevealSchedule {
    /// リビール時刻列（絶対の時刻）。
    pub fn times(&self) -> &[TimePoint] {
        &self.times
    }

    /// リビール時刻が 1 件も無いか。
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// フレームの時刻での可視グリフ数 `visible(frame_now) = |{ i : r_i <= frame_now }|`。
    ///
    /// `times` は単調非減少（`extend_chunk` が前の字以降に置く）なので二分探索で数える。
    pub fn visible(&self, frame_now: TimePoint) -> usize {
        self.times.partition_point(|&r| r <= frame_now)
    }

    /// 効く時刻より後の時刻を `delta` だけ動かす。効く時刻より前になる時刻は効く時刻にする（順は保たれる）。
    fn shift_after(&mut self, effective_at: TimePoint, delta: TimeDelta) {
        for t in self.times.iter_mut().filter(|t| **t > effective_at) {
            *t = (*t + delta).max(effective_at);
        }
    }

    /// chunk（Text cue 1 件分のグリフ列）のリビール時刻を時刻式で追記する。
    ///
    /// 塊の頭は合図の発火時刻。前の字は schedule の最後の字（空なら塊の頭から）で、塊の頭からの差で渡す。
    /// 改行マーカーはグリフでないため対象外。duration=0 なら全グリフが `chunk_start` で同時可視で、
    /// `times` は単調非減少に保たれる（`visible` の二分探索前提）。
    fn extend_chunk(&mut self, glyph_count: usize, chunk_start: TimePoint, duration: TimeSpan) {
        let prev = self.times.last().map(|&last| last - chunk_start);
        self.times.reserve(glyph_count);
        self.times.extend(
            chunk_reveal_offsets(prev, duration, glyph_count).map(|offset| chunk_start + offset),
        );
    }
}

/// 押せる範囲の種類（[`ChoiceSpan::kind`]）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    /// 選択肢（`\q`）。
    Choice,
    /// アンカー（`\_a`）。
    Anchor,
}

/// 押せる範囲の記録——選択肢とアンカーの共通の範囲（`ActorTextState` の一部）。
///
/// `Choice` cue 消費時に、追記したグリフ範囲＋不透明転写した `\q` 属性を 1 スパンとして記録する。
/// `ordinal` は配送順序数（hover／照会の主キー）で、記録時の `choices.len`。
/// `glyph_range` は items のグリフ序数空間（`Glyph` のみを数える・`visible_glyphs`／reveal と同一）で、
/// 空 `text` は空範囲（`start..start`）。
///
/// 不変条件: `ordinal` は列の添字に等しく追記順に単調。**同じ種類の**範囲の `glyph_range` は
/// 互いに素（各 Choice cue は自身が追記したグリフのみを範囲化するため構造的に成立）。
/// アンカーの範囲は選択肢の範囲を包んでよい。
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceSpan {
    /// 範囲の種類（選択肢／アンカー）。
    pub kind: SpanKind,
    /// 配送順序数（hover／照会の主キー・記録時の `choices.len`・選択肢とアンカーで共通の通し番号）。
    pub ordinal: usize,
    /// ID（`\q`／`\_a` の ID・不透明転写）。
    pub id: String,
    /// 表示文字列（不透明転写・選択肢は `text` の複製・アンカーは範囲の文字）。
    pub label: String,
    /// 付随する引数（`\q` は第 3 引数以降・`\_a` は第 2 引数以降・不透明転写）。
    pub references: Vec<String>,
    /// グリフ序数範囲（items のグリフ序数空間・空 `text` は空範囲）。
    pub glyph_range: core::ops::Range<usize>,
}

/// actor 1 人分の表示テキスト状態（追記順の正本＋リビール時刻列＋押せる範囲の列）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActorTextState {
    /// 追記順の正本（グリフ／改行マーカー／カーソル指定）。
    items: Vec<TextItem>,
    /// per-glyph リビール時刻列。
    reveal: RevealSchedule,
    /// 押せる範囲の列——選択肢とアンカーが同じ列に並ぶ（items と同一ライフサイクル＝
    /// `Clear`/`ClearAll` で同時初期化）。
    choices: Vec<ChoiceSpan>,
    /// 開いているアンカーの範囲（`choices` の添字）。開いていなければ `None`。
    /// `choices` と一緒に消える（操作は [`anchor`]）。
    anchor_open: Option<usize>,
    /// 済みの境目: この序数より前の選択肢は済み（`0..=choices.len`・`choices` と同じ寿命で
    /// 消去のとき 0 に戻る・アンカーは境目より前でも済まない）。
    settled: usize,
    /// グリフ序数と同じ序数空間の装飾番号列（操作は [`decoration`]）。
    glyph_styles: Vec<StyleId>,
    /// 装飾の表（番号 → 既定と異なる見た目・内容と同じライフサイクル）。
    styles: StyleTable,
    /// スコープの装飾状態（2 層・現在の見た目・所有外キーの保持・記録済みの値）。
    decor: Decoration,
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

    /// 押せる範囲の列（選択肢とアンカー・配送順・照会の源）。
    pub fn choices(&self) -> &[ChoiceSpan] {
        &self.choices
    }

    /// テキスト状態が初期状態（空）か。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.reveal.is_empty() && self.choices.is_empty()
    }

    /// 今ある範囲を全部済みにする（境目を範囲の数へ進める）。済むのは種類が選択肢の範囲だけ。
    pub(crate) fn settle_choices(&mut self) {
        self.settled = self.choices.len();
    }

    /// 済みの境目（この序数より前の選択肢は済み・配置の覚えの鍵）。
    pub(crate) fn settled(&self) -> usize {
        self.settled
    }

    /// その範囲が済んだ選択肢か（種類が選択肢で、序数が境目より前）。
    pub fn is_settled_choice(&self, span: &ChoiceSpan) -> bool {
        span.kind == SpanKind::Choice && span.ordinal < self.settled
    }

    /// 済んでいない選択肢が 1 つでもあるか。
    pub fn has_live_choice(&self) -> bool {
        self.choices
            .iter()
            .any(|span| span.kind == SpanKind::Choice && !self.is_settled_choice(span))
    }
}

/// actor 別テキスト状態の集約（cue→行/グリフ状態の純粋遷移）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayerState {
    /// `PlaceKey → ActorTextState`（スコープと文字の場所の組ごと・決定論的順序のため BTreeMap）。
    actors: BTreeMap<PlaceKey, ActorTextState>,
    /// scope ごとの内容が消された回数（`Clear`／`ClearAll` の累計・単調増加）。
    /// `ActorTextState` の欄にしないのは、内容の状態の形と等しさを変えないため。
    /// 普通のバルーンの場所の回数は場所を外しても消さない（わざと残す）: 読み手（`crates/areka` の
    /// `balloon_visibility`）が単調増加を前提に `clear_count` の変化を見るので、消すと 0 へ戻って
    /// 変化を取り違える。箱の場所の回数は `set_box_index` が箱の場所と一緒に捨てる。
    clears: BTreeMap<PlaceKey, u64>,
    /// スコープごとの今のサーフェス番号と文字の行き先（操作は [`route`]）。
    /// `set_box_index` でも消さない（わざと残す）: スコープの今のサーフェス番号はシェルの切替を
    /// またいで保つ状態なので、使ったスコープの数だけ残る（1 つにつき小さな構造体 1 つ）。
    routes: BTreeMap<ActorKey, route::ScopeRoute>,
    /// サーフェス番号 → element番号の昇順の箱の名前（空なら行き先は常に普通のバルーン）。
    box_index: BTreeMap<u32, Vec<areka_emo_compose::BoxName>>,
    /// 箱の名前 → 既定の見た目と種類（[`TextLayerState::set_box_traits`]・無ければ既定の層でスコープに従う）。
    box_traits: BTreeMap<areka_emo_compose::BoxName, BoxTraits>,
    /// 「空回し」の印（既定は偽）。立てる口は [`TextLayerState::rehearsal_copy`] だけで、
    /// 本番の状態では決して立たない。立っている間は状態の層の warn 4 か所を出さない
    /// （本番の適用が同じ warn を出すので、空回しが出すと 2 回になる）。
    /// アンカーの開き・閉じの記録だけは `debug!` も出さない（`state_anchor.rs`）。
    rehearsal: bool,
}

impl TextLayerState {
    /// 印を立てた写しを返す（空回し専用）。印の立った状態は warn を出さない。
    ///
    /// 止めるのは warn（とアンカーの開き・閉じの `debug!`）だけで、ほかの `debug!` と
    /// warn 済みの記録の集合の更新は本番と同じ（集合は写しの中だけで進み、本番の集合には触れない）。
    pub(crate) fn rehearsal_copy(&self) -> TextLayerState {
        TextLayerState {
            rehearsal: true,
            ..self.clone()
        }
    }

    /// 場所の選択肢を済みにする。場所の状態が無ければ何もしない（偽を返す・場所も作らない）。
    pub(crate) fn settle_choices_at(&mut self, place: &PlaceKey) -> bool {
        match self.actors.get_mut(place) {
            Some(state) => {
                state.settle_choices();
                true
            }
            None => false,
        }
    }

    /// 全消しの後に呼ぶ。中身が作りたてと同じで、`keep` が偽の普通のバルーンの場所を外す。
    /// 消した回数（`clears`）は外さない（`crates/areka` が `clear_count` の変化を見るため）。
    pub(crate) fn forget_pristine_places(&mut self, keep: impl Fn(&PlaceKey) -> bool) {
        let pristine = ActorTextState::default();
        self.actors.retain(|key, state| {
            key.place != TextPlace::Balloon || *state != pristine || keep(key)
        });
    }

    /// 全部の場所の選択肢を済みにする。
    pub(crate) fn settle_all_choices(&mut self) {
        for state in self.actors.values_mut() {
            state.settle_choices();
        }
    }

    /// cue の純粋適用（DirectWrite 非依存・決定論）。
    ///
    /// 後出し優先の即時適用（`Text`＝追記・`NewLine`＝改行マーカー追記・`Clear`＝未リビール分を含む全消去）。
    /// トーク上書きガードは持たず、届いた cue 列を到着順に適用する。未知 actor は lazily 生成する。
    ///
    /// `Text` は追記と同時に per-glyph リビール時刻を `cue.fire_time`（リビール開始の下限）と `duration` から
    /// [`chunk_reveal_offsets`] で確定する。`glyph_count = 0`（空テキスト）は追記しない。
    /// `duration = 0`（瞬時の cue）は全グリフが `cue.fire_time` で同時可視になる。
    /// リビール中の後続 cue も即時適用される（本層は cue の `fire_time`／`duration` を読むだけで書き換えない）。
    ///
    /// 戻り値は名前の形の `\b[名前]` の箱が無かった（空回しの写しでない）か（[`Self::route_select`]）。
    pub fn apply_cue(&mut self, cue: &TalkCue) -> bool {
        let dest = PlaceKey {
            actor: cue.actor.clone(),
            place: self.destination(&cue.actor),
        };
        let quiet = self.rehearsal;
        let mut box_missing = false;
        match &cue.command {
            CueCommand::Text(text) => {
                let glyph_units: Vec<&str> = clusters(text).collect();
                let glyph_count = glyph_units.len();
                tracing::debug!(actor = %cue.actor, len = glyph_count, fire_time = %cue.fire_time, duration = %cue.duration, "Text cue 適用（追記＋配送 duration 由来のリビール時刻確定）");
                let state = self.place_entry(&dest);
                state
                    .items
                    .extend(glyph_units.iter().map(|c| TextItem::glyph(c)));
                state.extend_open_anchor(&glyph_units);
                state
                    .reveal
                    .extend_chunk(glyph_count, cue.fire_time, cue.duration);
                state.push_current_style(&cue.actor, glyph_count, quiet);
            }
            CueCommand::NewLine { ratio } => {
                tracing::debug!(actor = %cue.actor, ratio, "NewLine cue 適用（改行マーカー追記）");
                let state = self.place_entry(&dest);
                state.items.push(TextItem::LineBreak { ratio: *ratio });
            }
            CueCommand::Clear => {
                tracing::debug!(actor = %cue.actor, "Clear cue 適用（未リビール分含む全消去・装飾は保持）");
                // `\c` は装飾を戻さない: `ActorTextState::default` へ戻すと装飾まで消える。
                self.place_entry(&dest).clear_content();
                *self.clears.entry(dest.clone()).or_default() += 1;
            }
            CueCommand::ClearAll => {
                tracing::debug!(actor = %cue.actor, "ClearAll cue 適用（全スコープを未リビール分含め消去）");
                // 上流は残存スコープを列挙できないため、全 actor スコープの消去は本状態機械が行う。
                // entry は除去しない: 提示層（`present_frame`）は `actors` の走査で再描画対象を
                // 決めるので、外すと既描画のテキストが画面に残留する。
                // 台本の先頭ゆえ装飾も既定へ戻す（装飾は台詞をまたがない）。記録済みの値も空にして、
                // 次の台詞では同じ不正な指定がもう 1 度記録されるようにする。
                // 回数を進めるのは既にある scope だけ（状態の無い scope は 0 のまま）。
                for (key, state) in self.actors.iter_mut() {
                    state.clear_content();
                    state.reset_for_new_talk();
                    *self.clears.entry(key.clone()).or_default() += 1;
                }
                self.reset_routes();
            }
            CueCommand::Choice {
                id,
                text,
                references,
            } => {
                let glyph_units: Vec<&str> = clusters(text).collect();
                let glyph_count = glyph_units.len();
                let state = self.place_entry(&dest);
                // 字を足す所はいつも同じ数で `extend_chunk` を呼ぶので、時刻の列の長さ＝字の数。
                let start = state.reveal.times().len();
                if glyph_count == 0 {
                    // 空回しの写しでは記録しない（本番の適用が記録する）。
                    if !quiet {
                        tracing::warn!(actor = %cue.actor, id = %id, "Choice cue の text が空——空範囲スパンを記録（グリフ追記なし・R1.5 縮退）");
                    }
                } else {
                    tracing::debug!(actor = %cue.actor, id = %id, len = glyph_count, fire_time = %cue.fire_time, duration = %cue.duration, "Choice cue 適用（グリフ追記＋配送 duration 由来のリビール時刻確定＋スパン記録）");
                    state
                        .items
                        .extend(glyph_units.iter().map(|c| TextItem::glyph(c)));
                    state.extend_open_anchor(&glyph_units);
                    state
                        .reveal
                        .extend_chunk(glyph_count, cue.fire_time, cue.duration);
                    state.push_current_style(&cue.actor, glyph_count, quiet);
                }
                let ordinal = state.choices.len();
                state.choices.push(ChoiceSpan {
                    kind: SpanKind::Choice,
                    ordinal,
                    id: id.clone(),
                    label: text.clone(),
                    references: references.clone(),
                    glyph_range: start..(start + glyph_count),
                });
            }
            CueCommand::Cursor { x, y } => {
                let x = parse_cursor_coord(x);
                let y = parse_cursor_coord(y);
                tracing::debug!(actor = %cue.actor, ?x, ?y, "Cursor cue 適用（CursorMove 追記・グリフ/リビール不変）");
                let state = self.place_entry(&dest);
                state.items.push(TextItem::CursorMove { x, y });
            }
            // `\f` 文字装飾の汎用キャリアは名前で自己選別して消費する。名前が合わない運搬
            // （`\!` の他コマンド）と非正準 params は読み飛ばす。
            // 印の合図（`\![*]`・`%*`）は書式の見分けの前に見て、今の場所に目印の字を 1 つ積む。
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5b_2a_5d:1
            CueCommand::Custom { .. } if is_marker_carrier(&cue.command) => {
                tracing::debug!(actor = %cue.actor, fire_time = %cue.fire_time, "印の cue 適用（目印の字を 1 つ追記）");
                let state = self.place_entry(&dest);
                state.items.push(TextItem::glyph(MARKER_GLYPH));
                // アンカーの題（Reference0）には足さない（範囲だけ伸ばす）。
                state.extend_open_anchor_range(1);
                state.reveal.extend_chunk(1, cue.fire_time, cue.duration);
                state.push_current_style(&cue.actor, 1, quiet);
            }
            CueCommand::Custom { .. } => match font_tag_tokens(&cue.command) {
                Some(tokens) => {
                    tracing::debug!(actor = %cue.actor, ?tokens, "\\f cue 適用（以降に追記される文字へ効く）");
                    self.place_entry(&dest)
                        .apply_font_args(&cue.actor, &tokens, quiet);
                }
                None => {
                    tracing::debug!(actor = %cue.actor, command = ?cue.command, "文字状態機械が消費しない cue を無視（上流 routing の対象外流入）");
                }
            },
            // 名前の形の `\b[名前]`（鍵が整数として読めない）は行き先の切り替え。
            // 整数の `\b[ID番号]` は普通のバルーンへの指定（seriko の担当）で、下の腕で読み飛ばす。
            CueCommand::BalloonSurface { key } if areka_emo_compose::TextRoute::is_box_key(key) => {
                box_missing = self.route_select(&cue.actor, key);
            }
            // 閉じは宛先に関わらず開いている範囲を閉じる（[`anchor`]）。
            CueCommand::AnchorBegin { id, references } => self.open_anchor(&dest, id, references),
            CueCommand::AnchorEnd => self.close_anchor(&cue.actor),
            // 文字状態機械が消費しない command は無視する。catch-all を置かないのは、dola の
            // variant 追加時にコンパイラが再検討を強制するため。整数の `BalloonSurface` は
            // seriko の担当。`Wait` の時間は envelope duration が担い上流が後続 cue の時刻へ
            // 焼き込み済みなので、ここで遅延を足してはならない（二重待ち）。
            CueCommand::Emote { .. } | CueCommand::BalloonSurface { .. } | CueCommand::Wait => {
                tracing::debug!(actor = %cue.actor, command = ?cue.command, "文字状態機械が消費しない cue を無視（上流 routing の対象外流入）");
            }
        }
        box_missing
    }

    /// 置き直しの知らせを全部の場所のリビール時刻へ当てる（最初の起点の知らせは何も動かさない）。
    pub(crate) fn apply_origin_notice(&mut self, notice: &OriginNotice) {
        let Some(delta) = notice.delta() else { return };
        for state in self.actors.values_mut() {
            state.reveal.shift_after(notice.effective_at, delta);
        }
    }

    /// フレームの時刻での actor 別可視グリフ数（決定論）。
    ///
    /// `visible(frame_now) = |{ i : r_i <= frame_now }|`（改行マーカーは数えない・グリフのみ）。
    /// 未生成の actor は 0。同一 cue 列＋同一注入時刻列→同一可視数（決定論檻）。
    pub fn visible_glyphs(&self, actor: &ActorKey, frame_now: TimePoint) -> usize {
        self.actor_state(actor)
            .map_or(0, |state| state.reveal.visible(frame_now))
    }

    /// scope の内容が消された回数（`Clear` と `ClearAll` の累計・普通のバルーンの場所）。
    /// 一度も消されていない・状態の無い scope は 0。読むだけで状態を変えない。
    pub fn clear_count(&self, actor: &ActorKey) -> u64 {
        self.clears_at(&PlaceKey::balloon(actor))
    }

    /// その場所（箱の場所も）の内容が消された回数（配置の覚えの鍵）。読むだけで状態を変えない。
    pub(crate) fn clears_at(&self, place: &PlaceKey) -> u64 {
        self.clears.get(place).copied().unwrap_or(0)
    }

    /// actor の普通のバルーンの場所のテキスト状態（未生成の actor は `None`）。
    pub fn actor_state(&self, actor: &ActorKey) -> Option<&ActorTextState> {
        self.place_state(&PlaceKey::balloon(actor))
    }

    /// 場所の鍵で引くテキスト状態（未生成の場所は `None`）。
    pub fn place_state(&self, key: &PlaceKey) -> Option<&ActorTextState> {
        self.actors.get(key)
    }

    /// 全部の場所のテキスト状態（`PlaceKey` 昇順＝スコープごとに普通のバルーン・箱の名前の順）。
    pub fn places(&self) -> impl Iterator<Item = (&PlaceKey, &ActorTextState)> {
        self.actors.iter()
    }

    /// 全 actor の普通のバルーンの場所のテキスト状態（`ActorKey` 昇順・決定論的順序）。
    pub fn actors(&self) -> impl Iterator<Item = (&ActorKey, &ActorTextState)> {
        self.actors
            .iter()
            .filter(|(key, _)| key.place == TextPlace::Balloon)
            .map(|(key, state)| (&key.actor, state))
    }
}

#[path = "state_decoration.rs"]
mod decoration;

#[path = "state_route.rs"]
mod route;

#[path = "state_anchor.rs"]
mod anchor;

#[cfg(test)]
#[path = "state_test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "state_cue_apply_tests.rs"]
mod cue_apply_tests;

#[cfg(test)]
#[path = "state_reveal_tests.rs"]
mod reveal_tests;

#[cfg(test)]
#[path = "state_cluster_tests.rs"]
mod cluster_tests;

#[cfg(test)]
#[path = "state_cursor_coord_parse_tests.rs"]
mod cursor_coord_parse_tests;

#[cfg(test)]
#[path = "state_clear_count_tests.rs"]
mod clear_count_tests;

#[cfg(test)]
#[path = "state_route_tests.rs"]
mod route_tests;

#[cfg(test)]
#[path = "state_place_tests.rs"]
mod place_tests;

#[cfg(test)]
#[path = "state_anchor_tests.rs"]
mod anchor_tests;

#[cfg(test)]
#[path = "state_marker_tests.rs"]
mod marker_tests;
