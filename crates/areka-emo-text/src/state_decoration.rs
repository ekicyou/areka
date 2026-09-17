//! スコープごとの装飾状態と、追記される文字への番号付け（純粋層・`state.rs` の子モジュール）。
//!
//! `\f[...]` は台本のうえでは再生時間 0 の**汎用キャリア**（[`FONT_TAG_CARRIER`]）で運ばれ、
//! 本モジュールがその名前で自己選別して消費する（要件 2.4）。効果はスコープ（`\0`・`\1`・
//! `\p[n]`）ごとに独立で（要件 3.1）、**それ以降に追記される文字**にだけ効く（要件 3.2）。
//!
//! ## 何を持ち、何を持たないか
//!
//! - 持つもの——[`Decoration`]（2 層・現在の見た目・所有外キーの保持・記録済みの値）と、
//!   [`ActorTextState`] 側の番号列（`glyph_styles`）・装飾の表（`styles`）。
//! - 持たないもの——見た目の値の解釈そのもの。`\f` 1 件をどう見た目へ写すかは
//!   [`crate::look::apply_font_tag`] が唯一の実装点で、本モジュールは
//!   「どのスコープへ」「いつ」「記録をどう残すか」だけを決める。
//!
//! ## 寿命の 2 系統（要件 3.7／3.8・design.md「Flow 2」）
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
//! 台本の先頭はそこを通る（要件 10.3）。後続仕様のクリック待ち（`\x`）も同じ実体を
//! 通す前提で口を開けてあるが、`\x` の実装は本仕様の射程外で**本番の呼び出し元はまだ
//! 無い**（引受先＝`areka-P0-balloon-lifecycle-events`）。
//! 外から呼ぶ入口は [`TextLayerState::reset_decoration`] 1 本で、スコープ指定と
//! 全スコープ（要件 10.5）の両方を受ける。`\f[disable]` は戻し先の層が違うだけの
//! 同じ一括の戻しである（[`ActorTextState::reset_look_disabled`] の裁定）。
//!
//! バルーンの装着（[`TextLayerState::set_look_layers`]）は戻す操作ではない——2 層を
//! 差し替え、現在の見た目を**新しい層の上へ載せ直す**（[`Decoration::rebase`]）。作者が
//! `\f` で明示した指定は残り、明示しなかった項目はバルーン定義の値になるので、
//! **装着より先に `\f` が届いても既定を取りこぼさない**（開発者裁定 2026-09-13）。
//!
//! 見た目を**丸ごと**置き換えるので、後続仕様が [`crate::look::TextLook`] へ項目を足せば
//! 列挙を直さずに戻しへ含まれる（要件 10.4）。載せ直しも同じ性質を持つ——憶えるのは
//! `\f` の命令の**トークン列**であって項目ごとの欄ではない。戻す操作は既に追記済みの文字の番号を
//! 書き換えない——番号列は追記時の写しであり、表の既存の番号の意味も変わらないから
//! である（要件 10.7）。
//!
//! ## 記録（要件 13.1／13.4／13.5）
//!
//! 記録は「キーと値」ごとに 1 台詞 1 度（`warned` の集合）。集合の要素は
//! **（記録の種別, キー, 値の列）**の 3 つ組で、台本作者の書いた文字列を 1 本の綴りへ
//! **一度も繋げない**——鍵のどの要素も連結された綴りではない。種別が違えばキーと値が
//! 同じでも別の 1 件に数え（[`RECORD_FONT_ARG`]・[`RECORD_SCRIPT_APPEND`]）、同じ種別の
//! 中でもキーと値は別々の欄に入るので区切り文字を含む指定（`\f[a=b,c]` と `\f[a,b=c]`）が
//! 同じ鍵へ潰れることはなく、値も列のまま持つので `\f[bold,"x,y"]`（値 1 個）と
//! `\f[bold,x,y]`（値 2 個）も別の 1 件に数える（引用符が守るカンマは
//! `areka-parsers` の `lexer_tests.rs::quoted_arg_protects_comma` が固定している）。台本の先頭で集合を空にするので、
//! 次の台詞では同じ指定がもう 1 度残る。記録なしで失敗を飲み込む経路は持たない——
//! [`ActorTextState::apply_font_args`] の腕がすべて記録へ落ちることは、`match` の腕を
//! 字面で数える試験（`state_decoration_reset_tests.rs` §13）が見張る。

