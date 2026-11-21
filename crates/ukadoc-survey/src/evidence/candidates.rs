//! 手掛かり候補の提示（証拠とは別の出力）。
//!
//! ここは **証拠を作らない**。作るのは「正典 URL をまだ置いていない既存コードの、
//! URL を置くとよさそうな場所」の一覧だけである。判定は付けない
//! ——状態を決めるのは調査 spec の人手であって、この道具ではない。
//!
//! 出口は [`Candidate`] の並びで、[`super::EvidenceIndex`] には 1 件も入らない。
//! 型が別なので混ざりようがなく、検査の段もこの値を見ない。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない。ソースの
//! 本文は入出力層の [`crate::io::sources::walk`] が読んで渡す。
//!
//! # 拾う 4 種
//!
//! | 種類 | 拾う形 | 拾う文字列 |
//! |---|---|---|
//! | [`CandidateKind::AllowListElement`] | `const`/`static` のスライス定数（`= &[`〜`]`） | 要素ごとの最初の文字列リテラル |
//! | [`CandidateKind::BangCommandConsumer`] | 名前が `register` で終わる関数の呼び出し | 実引数の最初の文字列リテラル |
//! | [`CandidateKind::ConfigKey`] | `const`/`static` の文字列・`.get("…")` の照会 | 点で区切られた設定キーの綴り |
//! | [`CandidateKind::LogLine`] | ログマクロの呼び出し | [`LOG_WORDS`] を含む最初の文字列リテラル |
//!
//! # これは当て推量である
//!
//! 4 種はいずれも**綴りの形**だけで拾う。拾う数は実装が変わるたびに動くので、ここには
//! 書かない。
//!
//! **拾う数は常時走るテストでは守らない**。当て推量の出力を契約に
//! すると、実装が 1 行増えるたびに赤くなる。拾えたものが正典の項目である保証は無く、
//! 拾えなかった場所に項目が無い保証も無い。**当て推量のまま人へ渡す**のがこの出口の
//! 役目である。
//!
//! # 拾わない場所
//!
//! - `crates/ukadoc-survey/` 自身（走査の段で除く）。
//! - テストの本文——テストの置き場のファイル（[`is_test_path`]）と
//!   `#[cfg(test)] mod … { … }` の中（[`cfg_test_regions`]）。テストの見本は正典 URL を
//!   置く場所ではない。
//! - **すでに正典 URL が置かれている場所**。空行と属性（`#[…]`）を飛ばして遡った
//!   直上のコメントの塊に `ukadoc: <URL>` の行があれば、そこはもう証拠であって候補では
//!   ない。判定は [`extract`] へ委ねる——行の形の規則を
//!   ここで書き直すと、片方だけが直されて静かにずれる。
//! - コメント行そのもの。ログマクロの綴りを説明する doc コメントは実物にあり、行を
//!   選ばずに拾うと散文が候補になる。

use std::collections::BTreeSet;

use super::extract::extract;
use super::lex::{
    SLICE_START, call_string_literals, line_offsets, matching_brace, read_string,
    slice_element_names,
};
use super::{Candidate, CandidateKind};

/// ログ行を手掛かりとみなす語。
///
/// 前の 3 語は「縮退」「無視」「未知」。残りの 4 語は「正典の項目を十分に扱えていない」
/// ことを述べる同族として実物のログから選んだ。**書かれていない語は入れない**——語を
/// 増やすほど当たりは増えるが、当たった先に実物が無ければ人の手間が増えるだけである。
const LOG_WORDS: &[&str] = &[
    "縮退",
    "無視",
    "未知",
    "未対応",
    "非対応",
    "未実装",
    "対象外",
];

/// ログのマクロ名（末尾の `!` まで）。`tracing::debug!` のような経路付きも拾える
/// ——直前の 1 バイトが識別子の続きでなければよい。
const LOG_MACROS: &[&str] = &["trace!", "debug!", "info!", "warn!", "error!"];

