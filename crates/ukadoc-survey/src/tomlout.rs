//! 決定的な TOML 本文の組み立て（逃がし・インラインテーブル・キー付きテーブル）。
//!
//! カタログと台帳の**書き出しはここに閉じる**（設計「Architecture Pattern & Boundary
//! Map」の「境界の要点」1 つ目）。読み取りは `toml` に任せるのに、書き出しだけ自前で
//! 持つのには理由がある。TOML の書き出しライブラリは値の書き方を自分で選ぶため、
//! 要件付録 A.3 が凍結した書き方（文字列は二重引用符・逆斜線は 2 つ重ねる）と一致
//! しない。実測で `toml` 1.1.4 は逆斜線を含む文字列を**単引用符の素の文字列**で書く。
//! 台帳は人が手で書き、機械が差し込む文書なので、両者の書き方が割れると差分が濁る。
//!
//! 自前で書くことの危うさは、在中テスト（`tomlout_tests.rs`）の 2 本立てで潰す。
//! 「組み上げた本文を `toml` で読み戻すと元の値に戻る」（設計 `tomlout` 節の不変条件）と、
//! 「組み上がった本文そのものの逐語一致」である。前者だけでは、必要以上に逃がす書き方も
//! 通ってしまう。
//!
//! # 逃がしの規則（設計 D-10）
//!
//! 書き出しはすべて TOML の二重引用符の文字列を使い、
//!
//! - `\` は `\\`、`"` は `\"`、
//! - 制御文字（U+0000〜U+001F と U+007F）は `\u00XX`（16 進大文字）
//!
//! に逃がす。改行やタブも同じ規則で扱い、短縮形（`\n`・`\t`）は使わない——制御文字の
//! 書き方を 1 つに揃えておくと、逃がし漏れの検査が「制御文字が生で残っていないこと」の
//! 一言で済む。実測ではカタログに載る id にも見出しにも制御文字は 1 件も無いので
//! （設計 D-10）、この選択がカタログの本文を動かすことはない。
//!
//! **非 ASCII は逃がさない**。ukadoc の見出しは日本語であり、要件 9.5 は人が読める
//! 報告を求めている。`\uXXXX` に潰すと読み戻しは通るがカタログが読めなくなり、
//! 設計 D-1 の実測（1 行最大 579 文字）も破る。単引用符も二重引用符の文字列の中では
//! 逃がす必要が無い（実測 3 件）。
//!
//! # 並び
//!
//! ここにある関数は**与えた順を一切変えない**（設計 D-9）。カタログの欄の並びは
//! 列順で凍結されており、版番号の配列を昇順に整えるのは呼ぶ側の仕事である。
//! 並べ替えても TOML としては同じ表に読み戻るので、この不変は逐語一致テストが守る。
//!
//! # 事後条件
//!
//! 出力は生の改行を含まない 1 行である（設計 `tomlout` 節）。カタログは 1 項目 1 行
//! （要件 1.1）であり、台帳の複数行の備考は人が手で書く欄なので、ここを通らない。

/// TOML の二重引用符の文字列を組み立てる。
///
/// `\` と `"` と制御文字を逃がす。非 ASCII と単引用符はそのまま書く（設計 D-10）。
/// 返り値は前後の引用符を含み、生の改行を含まない。
///
/// # 例
///
/// ```
/// use ukadoc_survey::tomlout::basic_string;
///
/// assert_eq!(basic_string("dev_bind"), r#""dev_bind""#);
/// assert_eq!(basic_string(r"\![raise]"), r#""\\![raise]""#);
/// assert_eq!(basic_string("改行\nあり"), r#""改行\u000Aあり""#);
/// ```
pub fn basic_string(value: &str) -> String {
    // 逃がしで伸びる分の余地を少しだけ見ておく（実測の見出しは最大 105 文字）。
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str("\\\""),
            _ if is_toml_control(ch) => push_unicode_escape(&mut out, ch),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// 文字列の配列を組み立てる。`["a", "b"]`・空なら `[]`。
///
/// 要素は 1 つずつ [`basic_string`] を通る。**与えた順のまま**で、並べ替えない
/// （設計 D-9。カタログの版番号を昇順に整えるのは呼ぶ側の仕事）。
///
/// # 例
///
/// ```
/// use ukadoc_survey::tomlout::string_array;
///
/// assert_eq!(string_array(&[]), "[]");
/// assert_eq!(
///     string_array(&["2.3.53".to_owned(), "2.5.60".to_owned()]),
///     r#"["2.3.53", "2.5.60"]"#
/// );
/// ```
pub fn string_array(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&basic_string(value));
    }
    out.push(']');
    out
}

