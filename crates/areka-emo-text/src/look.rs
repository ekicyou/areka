//! 1 文字に効く「見た目」の値と、その 2 層・装飾の表（純粋層・要件 4.1／4.3／4.5／4.7）。
//!
//! `\f[...]` の装飾は、純粋層を**文字ごとの番号**（[`StyleId`]）として流れ、COM 層が番号を
//! 見た目（[`TextLook`]）へ引き直して DirectWrite の範囲指定に写す（design.md
//! 「Components and Interfaces > emo-text 純粋層」）。本モジュールはその番号と見た目の
//! 定義点で、`windows` 系 crate に一切依存しない（描画なしで決定論テストできる）。
//!
//! ## 3 つの部品
//!
//! - [`TextLook`]——1 文字に効く見た目の全項目（候補列・大きさ・色・太字・斜体・下線・
//!   打ち消し線・白抜き・上下付き）。値型で、等値なら畳み込める。送り幅に効く項目だけを
//!   取り出した [`FontKey`] が計測と書式生成の鍵になる。
//! - [`LookLayers`]——既定・無効表示・選択肢文字色の 2 層＋1 色（要件 4.1／4.4／4.5）。
//!   無効表示は**色だけ**が混色で、他の項目は既定と同じ。
//! - [`StyleTable`]／[`StyleId`]／[`GlyphStyles`]——「既定と異なる見た目」だけを番号付きで
//!   持つ追記専用の表。番号 0（[`StyleId::DEFAULT`]）は「そのスコープの既定の見た目」を指す
//!   記号であって表の要素ではない（design.md D14）。既定だけの台本は番号がすべて 0 になり、
//!   COM 層は範囲指定を 1 度も呼ばない。
//!
//! ## 正典（ukadoc）
//!
//! 既定の見た目はバルーン定義から**いま読めている 5 キー**（`font.name`・`font.height`・
//! `font.color.r/g/b`）と正典の既定（ＭＳ ゴシック・12・黒・装飾なし）から組む（要件 4.1）。
//! `font.height` の単位は「ピクセル：ポイントではない」。
//! <https://ssp.shillest.net/ukadoc/manual/descript_balloon.html>
//!
//! 無効表示の層は正典の「`disable.font.color` のみバルーンの画像色とミックスした色、ほかは
//! `font.` 定義群と同じ」に従う（要件 4.5）。混色の式は 1 か所——[`crate::color::mix_disabled`]
//! だけが実装点で、本モジュールはそれを呼ぶ（式を写さない）。
//!
//! ## 残り 8 キーの「口」（要件 4.3／4.7）
//!
//! `font.bold`／`font.italic`／`font.underline`／`font.strike`／`font.outline`／
//! `font.shadowcolor.*`／`font.shadowstyle`／`disable.font.*` はバルーン定義側がまだ読めない
//! （読み取りの所有は `areka-P0-balloon-font-descript-keys`）。その口は
//! **[`TextLook`] の各フィールド**と **[`LookLayers::from_balloon`] の引数列**で、
//! 読めるようになったときは
//!
//! 1. `from_balloon` に引数を 1 つ足し、
//! 2. 組み立てている [`TextLook`] の当該フィールドへ入れる（`disable.font.*` は
//!    [`LookLayers::disable`] 側の当該フィールドを上書きする）
//!
//! だけで効く。層の組み方（無効表示は色だけ混色・他は既定と同じ）も、番号の畳み込みも、
//! 呼び手も変わらない。いま値が無い項目は正典の既定（すべて無効）のままで、
//! 「読めていないから既定」という状態を `look_tests.rs` §3 が明示的に固定している。

use crate::color::{ColorSpec, mix_disabled, parse_color};

/// 上下付き（語彙のみ・表示は変えない・後勝ちの排他なので enum で同時に 1 値）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Script {
    /// 上下付きなし（正典の既定）。
    #[default]
    None,
    /// 下付き（`\f[sub,...]`・語彙のみ）。
    Sub,
    /// 上付き（`\f[sup,...]`・語彙のみ）。
    Sup,
}

/// 正典の既定フォント名（**全角表記** ＭＳ ゴシック）。
///
/// COM 層の [`crate::draw::DEFAULT_FONT_NAME`] と同じ正典で、両者が黙って食い違わないことは
/// `look_tests.rs` §1 が見張る。
pub const UKADOC_DEFAULT_FONT_NAME: &str = "ＭＳ ゴシック";

