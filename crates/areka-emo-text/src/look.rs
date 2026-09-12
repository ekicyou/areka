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

use crate::color::mix_disabled;

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
    ///   （`font.height` が 0 など）は [`crate::draw::ResolvedFont::resolve`] が `warn!` で
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

#[cfg(test)]
#[path = "look_tests.rs"]
mod tests;
