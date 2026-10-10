//! 構造テスト: 層の向きと、端末へ出る文の ASCII を、本番ファイルの綴りで見張る。
//!
//! - 判断の中核（`plan.rs`）の本文が、ファイル・時計・プロセス・ログを読み込む綴りを持たない
//!   （設計「依存の向き」。走査は `plan.rs` だけ。兄弟の `plan_*_tests.rs`・`plan_test_support.rs`
//!   はテストで、時刻や偽の口を使ってよい）。
//! - 端末へ出る文を持つファイルの文字列・文字のリテラルが ASCII だけ（要件 9.2）。対象は設計が
//!   名指す 4 つ: 入口 `cli.rs`・表示 `status.rs`・待ちの終わりの 1 行を作る `wait.rs`・失敗の
//!   本文を持つ `error.rs`（どれも、リテラルがそのまま端末へ出る）。兄弟のテストファイルは
//!   日本語の値（名前・内容）をわざと入れるので対象に入れない。
//!
//! 走査は [`split`] の 1 本: 本文を 1 字ずつ読んで、コメントを捨て、文字列・文字のリテラルを
//! 抜き出す。コメントを捨てるのは、ここのコメントが日本語で書かれ、「`std::fs` を読まない」の
//! ような説明に禁じた綴りが出てよいから（見張るのは読み込みで、説明の文ではない）。
//!
//! 知っている限界（どれも、この 2 つの主張を保つには足りる）:
//!
//! - 手書きの読み取りで、Rust の字句の全部は知らない。知っているのは `//`・入れ子の `/* */`・
//!   `"…"`（`\` の逃がし）・`r"…"`／`r#"…"#`・`'x'`／`'\…'`・寿命の `'a`。
//! - リテラルは綴りを見る。`"\u{3042}"` のように逃がして書いた ASCII の外の字は見えない
//!   （端末へ出た文そのものは、`cli_test_support.rs` の `call` が毎回 ASCII かを確かめている）。
//! - 禁じた綴りは名前で見る。`use std as s;` のような別名や、マクロの中で組んだ道筋は見えない。

/// 本文を「コメントとリテラルを抜いたコード」と「リテラルの中身（始まりの行番号つき）」に分ける。
/// コードの側には、リテラルの在った場所に空の `""`／`''` を残す。
fn split(source: &str) -> (String, Vec<(usize, String)>) {
    let chars: Vec<char> = source.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let lines_in = |from: usize, to: usize| chars[from..to].iter().filter(|c| **c == '\n').count();
    let (mut code, mut literals) = (String::new(), Vec::new());
    let (mut i, mut line) = (0, 1);
    while let Some(c) = at(i) {
        // この字から始まるコメント・リテラルの終わり（次に読む位置）を決める。
        let start = i;
        match (c, at(i + 1), at(i + 2)) {
            ('/', Some('/'), _) => {
                // 改行は残す（行番号と、語の区切りのため）。
                i = (i..chars.len())
                    .find(|&j| chars[j] == '\n')
                    .unwrap_or(chars.len());
            }
            ('/', Some('*'), _) => {
                let mut depth = 0;
                while i < chars.len() {
                    match (chars[i], at(i + 1)) {
                        ('/', Some('*')) => (depth, i) = (depth + 1, i + 2),
                        ('*', Some('/')) => (depth, i) = (depth - 1, i + 2),
                        _ => i += 1,
                    }
                    if depth == 0 {
                        break;
                    }
                }
                code.push(' ');
            }
            ('"', ..) => {
                // 直前の語が `r`／`br`（その後に `#` が何個か）なら生の文字列で、`\` は逃がしでない。
                let head = code.trim_end_matches('#');
                let word = |c: &char| c.is_alphanumeric() || *c == '_';
                let prefix: String = head.chars().rev().take_while(word).collect();
                let raw = matches!(prefix.as_str(), "r" | "rb");
                let hashes = if raw { code.len() - head.len() } else { 0 };
                let closes =
                    |j: usize| chars[j] == '"' && (1..=hashes).all(|n| at(j + n) == Some('#'));
                let mut end = i + 1;
                while end < chars.len() && !closes(end) {
                    end += if !raw && chars[end] == '\\' { 2 } else { 1 };
                }
                let end = end.min(chars.len());
                literals.push((line, chars[i + 1..end].iter().collect()));
                code.push_str("\"\"");
                i = (end + 1 + hashes).min(chars.len());
            }
            // 文字のリテラルは `'\…'` か `'x'`。それ以外の `'` は寿命・ラベルで、コードのまま。
            ('\'', Some('\\'), _) | ('\'', Some(_), Some('\'')) => {
                let from = if at(i + 1) == Some('\\') {
                    i + 3
                } else {
                    i + 2
                };
                let end = (from..chars.len()).find(|&j| chars[j] == '\'');
                let end = end.unwrap_or(chars.len());
                literals.push((line, chars[i + 1..end].iter().collect()));
                code.push_str("''");
                i = (end + 1).min(chars.len());
            }
            _ => {
                code.push(c);
                i += 1;
            }
        }
        line += lines_in(start, i);
    }
    (code, literals)
}

/// 判断の中核に在ってはならない道筋の綴り（空白を詰めたコードの中で探す）。
/// `std::{`・`std::*` は、まとめ書きで何を読み込んだかが綴りに出なくなるので、まとめて禁じる
/// （`std` からは 1 行に 1 つずつ読み込む）。
const IMPURE_PATHS: [&str; 5] = ["std::fs", "std::time", "std::process", "std::{", "std::*"];
/// 同じく、在ってはならない名前（1 語として探す）。ログのクレートと、時計・ファイルの型。
const IMPURE_WORDS: [&str; 4] = ["tracing", "SystemTime", "Instant", "File"];