use std::collections::{BTreeMap, BTreeSet};

use areka_sakura::contract::{ActorKey, FONT_TAG_CARRIER};

use crate::look::{LookLayers, Note, Script, StyleId, TextLook, apply_font_tag};

use super::{ActorTextState, TextLayerState};

/// 記録の種別——`\f` の指定を適用できなかった側（[`ActorTextState::warn_once`]）。
///
/// [`Decoration::warned`] の鍵は（種別, キー, 値の列）の 3 つ組で、この定数はその第 1 要素である。
/// キーと値を繋げず別々の欄に置くのは、どちらも台本作者の書いた任意の文字列だからで、
/// 1 本の綴りへ潰すと `\f[a=b,c]`（キーは `a=b`）と `\f[a,b=c]`（キーは `a`）が同じ鍵になり、
/// 後から来た方が無記録で飲み込まれる。種別を鍵に含めるのも同じ理由で、そうしないと
/// `\f[sub,追記]` の記録が上下付きの追記の記録と同じ鍵になる（要件 13.4／13.5）。
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
    /// 一括の戻しから後に届いた `\f` の命令列（**トークンのまま**・要件 4.1／10.4）。
    ///
    /// [`Decoration::rebase`] が新しい土台の上でこれを再生する。**トークンのまま持つ**のが
    /// 肝で、「どの項目が明示されたか」を項目ごとの欄で持つと後続仕様が
    /// [`crate::look::TextLook`] へ項目を足すたびに欄と併合を直すことになる（要件 10.4 が
    /// 禁じている列挙）。再生は [`crate::look::apply_font_tag`] 1 か所を通るので、
    /// 項目が増えても再生の側は変わらない。
    applied: Vec<Vec<String>>,
    /// 本仕様が意味を与えないキーの最新の引数列（要件 2.5・キー→引数列）。
    unowned: BTreeMap<String, Vec<String>>,
    /// 記録済みの（種別, キー, 値の列）（同じ指定の警告を 1 台詞に 1 度へ抑えるための集合・要件 13.4）。
    ///
    /// 種別は [`RECORD_FONT_ARG`]／[`RECORD_SCRIPT_APPEND`] で、2 種類の記録に互いに素な
    /// 鍵空間を与える。キーと値も繋げず別々の欄に置き、**値は列のまま**持つ——台本作者の
    /// 書いた文字列を区切り文字で 1 本の綴りへ潰すと、別々の指定が同じ鍵になって片方が
    /// 無記録で消える（要件 13.5）。潰れる例は 2 つとも実測で確かめた: キーと値を `=` で
    /// 繋ぐと `\f[a=b,c]` と `\f[a,b=c]`、値の列を `,` で繋ぐと `\f[bold,"x,y"]` と
    /// `\f[bold,x,y]`。よって鍵のどの要素も連結された綴りではない。
    /// 値を持たない記録（[`RECORD_SCRIPT_APPEND`]）は第 3 要素を空の列にする。
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
        }
    }
}

impl Decoration {
    /// いま土台にしている層の見た目。
    fn base_look(&self) -> TextLook {
        match self.base {
            BaseLayer::Default => self.layers.default.clone(),
            BaseLayer::Disable => self.layers.disable.clone(),
        }
    }

