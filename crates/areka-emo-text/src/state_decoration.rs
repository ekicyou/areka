//! スコープごとの装飾状態と、追記される文字への番号付け（純粋層・`state.rs` の子モジュール）。
//!
//! `\f[...]` は台本のうえでは再生時間 0 の**汎用キャリア**（[`FONT_TAG_CARRIER`]）で運ばれ、
//! 本モジュールがその名前で自己選別して消費する。効果はスコープ（`\0`・`\1`・
//! `\p[n]`）ごとに独立で、**それ以降に追記される文字**にだけ効く。
//!
//! ## 何を持ち、何を持たないか
//!
//! - 持つもの——[`Decoration`]（2 層・現在の見た目・所有外キーの保持・記録済みの値）と、
//!   [`ActorTextState`] 側の番号列（`glyph_styles`）・装飾の表（`styles`）。
//! - 持たないもの——見た目の値の解釈そのもの。`\f` 1 件をどう見た目へ写すかは
//!   [`crate::look::apply_font_tag`] が唯一の実装点で、本モジュールは
//!   「どのスコープへ」「いつ」「記録をどう残すか」だけを決める。
//!
//! ## 寿命の 2 系統
//!
//! | 出来事 | 内容（`items`・`reveal`・`choices`・番号列・表） | 装飾状態（`decor`） |
//! |---|---|---|
//! | `\c`（`Clear`） | 消える | **保つ** |
//! | `\n`（`NewLine`）・`\_l`（`Cursor`） | 変わらない | **保つ** |
//! | 台本の先頭（`ClearAll`） | 消える | 既定へ戻る |
//! | `\f[default]` | 変わらない | 既定へ戻る |
//! | `\f[disable]` | 変わらない | 無効表示の層へ合う |
//!
//! 「戻す操作」の実体は 1 か所（[`ActorTextState::reset_look`]）で、`\f[default]` と
//! 台本の先頭はそこを通る。クリック待ち（`\x`）も同じ実体を通す前提だが、本番の呼び出し元はまだ無い。
//! 外から呼ぶ入口は [`TextLayerState::reset_decoration`] 1 本で、スコープ指定と
//! 全スコープの両方を受ける。`\f[disable]` は戻し先の層が違うだけの
//! 同じ一括の戻しである（[`ActorTextState::reset_look_disabled`]）。
//!
//! バルーンの装着（[`TextLayerState::set_look_layers`]）は戻す操作ではない——2 層を
//! 差し替え、現在の見た目を**新しい層の上へ載せ直す**（[`Decoration::rebase`]）。作者が
//! `\f` で明示した指定は残り、明示しなかった項目はバルーン定義の値になるので、
//! 装着より先に `\f` が届いても既定を取りこぼさない。
//!
//! 見た目を丸ごと置き換え、載せ直しで憶えるのも `\f` の命令の**トークン列**なので、
//! [`crate::look::TextLook`] へ項目が増えても列挙を直さずに戻し・載せ直しへ含まれる。
//! 戻す操作は既に追記済みの文字の番号を書き換えない（番号列は追記時の写しで、
//! 表の既存の番号の意味も変わらない）。
//!
//! ## 記録
//!
//! 記録は「キーと値」ごとに 1 台詞 1 度（`warned` の集合）。集合の要素は
//! **（記録の種別, キー, 値の列）**の 3 つ組で、台本作者の書いた文字列を 1 本の綴りへ一度も繋げない。
//! 種別が違えばキーと値が同じでも別の 1 件に数え（[`RECORD_FONT_ARG`]・[`RECORD_SCRIPT_APPEND`]）、
//! キーと値は別々の欄に入るので `\f[a=b,c]` と `\f[a,b=c]` が同じ鍵へ潰れることはなく、
//! 値も列のまま持つので `\f[bold,"x,y"]`（値 1 個）と `\f[bold,x,y]`（値 2 個）も別の 1 件に数える
//! （引用符が守るカンマは `areka-parsers` の `lexer_tests.rs::quoted_and_unquoted_args_mixed` が固定している）。
//! 台本の先頭で集合を空にするので、次の台詞では同じ指定がもう 1 度残る。
//! 記録なしで失敗を飲み込む経路は持たない——[`ActorTextState::apply_font_args`] の腕がすべて記録へ
//! 落ちることは、`match` の腕を字面で数える試験（`state_decoration_reset_tests.rs`）が見張る。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_compose::{BoxName, FontFollow};
use areka_sakura::contract::{ActorKey, FONT_TAG_CARRIER};

use crate::look::{LookLayers, Note, Script, StyleId, TextLook, apply_font_tag};
use crate::place::{PlaceKey, TextPlace};

use super::{ActorTextState, TextLayerState};