/// 正典の既定フォント高さ 12（**image px**・「単位はピクセル：ポイントではない」）。
///
/// COM 層の [`crate::draw::DEFAULT_FONT_HEIGHT`] と同じ正典（`look_tests.rs` §1 が見張る）。
pub const UKADOC_DEFAULT_FONT_HEIGHT: f32 = 12.0;

/// 1 文字に効く見た目の全項目（値型・等値なら同じ番号へ畳み込まれる）。
///
/// 後続仕様（寄せ・影・選択肢マーカー・アンカー）はここへフィールドを足す。足した項目は
/// 「戻す操作」（見た目の丸ごと置換）に自動で含まれ、[`StyleTable`] の畳み込みにも自動で効く。
#[derive(Clone, Debug, PartialEq)]
pub struct TextLook {
    /// フォント名の候補列（記述順＝優先順）。実在の判定と読み飛ばしは COM 層の担当。
    pub name: Vec<String>,
    /// em の大きさ（image px・常に正の有限値）。
    pub height: f32,
    /// 文字色 r/g/b。
    pub color: (u8, u8, u8),
    /// 太字（DirectWrite の `SetFontWeight`）。
    pub bold: bool,
    /// 斜体（`SetFontStyle`）。
    pub italic: bool,
    /// 下線（`SetUnderline`）。
    pub underline: bool,
    /// 打ち消し線（`SetStrikethrough`）。
    pub strike: bool,
    /// 白抜き——語彙のみ（表示に効かない・M2 予約・要件 5.9）。
    pub outline: bool,
    /// 上下付き——語彙のみ（表示に効かない・要件 6.1）。
    pub script: Script,
}

impl TextLook {
    /// 正典の既定——ＭＳ ゴシック・12・黒・装飾はすべて無効（要件 4.1）。
    pub fn ukadoc_default() -> TextLook {
        TextLook {
            name: vec![UKADOC_DEFAULT_FONT_NAME.to_owned()],
            height: UKADOC_DEFAULT_FONT_HEIGHT,
            color: (0, 0, 0),
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            outline: false,
            script: Script::None,
        }
    }

    /// 計測と書式生成の鍵——**送り幅に効く項目だけ**（候補列・大きさ・太字・斜体）。
    ///
    /// 色・下線・打ち消し線・白抜き・上下付きは送り幅を動かさないので鍵に入れない。
    /// 入れてしまうと、幅の同じ文字が別の計測・別の書式・別の run へ無駄に割れる。
    pub fn font_key(&self) -> FontKey {
        FontKey {
            name: self.name.clone(),
            height_bits: self.height.to_bits(),
            bold: self.bold,
            italic: self.italic,
        }
    }
}

/// 計測と書式生成の鍵（[`TextLook::font_key`] の戻り値）。
///
/// 大きさは `f32` のビット表現で持つ（`Hash`・`Eq` を導けるようにするため。書式は同じ値なら
/// 同じ鍵、というだけが必要で、近い値どうしを同一視する必要はない）。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontKey {
    /// フォント名の候補列（記述順）。
    pub name: Vec<String>,
    /// em の大きさのビット表現（`f32::to_bits`）。
    pub height_bits: u32,
    /// 太字。
    pub bold: bool,
    /// 斜体。
    pub italic: bool,
}

/// 既定・無効表示・選択肢文字色（要件 4.1／4.4／4.5／4.7）。
///
/// `\f[default]`／`\f[キー,default]` の戻し先が [`LookLayers::default`]、
/// `\f[disable]`／`\f[キー,disable]` の戻し先が [`LookLayers::disable`]、
/// `\f[color,default.cursor]`／`default.cursornotselect` が引く色が
/// [`LookLayers::cursor_text`]。
#[derive(Clone, Debug, PartialEq)]
pub struct LookLayers {
    /// 既定の見た目（バルーン定義＋正典の既定）。
    pub default: TextLook,
    /// 無効表示の見た目——**色だけ**が混色で、他の項目は [`LookLayers::default`] と同じ。
    pub disable: TextLook,
    /// 選択肢の既定文字色（バルーン定義の選択肢文字色）。
    pub cursor_text: (u8, u8, u8),
}