/// 設定キーとみなさない末尾（ファイル名の拡張子）。
///
/// 実物の点付き小文字リテラルの末尾から選んだ。`descript.txt`・`*.png` のような
/// ファイル名が多く、これを外さないと ⑶ はファイル名の一覧になる。
const KEY_FILE_SUFFIXES: &[&str] = &["png", "txt", "dll", "exe", "toml", "rs", "bin", "dat"];

/// 手掛かり候補を集める。
///
/// `sources` は走査で読んだ `(パス, 本文)` の組。失敗しない——当て推量に失敗は無く、
/// 拾えなければ空が返るだけである。
///
/// 返る並びは **パス → 種類 → 文字列の昇順で、同じ組は 1 件に畳む**。入力の並びにも
/// 本文の中の現れる順にも依らない。
///
/// この値は [`super::EvidenceIndex`] へは入らない。
pub fn candidates(sources: &[(String, String)]) -> Vec<Candidate> {
    let mut found: BTreeSet<Candidate> = BTreeSet::new();
    for (path, text) in sources {
        if is_test_path(path) {
            continue;
        }
        scan(path, text, &mut found);
    }
    found.into_iter().collect()
}

/// テストの本文を持つファイルかどうか（パスだけで決める）。
///
/// 兄弟テストファイル（`*_tests.rs`）・`*_test.rs`・`tests/` 配下・`*_tests/` 配下
/// （`placement_tests/` の類）・`mod tests;` が読む `tests.rs` の 5 形。
fn is_test_path(path: &str) -> bool {
    path.ends_with("_tests.rs")
        || path.ends_with("_test.rs")
        || path.contains("/tests/")
        || path.contains("_tests/")
        || path.ends_with("/tests.rs")
}

/// 1 ファイルの本文から候補を拾う。
fn scan(path: &str, text: &str, out: &mut BTreeSet<Candidate>) {
    let bytes = text.as_bytes();
    let lines = line_offsets(text);
    let skips = cfg_test_regions(bytes, &lines);

    for (index, (offset, line)) in lines.iter().enumerate() {
        if in_skipped(&skips, *offset) {
            continue;
        }
        if line.trim_start().starts_with("//") {
            continue;
        }
        let mut hits: Vec<(CandidateKind, String)> = Vec::new();
        for name in allow_list_elements(bytes, *offset, line) {
            hits.push((CandidateKind::AllowListElement, name));
        }
        for name in registration_names(bytes, *offset, line) {
            hits.push((CandidateKind::BangCommandConsumer, name));
        }
        for key in config_keys(bytes, *offset, line) {
            hits.push((CandidateKind::ConfigKey, key));
        }
        for message in log_messages(bytes, *offset, line) {
            hits.push((CandidateKind::LogLine, message));
        }
        if hits.is_empty() || has_marker_above(&lines, index) {
            continue;
        }
        for (kind, text) in hits {
            out.insert(Candidate {
                path: path.to_owned(),
                kind,
                text,
            });
        }
    }
}

/// ⑴ 許可表の要素文字列。
///
/// 行が `const`/`static` の宣言で、その行に `= &[` があれば、そこから始まる表の
/// **要素ごとの最初の文字列リテラル**を拾う（語彙表の取り出しと同じ規則）。要素が名前付き
/// 定数だけで綴られている表（実物の `HOLD_FIELDS` の類）は文字列を 1 つも持たない
/// ので、この規則では何も拾わない。
fn allow_list_elements(bytes: &[u8], line_offset: usize, line: &str) -> Vec<String> {
    if !is_const_item(line) {
        return Vec::new();
    }
    let Some(at) = line.find(SLICE_START) else {
        return Vec::new();
    };
    slice_element_names(bytes, line_offset + at + SLICE_START.len()).unwrap_or_default()
}

