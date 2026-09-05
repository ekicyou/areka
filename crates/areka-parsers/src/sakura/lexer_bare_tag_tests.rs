//! 角括弧を伴わないタグの消費規律（`bare_tag_len`）を字句層で固定するテスト。
//!
//! 対象は `lex` の出力トークン列のみ（意味付けは `decode` の領分ゆえここでは見ない）。
//! 固定するのは次の 2 方向:
//! - **読み足りない**（`\_a本文` の `a` が本文へ漏れる旧欠陥）— 要件 1.1／1.1a／2.4。
//! - **読みすぎ**（ワード走査の長さで消費して台詞を飲み込む）— 要件 2.1／2.4／5.6。
//!
//! あわせて、本仕様で変わってはならない既存規律（既知 1 文字タグ・短縮形・
//! エスケープ・クォート・未閉じの寛容吸収・引数分割）も同じファイルで固定する
//! （要件 4.1〜4.6／5.7）。すべて決定論（`#[test]`・時計／GPU／実機に依存しない・要件 5.8）。

use super::{Token, lex};

// ───────────────────────────────────────────────────────────────────
// 期待値の組み立て補助（本ファイル内のみで使う）。
// ───────────────────────────────────────────────────────────────────

fn bare(spelling: &str) -> Token {
    Token::Bare(spelling.to_string())
}

fn text(s: &str) -> Token {
    Token::Text(s.to_string())
}

fn sysvar(keyword: &str) -> Token {
    Token::SysVar(keyword.to_string())
}

fn raw(s: &str) -> Token {
    Token::Raw(s.to_string())
}