    /// 差し替わった 2 層の上へ現在の見た目を載せ直す（要件 4.1・装着の 1 点から呼ぶ）。
    ///
    /// 土台を新しい層にしてから、一括の戻しから後に届いた `\f` の命令を**そのままの順で
    /// 再生する**。作者が明示した指定はそのまま残り、明示しなかった項目はバルーン定義の
    /// 値になる——項目を 1 つも名指ししないので、後続仕様が [`crate::look::TextLook`] へ
    /// 足した項目も同じ規則で載る（要件 10.4）。
    ///
    /// 相対値（`\f[height,+4]`・`\f[height,150%]`）と `\f[キー,default]` の類は、再生の
    /// 時点で**新しい既定**を基準に解き直される。これは装着の前後で「バルーン定義に対する
    /// 相対」という作者の意図が保たれる側であり、旧い既定を基準に確定した数値を運ぶより
    /// 正しい。
    ///
    /// 記録は残さない——再生する命令は既に 1 度適用されており、そのとき記録すべきものは
    /// 記録済みだからである（[`Decoration::warned`] は台詞の寿命で、装着はそれを跨がない）。
    fn rebase(&mut self) {
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
    /// **本仕様の本番経路にこの読み口の呼び手は無い**（差し込みは
    /// [`TextLayerState::set_look_layers`]、戻し先と載せ直しの参照は `self.decor.layers` の
    /// 直読み）。それでも `pub` で残すのは、design.md「`state_decoration.rs`」の Contracts 表が
    /// 本関数を後続仕様の読み口として載せているためである（`disable.font.*` の配線を着地させた
    /// `areka-P0-balloon-font-descript-keys` は 2 層の実値を `ResolvedFont::resolve` の結果で
    /// 突き合わせたので、この読み口は使っていない）。
    /// 今日の呼び手は決定論テスト（`actor_decoration_tests.rs`／`state_decoration_tests.rs`／
    /// `state_decoration_reset_tests.rs`）だけである。
    pub fn look_layers(&self) -> &LookLayers {
        &self.decor.layers
    }

    /// 所有外キーの最新の引数列（後続仕様の消費者が読む・戻す操作で空になる・要件 2.5）。
    pub fn unowned_vocab(&self) -> &BTreeMap<String, Vec<String>> {
        &self.decor.unowned
    }

    /// 内容だけを消す（`\c`＝`Clear`・装飾状態は保つ・要件 3.7）。
    ///
    /// 番号列と装飾の表は内容と同じ寿命である——番号列はグリフ序数の写しなので内容が消えれば
    /// 空になり、表は「その内容が使っていた見た目」の入れ物なので同時に空にする。
    /// 消えないのは `decor`（現在の見た目・所有外キーの保持・記録済みの値）だけ。
    pub(super) fn clear_content(&mut self) {
        self.items.clear();
        self.reveal = super::RevealSchedule::default();
        self.choices.clear();
        self.glyph_styles.clear();
        self.styles.clear();
    }

    /// いま効いている見た目を表に登録し、追記する文字数だけ番号を並べる（要件 3.2／3.3）。
    ///
    /// design.md の `intern_current` と呼び手側の push を 1 つにまとめてある——追記点が
    /// 文字（`Text`）と選択肢（`Choice`）の 2 か所あり、同じ数行を写すと片方だけ
    /// 「0 文字なら表を汚さない」を落とすからである（不変条件
    /// 「`glyph_styles.len()` は `items` のグリフ数に等しい」の実装点はここ 1 つ）。
    /// 0 文字ガードを実際に踏むのは `Text` の腕（無条件に呼ぶ）で、`Choice` の腕は
    /// 上位で 0 文字を分岐するのでここへ来ない。
    ///
    /// `actor` は R6.3 の記録（上下付きが有効なまま文字が追記された）に使う——ここが
    /// 「文字が追記された」ことを知る唯一の地点だからである。
    pub(super) fn push_current_style(&mut self, actor: &ActorKey, glyph_count: usize) {
        if glyph_count == 0 {
            return;
        }
        self.warn_script_once(actor);
        let id = self
            .styles
            .intern(&self.decor.current, &self.decor.layers.default);
        self.glyph_styles
            .extend(std::iter::repeat_n(id, glyph_count));
    }

    /// 「戻す操作」の実体——見た目を既定へ丸ごと戻し、所有外キーの保持も空にする
    /// （要件 10.1／10.3／10.4）。既に追記済みの文字には効かない（要件 10.7）。
    pub(super) fn reset_look(&mut self) {
        self.reset_look_to(BaseLayer::Default);
    }

    /// 無効表示への一括の戻し（`\f[disable]`・要件 10.2）。
    ///
    /// **タスク 4.2 の裁定**——`\f[disable]` も `\f[default]` と同じ「一括の戻し」であり、
    /// 所有外キーの保持も空にする。要件 10.2 が「**全項目**を無効表示の見た目に合わせる」
    /// と述べ、要件 10.4 が戻す操作の対象を「装飾状態の全項目（後続仕様が登記する項目を
    /// 含む）」と定めている以上、後続仕様の項目である所有外キーだけが `\f[disable]` を
    /// 生き延びるのは「全項目」に反する。ゆえに 2 つの綴りの違いは**戻し先の層だけ**とする。
    /// この裁定の結果、`look.rs::apply_font_tag` の `key == "disable"` の腕は
    /// `key == "default"` の腕と同様に本番経路から到達しない（テストからのみ呼ばれる）。
    pub(super) fn reset_look_disabled(&mut self) {
        self.reset_look_to(BaseLayer::Disable);
    }

    /// 一括の戻しの共通実体——見た目を丸ごと置き換え、所有外キーの保持を空にする。
    ///
    /// 見た目は**丸ごと**置き換える（項目を列挙しない）ので、後続仕様が
    /// [`TextLook`] へ項目を足せば戻しへ自動で含まれる（要件 10.4）。
    /// 併せて土台の層を憶え、そこから後の `\f` の命令列を空にする——戻しの後は
    /// 「土台そのまま」が現在の見た目なので、装着の載せ直し（[`Decoration::rebase`]）は
    /// 新しい土台をそのまま採ればよい。
    fn reset_look_to(&mut self, base: BaseLayer) {
        self.decor.base = base;
        self.decor.applied.clear();
        let layer = self.decor.base_look();
        self.decor.current = layer;
        self.decor.unowned.clear();
    }

    /// 台本の先頭（`ClearAll`）の戻し——[`ActorTextState::reset_look`] に加えて記録済みの値も
    /// 空にする（次の台詞では同じ指定がもう 1 度記録される・要件 3.8／13.4）。
    pub(super) fn reset_for_new_talk(&mut self) {
        self.reset_look();
        self.decor.warned.clear();
    }

    /// `\f` 1 件（キャリアのトークン列）を適用し、必要な記録を残す（要件 2.5／2.6／3.2）。
    ///
    /// `tokens[0]` がキー、`tokens[1..]` が値の列。値の解釈は
    /// [`crate::look::apply_font_tag`] の担当で、本関数は「戻す操作へ回すか」「所有外キーを
    /// 保持するか」「何を記録するか」だけを決める。記録なしで失敗を飲み込む経路は無い
    /// （`Err` と記録すべき [`Note`] は必ず `warn!`・所有外キーは `debug!`）。
    pub(super) fn apply_font_args(&mut self, actor: &ActorKey, tokens: &[&str]) {
        // 一括の戻し（`\f[default]`）は「戻す操作」1 か所を通す（要件 10.3）——
        // 見た目だけを置き換える apply_font_tag の腕では所有外キーの保持が残ってしまう。
        // ゆえに `look.rs::apply_font_tag` の `key == "default"` の腕は本番経路から到達しない
        // （テストからのみ呼ばれる）。所有外キーの保持を戻しに含めるため、本番はこちらを
        // 通す（要件 10.4）。
        // `\f[disable]` も同じ一括の戻しで、違うのは戻し先の層だけ
        // （[`ActorTextState::reset_look_disabled`] の裁定・要件 10.2）。
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
            // 装着で 2 層が差し替わったとき新しい土台の上へ載せ直すため、命令を
            // **トークンのまま**憶える（[`Decoration::rebase`]・項目を列挙しない・要件 10.4）。
            // 失敗した指定は見た目を変えていないので憶えない（再生しても何も起きない）。
            self.decor
                .applied
                .push(tokens.iter().map(|token| (*token).to_owned()).collect());
        }
        match outcome {
            Ok(None) => {}
            // 記録すべき印は 4 種——catch-all を置かず、`Note` に腕が増えたときは
            // コンパイラに「これは保持か記録か」の再検討を強制する。
            Ok(Some(Note::Unowned)) => {
                // 要件 2.5: 値を捨てずに保持し、表示は変えず debug の記録を残す。
                // キーが空なら apply_font_tag は Err を返すので、ここでは必ずキーがある。
                let key = tokens[0].to_owned();
                tracing::debug!(actor = %actor, key = %key, "本仕様の所有外の \\f のキー——引数列を保持し表示は変えない（要件 2.5）");
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
                );
            }
            Ok(Some(Note::StylesheetKeyword)) => {
                self.warn_once(
                    actor,
                    "height",
                    &tokens[1..],
                    "スタイルシートの大きさの語は語彙のみ——大きさを変えない",
                );
            }
            Ok(Some(Note::AnchorColorAsDefault)) => {
                self.warn_once(
                    actor,
                    "color",
                    &tokens[1..],
                    "アンカーの色定義がまだ無い——default と同じ色を適用した",
                );
            }
            Err(issue) => {
                // 鍵はトークンの列から作る（`FontTagIssue::value` は連結済みの綴りなので、
                // それを鍵にすると `\f[bold,"x,y"]` と `\f[bold,x,y]` が同じ鍵へ潰れる）。
                // キーが空の失敗（要件 2.6）では `tokens` 自体が空になりうるので `get(1..)` で取る。
                let key = issue.key.clone();
                let rest = tokens.get(1..).unwrap_or(&[]);
                self.warn_once(actor, &key, rest, issue.reason);
            }
        }
    }

    /// 同じ（キー, 値の列）の警告を 1 台詞に 1 度だけ残す（要件 13.1／13.4）。
    ///
    /// 記録の本文には値の列をカンマで繋いだ読みやすい綴りを載せるが、**鍵は列のまま**である
    /// （要件 13.5——綴りを鍵にすると、引用符がカンマを守るせいで `\f[bold,"x,y"]` と
    /// `\f[bold,x,y]` が同じ鍵へ潰れ、片方が無記録で消える）。
    fn warn_once(&mut self, actor: &ActorKey, key: &str, values: &[&str], reason: &str) {
        let text = values.join(",");
        let owned: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
        if self
            .decor
            .warned
            .insert((RECORD_FONT_ARG, key.to_owned(), owned))
        {
            tracing::warn!(actor = %actor, key, value = text, reason, "\\f の指定を適用できない——当該項目は変えずに再生を続ける");
        }
    }

    /// 上下付きが有効なまま文字が追記されたことを 1 台詞に 1 度だけ残す（要件 6.3）。
    ///
    /// 記録済みの集合そのものは [`ActorTextState::warn_once`] と共有するが、**鍵空間は互いに
    /// 素**である——こちらの鍵は（[`RECORD_SCRIPT_APPEND`], キー（`sub`／`sup`）, 空の列）で、
    /// あちらは（[`RECORD_FONT_ARG`], キー, 値の列）。種別を鍵に含めるのは、キーの側が
    /// 台本作者の書いた任意の文字列だからである——種別を落とすと
    /// `\f[sub,追記]`（値が不正なので `warn_once` が記録する）が、直後の
    /// 追記の記録を無記録で飲み込む（逆順なら `warn_once` の側が消える・要件 13.5）。
    /// 文面を分けているのは、利用者が「指定が通らない」のか「指定は通ったが表示に出ない」のかを
    /// ログから見分けられるようにするためである。
    fn warn_script_once(&mut self, actor: &ActorKey) {
        let key = match self.decor.current.script {
            Script::None => return,
            Script::Sub => "sub",
            Script::Sup => "sup",
        };
        if self
            .decor
            .warned
            .insert((RECORD_SCRIPT_APPEND, key.to_owned(), Vec::new()))
        {
            tracing::warn!(actor = %actor, key, reason = "上下付きは語彙のみ——基線も大きさも送り幅も変えない", "上下付きが有効なまま文字を追記した——表示は変わらない");
        }
    }
}