impl LookLayers {
    /// バルーン定義から読めている値と正典の既定で 2 層を組む（要件 4.1／4.5）。
    ///
    /// - `name_candidates` が空なら正典の既定名（[`UKADOC_DEFAULT_FONT_NAME`]）。
    /// - `height` が正の有限値でなければ正典の既定 12（下流の DirectWrite は em に正値を
    ///   要求するので、2 層は「常に正の大きさ」を約束する）。バルーン定義の側の縮退
    ///   （`font.height` が 0 など）は [`crate::draw::ResolvedFont::resolve_with_background`]
    ///   （[`resolve`](crate::draw::ResolvedFont::resolve) の実装本体）が `warn!` で
    ///   記録済みなので、ここでは二重に記録しない。
    /// - `background` はバルーンの背景色で、無効表示の色を導くためだけに使う
    ///   （`disable.color = mix_disabled(default.color, background)`・要件 4.6）。
    ///
    /// 残り 8 キーはこの引数列へ足すだけで効く（モジュールの説明「残り 8 キーの口」）。
    pub fn from_balloon(
        name_candidates: Vec<String>,
        height: f32,
        color: (u8, u8, u8),
        background: (u8, u8, u8),
        cursor_text: (u8, u8, u8),
    ) -> LookLayers {
        let canon = TextLook::ukadoc_default();
        let default = TextLook {
            name: if name_candidates.is_empty() {
                canon.name
            } else {
                name_candidates
            },
            height: if height.is_finite() && height > 0.0 {
                height
            } else {
                canon.height
            },
            color,
            ..canon
        };
        // 無効表示は色だけが違う——項目を列挙せず既定層から複製するので、
        // 後続仕様が TextLook へ項目を足しても「色以外は既定と同じ」が自動で保たれる。
        let disable = TextLook {
            color: mix_disabled(default.color, background),
            ..default.clone()
        };
        LookLayers {
            default,
            disable,
            cursor_text,
        }
    }
}

impl Default for LookLayers {
    /// バルーン定義が無いときの 2 層——正典の既定・背景は白・選択肢文字色は黒。
    fn default() -> LookLayers {
        LookLayers::from_balloon(
            Vec::new(),
            UKADOC_DEFAULT_FONT_HEIGHT,
            (0, 0, 0),
            (255, 255, 255),
            (0, 0, 0),
        )
    }
}

/// 装飾の番号（0 は「そのスコープの既定の見た目」を指す記号）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StyleId(pub u32);

impl StyleId {
    /// そのスコープの既定の見た目（表の要素ではない・design.md D14）。
    pub const DEFAULT: StyleId = StyleId(0);
}

/// 「既定と異なる見た目」だけの追記専用の表（同値は同じ番号へ畳み込む）。
///
/// 番号 `n`（1 以上）は `looks[n - 1]`。既存の番号の意味は追記で変わらない
/// （配置済みのグリフが持つ番号が後から別の見た目を指すことは無い）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleTable {
    looks: Vec<TextLook>,
}

impl StyleTable {
    /// 見た目を番号へ——既定と同値なら [`StyleId::DEFAULT`]、既存と同値ならその番号、
    /// どちらでもなければ末尾へ追記してその番号。
    ///
    /// 引き当ては先頭からの線形走査。1 スコープの表は「1 台詞の中で既定と異なる見た目」の
    /// 種類数（実際には数個）なので、これで足りる。
    pub fn intern(&mut self, look: &TextLook, default: &TextLook) -> StyleId {
        if look == default {
            return StyleId::DEFAULT;
        }
        if let Some(index) = self.looks.iter().position(|known| known == look) {
            return StyleId(index as u32 + 1);
        }
        self.looks.push(look.clone());
        StyleId(self.looks.len() as u32)
    }

    /// 番号を見た目へ——[`StyleId::DEFAULT`] と範囲外の番号は `default` に落ちる
    /// （表を消したあとに古い番号が残っていても、別の見た目には決してならない）。
    pub fn resolve<'a>(&'a self, id: StyleId, default: &'a TextLook) -> &'a TextLook {
        match id.0.checked_sub(1) {
            None => default,
            Some(index) => self.looks.get(index as usize).unwrap_or(default),
        }
    }

    /// 表に積まれた見た目の数（既定は含まない）。
    pub fn len(&self) -> usize {
        self.looks.len()
    }

    /// 表が空か（既定しか使われていないか）。
    pub fn is_empty(&self) -> bool {
        self.looks.is_empty()
    }

    /// 表を空にする（スコープの消去点で呼ぶ）。
    pub fn clear(&mut self) {
        self.looks.clear();
    }
}

