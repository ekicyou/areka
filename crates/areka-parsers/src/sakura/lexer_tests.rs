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
            Token::Shorthand { word: 'w', n: 2 },
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
    assert_eq!(lex(r"\w2"), vec![Token::Shorthand { word: 'w', n: 2 }]);
    assert_eq!(lex(r"\w9"), vec![Token::Shorthand { word: 'w', n: 9 }]);
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

// ───────────────────────────────────────────────────────────────────
// タスク 3.2: エスケープ・引数クォート・寛容な境界処理
// ───────────────────────────────────────────────────────────────────

/// 要件 13.5: `\\` はタグ開始でなくリテラルの `\` 1 文字としてテキストへ。
#[test]
fn escape_backslash_to_literal_backslash() {
    assert_eq!(lex(r"a\\b"), vec![Token::Text(r"a\b".to_string())]);
}

/// 要件 13.5: 単独の `\\`（前後テキストなし）もリテラル `\` テキスト 1 個。
#[test]
fn escape_backslash_alone() {
    assert_eq!(lex(r"\\"), vec![Token::Text(r"\".to_string())]);
}

/// 要件 13.5: `\\` の直後の `\e` は通常 bare タグとして区切られる
/// （`\\` がエスケープを消費し、次の `\e` は独立）。
#[test]
fn escape_backslash_then_real_tag() {
    assert_eq!(
        lex(r"\\\e"),
        vec![Token::Text(r"\".to_string()), Token::Bare('e')],
    );
}

/// 要件 13.6: `\%` はシステム変数開始でなくリテラルの `%` としてテキストへ。
#[test]
fn escape_percent_to_literal_percent() {
    assert_eq!(
        lex(r"a\%usernameb"),
        vec![Token::Text("a%usernameb".to_string())],
    );
}

/// 要件 13.6: `\%` 単独もリテラル `%`。後続の本物の `%username` は sysvar。
#[test]
fn escape_percent_then_real_sysvar() {
    assert_eq!(
        lex(r"\%%username"),
        vec![
            Token::Text("%".to_string()),
            Token::SysVar("username".to_string()),
        ],
    );
}

/// 要件 13.7: 角括弧内の `\]` はタグ終端でなくリテラルの `]` として引数へ。
#[test]
fn escape_bracket_close_inside_args() {
    assert_eq!(
        lex(r"\s[a\]b]"),
        vec![Token::Tag {
            word: "s".to_string(),
            args: vec!["a]b".to_string()],
        }],
    );
}

/// 要件 13.7: 角括弧内の `\]` を含んでも後続の本物の `]` が終端として働く。
#[test]
fn escape_bracket_close_then_real_terminator() {
    assert_eq!(
        lex(r"\s[x\]]ok"),
        vec![
            Token::Tag {
                word: "s".to_string(),
                args: vec!["x]".to_string()],
            },
            Token::Text("ok".to_string()),
        ],
    );
}

/// 要件 13.4: クォート `"..."` は囲まれた全体を 1 引数扱い、内側の `,` を保護。
#[test]
fn quoted_arg_protects_comma() {
    assert_eq!(
        lex(r#"\s["a,b"]"#),
        vec![Token::Tag {
            word: "s".to_string(),
            args: vec!["a,b".to_string()],
        }],
    );
}

/// 要件 13.4: 二重化 `""` は引数内のリテラル `"` として解釈。
#[test]
fn quoted_arg_doubled_quote_is_literal() {
    assert_eq!(
        lex(r#"\s["a""b"]"#),
        vec![Token::Tag {
            word: "s".to_string(),
            args: vec![r#"a"b"#.to_string()],
        }],
    );
}

/// 要件 13.4: クォート引数と非クォート引数の混在（`,` での分割は維持）。
#[test]
fn quoted_and_unquoted_args_mixed() {
    assert_eq!(
        lex(r#"\![raise,"a,b",c]"#),
        vec![Token::Tag {
            word: "!".to_string(),
            args: vec!["raise".to_string(), "a,b".to_string(), "c".to_string()],
        }],
    );
}

/// 要件 10.3/13.8: 未閉じ `[` は入力末尾までを Raw へ吸収し、走査を中断しない。
/// 直前の正常トークンは欠落しない。
#[test]
fn unclosed_bracket_absorbed_as_raw_preserving_prior() {
    assert_eq!(
        lex(r"\e\s[1000"),
        vec![Token::Bare('e'), Token::Raw(r"\s[1000".to_string())],
    );
}

/// 要件 10.3/13.8: 未閉じ `"`（クォート閉じ忘れ）も Raw へ吸収。
#[test]
fn unclosed_quote_absorbed_as_raw() {
    assert_eq!(
        lex(r#"\s["a,b]"#),
        vec![Token::Raw(r#"\s["a,b]"#.to_string())],
    );
}

/// 要件 10.3/13.8: 未閉じ `[` の Raw 吸収後、前のテキストトークンも保持される。
#[test]
fn unclosed_bracket_preserves_preceding_text() {
    assert_eq!(
        lex(r"foo\s[1000"),
        vec![
            Token::Text("foo".to_string()),
            Token::Raw(r"\s[1000".to_string()),
        ],
    );
}

/// 要件 13.8: 未知タグ（emo2 subset 外）も構文として 1 単位に区切り、
/// 前後の正常トークンを破壊しない（正準形なら Tag として区切れる）。
#[test]
fn unknown_tag_split_as_tag_preserving_neighbors() {
    assert_eq!(
        lex(r"あ\foo[a,b]い"),
        vec![
            Token::Text("あ".to_string()),
            Token::Tag {
                word: "foo".to_string(),
                args: vec!["a".to_string(), "b".to_string()],
            },
            Token::Text("い".to_string()),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// タスク 1.1（areka-P0-balloon-face-cue）: 裸形 `\bN` 短縮の構文檻。
//
// 一桁の面数字 `N` を短縮形として取り込み本文へ漏らさない（要件 1.3）。
// `\w` と同一の shorthand 規律を語（`'b'`）に依らず共通適用する（要件 1.2）。
// 既存タグ（とりわけ `\w` 系）のパース結果は不変（要件 1.6）。
// ───────────────────────────────────────────────────────────────────

/// 要件 1.3: 裸形 `\b1` の一桁面数字は本文テキストとして漏れてはならない
/// （本文数字漏れの根絶）。分割語彙に依らず「`Text` トークンが 1 つも出ない」
/// ことで構造的に固定する。
#[test]
fn balloon_bare_single_digit_not_leaked_as_text() {
    let tokens = lex(r"\b1");
    assert!(
        !tokens.iter().any(|t| matches!(t, Token::Text(_))),
        r"\b1 must not leak the face digit as body Text, got {tokens:?}",
    );
}

/// 要件 1.2/1.3: 裸形 `\b1` は 1 桁面数字を取り込み、単一の shorthand トークンへ。
/// `\w1` と同型（`word` のみ `'b'`）＝語に依らず共通規律。
#[test]
fn balloon_bare_shorthand_single_digit() {
    assert_eq!(lex(r"\b1"), vec![Token::Shorthand { word: 'b', n: 1 }]);
}

/// 要件 1.3: `\b12` は先頭 1 桁のみ面数字として取り込み、続く `2` は正当な
/// 本文テキストとして分離される（多桁は shorthand でなくブラケット形の領分）。
#[test]
fn balloon_bare_shorthand_consumes_one_digit_rest_is_text() {
    assert_eq!(
        lex(r"\b12"),
        vec![
            Token::Shorthand { word: 'b', n: 1 },
            Token::Text("2".to_string()),
        ],
    );
}

/// 要件 1.2（境界）: `\b2[x]`（数字＋直後 `[`）は shorthand でなく正準タグ扱い
/// （word=`b2`）。既存 `\w2[x]` と同型＝直後が数字+`[` は正準タグ優先という
/// 共通規則が語に依らず働く。
#[test]
fn balloon_digit_then_bracket_is_tag_not_shorthand() {
    assert_eq!(
        lex(r"\b2[x]"),
        vec![Token::Tag {
            word: "b2".to_string(),
            args: vec!["x".to_string()],
        }],
    );
}

/// 既存 `\w` 系との対称確認: `\w2[x]` も同型（word=`w2` の正準タグ）。
/// `\b` 一般化が `\w` の同じ境界挙動を保つこと（要件 1.6）を固定する。
#[test]
fn wait_digit_then_bracket_is_tag_not_shorthand() {
    assert_eq!(
        lex(r"\w2[x]"),
        vec![Token::Tag {
            word: "w2".to_string(),
            args: vec!["x".to_string()],
        }],
    );
}

/// 要件 1.6: 数字を伴わない裸形 `\b`（単独）は既存の非短縮規則のまま
/// bare タグとして処理される（passthrough 維持）。
#[test]
fn balloon_bare_without_digit_is_bare_tag() {
    assert_eq!(lex(r"\b"), vec![Token::Bare('b')]);
}

/// 要件 1.6（境界）: 未閉じ `\b1[` は shorthand 判定を経ず（数字直後が `[`）、
/// 未閉じブラケットとして `\` から入力末尾までを `Raw` へ吸収する既存規則どおり。
#[test]
fn balloon_unclosed_bracket_absorbed_as_raw() {
    assert_eq!(lex(r"\b1["), vec![Token::Raw(r"\b1[".to_string())]);
}

/// 要件 1.6: `\b` 短縮の追加が前後の正常トークンを壊さない（本文・bare タグと共存）。
#[test]
fn balloon_bare_shorthand_preserves_neighbors() {
    assert_eq!(
        lex(r"あ\b3\eい"),
        vec![
            Token::Text("あ".to_string()),
            Token::Shorthand { word: 'b', n: 3 },
            Token::Bare('e'),
            Token::Text("い".to_string()),
        ],
    );
}