impl TextLayerState {
    /// バルーンの装着で 2 層を差し込む（要件 4.1・結線層の 1 点から呼ぶ）。
    ///
    /// 2 層を差し替えたうえで、現在の見た目を新しい層の上へ**載せ直す**
    /// （[`Decoration::rebase`]）——土台は新しいバルーン定義の値になり、作者が `\f` で
    /// 明示した指定はそのまま残る。項目を 1 つも名指ししないので、後続仕様が
    /// [`TextLook`] へ足した項目も同じ規則で載る（要件 10.4）。
    ///
    /// # 装着より先に `\f` が届いても取りこぼさない
    ///
    /// cue のドレインは非同期で、`text_slot_view` が `None` の間は装着が次フレームへ委ねられる
    /// （`areka` の `emo2_boot::frame::attach::connect_balloon_text` の `None` の腕）。この窓で
    /// `\f[...]` が先に届いても、載せ直しが作者の命令を新しい既定の上で再生するので
    /// **バルーン定義の既定（大きさ・色・フォント名）はその台詞に届く**。台詞の流し始めを
    /// 1 フレーム遅らせて順序を作る必要は無い（開発者裁定 2026-09-13）。
    ///
    /// 再装着（k 再追従）でも同じ経路を通る——土台だけが新しい値へ移り、明示された指定は
    /// 保たれる。
    pub fn set_look_layers(&mut self, actor: &ActorKey, layers: LookLayers) {
        let state = self.actors.entry(actor.clone()).or_default();
        state.decor.layers = layers;
        state.decor.rebase();
    }