/// 配置層へ渡す「グリフ序数→見た目」の読み口（表・番号列・既定・いま効いている見た目）。
///
/// `current` は「次に置く文字が無い」末尾の行の高さに使う（そのとき効いている大きさ）。
#[derive(Clone, Copy)]
pub struct GlyphStyles<'a> {
    /// 装飾の表。
    pub table: &'a StyleTable,
    /// グリフ序数と同じ序数空間の番号列。
    pub ids: &'a [StyleId],
    /// そのスコープの既定の見た目。
    pub default: &'a TextLook,
    /// いま効いている見た目（まだ文字が置かれていない末尾の行のため）。
    pub current: &'a TextLook,
}

impl<'a> GlyphStyles<'a> {
    /// 序数の番号（番号列の外は [`StyleId::DEFAULT`]）。
    pub fn id_of(&self, ordinal: usize) -> StyleId {
        self.ids.get(ordinal).copied().unwrap_or(StyleId::DEFAULT)
    }

    /// 序数の見た目（[`GlyphStyles::id_of`] を表で引いたもの）。
    pub fn look_of(&self, ordinal: usize) -> &'a TextLook {
        self.table.resolve(self.id_of(ordinal), self.default)
    }
}

// ------------------------------------------------------------- `\f` の値の状態機械

/// `\f` を 1 件適用した結果のうち、呼び手が記録に残すべき印（記録そのものは呼び手の担当）。
///
/// 純粋層は `warn!`／`debug!` を出さない——同じキーと値の繰り返しを「1 台詞に 1 度」へ
/// まとめるのは、台詞の区切りを知っている呼び手（`state_decoration`）の役目だからである
/// （design.md「Error Handling」）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    /// 状態は更新したが**表示は変えない**——`sub`／`sup`（要件 6.1）と `outline`（要件 5.9）。
    /// 呼び手は `warn!` を 1 台詞に 1 度残す。
    VocabularyOnly {
        /// どの項目か（`"sub"`／`"sup"`／`"outline"`）。
        key: &'static str,
    },
    /// スタイルシートの大きさキーワード（`xx-small`〜`xx-large`・`larger`・`smaller`）——
    /// 語彙として受理するが**大きさは変えない**（要件 7.7・design §A 項目 3）。
    /// 呼び手は `warn!` を 1 台詞に 1 度残す。
    StylesheetKeyword,
    /// `\f[color,default.anchor]`／`default.anchornotselect`／`default.anchorvisited`——
    /// アンカーの色定義がまだ無いので **`default` と同じ色**を適用した（要件 8.8・
    /// design §A 項目 11）。呼び手は `warn!` を 1 台詞に 1 度残す。消費者（本物のアンカー色）は
    /// `areka-P0-anchor-tag-canon` がこの腕を差し替えて足す。
    AnchorColorAsDefault,
    /// 本仕様が意味を与えないキー——見た目を変えない（要件 2.5）。値を捨てずに保つのと
    /// `debug!` の記録は呼び手の担当。
    Unowned,
}

/// 適用できなかった `\f` 1 件（当該項目は変わらない・呼び手が `warn!` に載せる材料）。
///
/// 呼び手は `warn!(actor, key, value, reason)` の構造化フィールドへそのまま載せる
/// （design.md「Error Handling」の「キー＝値ごとに 1 台詞 1 度」）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontTagIssue {
    /// 受け取ったキー（キーが無い形では空文字列）。
    pub key: String,
    /// 受け取った値の列をカンマで繋いだもの（値が無ければ空文字列）。
    pub value: String,
    /// 何が受け付けられなかったか（記録用の短い日本語）。
    pub reason: &'static str,
}

/// `\f`／`\f[]`／`\f[""]`——キーが無い（要件 2.6）。
pub(crate) const REASON_NO_KEY: &str = "\\f にキーが無い";
/// 43 形のいずれでもないキー（要件 2.6）。キーも小文字の完全一致のみ。
pub(crate) const REASON_UNKNOWN_KEY: &str = "未知の \\f のキー（キーは小文字のみ）";
/// 値がちょうど 1 つでない——値なし（`\f[bold]`・`\f[height]`）と値が 2 つ以上
/// （`\f[bold,1,0]`・`\f[height,15,20]`）の両方（要件 5.5・7.6）。
pub(crate) const REASON_VALUE_COUNT: &str = "値がちょうど 1 つでない";
/// 6 語のいずれでもない値（要件 5.5）。語は小文字の完全一致のみ。
pub(crate) const REASON_BAD_SWITCH: &str = "真偽値が 6 語のいずれでもない（語は小文字のみ）";
/// 大きさの値が数値・相対（`+N`／`-N`）・百分率・`default`／`disable`・スタイルシートの語の
/// いずれとしても読めない（要件 7.6 の「非数」）。
pub(crate) const REASON_BAD_HEIGHT: &str = "大きさの値が読めない";
/// 大きさの結果が正の有限値にならない——`0` 以下・非有限・相対指定で 0 以下へ落ちた場合
/// （要件 7.6）。
pub(crate) const REASON_HEIGHT_NOT_POSITIVE: &str = "大きさが正の有限値にならない";
/// `\f[name]` に候補が 1 つも無い（値なし、または候補がすべて空文字列・要件 9.1）。
pub(crate) const REASON_NO_CANDIDATE: &str = "フォント名の候補が無い";

