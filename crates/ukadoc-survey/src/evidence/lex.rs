//! ソースの本文の字句の読み手（行の割り方・スライス定数の要素・文字列リテラル・
//! コメント）。
//!
//! 手掛かり候補（`candidates`）と証拠の語彙表の取り出し（`resolve`）が同じ読み手を
//! 借りる。候補と証拠が同じ表を同じ要素に割るための 1 か所である。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない。

/// スライス定数の始まりの綴り。
pub(super) const SLICE_START: &str = "= &[";

/// 本文を行に割り、各行の開始位置を添えて返す（行末の改行は含めない）。
pub(super) fn line_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        out.push((offset, line.trim_end_matches('\n')));
        offset += line.len();
    }
    out
}

/// スライス定数の要素ごとの最初の文字列リテラル。
///
/// `start` は `= &[` の直後。閉じる `]` に届かなければ `None`（読み切れない表から
/// 何も採らない）。文字列・生文字列・文字リテラル・コメントの中は構造として読まない
/// ——要素の名前には角括弧を含むものがあり、注記のコメントには括弧もコンマも現れる。
pub(super) fn slice_element_names(bytes: &[u8], start: usize) -> Option<Vec<String>> {
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
            b'r' => match read_raw_string(bytes, i) {
                Some((literal, next)) => {
                    if first.is_none() {
                        first = Some(literal);
                    }
                    i = next;
                }
                None => i += 1,
            },
            b']' if depth == 0 => {
                names.extend(first);
                return Some(names);
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
    None
}

/// `open`（`(` の位置）から始まる呼び出しの中の文字列リテラルを、現れた順に返す。
///
/// 入れ子の括弧は数え、文字列・文字リテラル・コメントの中は構造として読まない。
/// 対応する `)` で終わる。閉じなければ本文の終端まで読む。
pub(super) fn call_string_literals(bytes: &[u8], open: usize) -> Vec<String> {
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
            b'r' => match read_raw_string(bytes, i) {
                Some((literal, next)) => {
                    out.push(literal);
                    i = next;
                }
                None => i += 1,
            },
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
pub(super) fn matching_brace(bytes: &[u8], at: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = at;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = read_string(bytes, i).1,
            b'r' => i = read_raw_string(bytes, i).map_or(i + 1, |(_, next)| next),
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
/// （`\\` `\"` `\'` `\n` `\r` `\t` `\0` の 7 つ）。
pub(super) fn read_string(bytes: &[u8], at: usize) -> (String, usize) {
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

/// `at` の `r` が生文字列（`r"…"`・`r#"…"#`・`br"…"`）の始まりなら、中身と閉じた
/// 次の位置。識別子の途中の `r` や閉じない生文字列は `None`。
///
/// 生文字列は `"` を逃がさずに持てるので、普通の文字列として読むと引用符の数がずれて
/// 括弧の数えが狂う。
pub(super) fn read_raw_string(bytes: &[u8], at: usize) -> Option<(String, usize)> {
    let is_ident = |i: usize| bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_';
    let mut start = at;
    if start > 0 && bytes[start - 1] == b'b' {
        start -= 1;
    }
    if start > 0 && is_ident(start - 1) {
        return None;
    }
    let hashes = bytes[at + 1..]
        .iter()
        .take_while(|byte| **byte == b'#')
        .count();
    let open = at + 1 + hashes;
    if bytes.get(open) != Some(&b'"') {
        return None;
    }
    let closing: Vec<u8> = std::iter::once(b'"')
        .chain(std::iter::repeat_n(b'#', hashes))
        .collect();
    let body = open + 1;
    let end = body
        + bytes[body..]
            .windows(closing.len())
            .position(|window| window == closing.as_slice())?;
    let literal = String::from_utf8(bytes[body..end].to_vec()).unwrap_or_default();
    Some((literal, end + closing.len()))
}

/// `at` の `'` が文字リテラルなら、その次の位置。そうでなければ 1 つ進めた位置。
///
/// 生存期間（`&'a str`）を文字リテラルと読み違えないよう、`'x'` と `'\x'` の 2 形
/// だけを文字リテラルとみなす。`b'{'` の類が括弧の数えを狂わせるのを防ぐのが目的。
pub(super) fn skip_char_literal(bytes: &[u8], at: usize) -> usize {
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
pub(super) fn skip_line_comment(bytes: &[u8], at: usize) -> usize {
    match bytes[at..].iter().position(|byte| *byte == b'\n') {
        Some(offset) => at + offset + 1,
        None => bytes.len(),
    }
}

/// `/*` から対応する `*/` の次までを飛ばす（入れ子を数える）。
pub(super) fn skip_block_comment(bytes: &[u8], at: usize) -> usize {
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
