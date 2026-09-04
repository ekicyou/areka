//! 手掛かり候補の提示（要件 5.8・証拠とは別の出力）。
//!
//! ここは **証拠を作らない**。作るのは「正典 URL をまだ置いていない既存コードの、
//! URL を置くとよさそうな場所」の一覧だけである（要件 5.8）。判定は付けない
//! ——状態を決めるのは調査 spec の人手であって、この道具ではない（要件 5.9）。
//!
//! 出口は [`Candidate`] の並びで、[`super::EvidenceIndex`] には 1 件も入らない。
//! 型が別なので混ざりようがなく、検査の段もこの値を見ない（設計 Requirements
//! Traceability 5.9「検査は候補を見ない」）。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。ソースの
//! 本文は入出力層の [`crate::io::sources::walk`] が読んで渡す。
//!
//! # 拾う 4 種（設計「候補の種類（要件 5.8）」）
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
//! 4 種はいずれも**綴りの形**だけで拾う。実装時に実物（`crates/**/*.rs` 1,089 本）へ
//! 当てて数えたところ、設計の挙げる 3 か所の件数と一致した——⑴ は
//! `crates/areka-kanade/src/schedule/events.rs` の 11 件と
//! `crates/areka-kanade/src/schedule/resources.rs` の 1 件、⑵ は
//! `crates/areka/src/emo2_boot/consumer_ledger.rs` の 4 件。全体では 381 件
//! （⑴ 254・⑵ 4・⑶ 24・⑷ 99）である。
//!
//! **この件数は常時走るテストでは守らない**（要件 6.2）。当て推量の出力を契約に
//! すると、実装が 1 行増えるたびに赤くなる。拾えたものが正典の項目である保証は無く、
//! 拾えなかった場所に項目が無い保証も無い。**当て推量のまま人へ渡す**のがこの出口の
//! 役目である（要件 5.9）。
//!
//! # 拾わない場所
//!
//! - `crates/ukadoc-survey/` 自身（走査の段で除く・設計 D-3）。
//! - テストの本文——`_tests.rs`・`_test.rs`・`tests/` 配下・`#[cfg(test)] mod … { … }`
//!   の中（[`is_test_path`]・[`cfg_test_regions`]）。テストの見本は正典 URL を置く
//!   場所ではない。理由は要件 5.2 の「定義箇所だけに置く」であって、件数の話では
//!   ない——実測では `consumer_ledger.rs` の `try_register` 18 か所のうち 13 か所が
//!   テストの中だが、名前は `canonical()` の 4 件と重なるので、除外しなくても畳んだ
//!   後の件数は 5 件（設計の数える 4 件＋テストだけに現れる `"resize"`）にしかならない。
//! - **すでに正典 URL が置かれている場所**。直上の連続したコメントの塊に
//!   `ukadoc: <URL>` の行があれば、そこはもう証拠であって候補ではない（要件 5.8 の
//!   「まだ置かれていない既存コード」）。判定は [`extract`] へ委ねる——行の形の規則を
//!   ここで書き直すと、片方だけが直されて静かにずれる。
//! - コメント行そのもの。`debug!(…)` の綴りを説明する doc コメントが実物にあり
//!   （`crates/areka/src/input_events/balloon.rs`）、行を選ばずに拾うと散文が候補になる。

use std::collections::BTreeSet;

use super::extract::extract;
use super::{Candidate, CandidateKind};

/// ログ行を手掛かりとみなす語（要件 5.8「「縮退」「無視」「未知」など」）。
///
/// 前の 3 語は要件が名指しした語。残りの 4 語は「正典の項目を十分に扱えていない」
/// ことを述べる同族として実物から選んだ。**書かれていない語は入れない**——語を
/// 増やすほど当たりは増えるが、当たった先に実物が無ければ人の手間が増えるだけである。
/// 7 語すべてが実物のログに現れる（実装時の実測で 縮退 58・未知 18・無視 16・
/// 未対応 5・非対応 2・対象外 2・未実装 1 件）。
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
/// 実物の点付き小文字リテラルの末尾を数えて選んだ。`descript.txt`（90 件）・
/// `*.png`（665 件）のようなファイル名が最も多く、これを外さないと ⑶ はファイル名の
/// 一覧になる。
const KEY_FILE_SUFFIXES: &[&str] = &["png", "txt", "dll", "exe", "toml", "rs", "bin", "dat"];