/// スタイルシートの大きさキーワード——**語彙として受理するだけ**で大きさを変えない
/// （要件 7.7・design §A 項目 3）。
///
/// CSS の絶対サイズ 7 語（`xx-small`〜`xx-large`）と相対サイズ 2 語（`larger`／`smaller`）。
/// ukadoc は「スタイルシートのサイズ指定も可能」としか述べず、どの語が何ピクセルになるかを
/// 定めないので、areka は換算そのものを見送る。語は 6 語の規律と同じ**小文字の完全一致のみ**。
pub(crate) const STYLESHEET_SIZE_KEYWORDS: [&str; 9] = [
    "xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "larger", "smaller",
];

/// 真偽値で切り替える項目が受ける 6 語（**小文字の完全一致のみ**）。
///
/// ukadoc は「パラメータに true または 1 を指定すると太字」「false または 0 を指定すると無効」
/// 「default を指定するとバルーン設定の標準に戻る」「disable を指定すると無効表示と同じ設定に
/// なる」と定める。大文字を畳み込んで受けることは正典が定めておらず、areka は
/// `doc/COMPAT_ARCHITECTURE.md` §8 の「小文字の完全一致のみ」の先例に揃える（design §A 項目 12）。
/// <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Switch {
    /// `true`／`1`。
    On,
    /// `false`／`0`。
    Off,
    /// `default`——当該項目だけを既定層の値へ。
    Default,
    /// `disable`——当該項目だけを無効表示層の値へ。
    Disable,
}

fn parse_switch(value: &str) -> Option<Switch> {
    match value {
        "true" | "1" => Some(Switch::On),
        "false" | "0" => Some(Switch::Off),
        "default" => Some(Switch::Default),
        "disable" => Some(Switch::Disable),
        _ => None,
    }
}

/// 真偽 5 項目の「どのフィールドか」を指す関数（読みも書きもこの 1 つを通す）。
type SwitchSlot = fn(&mut TextLook) -> &mut bool;

/// キー → 項目。**この 1 か所だけ**がキーと項目の対応を持つ（読み用と書き用に分けると
/// 並びが 2 か所へ散り、片方だけ取り違えても気づけない）。
fn switch_slot(key: &str) -> Option<SwitchSlot> {
    let slot: SwitchSlot = match key {
        "bold" => |look| &mut look.bold,
        "italic" => |look| &mut look.italic,
        "underline" => |look| &mut look.underline,
        "strike" => |look| &mut look.strike,
        "outline" => |look| &mut look.outline,
        _ => return None,
    };
    Some(slot)
}

/// キー → 上下付きの値（`sub`／`sup`）。
fn script_of_key(key: &str) -> Option<Script> {
    match key {
        "sub" => Some(Script::Sub),
        "sup" => Some(Script::Sup),
        _ => None,
    }
}

/// 本仕様が意味を与えないキーのうち、接頭辞では拾えないもの（寄せ 2・影 2）。
const UNOWNED_KEYS: [&str; 4] = ["align", "valign", "shadowcolor", "shadowstyle"];

/// 所有外のキーか（要件 2.5 の列挙——寄せ 2・影 2・`cursor*` 10・`anchor*` 17）。
///
/// 所有仕様は `areka-P0-text-align-shadow-canon`（寄せ・影）・
/// `areka-P0-choice-marker-styling`（`cursor*`）・`areka-P0-anchor-tag-canon`
/// （`anchor*`・`anchor.font.color` を含む）。
fn is_unowned(key: &str) -> bool {
    UNOWNED_KEYS.contains(&key) || key.starts_with("cursor") || key.starts_with("anchor")
}