    /// 「戻す操作」（要件 10.3 の権威定義）——装飾状態の全項目を既定の見た目へ戻す。
    ///
    /// `scope` が `Some` なら当該スコープだけ、`None` なら全スコープを 1 回で戻す
    /// （要件 10.5）。`\f[default]`（[`ActorTextState::apply_font_args`]）と台詞の開始
    /// （`ClearAll`→[`ActorTextState::reset_for_new_talk`]）は同じ実体
    /// （[`ActorTextState::reset_look`]）を通り、別々の戻し方を持たない。後続仕様の
    /// クリック待ち（`\x`）もここを通す前提だが、**本番の呼び出し元はまだ無い**
    /// （引受先＝`areka-P0-balloon-lifecycle-events`）。
    /// 既に表示済みの文字の見た目は変わらない（要件 10.7）。
    pub fn reset_decoration(&mut self, scope: Option<&ActorKey>) {
        match scope {
            Some(actor) => {
                tracing::debug!(actor = %actor, "戻す操作——当該スコープの装飾状態を既定へ戻す（要件 10.3）");
                // まだ生まれていないスコープには戻すものが無いので、`entry().or_default()`
                // で器を作らない（`get_mut` で引く）。空の器を作っても `present_frame` の
                // 走査は「中身か供給面のどちらかがある」で弾くので提示層には載らないが、
                // 状態表に意味の無い項目を増やさない。
                if let Some(state) = self.actors.get_mut(actor) {
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

/// `\f` を運ぶ汎用キャリアか（名前による自己選別・要件 2.4）。
///
/// 名前が合わない運搬（`\!` の他コマンド）と非正準な params は `None` で、呼び手は従来どおり
/// 読み飛ばす。
pub(super) fn font_tag_tokens(command: &areka_sakura::contract::CueCommand) -> Option<Vec<&str>> {
    match command.as_command_carrier() {
        Some((name, tokens)) if name == FONT_TAG_CARRIER => Some(tokens),
        _ => None,
    }
}

#[cfg(test)]
#[path = "state_decoration_tests.rs"]
mod tests;