/// スライス定数の始まりの綴り（設計 D-5 と同じ「`= &[` から」）。
const SLICE_START: &str = "= &[";

/// 手掛かり候補を集める（要件 5.8）。
///
/// `sources` は走査で読んだ `(パス, 本文)` の組。失敗しない——当て推量に失敗は無く、
/// 拾えなければ空が返るだけである。
///
/// 返る並びは **パス → 種類 → 文字列の昇順で、同じ組は 1 件に畳む**。入力の並びにも
/// 本文の中の現れる順にも依らない（要件 7.3 の決定性）。
///
/// この値は [`super::EvidenceIndex`] へは入らない（要件 5.9）。
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
/// 兄弟テストファイル（`*_tests.rs`）・`tests/` 配下・`*_test.rs` の 3 形。
fn is_test_path(path: &str) -> bool {
    path.ends_with("_tests.rs")
        || path.ends_with("_test.rs")
        || path.contains("/tests/")
        || path.starts_with("tests/")
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
/// **要素ごとの最初の文字列リテラル**を拾う（設計 D-5 と同じ規則）。要素が名前付き
/// 定数だけで綴られている表（実物の `HOLD_FIELDS` の類）は文字列を 1 つも持たない
/// ので、この規則では何も拾わない。
fn allow_list_elements(bytes: &[u8], line_offset: usize, line: &str) -> Vec<String> {
    if !is_const_item(line) {
        return Vec::new();
    }
    let Some(at) = line.find(SLICE_START) else {
        return Vec::new();
    };
    slice_element_names(bytes, line_offset + at + SLICE_START.len())
}

/// ⑵ `\![...]` の消費側の登録名。
///
/// 名前が `register` で終わる関数の呼び出しの、実引数の**最初の文字列リテラル**。
/// 実物は `ledger.try_register("move", None, CommandConsumer::MoveSink)` の形で、
/// 定義（`pub fn try_register(`）は実引数に文字列を持たないので拾われない。
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
/// **設計にはこの種の実例の引用が無い**ので、実物の綴りから形を決めた——
/// `const SHELL_DPI_KEY: &str = "seriko.dpi";`（`crates/areka/src/placement/source.rs`）と
/// `shell_kv.get("seriko.zorder")`（`crates/areka/src/placement/config.rs`）の 2 形が
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

/// 直上の連続したコメントの塊に正典 URL の行があるか。
///
/// 行の形の判定は [`extract`] に任せる（証拠の取り出しと同じ規則をここで書き直さない）。
/// 属性（`#[…]`）はコメントと定義の間に挟まるので飛ばして遡る。
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
/// 実物の `#[cfg(test)]` は 148 か所が `mod tests {` で、残りは別ファイルへ出す
/// `mod …;` である。中身の無い `mod …;` は範囲を持たないので何もしない。`mod` 以外の
/// 項目に付いた `#[cfg(test)]`（実測でごく少数）は**飛ばさない**——次の中括弧まで
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

/// 本文を行に割り、各行の開始位置を添えて返す（行末の改行は含めない）。
fn line_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        out.push((offset, line.trim_end_matches('\n')));
        offset += line.len();
    }
    out
}