/// 記録の種別——`\f` の指定を適用できなかった側（[`ActorTextState::warn_once`]）。
///
/// [`Decoration::warned`] の鍵は（種別, キー, 値の列）の 3 つ組で、この定数はその第 1 要素である。
/// キーと値を繋げず別々の欄に置くのは、どちらも台本作者の書いた任意の文字列だからで、
/// 1 本の綴りへ潰すと `\f[a=b,c]`（キーは `a=b`）と `\f[a,b=c]`（キーは `a`）が同じ鍵になり、
/// 後から来た方が無記録で飲み込まれる。種別を鍵に含めるのも同じ理由で、そうしないと
/// `\f[sub,追記]` の記録が上下付きの追記の記録と同じ鍵になる。
const RECORD_FONT_ARG: u8 = 0;
/// 記録の種別——上下付きが有効なまま文字を追記した側（[`ActorTextState::warn_script_once`]）。
const RECORD_SCRIPT_APPEND: u8 = 1;

/// いま土台にしている層——一括の戻しの行き先（[`Decoration::base`]）。
///
/// 「どちらの層か」の 1 値だけを持ち、層の中身は持たない。装着で 2 層が差し替わったとき、
/// 新しい方の同じ層へ載せ直すために要る（[`Decoration::rebase`]）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BaseLayer {
    /// 既定の見た目（`\f[default]`・台本の先頭の戻し先）。
    #[default]
    Default,
    /// 無効表示の見た目（`\f[disable]` の戻し先）。
    Disable,
}

/// 台本の指定の系列の印——一括の戻しのたびと場所が
/// 生まれるときに、場所をまたいで一意の番号を振る（[`Lineage::fresh`]）。
///
/// 比べ（`PartialEq`）には入らない——同じ見た目と同じ指定の列を持つ装飾状態は同じとみなす
/// （`forget_pristine_places` の「作りたてと同じか」を変えないため）。`PartialEq` は手書きで、
/// いつも真を返す。写しの判定は中の番号を直に比べる。
#[derive(Clone, Copy, Debug, Default)]
struct Lineage(u64);

impl PartialEq for Lineage {
    fn eq(&self, _: &Lineage) -> bool {
        true
    }
}

