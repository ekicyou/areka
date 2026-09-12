//! 通信に使う文字コード（純粋・決定的・`windows` 非依存）。
//!
//! 本モジュールは「任意の文字コード」を列挙せずに表す [`Charset`] を提供する。
//! 対応する集合はファイル層（`areka-parsers` の `charset`）と同一——どちらも
//! `encoding_rs`（WHATWG Encoding Standard の全ラベル）へ委ねるため、通信層だけの
//! 文字コード一覧を持たない（要件 1.1）。ラベルの寛容な解釈（前後空白・大小文字・
//! 別名）も `encoding_rs::Encoding::for_label` の寛容さがそのまま規則になる（要件 1.2）。
//!
//! ## 設計原則
//! - **文字コードで分岐しない**: 既定が Shift_JIS であることを理由に Shift_JIS と
//!   UTF-8 を特別扱いしない（要件 1.4）。本ファイルに現れる固有名は定数 2 つだけで、
//!   符号化・復号・正規名はいずれも同じ 1 本の経路を通る。
//! - **不変条件**: 「符号化の出力が自身と一致する」（`output_encoding() == self`）。
//!   構築経路は定数 2 つと [`Charset::for_label`] のみで、不変条件が破れる値は作れない。
//!   これにより `Charset` ヘッダに書く綴りと実際に符号化した文字コードが常に一致する
//!   （要件 3.2・10.3）。
//! - **panic しない**: 任意のラベル・任意の文字列・任意のバイト列を受け、失敗は
//!   戻り値（`Result` と置換の事実）で表す（要件 7.3）。

use std::borrow::Cow;

/// 通信に使う文字コード（不変条件: 符号化の出力が自身と一致する）。
///
/// `encoding_rs` の静的 `Encoding` への参照の newtype。値同士の同値性は `Encoding` の
/// 同一性なので、同じ文字コードの別名はすべて同じ値へ解決する（要件 1.2）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Charset(&'static encoding_rs::Encoding);

/// ラベルを採用できない理由（ログの `reason` フィールドへ写す・要件 10.3）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LabelError {
    /// Encoding Standard に無いラベル。
    Unknown,
    /// ラベルは既知だが符号化の出力が一致しない（UTF-16 系・replacement 系）
    /// ＝通信では使えない既知の限界（要件 1.5）。綴りとバイト列が食い違う要求を
    /// 決して送らないため、解決できたとは扱わない。
    NotEncodable,
}

impl Charset {
    /// UTF-8。
    pub const UTF_8: Charset = Charset(&encoding_rs::UTF_8_INIT);
    /// Shift_JIS（本番の既定＝ANSI の固定写像先。既定であって対応範囲ではない・要件 1.4）。
    pub const SHIFT_JIS: Charset = Charset(&encoding_rs::SHIFT_JIS_INIT);

    /// ラベルを解決する（前後空白・大小文字・別名に寛容・要件 1.2）。
    ///
    /// ラベル表は持たず `encoding_rs::Encoding::for_label` に委ねる（要件 1.1）。
    /// 符号化の出力が自身と一致しない文字コード（UTF-16LE／UTF-16BE／replacement）は
    /// [`LabelError::NotEncodable`] として退け、Encoding Standard に無いラベル
    /// （[`LabelError::Unknown`]）と理由で区別する（要件 1.5・10.3）。
    pub fn for_label(label: &str) -> Result<Charset, LabelError> {
        let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) else {
            return Err(LabelError::Unknown);
        };
        if encoding.output_encoding() != encoding {
            return Err(LabelError::NotEncodable);
        }
        Ok(Charset(encoding))
    }

    /// `Charset` ヘッダに書く正規名（`UTF-8`／`Shift_JIS`／`EUC-JP`／`ISO-2022-JP` 等）。
    ///
    /// 実際に符号化する文字コードと常に一致する（不変条件・要件 3.2）。
    #[must_use]
    pub fn name(self) -> &'static str {
        self.0.name()
    }

    /// 符号化する。表せない文字は 10 進の数値文字参照 `&#NNNN;` へ置換し、
    /// 置換した文字数を返す（要件 10.1・3.5）。
    ///
    /// UTF-8 では入力のバイト列がそのまま借用で返る（複製 0・差分 0 バイト・要件 3.4）。
    /// 置換数は `encoding_rs` が「置換の有無」しか返さないため、**置換があったときだけ**
    /// 文字ごとに数える（通常経路に追加コストを掛けない）。
    #[must_use]
    pub fn encode(self, text: &str) -> (Cow<'_, [u8]>, usize) {
        let (bytes, _actual, had_replacements) = self.0.encode(text);
        let replaced = if had_replacements {
            text.chars()
                .filter(|c| {
                    let mut buf = [0u8; 4];
                    self.0.encode(c.encode_utf8(&mut buf)).2
                })
                .count()
        } else {
            0
        };
        (bytes, replaced)
    }

    /// 復号する（BOM 判定なし）。不正な並びは代替文字 U+FFFD へ吸収し、置換の有無を返す
    /// （要件 10.2）。
    ///
    /// 先頭の BOM で文字コードを切り替えない（宣言と食い違う BOM で交渉結果を黙って
    /// 覆さないため）。UTF-8 の妥当な入力では `str::from_utf8` と同じ文字列になる。
    #[must_use]
    pub fn decode(self, bytes: &[u8]) -> (Cow<'_, str>, bool) {
        self.0.decode_without_bom_handling(bytes)
    }
}

#[cfg(test)]
#[path = "charset_tests.rs"]
mod charset_tests;