/// スライス定数の要素ごとの最初の文字列リテラル（設計 D-5 と同じ規則）。
///
/// `start` は `= &[` の直後。閉じる `]` に届かなければ何も拾わない。文字列の中と
/// コメントの中は構造として読まない——要素の名前には角括弧を含むものがあり、注記の
/// コメントには括弧もコンマも現れる。
fn slice_element_names(bytes: &[u8], start: usize) -> Vec<String> {
    let mut names = Vec::new();
    let mut first: Option<String> = None;
    let mut depth = 0usize;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let (literal, next) = read_string(bytes, i);
                if first.is_none() {
                    first = Some(literal);
                }
                i = next;
            }
            b'\'' => i = skip_char_literal(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'/') => i = skip_line_comment(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i),
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b'}' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b']' if depth == 0 => {
                names.extend(first);
                return names;
            }
            b']' => {
                depth -= 1;
                i += 1;
            }
            b',' if depth == 0 => {
                names.extend(first.take());
                i += 1;
            }
            _ => i += 1,
        }
    }
    Vec::new()
}

/// `open`（`(` の位置）から始まる呼び出しの中の文字列リテラルを、現れた順に返す。
///
/// 入れ子の括弧は数え、文字列・文字リテラル・コメントの中は構造として読まない。
/// 対応する `)` で終わる。閉じなければ本文の終端まで読む。
fn call_string_literals(bytes: &[u8], open: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let (literal, next) = read_string(bytes, i);
                out.push(literal);
                i = next;
            }
            b'\'' => i = skip_char_literal(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'/') => i = skip_line_comment(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i),
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                i += 1;
                if depth == 0 {
                    return out;
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// `at` の `{` に対応する `}` の次の位置。対応が無ければ `None`。
fn matching_brace(bytes: &[u8], at: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = at;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = read_string(bytes, i).1,
            b'\'' => i = skip_char_literal(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'/') => i = skip_line_comment(bytes, i),
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i),
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// `at` の引用符から文字列リテラルを 1 つ読み、中身と次の位置を返す。
///
/// 逃がした引用符は文字列を終わらせない。逃がし形は綴りを戻す
/// （`\\` `\"` `\'` `\n` `\r` `\t` `\0` の 7 つ・設計 D-5 の取り出しと同じ扱い）。
fn read_string(bytes: &[u8], at: usize) -> (String, usize) {
    let mut out = Vec::new();
    let mut i = at + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                break;
            }
            b'\\' => {
                match bytes.get(i + 1).copied() {
                    Some(b'\\') => out.push(b'\\'),
                    Some(b'"') => out.push(b'"'),
                    Some(b'\'') => out.push(b'\''),
                    Some(b'n') => out.push(b'\n'),
                    Some(b'r') => out.push(b'\r'),
                    Some(b't') => out.push(b'\t'),
                    Some(b'0') => out.push(0),
                    Some(other) => {
                        out.push(b'\\');
                        out.push(other);
                    }
                    None => out.push(b'\\'),
                }
                i += 2;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    // 元が UTF-8 の本文で、切り出しは ASCII の位置でしか起きないので必ず戻る。
    (String::from_utf8(out).unwrap_or_default(), i)
}

/// `at` の `'` が文字リテラルなら、その次の位置。そうでなければ 1 つ進めた位置。
///
/// 生存期間（`&'a str`）を文字リテラルと読み違えないよう、`'x'` と `'\x'` の 2 形
/// だけを文字リテラルとみなす。`b'{'` の類が括弧の数えを狂わせるのを防ぐのが目的。
fn skip_char_literal(bytes: &[u8], at: usize) -> usize {
    if bytes.get(at + 1) == Some(&b'\\') {
        if bytes.get(at + 3) == Some(&b'\'') {
            return at + 4;
        }
    } else if bytes.get(at + 2) == Some(&b'\'') {
        return at + 3;
    }
    at + 1
}

/// `//` から行末（改行を含む）までを飛ばす。
fn skip_line_comment(bytes: &[u8], at: usize) -> usize {
    match bytes[at..].iter().position(|byte| *byte == b'\n') {
        Some(offset) => at + offset + 1,
        None => bytes.len(),
    }
}

/// `/*` から対応する `*/` の次までを飛ばす（入れ子を数える）。
fn skip_block_comment(bytes: &[u8], at: usize) -> usize {
    let mut depth = 0usize;
    let mut i = at;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth = depth.saturating_sub(1);
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

#[cfg(test)]
#[path = "candidates_tests.rs"]
mod tests;