/// 次に振る系列の印（1 から・番号は比べにも結果にも出ないので空回しの写しが振っても構わない）。
static NEXT_LINEAGE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Lineage {
    /// 場所をまたいで一意の新しい番号。
    fn fresh() -> Lineage {
        Lineage(NEXT_LINEAGE.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[cfg(test)]
thread_local! {
    /// 写しで当てた指定の数（[`Decoration::take_script`] が当てた指定の数の合計）。
    pub(super) static COPIED_SPECS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// 当て直しで当てた指定の数（[`Decoration::rebase`] が当てた `applied` の数の合計）。
    pub(super) static REBASED_SPECS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// 真なら、写しは 4 欄を複製して当て直し、2 層の差し込みは値に依らず当て直す作りで動く
    /// （試験の比べの正・読むのは写しと差し込みの道）。
    pub(super) static REPLAY_ALL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// スコープ 1 つ分の装飾状態（2 層・現在の見た目・所有外キーの保持・記録済みの値）。
///
/// [`Decoration::default`] は ukadoc の既定（[`LookLayers::default`]）で、バルーンが装着される
/// 前に届いた cue のための出発点である。装着時に 2 層を差し込む口は
/// [`TextLayerState::set_look_layers`]。
#[derive(Clone, Debug, PartialEq)]
pub struct Decoration {
    /// 既定・無効表示・選択肢文字色（`\f[...,default]`／`\f[...,disable]` の戻し先）。
    layers: LookLayers,
    /// いま効いている見た目（次に追記される文字が受け取る値）。
    current: TextLook,
    /// いま土台にしている層（[`Decoration::rebase`] の載せ直し先）。
    base: BaseLayer,
    /// 一括の戻しから後に届いた `\f` の命令列（**トークンのまま**）。
    ///
    /// [`Decoration::rebase`] が新しい土台の上でこれを再生する。トークンのまま持つのは、
    /// 「どの項目が明示されたか」を項目ごとの欄で持つと [`crate::look::TextLook`] へ項目を足すたびに
    /// 欄と併合を直すことになるため。再生は [`crate::look::apply_font_tag`] 1 か所を通る。
    applied: Vec<Vec<String>>,
    /// 装飾が意味を与えないキーの最新の引数列（キー→引数列）。
    unowned: BTreeMap<String, Vec<String>>,
    /// 記録済みの（種別, キー, 値の列）（同じ指定の警告を 1 台詞に 1 度へ抑えるための集合）。
    ///
    /// 種別は [`RECORD_FONT_ARG`]／[`RECORD_SCRIPT_APPEND`] で、2 種類の記録に互いに素な
    /// 鍵空間を与える。キーと値も繋げず別々の欄に置き、値は列のまま持つ——区切り文字で 1 本の綴りへ
    /// 潰すと別々の指定が同じ鍵になって片方が無記録で消える（キーと値を `=` で繋ぐと
    /// `\f[a=b,c]` と `\f[a,b=c]`、値の列を `,` で繋ぐと `\f[bold,"x,y"]` と `\f[bold,x,y]`）。
    /// 値を持たない記録（[`RECORD_SCRIPT_APPEND`]）は第 3 要素を空の列にする。
    warned: BTreeSet<(u8, String, Vec<String>)>,
    /// 台本の指定の系列の印（比べには入らない・[`Lineage`]）。
    lineage: Lineage,
}

/// 写しの材料（持ち主から差分だけを取り出したもの・[`Decoration::delta_for`]）。
struct ScriptDelta {
    lineage: Lineage,
    base: BaseLayer,
    /// 真なら先を元の土台から作り直してから `specs` を当てる（全部の道）。
    from_start: bool,
    /// 先がまだ当てていない指定（元の `applied` の後ろの部分の複製）。
    specs: Vec<Vec<String>>,
    /// 元から動かした記録済みの印。
    warned: BTreeSet<(u8, String, Vec<String>)>,
}

impl Default for Decoration {
    fn default() -> Decoration {
        let layers = LookLayers::default();
        let current = layers.default.clone();
        Decoration {
            layers,
            current,
            base: BaseLayer::Default,
            applied: Vec::new(),
            unowned: BTreeMap::new(),
            warned: BTreeSet::new(),
            lineage: Lineage::fresh(),
        }
    }
}

impl Decoration {
    /// 場所の既定（2 層）だけを持ち、台本の指定が空の装飾状態（箱の場所の生まれたとき）。
    fn with_layers(layers: LookLayers) -> Decoration {
        let current = layers.default.clone();
        Decoration {
            layers,
            current,
            ..Decoration::default()
        }
    }

    /// 台本の指定（土台・当てた指定の列・所有外のキー・記録済みの値）が 1 つでもあるか。
    fn has_script(&self) -> bool {
        self.base != BaseLayer::Default
            || !self.applied.is_empty()
            || !self.unowned.is_empty()
            || !self.warned.is_empty()
    }

    /// 先が持つ（系列の印, 当てた数）。
    fn held(&self) -> (Lineage, usize) {
        (self.lineage, self.applied.len())
    }

    /// 先が持つ `held` が自分（持ち主）の `applied` の頭か——同じ印で、数が 0 より大きく自分の数以下
    /// （1 つの系列へ書き込むのは今の持ち主だけ）。
    fn continues(&self, (lineage, count): (Lineage, usize)) -> bool {
        count > 0 && lineage.0 == self.lineage.0 && count <= self.applied.len()
    }

    /// 持ち主から、先が持つ `held`（先が無ければ `None`）に足りない差分を取り出し、記録済みの印を動かす。
    fn delta_for(&mut self, held: Option<(Lineage, usize)>) -> ScriptDelta {
        let skip = held.filter(|h| self.continues(*h)).map(|(_, count)| count);
        ScriptDelta {
            lineage: self.lineage,
            base: self.base,
            from_start: skip.is_none(),
            specs: self.applied[skip.unwrap_or(0)..].to_vec(),
            warned: std::mem::take(&mut self.warned),
        }
    }

    /// 差分を受け取る（全部の道なら元の土台から作り直す）。指定を今の見た目へ 1 度ずつ当て、
    /// 所有外のキーはその場で入れる（`Note::Unowned` はキーだけで決まる）。記録は出さない
    /// （元で 1 度記録済み）。場所の既定（`layers`）は動かさない。
    fn take_script(&mut self, delta: ScriptDelta) {
        if delta.from_start {
            self.base = delta.base;
            self.applied.clear();
            self.unowned.clear();
            self.current = self.base_look();
        }
        self.lineage = delta.lineage;
        self.warned = delta.warned;
        #[cfg(test)]
        COPIED_SPECS.with(|c| c.set(c.get() + delta.specs.len()));
        for tokens in delta.specs {
            let args: Vec<&str> = tokens.iter().map(String::as_str).collect();
            if let Ok(Some(Note::Unowned)) = apply_font_tag(&mut self.current, &self.layers, &args)
            {
                self.unowned.insert(tokens[0].clone(), tokens[1..].to_vec());
            }
            self.applied.push(tokens);
        }
    }

    /// 試験用の写し（4 欄を `from` から複製して当て直す・`REPLAY_ALL` の比べの正）。
    #[cfg(test)]
    fn take_script_replay_all(&mut self, from: &Decoration) {
        self.base = from.base;
        self.applied = from.applied.clone();
        self.unowned = from.unowned.clone();
        self.warned = from.warned.clone();
        COPIED_SPECS.with(|c| c.set(c.get() + self.applied.len()));
        self.replay();
    }

    /// いま土台にしている層の見た目。
    fn base_look(&self) -> TextLook {
        match self.base {
            BaseLayer::Default => self.layers.default.clone(),
            BaseLayer::Disable => self.layers.disable.clone(),
        }
    }

    /// 差し替わった 2 層の上へ現在の見た目を載せ直す（装着の 1 点から呼ぶ）。
    ///
    /// 土台を新しい層にしてから、一括の戻しから後に届いた `\f` の命令をそのままの順で再生する。
    /// 作者が明示した指定は残り、明示しなかった項目はバルーン定義の値になる。
    ///
    /// 相対値（`\f[height,+4]`・`\f[height,150%]`）と `\f[キー,default]` の類は、再生の時点で
    /// **新しい既定**を基準に解き直される（装着の前後で「バルーン定義に対する相対」という
    /// 作者の意図が保たれる）。
    ///
    /// 記録は残さない——再生する命令は既に 1 度適用され、記録すべきものは記録済み
    /// （[`Decoration::warned`] は台詞の寿命で、装着はそれを跨がない）。
    fn rebase(&mut self) {
        #[cfg(test)]
        REBASED_SPECS.with(|c| c.set(c.get() + self.applied.len()));
        self.replay();
    }

    /// 2 層の差し込みの共通の実体（呼び手は [`TextLayerState::set_look_layers`] と
    /// [`TextLayerState::set_box_traits`] の 2 か所だけ）。今と同じ値なら何もせず、違えば差し替えて
    /// 当て直す。`REPLAY_ALL` が真なら値に依らず当て直す（試験用）。
    fn set_layers(&mut self, layers: &LookLayers) {
        #[cfg(test)]
        let always = REPLAY_ALL.with(std::cell::Cell::get);
        #[cfg(not(test))]
        let always = false;
        if always || self.layers != *layers {
            self.layers = layers.clone();
            self.rebase();
        }
    }

    /// 土台の見た目から `applied` を順に当てる（写しと当て直しの共通の実体・数えは呼び手が持つ）。
    fn replay(&mut self) {
        self.current = self.base_look();
        for tokens in &self.applied {
            let args: Vec<&str> = tokens.iter().map(String::as_str).collect();
            let _ = apply_font_tag(&mut self.current, &self.layers, &args);
        }
    }
}

impl ActorTextState {
    /// グリフ序数と同じ序数空間の装飾番号列（`items` のグリフだけを数えた序数）。
    pub fn glyph_styles(&self) -> &[StyleId] {
        &self.glyph_styles
    }

    /// このスコープの装飾の表（番号 → 既定と異なる見た目）。
    pub fn styles(&self) -> &crate::look::StyleTable {
        &self.styles
    }

    /// いま効いている見た目（次に追記される文字が受け取る値）。
    pub fn current_look(&self) -> &TextLook {
        &self.decor.current
    }

    /// このスコープの 2 層（既定・無効表示・選択肢文字色）。
    ///
    /// 本番の経路にこの読み口の呼び手は無く（差し込みは [`TextLayerState::set_look_layers`]、
    /// 戻し先と載せ直しは `self.decor.layers` の直読み）、呼び手はテストだけ
    /// （`src/` の `actor_*`・`state_*` の決定論テストと `tests/shell_balloon_fixture_test.rs`）。
    pub fn look_layers(&self) -> &LookLayers {
        &self.decor.layers
    }

    /// 所有外キーの最新の引数列（戻す操作で空になる）。本番の呼び手はまだ無い継ぎ目で、
    /// 後続の消費者が読む前提。
    pub fn unowned_vocab(&self) -> &BTreeMap<String, Vec<String>> {
        &self.decor.unowned
    }

    /// 内容だけを消す（`\c`＝`Clear`・装飾状態は保つ）。
    ///
    /// 番号列（グリフ序数の写し）と装飾の表（その内容が使っていた見た目の入れ物）は内容と同じ寿命で、
    /// 同時に空にする。消えないのは `decor`（現在の見た目・所有外キーの保持・記録済みの値）だけ。
    pub(super) fn clear_content(&mut self) {
        self.items.clear();
        self.reveal = super::RevealSchedule::default();
        self.choices.clear();
        self.anchor_open = None;
        self.settled = 0;
        self.glyph_styles.clear();
        self.styles.clear();
    }

    /// いま効いている見た目を表に登録し、追記する文字数だけ番号を並べる。
    ///
    /// 追記点が文字（`Text`）と選択肢（`Choice`）の 2 か所あるので、登録と番号の push をここ 1 つにまとめる
    /// （不変条件「`glyph_styles.len` は `items` のグリフ数に等しい」の実装点）。0 文字ガードを実際に踏むのは
    /// `Text` の腕で、`Choice` の腕は上位で 0 文字を分岐するのでここへ来ない。
    ///
    /// `actor` は記録（上下付きが有効なまま文字が追記された）に使う——ここが「文字が追記された」ことを
    /// 知る唯一の地点。`quiet` が真（空回しの写し）なら `warn!` を出さない（記録済みの集合は同じく更新する）。
    ///
    /// アンカー（`\_a`）の範囲が開いている間は、いま効いている見た目の**写し**に下線を立てて登録する。
    /// 作者の装飾の状態そのものは変えないので、`\f[default]`／`\f[disable]` の戻し先にも閉じた後の文字にも
    /// 残らない。範囲の中で作者が下線を切っても、アンカーの下線が勝つ。
    pub(super) fn push_current_style(&mut self, actor: &ActorKey, glyph_count: usize, quiet: bool) {
        if glyph_count == 0 {
            return;
        }
        self.warn_script_once(actor, quiet);
        let mut look = std::borrow::Cow::Borrowed(&self.decor.current);
        if self.anchor_is_open() {
            look.to_mut().underline = true;
        }
        let id = self.styles.intern(&look, &self.decor.layers.default);
        self.glyph_styles
            .extend(std::iter::repeat_n(id, glyph_count));
    }

    /// 「戻す操作」の実体——見た目を既定へ丸ごと戻し、所有外キーの保持も空にする。既に追記済みの文字には効かない。
    pub(super) fn reset_look(&mut self) {
        self.reset_look_to(BaseLayer::Default);
    }

    /// 無効表示への一括の戻し（`\f[disable]`）。
    ///
    /// `\f[disable]` も `\f[default]` と同じ「一括の戻し」で、所有外キーの保持も空にする
    /// （装飾状態の全項目が対象なので、所有外キーだけが生き延びてはならない）。
    /// 2 つの綴りの違いは戻し先の層だけ。装飾状態の道は `look.rs::apply_font_tag` の
    /// `key == "disable"`／`"default"` の腕を通らない（通るのは `look.rs::classify_font_tag` の分類とテストだけ）。
    pub(super) fn reset_look_disabled(&mut self) {
        self.reset_look_to(BaseLayer::Disable);
    }

    /// 一括の戻しの共通実体——見た目を丸ごと置き換え、所有外キーの保持を空にする。
    ///
    /// 見た目は丸ごと置き換える（項目を列挙しない）ので、[`TextLook`] へ項目が増えても自動で含まれる。
    /// 併せて土台の層を憶え、そこから後の `\f` の命令列を空にする——戻しの後は「土台そのまま」が
    /// 現在の見た目なので、載せ直し（[`Decoration::rebase`]）は新しい土台をそのまま採ればよい。
    fn reset_look_to(&mut self, base: BaseLayer) {
        self.decor.base = base;
        self.decor.applied.clear();
        let layer = self.decor.base_look();
        self.decor.current = layer;
        self.decor.unowned.clear();
        self.decor.lineage = Lineage::fresh();
    }

    /// 台本の先頭（`ClearAll`）の戻し——[`ActorTextState::reset_look`] に加えて記録済みの値も
    /// 空にする（次の台詞では同じ指定がもう 1 度記録される）。
    pub(super) fn reset_for_new_talk(&mut self) {
        self.reset_look();
        self.decor.warned.clear();
    }

    /// `\f` 1 件（キャリアのトークン列）を適用し、必要な記録を残す。
    ///
    /// `tokens[0]` がキー、`tokens[1..]` が値の列。値の解釈は
    /// [`crate::look::apply_font_tag`] の担当で、本関数は「戻す操作へ回すか」「所有外キーを
    /// 保持するか」「何を記録するか」だけを決める。記録なしで失敗を飲み込む経路は無い
    /// （`Err` と記録すべき [`Note`] は必ず `warn!`・所有外キーは `debug!`）。
    /// `quiet` が真（空回しの写し）なら `warn!` だけを出さない（記録済みの集合と `debug!` は同じ）。
    pub(super) fn apply_font_args(&mut self, actor: &ActorKey, tokens: &[&str], quiet: bool) {
        // 一括の戻しは `apply_font_tag` の腕でなく「戻す操作」を通す: 腕は見た目しか置き換えず、
        // 所有外キーの保持が残ってしまう。`\f[disable]` は戻し先の層が違うだけ。
        match tokens.first().copied() {
            Some("default") => {
                tracing::debug!(actor = %actor, "\\f[default]——スコープの装飾状態を既定へ戻す（要件 10.1）");
                self.reset_look();
                return;
            }
            Some("disable") => {
                tracing::debug!(actor = %actor, "\\f[disable]——スコープの装飾状態を無効表示へ合わせる（要件 10.2）");
                self.reset_look_disabled();
                return;
            }
            _ => {}
        }
        let outcome = apply_font_tag(&mut self.decor.current, &self.decor.layers, tokens);
        if outcome.is_ok() {
            // 載せ直しのためトークンのまま憶える（[`Decoration::rebase`]）。失敗した指定は憶えない。
            self.decor
                .applied
                .push(tokens.iter().map(|token| (*token).to_owned()).collect());
        }
        match outcome {
            Ok(None) => {}
            // catch-all を置かないのは、`Note` に腕が増えたときコンパイラに「保持か記録か」の再検討を強制するため。
            Ok(Some(Note::Unowned)) => {
                // キーが空なら apply_font_tag は Err を返すので、ここでは必ずキーがある。
                let key = tokens[0].to_owned();
                tracing::debug!(actor = %actor, key = %key, "`areka-P0-text-decoration-canon` の所有外の \\f のキー——引数列を保持し表示は変えない（要件 2.5）");
                self.decor.unowned.insert(
                    key,
                    tokens[1..]
                        .iter()
                        .map(|value| (*value).to_owned())
                        .collect(),
                );
            }
            Ok(Some(Note::VocabularyOnly { key })) => {
                self.warn_once(
                    actor,
                    key,
                    &tokens[1..],
                    "語彙として受理したが表示は変えない",
                    quiet,
                );
            }
            Ok(Some(Note::StylesheetKeyword)) => {
                self.warn_once(
                    actor,
                    "height",
                    &tokens[1..],
                    "スタイルシートの大きさの語は語彙のみ——大きさを変えない",
                    quiet,
                );
            }
            Ok(Some(Note::AnchorColorAsDefault)) => {
                self.warn_once(
                    actor,
                    "color",
                    &tokens[1..],
                    "アンカーの色定義がまだ無い——default と同じ色を適用した",
                    quiet,
                );
            }
            Err(issue) => {
                // 鍵はトークンの列から作る（`FontTagIssue::value` は連結済みの綴りで、鍵にすると
                // `\f[bold,"x,y"]` と `\f[bold,x,y]` が潰れる）。キーが空の失敗では `tokens` が空になりうる。
                let key = issue.key.clone();
                let rest = tokens.get(1..).unwrap_or(&[]);
                self.warn_once(actor, &key, rest, issue.reason, quiet);
            }
        }
    }

    /// 同じ（キー, 値の列）の警告を 1 台詞に 1 度だけ残す。
    ///
    /// 記録の本文には値の列をカンマで繋いだ綴りを載せるが、鍵は列のまま
    /// （綴りを鍵にすると `\f[bold,"x,y"]` と `\f[bold,x,y]` が潰れ、片方が無記録で消える）。
    /// `quiet` が真（空回しの写し）なら集合だけを更新して `warn!` を出さない。
    fn warn_once(
        &mut self,
        actor: &ActorKey,
        key: &str,
        values: &[&str],
        reason: &str,
        quiet: bool,
    ) {
        let text = values.join(",");
        let owned: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
        if self
            .decor
            .warned
            .insert((RECORD_FONT_ARG, key.to_owned(), owned))
            && !quiet
        {
            tracing::warn!(actor = %actor, key, value = text, reason, "\\f の指定を適用できない——当該項目は変えずに再生を続ける");
        }
    }

    /// 上下付きが有効なまま文字が追記されたことを 1 台詞に 1 度だけ残す。
    ///
    /// 記録済みの集合は [`ActorTextState::warn_once`] と共有するが鍵空間は互いに素で、こちらの鍵は
    /// （[`RECORD_SCRIPT_APPEND`], キー（`sub`／`sup`）, 空の列）。種別を落とすと
    /// `\f[sub,追記]`（値が不正なので `warn_once` が記録する）が直後の追記の記録を無記録で飲み込む
    /// （逆順なら `warn_once` の側が消える）。文面を分けるのは、「指定が通らない」のか
    /// 「指定は通ったが表示に出ない」のかをログから見分けられるようにするため。
    fn warn_script_once(&mut self, actor: &ActorKey, quiet: bool) {
        let key = match self.decor.current.script {
            Script::None => return,
            Script::Sub => "sub",
            Script::Sup => "sup",
        };
        if self
            .decor
            .warned
            .insert((RECORD_SCRIPT_APPEND, key.to_owned(), Vec::new()))
            && !quiet
        {
            tracing::warn!(actor = %actor, key, reason = "上下付きは語彙のみ——基線も大きさも送り幅も変えない", "上下付きが有効なまま文字を追記した——表示は変わらない");
        }
    }
}

/// 箱の名前ごとの既定の見た目（拡大率に依らない 2 層）と箱の種類（`font.follow`）。
#[derive(Clone, Debug, PartialEq)]
pub struct BoxTraits {
    /// 箱の `balloon.*`ブレスから解決した 2 層（箱の場所の既定）。
    pub looks: LookLayers,
    /// 装飾の指定の行き先（スコープに従う／箱に閉じる）。
    pub follow: FontFollow,
}

impl TextLayerState {
    /// 箱の名前ごとの既定の見た目（拡大率に依らない 2 層）と種類を受け取る。
    ///
    /// 箱の場所の状態は作られるときにこの 2 層を受け取る（[`TextLayerState::place_entry`]）ので、
    /// 文字が届く前に既定が決まっている。すでにある箱の場所は、装着と同じく新しい 2 層の上へ
    /// 載せ直す（[`Decoration::rebase`]・今と同じ 2 層なら何もしない）。種類は行き先が替わるときの写しが引く
    /// （[`TextLayerState::carry_script_decor`]）。表に無い名前の箱は ukadoc の既定の 2 層で、
    /// スコープに従う。
    pub fn set_box_traits(&mut self, traits: BTreeMap<BoxName, BoxTraits>) {
        for (key, state) in self.actors.iter_mut() {
            if let TextPlace::Box(name) = &key.place
                && let Some(t) = traits.get(name)
            {
                state.decor.set_layers(&t.looks);
            }
        }
        tracing::debug!(boxes = traits.len(), "箱の既定の見た目と種類を受け取った");
        self.box_traits = traits;
    }

    /// 場所の状態を引く（無ければ作る）。箱の場所はその箱の既定の 2 層を持って生まれる。
    pub(super) fn place_entry(&mut self, key: &PlaceKey) -> &mut ActorTextState {
        let traits = &self.box_traits;
        self.actors.entry(key.clone()).or_insert_with(|| {
            let looks = match &key.place {
                TextPlace::Box(name) => traits.get(name).map(|t| t.looks.clone()),
                TextPlace::Balloon => None,
            };
            ActorTextState {
                decor: looks.map(Decoration::with_layers).unwrap_or_default(),
                ..ActorTextState::default()
            }
        })
    }

    /// 場所がスコープに従うか（普通のバルーンは常に従う・表に無い箱も従う）。
    fn follows_scope(&self, place: &TextPlace) -> bool {
        match place {
            TextPlace::Balloon => true,
            TextPlace::Box(name) => self
                .box_traits
                .get(name)
                .is_none_or(|t| t.follow == FontFollow::Scope),
        }
    }

    /// 行き先が `new` へ替わる瞬間の装飾の持ち運び。
    ///
    /// `new` がスコープに従う場所なら、スコープの指定を持つ場所（`shared`）の台本の指定を
    /// `new` へ写して `new` を持ち主にする。箱に閉じる箱なら何も写さず持ち主も変えない
    /// （その箱は自分の指定のまま）。前の行き先の種類は見ない。行き先そのものの書き換えは
    /// 呼び手（`state_route.rs`）が行う。
    pub(super) fn carry_script_decor(&mut self, actor: &ActorKey, new: &TextPlace) {
        if !self.follows_scope(new) {
            tracing::debug!(actor = %actor, dest = ?new, "箱に閉じる箱へ——\\f の指定を写さない");
            return;
        }
        let route = self.routes.entry(actor.clone()).or_default();
        let shared = std::mem::replace(&mut route.shared, new.clone());
        if shared != *new {
            self.copy_script(actor, &shared, new);
        }
    }

    /// 台本の指定を `from`（持ち主）の場所から `to` の場所へ写す——`to` が持つ系列の印と当てた数を読み、
    /// 足りない後ろの指定だけを当てる（印が違うか数が合わなければ元の土台から全部当てる）。記録済みの印は
    /// 動かし、所有外のキーは当てながら作る。
    ///
    /// `from` の状態がまだ無ければ空の指定を写す。写すものが空で `to` の状態もまだ無ければ、
    /// 生まれたときの状態と同じなので作らない。
    pub(super) fn copy_script(&mut self, actor: &ActorKey, from: &TextPlace, to: &TextPlace) {
        let from_key = PlaceKey {
            actor: actor.clone(),
            place: from.clone(),
        };
        let to_key = PlaceKey {
            actor: actor.clone(),
            place: to.clone(),
        };
        #[cfg(test)]
        if REPLAY_ALL.with(std::cell::Cell::get) {
            let source = self
                .actors
                .get(&from_key)
                .map(|state| state.decor.clone())
                .unwrap_or_default();
            if !source.has_script() && !self.actors.contains_key(&to_key) {
                return;
            }
            tracing::debug!(actor = %actor, ?from, ?to, "スコープの \\f の指定を新しい行き先へ写す（`areka-P0-shell-balloon` 要件 3.13）");
            self.place_entry(&to_key)
                .decor
                .take_script_replay_all(&source);
            return;
        }
        let source = self.actors.get(&from_key).map(|state| &state.decor);
        let held = self.actors.get(&to_key).map(|state| state.decor.held());
        if held.is_none() && !source.is_some_and(Decoration::has_script) {
            return;
        }
        // 前提「同じ系列で短い方は長い方の頭」を試験の作りでだけ確かめる（最後の指定が元の同じ位置と同じ）。
        debug_assert!(
            match (source, self.actors.get(&to_key)) {
                (Some(src), Some(dst)) if src.continues(dst.decor.held()) => {
                    dst.decor.applied.last() == src.applied.get(dst.decor.applied.len() - 1)
                }
                _ => true,
            },
            "同じ系列の先の指定が元の指定の頭でない"
        );
        tracing::debug!(actor = %actor, ?from, ?to, "スコープの \\f の指定を新しい行き先へ写す（`areka-P0-shell-balloon` 要件 3.13）");
        let delta = match self.actors.get_mut(&from_key) {
            Some(state) => state.decor.delta_for(held),
            // 元の場所が無いときは空の指定を写す（先を全部の道で空にする）。
            None => Decoration::default().delta_for(held),
        };
        self.place_entry(&to_key).decor.take_script(delta);
    }

    /// バルーンの装着で 2 層を差し込む（結線層の 1 点から呼ぶ）。
    ///
    /// 2 層を差し替えたうえで、現在の見た目を新しい層の上へ**載せ直す**（[`Decoration::rebase`]）。
    /// 土台は新しいバルーン定義の値になり、作者が `\f` で明示した指定は残る。
    ///
    /// # 装着より先に `\f` が届いても取りこぼさない
    ///
    /// cue のドレインは非同期で、`text_slot_view` が `None` の間は装着が次フレームへ委ねられる
    /// （`areka` の `runtime::frame::attach::connect_balloon_text` の `None` の腕）。この窓で
    /// `\f[...]` が先に届いても、載せ直しが作者の命令を新しい既定の上で再生するので
    /// バルーン定義の既定（大きさ・色・フォント名）はその台詞に届く。流し始めを遅らせて順序を作る必要は無い。
    ///
    /// 再装着でも同じ経路を通る（土台だけが新しい値へ移り、明示された指定は保たれる）。
    /// 今と同じ 2 層なら何もしない。
    pub fn set_look_layers(&mut self, actor: &ActorKey, layers: LookLayers) {
        let state = self.actors.entry(PlaceKey::balloon(actor)).or_default();
        state.decor.set_layers(&layers);
    }

    /// 「戻す操作」の権威定義——装飾状態の全項目を既定の見た目へ戻す。
    ///
    /// `scope` が `Some` なら当該スコープだけ、`None` なら全スコープを 1 回で戻す。
    /// `\f[default]`（[`ActorTextState::apply_font_args`]）と台詞の開始
    /// （`ClearAll`→[`ActorTextState::reset_for_new_talk`]）は同じ実体（[`ActorTextState::reset_look`]）を通る。
    /// クリック待ち（`\x`）もここを通す前提だが、本番の呼び出し元はまだ無い。
    /// 既に表示済みの文字の見た目は変わらない。
    pub fn reset_decoration(&mut self, scope: Option<&ActorKey>) {
        match scope {
            Some(actor) => {
                tracing::debug!(actor = %actor, "戻す操作——当該スコープの装飾状態を既定へ戻す（要件 10.3）");
                // まだ生まれていないスコープには戻すものが無いので、`entry` で器を作らず `get_mut` で引く。
                let key = PlaceKey {
                    actor: actor.clone(),
                    place: self.destination(actor),
                };
                if let Some(state) = self.actors.get_mut(&key) {
                    state.reset_look();
                }
            }
            None => {
                tracing::debug!("戻す操作——全スコープの装飾状態を既定へ戻す（要件 10.5）");
                for state in self.actors.values_mut() {
                    state.reset_look();
                }
            }
        }
    }
}

/// `\f` を運ぶ汎用キャリアか（名前による自己選別）。
///
/// 名前が合わない運搬（`\!` の他コマンド）と非正準な params は `None`（呼び手は読み飛ばす）。
pub(super) fn font_tag_tokens(command: &areka_sakura::contract::CueCommand) -> Option<Vec<&str>> {
    match command.as_command_carrier() {
        Some((name, tokens)) if name == FONT_TAG_CARRIER => Some(tokens),
        _ => None,
    }
}

#[cfg(test)]
#[path = "state_decoration_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "state_decoration_carry_tests.rs"]
mod carry_tests;

#[cfg(test)]
#[path = "state_decoration_incremental_tests.rs"]
mod incremental_tests;