fn issue(key: &str, values: &[&str], reason: &'static str) -> FontTagIssue {
    FontTagIssue {
        key: key.to_owned(),
        value: values.join(","),
        reason,
    }
}

/// 値をちょうど 1 つ取り出す（値なしと 2 つ以上はどちらも当該項目を変えない・要件 5.5・7.6）。
fn single_value<'a>(key: &str, values: &[&'a str]) -> Result<&'a str, FontTagIssue> {
    match values {
        [one] => Ok(one),
        _ => Err(issue(key, values, REASON_VALUE_COUNT)),
    }
}

/// `\f` 1 件を「いま効いている見た目」へ適用する（全入力で値を返す・panic しない）。
///
/// `args[0]` がキー、`args[1..]` が値の列（記述順）。`Ok(None)` は「適用済み・記録は要らない」、
/// `Ok(Some(note))` は「適用したが呼び手が記録すべきことがある」、`Err` は「**当該項目を
/// 変えなかった**・呼び手が理由を記録する」を表す。
///
/// # 持っている腕
///
/// - 一括の戻し `\f[default]`／`\f[disable]`——見た目を**丸ごと置き換える**（要件 10.1／10.2）。
///   項目を列挙しないので、後続仕様が [`TextLook`] へ項目を足せば自動で戻しに含まれる（要件 10.4）。
/// - 真偽 5 項目 `bold`／`italic`／`underline`／`strike`／`outline`——6 語で当該項目だけを
///   動かす（要件 5.1〜5.4・5.8）。`outline` は状態だけ更新して表示は変えない（要件 5.9）。
/// - 上下付き `sub`／`sup`——同じ 6 語で解釈し、後から指定した方が先を外す（要件 6.1／6.2）。
/// - 値を持つ 3 キー——`height` は絶対・相対（そのとき効いている大きさが基準）・百分率
///   （既定の大きさが基準）・層への戻し・スタイルシートの語（語彙のみ）を受ける
///   （要件 7.1〜7.7）。`color` は書式の解析を [`crate::color::parse_color`] に委ね、
///   語彙をどの層へ解決するかだけを持つ（要件 8.5〜8.8）。`name` は候補列を記述順のまま
///   保つ（要件 9.1／9.2／9.5／9.6）。いずれも**当該項目以外は触らない**。
/// - 所有外のキー——見た目を変えず [`Note::Unowned`]（要件 2.5）。
///
/// # 事前条件・事後条件
///
/// - `layers.default.height` は正の有限値（[`LookLayers::from_balloon`] が保証）。
/// - `Err` のとき `current` は不変。
pub fn apply_font_tag(
    current: &mut TextLook,
    layers: &LookLayers,
    args: &[&str],
) -> Result<Option<Note>, FontTagIssue> {
    let Some(key) = args.first().copied().filter(|key| !key.is_empty()) else {
        return Err(issue("", &[], REASON_NO_KEY));
    };
    let values = &args[1..];

    // 一括の戻し——見た目を丸ごと置き換える（項目を列挙しない・要件 10.4）。
    // 正典の綴りは値を取らない `\f[default]`／`\f[disable]` で、意味はキーだけで決まる。
    // 余った値（`\f[default,1]` のような綴り誤り）は戻しを妨げない。
    if key == "default" {
        *current = layers.default.clone();
        return Ok(None);
    }
    if key == "disable" {
        *current = layers.disable.clone();
        return Ok(None);
    }
    if let Some(slot) = switch_slot(key) {
        return apply_switch(current, layers, key, values, slot);
    }
    if let Some(want) = script_of_key(key) {
        return apply_script(current, layers, key, values, want);
    }
    match key {
        "height" => return apply_height(current, layers, key, values),
        "color" => return apply_color(current, layers, key, values),
        "name" => return apply_name(current, layers, key, values),
        _ => {}
    }
    if is_unowned(key) {
        return Ok(Some(Note::Unowned));
    }
    Err(issue(key, values, REASON_UNKNOWN_KEY))
}