fn tag(word: &str, args: &[&str]) -> Token {
    Token::Tag {
        word: word.to_string(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
    }
}

// ───────────────────────────────────────────────────────────────────
// T1／T2: 正典が定める角括弧なし `\_` タグの全件（要件 1.1・1.1a・1.2・5.1）。
// ───────────────────────────────────────────────────────────────────

/// T1（要件 1.1・1.2・5.1）: 2 文字形 8 個のすべてが `\_` ＋ 1 文字で 1 単位になり、
/// 本文トークンを 1 つも生まない。
#[test]
fn two_char_underscore_tags_consume_as_single_unit() {
    assert_eq!(lex(r"\_a"), vec![bare("_a")]);
    assert_eq!(lex(r"\_q"), vec![bare("_q")]);
    assert_eq!(lex(r"\_n"), vec![bare("_n")]);
    assert_eq!(lex(r"\_s"), vec![bare("_s")]);
    assert_eq!(lex(r"\_V"), vec![bare("_V")]);
    assert_eq!(lex(r"\_?"), vec![bare("_?")]);
    assert_eq!(lex(r"\_+"), vec![bare("_+")]);
    assert_eq!(lex(r"\_!"), vec![bare("_!")]);
}

/// T2（要件 1.1a・1.2・5.1）: 3 文字形 4 個のすべてが `\__` ＋ 1 文字で 1 単位になり、
/// 2 個目の `_` も続く 1 文字も本文へ残らない。
#[test]
fn three_char_underscore_tags_consume_as_single_unit() {
    assert_eq!(lex(r"\__c"), vec![bare("__c")]);
    assert_eq!(lex(r"\__t"), vec![bare("__t")]);
    assert_eq!(lex(r"\__q"), vec![bare("__q")]);
    assert_eq!(lex(r"\__v"), vec![bare("__v")]);
}

// ───────────────────────────────────────────────────────────────────
// T3／T4: 綴りの同一視をしない・正典外の組み合わせも同じ規律（要件 1.3・1.4）。
// ───────────────────────────────────────────────────────────────────

/// T3（要件 1.4）: 大文字と小文字は別の綴りとして区別する（`\_V` と `\_v` を同一視しない）。
#[test]
fn underscore_tag_spelling_is_case_sensitive() {
    assert_eq!(lex(r"\_V"), vec![bare("_V")]);
    assert_eq!(lex(r"\_v"), vec![bare("_v")]);
    assert_ne!(lex(r"\_V"), lex(r"\_v"));
}

/// T4（要件 1.3）: 正典に定義が無い組み合わせ（英小文字・数字・記号）も同じ規律で
/// 1 単位になる。
#[test]
fn underscore_tag_applies_to_non_canonical_followers() {
    assert_eq!(lex(r"\_z"), vec![bare("_z")]);
    assert_eq!(lex(r"\_9"), vec![bare("_9")]);
    assert_eq!(lex(r"\_#"), vec![bare("_#")]);
}

// ───────────────────────────────────────────────────────────────────
// T5〜T7: 直後の 3 境界（本文・`\`・`%`）× 2 形（要件 1.5・1.6・2.4・5.4）。
// ───────────────────────────────────────────────────────────────────

/// T5（要件 1.5・2.4・5.4・5.6）: 直後に本文が続いてもタグは 1 文字で終端し、
/// 本文はそのまま独立した `Text` として残る（読みすぎの検出対）。
#[test]
fn underscore_tag_does_not_swallow_following_body_text() {
    assert_eq!(lex(r"\_a本文"), vec![bare("_a"), text("本文")]);
    assert_eq!(lex(r"\__q本文"), vec![bare("__q"), text("本文")]);
}

/// T6（要件 1.6・5.4）: 直後が別のタグの開始（`\`）なら、そこで終端して次のタグへ渡す。
#[test]
fn underscore_tag_terminates_before_next_tag() {
    assert_eq!(lex(r"\_a\e"), vec![bare("_a"), bare("e")]);
    assert_eq!(lex(r"\__q\e"), vec![bare("__q"), bare("e")]);
}

/// T7（要件 1.6・5.4）: 直後がシステム変数の開始（`%`）なら、そこで終端して
/// `%keyword` を独立した `SysVar` として読む。
#[test]
fn underscore_tag_terminates_before_sysvar() {
    assert_eq!(lex(r"\_a%username"), vec![bare("_a"), sysvar("username")]);
    assert_eq!(lex(r"\__q%username"), vec![bare("__q"), sysvar("username")]);
}

// ───────────────────────────────────────────────────────────────────
// T8／T9: 入力末尾と `_` の重ね（要件 1.7・1.7a・1.1b・5.3・5.3a）。
// ───────────────────────────────────────────────────────────────────

/// T8（要件 1.7・1.7a・5.3）: `\_`／`\__` が入力末尾に単独で現れても解析は中断せず、
/// 在るだけを綴りとして消費する（余分な文字を本文へ残さない）。
#[test]
fn underscore_tag_at_end_of_input_consumes_what_is_there() {
    assert_eq!(lex(r"\_"), vec![bare("_")]);
    assert_eq!(lex(r"\__"), vec![bare("__")]);
}

/// T9（要件 1.1b・5.3a）: `_` を 3 個以上重ねた形は新しいタグ形として扱わない。
/// `\___x` は綴り `___` ＋ 本文 `x`。
#[test]
fn three_or_more_underscores_are_not_a_new_tag_form() {
    assert_eq!(lex(r"\___x"), vec![bare("___"), text("x")]);
}

// ───────────────────────────────────────────────────────────────────
// T10: 角括弧が続く形は既存の角括弧経路が優先（要件 1.8）。
// ───────────────────────────────────────────────────────────────────

/// T10（要件 1.8）: ワードの直後に `[` が来たら、角括弧なしの消費規律は働かず
/// 既存の角括弧付きタグとしての切り分けをそのまま用いる。
#[test]
fn bracket_form_takes_precedence_over_bare_underscore_tag() {
    assert_eq!(lex(r"\_a[Hint]"), vec![tag("_a", &["Hint"])]);
    assert_eq!(lex(r"\__q[OnTest]"), vec![tag("__q", &["OnTest"])]);
    assert_eq!(lex(r"\_[x]"), vec![tag("_", &["x"])]);
}

// ───────────────────────────────────────────────────────────────────
// T11／T12: 本文の逐語保存と開始形／終了形の対（要件 2.5・5.5・5.6）。
// ───────────────────────────────────────────────────────────────────

/// T11（要件 2.5）: 全角と半角が混ざった本文でも、挟まれた文字が 1 文字も欠けず
/// 1 文字も増えない。
#[test]
fn underscore_tag_preserves_mixed_width_body_text() {
    assert_eq!(
        lex(r"\_qあaい\_q"),
        vec![bare("_q"), text("あaい"), bare("_q")],
    );
}

/// T12（要件 5.5・5.6）: 変異対の要。角括弧付きの開始形と角括弧なしの終了形が
/// 混在しても、本文 2 つがそれぞれ逐語で残る（読み足りない／読みすぎの両方向が赤になる形）。
#[test]
fn anchor_open_and_close_split_without_losing_body() {
    assert_eq!(
        lex(r"\_a[Hint]アンカー\_aをクリックする。"),
        vec![
            tag("_a", &["Hint"]),
            text("アンカー"),
            bare("_a"),
            text("をクリックする。"),
        ],
    );
}

// ───────────────────────────────────────────────────────────────────
// T13〜T15: 既存規律が本仕様の前後で同一であることの固定（要件 4.1〜4.6・5.7）。
// ───────────────────────────────────────────────────────────────────

/// T13（要件 4.1・4.2・5.7）: 既知 1 文字タグ 8 個は、直後に本文が続いても
/// 1 文字で終端し、本文をタグの一部として吸い込まない。
#[test]
fn known_one_char_tags_do_not_swallow_following_body_text() {
    assert_eq!(lex(r"\eあ"), vec![bare("e"), text("あ")]);
    assert_eq!(lex(r"\cあ"), vec![bare("c"), text("あ")]);
    assert_eq!(lex(r"\-あ"), vec![bare("-"), text("あ")]);
    assert_eq!(lex(r"\nあ"), vec![bare("n"), text("あ")]);
    assert_eq!(lex(r"\0あ"), vec![bare("0"), text("あ")]);
    assert_eq!(lex(r"\1あ"), vec![bare("1"), text("あ")]);
    assert_eq!(lex(r"\hあ"), vec![bare("h"), text("あ")]);
    assert_eq!(lex(r"\uあ"), vec![bare("u"), text("あ")]);
}

/// T14（要件 4.3・5.7）: 短縮形の規律は不変——1 桁のみを読み、直後が `[` なら
/// 角括弧付きタグを優先する。
#[test]
fn shorthand_rules_are_unchanged() {
    assert_eq!(lex(r"\w2"), vec![Token::Shorthand { word: 'w', n: 2 }]);
    assert_eq!(lex(r"\w[2]"), vec![tag("w", &["2"])]);
}

/// T14（要件 4.4・5.7）: エスケープの扱いは不変——`\\` はリテラル `\`、
/// `\%` はリテラル `%`、角括弧内の `\]` はリテラル `]`。
#[test]
fn escapes_are_unchanged() {
    assert_eq!(lex(r"\\"), vec![text(r"\")]);
    assert_eq!(lex(r"\%"), vec![text("%")]);
    assert_eq!(lex(r"\s[a\]b]"), vec![tag("s", &["a]b"])]);
}

/// T14（要件 4.4・5.7）: 引数のクォートは不変——`"..."` は内側の `,` を保護して
/// 1 引数になる。
#[test]
fn quoted_args_are_unchanged() {
    assert_eq!(lex(r#"\s["a,b"]"#), vec![tag("s", &["a,b"])]);
}

/// T14（要件 4.5・5.7）: 未閉じの寛容吸収は不変——`\` から入力末尾までを 1 単位の
/// `Raw` として取り込み、解析を中断しない。角括弧なし `\_` タグの未閉じ形も同じ。
#[test]
fn unclosed_brackets_are_absorbed_as_raw_unchanged() {
    assert_eq!(lex(r"\s[1000"), vec![raw(r"\s[1000")]);
    assert_eq!(lex(r"\_a["), vec![raw(r"\_a[")]);
}

/// T14（要件 4.6・5.7）: 角括弧付きタグの引数分割は不変——カンマ区切り・空引数・
/// 引数 0 個の結果が変わらない。
#[test]
fn bracket_arg_splitting_is_unchanged() {
    assert_eq!(lex(r"\![a,,c]"), vec![tag("!", &["a", "", "c"])]);
    assert_eq!(lex(r"\s[]"), vec![tag("s", &[])]);
}

/// T15（要件 4.1）: 入力末尾の裸 `\` は綴り `"\\"` の `Bare` として載る（挙動不変）。
#[test]
fn trailing_lone_backslash_is_bare_spelling() {
    assert_eq!(lex(r"\"), vec![bare(r"\")]);
}
