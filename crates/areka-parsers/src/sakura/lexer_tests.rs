//! `lexer` の単体テスト（一般構文分割・タスク 3.1 範囲）。
//!
//! 検証対象は構文トークン分割のみ:
//! - 正準タグ（`\` ＋ word ＋ `[args]`）
//! - bare タグ（`\e` `\c` `\-` `\n`）
//! - `\wN` 短縮（1 桁）
//! - `%keyword` システム変数
//! - タグ間テキスト
//! - 角括弧内のカンマ区切り複数引数
//!
//! エスケープ／クォート／寛容境界（未閉じ `[`/`"`）はタスク 3.2 の領分ゆえ
//! ここでは検証しない。

use super::lexer::{lex, Token};

/// タスク 3.1 の観測基準（tasks.md L23）:
/// `\s[1000]` `\p[0]` `%username` `\w2` `\![a,b,c]` `こんにちは` が
/// 期待トークン列へ分割されること。
#[test]
fn task_3_1_observable_done_example() {
    let input = r"\s[1000]\p[0]%username\w2\![a,b,c]こんにちは";
    let tokens = lex(input);
    assert_eq!(
        tokens,
        vec![
            Token::Tag {
                word: "s".to_string(),
                args: vec!["1000".to_string()],
            },
            Token::Tag {
                word: "p".to_string(),
                args: vec!["0".to_string()],
            },
            Token::SysVar("username".to_string()),
            Token::WaitShorthand(2),
            Token::Tag {
                word: "!".to_string(),
                args: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            },
            Token::Text("こんにちは".to_string()),
        ],
    );
}

#[test]
fn canonical_tag_single_arg() {
    assert_eq!(
        lex(r"\s[1000]"),
        vec![Token::Tag {
            word: "s".to_string(),
            args: vec!["1000".to_string()],
        }],
    );
}

#[test]
fn canonical_tag_multi_word_with_underscore() {
    // `\_l[x,y]` — word は `_l`、`[` がワード終端。
    assert_eq!(
        lex(r"\_l[10,20]"),
        vec![Token::Tag {
            word: "_l".to_string(),
            args: vec!["10".to_string(), "20".to_string()],
        }],
    );
}

#[test]
fn bracket_args_split_on_comma() {
    assert_eq!(
        lex(r"\![a,b,c]"),
        vec![Token::Tag {
            word: "!".to_string(),
            args: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        }],
    );
}

#[test]
fn empty_bracket_yields_no_args() {
    assert_eq!(
        lex(r"\s[]"),
        vec![Token::Tag {
            word: "s".to_string(),
            args: vec![],
        }],
    );
}

#[test]
fn bare_tags_e_c_minus() {
    assert_eq!(lex(r"\e"), vec![Token::Bare('e')]);
    assert_eq!(lex(r"\c"), vec![Token::Bare('c')]);
    assert_eq!(lex(r"\-"), vec![Token::Bare('-')]);
}

#[test]
fn bare_n_without_bracket() {
    // 引数なしの `\n` は bare（`\n[...]` は Tag）。
    assert_eq!(lex(r"\n"), vec![Token::Bare('n')]);
}

#[test]
fn newline_tag_with_percent_bracket() {
    assert_eq!(
        lex(r"\n[150]"),
        vec![Token::Tag {
            word: "n".to_string(),
            args: vec!["150".to_string()],
        }],
    );
}

#[test]
fn wait_shorthand_single_digit() {
    assert_eq!(lex(r"\w2"), vec![Token::WaitShorthand(2)]);
    assert_eq!(lex(r"\w9"), vec![Token::WaitShorthand(9)]);
}

#[test]
fn wait_with_bracket_is_tag_not_shorthand() {
    // `\w[2]` は短縮形でなく正準タグ。
    assert_eq!(
        lex(r"\w[2]"),
        vec![Token::Tag {
            word: "w".to_string(),
            args: vec!["2".to_string()],
        }],
    );
}

#[test]
fn sysvar_keyword() {
    assert_eq!(
        lex(r"%username"),
        vec![Token::SysVar("username".to_string())],
    );
}

#[test]
fn plain_text_run() {
    assert_eq!(lex("こんにちは"), vec![Token::Text("こんにちは".to_string())]);
}

#[test]
fn empty_input_yields_no_tokens() {
    assert_eq!(lex(""), vec![]);
}

#[test]
fn text_between_tags_preserves_order_and_chars() {
    // タグ間テキストが順序・文字順を保って分割される（要件 9.1/9.2）。
    let tokens = lex(r"あ\eいう\cえ");
    assert_eq!(
        tokens,
        vec![
            Token::Text("あ".to_string()),
            Token::Bare('e'),
            Token::Text("いう".to_string()),
            Token::Bare('c'),
            Token::Text("え".to_string()),
        ],
    );
}

#[test]
fn sysvar_terminates_at_backslash() {
    // `%username` の直後にタグが来たら sysvar はそこで終端する。
    assert_eq!(
        lex(r"%username\e"),
        vec![Token::SysVar("username".to_string()), Token::Bare('e')],
    );
}

#[test]
fn sysvar_terminates_at_text_is_not_split() {
    // sysvar keyword は英数字を読み、続く非英数字（日本語）はテキストへ。
    assert_eq!(
        lex(r"%usernameさん"),
        vec![
            Token::SysVar("username".to_string()),
            Token::Text("さん".to_string()),
        ],
    );
}

#[test]
fn speaker_scope_then_surface() {
    assert_eq!(
        lex(r"\p[0]\s[通常]"),
        vec![
            Token::Tag {
                word: "p".to_string(),
                args: vec!["0".to_string()],
            },
            Token::Tag {
                word: "s".to_string(),
                args: vec!["通常".to_string()],
            },
        ],
    );
}