/// ⑵ `\![...]` の消費側の登録名。
///
/// 名前が `register` で終わる関数の呼び出しの、実引数の**最初の文字列リテラル**。
/// 実物は `ledger.try_register("move", None, CommandConsumer::MoveSink)` の形で、
/// 定義（`pub fn try_register(`）は実引数に文字列を持たないので拾われない。
/// 表の行の形（`("move", None, …)`）は拾わない——広げるとほかの 3 つ組のタプルまで拾う。
fn registration_names(bytes: &[u8], line_offset: usize, line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(at) = line[from..].find("register(") {
        let open = from + at + "register".len();
        if let Some(name) = call_string_literals(bytes, line_offset + open)
            .into_iter()
            .next()
        {
            out.push(name);
        }
        from = open + 1;
    }
    out
}

/// ⑶ 設定キーの表。
///
/// 2 つの形から拾う。⑴ `const`/`static` の文字列定数（スライスでないもの）
/// ⑵ `.get("…")` の照会。どちらも綴りが[点で区切られた設定キー](is_dotted_key)の形を
/// していることを要る。
///
/// 実物の綴りから形を決めた——
/// `const SHELL_DPI_KEY: &str = "seriko.dpi";`（`crates/areka-placement/src/source.rs`）と
/// `shell_kv.get("seriko.zorder")`（`crates/areka-placement/src/config.rs`）の 2 形が
/// 実物の設定キーの置かれ方である。
fn config_keys(bytes: &[u8], line_offset: usize, line: &str) -> Vec<String> {
    let mut out = Vec::new();
    if is_const_item(line) && !line.contains(SLICE_START) {
        out.extend(dotted_key_at(bytes, line_offset, line.find('"')));
    }
    let mut from = 0usize;
    while let Some(at) = line[from..].find(".get(") {
        let quote = from + at + ".get(".len();
        if line.as_bytes().get(quote) == Some(&b'"') {
            out.extend(dotted_key_at(bytes, line_offset, Some(quote)));
        }
        from = quote;
    }
    out
}

/// 行の中の `at`（行頭からの相対）にある文字列リテラルが設定キーの形なら返す。
fn dotted_key_at(bytes: &[u8], line_offset: usize, at: Option<usize>) -> Option<String> {
    let at = at?;
    let (literal, _) = read_string(bytes, line_offset + at);
    is_dotted_key(&literal).then_some(literal)
}

/// ⑷「縮退」「無視」「未知」などを含むログ行。
///
/// ログマクロの呼び出しから文字列リテラルをすべて読み、[`LOG_WORDS`] のいずれかを
/// 含む**最初の 1 本**を拾う。最初の 1 本ではなく最初に語を含む 1 本を選ぶのは、実物に
/// `warn!(target: "…", "…")` と `debug!(?size, "…")` の両方があり、どちらでも本文は
/// 「語を含む方」だからである。
fn log_messages(bytes: &[u8], line_offset: usize, line: &str) -> Vec<String> {
    let Some(open) = log_macro_open(line) else {
        return Vec::new();
    };
    call_string_literals(bytes, line_offset + open)
        .into_iter()
        .find(|literal| LOG_WORDS.iter().any(|word| literal.contains(word)))
        .into_iter()
        .collect()
}

/// 行の中のログマクロの `(` の位置（行頭からの相対）。
///
/// 直前の 1 バイトが識別子の続き（英数字か下線）なら別の名前の一部なので取らない。
fn log_macro_open(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut best: Option<usize> = None;
    for macro_name in LOG_MACROS {
        let mut from = 0usize;
        while let Some(at) = line[from..].find(macro_name) {
            let start = from + at;
            let preceded = start
                .checked_sub(1)
                .and_then(|before| bytes.get(before))
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
            let open = start + macro_name.len();
            if !preceded && bytes.get(open) == Some(&b'(') {
                best = Some(best.map_or(open, |current: usize| current.min(open)));
                break;
            }
            from = start + 1;
        }
    }
    best
}