/// `source` のコード（コメントとリテラルを除く）に在る、禁じた綴り。
fn impure_spellings(source: &str) -> Vec<&'static str> {
    let (code, _) = split(source);
    let dense: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let words: Vec<&str> = code
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .collect();
    let paths = IMPURE_PATHS.into_iter().filter(|path| dense.contains(path));
    let named = IMPURE_WORDS.into_iter().filter(|word| words.contains(word));
    paths.chain(named).collect()
}

/// `source` の文字列・文字のリテラルのうち、ASCII の外の字を含むもの（行番号つき）。
fn non_ascii_literals(source: &str) -> Vec<(usize, String)> {
    let (_, literals) = split(source);
    let outside = |(_, text): &(usize, String)| !text.is_ascii();
    literals.into_iter().filter(outside).collect()
}

fn literals(source: &str) -> Vec<String> {
    split(source).1.into_iter().map(|(_, text)| text).collect()
}

// ---- 主張 ----

/// 判断の中核は、ファイル・時計・プロセス・ログを読み込まない。
#[test]
fn the_judgement_core_spells_no_file_clock_process_or_log() {
    let found = impure_spellings(include_str!("plan.rs"));
    assert!(found.is_empty(), "plan.rs spells {found:?}");
}

/// 端末へ出る文を持つファイルの文字列・文字のリテラルは ASCII だけ。
#[test]
fn files_that_write_to_the_terminal_have_ascii_only_literals() {
    let sources = [
        ("cli.rs", include_str!("cli.rs")),
        ("status.rs", include_str!("status.rs")),
        ("wait.rs", include_str!("wait.rs")),
        ("error.rs", include_str!("error.rs")),
    ];
    for (file, source) in sources {
        // 較正: リテラルを 1 つも拾えない走査は、何が書かれていても緑になる。
        assert!(!literals(source).is_empty(), "{file}: no literal found");
        let found = non_ascii_literals(source);
        assert!(found.is_empty(), "{file}: non-ASCII literals {found:?}");
    }
}

// ---- 走査の較正 ----

#[test]
fn the_scan_reads_strings_chars_and_comments_apart() {
    // 文字列の中の `//` はコメントでない。
    assert_eq!(literals(r#"let a = "x // y"; // "z""#), ["x // y"]);
    // 文字のリテラルの中の `"` は文字列を始めない。
    assert_eq!(literals(r#"let a = '"'; let b = "c";"#), ["\"", "c"]);
    // 逃がした `"` は文字列を終えない。逃がした `\` の後の `"` は終える。
    assert_eq!(
        literals(r#"("a \" b", "c\\", "d")"#),
        [r#"a \" b"#, r"c\\", "d"]
    );
    // 生の文字列は `"` を含められ、`\` は逃がしでない。
    assert_eq!(
        literals(r###"(r"a\", r#"b "c" d"#, br##"e "# f"##)"###),
        [r"a\", r#"b "c" d"#, r##"e "# f"##]
    );
    // ブロックコメントの中の `"` は文字列を始めない（入れ子も 1 つのコメント）。
    assert_eq!(literals(r#"/* " /* ' */ " */ "a""#), ["a"]);
    // 逃がした文字のリテラル。寿命とラベルの `'` は文字を始めない。
    assert_eq!(
        literals(r"fn f<'a>(x: &'a str) { 'l: loop { ('\'', '\\', 'あ', '\u{3042}'); } }"),
        ["\\'", r"\\", "あ", r"\u{3042}"]
    );
}

#[test]
fn the_scan_reports_the_line_of_a_non_ascii_literal_and_ignores_comments() {
    let source = "// 日本語のコメント\n/* 複数行の\n   コメント */\nlet a = \"ascii\";\n\
                  /// 説明\nlet b = \"日本語\"; // 行末\nlet c = 'あ';\n";
    assert_eq!(
        non_ascii_literals(source),
        [(6, "日本語".to_owned()), (7, "あ".to_owned())]
    );
    // 複数行の文字列は始まりの行で指し、その後の行番号もずれない。
    assert_eq!(
        non_ascii_literals("let a = \"1\n2\n3\";\nlet b = \"é\";"),
        [(4, "é".to_owned())]
    );
}

#[test]
fn the_scan_catches_every_impure_spelling_and_none_in_comments_or_literals() {
    let caught = |source: &str| !impure_spellings(source).is_empty();
    for source in [
        "use std::fs;",
        "let text = std::fs::read_to_string(path);",
        "let text = ::std::fs::read(path);",
        "use std :: fs ;",
        "use std::\n    fs;",
        "use std::{collections::BTreeMap, fs};",
        "use std::*;",
        "use std::time::Duration;",
        "let now = SystemTime::now();",
        "let t = Instant::now();",
        "let f = File::open(path);",
        "std::process::exit(1);",
        "let pid = std::process::id();",
        "tracing::info!(id);",
        "use tracing::warn;",
    ] {
        assert!(caught(source), "not caught: {source}");
    }
    for source in [
        "// std::fs も tracing も読み込まない\nfn f() {}",
        "/// [`std::time::SystemTime`] は口の層が読む。\nfn f() {}",
        "/* use std::process; */ fn f() {}",
        r#"const WHY: &str = "std::fs tracing File";"#,
        "use std::collections::BTreeMap;",
        "use std::fmt;",
        "struct StateFile; fn profile() {} let instantly = 1;",
    ] {
        assert_eq!(impure_spellings(source), [""; 0], "{source}");
    }
}