/// 真偽 5 項目の 1 つを 6 語で動かす（当該項目以外は触らない・要件 5.1〜5.5）。
fn apply_switch(
    current: &mut TextLook,
    layers: &LookLayers,
    key: &str,
    values: &[&str],
    slot: SwitchSlot,
) -> Result<Option<Note>, FontTagIssue> {
    let word = single_value(key, values)?;
    let next = match parse_switch(word).ok_or_else(|| issue(key, values, REASON_BAD_SWITCH))? {
        Switch::On => true,
        Switch::Off => false,
        // 層の値も同じ `slot` を通して読む（キーと項目の対応が 1 か所に留まる）。
        Switch::Default => {
            let mut probe = layers.default.clone();
            *slot(&mut probe)
        }
        Switch::Disable => {
            let mut probe = layers.disable.clone();
            *slot(&mut probe)
        }
    };
    *slot(current) = next;
    // 白抜きは状態だけ更新して表示は変えない（要件 5.9）。
    Ok((key == "outline").then_some(Note::VocabularyOnly { key: "outline" }))
}

/// 上下付きを 6 語で動かす（要件 6.1／6.2）。
///
/// 上下付きは 1 つの値（[`Script`]）なので、`sub` を立てれば `sup` は自動的に外れる
/// ——後から指定した方が残る（要件 6.2）。外す側は「いま効いているのが当該の方のとき
/// だけ」外す（`\f[sub,0]` が上付きを消してしまわない）。
fn apply_script(
    current: &mut TextLook,
    layers: &LookLayers,
    key: &str,
    values: &[&str],
    want: Script,
) -> Result<Option<Note>, FontTagIssue> {
    let word = single_value(key, values)?;
    let on = match parse_switch(word).ok_or_else(|| issue(key, values, REASON_BAD_SWITCH))? {
        Switch::On => true,
        Switch::Off => false,
        Switch::Default => layers.default.script == want,
        Switch::Disable => layers.disable.script == want,
    };
    if on {
        current.script = want;
    } else if current.script == want {
        current.script = Script::None;
    }
    Ok(Some(Note::VocabularyOnly {
        key: match want {
            Script::Sub => "sub",
            Script::Sup => "sup",
            // `script_of_key` が選り分けるので `None` はここへ来ない。
            // ワイルドカードにせず全腕を書き、`Script` に値が増えたら型検査で気づけるようにする。
            Script::None => "sup",
        },
    }))
}

/// `\f[height,…]` の値の読み——**大きさをどう決めるか**の 6 通り（要件 7.1〜7.5・7.7）。
///
/// 「基準は何か」を型で分けておくのが肝で、[`HeightSpec::Relative`] は**そのとき効いている
/// 大きさ**、[`HeightSpec::Percent`] は**既定の見た目の大きさ**という別の基準を持つ
/// （design §A 項目 1・2）。両者を同じ「数値」に潰すと基準の取り違えが型に映らない。
enum HeightSpec {
    /// `N`——em を image px で直に指定（要件 7.1）。
    Absolute(f32),
    /// `+N`／`-N`——そのとき効いている大きさへの加減（符号込みの増分・要件 7.2）。
    Relative(f32),
    /// `N%`——既定の見た目の大きさに対する百分率（要件 7.3）。
    Percent(f32),
    /// `default`——大きさだけを既定層へ（要件 7.4）。
    Default,
    /// `disable`——大きさだけを無効表示層へ（要件 7.5）。
    Disable,
    /// スタイルシートの大きさキーワード——語彙のみ（大きさを変えない・要件 7.7）。
    Keyword,
}

/// 大きさの値 1 つを [`HeightSpec`] へ（読めなければ `None`）。
///
/// 見る順に意味がある——`default`／`disable`／スタイルシートの語を先に選り分けてから
/// 書式を見る。百分率（末尾 `%`）と相対（先頭の符号）は数値より先で、残りが絶対指定。
/// 語はいずれも**小文字の完全一致のみ**（6 語の規律と同じ・design §A 項目 12）。
fn parse_height(value: &str) -> Option<HeightSpec> {
    match value {
        "default" => return Some(HeightSpec::Default),
        "disable" => return Some(HeightSpec::Disable),
        _ => {}
    }
    if STYLESHEET_SIZE_KEYWORDS.contains(&value) {
        return Some(HeightSpec::Keyword);
    }
    if let Some(body) = value.strip_suffix('%') {
        return body.parse().ok().map(HeightSpec::Percent);
    }
    if value.starts_with('+') || value.starts_with('-') {
        return value.parse().ok().map(HeightSpec::Relative);
    }
    value.parse().ok().map(HeightSpec::Absolute)
}

