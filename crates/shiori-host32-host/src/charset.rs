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

impl LabelError {
    /// ログの `reason` フィールドへ写す綴り。
    ///
    /// 「Encoding Standard に無い」（`unknown`）と「通信では使えない既知の限界」
    /// （`not_encodable`）を根拠フィールドで区別する（要件 10.3）。
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            LabelError::Unknown => "unknown",
            LabelError::NotEncodable => "not_encodable",
        }
    }
}

/// 交渉のログの宛先。
///
/// `RUST_LOG` は **target 名**で指定する（`shiori-charset=debug`。モジュールパス名では
/// 点かない）。
const LOG_TARGET: &str = "shiori-charset";

/// 初出なら警告、以後は開発者向け詳細ログ（要件 7.4——毎秒のイベントで同じ後退が
/// 繰り返されても警告は 1 回）。
macro_rules! warn_then_debug {
    ($first:expr, $($fields:tt)*) => {
        if $first {
            tracing::warn!(target: LOG_TARGET, $($fields)*);
        } else {
            tracing::debug!(target: LOG_TARGET, $($fields)*);
        }
    };
}

/// 応答の復号方針（codec へ渡す 2 値・要件 4.5／5.4）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CharsetPolicy {
    /// 応答の `Charset` ヘッダが解決できればそれで復号し、できなければ中の文字コードで復号する。
    Negotiate(Charset),
    /// 応答の `Charset` ヘッダを見ずに中の文字コードで復号する（強制・in-proc）。
    Force(Charset),
}

/// 1 セッション（SHIORI の load から unload まで）の交渉状態と採用規則（要件 5.1）。
///
/// 窓も入出力も持たず、副作用は `tracing` のログだけ。交渉規則の唯一の置き場で、
/// 上位（kanade）は状態を持つだけで規則を書かない。
///
/// 起動時のクロージャ（`Send + 'static`）へ move されるため `Send` を満たす
/// （`&'static Encoding` は `Sync`・`BTreeSet<String>` は `Send`。`Rc` 等を入れない）。
#[derive(Debug)]
pub struct CharsetNegotiator {
    /// 次の要求に使う文字コード。
    current: Charset,
    /// `shiori.forceencoding` が効いているか（効いていれば応答のヘッダで変わらない・要件 2.2）。
    forced: bool,
    /// 警告済みの「種別＋鍵」（`label:<x>`／`unmappable:<id>`／`invalid:<charset>`／
    /// `forced-mismatch`）。鍵の種類は有限（ラベル・イベント名・文字コード名）で、
    /// ラベルは正規化してから入れるので際限なく育たない。
    warned: std::collections::BTreeSet<String>,
}

impl CharsetNegotiator {
    /// 初期の文字コードと強制の有無で作る（要件 2.2・2.3・5.2）。
    #[must_use]
    pub fn new(initial: Charset, forced: bool) -> Self {
        Self {
            current: initial,
            forced,
            warned: std::collections::BTreeSet::new(),
        }
    }

    /// 次の要求に使う文字コード。
    #[must_use]
    pub fn current(&self) -> Charset {
        self.current
    }

    /// 次の応答の復号方針（強制なら [`CharsetPolicy::Force`]、それ以外は
    /// [`CharsetPolicy::Negotiate`]）。
    #[must_use]
    pub fn policy(&self) -> CharsetPolicy {
        if self.forced {
            CharsetPolicy::Force(self.current)
        } else {
            CharsetPolicy::Negotiate(self.current)
        }
    }

    /// 「種別＋鍵」が初出かを返し、記録する。
    fn first_time(&mut self, key: String) -> bool {
        self.warned.insert(key)
    }

    /// 要求の符号化結果を受け取り、表せない文字を置換していれば記録する（要件 3.5・7.2・7.4）。
    ///
    /// 置換 0 の通常経路では何もしない。
    pub fn note_request(&mut self, id: &str, replaced: usize) {
        if replaced == 0 {
            return;
        }
        let first = self.first_time(format!("unmappable:{id}"));
        warn_then_debug!(
            first,
            event = "charset_unmappable_replaced",
            id,
            replaced,
            "要求に現在の文字コードで表せない文字があった——数値文字参照へ置換して送る"
        );
    }

    /// GET 応答の `Charset` ヘッダ（生ラベル・省略時 `None`）と復号エラーの有無を受け取り、
    /// 採用と記録を行う（要件 2.2・4.2・4.4〜4.7）。
    ///
    /// 記録なしに後退する経路を持たない（要件 7.1）。NOTIFY の応答は採用の根拠に
    /// 用いないため、呼び手（`Shiori3Client::notify`）はこれを呼ばない（要件 5.3）。
    pub fn note_response(&mut self, charset_header: Option<&str>, decode_had_errors: bool) {
        if let Some(header) = charset_header {
            let resolved = Charset::for_label(header);
            if self.forced {
                // 強制中はヘッダを復号にも採用にも用いない。食い違いは初回のみ知らせる
                // （要件 2.2・4.5）。
                if resolved != Ok(self.current) && self.first_time("forced-mismatch".to_owned()) {
                    tracing::debug!(
                        target: LOG_TARGET,
                        event = "charset_forced_ignores_header",
                        forced = self.current.name(),
                        header,
                        "forceencoding が効いているため応答の Charset ヘッダを無視する"
                    );
                }
            } else {
                match resolved {
                    // SHIORI 側の宣言が優先。切替 1 回につき詳細ログ 1 行（要件 4.2・4.6）。
                    Ok(charset) if charset != self.current => {
                        let from = self.current.name();
                        self.current = charset;
                        tracing::debug!(
                            target: LOG_TARGET,
                            event = "charset_switched",
                            from,
                            to = charset.name(),
                            "応答の Charset ヘッダを以後の要求の文字コードとして採用した"
                        );
                    }
                    // 同じ文字コード（別名の綴りを含む）は状態も記録も変えない（要件 4.6）。
                    Ok(_) => {}
                    // 解決できないラベルは採用せず、現在の文字コードを継続する（要件 4.4・10.3）。
                    Err(error) => {
                        // 鍵は trim＋ASCII 小文字化（`foo`／`FOO` で 2 回警告しない）。
                        let key = format!("label:{}", header.trim().to_ascii_lowercase());
                        let first = self.first_time(key);
                        warn_then_debug!(
                            first,
                            event = "charset_label_unresolved",
                            label = header,
                            reason = error.reason(),
                            kept = self.current.name(),
                            "応答の Charset ヘッダを解決できない——採用せず現在の文字コードを続ける"
                        );
                    }
                }
            }
        }

        // 不正な並びの吸収は採用規則と独立に記録する（要件 4.7・10.2・7.4）。
        // 記録する名前は実際に復号に使った文字コード＝採用後の現在値。
        if decode_had_errors {
            let first = self.first_time(format!("invalid:{}", self.current.name()));
            warn_then_debug!(
                first,
                event = "charset_invalid_bytes_replaced",
                charset = self.current.name(),
                "応答に宣言された文字コードとして不正な並びがあった——代替文字で吸収して解析を続ける"
            );
        }
    }
}

#[cfg(test)]
#[path = "charset_tests.rs"]
mod charset_tests;