/// インラインテーブルを組み立てる。`{ k = v, ... }`・対が無ければ `{}`。
///
/// 値は**組み上がった TOML の値の断片**を受け取る。呼ぶ側が [`basic_string`] や
/// [`string_array`] を通した結果を渡し、ここで引用符を付け直すことはしない
/// （設計「Data Models」のカタログ 1 行が `versions = []` を含むのはこのため）。
///
/// 対は**与えた順のまま**並ぶ。名前順に並べ替えると TOML としては同じ表に読み戻るが、
/// カタログの列順（設計 D-9）が崩れ、要件 1.5 が守る対象そのものが変わる。
///
/// 鍵は素の鍵としてそのまま書く（カタログの欄名はいずれも `[A-Za-z_]` のみ）。
///
/// # 例
///
/// ```
/// use ukadoc_survey::tomlout::{basic_string, inline_table, string_array};
///
/// let pairs = [
///     ("page", basic_string("dev_bind")),
///     ("versions", string_array(&[])),
/// ];
/// assert_eq!(inline_table(&pairs), r#"{ page = "dev_bind", versions = [] }"#);
/// ```
pub fn inline_table(pairs: &[(&str, String)]) -> String {
    if pairs.is_empty() {
        return "{}".to_owned();
    }
    let mut out = String::from("{ ");
    for (index, (key, value)) in pairs.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(key);
        out.push_str(" = ");
        out.push_str(value);
    }
    out.push_str(" }");
    out
}

/// キー付きテーブルの見出しを組み立てる。`[entry."<id>"]`（要件付録 A.1）。
///
/// `prefix` は素の鍵としてそのまま書き、`key` は [`basic_string`] を通す。
/// 台帳の id には逆斜線が入り得るので、ここを通さないと付録 A.3 の形にならない。
///
/// # 例
///
/// ```
/// use ukadoc_survey::tomlout::keyed_table_header;
///
/// assert_eq!(
///     keyed_table_header("entry", "ukadoc:list_propertysystem:system.year:1"),
///     r#"[entry."ukadoc:list_propertysystem:system.year:1"]"#
/// );
/// ```
pub fn keyed_table_header(prefix: &str, key: &str) -> String {
    format!("[{prefix}.{}]", basic_string(key))
}

/// TOML の二重引用符の文字列の中で逃がさなければならない制御文字か。
///
/// TOML は U+0000〜U+0008・U+000A〜U+001F・U+007F の生書きを禁じる。水平タブ
/// （U+0009）だけは生で書いてよいが、ここでは制御文字を一様に逃がす（設計 D-10）。
/// 非 ASCII は対象外——日本語の見出しをそのまま書くため。
fn is_toml_control(ch: char) -> bool {
    matches!(ch, '\u{0}'..='\u{1f}' | '\u{7f}')
}

/// 制御文字を `\u00XX`（16 進大文字）として押し込む。
///
/// 呼ばれるのは [`is_toml_control`] が真の文字だけで、いずれも U+007F 以下なので
/// 上位 2 桁は必ず `00` である。
fn push_unicode_escape(out: &mut String, ch: char) {
    let code = ch as u32;
    out.push_str("\\u00");
    out.push(hex_digit((code >> 4) & 0xf));
    out.push(hex_digit(code & 0xf));
}

/// 0〜15 を 16 進の大文字 1 桁にする。
fn hex_digit(value: u32) -> char {
    match value & 0xf {
        digit @ 0..=9 => char::from(b'0' + digit as u8),
        digit => char::from(b'A' + (digit as u8 - 10)),
    }
}

#[cfg(test)]
#[path = "tomlout_tests.rs"]
mod tests;