/// 空行と属性（`#[…]`）を飛ばして遡った直上のコメントの塊に正典 URL の行があるか。
///
/// 行の形の判定は [`extract`] に任せる（証拠の取り出しと同じ規則をここで書き直さない）。
/// 空行を飛ばすのは、目印の後ろの空行を越えて表を探す `resolve` の読み方と揃えるため。
fn has_marker_above(lines: &[(usize, &str)], index: usize) -> bool {
    let mut at = index;
    while at > 0 {
        at -= 1;
        let trimmed = lines[at].1.trim_start();
        if trimmed.starts_with("//") {
            if !extract("", trimmed).is_empty() {
                return true;
            }
            continue;
        }
        if trimmed.starts_with("#[") || trimmed.is_empty() {
            continue;
        }
        return false;
    }
    false
}

/// `#[cfg(test)]` の付いた `mod … { … }` の占める範囲（バイト位置の半開区間）。
///
/// 実物の `#[cfg(test)]` の多くは `mod tests {` で、残りは別ファイルへ出す `mod …;`
/// である。中身の無い `mod …;` は範囲を持たないので何もしない。`mod` 以外の項目に
/// 付いた `#[cfg(test)]` は**飛ばさない**——次の中括弧まで
/// 読み飛ばすと、無関係な後続の項目まで消えるからである。
fn cfg_test_regions(bytes: &[u8], lines: &[(usize, &str)]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (index, (offset, line)) in lines.iter().enumerate() {
        if line.trim() != "#[cfg(test)]" {
            continue;
        }
        let mut at = index + 1;
        while at < lines.len() {
            let trimmed = lines[at].1.trim_start();
            if trimmed.is_empty() || trimmed.starts_with("#[") || trimmed.starts_with("//") {
                at += 1;
                continue;
            }
            break;
        }
        let Some((mod_offset, mod_line)) = lines.get(at) else {
            continue;
        };
        if !is_mod_item(mod_line) {
            continue;
        }
        let Some(brace) = mod_line.find('{') else {
            continue;
        };
        if let Some(end) = matching_brace(bytes, mod_offset + brace) {
            out.push((*offset, end));
        }
    }
    out
}

/// その位置が読み飛ばす範囲の中か。
fn in_skipped(regions: &[(usize, usize)], offset: usize) -> bool {
    regions
        .iter()
        .any(|(start, end)| *start <= offset && offset < *end)
}

/// 行が `mod` の宣言か（`pub` と可視性の括弧を落として見る）。
fn is_mod_item(line: &str) -> bool {
    strip_visibility(line).starts_with("mod ")
}

/// 行が `const`/`static` の宣言か（`pub` と可視性の括弧を落として見る）。
fn is_const_item(line: &str) -> bool {
    let rest = strip_visibility(line);
    rest.starts_with("const ") || rest.starts_with("static ")
}

/// 行頭の空白と `pub`・`pub(crate)` の類を落とす。
fn strip_visibility(line: &str) -> &str {
    let rest = line.trim_start();
    let Some(after) = rest.strip_prefix("pub") else {
        return rest;
    };
    let after = after.trim_start();
    let Some(paren) = after.strip_prefix('(') else {
        return after;
    };
    match paren.find(')') {
        Some(at) => paren[at + 1..].trim_start(),
        None => "",
    }
}

/// 綴りが点で区切られた設定キーの形か。
///
/// 小文字で始まり、小文字・数字・`.`・`-`・`_` だけででき、点を 1 つ以上含み、空の
/// 区間を持たず、末尾が[ファイル名の拡張子](KEY_FILE_SUFFIXES)でないもの。
fn is_dotted_key(raw: &str) -> bool {
    if !raw.starts_with(|c: char| c.is_ascii_lowercase()) {
        return false;
    }
    if !raw
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_')
    {
        return false;
    }
    let segments: Vec<&str> = raw.split('.').collect();
    if segments.len() < 2 || segments.iter().any(|segment| segment.is_empty()) {
        return false;
    }
    match segments.last() {
        Some(last) => !KEY_FILE_SUFFIXES.contains(last),
        None => false,
    }
}

#[cfg(test)]
#[path = "candidates_tests.rs"]
mod tests;