/// `\f[height,…]`——大きさだけを動かす（要件 7.1〜7.7）。
///
/// 結果が正の有限値にならなければ**大きさを変えず**理由を返す（要件 7.6）。この検査は
/// 6 通りすべての出口に 1 つだけ置く——`0` の直指定も、相対指定で 0 以下へ落ちた場合も、
/// `inf`／`NaN` も、同じ 1 か所で止まる。
fn apply_height(
    current: &mut TextLook,
    layers: &LookLayers,
    key: &str,
    values: &[&str],
) -> Result<Option<Note>, FontTagIssue> {
    let value = single_value(key, values)?;
    let spec = parse_height(value).ok_or_else(|| issue(key, values, REASON_BAD_HEIGHT))?;
    let next = match spec {
        HeightSpec::Absolute(size) => size,
        // 基準は「そのとき効いている大きさ」——重ねて効く（design §A 項目 1）。
        HeightSpec::Relative(delta) => current.height + delta,
        // 基準は「既定の見た目の大きさ」——重ねて効かない（design §A 項目 2）。
        HeightSpec::Percent(percent) => layers.default.height * percent / 100.0,
        HeightSpec::Default => layers.default.height,
        HeightSpec::Disable => layers.disable.height,
        // 語彙として受理するだけ——大きさは変えない（呼び手が warn を 1 度残す・要件 7.7）。
        HeightSpec::Keyword => return Ok(Some(Note::StylesheetKeyword)),
    };
    if !next.is_finite() || next <= 0.0 {
        return Err(issue(key, values, REASON_HEIGHT_NOT_POSITIVE));
    }
    current.height = next;
    Ok(None)
}

/// `\f[color,…]`——文字色だけを動かす（要件 8.1〜8.9）。
///
/// 書式の解析は [`crate::color::parse_color`] の唯一の担当（要件 8.10）で、ここが持つのは
/// **語彙をどの層へ解決するか**だけ。アンカー 3 語はアンカーの色定義がまだ無いので既定層へ
/// 落とし、呼び手が warn を残せるよう印を返す（要件 8.8・design §A 項目 11）。
fn apply_color(
    current: &mut TextLook,
    layers: &LookLayers,
    key: &str,
    values: &[&str],
) -> Result<Option<Note>, FontTagIssue> {
    let spec = parse_color(values).map_err(|error| issue(key, values, error.reason))?;
    let (color, note) = match spec {
        ColorSpec::Rgb(r, g, b) => ((r, g, b), None),
        ColorSpec::Default | ColorSpec::DefaultPlain => (layers.default.color, None),
        ColorSpec::Disable => (layers.disable.color, None),
        ColorSpec::DefaultCursor | ColorSpec::DefaultCursorNotSelect => (layers.cursor_text, None),
        ColorSpec::DefaultAnchor
        | ColorSpec::DefaultAnchorNotSelect
        | ColorSpec::DefaultAnchorVisited => {
            (layers.default.color, Some(Note::AnchorColorAsDefault))
        }
    };
    current.color = color;
    Ok(note)
}

/// `\f[name,…]`——フォント名の候補列だけを動かす（要件 9.1／9.2／9.5／9.6）。
///
/// 候補列は**記述順のまま**保つ。実在の判定・フォントファイル名の読み飛ばし（要件 9.3）・
/// 全滅時の既定への差し戻し（要件 9.4）はいずれも描画層（`FontCatalog`）の担当で、
/// 純粋層はここで候補を落としたり並べ替えたりしない。
fn apply_name(
    current: &mut TextLook,
    layers: &LookLayers,
    key: &str,
    values: &[&str],
) -> Result<Option<Note>, FontTagIssue> {
    // 候補が 1 つも無い（値なし・空文字列だけ）——[`TextLook::name`] は空にしない。
    // 途中の空トークン（`[name,a,,b]`）は落とさず候補列に残す: 解読層が空を潰さずに
    // 渡してくる（要件 2.2）ので、この層でも記述順を変えない。実在しない候補として
    // 描画層の [`FontCatalog`] が読み飛ばす（要件 9.1・9.2）。
    if values.iter().all(|candidate| candidate.is_empty()) {
        return Err(issue(key, values, REASON_NO_CANDIDATE));
    }
    current.name = match values {
        ["default"] => layers.default.name.clone(),
        ["disable"] => layers.disable.name.clone(),
        candidates => candidates
            .iter()
            .map(|candidate| (*candidate).to_owned())
            .collect(),
    };
    Ok(None)
}

#[cfg(test)]
#[path = "look_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "look_font_tag_tests.rs"]
mod font_tag_tests;

#[cfg(test)]
#[path = "look_font_tag_value_tests.rs"]
mod font_tag_value_tests;
